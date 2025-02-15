// SPDX-License-Identifier: GPL-2.0

//! Delay routines, using a pre-computed "loops_per_jiffy" value.
//! Sleep routines using timer list timers or hrtimers.
//!
//! C header: [`include/linux/delay.h`](srctree/include/linux/delay.h).

use crate::bindings;

pub fn msleep(msecs: u32) {
    // SAFETY: The behavior of msleep it defined for the full range of `u32`.
    unsafe { bindings::msleep(msecs) }
}
