// SPDX-License-Identifier: GPL-2.0

//! Driver for the Onsemi Buck Converter NCV6336
//!
//! Datasheet: https://www.onsemi.com/pdf/datasheet/ncv6336bm-d.pdf

use kernel::{
    c_str, i2c, of,
    prelude::*,
    regmap::{self, BitFieldReadOps, BitFieldWriteOps, RawFieldWriteOps},
    regulator::{
        driver::{Config, Desc, Device, Driver, RegmapHelpers, Status, Type},
        Mode,
    },
    sync::{new_mutex, Arc, Mutex},
};
use register::*;

kernel::module_i2c_driver! {
    type: Ncv6336,
    name: "ncv6336",
    author: "Fabien Parent <fabien.parent@linaro.org>",
    license: "GPL",
}

kernel::i2c_device_table!(
    I2C_ID_TABLE,
    MODULE_I2C_ID_TABLE,
    <Ncv6336 as i2c::Driver>::IdInfo,
    [(i2c::DeviceId::new(c_str!("ncv6336")), ()),]
);

kernel::of_device_table!(
    OF_ID_TABLE,
    MODULE_OF_ID_TABLE,
    <Ncv6336 as i2c::Driver>::IdInfo,
    [(of::DeviceId::new(c_str!("onnn,ncv6336")), ()),]
);

regmap::define_regmap_field_descs!(FIELD_DESCS, {
    (pid, 0x3, READ, { value => raw([7:0], ro) }),
    (rid, 0x4, READ, { value => raw([7:0], ro) }),
    (fid, 0x5, READ, { value => raw([7:0], ro) }),
    (progvsel1, 0x10, RW, {
        voutvsel1 => raw([6:0], rw),
        envsel1   => bit(7, rw),
    }),
    (progvsel0, 0x11, RW, {
        voutvsel0 => raw([6:0], rw),
        envsel0   => bit(7, rw),
    }),
    (pgood, 0x12, RW, { dischg => bit(4, rw) }),
    (command, 0x14, RW, {
        vselgt   => bit(0, rw),
        pwmvsel1 => bit(6, rw),
        pwmvsel0 => bit(7, rw),
    }),
    (limconf, 0x16, RW, {
        rearm     => bit(0, rw),
        rststatus => bit(1, rw),
        tpwth     => enum([5:4], rw, {
            Temp83C  = 0x0,
            Temp94C  = 0x1,
            Temp105C = 0x2,
            Temp116C = 0x3,
        }),
        ipeak     => enum([7:6], rw, {
            Peak3p5A = 0x0,
            Peak4p0A = 0x1,
            Peak4p5A = 0x2,
            Peak5p0A = 0x3,
        }),
    })
});

static NCV6336_DESC: Desc = Desc::new::<Ncv6336>(c_str!("ncv6336"), Type::Voltage)
    .with_owner(&THIS_MODULE)
    .with_of_match(c_str!("buck"))
    .with_active_discharge(
        pgood::addr(),
        pgood::dischg::mask(),
        pgood::dischg::mask(),
        0,
    )
    .with_csel(
        limconf::addr(),
        limconf::ipeak::mask(),
        &[3_500_000, 4_000_000, 4_500_000, 5_000_000],
    )
    .with_enable(
        progvsel0::addr(),
        progvsel0::envsel0::mask(),
        progvsel0::envsel0::mask(),
        0,
    )
    .with_linear_mapping(
        progvsel0::addr(),
        progvsel0::voutvsel0::mask(),
        600_000,
        6250,
        128,
        0,
    );

struct Ncv6336RegulatorData {
    fields: regmap::Fields<{ FIELD_DESCS.len() }>,
}

struct Ncv6336(#[expect(dead_code)] Device<<Self as Driver>::Data>);

impl i2c::Driver for Ncv6336 {
    type IdInfo = ();

    const I2C_ID_TABLE: Option<i2c::IdTable<Self::IdInfo>> = Some(&I2C_ID_TABLE);
    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_ID_TABLE);

    fn probe(client: &mut i2c::Client, _id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        let config = regmap::Config::new(8, 8)
            .with_access_ops::<AccessOps>()
            .with_max_register(0x16)
            .with_cache_type(regmap::CacheType::RbTree);
        let regmap = Arc::new(regmap::Regmap::init_i2c(client, &config)?, GFP_KERNEL)?;
        let fields = regmap::Fields::new(&regmap, &FIELD_DESCS)?;

        let data = Arc::pin_init(new_mutex!(Ncv6336RegulatorData { fields }), GFP_KERNEL)?;
        let config = Config::new(client.as_ref(), data.clone()).with_regmap(regmap.clone());
        let regulator = Device::register(client.as_ref(), &NCV6336_DESC, config)?;

        let drvdata = KBox::new(Self(regulator), GFP_KERNEL)?;

        Ok(drvdata.into())
    }
}

#[vtable]
impl Driver for Ncv6336 {
    type Data = Arc<Mutex<Ncv6336RegulatorData>>;

    fn list_voltage(reg: &mut Device<Self::Data>, selector: u32) -> Result<i32> {
        reg.list_voltage_linear(selector)
    }

    fn enable(reg: &mut Device<Self::Data>) -> Result {
        reg.enable_regmap()
    }

    fn disable(reg: &mut Device<Self::Data>) -> Result {
        reg.disable_regmap()
    }

    fn is_enabled(reg: &mut Device<Self::Data>) -> Result<bool> {
        reg.is_enabled_regmap()
    }

    fn set_active_discharge(reg: &mut Device<Self::Data>, enable: bool) -> Result {
        reg.set_active_discharge_regmap(enable)
    }

    fn set_current_limit(reg: &mut Device<Self::Data>, min_ua: i32, max_ua: i32) -> Result {
        reg.set_current_limit_regmap(min_ua, max_ua)
    }

    fn get_current_limit(reg: &mut Device<Self::Data>) -> Result<i32> {
        reg.get_current_limit_regmap()
    }

    fn set_voltage_sel(reg: &mut Device<Self::Data>, selector: u32) -> Result {
        reg.set_voltage_sel_regmap(selector)
    }

    fn get_voltage_sel(reg: &mut Device<Self::Data>) -> Result<i32> {
        reg.get_voltage_sel_regmap()
    }

    fn set_mode(reg: &mut Device<Self::Data>, mode: Mode) -> Result {
        let data = reg.data();
        let fields = &mut data.lock().fields;

        match mode {
            Mode::Normal => command::pwmvsel0::clear(fields),
            Mode::Fast => command::pwmvsel0::set(fields),
            _ => Err(ENOTSUPP),
        }
    }

    fn get_mode(reg: &mut Device<Self::Data>) -> Mode {
        let data = reg.data();
        let fields = &mut data.lock().fields;

        match command::pwmvsel0::is_set(fields) {
            Ok(true) => Mode::Fast,
            Ok(false) => Mode::Normal,
            Err(_) => Mode::Invalid,
        }
    }

    fn get_status(reg: &mut Device<Self::Data>) -> Result<Status> {
        if !Self::is_enabled(reg)? {
            return Ok(Status::Off);
        }

        Ok(Self::get_mode(reg).into())
    }

    fn set_suspend_voltage(reg: &mut Device<Self::Data>, uv: i32) -> Result {
        let data = reg.data();
        let fields = &mut data.lock().fields;

        let quot = (uv - 600000) / 6250;
        let rem = (uv - 600000) % 6250;
        let selector = if rem > 0 { quot + 1 } else { quot };

        progvsel1::voutvsel1::write(fields, selector as _)
    }

    fn set_suspend_enable(reg: &mut Device<Self::Data>) -> Result {
        let data = reg.data();
        let fields = &mut data.lock().fields;

        progvsel1::envsel1::set(fields)?;
        command::vselgt::clear(fields)
    }

    fn set_suspend_disable(reg: &mut Device<Self::Data>) -> Result {
        let data = reg.data();
        let fields = &mut data.lock().fields;

        progvsel1::envsel1::clear(fields)?;
        command::vselgt::set(fields)
    }

    fn set_suspend_mode(reg: &mut Device<Self::Data>, mode: Mode) -> Result {
        let data = reg.data();
        let fields = &mut data.lock().fields;

        match mode {
            Mode::Normal => command::pwmvsel1::clear(fields),
            Mode::Fast => command::pwmvsel1::set(fields),
            _ => Err(ENOTSUPP),
        }
    }
}
