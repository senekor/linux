// SPDX-License-Identifier: GPL-2.0

//! GPIO consumer API

use crate::{
    device::Device,
    error::{code::*, from_err_ptr, Result},
    str::CStr,
};
use core::ptr::NonNull;

/// Flags that can be passed to passed to configure direction and output value.
#[derive(Copy, Clone)]
#[repr(u32)]
pub enum Flags {
    /// Don't change anything.
    AsIs = bindings::gpiod_flags_GPIOD_ASIS,
    /// Set lines to input mode.
    In = bindings::gpiod_flags_GPIOD_IN,
    /// Set lines to output and drive them low.
    OutLow = bindings::gpiod_flags_GPIOD_OUT_LOW,
    /// Set lines to output and drive them high.
    OutHigh = bindings::gpiod_flags_GPIOD_OUT_HIGH,
    /// Set lines to open-drain output and drive them low.
    OutLowOpenDrain = bindings::gpiod_flags_GPIOD_OUT_LOW_OPEN_DRAIN,
    /// Set lines to open-drain output and drive them high.
    OutHighOpenDrain = bindings::gpiod_flags_GPIOD_OUT_HIGH_OPEN_DRAIN,
}

pub struct Desc(NonNull<bindings::gpio_desc>);

impl Desc {
    /// Obtain a GPIO for a given GPIO function.
    ///
    /// See [gpiod_get](`https://docs.kernel.org/driver-api/gpio/index.html#c.gpiod_get`)
    pub fn get(dev: &Device, con_id: &'static CStr, flags: Flags) -> Result<Self> {
        let desc = from_err_ptr(unsafe {
            bindings::gpiod_get(dev.as_raw(), con_id.as_char_ptr(), flags as _)
        })?;

        Ok(Self(NonNull::new(desc).ok_or(EINVAL)?))
    }

    /// Obtain an optional GPIO for a given GPIO function.
    ///
    /// See [gpiod_get_optional](`https://docs.kernel.org/driver-api/gpio/index.html#c.gpiod_get_optional`)
    pub fn get_optional(dev: &Device, con_id: &'static CStr, flags: Flags) -> Result<Option<Self>> {
        let desc = from_err_ptr(unsafe {
            bindings::gpiod_get_optional(dev.as_raw(), con_id.as_char_ptr(), flags as _)
        })?;

        if desc.is_null() {
            return Ok(None);
        }

        Ok(Some(Self(unsafe { NonNull::new_unchecked(desc) })))
    }

    /// Assign a GPIO's value.
    ///
    /// See [gpiod_set_value](`https://docs.kernel.org/driver-api/gpio/index.html#c.gpiod_set_value`)
    pub fn set_value(&mut self, value: i32) {
        // SAFETY: Type invariants insures that `self.0` is a valid and non-null pointer, hence it
        // is safe to perform this FFI function call.
        unsafe { bindings::gpiod_set_value(self.0.as_ptr(), value) }
    }

    /// Assign a GPIO's value.
    ///
    /// See [gpiod_set_value_cansleep](`https://docs.kernel.org/driver-api/gpio/index.html#c.gpiod_set_value_cansleep`)
    pub fn set_value_cansleep(&mut self, value: i32) {
        // SAFETY: Type invariants insures that `self.0` is a valid and non-null pointer, hence it
        // is safe to perform this FFI function call.
        unsafe { bindings::gpiod_set_value_cansleep(self.0.as_ptr(), value) }
    }
}

impl Drop for Desc {
    fn drop(&mut self) {
        unsafe { bindings::gpiod_put(self.0.as_ptr()) }
    }
}

unsafe impl Send for Desc {}
