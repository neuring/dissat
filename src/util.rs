pub use remainder::{remaining, Remaining};

mod remainder {
    use std::marker::PhantomData;

    pub struct Remaining<'a, T> {
        used_index: usize,
        data: *mut T,
        len: usize,
        _marker: PhantomData<&'a mut ()>,
    }

    impl<'a, T> Remaining<'a, T> {
        /// Get the value at position `i` from the underlying slice.
        /// Returns `None` if the index is out-of-bounds or the same index
        /// as used on the call to `remaining`.
        pub fn get_mut<'b>(&'b mut self, i: usize) -> Option<&'b mut T> {
            unsafe {
                if self.used_index != i && i < self.len {
                    Some(&mut *self.data.add(i))
                } else {
                    None
                }
            }
        }

        pub fn get<'b>(&'b self, i: usize) -> Option<&'b T> {
            unsafe {
                if self.used_index != i && i < self.len {
                    Some(&*self.data.add(i))
                } else {
                    None
                }
            }
        }
    }

    /// Retrieve a value form a slice, but allow accessing the remaining elements using
    /// the returned `Remaining` object.
    pub fn remaining<T>(slice: &mut [T], i: usize) -> Option<(&mut T, Remaining<'_, T>)> {
        unsafe {
            let len = slice.len();
            let ptr = slice.as_mut_ptr();

            if i >= len {
                return None;
            }
            let value = &mut *ptr.add(i);
            let remaining = Remaining {
                used_index: i,
                data: ptr,
                len,
                _marker: PhantomData,
            };
            Some((value, remaining))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn remaining_test() {
            let mut data = vec![1, 2, 3, 4];

            let (val, mut remaining) = remaining(&mut data, 2).unwrap();
            assert_eq!(*val, 3);

            let val1 = remaining.get(0).unwrap();
            assert_eq!(*val1, 1);

            let val2 = remaining.get(1).unwrap();
            assert_eq!(*val2, 2);

            assert!(remaining.get(2).is_none());

            let val4 = remaining.get(3).unwrap();
            assert_eq!(*val4, 4);

            assert!(remaining.get(4).is_none());
        }
    }
}
