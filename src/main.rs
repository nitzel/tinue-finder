use arrayvec::ArrayVec;
use board_game_traits::board::{Board as BoardTrait, Color, GameResult};
use pgn_traits::pgn::PgnBoard;
use std::{cmp::Ordering, io, iter, str::FromStr};
use taik::{board, board::{Board, Direction, Move, Movement, Role, StackMovement}};

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

        let pieces_taken: ArrayVec<[Movement; board::MAX_BOARD_SIZE - 1]> = iter::once(num_pieces_taken)
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

fn do_it<const S: usize>(server_notation: &str) {
    let moves = parse_server_notation::<S>(server_notation);
    println!("SRV: {}", server_notation);
    println!("PTN: {:?}", moves.iter().map(|m| m.to_string::<S>()).collect::<Vec<String>>());

    // Apply moves
    let mut position = Board::<S>::start_board();
    for (_i, ply) in moves.iter().enumerate() {
        println!("Ply{} {}",_i, ply.to_string::<S>());
        position.do_move(ply.clone());
    }

    print!("Tinue moves: ");
    for mvs in win_in_one(&mut position) {
        for mv in mvs {
            print!("{}, ", position.move_to_san(&mv));
        }
    }
    println!("");
}

fn main() {
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
    let input = "a1 e1 e2 a4 e3 d3 e4 e5 d5 e5- Cd4 c1 e3< 2e4-11"; // e3-"; // e3"; // d2";
    let moves_till_tinue = 3;
    let mut position = <Board<5>>::start_board();
    for move_string in input.split_whitespace() {
        let mv = position.move_from_san(move_string).unwrap();
        position.do_move(mv);
    }
    println!("\nTinue in up to {} plies: ", moves_till_tinue);
    for mvs in win_in_n(&mut position, moves_till_tinue) {
        for mv in mvs {
            print!("{}  ", position.move_to_san(&mv));
        }
        println!("");
    }
    println!();
}

/// Returns every move that immediately wins the game
fn win_in_one<const S: usize>(position: &mut Board<S>) -> Vec<Vec<Move>> {
    let mut legal_moves = vec![];
    let mut tinue_moves = vec![];

    position.generate_moves(&mut legal_moves);

    for mv in legal_moves {
        let reverse_move = position.do_move(mv.clone());
        if let Some(result) = position.game_result() {
            if result == GameResult::WhiteWin && position.side_to_move() == Color::Black
                || result == GameResult::BlackWin && position.side_to_move() == Color::White
            {
                tinue_moves.push(vec![mv]);
            }
        }
        position.reverse_move(reverse_move);
    }

    tinue_moves
}

fn win_in_n<const S: usize>(position: &mut Board<S>, depth: u32) -> Vec<Vec<Move>> {
    if depth == 1 {
        return win_in_one(position);
    }

    let mut legal_moves = vec![];
    let mut tinue_moves = vec![];

    position.generate_moves(&mut legal_moves);

    for mv in legal_moves {
        let me_plays = depth % 2 == 1;
        let reverse_move = position.do_move(mv.clone());
        // Early win or loss
        if let Some(result) = position.game_result() {
            if me_plays
                && (result == GameResult::WhiteWin && position.side_to_move() == Color::Black
                    || result == GameResult::BlackWin && position.side_to_move() == Color::White)
            {
                tinue_moves.push(vec![mv]);
            }
        }
        else {
            let mut winning_moves = win_in_n(position, depth - 1);
            match me_plays {
                false => { // even - opponent plays
                    if winning_moves.is_empty() {
                        position.reverse_move(reverse_move);
                        return vec![]; // abort if not all of them are succesfull
                    }
                    for wmvs in &mut winning_moves {
                        wmvs.insert(0, mv.clone());
                        tinue_moves.push(wmvs.clone());
                    }
                },
                true => { // odd - I play
                    if !winning_moves.is_empty() {
                        for mvs in &mut winning_moves {
                            mvs.insert(0, mv.clone());
                            tinue_moves.push(mvs.clone());
                        }
                    }
                }
            }
        }
        position.reverse_move(reverse_move);
    }

    tinue_moves
}
