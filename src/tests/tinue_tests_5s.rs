use crate::tests::run_tinue_test;

#[test]
fn tinue_test() {
    // a1 a5 b5 Cc3 c5 d5 Cd4 c4 e5 1c4+1 1d4+1 c4 1d5<1 1d5>1 d5 e4 2c5>11 1d5<1 2e5<11 2d5>2
    let move_strings = [
        "a1", "a5", "b5", "Cc3", "c5", "d5", "Cd4", "c4", "e5", "1c4+1", "1d4+1", "c4", "1d5<1",
        "1d5>1", "d5", "e4", "2c5>11", "1d5<1", "2e5<11", "2d5>2",
    ];

    run_tinue_test::<5>(3, &move_strings, &"2c5>11");
}

#[test]
fn tinue_test2() {
    let move_strings = [
        "a5", "e4", "Cc3", "c4", "b3", "Cd3", "b4", "b5", "d4", "d5", "a4", "c4>", "e4<", "d3+",
        "e3", "d3", "d2", "4d4<22", "a3", "3b4-", "c5", "2c4+", "a4+", "b2", "b4", "c4", "b1",
        "c2", "c1", "d1", "d4", "a2", "a4", "e2", "d2<", "c4<", "a4>", "d2", "c4", "b2>", "c1+",
        "b5-", "4c2>22", "4b4>22", "c3+", "d3-", "3c4>", "3d2>", "4d4-22", "5e2+122", "d4>",
        "2e3+", "d4>", "2e5-", "d4", "3e4<", "e3", "c2", "a4", "e1", "e3+", "4d4>", "a1", "a2+",
        "a2",
    ];

    run_tinue_test::<5>(3, &move_strings, &"5e4+");
}

#[test]
fn tinue_test3() {
    // b4 a5 e5 b5 b3 Cc3 Cc5 d5 d4 d3 b3+ a4 2b4+ a4+ a4 b4 d4+ b4< b4 a3 b3 a2 3b5< 2a4+ Sa4 b2 e3 e2 a4+ d2 5a5-122 3a5-21 3a2+ c2 5a3- c3< 5a2>113 2a4- a5 Sb5 d4 c3 e3< c3> 4d2< e3 Sc3 3d3+12 c5> e4 5d5> c4 5e5-212 2d4> e3+ e1
    let move_strings = [
        "b4", "a5", "e5", "b5", "b3", "Cc3", "Cc5", "d5", "d4", "d3", "b3+", "a4", "2b4+", "a4+",
        "a4", "b4", "d4+", "b4<", "b4", "a3", "b3", "a2", "3b5<", "2a4+", "Sa4", "b2", "e3", "e2",
        "a4+", "d2", "5a5-122", "3a5-21", "3a2+", "c2", "5a3-", "c3<", "5a2>113", "2a4-", "a5",
        "Sb5", "d4", "c3", "e3<", "c3>", "4d2<", "e3", "Sc3", "3d3+12", "c5>", "e4", "5d5>", "c4",
        "5e5-212", "2d4>", "e3+", "e1",
    ];

    run_tinue_test::<5>(3, &move_strings, &"2e2+11");
}

#[test]
fn tinue_test4() {
    // e1 e5 Cc3 c1 d1 d2 a3 b1 b3 d2- a1 a2 a1> Cb2 Sc2 a1 2b1> b2+ b5 b1 c4 d2 c5
    let move_strings = [
        "e1", "e5", "Cc3", "c1", "d1", "d2", "a3", "b1", "b3", "d2-", "a1", "a2", "a1>", "Cb2",
        "Sc2", "a1", "2b1>", "b2+", "b5", "b1", "c4", "d2", "c5",
    ];
    run_tinue_test::<5>(5, &move_strings, &"2b3-11");
}

#[test]
fn tinue_test5() {
    // c4 a5 e1 c3 d1 c2 c1 b1 Cb2 c5 b2- a1 a2 c2- c2 2c1> d2 Cb2 c1 b2> d2- 2c2- c2 3c1> b2 d3 Sd2 c1 a3 a1+ a3-
    let move_strings = [
        "c4", "a5", "e1", "c3", "d1", "c2", "c1", "b1", "Cb2", "c5", "b2-", "a1", "a2", "c2-",
        "c2", "2c1>", "d2", "Cb2", "c1", "b2>", "d2-", "2c2-", "c2", "3c1>", "b2", "d3", "Sd2",
        "c1", "a3", "a1+", "a3-",
    ];
    run_tinue_test::<5>(5, &move_strings, &"d1<");
}
