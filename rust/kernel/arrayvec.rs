// SPDX-License-Identifier: GPL-2.0

//! The `arrayvec` crate.
//!
//! Provides [ArrayVec], a stack-allocated vector with statically fixed capacity.

use core::mem::MaybeUninit;

/// A stack-allocated vector with statically fixed capacity.
///
/// This can be useful to avoid heap allocation and still ensure safety where a small but dynamic number
/// of elements is needed.
///
/// For example, consider a function that returns a variable number of values, but no more than 8.
/// In C, one might achieve this by passing a pointer to a stack-allocated array as an out-parameter and making the function return the actual number of elements.
/// This is not safe, because nothing prevents the caller from reading elements from the array that weren't actually initialized by the function.
/// `ArrayVec` solves this problem.
/// You can either return it directly from a function or still pass a `&mut ArrayVec` as an out-parameter.
/// Users are prevented from accessing uninitialized elements.
///
/// This basically exists already (in a much more mature form) on crates.io:
/// https://crates.io/crates/arrayvec
#[derive(Debug)]
pub struct ArrayVec<const N: usize, T> {
    array: [core::mem::MaybeUninit<T>; N],
    len: usize,
}

impl<const N: usize, T> ArrayVec<N, T> {
    pub fn push(&mut self, elem: T) {
        if self.len == N {
            panic!("OOM")
        }
        self.array[self.len] = MaybeUninit::new(elem);
        self.len += 1;
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<const N: usize, T> Default for ArrayVec<N, T> {
    fn default() -> Self {
        Self {
            array: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }
}

impl<const N: usize, T> AsRef<[T]> for ArrayVec<N, T> {
    fn as_ref(&self) -> &[T] {
        // SAFETY: As per the type invariant, all elements at index < self.len
        // are initialized.
        unsafe { core::mem::transmute(&self.array[..self.len]) }
    }
}

impl<const N: usize, T> AsMut<[T]> for ArrayVec<N, T> {
    fn as_mut(&mut self) -> &mut [T] {
        // SAFETY: As per the type invariant, all elements at index < self.len
        // are initialized.
        unsafe { core::mem::transmute(&mut self.array[..self.len]) }
    }
}

impl<const N: usize, T> Drop for ArrayVec<N, T> {
    fn drop(&mut self) {
        unsafe {
            let slice: &mut [T] =
                core::slice::from_raw_parts_mut(self.array.as_mut_ptr().cast(), self.len);
            core::ptr::drop_in_place(slice);
        }
    }
}
