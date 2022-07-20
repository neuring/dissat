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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrailElement {
    pub(crate) lit: Lit,
    pub(crate) reason: TrailReason,
}

#[derive(Default)]
pub(crate) struct Trail {
    trail: Vec<TrailElement>,
    decision_positions: Vec<usize>,
    assignment: Assignment,
}

impl Trail {
    pub fn assigned_vars(&self) -> usize {
        self.trail.len()
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
        let decision_level = self.trail.len();
        self.assignment.assign_lit(lit, decision_level as u32);
    }

    pub fn trail(&self) -> &[TrailElement] {
        &self.trail
    }

    pub fn is_clause_satisfied(&self, clause: Clause) -> bool {
        clause.iter().copied().any(|lit| self.is_lit_satisfied(lit))
    }
}
