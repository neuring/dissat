/// Clauses are stores continously in memory.
/// Each clause has atleast two literals.
/// The first two literals are watched.
/// A variable can only appear once in a clause.
use std::{num::NonZeroU32, ops::Range};

use super::Lit;

pub type Clause<'db> = &'db [Lit];
pub type ClauseMut<'db> = &'db mut [Lit];

fn clause_to_data(clause: Clause) -> &[u32] {
    // SAFETY: Clause is slice of Lit.
    //         Lit is a transparent newtype over NonZeroI32
    //         which is a transparent wrapper over i32
    //         which can be transmuted to u32.
    unsafe { std::mem::transmute(clause) }
}

fn data_to_clause(clause: &[u32]) -> Clause {
    // SAFETY: Clause is slice of Lit.
    //         Lit is a transparent newtype over NonZeroI32
    //         which is a transparent wrapper over i32
    //         which can be transmuted to u32.
    unsafe { std::mem::transmute(clause) }
}

fn data_to_clause_mut(clause: &mut [u32]) -> ClauseMut {
    // SAFETY: Clause is slice of Lit.
    //         Lit is a transparent newtype over NonZeroI32
    //         which is a transparent wrapper over i32
    //         which can be transmuted to u32.
    unsafe { std::mem::transmute(clause) }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ClauseIdx {
    pub(crate) start: u32,
    pub(crate) size: NonZeroU32,
    #[cfg(debug_assertions)]
    pub(crate) generation: u64,
}

#[derive(Debug, Clone)]
pub struct ClauseMeta {
    pub range: Range<u32>,
    /// LDB Value of the clause, if it was learned.
    /// Clauses provided by the input have no ldb value.
    pub ldb_glue: Option<NonZeroU32>,

    /// Is this clause considered useless?
    /// If so it will removed in the next sweep.
    pub is_garbage: bool,

    /// Is this clause used as the reason for a propagation.
    pub is_reason: bool,
}

#[derive(Clone, Default)]
pub struct ClauseDB {
    // This Vec serves as an arena where we allocate clauses.
    // each u32 represents a literal.
    // We use u32 instead of `Lit` because in the garbage collection phase
    // we store the offset into the new arena where the clause has moved, in order to be able
    // to efficiently update `ClauseIdx` which would be dangling otherwise.
    pub(crate) clause_data: Vec<u32>,
    pub(crate) clause_data_old: Vec<u32>,

    pub(crate) clause_meta: Vec<ClauseMeta>,

    /// Generation counter to detect if ClauseIdx are outdated.
    /// This field is incremented whenever we perform garbage collection.
    #[cfg(debug_assertions)]
    pub(crate) generation: u64,
}

// Make sure that Lit has the same size as u32, since we use this as the internal type to
// store lits in the ClauseDB.
// This is actually not sufficient because we need to ensure we can safely transmute between
// u32 and Lit, but since we don't have safe transmute yet this will have to suffice.
// (And its unlikely I'll ever want to change the internal representation of a Lit)
const _: () = if std::mem::size_of::<u32>() != std::mem::size_of::<Lit>() {
    panic!("Lit has unexpected size")
};

impl ClauseDB {
    pub fn insert_clause(&mut self, cls: Clause, ldb_glue: Option<NonZeroU32>) -> ClauseIdx {
        let start = self.clause_data.len();

        self.clause_data.extend(clause_to_data(cls));

        let end = self.clause_data.len();
        let size = end - start;

        debug_assert!(u32::try_from(start).is_ok());
        let start = start as u32;
        debug_assert!(u32::try_from(end).is_ok());
        let end = end as u32;
        debug_assert!(u32::try_from(size).is_ok());
        let size = size as u32;

        self.clause_meta.push(ClauseMeta {
            range: start..end,
            ldb_glue,
            is_garbage: false,
            is_reason: false,
        });
        ClauseIdx {
            start,
            size: NonZeroU32::new(size).expect("Insertion of empty clause."),
            #[cfg(debug_assertions)]
            generation: self.generation,
        }
    }

    #[allow(unused)]
    pub fn get(&self, r: ClauseIdx) -> Clause {
        debug_assert!(self.is_valid_clause_idx(r));

        let start = r.start as usize;
        let end = (r.start + r.size.get()) as usize;
        data_to_clause(&self.clause_data[start..end])
    }

    pub fn get_mut(&mut self, r: ClauseIdx) -> ClauseMut {
        debug_assert!(self.is_valid_clause_idx(r));

        let start = r.start as usize;
        let end = (r.start + r.size.get()) as usize;

        data_to_clause_mut(&mut self.clause_data[start..end])
    }

    fn is_valid_clause_idx(&self, r: ClauseIdx) -> bool {
        #[cfg(debug_assertions)]
        if self.generation != r.generation {
            tracing::debug!("clause index has invalid generation. ClauseDB generation: {}, clause idx generation: {}", self.generation, r.generation);
            return false;
        }

        let entry = self
            .clause_meta
            .binary_search_by_key(&r.start, |data| data.range.start);

        match entry {
            Ok(e) => {
                let range = self.clause_meta[e].range.clone();
                range.start == r.start && range.end == r.start + r.size.get()
            }
            Err(_) => false,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = Clause<'_>> {
        type Iter<'db> = impl Iterator<Item = Range<u32>> + 'db;

        struct ClauseIter<'db> {
            ranges: Iter<'db>,
            clauses: &'db [Lit],
        }

        impl<'db> Iterator for ClauseIter<'db> {
            type Item = Clause<'db>;

            fn next(&mut self) -> Option<Self::Item> {
                let range = self.ranges.next()?;
                Some(&self.clauses[range.start as usize..range.end as usize])
            }
        }

        ClauseIter {
            ranges: self.clause_meta.iter().map(|d| d.range.clone()),
            clauses: data_to_clause(&self.clause_data),
        }
    }

    pub fn iter_clause_meta_mut(&mut self) -> impl Iterator<Item = &mut ClauseMeta> + '_ {
        self.clause_meta.iter_mut()
    }

    pub(crate) fn get_meta_mut(&mut self, cls: ClauseIdx) -> &mut ClauseMeta {
        let entry = self
            .clause_meta
            .binary_search_by_key(&cls.start, |data| data.range.start);

        match entry {
            Ok(e) => &mut self.clause_meta[e],
            Err(_) => panic!("Invalid clause index"),
        }
    }
}
