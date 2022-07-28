use tracing::debug;

/// Implementation of the unit propagation algorithm for two watched literals.
use super::{clause::ClauseIdx, trail::TrailReason, watch::Watch, Solver};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PropagationResult {
    Contradiction(ClauseIdx),
    Done,
}

impl Solver {
    pub(crate) fn propagate(&mut self) -> PropagationResult {
        debug!(
            "starting unit propagation {} (at {})",
            self.trail.fmt_trail(),
            self.unpropagated_lit_pos
        );
        let mut trail_pos = self.unpropagated_lit_pos;

        while let Some(&trail_elem) = self.trail.get(trail_pos) {
            let lit = trail_elem.lit;
            debug!(
                "propagating {lit}, with trail {trail}",
                trail = self.trail.fmt_trail()
            );
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

                let other_watched_lit_idx = (lit_idx + 1) & 1;
                let other_watched_lit = cls[other_watched_lit_idx];
                if self.trail.is_lit_satisfied(other_watched_lit) {
                    // Clause is satisfied, no need to consider further
                    // Watch hasn't changed.
                    return true;
                }

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
                let new_unit_lit_idx = other_watched_lit_idx;
                let new_unit_lit = other_watched_lit;

                // Depending on whether the other literal is unsatisfied or unassigned (it can never be satisfied),
                // we either have a new unit literal to propagate or found a contradiction.
                if self.trail.is_lit_unassigned(new_unit_lit) {
                    debug!(
                        "new unit clause found {cls}",
                        cls = self.trail.fmt_clause(cls)
                    );
                    debug!("assigning literal {new_unit_lit}");
                    self.trail
                        .assign_lit(new_unit_lit, TrailReason::Propagated { cls: cls_idx });
                    // Make sure the newly assigned literal is at the beginning of the clause.
                    cls.swap(0, new_unit_lit_idx);
                    self.stats.propagations += 1;
                    true
                } else {
                    debug!("contradiction encountered {}", self.trail.fmt_clause(cls));
                    debug_assert!(self.trail.is_lit_unsatisfied(new_unit_lit));
                    contradiction_found = Some(cls_idx);
                    self.stats.contradictions += 1;
                    true
                }
            });

            if let Some(conflicting_clause) = contradiction_found {
                debug!(
                    "unpropagated_lit_pos = {}, assigned_vars = {}",
                    self.unpropagated_lit_pos,
                    self.trail.assigned_vars()
                );
                return PropagationResult::Contradiction(conflicting_clause);
            }

            trail_pos += 1;
        }

        self.unpropagated_lit_pos = trail_pos;
        debug_assert_eq!(self.unpropagated_lit_pos, self.trail.assigned_vars());
        PropagationResult::Done
    }
}
