// SPDX-License-Identifier: GPL-2.0

//! Driver for the DS90UB954 FDP Link III deserializer in connection
//! with the DS90UB953 serializer from Texas Instruments
//!
//! Datasheet: https://www.ti.com/lit/ds/symlink/ds90ub954-q1.pdf

use kernel::{c_str, gpio::consumer as gpio, i2c, of, prelude::*, regmap};

kernel::module_i2c_driver! {
    type: Ds90ub954,
    name: "ds90ub954",
    author: "Remo Senekowitsch <remo@buenzli.dev>",
    license: "GPL",
}

kernel::i2c_device_table!(
    I2C_ID_TABLE,
    MODULE_I2C_ID_TABLE,
    <Ds90ub954 as i2c::Driver>::IdInfo,
    [(i2c::DeviceId::new(c_str!("ds90ub954")), ())]
);

kernel::of_device_table!(
    OF_ID_TABLE,
    MODULE_OF_ID_TABLE,
    <Ds90ub954 as i2c::Driver>::IdInfo,
    [(of::DeviceId::new(c_str!("ti,ds90ub954")), ()),]
);

// TODO ?
// regmap::define_regmap_field_descs!(FIELD_DESCS, {
//     (pid, 0x3, READ, { value => raw([7:0], ro) }),
//     (rid, 0x4, READ, { value => raw([7:0], ro) }),
//     (fid, 0x5, READ, { value => raw([7:0], ro) }),
//     // ...
// });

struct Ds90ub954 {
    i2c_client: i2c::Client,
    pass_gpio: Option<gpio::Desc>,
    lock_gpio: Option<gpio::Desc>,
    pdb_gpio: Option<gpio::Desc>,
    // struct regmap *regmap;
    // struct ds90ub953_priv *ser[NUM_SERIALIZER]; //serializers
    // int sel_rx_port; // selected rx port
    // int sel_ia_config; // selected ia configuration
    csi_lane_count: u32,
    csi_lane_speed: u32,
    test_pattern: bool,
    // int num_ser; // number of serializers connected
    continuous_clock: bool, // continuous clock (0: discontinuous, 1: continuous)
}

impl i2c::Driver for Ds90ub954 {
    type IdInfo = ();

    const I2C_ID_TABLE: Option<i2c::IdTable<Self::IdInfo>> = Some(&I2C_ID_TABLE);
    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_ID_TABLE);

    fn probe(client: &mut i2c::Client, id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        pr_info!("Hello from DS90UB954 driver\n");

        let dev = client.as_ref();

        let Ds90ub954ParseDtReturn {
            pass_gpio,
            lock_gpio,
            pdb_gpio,
            csi_lane_count,
            csi_lane_speed,
            test_pattern,
            continuous_clock,
        } = ds90ub954_parse_dt(dev, id_info).map_err(|e| {
            dev_err!(dev, "error parsing device tree\n");
            e
        })?;

        let driver_data = Self {
            i2c_client: client.clone(),
            pass_gpio,
            lock_gpio,
            pdb_gpio,
            csi_lane_count,
            csi_lane_speed,
            test_pattern,
            continuous_clock,
        };
        let driver_data = KBox::new(driver_data, GFP_KERNEL)?;
        Ok(driver_data.into())
    }
}

fn ds90ub954_parse_dt(
    dev: &kernel::device::Device,
    id_info: Option<&()>,
) -> Result<Ds90ub954ParseDtReturn> {
    if id_info.is_none() {
        dev_err!(dev, "Failed to find matching dt id\n");
        return Err(ENODEV);
    }

    let try_get_gpio = |con_id: &'static CStr, flags: gpio::Flags| -> Result<Option<gpio::Desc>> {
        match gpio::Desc::get(dev, con_id, flags) {
            Ok(gpio) => Ok(Some(gpio)),
            Err(e) if e == EPROBE_DEFER => {
                dev_err!(dev, "{con_id}-gpio read failed: (EPROBE_DEFER)\n");
                Err(e)
            }
            Err(_) => {
                dev_info!(dev, "{con_id}-gpio not found, ignoring\n");
                Ok(None)
            }
        }
    };
    let pass_gpio = try_get_gpio(c_str!("pass"), gpio::Flags::In)?;
    let lock_gpio = try_get_gpio(c_str!("lock"), gpio::Flags::In)?;
    let pdb_gpio = try_get_gpio(c_str!("pdb"), gpio::Flags::OutLow)?;

    let csi_lane_count = dev
        .property_read::<u32>(c_str!("csi-lane-count"), None)
        .unwrap_or_else(|_| {
            dev_info!(
                dev,
                "csi-lane-count property not found, set to default value\n"
            );
            4
        });
    dev_info!(dev, "csi-lane-count: {csi_lane_count}\n");

    let csi_lane_speed = dev
        .property_read::<u32>(c_str!("csi-lane-speed"), None)
        .unwrap_or_else(|_| {
            dev_info!(
                dev,
                "csi-lane-speed property not found, set to default value\n"
            );
            1600
        });
    dev_info!(dev, "csi-lane-speed: {csi_lane_speed}\n");

    let test_pattern = dev.property_read_bool(c_str!("test-pattern"));
    if test_pattern {
        dev_info!(dev, "test-pattern enabled\n");
    } else {
        dev_info!(dev, "test-pattern disabled\n");
    }

    let continuous_clock = dev.property_read_bool(c_str!("continuous-clock"));
    if continuous_clock {
        dev_info!(dev, "continuous clock enabled\n");
    } else {
        dev_info!(dev, "discontinuous clock used\n");
    }

    Ok(Ds90ub954ParseDtReturn {
        pass_gpio,
        lock_gpio,
        pdb_gpio,
        csi_lane_count,
        csi_lane_speed,
        test_pattern,
        continuous_clock,
    })
}
struct Ds90ub954ParseDtReturn {
    pass_gpio: Option<gpio::Desc>,
    lock_gpio: Option<gpio::Desc>,
    pdb_gpio: Option<gpio::Desc>,
    csi_lane_count: u32,
    csi_lane_speed: u32,
    test_pattern: bool,
    continuous_clock: bool,
}

impl Drop for Ds90ub954 {
    fn drop(&mut self) {
        // TODO
        //
        // ds90ub953_free(priv);
        // ds90ub954_pwr_disable(priv);
        if let Some(mut pdb_gpio) = self.pdb_gpio.as_mut() {
            pdb_gpio.set_value_cansleep(0);
        }

        pr_info!("Goodbye from DS90UB954 driver\n");
    }
}
