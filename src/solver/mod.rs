mod analyze;
mod assignment;
mod clause;
mod data;
mod propagate;
mod trail;
mod watch;

use analyze::AnalyzeResult;

use clause::ClauseDB;
use data::LitVec;
pub use data::{Lit, Var};
use propagate::PropagationResult;
use trail::{Trail, TrailReason};
use watch::Watch;

#[derive(Default)]
pub struct Solver {
    clause_db: ClauseDB,

    watches: LitVec<Vec<Watch>>,

    trail: Trail,
    last_propagation_depth: usize,
}

pub struct Model<'a> {
    assignment: &'a Trail,
}

impl<'a> Model<'a> {
    /// Get truth assignment of literal.
    pub fn lit(&self, l: i32) -> bool {
        self.assignment.is_lit_satisfied(Lit::new(l))
    }

    pub fn as_vec(&self) -> Vec<i32> {
        (1..=self.assignment.total_vars())
            .map(|i| {
                let i = i as i32;
                i * if self.assignment.is_lit_satisfied(Lit::new(i)) {
                    1
                } else {
                    -1
                }
            })
            .collect()
    }
}

impl std::fmt::Debug for Model<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Model").field(&self.as_vec()).finish()
    }
}

#[derive(Debug)]
pub struct Proof;

#[derive(Debug)]
pub enum Result<'a> {
    Sat(Model<'a>),
    Unsat(Proof),
}

impl<'a> Result<'a> {
    pub fn is_sat(&self) -> bool {
        matches!(self, Result::Sat(_))
    }

    pub fn is_unsat(&self) -> bool {
        matches!(self, Result::Unsat(_))
    }

    pub fn unwrap_sat(self) -> Model<'a> {
        match self {
            Result::Sat(model) => model,
            Result::Unsat(_) => panic!("Result is not SAT."),
        }
    }

    pub fn unwrap_unsat(self) -> Proof {
        match self {
            Result::Unsat(proof) => proof,
            Result::Sat(_) => panic!("Result is not SAT."),
        }
    }
}

impl Solver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_dimacs(input: &str) -> std::result::Result<Solver, Box<dyn std::error::Error>> {
        let clauses = crate::dimacs::Dimacs::parse(input)?;

        let mut solver = Solver::new();
        for clause in clauses {
            solver.add_clause(clause);
        }

        Ok(solver)
    }

    pub fn add_clause<I>(&mut self, cls: I)
    where
        I: IntoIterator<Item = i32>,
    {
        let cls: Vec<Lit> = cls.into_iter().map(Lit::new).collect();

        let max_lit = cls.iter().max_by_key(|l| l.var().get());

        if let Some(max_lit) = max_lit {
            self.trail.expand(max_lit.var());
            self.watches
                .expand(-Lit::new(max_lit.var().get()), Vec::new())
        }

        match cls.len() {
            0 => {
                panic!("Empty clause added. Formula is trivially unsat.")
            }
            1 => {
                self.trail.assign_lit(cls[0], TrailReason::Axiom);
            }
            _ => {
                let cls_idx = self.clause_db.insert_clause(&cls);
                for &lit in &cls[0..2] {
                    self.watches[lit].push(Watch { clause: cls_idx });
                }
            }
        }
    }

    fn is_satisfied(&self) -> bool {
        self.trail.assignment_complete()
    }

    fn extract_model(&self) -> Model {
        Model {
            assignment: &self.trail,
        }
    }

    fn decide(&mut self) -> Option<Var> {
        self.trail.find_unassigned_variable()
    }

    pub fn solve(&mut self) -> Result {
        loop {
            let result = self.propagate();

            if let PropagationResult::Contradiction(conflicting_clause) = result {
                if self.analyze_contradiction(conflicting_clause) == AnalyzeResult::Unsat {
                    return Result::Unsat(Proof);
                }
            } else if self.is_satisfied() {
                return Result::Sat(self.extract_model());
            }

            match self.decide() {
                Some(var) => self.trail.assign_lit(var.into(), TrailReason::Decision),
                None => {
                    unreachable!("
                        No new decision variable candidate found, this means all variables are successfully satisified.
                        However we just checked that the formula hasn't been satisfied yet. 
                        Therefore that have to be some unassigned variables.
                    ");
                }
            }
        }
    }

    #[allow(unused)]
    pub(crate) fn print_state(&self) {
        const RED: &str = "\u{1b}[31m";
        const GREEN: &str = "\u{1b}[32m";
        const END: &str = "\u{1b}[0m";

        for cls in self.clause_db.iter() {
            let cls_str = cls
                .iter()
                .map(|&lit| match self.trail.get_lit_assignment(lit) {
                    Some(true) => format!("{GREEN}{}{END}", lit.get()),
                    Some(false) => format!("{RED}{}{END}", lit.get()),
                    None => format!("{}", lit.get()),
                })
                .intersperse(", ".into())
                .collect::<String>();
            println!("{cls_str}");
        }
    }
}