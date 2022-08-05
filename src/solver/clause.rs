use bitflags::bitflags;
/// A clause consists of some metadata and its literals.
/// Concretely, some flags about the state of the clause, the number of literals, its lbd glue value.
/// Each can be represented as a u32 and we store all clauses continously in a Vec<u32>.
/// To access a clause one can use `ClauseIdx` which is wrapper over a usize which is the offset into the Vec.
/// We can then transmute the individual components of a clause to their respective type.
/// In order to ensure that a ClauseIdx is valid we use to methods.
///
/// 1. Only the first u32 of Clause data (which stores the flags) has the most significant bit set.
///    If a clause index every points to a u32 without this bit set, we know it is invalid.
///    This is only a branch that should be always true, and therefore efficient.
///
/// 2. In debug builds we have an additional generation counter which helps with temporal correctness.
///    After each garbage collection phase this counter is incremented.
///    Additionally, every ClauseIdx has a generation counter.
///    On every access if both counters disagree we know that the index is out of date.
///    Because the counter doubles the size of the ClauseIdx we only use this in debug builds.
///    We remain memory safe and without undefined behaviour even without this generation mechanism.
///
use std::num::NonZeroU32;

use super::Lit;

#[derive(Default)]
pub(crate) struct ClauseDB {
    clause_data: Vec<u32>,
    clause_data_old: Vec<u32>,

    #[cfg(debug_assertions)]
    generation: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Clause<'a> {
    data: &'a [u32],
}

#[derive(PartialEq, Eq)]
pub struct ClauseMut<'a> {
    data: &'a mut [u32],
}

bitflags! {
    #[repr(transparent)]
    struct FlagsInternal : u32 {
        const CLAUSE_BEGIN_SENTINEL = (1 << 31); // This should be always one.
        const IS_GARBAGE = (1 << 0);
        const IS_REASON = (1 << 1);
    }
}

#[repr(transparent)]
pub(crate) struct Flags(FlagsInternal);

impl Flags {
    pub fn new() -> Self {
        Self(FlagsInternal::CLAUSE_BEGIN_SENTINEL)
    }

    pub fn set_is_reason(&mut self, value: bool) {
        self.0.set(FlagsInternal::IS_REASON, value);
    }

    pub fn set_is_garbage(&mut self, value: bool) {
        self.0.set(FlagsInternal::IS_GARBAGE, value);
    }

    pub fn is_reason(&self) -> bool {
        self.0.contains(FlagsInternal::IS_REASON)
    }

    pub fn is_garbage(&self) -> bool {
        self.0.contains(FlagsInternal::IS_GARBAGE)
    }
}

impl std::fmt::Debug for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClauseIdx {
    offset: usize,
    #[cfg(debug_assertions)]
    generation: u64,
}

#[repr(usize)]
enum Offsets {
    Flags = 0,
    Len,
    Glue,
    // This is always the last Variant, because lits are at the end of a clause.
    Lits,
}

fn clause_length_with_data(num_lits: usize) -> usize {
    num_lits + Offsets::Lits as usize
}

impl<'a> Clause<'a> {
    pub fn len(&self) -> u32 {
        self.data[Offsets::Len as usize]
    }

    pub fn lits(&self) -> &'a [Lit] {
        let len = self.len();
        unsafe {
            let ptr = self.data.as_ptr();
            let ptr = ptr.add(Offsets::Lits as usize);
            let ptr = ptr.cast::<Lit>();
            std::slice::from_raw_parts(ptr, len as usize)
        }
    }

    pub fn glue(&self) -> Option<NonZeroU32> {
        NonZeroU32::new(self.data[Offsets::Glue as usize])
    }

    pub fn flags(&self) -> Flags {
        let meta = self.data[Offsets::Flags as usize];
        unsafe { Flags(FlagsInternal::from_bits_unchecked(meta)) }
    }
}

impl<'a> std::ops::Deref for Clause<'a> {
    type Target = [Lit];

    fn deref(&self) -> &Self::Target {
        self.lits()
    }
}

impl<'a> IntoIterator for Clause<'a> {
    type Item = Lit;

    type IntoIter = impl Iterator<Item = Self::Item> + 'a;

    fn into_iter(self) -> Self::IntoIter {
        self.lits().iter().copied()
    }
}

impl std::fmt::Debug for Clause<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clause")
            .field("flags", &self.flags())
            .field("lbd_glue", &self.glue())
            .field("lits", &self.lits())
            .finish()
    }
}

fn debug_assert_msb_is_one(x: u32) -> u32 {
    debug_assert_eq!(x & (1 << 31), 1);
    x
}

fn debug_assert_msb_is_zero(x: u32) -> u32 {
    debug_assert_eq!(x & (1 << 31), 0);
    x
}

impl<'a> ClauseMut<'a> {
    pub fn len(&self) -> u32 {
        let len = self.data[Offsets::Len as usize];
        debug_assert_eq!(len as usize + Offsets::Lits as usize, self.data.len());
        len
    }

    pub fn lits(&mut self) -> &'a mut [Lit] {
        let len = self.len();
        unsafe {
            let ptr = self.data.as_mut_ptr();
            let ptr = ptr.add(Offsets::Lits as usize);
            let ptr = ptr.cast::<Lit>();
            std::slice::from_raw_parts_mut(ptr, len as usize)
        }
    }

    pub fn glue(&mut self) -> &'a mut Option<NonZeroU32> {
        let glue = &mut self.data[Offsets::Glue as usize];
        unsafe { std::mem::transmute(glue) }
    }

    pub fn flags(&mut self) -> &'a mut Flags {
        let meta = &mut self.data[Offsets::Flags as usize];
        unsafe { std::mem::transmute(meta) }
    }
}

impl<'a> std::ops::Deref for ClauseMut<'a> {
    type Target = [Lit];

    fn deref(&self) -> &Self::Target {
        &*self.lits()
    }
}

impl<'a> std::ops::DerefMut for ClauseMut<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.lits()
    }
}

impl std::fmt::Debug for ClauseMut<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Clause")
            .field("flags", &self.flags())
            .field("lbd_glue", &self.glue())
            .field("lits", &self.lits())
            .finish()
    }
}

impl PartialEq<Clause<'_>> for ClauseMut<'_> {
    fn eq(&self, other: &Clause<'_>) -> bool {
        self.data == other.data
    }
}

impl PartialEq<ClauseMut<'_>> for Clause<'_> {
    fn eq(&self, other: &ClauseMut<'_>) -> bool {
        self.data == other.data
    }
}

impl ClauseDB {
    pub fn new() -> Self {
        Self {
            clause_data: Vec::new(),
            clause_data_old: Vec::new(),

            #[cfg(debug_assertions)]
            generation: 0,
        }
    }

    pub fn insert_clause(&mut self, lits: &[Lit], glue: Option<NonZeroU32>) -> ClauseIdx {
        let start = self.clause_data.len();

        let clause_len = lits.len() as u32;
        assert!(clause_len & (1 << 31) == 0);
        let clause_glue = glue.map_or(0, NonZeroU32::get);
        let clause_flags = FlagsInternal::CLAUSE_BEGIN_SENTINEL;

        self.clause_data
            .reserve(clause_length_with_data(clause_len as usize));

        self.clause_data.push(clause_flags.bits());
        self.clause_data.push(debug_assert_msb_is_zero(clause_len));
        self.clause_data.push(debug_assert_msb_is_zero(clause_glue));
        self.clause_data.extend(Lit::lit_slice_to_u32_slice(lits));

        ClauseIdx {
            offset: start,
            generation: self.generation,
        }
    }

    fn assert_valid_clause_index(&self, clause_idx: ClauseIdx) {
        #[cfg(debug_assertions)]
        self.assert_valid_generation(clause_idx.generation);

        Self::assert_valid_offset(&self.clause_data, clause_idx.offset);
    }

    fn assert_valid_offset(clause_data: &[u32], offset: usize) {
        let flags = FlagsInternal::from_bits_truncate(clause_data[offset]);
        assert!(flags.contains(FlagsInternal::CLAUSE_BEGIN_SENTINEL));
    }

    fn assert_valid_generation(&self, generation: u64) {
        #[cfg(debug_assertions)]
        debug_assert_eq!(
            self.generation, generation,
            "Invalid generation. current {}, got {generation}",
            self.generation
        );
    }

    pub fn get(&self, clause_idx: ClauseIdx) -> Clause {
        self.assert_valid_clause_index(clause_idx);

        let data_range = Self::clause_data_range(&self.clause_data, clause_idx.offset);
        let data = &self.clause_data[data_range];

        Clause { data }
    }

    pub fn get_mut(&mut self, clause_idx: ClauseIdx) -> ClauseMut {
        self.assert_valid_clause_index(clause_idx);

        let data_range = Self::clause_data_range(&self.clause_data, clause_idx.offset);
        let data = &mut self.clause_data[data_range];

        ClauseMut { data }
    }

    pub fn iter(&self) -> impl Iterator<Item = Clause<'_>> {
        struct ClauseIter<'a> {
            clause_data: &'a [u32],
            next_offset: usize,
        }

        impl<'a> Iterator for ClauseIter<'a> {
            type Item = Clause<'a>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.next_offset >= self.clause_data.len() {
                    return None;
                }

                #[cfg(debug_assertions)]
                ClauseDB::assert_valid_offset(&self.clause_data, self.next_offset);

                let data_range = ClauseDB::clause_data_range(&self.clause_data, self.next_offset);
                let data = &self.clause_data[data_range];
                self.next_offset += data.len();

                Some(Clause { data })
            }
        }

        ClauseIter {
            clause_data: &self.clause_data,
            next_offset: 0,
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = ClauseMut<'_>> {
        struct ClauseIter<'a> {
            clause_data: &'a mut [u32],
            next_offset: usize,
        }

        impl<'a> Iterator for ClauseIter<'a> {
            type Item = ClauseMut<'a>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.next_offset >= self.clause_data.len() {
                    return None;
                }

                #[cfg(debug_assertions)]
                ClauseDB::assert_valid_offset(&self.clause_data, self.next_offset);

                let data_range = ClauseDB::clause_data_range(&self.clause_data, self.next_offset);
                let data = &mut self.clause_data[data_range];
                self.next_offset += data.len();

                Some(ClauseMut { data })
            }
        }

        ClauseIter {
            clause_data: &mut self.clause_data,
            next_offset: 0,
        }
    }

    /// Retrieves the data range of a clause at offset.
    /// This function doesn't check if the offset is valid.
    fn clause_data_range(clause_data: &[u32], offset: usize) -> std::ops::Range<usize> {
        let lits_len = clause_data[offset + Offsets::Len as usize];
        let data_len = clause_length_with_data(lits_len as usize);
        offset..offset + data_len
    }

    /// Remove all clauses that are marked as is_garbage in their Flags.
    pub(crate) fn collect_garbage(&mut self) {
        let new_arena = &mut self.clause_data_old;
        new_arena.clear();

        let mut current_pos = 0;

        while current_pos < self.clause_data.len() {
            #[cfg(debug_assertions)]
            ClauseDB::assert_valid_offset(&self.clause_data, current_pos);

            let data_range = Self::clause_data_range(&self.clause_data, current_pos);
            current_pos += data_range.len();

            let is_garbage = FlagsInternal::from_bits_truncate(
                self.clause_data[data_range][Offsets::Flags as usize],
            )
            .contains(FlagsInternal::IS_GARBAGE);

            if is_garbage {
                // Sentinel value to remember that this clause has been removed.
                self.clause_data[data_range][Offsets::Len as usize] = u32::MAX;
            } else {
                let new_clause_offset = new_arena.len() as u32;

                // Sentinel value to remember that this clause has been removed.
                self.clause_data[data_range][Offsets::Len as usize] = new_clause_offset;

                new_arena.extend(&self.clause_data[data_range]);
            }
        }

        std::mem::swap(&mut self.clause_data, &mut self.clause_data_old);
    }

    pub(crate) fn update_old_clause_index(&self, idx: &mut ClauseIdx) -> bool {
        #[cfg(debug_assertions)]
        debug_assert_eq!(idx.generation + 1, self.generation);

        ClauseDB::assert_valid_offset(&self.clause_data_old, idx.offset);

        // After calling `ClauseDB::collect_garbage` the clause data buffer are swapped and the old
        // buffer contains the offsets of retained clauses in the new buffer.
        let new_pos = self.clause_data_old[idx.offset..][Offsets::Len as usize];

        if new_pos == u32::MAX {
            // ClauseIdx should be removed wherever it is contained.
            // We don't update the generation, so its detected if this index is ever used.
            false
        } else {
            idx.offset = new_pos as usize; // TODO: change clause idx offset from usize to u32
            #[cfg(debug_assertions)]
            {
                idx.generation += 1;
            }
            true
        }
    }
}
