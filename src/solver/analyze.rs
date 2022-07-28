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

struct AnalyzeState {
    seen: VarVec<bool>,

    /// New learned 1UIP clause which is under construction.
    new_clause: Vec<Lit>,

    /// Seen literals, whose reason clauses haven't been processed yet.
    open: u32,
}

impl AnalyzeState {
    fn new(num_vars: usize) -> Self {
        Self {
            seen: VarVec::with_size(num_vars, false),
            new_clause: Vec::new(),
            open: 0,
        }
    }

    fn analyze_reason(&mut self, lit: Option<Lit>, reason: Clause, trail: &Trail) {
        debug!("analyzing reason clause {reason:?}");
        for &other_lit in reason {
            if lit == Some(other_lit) {
                continue;
            }
            self.analyze_literal(other_lit, trail);
        }
    }

    fn analyze_literal(&mut self, lit: Lit, trail: &Trail) {
        if self.has_seen(lit) {
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

        self.seen[lit.var()] = true;
    }

    fn has_seen(&self, lit: Lit) -> bool {
        self.seen[lit.var()]
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

        let mut analyze_state = AnalyzeState::new(self.trail.total_vars());

        // Determine new 1UIP clause
        loop {
            analyze_state.analyze_reason(maybe_uip, reason, &self.trail);

            let uip = loop {
                debug_assert!(trail_pos > 0);

                trail_pos -= 1;
                let trail_elem = self.trail.get(trail_pos).unwrap();
                let lit = trail_elem.lit;

                if !analyze_state.has_seen(lit) {
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

        let uip_clause = analyze_state.new_clause;
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

        self.unpropagated_lit_pos = self.trail.backtrack(backjump_level);

        if uip_clause.len() == 1 {
            debug_assert_eq!(backjump_level, 0);
            self.trail.assign_lit(-uip, TrailReason::Axiom);
        } else {
            let uip_clause_idx = self.clause_db.insert_clause(&uip_clause);
            debug_assert!(self.trail.is_clause_all_unassigned(&uip_clause));
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
        }

        AnalyzeResult::Done
    }
}
