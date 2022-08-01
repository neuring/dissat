use std::cmp::Ordering;

use crate::Solver;

use super::clause::{ClauseDB, ClauseIdx};

impl Solver {
    fn mark_garbage(&mut self) {
        let mut removal_candidates: Vec<_> = self
            .clause_db
            .iter_clause_meta_mut()
            .filter(|meta| !meta.is_garbage) // Already decided that this clause is garbage.
            .filter(|meta| matches!(meta.ldb_glue, Some(ldb) if ldb.get() > 2)) // We always keep clauses with LDB of two.
            .collect();

        // clauses with higher lbd first.
        removal_candidates.sort_by(|l, r| {
            let ord = l.ldb_glue.unwrap().get().cmp(&r.ldb_glue.unwrap().get());
            if ord == Ordering::Equal {
                l.range.len().cmp(&r.range.len()).reverse()
            } else {
                ord.reverse()
            }
        });

        // Remove 75% of remaining clauses.
        let target = (0.75 * removal_candidates.len() as f32) as usize;
        for meta in removal_candidates[..target].iter_mut() {
            meta.is_garbage = true;
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
            watches.retain_mut(|watch| update_clause_index(&mut watch.clause, &self.clause_db));
        }

        self.trail.update_clause_indices(|cls_idx| {
            let result = update_clause_index(cls_idx, &self.clause_db);
            debug_assert!(result); // No clauses that are contained in the trail can be removed.
        });
    }
}

fn update_clause_index(clause_idx: &mut ClauseIdx, clause_db: &ClauseDB) -> bool {
    let new_pos = clause_db.clause_data_old[clause_idx.start as usize];

    if new_pos == u32::MAX {
        false
    } else {
        clause_idx.start = new_pos;
        true
    }
}

impl ClauseDB {
    /// Remove all clauses that are marked as garbage in their metadata.
    /// `clause_data_old` contains their new offsets or u32::MAX, if they were removed.
    fn collect_garbage(&mut self) {
        let new_arena = &mut self.clause_data_old;
        new_arena.clear();

        self.clause_meta.retain_mut(|meta| {
            if meta.is_garbage {
                // Set sentinel value to mark that this clause has been removed.
                false
            } else {
                let clause = &self.clause_data[meta.range.start as usize..meta.range.end as usize];

                let new_start = new_arena.len() as u32;

                new_arena.extend(clause);

                meta.range = new_start..new_start + clause.len() as u32;

                self.clause_data[meta.range.start as usize] = new_start;

                true
            }
        });

        std::mem::swap(&mut self.clause_data, &mut self.clause_data_old);
    }
}
