/// Clauses are stores continously in memory.
/// Each clause has atleast two literals.
/// The first two literals are watched.
/// A variable can only appear once in a clause.
use std::{num::NonZeroU32, ops::Range};

use crate::Lit;

pub type Clause<'db> = &'db [Lit];
pub type ClauseMut<'db> = &'db mut [Lit];

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ClauseIdx {
    start: u32,
    size: NonZeroU32,
}
#[derive(Clone, Default)]
pub struct ClauseDB {
    clause_data: Vec<Lit>,
    clause_ranges: Vec<Range<u32>>,
}

impl ClauseDB {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert_clause(&mut self, cls: &[Lit]) -> ClauseIdx {
        let start = self.clause_data.len();

        self.clause_data.extend(cls);

        let end = self.clause_data.len();
        let size = end - start;

        debug_assert!(<usize as TryInto<u32>>::try_into(start).is_ok());
        let start = start as u32;
        debug_assert!(<usize as TryInto<u32>>::try_into(end).is_ok());
        let end = end as u32;
        debug_assert!(<usize as TryInto<u32>>::try_into(size).is_ok());
        let size = size as u32;

        self.clause_ranges.push(start..end);
        ClauseIdx {
            start,
            size: NonZeroU32::new(size).expect("Insertion of empty clause."),
        }
    }

    pub fn get<'db>(&'db self, r: ClauseIdx) -> Clause<'db> {
        debug_assert!(self.is_valid_clause_idx(r));

        let start = r.start as usize;
        let end = (r.start + r.size.get()) as usize;

        &self.clause_data[start..end]
    }

    pub fn get_mut<'db>(&'db mut self, r: ClauseIdx) -> ClauseMut<'db> {
        debug_assert!(self.is_valid_clause_idx(r));

        let start = r.start as usize;
        let end = (r.start + r.size.get()) as usize;

        &mut self.clause_data[start..end]
    }

    fn is_valid_clause_idx(&self, r: ClauseIdx) -> bool {
        let entry = self
            .clause_ranges
            .binary_search_by_key(&r.start, |range| range.start);

        match entry {
            Ok(e) => {
                let range = self.clause_ranges[e].clone();
                range.start == r.start && range.end == r.start + r.size.get()
            }
            Err(_) => false,
        }
    }

    pub fn iter<'db>(&'db self) -> impl Iterator<Item = Clause<'db>> {
        struct ClauseIter<'db> {
            ranges: std::slice::Iter<'db, Range<u32>>,
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
            ranges: self.clause_ranges.iter(),
            clauses: &self.clause_data,
        }
    }
}
