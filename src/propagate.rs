use crate::{watch::Watch, Solver};

pub(crate) enum PropagationResult {
    Contradiction,
    Done,
}

impl Solver {
    pub(crate) fn propagate(&mut self) -> PropagationResult {
        let mut trail_pos = self.last_propagation_depth;

        while let Some(&lit) = self.trail.get(trail_pos) {
            dbg!(trail_pos);
            println!("trail = {:?}", &self.trail);
            self.print_state();

            debug_assert!(self.assignment.is_lit_satisified(lit));

            let (lit_watch, mut remaining_watches) = self.watches.remaining(-lit);
            lit_watch.retain(|watch| {
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
                    if !self.assignment.is_lit_unsatisfied(*candidate) {
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
                if self.assignment.is_lit_unassigned(new_unit_lit) {
                    self.trail.push(new_unit_lit);
                    self.assignment.assign_lit(new_unit_lit);
                    // Make sure the newly assigned literal is at the beginning of the clause.
                    cls.swap(0, new_unit_lit_idx);
                    return true;
                } else {
                    debug_assert!(self.assignment.is_lit_unsatisfied(new_unit_lit));
                    // Contradiction found
                    return true;
                }
            });

            trail_pos += 1;
        }

        self.last_propagation_depth = trail_pos;
        debug_assert!(self.last_propagation_depth == self.trail.len());
        PropagationResult::Done
    }
}
