use crate::{clause::Clause, data::VarVec, Lit, Var};

pub(crate) struct Assignment {
    assignment: VarVec<Option<bool>>,
}

impl Assignment {
    pub fn new() -> Self {
        Self {
            assignment: VarVec::new(),
        }
    }

    pub fn expand(&mut self, v: Var) {
        self.assignment.expand(v, None);
    }

    pub fn get(&self, lit: Lit) -> Option<bool> {
        self.assignment[lit.var()].map(|var_val| var_val == lit.is_pos())
    }

    pub fn is_lit_satisified(&self, lit: Lit) -> bool {
        match self.assignment[lit.var()] {
            Some(var_val) => var_val == lit.is_pos(),
            None => false,
        }
    }

    pub fn is_lit_assigned(&self, lit: Lit) -> bool {
        self.assignment[lit.var()].is_some()
    }

    pub fn is_lit_unassigned(&self, lit: Lit) -> bool {
        self.assignment[lit.var()].is_none()
    }

    pub fn is_lit_unsatisfied(&self, lit: Lit) -> bool {
        match self.assignment[lit.var()] {
            Some(var_val) => var_val != lit.is_pos(),
            None => false,
        }
    }

    pub fn is_clause_satisified(&self, cls: Clause) -> bool {
        cls.iter().copied().any(|lit| self.is_lit_satisified(lit))
    }

    pub fn assign_lit(&mut self, lit: Lit) {
        debug_assert!(self.is_lit_unassigned(lit));

        self.assignment[lit.var()] = Some(lit.is_pos());
    }
}
