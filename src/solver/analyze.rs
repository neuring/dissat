use std::num::NonZeroU32;

use tracing::debug;

use super::{
    clause::{Clause, ClauseIdx},
    data::VarVec,
    trail::{Trail, TrailReason},
    Lit, Solver,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalyzeResult {
    Unsat,
    Done,
}

#[derive(Default)]
pub(crate) struct AnalyzeState {
    seen: VarVec<bool>,

    /// New learned 1UIP clause which is under construction.
    new_clause: Vec<Lit>,

    /// Seen literals, whose reason clauses haven't been processed yet.
    open: u32,

    /// Have we seen this decision level during conflict analysis
    levels_seen: Vec<bool>,

    /// Levels in new clause. We use this to derive the LDB value of a new clause.
    levels_in_clause: Vec<u32>,
}

impl AnalyzeState {
    fn reset(&mut self, num_vars: usize, decision_levels: usize) {
        self.seen.fill(false);
        self.seen.resize(num_vars, false);
        self.new_clause.clear();
        self.levels_in_clause.clear();
        self.levels_seen.clear();
        self.levels_seen.resize(decision_levels + 1, false);
        self.open = 0;
    }

    fn analyze_reason(&mut self, lit: Option<Lit>, reason: Clause, trail: &Trail) {
        debug!("analyzing reason clause {reason:?}");
        for other_lit in reason {
            if lit == Some(other_lit) {
                continue;
            }
            self.analyze_literal(other_lit, trail);
        }
    }

    fn analyze_literal(&mut self, lit: Lit, trail: &Trail) {
        if self.has_seen_lit(lit) {
            debug!("analyzing literal {lit}, already analyzed (seen)");
            return;
        }

        let lit_level = trail.get_decision_level(lit).unwrap();
        let current_level = trail.current_decision_level();

        debug_assert!(trail.is_lit_unsatisfied(lit));
        debug_assert!(lit_level <= current_level, "{lit_level} <= {current_level}");

        if lit_level < current_level {
            debug!(
                "analyzing literal {lit} which is before current decision => include in new clause"
            );
            self.new_clause.push(lit)
        } else {
            debug!("analyzing literal {lit}.");
            self.open += 1;
        }

        if !self.has_seen_level(lit_level) {
            self.levels_seen[lit_level as usize] = true;
            self.levels_in_clause.push(lit_level);
        }

        self.seen[lit.var()] = true;
    }

    fn has_seen_lit(&self, lit: Lit) -> bool {
        self.seen[lit.var()]
    }

    fn has_seen_level(&self, lvl: u32) -> bool {
        self.levels_seen[lvl as usize]
    }
}

impl Solver {
    fn everything_before_last_decision_has_been_propagated(&self) -> bool {
        if let Some(pos) = self.trail.last_decision_pos() {
            pos <= self.unpropagated_lit_pos
        } else {
            true
        }
    }

    #[allow(unused)]
    pub(crate) fn analyze_contradiction_dpll(&mut self, _clause: ClauseIdx) -> AnalyzeResult {
        debug_assert!(self.everything_before_last_decision_has_been_propagated());
        debug_assert!(self.unpropagated_lit_pos <= self.trail.assigned_vars());

        while let Some(decision_elem) = self.trail.pop_decision() {
            debug_assert!(matches!(decision_elem.reason, TrailReason::Decision));
            if decision_elem.lit.is_pos() {
                debug!("inverting decision literal to {}", -decision_elem.lit);
                self.unpropagated_lit_pos = self.trail.assigned_vars();
                self.trail
                    .assign_lit(-decision_elem.lit, TrailReason::Decision);

                return AnalyzeResult::Done;
            } else {
                debug!("popping decision lit {}", decision_elem.lit);
            }
        }

        AnalyzeResult::Unsat
    }

    pub(crate) fn analyze_contradiction(&mut self, clause: ClauseIdx) -> AnalyzeResult {
        debug!("analyzing contradiction. Trail: {}", self.trail.fmt_trail());
        debug_assert!(self.everything_before_last_decision_has_been_propagated());
        debug_assert!(self.unpropagated_lit_pos <= self.trail.assigned_vars());

        let conflict_clause = self.clause_db.get(clause);

        debug_assert!(conflict_clause
            .iter()
            .all(|&lit| self.trail.is_lit_unsatisfied(lit)));

        let current_level = self.trail.current_decision_level();

        if current_level == 0 {
            return AnalyzeResult::Unsat;
        }

        let mut trail_pos = self.trail.assigned_vars();
        let mut reason = conflict_clause;
        let mut maybe_uip = None;

        let mut analyze_state = &mut self.analyze_state;
        analyze_state.reset(self.trail.total_vars(), current_level as usize);

        // Determine new 1UIP clause
        loop {
            analyze_state.analyze_reason(maybe_uip, reason, &self.trail);

            let uip = loop {
                debug_assert!(trail_pos > 0);

                trail_pos -= 1;
                let trail_elem = self.trail.get(trail_pos).unwrap();
                let lit = trail_elem.lit;

                if !analyze_state.has_seen_lit(lit) {
                    continue;
                }

                if self.trail.get_decision_level(lit).unwrap() == current_level {
                    break lit;
                }
            };
            maybe_uip = Some(uip);

            if analyze_state.open == 1 {
                break;
            }
            analyze_state.open -= 1;

            let reason_idx = self.trail.get_reason_cls(uip);
            reason = self.clause_db.get(reason_idx);

            debug!(
                "analyzing reason clause of literal {uip} (open = {})",
                analyze_state.open
            );
        }
        let uip = maybe_uip.unwrap();
        analyze_state.new_clause.push(-uip);

        let uip_clause = &mut analyze_state.new_clause;
        debug!("Learned new 1UIP clause: {:?}", uip_clause);

        let backjump_level = uip_clause[..uip_clause.len() - 1] // The last literal is the uip literal. The only literal with the highest lvl.
            .iter()
            .map(|&lit| self.trail.get_decision_level(lit).unwrap())
            .inspect(|&lvl| {
                debug_assert!(
                    lvl < self
                        .trail
                        .get_decision_level(*uip_clause.last().unwrap())
                        .unwrap()
                )
            })
            .max()
            .unwrap_or(0);

        self.unpropagated_lit_pos = self.trail.backtrack(backjump_level, |trail_elem| {
            if let TrailReason::Propagated { cls } = trail_elem.reason {
                self.clause_db.get_mut(cls).flags().set_is_reason(false);
            }
        });

        if uip_clause.len() == 1 {
            debug_assert_eq!(backjump_level, 0);
            self.trail.assign_lit(-uip, TrailReason::Axiom);
        } else {
            let ldb_glue = (analyze_state.levels_in_clause.len() as u32).try_into()
                .expect("There has to be atleast a one level in clause, otherwise the clause length would be one");
            debug_assert_eq!(
                ldb_glue,
                Self::calculate_ldb_from_lits(&self.trail, &uip_clause)
            );
            debug!("new 1UIP clause has ldb value of {ldb_glue}");

            let uip_clause_idx = self.clause_db.insert_clause(&uip_clause, Some(ldb_glue));
            debug_assert!(self.trail.are_lits_all_unassigned(&uip_clause));
            debug!(
                "Assigning flipped uip {} because of learned driving clause {uip_clause:?}",
                -uip
            );
            self.trail.assign_lit(
                -uip,
                TrailReason::Propagated {
                    cls: uip_clause_idx,
                },
            );
            self.clause_db
                .get_mut(uip_clause_idx)
                .flags()
                .set_is_reason(true);
        }

        AnalyzeResult::Done
    }

    fn calculate_ldb_from_lits(trail: &Trail, cls: &[Lit]) -> NonZeroU32 {
        (cls.iter()
            .map(|&lit| trail.get_decision_level(lit))
            .collect::<std::collections::HashSet<_>>()
            .len() as u32)
            .try_into()
            .unwrap()
    }
}
