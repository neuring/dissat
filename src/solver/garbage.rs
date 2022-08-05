use std::cmp::Ordering;

use tracing::debug;

use crate::{solver::trail::TrailReason, Solver};

use super::clause::{ClauseDB, ClauseIdx};

impl Solver {
    fn mark_garbage(&mut self) {
        let mut removal_candidates: Vec<_> = self
            .clause_db
            .iter_mut()
            .filter(|cls| !cls.flags().is_garbage()) // Already decided that this clause is garbage.
            .filter(|cls| !cls.flags().is_reason()) // This clause is a reason clause used in the trail.
            .filter(|cls| matches!(cls.glue(), Some(ldb) if ldb.get() > 2)) // We always keep clauses with LDB of two.
            .collect();

        // Make sure none of the removal_candidates appear in the trail.
        debug_assert!(removal_candidates
        .iter()
        .all(|&candidate_cls| self.trail.trail().iter().all(|elem| {
            if let TrailReason::Propagated { cls } = elem.reason {
                let b = candidate_cls != self.clause_db.get(cls);
                if !b { debug!("The reason clause for lit {} appears in trail and can not be marked as garbage.", elem.lit)}
                b
            } else {
                true
            }
        })));

        // clauses with higher lbd first.
        removal_candidates.sort_by(|l, r| {
            let ord = l.glue().unwrap().get().cmp(&r.glue().unwrap().get());
            if ord == Ordering::Equal {
                l.len().cmp(&r.len()).reverse()
            } else {
                ord.reverse()
            }
        });

        // Remove 75% of remaining clauses.
        let target = (0.75 * removal_candidates.len() as f32) as usize;
        for cls in removal_candidates[..target].iter_mut() {
            cls.flags().set_is_garbage(true);
        }
    }

    /// Initiate garbage collection.
    /// This functions marks clauses considered useless as garbage, removes them from the clause database
    /// and updates all clause indices.
    pub(crate) fn collect_garbage(&mut self) {
        self.mark_garbage();

        // Remove garbage clauses from clause database.
        self.clause_db.collect_garbage();

        // Update ClauseIdx wherever they appear (Watches and Trail)
        for watches in self.watches.iter_mut() {
            watches.retain_mut(|watch| self.clause_db.update_old_clause_index(&mut watch.clause));
        }

        self.trail.update_clause_indices(|cls_idx| {
            let result = self.clause_db.update_old_clause_index(cls_idx);
            debug_assert!(
                result,
                "No clauses that are contained in the trail can be removed.",
            );
        });
    }

    /// Checks if garbage collection limit is reached, and if so, will collect garbage clauses.
    pub(crate) fn maybe_collect_garbage(&mut self) {
        if self.stats.contradiction_since_last_garbage_collections
            >= self.limits.garbage_collection_conflicts
        {
            return;
        }

        debug!("Collecting garbage clauses.");

        self.stats.contradiction_since_last_garbage_collections = 0;
        self.limits.garbage_collection_conflicts =
            ((self.limits.garbage_collection_conflicts as f32) * 1.05) as u64;

        self.collect_garbage();
    }
}
