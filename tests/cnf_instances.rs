#![feature(assert_matches)]
#![feature(once_cell)]

use std::assert_matches::assert_matches;

use dissat::{Result, Solver};

fn set_up_log() {
    use std::sync::LazyLock;
    use tracing::metadata::LevelFilter;
    use tracing_subscriber::fmt;

    static LOG_SET_UP: LazyLock<()> = LazyLock::new(|| {
        let fmt = fmt::format().without_time().compact();

        tracing_subscriber::fmt()
            .with_max_level(LevelFilter::TRACE)
            .event_format(fmt)
            .init();

        tracing::info!("setup subscriber");

        ()
    });

    LazyLock::force(&LOG_SET_UP);
}

#[test]
fn decision_test() {
    set_up_log();

    let mut solver = Solver::from_dimacs(include_str!(
        "../cnf_examples/2_2_trivial_decision_and_prop.dimacs"
    ))
    .unwrap();
    let model = solver.solve().unwrap_sat();
    assert!(model.lit(2))
}

#[test]
fn propagation_test() {
    set_up_log();

    let mut solver =
        Solver::from_dimacs(include_str!("../cnf_examples/3_3_propagation.dimacs")).unwrap();
    assert_matches!(solver.solve(), Result::Sat(model) if model.as_vec() == vec![-1, -2, 3])
}

#[test]
fn other_test() {
    set_up_log();

    let mut solver =
        Solver::from_dimacs(include_str!("../cnf_examples/10_44_fla-010-0.dimacs")).unwrap();
    assert!(solver.solve().is_sat());
}
