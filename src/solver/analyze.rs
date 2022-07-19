use super::{
    clause::ClauseIdx,
    trail::{TrailElement, TrailReason},
    Solver,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnalyzeResult {
    Unsat,
    Done,
}

impl Solver {
    pub(crate) fn analyze_contradiction(&mut self, _clause: ClauseIdx) -> AnalyzeResult {
        while let Some(decision_elem) = self.trail.pop_decision() {
            if decision_elem.lit.is_pos() {
                self.trail.push(TrailElement {
                    lit: -decision_elem.lit,
                    reason: TrailReason::Decision,
                });

                return AnalyzeResult::Done;
            }
        }

        AnalyzeResult::Unsat
    }
}
