use board_game_traits::board::{Board as BoardTrait, Color, GameResult};
use pgn_traits::pgn::PgnBoard;
use std::io;
use taik::board::{Board, Move};

fn main() {
    println!("Enter move notation as a simple list: ");
    println!("Example input: d3 e3 d1 d2 c1 e1 Ce2 Cc2 a3 1c2>1 a4 d4 b4 c3 c5 d5 c4");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let mut position = <Board<5>>::start_board();
    for move_string in input.split_whitespace() {
        let mv = position.move_from_san(move_string).unwrap();
        position.do_move(mv);
    }
    print!("Tinue moves: ");
    for mv in win_in_one(&mut position) {
        print!("{}, ", position.move_to_san(&mv));
    }
    println!();
}

/// Returns every move that immediately wins the game
fn win_in_one<const S: usize>(position: &mut Board<S>) -> Vec<Move> {
    let mut legal_moves = vec![];
    let mut tinue_moves = vec![];

    position.generate_moves(&mut legal_moves);

    for mv in legal_moves {
        let reverse_move = position.do_move(mv.clone());
        if let Some(result) = position.game_result() {
            if result == GameResult::WhiteWin && position.side_to_move() == Color::Black
                || result == GameResult::BlackWin && position.side_to_move() == Color::White
            {
                tinue_moves.push(mv);
            }
        }
        position.reverse_move(reverse_move);
    }

    tinue_moves
}
