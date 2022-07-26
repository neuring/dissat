mod analyze;
mod assignment;
mod clause;
mod data;
mod garbage;
mod log;
mod propagate;
mod trail;
mod watch;

use analyze::AnalyzeResult;

use clause::ClauseDB;
use data::LitVec;
pub use data::{Lit, Var};
use propagate::PropagationResult;
use tracing::debug;
use trail::{Trail, TrailReason};
use watch::Watch;

use self::analyze::AnalyzeState;

#[derive(Default)]
pub struct Stats {
    pub contradictions: u64,
    pub propagations: u64,
    pub contradiction_since_last_garbage_collections: u64,
}

pub struct Limits {
    /// After how many conflicts do we initiate garbage collection.
    pub garbage_collection_conflicts: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            garbage_collection_conflicts: 3000,
        }
    }
}

#[derive(Default)]
pub struct Solver {
    clause_db: ClauseDB,

    watches: LitVec<Vec<Watch>>,

    trail: Trail,

    // Where on the trail, should the unit propgation continue.
    unpropagated_lit_pos: usize,

    // The input cnf formula is trivially unsat.
    // This might be because an empty clause was added or contradictory unit clauses.
    trivially_unsat: bool,

    /// Various stats that might be of interest
    stats: Stats,

    /// Various data, for analyzing conflicts. This field is mainly used in analyze.rs
    /// and reset for each new conflict analysis.
    analyze_state: AnalyzeState,

    /// Certain (dynamic limits) that are used to determine Behaviour
    limits: Limits,
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
            Result::Sat(_) => panic!("Result is SAT."),
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

    /// Remove duplicated literals
    /// Returns true if the clause is trivially satisfied (i.e. contains positive and negative literal of the same variable)
    fn normalise_clause(cls: &mut Vec<Lit>) -> bool {
        cls.sort_by_key(|lit| lit.var().get());
        cls.dedup();

        cls.array_windows().any(|[l0, l1]| l0.var() == l1.var())
    }

    pub fn add_clause<I>(&mut self, cls: I)
    where
        I: IntoIterator<Item = i32>,
    {
        let mut cls: Vec<Lit> = cls.into_iter().map(Lit::new).collect();

        if Self::normalise_clause(&mut cls) {
            return;
        };

        let max_lit = cls.iter().max_by_key(|l| l.var().get());

        if let Some(max_lit) = max_lit {
            self.trail.expand(max_lit.var());
            self.watches.expand(-Lit::from(max_lit.var()), Vec::new())
        }

        match cls.len() {
            0 => {
                self.trivially_unsat = true;
            }
            1 => {
                if self.trail.is_lit_unsatisfied(cls[0]) {
                    self.trivially_unsat = true;
                } else {
                    self.trail.assign_lit(cls[0], TrailReason::Axiom);
                }
            }
            _ => {
                let cls_idx = self.clause_db.insert_clause(&cls, None);
                for &lit in &cls[0..2] {
                    self.watches[lit].push(Watch { clause: cls_idx });
                }
            }
        }
    }

    fn all_vars_assigned(&self) -> bool {
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
        if self.trivially_unsat {
            return Result::Unsat(Proof);
        }

        loop {
            self.log_state();
            let result = self.propagate();

            if let PropagationResult::Contradiction(conflicting_clause) = result {
                if self.analyze_contradiction(conflicting_clause) == AnalyzeResult::Unsat {
                    debug!("Problem is Unsat");
                    return Result::Unsat(Proof);
                } else {
                    // 'analyze_contradiction` has flipped a decision variable.
                    // We need to start at the beginning with unit propagation.
                    continue;
                }
            } else if self.all_vars_assigned() {
                // When all variables are assigned we have to have a satisfying assignment, otherwise the
                // propagation result would have been `Contradiction`
                let model = self.extract_model();
                debug!("Satisfying assignment found! {:?}", model.as_vec());
                assert!(
                    self.check_assignment(),
                    "Generated assignment doesn't satisfy the input formula"
                );
                return Result::Sat(model);
            }

            self.maybe_collect_garbage();

            match self.decide() {
                Some(var) => {
                    debug!("new decision variable {var}");
                    self.trail.assign_lit(var.into(), TrailReason::Decision)
                }
                None => {
                    unreachable!("
                        No new decision variable candidate found, this means all variables are successfully satisified.
                        However we just checked that the formula hasn't been satisfied yet. 
                        Therefore, there have to be some unassigned variables.
                    ");
                }
            }
        }
    }

    /// check if the current assignment, satisfies the entire input formula.
    fn check_assignment(&self) -> bool {
        self.clause_db
            .iter()
            .all(|clause| self.trail.is_clause_satisfied(clause))
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }
}
