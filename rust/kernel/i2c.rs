// SPDX-License-Identifier: GPL-2.0

//! Abstractions for the I2C bus.
//!
//! C header: [`include/linux/i2c.h`](srctree/include/linux/i2c.h)

use crate::{
    bindings, container_of,
    device::Device,
    device_id::{self, RawDeviceId},
    driver,
    error::{to_result, Result},
    of,
    prelude::*,
    str::CStr,
    types::{ARef, ForeignOwnable, Opaque},
    ThisModule,
};

/// Abstraction for `bindings::i2c_device_id`.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct DeviceId(bindings::i2c_device_id);

impl DeviceId {
    /// Create a new device id from an I2C name.
    pub const fn new(name: &CStr) -> Self {
        let src = name.as_bytes_with_nul();
        // TODO: Replace with `bindings::i2c_device_id::default()` once stabilized for `const`.
        // SAFETY: FFI type is valid to be zero-initialized.
        let mut i2c: bindings::i2c_device_id = unsafe { core::mem::zeroed() };

        let mut i = 0;
        while i < src.len() {
            i2c.name[i] = src[i];
            i += 1;
        }

        Self(i2c)
    }
}

// SAFETY:
// * `DeviceId` is a `#[repr(transparent)` wrapper of `i2c_device_id` and does not add
//   additional invariants, so it's safe to transmute to `RawType`.
// * `DRIVER_DATA_OFFSET` is the offset to the `data` field.
unsafe impl RawDeviceId for DeviceId {
    type RawType = bindings::i2c_device_id;

    const DRIVER_DATA_OFFSET: usize = core::mem::offset_of!(bindings::i2c_device_id, driver_data);

    fn index(&self) -> usize {
        self.0.driver_data as _
    }
}

/// I2C [`DeviceId`] table.
pub type IdTable<T> = &'static dyn device_id::IdTable<DeviceId, T>;

/// An adapter for the registration of I2C drivers.
#[doc(hidden)]
pub struct Adapter<T: Driver + 'static>(T);

impl<T: Driver + 'static> driver::RegistrationOps for Adapter<T> {
    type RegType = bindings::i2c_driver;

    fn register(
        i2cdrv: &Opaque<Self::RegType>,
        name: &'static CStr,
        module: &'static ThisModule,
    ) -> Result {
        // SAFETY: It's safe to set the fields of `struct i2c_driver` on initialization.
        unsafe {
            (*i2cdrv.get()).driver.name = name.as_char_ptr();
            (*i2cdrv.get()).probe = Some(Self::probe_callback);
            (*i2cdrv.get()).remove = Some(Self::remove_callback);
            if let Some(t) = T::I2C_ID_TABLE {
                (*i2cdrv.get()).id_table = t.as_ptr();
            }
            if let Some(t) = T::OF_ID_TABLE {
                (*i2cdrv.get()).driver.of_match_table = t.as_ptr();
            }
        }

        // SAFETY: `i2cdrv` is guaranteed to be a valid `RegType`.
        to_result(unsafe { bindings::i2c_register_driver(module.0, i2cdrv.get()) })
    }

    fn unregister(i2cdrv: &Opaque<Self::RegType>) {
        // SAFETY: `i2cdrv` is guaranteed to be a valid `RegType`.
        unsafe { bindings::i2c_del_driver(i2cdrv.get()) };
    }
}

impl<T: Driver> Adapter<T> {
    /// Get the [`Self::IdInfo`] that matched during probe.
    fn id_info(client: &mut Client) -> Option<&'static T::IdInfo> {
        let id = <Self as driver::Adapter>::id_info(client.as_ref());
        if id.is_some() {
            return id;
        }

        // SAFETY: `client` and `client.as_raw()` are guaranteed to be valid.
        let id = unsafe { bindings::i2c_client_get_device_id(client.as_raw()) };
        if !id.is_null() {
            // SAFETY: `DeviceId` is a `#[repr(transparent)` wrapper of `struct i2c_device_id` and
            // does not add additional invariants, so it's safe to transmute.
            let id = unsafe { &*id.cast::<DeviceId>() };
            return Some(T::I2C_ID_TABLE?.info(id.index()));
        }

        None
    }

    extern "C" fn probe_callback(client: *mut bindings::i2c_client) -> kernel::ffi::c_int {
        // SAFETY: The i2c bus only ever calls the probe callback with a valid `client`.
        let dev = unsafe { Device::get_device(core::ptr::addr_of_mut!((*client).dev)) };
        // SAFETY: `dev` is guaranteed to be embedded in a valid `struct i2c_client` by the
        // call above.
        let mut client = unsafe { Client::from_dev(dev) };

        let info = Self::id_info(&mut client);
        match T::probe(&mut client, info) {
            Ok(data) => {
                // Let the `struct i2c_client` own a reference of the driver's private data.
                // SAFETY: By the type invariant `client.as_raw` returns a valid pointer to a
                // `struct i2c_client`.
                unsafe { bindings::i2c_set_clientdata(client.as_raw(), data.into_foreign() as _) };
            }
            Err(err) => return Error::to_errno(err),
        }

        0
    }

    extern "C" fn remove_callback(client: *mut bindings::i2c_client) {
        // SAFETY: `client` is a valid pointer to a `struct i2c_client`.
        let ptr = unsafe { bindings::i2c_get_clientdata(client) };

        // SAFETY: `remove_callback` is only ever called after a successful call to
        // `probe_callback`, hence it's guaranteed that `ptr` points to a valid and initialized
        // `KBox<T>` pointer created through `KBox::into_foreign`.
        let _ = unsafe { KBox::<T>::from_foreign(ptr) };
    }
}

impl<T: Driver + 'static> driver::Adapter for Adapter<T> {
    type IdInfo = T::IdInfo;

    fn of_id_table() -> Option<of::IdTable<Self::IdInfo>> {
        T::OF_ID_TABLE
    }
}

/// The I2C driver trait.
///
/// Drivers must implement this trait in order to get a i2c driver registered.
///
/// # Example
///
///```
/// # use kernel::{bindings, c_str, i2c, of};
/// #
/// kernel::of_device_table!(
///     OF_ID_TABLE,
///     MODULE_OF_ID_TABLE,
///     <MyDriver as i2c::Driver>::IdInfo,
///     [(of::DeviceId::new(c_str!("onnn,ncv6336")), ()),]
/// );
///
/// kernel::i2c_device_table!(
///     I2C_ID_TABLE,
///     MODULE_I2C_ID_TABLE,
///     <MyDriver as i2c::Driver>::IdInfo,
///     [(i2c::DeviceId::new(c_str!("ncv6336")), ()),]
/// );
///
/// struct MyDriver;
///
/// impl i2c::Driver for MyDriver {
///     type IdInfo = ();
///     const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_ID_TABLE);
///     const I2C_ID_TABLE: Option<i2c::IdTable<Self::IdInfo>> = Some(&I2C_ID_TABLE);
///
///     fn probe(_client: &mut i2c::Client,
///              id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
///         Ok(KBox::new(Self, GFP_KERNEL)?.into())
///     }
/// }
///```
pub trait Driver {
    /// The type holding information about each device id supported by the driver.
    // TODO: Use associated_type_defaults once stabilized:
    // type IdInfo: 'static = ();
    type IdInfo: 'static;

    /// An optional table of I2C device ids supported by the driver.
    const I2C_ID_TABLE: Option<IdTable<Self::IdInfo>>;

    /// An optional table of OF device ids supported by the driver.
    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>>;

    /// I2C driver probe.
    ///
    /// Called when a new I2C client is added or discovered.
    fn probe(client: &mut Client, id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>>;
}

/// An I2C Client.
///
/// # Invariants
///
/// `Client` holds a valid reference of `ARef<device::Device>` whose underlying `struct device` is a
/// member of a `struct i2c_client`.
#[derive(Clone)]
pub struct Client(ARef<Device>);

impl Client {
    /// Convert a raw kernel device into a `Client`
    ///
    /// # Safety
    ///
    /// `dev` must be an `Aref<Device>` whose underlying `bindings::device` is a member of a
    /// `bindings::i2c_client`.
    unsafe fn from_dev(dev: ARef<Device>) -> Self {
        Self(dev)
    }

    /// Returns the raw `struct i2c_client`.
    pub fn as_raw(&self) -> *mut bindings::i2c_client {
        // SAFETY: By the type invariant `self.0.as_raw` is a pointer to the `struct device`
        // embedded in `struct i2c_client`.
        unsafe { container_of!(self.0.as_raw(), bindings::i2c_client, dev) }.cast_mut()
    }
}

impl AsRef<Device> for Client {
    fn as_ref(&self) -> &Device {
        &self.0
    }
}

/// Declares a kernel module that exposes a single I2C driver.
///
/// # Examples
///
/// ```ignore
/// kernel::module_i2c_driver! {
///     type: MyDriver,
///     name: "Module name",
///     author: "Author name",
///     description: "Description",
///     license: "GPL v2",
/// }
/// ```
#[macro_export]
macro_rules! module_i2c_driver {
    ($($f:tt)*) => {
        $crate::module_driver!(<T>, $crate::i2c::Adapter<T>, { $($f)* });
    };
}

/// Create an I2C `IdTable` with an "alias" for modpost.
///
/// # Examples
///
/// ```
/// use kernel::{c_str, i2c};
///
/// kernel::i2c_device_table!(
///     I2C_ID_TABLE,
///     MODULE_I2C_ID_TABLE,
///     u32,
///     [(i2c::DeviceId::new(c_str!("ncv6336")), 0x6336),]
/// );
/// ```
#[macro_export]
macro_rules! i2c_device_table {
    ($table_name:ident, $module_table_name:ident, $id_info_type: ty, $table_data: expr) => {
        const $table_name: $crate::device_id::IdArray<
            $crate::i2c::DeviceId,
            $id_info_type,
            { $table_data.len() },
        > = $crate::device_id::IdArray::new($table_data);

        $crate::module_device_table!("i2c", $module_table_name, $table_name);
    };
}
