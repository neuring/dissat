#![feature(once_cell)]
#![feature(custom_test_frameworks)]
#![test_runner(datatest::runner)]

use std::{
    io::Write,
    path::{Path, PathBuf},
};

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
    });

    LazyLock::force(&LOG_SET_UP);
}

fn formula_too_complex(path: &Path) -> bool {
    assert!(path.is_file());
    assert!(path.extension().unwrap() == "cnf");

    let content = std::fs::read_to_string(path).unwrap();
    let header = content.lines().find(|l| l.starts_with('p')).unwrap();

    let header = header.trim().split_whitespace().collect::<Vec<_>>();

    // p cnf <variables> <clauses>
    let clauses = header[3].parse::<u32>().unwrap();
    clauses > 800
}

enum ExpectedSolution {
    Sat(Vec<i32>),
    Unsat,
}

fn extract_solution(content: &str) -> ExpectedSolution {
    let header = content
        .lines()
        .find(|line| line.trim().starts_with('s'))
        .unwrap()
        .trim()
        .trim_start_matches('s')
        .trim();

    let expect_sat = header == "SATISFIABLE";

    if expect_sat {
        let mut solution = content
            .lines()
            .filter(|line| line.trim().starts_with('v'))
            .map(|line| line.trim().trim_start_matches('v').trim())
            .flat_map(|line| {
                line.split_whitespace()
                    .map(|lit| lit.parse::<i32>().unwrap())
                    .filter(|lit| *lit != 0)
            })
            .collect::<Vec<i32>>();
        solution.sort_by_key(|lit| lit.abs());
        ExpectedSolution::Sat(solution)
    } else {
        ExpectedSolution::Unsat
    }
}

fn find_solution_path(input: &Path) -> Option<PathBuf> {
    assert_eq!(input.extension().unwrap(), "cnf");
    let mut solution_file = PathBuf::from(input.file_stem().unwrap());
    solution_file.push(".sol");

    solution_file.try_exists().unwrap().then_some(solution_file)
}

#[datatest::files("cnf_examples", {
  input in r"^(.*).cnf" if !formula_too_complex,
})]
fn example(input: &Path) {
    set_up_log();

    let input_str = std::fs::read_to_string(input).unwrap();
    let mut solver = Solver::from_dimacs(&input_str).expect("Malformed dimacs format.");
    let result = solver.solve();

    if let Some(output) = find_solution_path(input) {
        let output_str = std::fs::read_to_string(output).unwrap();
        let expected = extract_solution(&output_str);
        match (result, expected) {
            (dissat::Result::Sat(result_model), ExpectedSolution::Sat(expected_model)) => {
                assert_eq!(result_model.as_vec(), expected_model)
            }
            (dissat::Result::Sat(_), ExpectedSolution::Unsat) => {
                panic!("Result is SAT, but expected UNSAT")
            }
            (dissat::Result::Unsat(_), ExpectedSolution::Sat(_)) => {
                panic!("Result is UNSAT, but expected SAT")
            }
            (dissat::Result::Unsat(_), ExpectedSolution::Unsat) => {}
        }
    }

    tracing::info!(
        "finished: propagations = {}, contradictions = {}",
        solver.stats().propagations,
        solver.stats().contradictions
    );
}
