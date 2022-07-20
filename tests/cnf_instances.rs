#![feature(once_cell)]
#![feature(custom_test_frameworks)]
#![test_runner(datatest::runner)]

use std::io::Write;

use dissat::Solver;
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

fn assert_match_expected(result: dissat::Result, input: &str) {
    let header = input.lines().next().expect("Empty input").trim();
    match header {
        "c SAT" => assert!(result.is_sat(), "Expected satisfiable solution"),
        "c UNSAT" => assert!(result.is_unsat(), "Expected unsatisfiable solution"),
        _ => panic!("Invalid header"),
    }
}

#[datatest::files("cnf_examples", {
  input in "^.*",
})]
fn example(input: &str) {
    set_up_log();
    let mut solver = Solver::from_dimacs(input).expect("Malformed dimacs format.");
    assert_match_expected(solver.solve(), input);
}
