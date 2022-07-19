#![feature(assert_matches)]

use std::assert_matches::assert_matches;

use dissat::{Result, Solver};

#[test]
fn decision_test() {
    let mut solver = Solver::from_dimacs(include_str!(
        "../cnf_examples/2_2_trivial_decision_and_prop.dimacs"
    ))
    .unwrap();
    let model = solver.solve().unwrap_sat();
    assert!(model.lit(2))
}

#[test]
fn propagation_test() {
    let mut solver =
        Solver::from_dimacs(include_str!("../cnf_examples/3_3_propagation.dimacs")).unwrap();
    assert_matches!(solver.solve(), Result::Sat(model) if model.as_vec() == vec![-1, -2, 3])
}
