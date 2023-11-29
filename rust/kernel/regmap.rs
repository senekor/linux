// SPDX-License-Identifier: GPL-2.0

//! Register map access API.
//!
//! C header: [`include/linux/regmap.h`](srctree/include/linux/regmap.h)
//!
//! # Examples
//!
//! ```ignore
//! regmap::define_regmap_field_descs!(FIELD_DESCS, {
//!     (pid, 0x3, READ, { value => raw([7:0], ro) }),
//!     (limconf, 0x16, RW, {
//!         rearm     => bit(0, rw),
//!         rststatus => bit(1, rw),
//!         tpwth     => enum([5:4], rw, {
//!             Temp83C  = 0x0,
//!             Temp94C  = 0x1,
//!             Temp105C  = 0x2,
//!             Temp116C  = 0x3,
//!         }),
//!     })
//! });
//!
//! fn probe(client: &mut i2c::Client) -> Result {
//!     let config = regmap::Config::<AccessOps>::new(8, 8)
//!         .with_max_register(0x16)
//!         .with_cache_type(regmap::CacheType::RbTree);
//!     let regmap = regmap::Regmap::init_i2c(client, &config);
//!     let mut fields = regmap.alloc_fields(&FIELD_DESCS)?;
//!
//!     dev_info!(client.as_ref(), "PID: {:#x}", pid::value::read(&mut fields)?);
//! }
//! ```

use crate::{
    bindings,
    error::{code::*, to_result, Error, Result},
    macros::paste,
    sync::Arc,
};
#[cfg(CONFIG_REGMAP_I2C = "y")]
use crate::{error::from_err_ptr, i2c};
use core::{marker::PhantomData, ptr::NonNull};

/// Type of caching
#[repr(u32)]
pub enum CacheType {
    /// Don't cache anything
    None = bindings::regcache_type_REGCACHE_NONE,
    /// Use RbTree caching
    RbTree = bindings::regcache_type_REGCACHE_RBTREE,
    /// Use Flat caching
    Flat = bindings::regcache_type_REGCACHE_FLAT,
    /// Use Maple caching
    Maple = bindings::regcache_type_REGCACHE_MAPLE,
}

/// Register map
///
/// Note for Rust abstractions using Regmap:
/// Regmap C structure does not implement reference count, so in order to keep the abstractions
/// safe it is essential to keep a `Arc<Regmap>` instance whenever the associated C API is holding
/// on the `struct regmap` pointer.
///
/// # Invariants
///
/// * `self.0` is valid, non-zero, and the memory is owned by `self`.
/// * This abstraction does not allow to disable regmap locking.
pub struct Regmap(NonNull<bindings::regmap>);

impl Regmap {
    #[cfg(CONFIG_REGMAP_I2C = "y")]
    /// Initialize a [`Regmap`] instance for an `i2c` client.
    pub fn init_i2c<T: ConfigOps>(i2c: &i2c::Client, config: &Config<T>) -> Result<Self> {
        // SAFETY: Type invariants guarantee that `i2c.as_raw` is valid and non-null and
        // the Config type invariant guarantee that `config.raw` always contains valid data.
        let regmap = from_err_ptr(unsafe { bindings::regmap_init_i2c(i2c.as_raw(), &config.raw) })?;

        Ok(Regmap(NonNull::new(regmap).ok_or(EINVAL)?))
    }

    /// Return the raw pointer of this regmap.
    pub fn as_raw(&self) -> *mut bindings::regmap {
        self.0.as_ptr()
    }
}

impl Drop for Regmap {
    fn drop(&mut self) {
        // SAFETY: By the type invariant, `self.as_raw` is a valid pointer and it can be freed
        // because we own the memory.
        unsafe { bindings::regmap_exit(self.as_raw()) }
    }
}

// SAFETY: The type invariants guarantee that the memory of `bindings::regmap` is owned and
// guarantee that the C API is using locked accesses.
unsafe impl Send for Regmap {}

/// Field Descriptors
///
/// FieldDescriptors can be created by calling the [`define_regmap_field_descs`] macro.
///
/// # Examples
///
/// ```ignore
/// use kernel::regmap::{define_regmap_field_descs, Fields};
///
/// define_regmap_field_descs!(DESCS, {
///     (pid, 0x3, READ, { value => raw([7:0], ro) })
/// });
///
/// struct Registrations {
///    fields: Fields<{ DESCS.len() }>,
/// }
/// ```
pub struct FieldDescs<const N: usize>([bindings::reg_field; N]);

impl<const N: usize> FieldDescs<N> {
    // macro use only
    #[doc(hidden)]
    pub const fn new(fields: [bindings::reg_field; N]) -> Self {
        Self(fields)
    }

    /// Number of fields being held by `FieldDescs<N>`
    ///
    /// This function can be used to retrieve the number of fields that were
    /// created when calling [`define_regmap_field_descs`].
    #[allow(clippy::len_without_is_empty)]
    pub const fn len(&self) -> usize {
        N
    }
}

/// Regmap fields
///
/// # Invariants
///
/// `self.fields` array is garanteed to contains valid and non-null pointers.
/// `self.fields[0]` memory is owned by `Fields`.
/// `self.fields[*]` values cannot be modified.
pub struct Fields<const N: usize> {
    fields: [NonNull<bindings::regmap_field>; N],

    // Each regmap_field hold a pointer to the `struct regmap` instance, so we need to keep a copy
    // of the wrapper around.
    _regmap: Arc<Regmap>,
}
impl<const N: usize> Fields<N> {
    /// Allocate regmap [`Fields`]
    ///
    /// This function allocate regmap fields from the `reg_fields` descriptors
    pub fn new(regmap: &Arc<Regmap>, descs: &'static FieldDescs<N>) -> Result<Self> {
        let mut fields = [NonNull::<bindings::regmap_field>::dangling(); N];
        // SAFETY:
        // * [`Regmap`] type invariants guarantee that `Regmap::as_raw` returns a valid pointer.
        // * `FieldDescs::<N>` is guaranteed to hold a valid array of size N.
        to_result(unsafe {
            bindings::regmap_field_bulk_alloc(
                regmap.as_raw(),
                fields.as_mut_ptr().cast(),
                descs.0.as_ptr().cast(),
                descs.0.len() as i32,
            )
        })?;

        Ok(Fields {
            fields,
            _regmap: regmap.clone(),
        })
    }

    /// Get field `index`
    pub fn index(&mut self, index: usize) -> *mut bindings::regmap_field {
        self.fields[index].as_ptr()
    }

    // macro use only
    #[doc(hidden)]
    pub fn read(&mut self, index: usize) -> Result<kernel::ffi::c_uint> {
        let mut val = 0;

        // Make sure we don't panic if the index is out of bound.
        if index >= N {
            return Err(EINVAL);
        }

        // SAFETY: By the type invariants, we are garanteed that all fields entries point
        // to valid and initialized values, hence it is safe to make this FFI call.
        let ret = unsafe { bindings::regmap_field_read(self.fields[index].as_ptr(), &mut val) };
        if ret < 0 {
            return Err(Error::from_errno(ret));
        }

        Ok(val)
    }
}

impl<const N: usize> Drop for Fields<N> {
    fn drop(&mut self) {
        // SAFETY: Per type invariant, `self.fields[0].as_mut` is garanteed to be valid and
        // are owned by `Fields`.
        unsafe { bindings::regmap_field_bulk_free(core::ptr::from_mut(self.fields[0].as_mut())) }
    }
}

// SAFETY: The type invariants guarantee that we own the `struct regmap_field` data and that they
// cannot be modified after allocation, and _regmap is Send, so it is safe for `Fields` to be Send.
unsafe impl<const N: usize> Send for Fields<N> {}

macro_rules! config_with {
    ($(#[$meta:meta])* $name:ident: $type:ty) => {
        config_with!($(#[$meta])* $name: $type, $name);
    };

    ($(#[$meta:meta])* $name:ident: $type:ty, $e:expr) => {
        paste! {
            $(#[$meta])*
            pub const fn [<with_$name>](mut self, $name: $type) -> Self {
                self.raw.$name = $e;
                self
            }
        }
    };
}

// macro use only
#[doc(hidden)]
pub trait ConfigOps {
    fn is_readable_reg(reg: u32) -> bool;
    fn is_writeable_reg(reg: u32) -> bool;
    fn is_volatile_reg(reg: u32) -> bool;
    fn is_precious_reg(reg: u32) -> bool;
}

/// Regmap Configuration
///
/// # Invariants
///
/// `self.raw` always contain valid data.
pub struct Config<T: ConfigOps> {
    raw: bindings::regmap_config,
    _phantom: PhantomData<T>,
}
impl<T: ConfigOps> Config<T> {
    /// Create a new regmap Config
    pub const fn new(reg_bits: i32, val_bits: i32) -> Self {
        // SAFETY: FFI type is valid to be zero-initialized.
        let mut cfg: bindings::regmap_config = unsafe { core::mem::zeroed() };

        cfg.reg_bits = reg_bits;
        cfg.val_bits = val_bits;
        cfg.writeable_reg = Some(Self::writeable_reg_callback);
        cfg.readable_reg = Some(Self::readable_reg_callback);
        cfg.volatile_reg = Some(Self::volatile_reg_callback);
        cfg.precious_reg = Some(Self::precious_reg_callback);

        Self {
            raw: cfg,
            _phantom: PhantomData,
        }
    }

    config_with!(
        /// Specifies the maximum valid register address.
        max_register: u32
    );

    config_with!(
        /// Type of caching being performed.
        cache_type: CacheType, cache_type as _
    );

    /// # Safety
    ///
    /// `_dev` must be a non-null and valid `struct device` pointer.
    unsafe extern "C" fn writeable_reg_callback(_dev: *mut bindings::device, reg: u32) -> bool {
        T::is_writeable_reg(reg)
    }

    /// # Safety
    ///
    /// `_dev` must be a non-null and valid `struct device` pointer.
    unsafe extern "C" fn readable_reg_callback(_dev: *mut bindings::device, reg: u32) -> bool {
        T::is_readable_reg(reg)
    }

    /// # Safety
    ///
    /// `_dev` must be a non-null and valid `struct device` pointer.
    unsafe extern "C" fn volatile_reg_callback(_dev: *mut bindings::device, reg: u32) -> bool {
        T::is_volatile_reg(reg)
    }

    /// # Safety
    ///
    /// `_dev` must be a non-null and valid `struct device` pointer.
    unsafe extern "C" fn precious_reg_callback(_dev: *mut bindings::device, reg: u32) -> bool {
        T::is_precious_reg(reg)
    }
}

/// Definitions describing how registers can be accessed.
pub mod access {
    /// Register can be read from.
    pub const READ: u32 = 0b000001;
    /// Register can be written to.
    pub const WRITE: u32 = 0b000010;
    /// Register should not be read outside of a call from the driver.
    pub const PRECIOUS: u32 = 0b000100;
    /// Register value can't be cached.
    pub const VOLATILE: u32 = 0b001000;

    /// Register can be read from and written to.
    pub const RW: u32 = READ | WRITE;
}

// macro use only
#[doc(hidden)]
#[macro_export]
macro_rules! regmap_check_access {
    ($type:ident, $access:expr, $reg:ident, $addr:literal) => {
        if kernel::regmap::access::$type & $access > 0 && $reg == $addr {
            return true;
        }
    };
}
// macro use only
#[doc(hidden)]
pub use regmap_check_access;

/// Common operations for all field types
pub trait FieldCommonOps {
    /// Get the Mask for the field
    fn mask() -> u32;
}

/// Read operations for fields with `bit` type
pub trait BitFieldReadOps {
    /// Returns whether the bit is set
    fn is_set<const N: usize>(fields: &mut Fields<N>) -> Result<bool>;
}

/// Write operations for fields with `bit` type
pub trait BitFieldWriteOps {
    /// Set the bit
    fn set<const N: usize>(fields: &mut Fields<N>) -> Result;

    /// Force set the bit
    fn force_set<const N: usize>(fields: &mut Fields<N>) -> Result;

    /// Clear the bit
    fn clear<const N: usize>(fields: &mut Fields<N>) -> Result;

    /// Force clear the bit
    fn force_clear<const N: usize>(fields: &mut Fields<N>) -> Result;
}

/// Read operations for fields with `enum` type
pub trait EnumFieldReadOps {
    #[doc(hidden)]
    /// Underlying enum type reprensenting the field values
    type EnumType;

    /// Read the field
    fn read<const N: usize>(fields: &mut Fields<N>) -> Result<Self::EnumType>;
}

/// Write operations for fields with `enum` type
pub trait EnumFieldWriteOps {
    #[doc(hidden)]
    /// Underlying enum type reprensenting the field values
    type EnumType;

    /// Write the field
    fn write<const N: usize>(fields: &mut Fields<N>, val: Self::EnumType) -> Result;

    /// Force write the field
    fn force_write<const N: usize>(fields: &mut Fields<N>, val: Self::EnumType) -> Result;
}

/// Read operations for fields with `raw` type
pub trait RawFieldReadOps {
    /// Read the field
    fn read<const N: usize>(fields: &mut Fields<N>) -> Result<kernel::ffi::c_uint>;

    /// Test the field bits
    fn test_bits<const N: usize>(fields: &mut Fields<N>, bits: kernel::ffi::c_uint) -> Result;
}

/// Write operations for fields with `raw` type
pub trait RawFieldWriteOps {
    /// Write the field
    fn write<const N: usize>(fields: &mut Fields<N>, val: kernel::ffi::c_uint) -> Result;

    /// Force write the field
    fn force_write<const N: usize>(fields: &mut Fields<N>, val: kernel::ffi::c_uint) -> Result;

    /// Update the field using a mask
    fn update_bits<const N: usize>(
        fields: &mut Fields<N>,
        mask: kernel::ffi::c_uint,
        val: kernel::ffi::c_uint,
    ) -> Result;

    /// Force update the field using a mask
    fn force_update_bits<const N: usize>(
        fields: &mut Fields<N>,
        mask: kernel::ffi::c_uint,
        val: kernel::ffi::c_uint,
    ) -> Result;

    /// Set field bits
    fn set_bits<const N: usize>(fields: &mut Fields<N>, bits: kernel::ffi::c_uint) -> Result;

    /// Clear the field bits
    fn clear_bits<const N: usize>(fields: &mut Fields<N>, bits: kernel::ffi::c_uint) -> Result;
}

/// Bit field
///
/// `bit` should be use when a feature is implemented through reading or writing a single bit of
/// a register.
///
/// See [`BitFieldReadOps`] and [`BitFieldWriteOps`] for operations available..
///
/// # Syntax
///
/// `bit(index, access)`
///
/// where
/// * `index`: bit index starting from 0
/// * `access`: access of the bit with the following possible values:
///     - `ro`: read-only ([`BitFieldReadOps`] gets implemented)
///     - `wo`: write-only ([`BitFieldWriteOps`] gets implemented)
///     - `rw`: read and write (both [`BitFieldReadOps`] and [`BitFieldWriteOps`] gets
///         implemented)
///
/// # Examples
///
/// ```ignore
/// regmap::define_regmap_field_descs!(FIELD_DESCS, {
///     (command, 0x14, RW, {
///         vselgt   => bit(0, rw),
///         pwmvsel1 => bit(6, rw),
///         pwmvsel0 => bit(7, rw),
///     })
/// });
///
/// command::pwmvsel0::set(&mut fields);
/// command::pwmvsel0::is_set(&mut fields);
/// command::pwmvsel0::clear(&mut fields);
/// ```
#[macro_export]
macro_rules! regmap_field_bit {
    ($field_name:ident, $access: expr, $reg:literal, $pos:literal, rw) => {
        kernel::static_assert!($access & kernel::regmap::access::RW == kernel::regmap::access::RW);

        $crate::regmap_field_bit!($field_name, $reg, $pos, reserved);
        $crate::regmap_field_bit!($field_name, _ro);
        $crate::regmap_field_bit!($field_name, _wo);
    };

    ($field_name:ident, $access: expr, $reg:literal, $pos:literal, ro) => {
        kernel::static_assert!(
            $access & kernel::regmap::access::READ == kernel::regmap::access::READ
        );

        $crate::regmap_field_bit!($field_name, $reg, $pos, reserved);
        $crate::regmap_field_bit!($field_name, _ro);
    };

    ($field_name:ident, $access: expr, $reg:literal, $pos:literal, wo) => {
        kernel::static_assert!(
            $access & kernel::regmap::access::WRITE == kernel::regmap::access::WRITE
        );

        $crate::regmap_field_bit!($field_name, $reg, $pos, reserved);
        $crate::regmap_field_bit!($field_name, _wo);
    };

    ($field_name:ident, $reg:literal, $pos:literal, reserved) => {
        kernel::macros::paste! {
            struct [<_Bit $pos >];
        }

        impl $field_name {
            pub(crate) const fn reg_field() -> bindings::reg_field {
                bindings::reg_field {
                    reg: $reg,
                    lsb: $pos,
                    msb: $pos + 1,
                    id_offset: 0,
                    id_size: 0,
                }
            }

            #[allow(dead_code)]
            pub(crate) const fn mask() -> u32 {
                kernel::genmask!($pos, $pos) as _
            }
        }
    };

    ($field_name:ident, _ro) => {
        impl super::BitFieldReadOps for $field_name {
            fn is_set<const N: usize>(fields: &mut regmap::Fields<N>) -> Result<bool> {
                let field = fields.index(Self::id() as usize);
                let mut val: kernel::ffi::c_uint = 0;
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_read(field, &mut val) })?;
                Ok(val == 1)
            }
        }
    };

    ($field_name:ident, _wo) => {
        impl super::BitFieldWriteOps for $field_name {
            fn set<const N: usize>(fields: &mut regmap::Fields<N>) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_write(field, 1) })
            }

            fn force_set<const N: usize>(fields: &mut regmap::Fields<N>) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_force_write(field, 1) })
            }

            fn clear<const N: usize>(fields: &mut regmap::Fields<N>) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_write(field, 0) })
            }

            fn force_clear<const N: usize>(fields: &mut regmap::Fields<N>) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_force_write(field, 0) })
            }
        }
    };
}

/// Enum field
///
/// `enum` should be used when a series of contineous bits represent possible values that can be
/// enumerated.
/// `enum` fields provide type-safety and preventing to write into the fields incorrect values.
///
/// See [`EnumFieldReadOps`] and [`EnumFieldWriteOps`] for operations available..
///
/// # Syntax
///
/// `enum(bits_range, access, { variant_definitions })`
///
/// where
/// * `bits_range`: bit used to store the data.
/// * `access`: access of the bits with the following possible values:
///     - `ro`: read-only ([`EnumFieldReadOps`] gets implemented)
///     - `wo`: write-only ([`EnumFieldWriteOps`] gets implemented)
///     - `rw`: read and write (both [`EnumFieldReadOps`] and [`EnumFieldWriteOps`] gets
///         implemented)
/// * `variant_definitions`: list of all the enum variants using the syntax: `VariantName = Value,`.
///
/// # Examples
///
/// ```ignore
/// regmap::define_regmap_field_descs!(FIELD_DESCS, {
///     (limconf, 0x16, RW, {
///         ipeak     => enum([7:6], rw, {
///             Peak3p5A = 0x0,
///             Peak4p0A = 0x1,
///             Peak4p5A = 0x2,
///             Peak5p0A = 0x3,
///         }),
///     })
/// });
///
/// limconf::ipeak::write(&mut fields, limconf::ipeak::Peak4p0A);
/// limconf::ipeak::read(&mut fields);
/// ```
#[macro_export]
macro_rules! regmap_field_enum {
    ($field_name:ident, $access: expr, $reg:literal, [$msb:literal:$lsb:literal], ro, {
        $($k:ident = $v:literal,)+ }) => {
        kernel::static_assert!(
            $access & kernel::regmap::access::READ == kernel::regmap::access::READ
        );

        $crate::regmap_field_enum!($field_name, $reg, [$msb:$lsb], reserved, { $($k = $v,)+ });
        $crate::regmap_field_enum!($field_name, _ro);
    };

    ($field_name:ident, $access: expr, $reg:literal, [$msb:literal:$lsb:literal], rw, {
        $($k:ident = $v:literal,)+ }) => {
        kernel::static_assert!($access & kernel::regmap::access::RW == kernel::regmap::access::RW);

        $crate::regmap_field_enum!($field_name, $reg, [$msb:$lsb], reserved, { $($k = $v,)+ });
        $crate::regmap_field_enum!($field_name, _ro);
        $crate::regmap_field_enum!($field_name, _wo);
    };

    ($field_name:ident, $access: expr, $reg:literal, [$msb:literal:$lsb:literal], wo, {
        $($k:ident = $v:literal,)+ }) => {
        kernel::static_assert!(
            $access & kernel::regmap::access::WRITE == kernel::regmap::access::WRITE
        );

        $crate::regmap_field_enum!($field_name, $reg, [$msb:$lsb], reserved, { $($k = $v,)+ });
        $crate::regmap_field_enum!($field_name, _wo);
    };

    ($field_name:ident, $reg:literal, [$msb:literal:$lsb:literal], reserved, {
        $($k:ident = $v:literal,)+ }) => {
        kernel::macros::paste! {
            #[repr(u32)]
            #[allow(non_camel_case_types)]
            pub(crate) enum [<$field_name _enum>] {
                $($k = $v,)+
            }

            impl TryFrom<kernel::ffi::c_uint> for [<$field_name _enum>] {
                type Error = kernel::error::Error;

                fn try_from(raw_value: kernel::ffi::c_uint) -> Result<Self> {
                    match raw_value {
                        $($v => Ok(Self::$k),)+
                        _ => Err(kernel::error::code::EINVAL),
                    }
                }
            }

            impl $field_name {
                pub(crate) const fn reg_field() -> bindings::reg_field {
                    bindings::reg_field {
                        reg: $reg,
                        lsb: $lsb,
                        msb: $msb,
                        id_offset: 0,
                        id_size: 0,
                    }
                }

                #[allow(dead_code)]
                pub(crate) const fn mask() -> u32 {
                    kernel::genmask!($msb, $lsb) as _
                }
            }
        }
    };

    ($field_name:ident, _ro) => {
        impl super::EnumFieldReadOps for $field_name {
            type EnumType = kernel::macros::paste! {[<$field_name _enum>]};

            fn read<const N: usize>(fields: &mut regmap::Fields<N>) -> Result<Self::EnumType> {
                Self::EnumType::try_from(fields.read(Self::id() as usize)?)
            }
        }
    };

    ($field_name:ident, _wo) => {
        impl super::EnumFieldWriteOps for $field_name {
            type EnumType = kernel::macros::paste! {[<$field_name _enum>]};

            fn write<const N: usize>(
                fields: &mut regmap::Fields<N>,
                val: Self::EnumType
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                let ret = unsafe { bindings::regmap_field_write(field, val as _) };
                kernel::error::to_result(ret)
            }

            fn force_write<const N: usize>(
                fields: &mut regmap::Fields<N>,
                val: Self::EnumType
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                let ret = unsafe { bindings::regmap_field_force_write(field, val as _) };
                kernel::error::to_result(ret)
            }
        }
    };
}

/// Raw field
///
/// `raw` should be used when bits cannot be represented by any other field types. It provides
/// raw access to the register bits.
///
/// # Syntax
///
/// `raw(bits_range, access)`
///
/// where
/// * `bits_range`: bits used to store the data.
/// * `access`: access of the bit with the following possible values:
///     - `ro`: read-only ([`RawFieldReadOps`] gets implemented)
///     - `wo`: write-only ([`RawFieldWriteOps`] gets implemented)
///     - `rw`: read and write (both [`RawFieldReadOps`] and [`RawFieldWriteOps`] gets
///         implemented)
///
/// # Examples
///
/// ```ignore
/// regmap::define_regmap_field_descs!(FIELD_DESCS, {
///     (pid, 0x3, READ, { value => raw([7:0], ro) }),
///     (progvsel1, 0x10, RW, {
///         voutvsel1 => raw([6:0], rw),
///     })
/// });
///
/// pid::value::read(&mut fields);
/// progvsel1::voutvsel1::write(&mut fields, 0x42);
/// ```
#[macro_export]
macro_rules! regmap_field_raw {
    ($field_name:ident, $access: expr, $reg:literal, [$msb:literal:$lsb:literal], rw) => {
        kernel::static_assert!($access & kernel::regmap::access::RW == kernel::regmap::access::RW);

        $crate::regmap_field_raw!($field_name, $reg, [$msb:$lsb], reserved);
        $crate::regmap_field_raw!($field_name, $reg, [$msb:$lsb], _ro);
        $crate::regmap_field_raw!($field_name, $reg, [$msb:$lsb], _wo);
    };

    ($field_name:ident, $access: expr, $reg:literal, [$msb:literal:$lsb:literal], ro) => {
        kernel::static_assert!(
            $access & kernel::regmap::access::READ == kernel::regmap::access::READ
        );

        $crate::regmap_field_raw!($field_name, $reg, [$msb:$lsb], reserved);
        $crate::regmap_field_raw!($field_name, $reg, [$msb:$lsb], _ro);
    };

    ($field_name:ident, $access: expr, $reg:literal, [$msb:literal:$lsb:literal], wo) => {
        kernel::static_assert!(
            $access & kernel::regmap::access::WRITE == kernel::regmap::access::WRITE
        );

        $crate::regmap_field_raw!($field_name, $reg, [$msb:$lsb], reserved);
        $crate::regmap_field_raw!($field_name, $reg, [$msb:$lsb], _wo);
    };

    ($field_name:ident, $reg:literal, [$msb:literal:$lsb:literal], reserved) => {
        impl $field_name {
            pub(crate) const fn reg_field() -> bindings::reg_field {
                bindings::reg_field {
                    reg: $reg,
                    lsb: $lsb,
                    msb: $msb,
                    id_offset: 0,
                    id_size: 0,
                }
            }

            #[allow(dead_code)]
            pub(crate) const fn mask() -> u32 {
                kernel::genmask!($msb, $lsb) as _
            }
        }
    };

    ($field_name:ident, $reg:literal, [$msb:literal:$lsb:literal], _ro) => {
        impl super::RawFieldReadOps for $field_name {
            fn read<const N: usize>(fields: &mut regmap::Fields<N>) -> Result<kernel::ffi::c_uint> {
                fields.read(Self::id() as usize)
            }

            fn test_bits<const N: usize>(
                fields: &mut regmap::Fields<N>,
                bits: kernel::ffi::c_uint,
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_test_bits(field, bits) })
            }
        }
    };

    ($field_name:ident, $reg:literal, [$msb:literal:$lsb:literal], _wo) => {
        impl super::RawFieldWriteOps for $field_name {
            fn write<const N: usize>(
                fields: &mut regmap::Fields<N>,
                val: kernel::ffi::c_uint,
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_write(field, val as _) })
            }

            fn force_write<const N: usize>(
                fields: &mut regmap::Fields<N>,
                val: kernel::ffi::c_uint,
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe {
                    bindings::regmap_field_force_write(field, val as _)
                })
            }

            fn update_bits<const N: usize>(
                fields: &mut regmap::Fields<N>,
                mask: kernel::ffi::c_uint,
                val: kernel::ffi::c_uint,
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe {
                    bindings::regmap_field_update_bits(field, mask, val)
                })
            }

            fn force_update_bits<const N: usize>(
                fields: &mut regmap::Fields<N>,
                mask: kernel::ffi::c_uint,
                val: kernel::ffi::c_uint,
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe {
                    bindings::regmap_field_force_update_bits(field, mask, val)
                })
            }

            fn set_bits<const N: usize>(
                fields: &mut regmap::Fields<N>,
                bits: kernel::ffi::c_uint,
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_set_bits(field, bits) })
            }

            fn clear_bits<const N: usize>(
                fields: &mut regmap::Fields<N>,
                bits: kernel::ffi::c_uint,
            ) -> Result {
                let field = fields.index(Self::id() as usize);
                // SAFETY: `Fields` guarantee that anything returned from `Fields::index` is valid
                // and non-null, hence it is safe to perform the FFI function call.
                kernel::error::to_result(unsafe { bindings::regmap_field_clear_bits(field, bits) })
            }
        }
    };
}

// macro use only
#[doc(hidden)]
#[macro_export]
macro_rules! regmap_fields {
    ($type:ident, $reg:ident, $access:expr, $name:ident, $($t:tt)*) => {
        kernel::macros::paste! {
            #[allow(non_camel_case_types)]
            pub(crate) struct $name;

            impl $name {
                #[allow(dead_code)]
                pub(crate) const fn id() -> super::Fields {
                    super::Fields::[<$reg _ $name>]
                }
            }

            $crate::[<regmap_field_ $type>]!($name, $access, $($t)*);
        }
    };
}

// macro use only
#[doc(hidden)]
#[macro_export]
macro_rules! regmap_reg_field {
    ($reg_name:ident, $field_name:ident) => {
        register::$reg_name::$field_name::reg_field()
    };
}

// macro use only
#[doc(hidden)]
#[macro_export]
macro_rules! regmap_count_fields {
    () => { 0usize };
    ($type:ident $($rhs:ident)*) => { 1 + $crate::regmap_count_fields!($($rhs)*) };
}

/// Define regmap field descriptors
///
/// # Syntax
///
/// ```ignore
/// define_regmap_field_desc!(VAR_NAME, { <register_definition>, [<register_definition>, ...] });
/// ```
///
/// where `VAR_NAME`: symbol under which the regmap [`Fields`] are available.
///
/// register_definition:
/// ```ignore
/// (name, address, access_permission, { <field_definition>, [<field_definition>, ...] })
/// ```
/// where
///
/// * name: symbol under which this field will be available
/// * address: register address
/// * access_permission: [`access`] permission of the register
///
/// field_definition:
/// ```ignore
/// field_name => <field_type>(...),
/// ```
///
/// where `field_name` is the symbol under which the field will be accessible.
///
/// The following `<field_type>`s are available:
/// * [bit](`regmap_field_bit`)
/// * [enum](`regmap_field_enum`)
/// * [raw](`regmap_field_raw`)
///
/// # Examples
///
/// ```ignore
/// regmap::define_regmap_field_descs!(FIELD_DESCS, {
///     (pid, 0x3, READ, { value => raw([7:0], ro) }),
///     (limconf, 0x16, RW, {
///         rearm     => bit(0, rw),
///         rststatus => bit(1, rw),
///         tpwth     => enum([5:4], rw, {
///             Temp83C  = 0x0,
///             Temp94C  = 0x1,
///             Temp105C  = 0x2,
///             Temp116C  = 0x3,
///         }),
///     })
/// });
/// ```
#[macro_export]
macro_rules! define_regmap_field_descs {
    ($name:ident, {
        $((
            $reg_name:ident, $reg_addr:literal, $access:expr, {
                $($field_name:ident => $type:ident($($x:tt),*)),* $(,)?
            }
        )),+
    }) => {
        mod register {
            use kernel::regmap::{
                access::*,
                BitFieldReadOps, BitFieldWriteOps,
                ConfigOps,
                EnumFieldReadOps, EnumFieldWriteOps,
                RawFieldReadOps, RawFieldWriteOps
            };

            kernel::macros::paste! {
                $(
                    pub(crate) mod $reg_name {
                        use kernel::{bindings, error::{Result}, regmap::{self, access::*}};
                        $(
                            $crate::regmap_fields!($type, $reg_name, $access, $field_name,
                                                   $reg_addr, $($x),*);
                        )*

                        #[allow(dead_code)]
                        pub(crate) const fn addr() -> u32 {
                            $reg_addr
                        }
                    }
                )+

                #[repr(u32)]
                #[allow(non_camel_case_types)]
                pub(crate) enum Fields {
                    $($(
                        [<$reg_name _ $field_name>],
                    )*)+
                }

                pub(crate) struct AccessOps;
                impl ConfigOps for AccessOps {
                    fn is_readable_reg(reg: u32) -> bool {
                        $(
                            kernel::regmap::regmap_check_access!(READ, $access, reg, $reg_addr);
                        )+

                        false
                    }

                    fn is_writeable_reg(reg: u32) -> bool {
                        $(
                            kernel::regmap::regmap_check_access!(WRITE, $access, reg, $reg_addr);
                        )+

                        false
                    }

                    fn is_volatile_reg(reg: u32) -> bool {
                        $(
                            kernel::regmap::regmap_check_access!(VOLATILE, $access, reg, $reg_addr);
                        )+

                        false
                    }

                    fn is_precious_reg(reg: u32) -> bool {
                        $(
                            kernel::regmap::regmap_check_access!(PRECIOUS, $access, reg, $reg_addr);
                        )+

                        false
                    }
                }
            }
        }

        const $name: regmap::FieldDescs<{$crate::regmap_count_fields!($($($type)*)+)}> =
            regmap::FieldDescs::new([
                $(
                    $(
                        $crate::regmap_reg_field!($reg_name, $field_name)
                    ),*
                ),+
            ]);
    };
}
pub use define_regmap_field_descs;
