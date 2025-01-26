// SPDX-License-Identifier: GPL-2.0

//! Firmware device node object handle type definition.
//!
//! C header: [`include/linux/fwnode.h`](srctree/include/linux/fwnode.h)

use crate::{
    alloc::KVec,
    arrayvec::ArrayVec,
    bindings,
    error::{to_result, Result},
    prelude::*,
    str::{CStr, CString},
    types::{ARef, Integer, Opaque},
};
use core::{
    ffi::c_void,
    mem::{self, MaybeUninit},
    ptr::{self, NonNull},
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

    // SAFETY: `raw` must have its refcount incremented.
    unsafe fn from_raw(raw: *mut bindings::fwnode_handle) -> ARef<Self> {
        unsafe { ARef::from_raw(NonNull::new_unchecked(raw.cast())) }
    }

    pub fn get_child_by_name(&self, name: &CStr) -> Option<ARef<Self>> {
        // SAFETY: `self` and `name` are valid.
        let child =
            unsafe { bindings::fwnode_get_named_child_node(self.as_raw(), name.as_char_ptr()) };
        if child.is_null() {
            return None;
        }
        // SAFETY: `fwnode_get_named_child_node` returns a pointer with refcount incremented.
        Some(unsafe { Self::from_raw(child) })
    }

    pub fn children<'a>(&'a self) -> impl Iterator<Item = ARef<FwNode>> + 'a {
        struct Children<'a> {
            parent: &'a FwNode,
            prev: Option<ARef<FwNode>>,
        }

        impl<'a> Iterator for Children<'a> {
            type Item = ARef<FwNode>;

            fn next(&mut self) -> Option<Self::Item> {
                let prev = match self.prev.take() {
                    None => core::ptr::null_mut(),
                    Some(prev) => {
                        // We will pass `prev` to `fwnode_get_next_child_node`,
                        // which decrements its refcount, so we use
                        // `ARef::into_raw` to avoid decrementing the refcount
                        // twice.
                        let prev = ARef::into_raw(prev);
                        prev.as_ptr().cast()
                    }
                };
                let next =
                    unsafe { bindings::fwnode_get_next_child_node(self.parent.as_raw(), prev) };
                if next.is_null() {
                    return None;
                }
                // SAFETY: `fwnode_get_next_child_node` returns a pointer with
                // refcount incremented.
                let next = unsafe { FwNode::from_raw(next) };
                self.prev = Some(next.clone());
                Some(next)
            }
        }

        Children {
            parent: self,
            prev: None,
        }
    }

    pub fn property_get_reference_args(
        &self,
        prop: &CStr,
        nargs: NArgs<'_>,
        index: u32,
    ) -> Result<(
        ARef<Self>,
        ArrayVec<{ bindings::NR_FWNODE_REFERENCE_ARGS as usize }, u64>,
    )> {
        let mut out_args = bindings::fwnode_reference_args::default();

        let (nargs_prop, nargs) = match nargs {
            NArgs::Prop(nargs_prop) => (nargs_prop.as_char_ptr(), 0),
            NArgs::N(nargs) => (ptr::null(), nargs),
        };

        let ret = unsafe {
            bindings::fwnode_property_get_reference_args(
                self.0.get(),
                prop.as_char_ptr(),
                nargs_prop,
                nargs,
                index,
                &mut out_args,
            )
        };
        to_result(ret)?;

        let node = unsafe { FwNode::from_raw(out_args.fwnode) };
        let mut args = ArrayVec::default();

        for i in 0..out_args.nargs {
            args.push(out_args.args[i as usize]);
        }

        Ok((node, args))
    }
}

pub enum NArgs<'a> {
    Prop(&'a CStr),
    N(u32),
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
