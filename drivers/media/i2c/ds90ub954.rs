// SPDX-License-Identifier: GPL-2.0

//! Driver for the DS90UB954 FDP Link III deserializer in connection
//! with the DS90UB953 serializer from Texas Instruments
//!
//! Datasheet: https://www.ti.com/lit/ds/symlink/ds90ub954-q1.pdf

use kernel::prelude::*;

module! {
    type: Ds90ub954,
    name: "ds90ub954",
    author: "Remo Senekowitsch <remo@buenzli.dev>",
    description: "i2c ds90ub954 driver",
    license: "GPL",
}

struct Ds90ub954;

impl kernel::Module for Ds90ub954 {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Hello from Ds90ub954\n");

        Ok(Ds90ub954)
    }
}

impl Drop for Ds90ub954 {
    fn drop(&mut self) {
        pr_info!("Goodbye from Ds90ub954\n");
    }
}
