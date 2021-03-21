#![feature(slice_group_by)]
use arrayvec::ArrayVec;
use board_game_traits::board::{Board as BoardTrait, Color, GameResult};
use clap::{App, Arg};
use pgn_traits::pgn::PgnBoard;
use rusqlite::Connection;
use rusqlite::{params, OpenFlags};
use serde::Serialize;
use serde_json;
use std::sync::{Arc, Mutex};
use std::{cmp::Ordering, iter, str::FromStr};
use std::{time::Instant, usize};
use taik::{
    board,
    board::{Board, Direction, Move, Movement, Role, StackMovement},
};

pub fn parse_move<const S: usize>(input: &str) -> board::Move {
    let words: Vec<&str> = input.split_whitespace().collect();
    if words[0] == "P" {
        let square = board::Square::parse_square::<S>(&words[1].to_lowercase()).unwrap();
        let role = match words.get(2) {
            Some(&"C") => Role::Cap,
            Some(&"W") => Role::Wall,
            None => Role::Flat,
            Some(s) => panic!("Unknown role {} for move {}", s, input),
        };
        board::Move::Place(role, square)
    } else if words[0] == "M" {
        let start_square = board::Square::parse_square::<S>(&words[1].to_lowercase()).unwrap();
        let end_square = board::Square::parse_square::<S>(&words[2].to_lowercase()).unwrap();
        let pieces_dropped: ArrayVec<[u8; board::MAX_BOARD_SIZE - 1]> = words
            .iter()
            .skip(3)
            .map(|s| u8::from_str(s).unwrap())
            .collect();

        let num_pieces_taken: u8 = pieces_dropped.iter().sum();

        let mut pieces_held = num_pieces_taken;

        let pieces_taken: ArrayVec<[Movement; board::MAX_BOARD_SIZE - 1]> =
            iter::once(num_pieces_taken)
                .chain(
                    pieces_dropped
                        .iter()
                        .take(pieces_dropped.len() - 1)
                        .map(|pieces_to_drop| {
                            pieces_held -= pieces_to_drop;
                            pieces_held
                        }),
                )
                .map(|pieces_to_take| Movement { pieces_to_take })
                .collect();

        let direction = match (
            start_square.rank::<S>().cmp(&end_square.rank::<S>()),
            start_square.file::<S>().cmp(&end_square.file::<S>()),
        ) {
            (Ordering::Equal, Ordering::Less) => Direction::East,
            (Ordering::Equal, Ordering::Greater) => Direction::West,
            (Ordering::Less, Ordering::Equal) => Direction::South,
            (Ordering::Greater, Ordering::Equal) => Direction::North,
            _ => panic!("Diagonal move string {}", input),
        };

        board::Move::Move(
            start_square,
            direction,
            StackMovement {
                movements: pieces_taken,
            },
        )
    } else {
        unreachable!()
    }
}

fn parse_server_notation<const S: usize>(server_notation: &str) -> Vec<Move> {
    let move_splits = server_notation.split(",");
    move_splits.map(parse_move::<S>).collect()
}

fn do_it<const S: usize>(
    server_notation: &str,
    plies_to_undo: u32,
    depth: u32,
    find_only_one_tinue: bool,
) -> Option<IDDFSResult<Vec<TinueMove>>> {
    let moves = parse_server_notation::<S>(server_notation);
    // Apply moves
    let mut position = Board::<S>::start_board();
    for ply in moves.iter().take(moves.len() - plies_to_undo as usize) {
        position.do_move(ply.clone());
    }

    let active_color = position.side_to_move();

    return iddf_tinue_search(&mut position, depth, active_color, find_only_one_tinue);
}

fn do_it_sized(
    board_size: u32,
    server_notation: &str,
    plies_to_undo: u32,
    depth: u32,
    find_only_one_tinue: bool,
) -> Option<IDDFSResult<Vec<TinueMove>>> {
    match board_size {
        3 => do_it::<3>(server_notation, plies_to_undo, depth, find_only_one_tinue),
        4 => do_it::<4>(server_notation, plies_to_undo, depth, find_only_one_tinue),
        5 => do_it::<5>(server_notation, plies_to_undo, depth, find_only_one_tinue),
        6 => do_it::<6>(server_notation, plies_to_undo, depth, find_only_one_tinue),
        7 => do_it::<7>(server_notation, plies_to_undo, depth, find_only_one_tinue),
        8 => do_it::<8>(server_notation, plies_to_undo, depth, find_only_one_tinue),
        9 => do_it::<9>(server_notation, plies_to_undo, depth, find_only_one_tinue),
        _ => panic!("Board size '{}' is not supported", board_size),
    }
}

fn handle_game(
    game: &GameRow,
    max_depth: u32,
    plies_to_undo: u32,
    find_only_one_tinue: bool,
) -> Option<TinueGameRow> {
    let timer = Instant::now();

    let moves = do_it_sized(
        game.size,
        &game.notation,
        plies_to_undo,
        max_depth,
        find_only_one_tinue,
    );
    let actual_depth = moves.as_ref().map(|x| x.depth).unwrap_or(0);

    // moves include a first move from `me` and then answers to all possible replies from `opponent`
    // To reduce the data saved to the database (this one would be massive) the decision was taken to store only
    // a single example of a Tinue (the longest one available) in the move_options below if find_only_one_tinue
    let json_string = match find_only_one_tinue {
        true => {
            let move_options = moves.map(|IDDFSResult { depth: _, result }| {
                result
                    .first()
                    .map(|m| move_list_to_vec(get_longest_sequence(m).1))
            });
            serde_json::to_string(&move_options).unwrap()
        }
        false => {
            let move_options = moves.as_ref().map(|mvs| tinuemove_to_options(&mvs.result));
            serde_json::to_string(&move_options).unwrap()
        }
    };

    let time_taken = timer.elapsed().as_millis();
    println!(
        "{{\"id\":{}, \"size\":{}, \"result\":\"{}\", \"max-depth\":{}, \"depth\":{}, \"movesToUndo\":{}, \"timeMs\":{}, \"tinue\":{}}}",
        game.id, game.size, game.result, max_depth, actual_depth, plies_to_undo, time_taken, json_string
    );

    match actual_depth {
        0 | 1 => None, // Ignore no wins and  immediate wins
        _ => Some(TinueGameRow {
            plies_to_undo: plies_to_undo,
            gameid: game.id,
            tinue: json_string,
            size: game.size,
            tinue_depth: actual_depth,
        }),
    }
}
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
    let conn_mtx: Arc<Mutex<Connection>> = Arc::new(Mutex::new(conn));

    if !test {
        conn_mtx
            .lock()
            .unwrap()
            .execute(
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

    let conn = conn_mtx.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, notation, result, size FROM games WHERE (result = ? or result = ?) and id > ? AND size = ?")
        .unwrap();
    let gamerows = stmt
        .query_map(params!["R-0", "0-R", min_game_id - 1, board_size], |row| {
            Ok(GameRow {
                id: row.get(0)?,
                notation: row.get(1)?,
                result: row.get(2)?,
                size: row.get(3)?,
            })
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<GameRow>>();

    rayon::scope_fifo(|scope| {
        for game in gamerows.iter() {
            let conn_arc = Arc::clone(&conn_mtx);
            scope.spawn_fifo(move |_| {
                println!("# Processing game #{}", game.id);
                handle_game(&game, max_depth, plies_to_undo, !multi_tinue).and_then(|r| {
                    if test {
                        return None;
                    }

                    let local_conn = conn_arc.lock().unwrap();
                    Some(local_conn.execute("INSERT INTO tinues(gameid, size, plies_to_undo, tinue_depth, tinue) VALUES(?, ?, ?, ?, ?)", params![
                        r.gameid,
                        r.size,
                        r.plies_to_undo,
                        r.tinue_depth,
                        r.tinue
                    ]))
                });
            });
        }
    });

    return;

    /*
    P A1,P G1,P D4,P C4,P D3,P C3,P D5,P E4 C,P C5,P B5,P B4 C,P E3,P E5,M E4 D4 1,P F5,P D6,P G5,P A5,P B6,P C6,P A4,M B5 C5 1,P B5 C,M C5 D5 2,M B5 C5 1,M D4 D5 2,P D4,P E4,P F4,P E2,M C5 C4 1,M D5 D4 2,P C5 W,M D5 E5 3,P D5 W,P E6 C,M D5 E5 1,M E6 E5 1,P D5 W,P E6,P F3 W,P D2,M F3 E3 1,P E7,M C4 C3 2,P E1,P F6 W,P B1,P C1,P C2,P B2,P A2,P B5,P B7,P A7,P A6,P B3,P C7,M C3 C2 3,M A6 A7 1,P A6,M D4 D3 3,M F6 E6 1,M A5 A6 1,P D7,P A5 W,P G6,P G7,P F7,P F6,P F3,P D4,M D7 C7 1,M C6 C7 1,P D7 W,M E5 D5 1,P C6 W,M D5 E5 2,M D7 C7 1,M A5 B5 1,P D7,P D5,M C6 D6 1,P F2,P G2,P G3,P G4,M E5 F5 7,M F7 G7 1,M F5 G5 3,P E5 W,P A3,M E5 F5 1,P E5 W,M F5 F2 3 1 2,M E4 F4 1,M F3 F4 1,P E4 W,M F4 F7 1 2 2,M E5 F5 1,M E6 F6 2,P E5,P E6,P C3,M C7 A7 2 2,M B5 B6 2,P C7,M B6 B7 3,M D6 D7 2,M G5 G6 4,P A5,P B6,M A7 A6 3,M B7 A7 5,P C6,P B5,P D6,M G6 G7 5,M D7 E7 2,M G7 F7 6,M F6 E6 5,M F7 F6 7,P G5,M F6 E6 1,M F2 E2 3,M F5 F6 2,P G6
    */

    // println!("{:?}", parse_move::<5>("M A1 A3 23"));
    // println!("{:?}", parse_move::<5>("M A1 A3 23").to_string::<5>());
    // println!("Testing translation");
    // let size: usize = 5;
    // // notation for https://www.playtak.com/games/393529/ninjaviewer
    // let server_notation = "P B5,P E4,P E3,P C2,P D2 W,P D5,P E2,P E1 C,P E5,P D3,M E3 D3 1,P C4,M E4 D4 1,P D1,M D4 C4 1,P B2,M D2 D3 1,P D2,P A3 C,P B3, P A4"; //,P A2";
    // let expected_ptn = "c3 e5 c4 c3+ e3 c2 e3 Cd1 Ce1 Se4 d3 b3 d4 2c4> d3+ c4 3d4-12";

    // println!("EXP: {:?}", expected_ptn.split_whitespace().collect::<Vec<&str>>());
    // match size {
    //     3 => do_it::<3>(server_notation),
    //     4 => do_it::<4>(server_notation),
    //     5 => do_it::<5>(server_notation),
    //     6 => do_it::<6>(server_notation),
    //     7 => do_it::<7>(server_notation),
    //     8 => do_it::<8>(server_notation),
    //     s => panic!("Unsupported size {}", s),
    // };

    // println!("Enter move notation as a simple list: ");
    // println!("Example input: d3 e3 d1 d2 c1 e1 Ce2 Cc2 a3 1c2>1 a4 d4 b4 c3 c5 d5 c4");
    // let mut input = String::new();
    // io::stdin().read_line(&mut input).unwrap();
    // let input = "a1 e1 e2 a4 e3 d3 e4 e5 d5 e5- Cd4 c1 e3< 2e4-11 e3- e3"; // d2";
    // let input = "a1 e1 e2 a4 e3 d3 e4 e5 d5 e5- Cd4 c1 e3< 2e4-11 e3-"; // e3"; // d2";
    // let input = "a1 e1 e2 a4 e3 d3 e4 e5 d5 e5- Cd4 c1 e3< 2e4-11"; // e3-"; // e3"; // d2";

    // let mut position = <Board<5>>::start_board();
    // for move_string in input.split_whitespace() {
    //     let mv = position.move_from_san(move_string).unwrap();
    //     position.do_move(mv);
    // }

    // println!("Tinue 1 moves: ");
    // for mvs in win_in_one(&mut position) {
    //     for mv in mvs {
    //         println!(" {}, ", position.move_to_san(&mv));
    //     }
    // }

    // input contains a road with tinue at least for the last 3 plies
    // let input = "a1 e1 e2 a4 e3 d3 e4 e5 d5 e5- Cd4 c1 e3< 2e4-11"; // e3-"; // e3"; // d2";
    //                                                                 // https://ptn.ninja/NoZQlgXgpgBARAVjgXQLAChRgC6zgB2wDsA6IsIgKwEMUNgARa3eAJgAZWBGEzkrgOx1MAFTABbPBwBcXABzSEQtJgBKUAM4BXADbZ4qgLTthGHjGoIYAIwAsGViRgBjAMw2EGV04Amtl-botk4gPu4Awj6e6AhOUO5Q0QBsTuFQ-lGGGAK+rBasGHJO1FwuBegAnE7WeT5cZuzVpXUA1Gbm1lwtZQA8Zo42CeVc3oOGLtFcwTaliX3oXLEWXTBRAHxmKcswUKxZCzk2tt0g1q5mRTC2NWtcXHlQXWZVO67jIM716BzVeUbsQA&name=KwD2AIAoCYEog&ply=38!
    //                                                                 // Tinue starts here (7 ply deep)

    // let input = "a5 b4 c3 b5 d4 c4 Sd3 Cd5 e3 e5 Ce4 d5- d2 a2 a1 c2 b2 d1 b1 d1+ b1+ c2< b3 e2 b3- c5 b1 e5< a1+ d5> a1 e2-";
    // let input = "a5 b4 c3 b5 d4 c4 Sd3 Cd5 e3 e5 Ce4 d5- d2 a2 a1 c2 b2 d1 b1 d1+ b1+ c2< b3 e2 b3- c5 b1 e5< a1+ d5> a1 e2- b4+ Sb3 4b2>112 e1+ e3- Sc1 b2";
    // let input = "a5 b4 c3 b5 d4 c4 Sd3 Cd5 e3 e5 Ce4 d5- d2 a2 a1 c2 b2 d1 b1 d1+ b1+ c2< b3 e2 b3- c5 b1 e5< a1+ d5> a1 e2- b4+ Sb3"; // 4b2>112 e1+ e3- Sc1 b2";
    let input = "a5 b4 c3 b5 d4 c4 Sd3 Cd5 e3 e5 Ce4 d5- d2 a2 a1 c2 b2 d1 b1 d1+ b1+ c2< b3 e2 b3- c5 b1 e5< a1+ d5> a1 e2-"; // b4+ Sb3";// 4b2>112 e1+ e3- Sc1 b2";
    let input = "P A1,P D4,P D3,P E5,P D2,P B2,P D1,P A4,P D5,P D6,P C6,M E5 D5 1,M D4 D5 1,M D6 D5 1,P D6,M D5 D3 2 2,P D5,P B4,P C4,M D4 C4 2,P D4,M D3 D4 3,P D3,M D4 D3 4,M D2 D3 1,P B3 C,P D2,P D4 W,P C3 C,M D4 D3 1,M C3 D3 1,P D4 W,M D3 D4 1,M D3 D2 2,P E3,M D3 E3 5,P D3,M D2 D3 3,P D2,P A5,P A6 W,M D3 D2 4,P D3,M D2 D3 5,P D2,M D3 D2 6,M D4 D2 1 1,P D4 W,M D2 D4 5 1,M D3 D1 1 5,P D3,M D2 D3 3,P E5,P D2,P F3 W,P C1,M F3 E3 1,P C3 W,M E3 D3 6,M D1 E1 6,M D3 D1 1 4,M C3 D3 1,M D4 D3 1,P C3,P E4,M D4 D5 1,M E5 D5 1,P D4,M D1 E1 3,M C1 D1 1,M E1 E6 1 1 2 1 1,M D1 E1 2,M D5 F5 1 2,M D4 E4 1,M E3 E4 1,M C4 E4 1 2,M E5 E4 2,P C4 W,P D5,M C4 D4 1,M D3 D1 1 5,M E1 E3 1 3,M E2 E3 1,M D4 D6 1 1,M E4 A4 2 1 2 1,M B4 E4 1 1 1,M E3 E5 1 3,M D4 E4 2,M D5 D4 1,M E4 D4 3,P C5,M E4 E2 2 2,P B5,M D4 D5 4,M E5 D5 2,M D6 D5 2,M A4 A5 2,M D5 F5 1 3,M D1 D5 1 2 1 1,M C4 C5 2,M C6 C5 1,P B1,M D3 D4 3,P E4,M D5 E5 5,M E4 D4 1,M D2 D5 2 1 1,P E4 W,M E5 F5 1,M E5 A5 1 1 1 3,M D5 A5 1 1 1,P D5 W,M F5 D5 3 1,M E4 D4 1,M D5 D4 1,M D5 C5 1,M B5 E5 1 1 1,M E3 E5 1 2,P F4,M E5 E3 3 3,"; // M C5 F5 2 2 2,M E4 E5 4,M C5 E5 1 1,M E3 D3 3,M A5 C5 1 3";
    let input = "P A1,P D4,P D3,P E5,P D2,P B2,P D1,P A4,P D5,P D6,P C6,M E5 D5 1,M D4 D5 1,M D6 D5 1,P D6,M D5 D3 2 2,P D5,P B4,P C4,M D4 C4 2,P D4,M D3 D4 3,P D3,M D4 D3 4,M D2 D3 1,P B3 C,P D2,P D4 W,P C3 C,M D4 D3 1,M C3 D3 1,P D4 W,M D3 D4 1,M D3 D2 2,P E3,M D3 E3 5,P D3,M D2 D3 3,P D2,P A5,P A6 W,M D3 D2 4,P D3,M D2 D3 5,P D2,M D3 D2 6,M D4 D2 1 1,P D4 W,M D2 D4 5 1,M D3 D1 1 5,P D3,M D2 D3 3,P E5,P D2,P F3 W,P C1,M F3 E3 1,P C3 W,M E3 D3 6,M D1 E1 6,M D3 D1 1 4,M C3 D3 1,M D4 D3 1,P C3,P E4,M D4 D5 1,M E5 D5 1,P D4,M D1 E1 3,M C1 D1 1,M E1 E6 1 1 2 1 1,M D1 E1 2,M D5 F5 1 2,M D4 E4 1,M E3 E4 1,M C4 E4 1 2,M E5 E4 2,P C4 W,P D5,M C4 D4 1,M D3 D1 1 5,M E1 E3 1 3,M E2 E3 1,M D4 D6 1 1,M E4 A4 2 1 2 1,M B4 E4 1 1 1,M E3 E5 1 3,M D4 E4 2,M D5 D4 1,M E4 D4 3,P C5,M E4 E2 2 2,P B5,M D4 D5 4,M E5 D5 2,M D6 D5 2,M A4 A5 2,M D5 F5 1 3,M D1 D5 1 2 1 1,M C4 C5 2,M C6 C5 1,P B1,M D3 D4 3,P E4,M D5 E5 5,M E4 D4 1,M D2 D5 2 1 1,P E4 W,M E5 F5 1,M E5 A5 1 1 1 3,M D5 A5 1 1 1,P D5 W,M F5 D5 3 1,M E4 D4 1,M D5 D4 1,M D5 C5 1,M B5 E5 1 1 1,M E3 E5 1 2,P F4,M E5 E3 3 3, P B5";
    let plies_till_tinue = 4;
    let mut position = <Board<6>>::start_board();
    // for move_string in input.split_whitespace() {
    //     println!("INIT MV {}", move_string);
    //     let mv = position.move_from_san(move_string).unwrap();
    //     position.do_move(mv);
    // }
    for mv in parse_server_notation::<6>(input) {
        position.do_move(mv);
    }
    let me = Color::White; // position.side_to_move();
    println!("\n// Tinue in up to {} plies as {}: ", plies_till_tinue, me);
    let winning_moves = win_in_n(&mut position, plies_till_tinue, me, false);

    print_tinue_moves_json(&winning_moves);
    println!(
        "{}",
        serde_json::to_string(&tinuemove_to_options(&winning_moves)).unwrap()
    );
}

type Mov = String;

/// A structure that summarises multiple `TinueMove`s into a single `struct`.
///
/// If the winning replies (called `solutions`) to a set of `moves` are the same
/// then they are grouped into a single `TinueMoveOptions` instance.
/// # TLDR
/// Basically: If **Player A** plays one of `moves`, then **Player B** must play
///            one of `solutions` to stay on the **Road to Tinue**.
#[derive(Serialize, Debug, Eq, PartialEq, Clone)]
struct TinueMoveOptions {
    /// Possible moves
    moves: Vec<Mov>,
    /// Responses applicable to any of `moves` that stay on the **Road to Tinue**.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    solutions: Vec<TinueMoveOptions>,
}

/// Reduces `TinueMove`s to `TinueMoveOption`s
fn tinuemove_to_options(tmvs: &Vec<TinueMove>) -> Vec<TinueMoveOptions> {
    let trs: Vec<(Mov, Option<Vec<TinueMoveOptions>>)> = tmvs
        .iter()
        .map(|tm| {
            (
                tm.mv.clone(),
                match &tm.next {
                    Some(next) => Some(tinuemove_to_options(&next)),
                    None => None,
                },
            )
        })
        .collect();

    let groups = trs.group_by(|(_, next1), (_, next2)| next1 == next2);

    groups
        .map(|group| TinueMoveOptions {
            moves: group
                .iter()
                .map(|(mv, _)| mv.clone())
                .collect::<Vec<String>>(),
            solutions: match group.first() {
                None => vec![],
                Some((_, solution)) => solution.clone().unwrap_or(vec![]),
            },
        })
        .collect()
}

/// Concatenates the List into a vector
fn move_list_to_vec(mv: MoveListNode) -> Vec<Mov> {
    if let Some(next) = mv.next {
        let mut list = move_list_to_vec(*next);
        list.insert(0, mv.mv);
        return list;
    }
    return vec![mv.mv];
}

struct MoveListNode {
    mv: Mov,
    next: Option<Box<MoveListNode>>,
}
/// Returns a longest **Road to Tinue**.
///
/// This is probably a road where the opponent defends *fairly* well (that's hard to measure because it's a Tinue and no move is an effective defense)
fn get_longest_sequence(tinue_move: &TinueMove) -> (usize, MoveListNode) {
    if let Some(next) = &tinue_move.next {
        let longest_sequence = next
            .iter()
            .map(|n| get_longest_sequence(n))
            .max_by_key(|(depth, _)| *depth);

        if let Some((depth, next_move)) = longest_sequence {
            return (
                depth + 1,
                MoveListNode {
                    mv: tinue_move.mv.clone(),
                    next: Some(Box::new(next_move)),
                },
            );
        }
    }

    return (
        1,
        MoveListNode {
            mv: tinue_move.mv.clone(),
            next: None,
        },
    );
}

/// Represents a `Move` on the **Road to Tinue** and possible responses (`next`)
#[derive(Debug)]
struct TinueMove {
    mv: Mov,
    /// When `mv` is played, any of these responses will stay on the **Road to Tinue**
    next: Option<Vec<TinueMove>>,
}

/// Prints `TinueMove`s as a JSON object.
/// ### Example
/// ```
/// {
/// "b4+": {
///   "4b2<": {
///     "a5": { "4a2+112": {}, },
///     "Sa5": { "5a2>1112": {}, },
/// ...
/// ```
fn print_tinue_moves_json(moves: &Vec<TinueMove>) {
    fn print_tinue_move(mv: &TinueMove, depth: usize) {
        println!("{}\"{}\": {{", "  ".repeat(depth), mv.mv);
        if let Some(next) = &mv.next {
            for next_move in next {
                print_tinue_move(&next_move, depth + 1);
            }
        }
        println!("{}}},", "  ".repeat(depth))
    }

    println!("{{");
    for mv in moves {
        print_tinue_move(mv, 1);
    }
    println!("}}");
}

struct IDDFSResult<T> {
    depth: u32,
    result: T,
}

/// Returns the resulting tinue and the maximum length of it
fn iddf_tinue_search<const S: usize>(
    position: &mut Board<S>,
    max_depth: u32,
    me: Color,
    find_only_one_tinue: bool,
) -> Option<IDDFSResult<Vec<TinueMove>>> {
    for depth in (1..(max_depth + 1)).step_by(2) {
        let result = win_in_n(position, depth, me, find_only_one_tinue);
        if result.len() > 0 {
            return Some(IDDFSResult { depth, result });
        }
    }

    None
}

/// Recursive with `win_in_n` to explore each branch breadth first via IDDFS,
/// thus removing Tinues drawn in the length by both players making moves
/// that don't affect the Tinue.
fn iddf_win_in_n<const S: usize>(
    position: &mut Board<S>,
    max_depth: u32,
    me: Color,
    find_only_one_tinue: bool,
) -> Vec<TinueMove> {
    for depth in 1..(max_depth + 1) {
        let result = win_in_n(position, depth, me, find_only_one_tinue);
        if result.len() > 0 {
            return result;
        }
    }
    return vec![];
}

/// Returns all **Roads to Tinue** for player `me`
/// that are available at `position`.
///
/// `depth`: Number of plies to look into the future
///
/// `find_only_one_tinue`: If `true` returns only the first **Road to Tinue**
///
/// #### Remarks
/// If `depth` is high, this may still return sub-optimal Tinues
/// where the opponent is not blocking the Tinue and thus `me` can waste another ply.
///
/// #### Caution
/// This method becomes very slow very quickly. `depth=5`, maybe `7` is recommended.
/// Best to set `find_only_one_tinue=true`.
fn win_in_n<const S: usize>(
    position: &mut Board<S>,
    depth: u32,
    me: Color,
    find_only_one_tinue: bool,
) -> Vec<TinueMove> {
    let mut legal_moves = vec![];
    let mut tinue_moves = vec![];

    position.generate_moves(&mut legal_moves);

    let my_turn = position.side_to_move() == me;

    for mv in legal_moves {
        let reverse_move = position.do_move(mv.clone());
        if let Some(result) = position.game_result() {
            // Early win or loss
            if my_turn {
                if result == GameResult::WhiteWin && me == Color::White
                    || result == GameResult::BlackWin && me == Color::Black
                {
                    if find_only_one_tinue {
                        position.reverse_move(reverse_move);
                        return vec![TinueMove {
                            mv: position.move_to_san(&mv),
                            next: None,
                        }];
                    }
                    tinue_moves.push(TinueMove {
                        mv: position.move_to_san(&mv),
                        next: None,
                    })
                }
            } else {
                position.reverse_move(reverse_move);

                if result == GameResult::WhiteWin && me == Color::White
                    || result == GameResult::BlackWin && me == Color::Black
                {
                    // Win for me, but given to me by the opponent
                    continue;
                }

                // Early loss or draw
                // TODO: Actually, this could be an early road/flatwin if that's the only possible enemy move
                //       So we should add checks for that
                return vec![];
            }
        } else if depth > 1 {
            let winning_moves = iddf_win_in_n(position, depth - 1, me, find_only_one_tinue);
            if my_turn {
                // I play
                if !winning_moves.is_empty() {
                    // This move leads to Tinue
                    let this_move = TinueMove {
                        mv: position.move_to_san(&mv),
                        next: Some(winning_moves),
                    };

                    if find_only_one_tinue {
                        position.reverse_move(reverse_move);
                        return vec![this_move];
                    }
                    tinue_moves.push(this_move)
                }
            } else {
                // opponent plays
                if winning_moves.is_empty() {
                    // Because the opponet play `mv` doesn't lead to Tinue,
                    // this entire branch is not on the road to Tinue.
                    position.reverse_move(reverse_move);
                    return vec![];
                }
                // This and the previous opponent moves are on the road to Tinue so add it
                let this_move = TinueMove {
                    mv: position.move_to_san(&mv),
                    next: Some(winning_moves),
                };
                tinue_moves.push(this_move)
            }
        }
        position.reverse_move(reverse_move);
    }

    tinue_moves
}
