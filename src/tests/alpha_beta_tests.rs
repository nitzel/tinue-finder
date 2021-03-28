use crate::alpha_beta::NodeValue::*;

#[test]
fn node_values_sorting_test() {
    let mut node_values = vec![
        LossInPly(9),
        Unknown,
        WinInPly(2),
        WinInPly(4),
        LossInPly(3),
    ];
    node_values.sort_unstable();
    assert_eq!(
        node_values,
        vec![
            LossInPly(3),
            LossInPly(9),
            Unknown,
            WinInPly(4),
            WinInPly(2)
        ]
    );
}
