use crate::alpha_beta::{alpha_beta, NodeValue};
use crate::iddf_tinue_search;
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

    let side_to_move = position.side_to_move();

    // Check that there is no tinue on depth - 2
    let shallow_depth_result = iddf_tinue_search(&mut position, depth - 2, side_to_move, false);
    assert!(shallow_depth_result.is_none());

    // Check that the tinue solution is correct and unique
    let result = iddf_tinue_search(&mut position, depth, side_to_move, false).unwrap();
    assert_eq!(result.result.len(), 1);
    assert_eq!(result.result[0].mv, answer_move.to_string::<S>());

    let negamax_result = alpha_beta::<S>(
        &mut position,
        depth,
        NodeValue::WinInPly(depth),
        NodeValue::WinInPly(0),
    );
    assert_eq!(negamax_result, NodeValue::WinInPly(depth));
}
