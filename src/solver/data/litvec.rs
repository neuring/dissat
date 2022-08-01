use super::Lit;
use crate::util;

/// Wrapper over Vec which is indexed by [`Lit`]
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct LitVec<T>(Vec<T>);

impl<T> LitVec<T> {
    pub fn new() -> Self {
        Self(Default::default())
    }

    /// Get the element stored for `l` and a `Remaining` object which allows the indexing
    /// for the other indices that are not `l`.
    pub fn remaining(&mut self, l: Lit) -> (&mut T, Remaining<T>) {
        let (val, remaining) =
            util::remaining(&mut self.0, lit_to_idx(l)).expect("litvec is too small for lit");
        (val, Remaining(remaining))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> + '_ {
        self.0.iter_mut()
    }
}

pub struct Remaining<'a, T>(util::Remaining<'a, T>);

impl<'a, T> std::ops::Index<Lit> for Remaining<'a, T> {
    type Output = T;

    fn index(&self, index: Lit) -> &Self::Output {
        self.0
            .get(lit_to_idx(index))
            .expect("index out of bounds or already used.")
    }
}

impl<'a, T> std::ops::IndexMut<Lit> for Remaining<'a, T> {
    fn index_mut(&mut self, index: Lit) -> &mut Self::Output {
        self.0
            .get_mut(lit_to_idx(index))
            .expect("index out of bounds or already used.")
    }
}

impl<T: Clone> LitVec<T> {
    /// Resize so that `l` is valid index.
    pub fn expand(&mut self, l: Lit, val: T) {
        let len = lit_to_idx(l) + 1;

        if len >= self.0.len() {
            self.0.resize(len, val)
        }
    }
}

impl<T> Default for LitVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn lit_to_idx(lit: Lit) -> usize {
    let i = lit.get();

    // positive and negative lit of a variable are placed next to each other.
    // We subtract two, because there are not 0 or -0 literals.
    let idx = (i < 0) as i32 + 2 * i.abs() - 2;
    debug_assert!(idx >= 0);
    idx as usize
}

impl<T> std::ops::Index<Lit> for LitVec<T> {
    type Output = T;

    fn index(&self, index: Lit) -> &Self::Output {
        &self.0[lit_to_idx(index)]
    }
}

impl<T> std::ops::IndexMut<Lit> for LitVec<T> {
    fn index_mut(&mut self, index: Lit) -> &mut Self::Output {
        &mut self.0[lit_to_idx(index)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lit_to_idx() {
        assert_eq!(lit_to_idx(Lit::new(1)), 0);
        assert_eq!(lit_to_idx(Lit::new(-1)), 1);
        assert_eq!(lit_to_idx(Lit::new(2)), 2);
        assert_eq!(lit_to_idx(Lit::new(-2)), 3);
        assert_eq!(lit_to_idx(Lit::new(3)), 4);
        assert_eq!(lit_to_idx(Lit::new(-3)), 5);
        assert_eq!(lit_to_idx(Lit::new(4)), 6);
        assert_eq!(lit_to_idx(Lit::new(-4)), 7);
    }

    #[test]
    fn test() {
        let mut litvec: LitVec<i32> = LitVec::new();
        litvec.expand(Lit::new(4), 0);

        litvec[Lit::new(1)] = 1;
        litvec[Lit::new(-1)] = -1;

        litvec[Lit::new(3)] = 3;
        litvec[Lit::new(-3)] = -3;

        assert_eq!(litvec[Lit::new(1)], 1);
        assert_eq!(litvec[Lit::new(-1)], -1);
        assert_eq!(litvec[Lit::new(3)], 3);
        assert_eq!(litvec[Lit::new(-3)], -3);
        assert_eq!(litvec[Lit::new(2)], 0);
    }
}
