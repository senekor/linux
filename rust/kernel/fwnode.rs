// SPDX-License-Identifier: GPL-2.0

//! Firmware device node object handle type definition.
//!
//! C header: [`include/linux/fwnode.h`](srctree/include/linux/fwnode.h)

use crate::{
    alloc::KVec,
    bindings,
    error::{to_result, Result},
    prelude::*,
    str::{CStr, CString},
    types::{Integer, Opaque},
};
use core::{
    ffi::c_void,
    mem::{self, MaybeUninit},
    ptr,
};

/// A reference-counted fwnode_handle.
///
/// This structure represents the Rust abstraction for a
/// C `struct fwnode_handle`. This implementation abstracts the usage of an
/// already existing C `struct fwnode_handle` within Rust code that we get
/// passed from the C side.
///
/// # Invariants
///
/// A `FwNode` instance represents a valid `struct fwnode_handle` created by the
/// C portion of the kernel.
///
/// Instances of this type are always reference-counted, that is, a call to
/// `fwnode_handle_get` ensures that the allocation remains valid at least until
/// the matching call to `fwnode_handle_put`.
#[repr(transparent)]
pub struct FwNode(Opaque<bindings::fwnode_handle>);

impl FwNode {
    /// Obtain the raw `struct fwnode_handle *`.
    pub(crate) fn as_raw(&self) -> *mut bindings::fwnode_handle {
        self.0.get()
    }

    /// Returns if a firmware property `name` is present
    pub fn property_present(&self, name: &CStr) -> bool {
        // SAFETY: `name` is non-null and null-terminated. `self.as_raw` is valid
        // because `self` is valid.
        unsafe { bindings::fwnode_property_present(self.as_raw(), name.as_ptr() as *const u8) }
    }

    /// Returns if a firmware property `name` is true or false
    pub fn property_read_bool(&self, name: &CStr) -> bool {
        // TODO: replace with fwnode_property_read_bool() which warns on non-bool properties
        // SAFETY: `name` is non-null and null-terminated. `self.as_raw` is valid
        // because `self` is valid.
        unsafe { bindings::fwnode_property_present(self.as_raw(), name.as_ptr() as *const u8) }
    }

    /// Returns the index of matching string `match_str` for firmware string property `name`
    pub fn property_read_string(&self, name: &CStr) -> Result<CString> {
        let mut str: *mut u8 = core::ptr::null_mut();
        let pstr: *mut *mut u8 = &mut str;

        // SAFETY: `name` is non-null and null-terminated. `self.as_raw` is
        // valid because `self` is valid.
        let ret = unsafe {
            bindings::fwnode_property_read_string(
                self.as_raw(),
                name.as_ptr() as *const u8,
                pstr as _,
            )
        };
        to_result(ret)?;

        // SAFETY: `pstr` contains a non-null ptr on success
        let str = unsafe { CStr::from_char_ptr(*pstr) };
        Ok(str.try_into()?)
    }

    /// Returns the index of matching string `match_str` for firmware string property `name`
    pub fn property_match_string(&self, name: &CStr, match_str: &CStr) -> Result<usize> {
        // SAFETY: `name` and `match_str` are non-null and null-terminated. `self.as_raw` is
        // valid because `self` is valid.
        let ret = unsafe {
            bindings::fwnode_property_match_string(
                self.as_raw(),
                name.as_ptr() as *const u8,
                match_str.as_ptr() as *const u8,
            )
        };
        to_result(ret)?;
        Ok(ret as usize)
    }

    /// Returns firmware property `name` integer scalar value
    pub fn property_read<T: Integer>(&self, name: &CStr, default: Option<T>) -> Result<T> {
        let default = default.map(|default| [default; 1]);

        let val = Self::property_read_array(self, name, default)?;
        Ok(val[0])
    }

    /// Returns firmware property `name` integer array values
    pub fn property_read_array<T: Integer, const N: usize>(
        &self,
        name: &CStr,
        default: Option<[T; N]>,
    ) -> Result<[T; N]> {
        let val: [MaybeUninit<T>; N] = [const { MaybeUninit::uninit() }; N];

        // SAFETY: `name` is non-null and null-terminated. `self.as_raw` is valid
        // because `self` is valid. `val.as_ptr` is valid because `val` is valid.
        let ret = unsafe {
            bindings::fwnode_property_read_int_array(
                self.as_raw(),
                name.as_ptr() as *const u8,
                T::SIZE.try_into().unwrap(),
                val.as_ptr() as *mut c_void,
                val.len(),
            )
        };

        let val: [T; N] = match ret {
            // SAFETY: `val` is always initialized when fwnode_property_read_int_array
            // is successful.
            0 => unsafe { mem::transmute_copy(&val) },
            _ => match default {
                Some(default) => default,
                None => return Err(Error::from_errno(ret)),
            },
        };
        Ok(val)
    }

    /// Returns firmware property `name` integer array values in a KVec
    pub fn property_read_array_vec<T: Integer>(&self, name: &CStr, len: usize) -> Result<KVec<T>> {
        let mut val: KVec<T> = KVec::with_capacity(len, GFP_KERNEL)?;

        // SAFETY: `name` is non-null and null-terminated. `self.as_raw` is valid
        // because `self` is valid. `val.as_ptr` is valid because `val` is valid.
        to_result(unsafe {
            bindings::fwnode_property_read_int_array(
                self.as_raw(),
                name.as_ptr() as *const u8,
                T::SIZE.try_into().unwrap(),
                val.as_ptr() as *mut c_void,
                len,
            )
        })?;

        // SAFETY: fwnode_property_read_int_array() writes exactly `len` entries on success
        unsafe { val.set_len(len) }
        Ok(val)
    }

    /// Returns integer array length for firmware property `name`
    pub fn property_count_elem<T: Integer>(&self, name: &CStr) -> Result<usize> {
        // SAFETY: `name` is non-null and null-terminated. `self.as_raw` is valid
        // because `self` is valid. Passing null pointer buffer is valid to obtain
        // the number of elements in the property array.
        let ret = unsafe {
            bindings::fwnode_property_read_int_array(
                self.as_raw(),
                name.as_ptr() as *const u8,
                T::SIZE.try_into().unwrap(),
                ptr::null_mut(),
                0,
            )
        };
        to_result(ret)?;
        Ok(ret.try_into().unwrap())
    }
}

// SAFETY: Instances of `FwNode` are always reference-counted.
unsafe impl crate::types::AlwaysRefCounted for FwNode {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference guarantees that the refcount is non-zero.
        unsafe { bindings::fwnode_handle_get(self.as_raw()) };
    }

    unsafe fn dec_ref(obj: ptr::NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::fwnode_handle_put(obj.cast().as_ptr()) }
    }
}
