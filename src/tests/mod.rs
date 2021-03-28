use crate::alpha_beta::find_unique_tinue;
use board_game_traits::Position as PositionTrait;
use pgn_traits::PgnPosition;
use tiltak::position::{Move, Position};

mod alpha_beta_tests;
mod tinue_tests_5s;
mod tinue_tests_6s;

// Runs a tinue test with a single solution
fn run_tinue_test<const S: usize>(depth: u32, move_strings: &[&str], answer_move_string: &str) {
    let mut position: Position<S> = Position::start_position();
    // Check that the moves are legal
    for move_string in move_strings {
        let mv = position.move_from_san(move_string).unwrap();
        let mut legal_moves = vec![];
        position.generate_moves(&mut legal_moves);
        assert!(legal_moves.contains(&mv));
        position.do_move(mv);
    }

    let answer_move: Move = position.move_from_san(answer_move_string).unwrap();
    let mut legal_moves = vec![];
    position.generate_moves(&mut legal_moves);
    assert!(legal_moves.contains(&answer_move));

    assert_eq!(
        find_unique_tinue(&mut position, depth),
        Some((answer_move, depth))
    );
}
