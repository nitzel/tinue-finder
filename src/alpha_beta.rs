use crate::alpha_beta::NodeValue::*;
use board_game_traits::{Color::*, GameResult::*, Position as PositionTrait};
use std::cmp::Ordering;
use tiltak::position::Position;

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
        let mut moves = vec![];
        position.generate_moves(&mut moves);
        let mut value = NodeValue::MIN_VALUE;
        for mv in moves {
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
        alpha
    }
}
