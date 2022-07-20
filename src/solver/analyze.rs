use tracing::debug;

use super::{clause::ClauseIdx, trail::TrailReason, Solver};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalyzeResult {
    Unsat,
    Done,
}

impl Solver {
    pub(crate) fn analyze_contradiction(&mut self, _clause: ClauseIdx) -> AnalyzeResult {
        debug_assert!(if let Some(pos) = self.trail.last_decision_pos() {
            pos <= self.unpropagated_lit_pos
        } else {
            true
        });
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
}
