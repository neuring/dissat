/// Implementation of the unit propagation algorithm for two watched literals.
use super::{clause::ClauseIdx, trail::TrailReason, watch::Watch, Solver};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PropagationResult {
    Contradiction(ClauseIdx),
    Done,
}

impl Solver {
    pub(crate) fn propagate(&mut self) -> PropagationResult {
        let mut trail_pos = self.last_propagation_depth;

        while let Some(&trail_elem) = self.trail.get(trail_pos) {
            let lit = trail_elem.lit;
            debug_assert!(self.trail.is_lit_satisfied(lit));

            let mut contradiction_found = None;

            let (lit_watch, mut remaining_watches) = self.watches.remaining(-lit);
            lit_watch.retain(|watch| {
                // We stop propagating if a contradiction was found.
                // In this case we just want `retain` to keep the rest of the elements.
                if contradiction_found.is_some() {
                    return true;
                }

                // Which watched clauses do we need to search for new literal.
                let cls_idx = watch.clause;
                let cls = self.clause_db.get_mut(cls_idx);

                let lit_idx = if cls[0] == -lit {
                    0
                } else {
                    debug_assert!(cls[1] == -lit);
                    1
                };

                // search for new unassigned or satisified literal.
                for (candidate_idx, candidate) in cls.iter_mut().enumerate().skip(2) {
                    if !self.trail.is_lit_unsatisfied(*candidate) {
                        // In order to watch the new literal, we push a new watch.
                        remaining_watches[*candidate].push(Watch { clause: cls_idx });

                        // And move the new literal at the beginning, swapping the with the old watched literal
                        cls.swap(lit_idx, candidate_idx);

                        // Returning false for the surrounding `retain` call, in order to remove the old watch.
                        return false;
                    }
                }

                // No suitable new candidate was found, i.e. all other non-watched literals are unsatisified.
                let new_unit_lit_idx = (lit_idx + 1) & 1; // Get the other of the first two literals (which are watched).
                let new_unit_lit = cls[new_unit_lit_idx];

                // Depending on whether the other literal is unsatisfied or unassigned (it can never be satisfied),
                // we either have a new unit literal to propagate or found a contradiction.
                if self.trail.is_lit_unassigned(new_unit_lit) {
                    self.trail
                        .assign_lit(new_unit_lit, TrailReason::Propagated { cls: cls_idx });
                    // Make sure the newly assigned literal is at the beginning of the clause.
                    cls.swap(0, new_unit_lit_idx);
                    true
                } else {
                    debug_assert!(self.trail.is_lit_unsatisfied(new_unit_lit));
                    contradiction_found = Some(cls_idx);
                    true
                }
            });

            if let Some(conflicting_clause) = contradiction_found {
                return PropagationResult::Contradiction(conflicting_clause);
            }

            trail_pos += 1;
        }

        self.last_propagation_depth = trail_pos;
        debug_assert!(self.last_propagation_depth == self.trail.assigned_vars());
        PropagationResult::Done
    }
}
