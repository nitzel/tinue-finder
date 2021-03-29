use crate::tinue_search::NodeValue::*;
use crate::MoveString;
use board_game_traits::{Color::*, GameResult::*, Position as PositionTrait};
use pgn_traits::PgnPosition;
use std::cmp::Ordering;
use tiltak::position::{Move, Position, TunableBoard};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum NodeValue {
    WinInPly(u32),
    Unknown,
    LossInPly(u32),
}

impl Ord for NodeValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (WinInPly(a), WinInPly(b)) => a.cmp(b).reverse(),
            (WinInPly(_), _) => Ordering::Greater,
            (Unknown, WinInPly(_)) => Ordering::Less,
            (Unknown, Unknown) => Ordering::Equal,
            (Unknown, LossInPly(_)) => Ordering::Greater,
            (LossInPly(a), LossInPly(b)) => a.cmp(b),
            (LossInPly(_), _) => Ordering::Less,
        }
    }
}

impl PartialOrd for NodeValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl NodeValue {
    const MAX_VALUE: Self = WinInPly(0);
    const MIN_VALUE: Self = LossInPly(0);

    fn propagate_up(self) -> Self {
        match self {
            WinInPly(n) => LossInPly(n + 1),
            Unknown => Unknown,
            LossInPly(n) => WinInPly(n + 1),
        }
    }

    fn propagate_down(self) -> Self {
        match self {
            WinInPly(n) => LossInPly(n.saturating_sub(1)),
            Unknown => Unknown,
            LossInPly(n) => WinInPly(n.saturating_sub(1)),
        }
    }
}

pub fn generate_sorted_moves<const S: usize>(position: &Position<S>) -> Vec<(Move, f32)> {
    let mut moves = vec![];
    let mut moves_with_heuristic_scores = vec![];

    position.generate_moves_with_probabilities(
        &position.group_data(),
        &mut moves,
        &mut moves_with_heuristic_scores,
    );

    // Sort the moves using Tiltak's heuristic
    // Checking the best moves first gives a ~35% speedup for depth 5
    moves_with_heuristic_scores
        .sort_unstable_by(|(_, score1), (_, score2)| score1.partial_cmp(score2).unwrap().reverse());
    moves_with_heuristic_scores
}

/// The core alpha-beta function. Search for any tinue up to `depth`, within the `alpha` and `beta` bounds.
///
/// # Arguments
///
/// * `alpha` A lower bound on the result. We already know that we can achieve this score, so we will not look for lines that cannot improve on this
/// * `beta` An upper bound on the result. We already know that we cannot do better than this, so we will not look for lines that improve on this
pub fn alpha_beta<const S: usize>(
    position: &mut Position<S>,
    depth: u32,
    mut alpha: NodeValue,
    beta: NodeValue,
) -> NodeValue {
    let game_result = position.game_result();
    if depth == 0 || game_result.is_some() {
        match (position.side_to_move(), game_result) {
            (Black, Some(WhiteWin)) => LossInPly(0),
            (White, Some(WhiteWin)) => WinInPly(0),
            (Black, Some(BlackWin)) => WinInPly(0),
            (White, Some(BlackWin)) => LossInPly(0),
            _ => Unknown,
        }
    } else {
        let moves_with_heuristic_scores = generate_sorted_moves(position);

        let mut value = NodeValue::MIN_VALUE;
        for (mv, _score) in moves_with_heuristic_scores {
            let reverse_move = position.do_move(mv);
            value = value.max(
                alpha_beta(
                    position,
                    depth - 1,
                    beta.propagate_down(),
                    alpha.propagate_down(),
                )
                .propagate_up(),
            );
            position.reverse_move(reverse_move);
            alpha = alpha.max(value);
            if alpha >= beta {
                break;
            }
        }
        value
    }
}

enum TinueResult {
    None,
    Tinue(Move),
    Multiple,
}

/// Returns a tinue move for a certain depth *if it is unique*
/// Returns `TinueResult::None` if no tinue is found, `TinueResult::Multiple` if many are found
fn find_unique_tinue_for_depth<const S: usize>(
    position: &mut Position<S>,
    depth: u32,
) -> TinueResult {
    let moves = generate_sorted_moves(position);
    let mut tinue_move: Option<Move> = None;
    for (mv, _score) in moves {
        let reverse_move = position.do_move(mv.clone());
        let result = alpha_beta(
            position,
            depth - 1,
            NodeValue::WinInPly(0).propagate_down(),
            NodeValue::WinInPly(depth + 1).propagate_down(),
        );
        if matches!(result, LossInPly(_)) {
            if tinue_move.is_some() {
                return TinueResult::Multiple;
            } else {
                tinue_move = Some(mv);
            }
        }
        position.reverse_move(reverse_move);
    }
    if let Some(mv) = tinue_move {
        TinueResult::Tinue(mv)
    } else {
        TinueResult::None
    }
}

pub fn best_move<const S: usize>(position: &mut Position<S>, depth: u32) -> Move {
    let moves = generate_sorted_moves(position);

    let mut best_move = moves[0].clone().0;
    let mut best_score = NodeValue::MIN_VALUE;

    for (mv, _score) in moves {
        let reverse_move = position.do_move(mv.clone());
        let score = alpha_beta(
            position,
            depth - 1,
            NodeValue::MIN_VALUE,
            NodeValue::MAX_VALUE,
        )
        .propagate_up();
        position.reverse_move(reverse_move);
        if score > best_score {
            best_move = mv;
            best_score = score;
        }
    }
    best_move
}

/// Reconstruct the principal variation of a tinue sequence of `depth`
/// This is done in a separate function from `find_unique_tinue`, for performance and
/// code simplicity reasons
pub fn pv<const S: usize>(mut position: Position<S>, mut depth: u32) -> Vec<MoveString> {
    let mut pv = vec![];
    while position.game_result().is_none() {
        assert!(depth > 0);
        let best_move = best_move(&mut position, depth);
        pv.push(position.move_to_san(&best_move));
        position.do_move(best_move.clone());
        depth -= 1;
    }
    pv
}

/// Returns a tinue move and a depth, if the move is unique at that depth
/// If multiple tinue moves are found at a certain depth, returns `None`.
/// If no tinue moves are found at any depth, returns `None`
pub fn find_unique_tinue<const S: usize>(
    position: &mut Position<S>,
    max_depth: u32,
) -> Option<(Move, u32)> {
    for depth in 1..=max_depth {
        match find_unique_tinue_for_depth(position, depth) {
            TinueResult::None => continue,
            TinueResult::Tinue(mv) => return Some((mv, depth)),
            TinueResult::Multiple => return None,
        }
    }
    None
}
