#![feature(slice_group_by)]
use board_game_traits::Position as PositionTrait;
use clap::{App, Arg};
use pgn_traits::PgnPosition;
use rayon::current_thread_index;
use rusqlite::Connection;
use rusqlite::{params, OpenFlags};
use std::sync::{Arc, Mutex};
use std::{time::Instant, usize};
use tiltak::position::{Move, Position};
use tiltak::search::{MctsSetting, MonteCarloTree};

#[cfg(test)]
mod tests;
mod tinue_search;

fn parse_server_notation<const S: usize>(server_notation: &str) -> Vec<Move> {
    let move_splits = server_notation.split(',');
    move_splits.map(Move::from_string_playtak::<S>).collect()
}

/// Finds a unique tinue sequence, down to the given `depth`.
/// Returns the principal variation as a vector of move strings, and the depth of the tinue
fn find_unique_tinue_sized<const S: usize>(
    server_notation: &str,
    plies_to_undo: u32,
    depth: u32,
) -> Option<(Vec<MoveString>, u32)> {
    let moves = parse_server_notation::<S>(server_notation);
    // Apply moves
    let mut position = Position::<S>::start_position();
    print!("// ");
    for ply in moves.iter().take(moves.len() - plies_to_undo as usize) {
        print!("{} ", ply.to_string::<S>());
        position.do_move(ply.clone());
    }
    println!("TPS {}", position.to_fen());

    // Reconstruct the principal variation of the tinue
    let result = tinue_search::find_unique_tinue::<S>(&mut position, depth);

    result.map(|(mv, depth)| {
        let mcts_start_time = Instant::now();
        let mcts_settings = MctsSetting::default().exclude_moves(vec![mv.clone()]);
        let mut mcts_tree = MonteCarloTree::with_settings(position.clone(), mcts_settings);
        for _ in 0..1_000_000 {
            mcts_tree.select();
        }

        let (alternative_move, alternative_score) = mcts_tree.best_move();
        println!(
            "Tiltak variant analysis: {}, {:.1}% after {:.1}s, tps: {}",
            position.move_to_san(&alternative_move),
            alternative_score * 100.0,
            mcts_start_time.elapsed().as_secs_f64(),
            position.to_fen()
        );

        let mut pv_string = vec![position.move_to_san(&mv)];
        position.do_move(mv);
        pv_string.append(&mut tinue_search::pv(position.clone(), depth - 1));
        (pv_string, depth)
    })
}

fn handle_game(game: &GameRow, max_depth: u32, plies_to_undo: u32) -> Option<TinueGameRow> {
    let timer = Instant::now();

    let result = match game.size {
        4 => find_unique_tinue_sized::<4>(&game.notation, plies_to_undo, max_depth),
        5 => find_unique_tinue_sized::<5>(&game.notation, plies_to_undo, max_depth),
        6 => find_unique_tinue_sized::<6>(&game.notation, plies_to_undo, max_depth),
        s => panic!("Board size '{}' is not supported", s),
    };

    let time_taken = timer.elapsed().as_millis();

    if let Some((pv_strings, actual_depth)) = result {
        let json_string = serde_json::to_string(&pv_strings).unwrap();

        println!(
            "{{\"id\":{}, \"size\":{}, \"result\":\"{}\", \"max-depth\":{}, \"depth\":{}, \"movesToUndo\":{}, \"timeMs\":{}, \"tinue\":{}}}",
            game.id, game.size, game.result, max_depth, actual_depth, plies_to_undo, time_taken, json_string
        );

        Some(TinueGameRow {
            plies_to_undo,
            gameid: game.id,
            tinue: json_string,
            size: game.size,
            tinue_depth: actual_depth,
        })
    } else {
        println!(
            "{{\"id\":{}, \"size\":{}, \"result\":\"{}\", \"max-depth\":{}, \"depth\":0, \"movesToUndo\":{}, \"timeMs\":{}, \"tinue\":null}}",
            game.id, game.size, game.result, max_depth, plies_to_undo, time_taken
        );
        None
    }
}

/// A string representation of a single move
type MoveString = String;

struct TinueGameRow {
    gameid: u32,
    size: u32,
    plies_to_undo: u32,
    tinue_depth: u32,
    tinue: String,
}

struct GameRow {
    id: u32,
    notation: String,
    result: String,
    size: u32,
}

fn main() {
    let matches = App::new("Tinue Finder")
        .version("0.1.0")
        .author("Jan Schnitker <jan.s.92@web.de>")
        .about("Checks a database of Tak games for Tinues and writes them in a new table")
        .arg(
            Arg::with_name("database")
                .long("db")
                .takes_value(true)
                .help("Path of the database")
                .required(true),
        )
        .arg(
            Arg::with_name("board_size")
                .short("n")
                .long("board-size")
                .takes_value(true)
                .help("Checks only games of this board size")
                .required(true),
        )
        .arg(
            Arg::with_name("start_id")
                .short("s")
                .long("start-id")
                .takes_value(true)
                .help("ID of the game/row to start with (allows you to proceed where you left the last time)")
                .required(false)
                .default_value("8000"),
        )
        .arg(
            Arg::with_name("plies_to_undo")
                .short("u")
                .long("undo")
                .takes_value(true)
                .help("Number of plies to undo from the end position")
                .required(false)
                .default_value("3"),
        )
        .arg(
            Arg::with_name("max_depth")
                .short("d")
                .long("max-depth")
                .takes_value(true)
                .help("Maximum depth/length of a Tinue in plies (must be odd)")
                .required(false)
                .default_value("3"),
        )
        .arg(
            Arg::with_name("threads")
                .long("threads")
                .takes_value(true)
                .help("Number of threads to use to find puzzles concurrently")
                .required(false)
                .default_value("1"),
        )
        .arg(
            Arg::with_name("multi_tinue")
                .short("m")
                .long("multi-tinue")
                .help("Searches for all available tinues and how to handle all possible opponent replies. Increases computation time and output data massively.")
                .required(false)
        )
        .arg(
            Arg::with_name("test")
                .short("t")
                .long("test")
                .help("Only logs the output, does not write to the database")
                .required(false)
        )
        .get_matches();

    let get_arg_number =
        |arg_name: &str| -> u32 { matches.value_of(arg_name).unwrap().parse::<u32>().unwrap() };

    let board_size = get_arg_number("board_size");
    let plies_to_undo = get_arg_number("plies_to_undo");
    let max_depth = get_arg_number("max_depth");
    let number_of_threads = get_arg_number("threads");
    // To skip games already dealt with or that are old and invalid
    let min_game_id = get_arg_number("start_id");
    let db_path = matches.value_of("database").unwrap();
    let test = matches.occurrences_of("test") > 0;
    let multi_tinue = matches.occurrences_of("multi_tinue") > 0;

    if max_depth % 2 != 1 {
        panic!("max_depth must be an odd number as it represents the number of plies looked ahead. An even number would mean that your opponent does the final ply");
    }
    if plies_to_undo <= 1 {
        panic!("plies_to_undo must be greater than 1 to make sense");
    }
    if number_of_threads == 0 {
        panic!("at least 1 thread is required to run");
    }
    let number_of_threads = number_of_threads as usize;

    println!("Test={}", test);
    println!("multi_tinue={}", multi_tinue);
    println!("board_size={}", board_size);
    println!("plies_to_undo={}", plies_to_undo);
    println!("max_depth={}", max_depth);
    println!("min_game_id={}", min_game_id);
    println!("db_path={}", db_path);
    println!("threads={}", number_of_threads);

    // Configure maximum number of threads used
    rayon::ThreadPoolBuilder::new()
        .num_threads(number_of_threads)
        .build_global()
        .unwrap();

    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE).unwrap();

    if !test {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tinues (
        id integer primary key,
        gameid integer NOT NULL REFERENCES games(id),
        size integer,
        plies_to_undo integer,
        tinue_depth integer,
        tinue TEXT)",
            params![],
        )
        .unwrap();
    }

    // Do this step in its own block because `stmt` needs to go out of scope before we can take ownership of `conn` again
    let gamerows = {
        let mut stmt = conn.prepare("SELECT id, notation, result, size FROM games WHERE (result = ? or result = ?) and id > ? AND size = ?")
            .unwrap();

        stmt.query_map(params!["R-0", "0-R", min_game_id - 1, board_size], |row| {
            Ok(GameRow {
                id: row.get(0)?,
                notation: row.get(1)?,
                result: row.get(2)?,
                size: row.get(3)?,
            })
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<GameRow>>()
    };

    let conn_mtx: Arc<Mutex<Connection>> = Arc::new(Mutex::new(conn));
    rayon::scope_fifo(|scope| {
        for game in gamerows.iter() {
            let conn_arc = Arc::clone(&conn_mtx);
            scope.spawn_fifo(move |_| {
                println!("// Thread #{} Processing game #{}", current_thread_index().unwrap(), game.id);
                handle_game(&game, max_depth, plies_to_undo).and_then(|r| {
                    if test {
                        return None;
                    }

                    let local_conn = conn_arc.lock().unwrap();
                    Some(local_conn.execute("INSERT INTO tinues(gameid, size, plies_to_undo, tinue_depth, tinue) VALUES(?, ?, ?, ?, ?)", 
                        params![
                            r.gameid,
                            r.size,
                            r.plies_to_undo,
                            r.tinue_depth,
                            r.tinue]).unwrap())
                });
            });
        }
    });
}
