// SPDX-License-Identifier: GPL-2.0

//! Driver for the DS90UB954 FDP Link III deserializer in connection
//! with the DS90UB953 serializer from Texas Instruments
//!
//! Datasheet: https://www.ti.com/lit/ds/symlink/ds90ub954-q1.pdf

use kernel::{c_str, i2c, of, prelude::*};

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
    // struct regmap *regmap;
    // struct ds90ub953_priv *ser[NUM_SERIALIZER]; //serializers
    // int pass_gpio;
    // int lock_gpio;
    // int pdb_gpio;
    // int sel_rx_port; // selected rx port
    // int sel_ia_config; // selected ia configuration
    // int csi_lane_count;
    // int csi_lane_speed;
    // int test_pattern;
    // int num_ser; // number of serializers connected
    // int conts_clk; // continuous clock (0: discontinuous, 1: continuous)
}

impl i2c::Driver for Ds90ub954 {
    type IdInfo = ();

    const I2C_ID_TABLE: Option<i2c::IdTable<Self::IdInfo>> = Some(&I2C_ID_TABLE);
    const OF_ID_TABLE: Option<of::IdTable<Self::IdInfo>> = Some(&OF_ID_TABLE);

    fn probe(client: &mut i2c::Client, id_info: Option<&Self::IdInfo>) -> Result<Pin<KBox<Self>>> {
        pr_info!("Hello from DS90UB954 driver\n");

        let driver_data = Self {
            i2c_client: client.clone(),
        };
        let driver_data = KBox::new(driver_data, GFP_KERNEL)?;
        Ok(driver_data.into())
    }
}

impl Drop for Ds90ub954 {
    fn drop(&mut self) {
        pr_info!("Goodbye from DS90UB954 driver\n");
    }
}
