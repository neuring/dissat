#![feature(nonzero_ops)]
#![feature(iter_intersperse)]

mod assignment;
mod clause;
mod data;
mod propagate;
mod util;
mod watch;

use assignment::Assignment;
use clause::ClauseDB;
use data::LitVec;
pub use data::{Lit, Var};
use watch::Watch;

pub struct Solver {
    clause_db: ClauseDB,

    watches: LitVec<Vec<Watch>>,

    trail: Vec<Lit>,
    last_propagation_depth: usize,

    assignment: Assignment,
}

pub struct Model;

pub struct Proof;

pub enum Result {
    SAT(Model),
    UNSAT(Proof),
}

impl Solver {
    pub fn new() -> Self {
        Self {
            clause_db: ClauseDB::new(),
            watches: LitVec::new(),
            trail: Vec::new(),
            last_propagation_depth: 0,
            assignment: Assignment::new(),
        }
    }

    pub fn add_clause<I>(&mut self, cls: I)
    where
        I: IntoIterator<Item = i32>,
    {
        let cls: Vec<Lit> = cls.into_iter().map(|i| Lit::new(i)).collect();

        let max_lit = cls.iter().max_by_key(|l| l.var().get());

        if let Some(max_lit) = max_lit {
            self.assignment.expand(max_lit.var());
            self.watches
                .expand(-Lit::new(max_lit.var().get()), Vec::new())
        }

        match cls.len() {
            0 => {
                panic!("Empty clause added. Formula is trivially unsat.")
            }
            1 => {
                self.assignment.assign_lit(cls[0]);
                self.trail.push(cls[0]);
            }
            _ => {
                let cls_idx = self.clause_db.insert_clause(&cls);
                for &lit in &cls[0..2] {
                    self.watches[lit].push(Watch { clause: cls_idx });
                }
            }
        }
    }

    pub fn solve(&mut self) -> Result {
        let result = self.propagate();

        match result {
            propagate::PropagationResult::Contradiction => Result::UNSAT(Proof),
            propagate::PropagationResult::Done => Result::SAT(Model),
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
                .map(|&lit| match self.assignment.get(lit) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propagate_test() {
        let mut solver = Solver::new();
        solver.add_clause([-1, 2, 3]);
        solver.add_clause([-1, -2]);
        solver.add_clause([1]);

        solver.solve();
    }
}
