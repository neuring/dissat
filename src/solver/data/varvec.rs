use super::Var;

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

    pub fn len(&self) -> usize {
        // The first element is always empty, because we index using the underlying NonZero value of a variable.
        // Since this value can never be zero, the length is effectively on less.
        // We use this len value to also store the number of variables so it is important to be exact here.
        self.0.len() - 1
    }

    pub fn iter_with_var(&self) -> impl Iterator<Item = (Var, &T)> + '_ {
        self.0
            .iter()
            .enumerate()
            .skip(1)
            .map(|(var, val)| (Var::new(var as i32), val))
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
