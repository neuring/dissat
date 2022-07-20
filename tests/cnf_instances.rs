#![feature(assert_matches)]
#![feature(once_cell)]
//#![feature(custom_test_frameworks)]
//#![test_runner(datatest::runner)]

use std::{assert_matches::assert_matches, io::Write};

use dissat::{Result, Solver};
use tracing_subscriber::fmt::MakeWriter;

fn set_up_log() {
    use std::sync::LazyLock;
    use tracing::metadata::LevelFilter;

    // hack: Create my own Writer that always uses print (more importantly print_to underneath)
    // so that tests capture the logging output.
    struct MyWriter;

    impl Write for MyWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let s = std::str::from_utf8(buf).unwrap();
            print!("{s}");
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for MyWriter {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            MyWriter
        }
    }

    static LOG_SET_UP: LazyLock<()> = LazyLock::new(|| {
        tracing_subscriber::fmt()
            .with_writer(MyWriter)
            .with_max_level(LevelFilter::TRACE)
            .without_time()
            .compact()
            .init();

        tracing::info!("setup subscriber");
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
    assert!(model.lit(2));
    print!("HI");
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
