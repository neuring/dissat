use super::{data::VarVec, Lit, Var};

#[derive(Clone, Copy, Debug)]
struct AssignData {
    status: bool,
    decision_level: u32,
}

#[derive(Default)]
pub(crate) struct Assignment {
    assignment: VarVec<Option<AssignData>>,
}

impl Assignment {
    pub fn expand(&mut self, v: Var) {
        self.assignment.expand(v, None);
    }

    pub fn get(&self, lit: Lit) -> Option<bool> {
        self.assignment[lit.var()].map(|var_val| var_val.status == lit.is_pos())
    }

    pub fn is_lit_satisified(&self, lit: Lit) -> bool {
        match self.assignment[lit.var()] {
            Some(var_val) => var_val.status == lit.is_pos(),
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
            Some(var_val) => var_val.status != lit.is_pos(),
            None => false,
        }
    }

    pub fn assign_lit(&mut self, lit: Lit, decision_level: u32) {
        debug_assert!(self.is_lit_unassigned(lit));

        self.assignment[lit.var()] = Some(AssignData {
            status: lit.is_pos(),
            decision_level,
        });
    }

    pub fn unassign_lit(&mut self, lit: Lit) {
        debug_assert!(self.is_lit_assigned(lit));

        self.assignment[lit.var()] = None;
    }

    pub fn len(&self) -> usize {
        self.assignment.len()
    }

    /// For now this is just a bad but simple procedure to find next decision candidate
    pub fn find_unassigned_variable(&self) -> Option<Var> {
        let (var, _) = self
            .assignment
            .iter_with_var()
            .find(|&(_, data)| data.is_none())?;

        Some(var)
    }
}
