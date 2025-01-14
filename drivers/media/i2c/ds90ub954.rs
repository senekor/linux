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

struct Ds90ub954 {
    i2c_client: i2c::Client,
    pass_gpio: Option<gpio::Desc>,
    lock_gpio: Option<gpio::Desc>,
    pdb_gpio: Option<gpio::Desc>,
    // struct regmap *regmap;
    serializers: KVec<Ds90ub953>,
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
        let Some(_id_info) = id_info else {
            dev_err!(dev, "Failed to find matching dt id\n");
            return Err(ENODEV);
        };

        let Ds90ub954ParseDtReturn {
            pass_gpio,
            lock_gpio,
            pdb_gpio,
            csi_lane_count,
            csi_lane_speed,
            test_pattern,
            continuous_clock,
        } = ds90ub954_parse_dt(dev).map_err(|e| {
            dev_err!(dev, "error parsing device tree\n");
            e
        })?;

        // TODO create regmap abstractions that match C driver more closely
        // (use regmap without "fields" abstraction)
        // let regmap_config = regmap::Config::<AccessOps>::new(8, 8);
        // let regmap = regmap::Regmap::init_i2c(client, &regmap_config).map_err(|e| {
        //     dev_err!(dev, "regmap init failed ({})\n", e.to_errno());
        //     e
        // })?;

        let serializers = ds90ub953_parse_dt(dev).map_err(|e| {
            dev_err!(dev, "error parsing device tree\n");
            e
        })?;

        let driver_data = Self {
            i2c_client: client.clone(),
            pass_gpio,
            lock_gpio,
            pdb_gpio,
            serializers,
            csi_lane_count,
            csi_lane_speed,
            test_pattern,
            continuous_clock,
        };
        let driver_data = KBox::new(driver_data, GFP_KERNEL)?;
        Ok(driver_data.into())
    }
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
fn ds90ub954_parse_dt(dev: &kernel::device::Device) -> Result<Ds90ub954ParseDtReturn> {
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

struct Ds90ub953 {
    // struct i2c_client *client;
    // struct regmap *regmap;
    rx_channel: u32,
    test_pattern: bool,
    i2c_address: u32,
    csi_lane_count: u32,
    // int i2c_alias_num; // number of slave alias pairs
    // int i2c_slave[NUM_ALIAS]; // array with the i2c slave addresses
    // int i2c_alias[NUM_ALIAS]; // array with the i2c alias addresses
    continuous_clock: bool,
    i2c_pass_through_all: bool,

    gpio_config: [Ds90ub953GpioConfig; 4],

    // reference output clock control parameters
    hs_clk_div: u32,
    div_m_val: u32,
    div_n_val: u32,

    virtual_channel_map: u32,
}
struct Ds90ub953GpioConfig {
    output_enable: u32,
    control: u32,
}
fn ds90ub953_parse_dt(dev: &kernel::device::Device) -> Result<KVec<Ds90ub953>> {
    // TODO: This function body is pseudo-code.
    // There isn't yet a Rust abstraction for parsing nested device tree nodes.

    // TODO: needs Rust abstraction for iterating over devicetree nodes
    // let serializers = of_get_child_by_name(des, "serializers");
    let serializers: [&kernel::device::Device; 0] = []; // dummy to please compiler
    dev_warn!(dev, "ds90ub953_parse_dt is not yet implemented\n");

    let mut res = KVec::new();

    for serializer in serializers {
        let get_u32 = |prop, default| {
            let val = serializer
                .property_read::<u32>(prop, None)
                .unwrap_or_else(|_| {
                    dev_info!(dev, "{prop} property not found, set to default value\n");
                    default
                });
            dev_info!(dev, "{prop}: {val}\n");
            val
        };

        let rx_channel = get_u32(c_str!("rx-channel"), 0);

        let test_pattern = serializer.property_read_bool(c_str!("test-pattern"));
        if test_pattern {
            dev_info!(dev, "test-pattern enabled\n");
        } else {
            dev_info!(dev, "test-pattern disabled\n");
        }

        let csi_lane_count = get_u32(c_str!("csi-lane-count"), 4);

        let gpio_config = [
            Ds90ub953GpioConfig {
                output_enable: get_u32(c_str!("gpio0-output-enable"), 0),
                control: get_u32(c_str!("gpio0-control"), 0b1000),
            },
            Ds90ub953GpioConfig {
                output_enable: get_u32(c_str!("gpio1-output-enable"), 0),
                control: get_u32(c_str!("gpio1-control"), 0b1000),
            },
            Ds90ub953GpioConfig {
                output_enable: get_u32(c_str!("gpio2-output-enable"), 0),
                control: get_u32(c_str!("gpio2-control"), 0b1000),
            },
            Ds90ub953GpioConfig {
                output_enable: get_u32(c_str!("gpio3-output-enable"), 0),
                control: get_u32(c_str!("gpio3-control"), 0b1000),
            },
        ];

        let hs_clk_div_default = 0b010; // div by 4
        let hs_clk_div = match serializer.property_read::<u32>(c_str!("hs-clk-div"), None) {
            Ok(1) => 0b000,
            Ok(2) => 0b001,
            Ok(4) => 0b010,
            Ok(8) => 0b011,
            Ok(16) => 0b100,
            Ok(v) => {
                dev_err!(dev, "invalid value ({v}) for hs-clk-div, using default\n");
                hs_clk_div_default
            }
            Err(_) => {
                dev_info!(dev, "hs-clk-div property not found, set to default value\n");
                hs_clk_div_default
            }
        };
        dev_info!(
            dev,
            "hs-clk-div: {hs_clk_div} (div by {})\n",
            1 << hs_clk_div
        );

        let div_m_val = get_u32(c_str!("div-m-val"), 1);
        let div_n_val = get_u32(c_str!("div-n-val"), 0x28);
        let i2c_address = get_u32(c_str!("i2c-address"), 0x18);

        /*
            TODO

                err = ds90ub953_i2c_client(priv, counter, val);
        if(err) {
            dev_info(dev, "%s: - ds90ub953_i2c_client failed\n",
                 __func__);
            goto next;
        }

        err = ds90ub953_regmap_init(priv, counter);
        if(err) {
            dev_info(dev, "%s: - ds90ub953_regmap_init failed\n",
                 __func__);
            goto next;
        }

        /* get i2c-slave addresses*/
        err = of_parse_phandle_with_args(ser, "i2c-slave", "list-cells",
                         0, &i2c_addresses);
        if(err) {
            dev_info(dev, "%s: - reading i2c-slave addresses failed\n",
                 __func__);
            ds90ub953->i2c_alias_num = 0;
        } else {
            ds90ub953->i2c_alias_num = i2c_addresses.args_count;
            /* writting i2c slave addresses into array*/
            for(i = 0; (i < i2c_addresses.args_count) &&
                            (i<NUM_ALIAS) ; i++) {
                ds90ub953->i2c_slave[i] = i2c_addresses.args[i];
            }
        }

        /* get slave-aliases */
        err = of_parse_phandle_with_args(ser, "slave-alias",
                         "list-cells", 0, &i2c_addresses);
        if(err) {
            dev_info(dev, "%s: - reading i2c slave-alias addresses failed\n",
                 __func__);
            ds90ub953->i2c_alias_num = 0;
        } else {
            dev_info(dev, "%s: - num of slave alias pairs: %i\n",
                 __func__, i2c_addresses.args_count);
            /* writting i2c alias addresses into array*/
            for(i=0; (i<i2c_addresses.args_count) && (i<NUM_ALIAS);
                i++) {
                ds90ub953->i2c_alias[i] = i2c_addresses.args[i];
                dev_info(dev, "%s: - slave addr: 0x%X, alias addr: 0x%X\n",
                     __func__, ds90ub953->i2c_slave[i],
                     ds90ub953->i2c_alias[i]);
            }
        }
        */

        let continuous_clock = serializer.property_read_bool(c_str!("continuous-clock"));
        if continuous_clock {
            dev_info!(dev, "continuous clock enabled\n");
        } else {
            dev_info!(dev, "discontinuous clock used\n");
        }

        let i2c_pass_through_all = serializer.property_read_bool(c_str!("i2c-pass-through-all"));
        if i2c_pass_through_all {
            dev_info!(dev, "i2c-pass-through-all enabled\n");
        } else {
            dev_info!(dev, "i2c-pass-through-all disabled\n");
        }

        let virtual_channel_map = get_u32(c_str!("virtual-channel-map"), 0xE4);

        res.push(
            Ds90ub953 {
                gpio_config,
                rx_channel,
                test_pattern,
                csi_lane_count,
                hs_clk_div,
                i2c_address,
                continuous_clock,
                i2c_pass_through_all,
                div_m_val,
                div_n_val,
                virtual_channel_map,
            },
            GFP_KERNEL,
        )?;
    }

    dev_info!(dev, "ds90ub953_parse_dt done\n");

    Ok(res)
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
