use tracing::debug;

use super::{clause::ClauseIdx, trail::TrailReason, Solver};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalyzeResult {
    Unsat,
    Done,
}

impl Solver {
    pub(crate) fn analyze_contradiction(&mut self, _clause: ClauseIdx) -> AnalyzeResult {
        while let Some(decision_elem) = self.trail.pop_decision() {
            debug_assert!(matches!(decision_elem.reason, TrailReason::Decision));
            if decision_elem.lit.is_pos() {
                debug!("inverting decision to {}", -decision_elem.lit);
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
