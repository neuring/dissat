use crate::Var;

/// Wrapper over Vec which is indexed by [`Var`]
#[derive(Debug, PartialEq, Eq, Default, Clone, Hash)]
pub struct VarVec<T>(Vec<T>);

impl<T> VarVec<T> {
    pub fn new() -> Self {
        VarVec(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        VarVec(Vec::with_capacity(capacity))
    }
}

impl<T: Clone> VarVec<T> {
    /// Resize so that `v` is valid index.
    pub fn expand(&mut self, v: Var, val: T) {
        let len = v.get() as usize + 1;
        if len >= self.0.len() {
            self.0.resize(len, val)
        }
    }
}

impl<T> IntoIterator for VarVec<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a VarVec<T> {
    type Item = &'a T;

    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut VarVec<T> {
    type Item = &'a mut T;

    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter_mut()
    }
}

impl<T> std::ops::Index<Var> for VarVec<T> {
    type Output = T;

    fn index(&self, index: Var) -> &Self::Output {
        let index = index.get() as usize;
        &self.0[index]
    }
}

impl<T> std::ops::IndexMut<Var> for VarVec<T> {
    fn index_mut(&mut self, index: Var) -> &mut Self::Output {
        let index = index.get() as usize;
        &mut self.0[index]
    }
}
