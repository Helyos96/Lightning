use std::mem::MaybeUninit;
use std::ops::{Deref, DerefMut};
use std::{fmt, ptr, slice};

#[macro_export]
macro_rules! stackvec {
    ( $( $elem:expr ),* $(,)? ) => {{
        let mut vec = crate::stackvec::StackVec::new();
        // We unwrap here for simplicity, but you could handle capacity errors more gracefully.
        $( vec.push($elem); )*
        vec
    }};
}

/// A fixed-capacity, stack-allocated vector that can be `Copy` if `T` is `Copy`.
#[derive(Clone, Copy)]
pub struct StackVec<T: Copy, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
}

impl<T: Copy, const N: usize> StackVec<T, N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: unsafe { MaybeUninit::uninit().assume_init() },
        }
    }

    pub fn push(&mut self, value: T) {
        debug_assert!(self.len < N);
        self.data[self.len] = MaybeUninit::new(value);
        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(unsafe { self.data[self.len].assume_init() })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn extend_from_slice(&mut self, src: &[T]) {
        let available = N - self.len;
        debug_assert!(available >= src.len());

        unsafe {
            let dst_ptr = self.data.as_mut_ptr().add(self.len) as *mut T;
            let src_ptr = src.as_ptr();
            ptr::copy_nonoverlapping(src_ptr, dst_ptr, src.len());
        }

        self.len += src.len();
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        let ptr = self.data.as_ptr() as *const T;
        unsafe { std::slice::from_raw_parts(ptr, self.len) }.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        let ptr = self.data.as_mut_ptr() as *mut T;
        unsafe { std::slice::from_raw_parts_mut(ptr, self.len) }.iter_mut()
    }
}

impl<T: Copy, const N: usize> std::ops::Index<usize> for StackVec<T, N> {
    type Output = T;
    fn index(&self, idx: usize) -> &Self::Output {
        debug_assert!(idx < self.len);
        unsafe { &*self.data[idx].as_ptr() }
    }
}

impl<T: Copy, const N: usize> std::ops::IndexMut<usize> for StackVec<T, N> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        debug_assert!(idx < self.len);
        unsafe { &mut *self.data[idx].as_mut_ptr() }
    }
}

impl<T: Copy, const N: usize> Default for StackVec<T, N> {
    fn default() -> Self { Self::new() }
}

impl<T: Copy, const N: usize> Deref for StackVec<T, N> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe {
            let ptr = self.data.as_ptr() as *const T;
            slice::from_raw_parts(ptr, self.len)
        }
    }
}

impl<T: Copy, const N: usize> DerefMut for StackVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let ptr = self.data.as_mut_ptr() as *mut T;
            slice::from_raw_parts_mut(ptr, self.len)
        }
    }
}

impl<T: fmt::Debug + Copy, const N: usize> fmt::Debug for StackVec<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dbg_list = f.debug_list();
        for item in self.iter() {
            dbg_list.entry(item);
        }
        dbg_list.finish()
    }
}

impl<'a, T: Copy, const N: usize> IntoIterator for &'a StackVec<T, N> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[test]
fn test_stackvec() {
    let mut v = StackVec::<u32, 6>::new();
    v.push(10);
    v.push(20);
    v.push(30);

    let v2 = v;
    v.extend_from_slice(&v2);

    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    assert_eq!(v[2], 30);
    assert_eq!(v[3], 10);
    assert_eq!(v[4], 20);
    assert_eq!(v[5], 30);
}