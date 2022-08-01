use tracing::debug;

use super::{
    assignment::Assignment,
    clause::{Clause, ClauseIdx},
    Lit, Var,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrailReason {
    /// Literals was decided.
    Decision,

    /// Literal was propagated during unit propagation [`Solver::propagate`]
    Propagated { cls: ClauseIdx },

    /// Axiomatic literal. These are generated when the user is supplying a unit clause.
    Axiom,
}

impl TrailReason {
    /// Retrieve the clause index of a propagated literal.
    /// Panics if self is not the Propagated variant.
    pub(crate) fn get_cls_idx_mut(&mut self) -> &mut ClauseIdx {
        match self {
            TrailReason::Propagated { cls } => cls,
            TrailReason::Decision | TrailReason::Axiom => {
                panic!("`self` is not the `Propagated` variant.")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrailElement {
    pub(crate) lit: Lit,
    pub(crate) reason: TrailReason,
}

#[derive(Default)]
pub(crate) struct Trail {
    /// The order of literal assignments and the reason as for why.
    trail: Vec<TrailElement>,

    /// The position of decisions in the trail.
    /// Note: The decision for a certain lvl is found at the index `lvl - 1`
    decision_positions: Vec<usize>,

    assignment: Assignment,
}

impl Trail {
    pub fn assigned_vars(&self) -> usize {
        self.trail.len()
    }

    pub fn current_decision_level(&self) -> u32 {
        self.decision_positions.len() as u32
    }

    pub fn total_vars(&self) -> usize {
        self.assignment.len()
    }

    pub fn get(&self, idx: usize) -> Option<&TrailElement> {
        self.trail.get(idx)
    }

    pub fn get_lit_assignment(&self, lit: Lit) -> Option<bool> {
        self.assignment.get(lit)
    }

    pub fn last_decision_pos(&self) -> Option<usize> {
        let result = self.decision_positions.last().copied();
        debug_assert!(if let Some(pos) = result {
            self.trail[pos].reason == TrailReason::Decision
        } else {
            true
        });
        result
    }

    // Remove and return the last decision in the trail, including all literals with the same decision level.
    pub fn pop_decision(&mut self) -> Option<TrailElement> {
        tracing::debug!(
            "trail = {}, trail_pos = {:?}",
            self.fmt_trail(),
            self.decision_positions
        );
        let decision_pos = self.decision_positions.pop()?;

        let decision_elem = loop {
            match self.trail.pop() {
                Some(trail_elem @ TrailElement {
                    lit,
                    reason: TrailReason::Decision,
                }) => {
                    debug_assert!(self.trail.len() == decision_pos);
                    self.assignment.unassign_lit(lit);
                    break trail_elem;
                },
                Some(TrailElement { lit, .. }) => {
                    self.assignment.unassign_lit(lit)
                },
                None => unreachable!("Above, we found a decision in `decision_positions`, so we have to find TrailElement with Decision reason."),
            }
        };

        Some(decision_elem)
    }

    pub(crate) fn update_clause_indices(&mut self, update_fn: impl Fn(&mut ClauseIdx)) {
        for trail_elem in self.trail.iter_mut() {
            if let TrailReason::Propagated { cls } = &mut trail_elem.reason {
                update_fn(cls);

                let assignment_data = self
                    .assignment
                    .get_data_mut(trail_elem.lit)
                    .expect("We know that lit is in trail and therefore assigned.");

                update_fn(assignment_data.reason.get_cls_idx_mut())
            }
        }
    }

    /// Expands internal assignment for new max variable.
    pub(crate) fn expand(&mut self, var: Var) {
        self.assignment.expand(var)
    }

    pub fn assignment_complete(&self) -> bool {
        self.trail.len() == self.assignment.len()
    }

    /// Delegates over `Assignment`
    #[allow(unused)]
    pub fn is_lit_assigned(&self, lit: Lit) -> bool {
        self.assignment.is_lit_assigned(lit)
    }

    pub fn is_lit_unassigned(&self, lit: Lit) -> bool {
        self.assignment.is_lit_unassigned(lit)
    }

    pub fn is_lit_satisfied(&self, lit: Lit) -> bool {
        self.assignment.is_lit_satisified(lit)
    }

    pub fn is_lit_unsatisfied(&self, lit: Lit) -> bool {
        self.assignment.is_lit_unsatisfied(lit)
    }

    /// For now this is just a bad but simple procedure to find next decision candidate
    pub fn find_unassigned_variable(&self) -> Option<Var> {
        self.assignment.find_unassigned_variable()
    }

    pub fn assign_lit(&mut self, lit: Lit, reason: TrailReason) {
        self.trail.push(TrailElement { lit, reason });
        if reason == TrailReason::Decision {
            self.decision_positions.push(self.trail.len() - 1)
        }
        let decision_level = self.current_decision_level();
        self.assignment
            .assign_lit(lit, decision_level as u32, reason);
    }

    pub fn trail(&self) -> &[TrailElement] {
        &self.trail
    }

    pub fn is_clause_satisfied(&self, clause: Clause) -> bool {
        clause.iter().copied().any(|lit| self.is_lit_satisfied(lit))
    }

    pub fn is_clause_all_unassigned(&self, clause: Clause) -> bool {
        clause
            .iter()
            .copied()
            .any(|lit| self.is_lit_unsatisfied(lit))
    }

    pub fn get_decision_level(&self, lit: Lit) -> Option<u32> {
        self.assignment
            .get_data(lit)
            .map(|data| data.decision_level)
    }

    /// Get the reason clause of a propagated literal.
    /// Panics if lit is not a propagated clause.
    pub fn get_reason_cls(&self, lit: Lit) -> ClauseIdx {
        let reason = self.assignment.get_data(lit).unwrap().reason;
        if let TrailReason::Propagated { cls } = reason {
            cls
        } else {
            panic!("Literal {lit} wasn't propagated");
        }
    }

    // Backtrack assignments such that the literals with the decision level `lvl` are last on the trail (i.e. are not removed.)
    // Returns the position, where unit propagation should continue
    pub fn backtrack(&mut self, lvl: u32, mut on_pop: impl FnMut(&TrailElement)) -> usize {
        debug!("Backtrack to decision level {lvl}");

        // We look in `decision_positions` for the decision for lvl + 1
        // We don't need to add one to the index because `decision_positions` starts at zero.
        let pos = match self.decision_positions.get(lvl as usize) {
            Some(pos) => *pos,
            None => {
                // lvl is at the top of the trail. Nothing to remove.
                return self.trail.len();
            }
        };

        for &e in &self.trail[pos..] {
            self.assignment.unassign_lit(e.lit);
            on_pop(&e);
        }

        self.trail.truncate(pos);
        self.decision_positions.truncate(lvl as usize);

        self.trail.len()
    }
}
