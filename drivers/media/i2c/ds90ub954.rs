// SPDX-License-Identifier: GPL-2.0

//! Driver for the DS90UB954 FDP Link III deserializer in connection
//! with the DS90UB953 serializer from Texas Instruments
//!
//! Datasheet: https://www.ti.com/lit/ds/symlink/ds90ub954-q1.pdf

use kernel::{c_str, gpio::consumer as gpio, i2c, of, prelude::*, regmap, str::BStr};

///  Deserializer registers
#[allow(unused)]
mod ti954 {
    pub(crate) const REG_I2C_DEV_ID: u32 = 0x00;
    pub(crate) const DES_ID: usize = 0;
    pub(crate) const DEVICE_ID: usize = 1;

    pub(crate) const REG_RESET: usize = 0x01;
    pub(crate) const DIGITAL_RESET0: usize = 0;
    pub(crate) const DIGITAL_RESET1: usize = 1;
    pub(crate) const RESTART_AUTOLOAD: usize = 2;

    pub(crate) const REG_GENERAL_CFG: usize = 0x2;
    pub(crate) const FORCE_REFCLK_DET: usize = 0;
    pub(crate) const RX_PARITY_CHECKER_ENABLE: usize = 1;
    pub(crate) const OUTPUT_SLEEP_STATE_SELECT: usize = 2;
    pub(crate) const OUTPUT_ENABLE: usize = 3;
    pub(crate) const OUTPUT_EN_MODE: usize = 4;
    pub(crate) const I2C_MASTER_EN: usize = 5;

    pub(crate) const REG_REVISION: u32 = 0x03;
    pub(crate) const MASK_ID: usize = 0;

    pub(crate) const REG_DEVICE_STS: u32 = 0x04;
    pub(crate) const LOCK: usize = 2;
    pub(crate) const PASS: usize = 3;
    pub(crate) const REFCLK_VALID: usize = 4;
    pub(crate) const CFG_INIT_DONE: usize = 6;
    pub(crate) const CFG_CKSUM_STS: usize = 7;

    pub(crate) const REG_PAR_ERR_THOLD_HI: usize = 0x5;
    pub(crate) const PAR_ERR_THOLD_HI: usize = 0;

    pub(crate) const REG_PAR_ERR_THOLD_LO: usize = 0x6;
    pub(crate) const PAR_ERR_THOLD_LO: usize = 0;

    pub(crate) const REG_BCC_WD_CTL: usize = 0x07;
    pub(crate) const BCC_WATCHDOG_TIMER_DISABLE: usize = 0;
    pub(crate) const BCC_WATCHDOG_TIMER: usize = 1;

    pub(crate) const REG_I2C_CTL1: usize = 0x08;
    pub(crate) const I2C_FILTER_DEPTH: usize = 0;
    pub(crate) const I2C_SDA_HOLD: usize = 4;
    pub(crate) const LOCAL_WRITE_DISABLE: usize = 7;

    pub(crate) const REG_I2C_CTL2: usize = 0x09;
    pub(crate) const I2C_BUS_TIMER_DISABLE: usize = 0;
    pub(crate) const I2C_BUS_TIMER_SPEEDUP: usize = 1;
    pub(crate) const SDA_OUTPUT_DELAY: usize = 2;
    pub(crate) const SDA_OUTPUT_SETUP: usize = 4;

    pub(crate) const REG_SCL_HIGH_TIME: usize = 0x0a;
    pub(crate) const SCL_HIGH_TIME: usize = 0;

    pub(crate) const REG_SCL_LOW_TIME: usize = 0x0b;
    pub(crate) const SCL_LOW_TIME: usize = 0;

    pub(crate) const REG_RX_PORT_CTL: u32 = 0x0c;
    pub(crate) const PORT0_EN: u32 = 0;
    pub(crate) const PORT1_ER: usize = 1;
    pub(crate) const LOCK_SEL: usize = 2;
    pub(crate) const PASS_SEL: usize = 4;

    pub(crate) const REG_IO_CTL: usize = 0x0d;
    pub(crate) const IO_SUPPLY_MODE: usize = 4;
    pub(crate) const IO_SUPPLY_MODE_OV: usize = 6;
    pub(crate) const SEL3P3V: usize = 7;

    pub(crate) const REG_GPIO_PIN_STS: usize = 0x0e;
    pub(crate) const GPIO_STS: usize = 0;
    pub(crate) const GPIO0_STS: usize = 0;
    pub(crate) const GPIO1_STS: usize = 1;
    pub(crate) const GPIO2_STS: usize = 2;
    pub(crate) const GPIO3_STS: usize = 3;
    pub(crate) const GPIO4_STS: usize = 4;
    pub(crate) const GPIO5_STS: usize = 5;
    pub(crate) const GPIO6_STS: usize = 6;

    pub(crate) const REG_GPIO_INPUT_CTL: u32 = 0x0f;
    pub(crate) const GPIO_INPUT_EN: usize = 0;
    pub(crate) const GPIO0_INPUT_EN: usize = 0;
    pub(crate) const GPIO1_INPUT_EN: usize = 1;
    pub(crate) const GPIO2_INPUT_EN: usize = 2;
    pub(crate) const GPIO3_INPUT_EN: usize = 3;
    pub(crate) const GPIO4_INPUT_EN: usize = 4;
    pub(crate) const GPIO5_INPUT_EN: usize = 5;
    pub(crate) const GPIO6_INPUT_EN: usize = 6;

    pub(crate) const REG_GPIO0_PIN_CTL: u32 = 0x10;
    pub(crate) const GPIO0_OUT_EN: usize = 0;
    pub(crate) const GPIO0_OUT_VAL: usize = 1;
    pub(crate) const GPIO0_OUT_SRC: usize = 2;
    pub(crate) const GPIO0_OUT_SEL: usize = 5;

    pub(crate) const REG_GPIO1_PIN_CTL: u32 = 0x11;
    pub(crate) const GPIO1_OUT_EN: usize = 0;
    pub(crate) const GPIO1_OUT_VAL: usize = 1;
    pub(crate) const GPIO1_OUT_SRC: usize = 2;
    pub(crate) const GPIO1_OUT_SEL: usize = 5;

    pub(crate) const REG_GPIO2_PIN_CTL: u32 = 0x12;
    pub(crate) const GPIO2_OUT_EN: usize = 0;
    pub(crate) const GPIO2_OUT_VAL: usize = 1;
    pub(crate) const GPIO2_OUT_SRC: usize = 2;
    pub(crate) const GPIO2_OUT_SEL: usize = 5;

    pub(crate) const REG_GPIO3_PIN_CTL: u32 = 0x13;
    pub(crate) const GPIO3_OUT_EN: usize = 0;
    pub(crate) const GPIO3_OUT_VAL: usize = 1;
    pub(crate) const GPIO3_OUT_SRC: usize = 2;
    pub(crate) const GPIO3_OUT_SEL: usize = 5;

    pub(crate) const REG_GPIO4_PIN_CTL: u32 = 0x14;
    pub(crate) const GPIO4_OUT_EN: usize = 0;
    pub(crate) const GPIO4_OUT_VAL: usize = 1;
    pub(crate) const GPIO4_OUT_SRC: usize = 2;
    pub(crate) const GPIO4_OUT_SEL: usize = 5;

    pub(crate) const REG_GPIO5_PIN_CTL: u32 = 0x15;
    pub(crate) const GPIO5_OUT_EN: usize = 0;
    pub(crate) const GPIO5_OUT_VAL: usize = 1;
    pub(crate) const GPIO5_OUT_SRC: usize = 2;
    pub(crate) const GPIO5_OUT_SEL: usize = 5;

    pub(crate) const REG_GPIO6_PIN_CTL: u32 = 0x16;
    pub(crate) const GPIO6_OUT_EN: usize = 0;
    pub(crate) const GPIO6_OUT_VAL: usize = 1;
    pub(crate) const GPIO6_OUT_SRC: usize = 2;
    pub(crate) const GPIO6_OUT_SEL: usize = 5;

    pub(crate) const REG_RESERVED: usize = 0x17;

    pub(crate) const REG_FS_CTL: usize = 0x18;
    pub(crate) const FS_GEN_ENABLE: usize = 0;
    pub(crate) const FS_GEN_MODE: usize = 1;
    pub(crate) const FS_INIT_STATE: usize = 2;
    pub(crate) const FS_SINGLE: usize = 3;
    pub(crate) const FS_MODE: usize = 4;

    pub(crate) const REG_FS_HIGH_TIME_1: usize = 0x19;
    pub(crate) const FRAMESYNC_HIGH_TIME_1: usize = 0;

    pub(crate) const REG_FS_HIGH_TIME_0: usize = 0x1A;
    pub(crate) const FRAMESYNC_HIGH_TIME_0: usize = 0;

    pub(crate) const REG_FS_LOW_TIME_1: usize = 0x1B;
    pub(crate) const FRAMESYNC_LOW_TIME_1: usize = 0;

    pub(crate) const REG_FS_LOW_TIME_0: usize = 0x1C;
    pub(crate) const FRAMESYNC_LOW_TIME_0: usize = 0;

    pub(crate) const REG_MAX_FRM_HI: usize = 0x1d;
    pub(crate) const MAX_FRAME_HI: usize = 0;

    pub(crate) const REG_MAX_FRM_LO: usize = 0x1e;
    pub(crate) const MAX_FRAME_LO: usize = 0;

    pub(crate) const REG_CSI_PLL_CTL: u32 = 0x1f;
    pub(crate) const CSI_TX_SPEED: usize = 0;

    pub(crate) const REG_FWD_CTL1: u32 = 0x20;
    pub(crate) const FWD_PORT0_DIS: u32 = 4;
    pub(crate) const FWD_PORT1_DIS: usize = 6;

    pub(crate) const FWD_CTL2: usize = 0x21;
    pub(crate) const CSI0_RR_RWD: usize = 0;
    pub(crate) const CSI0_SYNC_FWD: usize = 2;
    pub(crate) const FWD_SYNC_AS_AVAIL: usize = 6;
    pub(crate) const CSI_REPLICATE: usize = 7;

    pub(crate) const REG_FWD_STS: usize = 0x22;
    pub(crate) const FWD_SYNC0: usize = 0;
    pub(crate) const FWD_SYNC_FAIL0: usize = 2;

    pub(crate) const REG_INTERRUPT_CTL: usize = 0x23;
    pub(crate) const IE_RX0: usize = 0;
    pub(crate) const IE_RX1: usize = 1;
    pub(crate) const IE_CSI_TX0: usize = 4;
    pub(crate) const INT_EN: usize = 7;

    pub(crate) const REG_INTERRUPT_STS: usize = 0x24;
    pub(crate) const IS_RX0: usize = 0;
    pub(crate) const IS_RX1: usize = 1;
    pub(crate) const IS_CSI_TX0: usize = 4;
    pub(crate) const INTERRUPT_STS: usize = 7;

    pub(crate) const REG_TS_CONFIG: usize = 0x25;
    pub(crate) const TS_MODE: usize = 0;
    pub(crate) const TS_FREERUN: usize = 1;
    pub(crate) const TS_AS_AVAIL: usize = 3;
    pub(crate) const TS_RES_CTL: usize = 4;
    pub(crate) const FS_POLARITY: usize = 6;

    pub(crate) const REG_TS_CONTROL: usize = 0x26;
    pub(crate) const TS_ENABLE0: usize = 0;
    pub(crate) const TS_ENABLE1: usize = 1;
    pub(crate) const TS_FREEZE: usize = 4;

    pub(crate) const REG_TS_LINE_LO: usize = 0x28;
    pub(crate) const TS_LINE_LO: usize = 0;

    pub(crate) const REG_TS_STATUS: usize = 0x29;
    pub(crate) const TS_VALID0: usize = 0;
    pub(crate) const TS_VALID1: usize = 1;
    pub(crate) const TS_READY: usize = 42;

    pub(crate) const REG_TIMESTAMP_P0_HI: usize = 0x2a;
    pub(crate) const TIMESTAMP_P0_HI: usize = 03;

    pub(crate) const REG_TIMESTAMP_P0_LO: usize = 0x2b;
    pub(crate) const TIMESTAMP_P0_LO: usize = 04;

    pub(crate) const REG_TIMESTAMP_P1_HI: usize = 0x2c;
    pub(crate) const TIMESTAMP_P1_HI: usize = 0;

    pub(crate) const REG_TIMESTAMP_P1_LO: usize = 0x2d;
    pub(crate) const TIMESTAMP_P1_LO: usize = 0;

    pub(crate) const REG_CSI_CTL: u32 = 0x33;
    pub(crate) const CSI_ENABLE: usize = 0;
    pub(crate) const CSI_CONTS_CLOCK: usize = 1;
    pub(crate) const CSI_ULP: usize = 2;
    pub(crate) const CSI_LANE_COUNT: usize = 4;
    pub(crate) const CSI_CAL_EN: usize = 6;
    pub(crate) const CSI_4_LANE: u32 = 0;
    pub(crate) const CSI_3_LANE: u32 = 1;
    pub(crate) const CSI_2_LANE: u32 = 2;
    pub(crate) const CSI_1_LANE: u32 = 3;

    pub(crate) const REG_CSI_CTL2: usize = 0x34;
    pub(crate) const CSI_CAL_PERIODIC: usize = 0;
    pub(crate) const CSI_CAL_SINGLE: usize = 1;
    pub(crate) const CSI_CAL_INV: usize = 2;
    pub(crate) const CSI_PASS_MODE: usize = 3;

    pub(crate) const REG_CSI_STS: usize = 0x35;
    pub(crate) const TX_PORT_PASS: usize = 0;
    pub(crate) const TX_PORT_SYNC: usize = 1;

    pub(crate) const REG_CSI_TX_ICR: usize = 0x36;
    pub(crate) const IE_CSI_PASS: usize = 0;
    pub(crate) const IE_SCI_PASS_ERROR: usize = 1;
    pub(crate) const IE_CSI_SYNC: usize = 2;
    pub(crate) const IE_CSI_SYNC_ERROR: usize = 3;

    pub(crate) const REG_CSI_TX_ISR: usize = 0x37;
    pub(crate) const IS_CSI_PASS: usize = 0;
    pub(crate) const IS_CSI_PASS_ERR_OR: usize = 1;
    pub(crate) const IS_CSI_SYNC: usize = 2;
    pub(crate) const IS_CSI_SYNC_ERR_OR: usize = 3;
    pub(crate) const IS_RX_PORT_INT: usize = 4;

    pub(crate) const REG_CSI_TEST_CTL: usize = 0x38;

    pub(crate) const REG_CSI_TEST_PATT_HI: usize = 0x39;
    pub(crate) const CSI_TEST_PATT_HI: usize = 0;

    pub(crate) const REG_CSI_TEST_PATT_LO: usize = 0x3a;
    pub(crate) const CSI_TEST_PATT_LO: usize = 0;

    pub(crate) const REG_SFILTER_CFG: usize = 0x41;
    pub(crate) const SFILTER_MIN: usize = 0;
    pub(crate) const SFILTER_MAX: usize = 4;

    pub(crate) const REG_AEQ_CTL1: usize = 0x42;
    pub(crate) const AEQ_SFILTER_EN: usize = 0;
    pub(crate) const AEQ_OUTER_LOOP: usize = 1;
    pub(crate) const AEQ_2STEP_EN: usize = 2;
    pub(crate) const AEQ_ERR_CTL: usize = 4;

    pub(crate) const REG_AEQ_ERR_THOLD: usize = 0x43;
    pub(crate) const AEQ_ERR_THRESHOLD: usize = 0;

    pub(crate) const REG_FPD3_CAP: usize = 0x4a;
    pub(crate) const FPD3_ENC_CRC_CAP: usize = 4;

    pub(crate) const REG_RAQ_EMBED_DTYPE: usize = 0x4b;
    pub(crate) const EMBED_DTYPE_ID: usize = 0;
    pub(crate) const EMBED_DTYPE_EN: usize = 6;

    pub(crate) const REG_FPD3_PORT_SEL: u32 = 0x4c;
    pub(crate) const RX_WRITE_PORT_0: usize = 0;
    pub(crate) const RX_WRITE_PORT_1: usize = 1;
    pub(crate) const RX_READ_PORT: usize = 4;
    pub(crate) const PHYS_PORT_NUM: usize = 6;

    pub(crate) const REG_RX_PORT_STS1: usize = 0x4d;
    pub(crate) const LOCK_STS: usize = 0;
    pub(crate) const PORT_PASS: usize = 1;
    pub(crate) const PARITY_ERROR: usize = 2;
    pub(crate) const BCC_SEQ_ERROR: usize = 3;
    pub(crate) const LOCK_STS_CHG: usize = 4;
    pub(crate) const BCC_CRC_ERROR: usize = 5;
    pub(crate) const RX_PORT_NUM: usize = 6;

    pub(crate) const REG_RX_PORT_STS2: usize = 0x4e;
    pub(crate) const LINE_CNT_CHG: usize = 0;
    pub(crate) const CABLE_FAULT: usize = 1;
    pub(crate) const FREQ_STABLE: usize = 2;
    pub(crate) const CSI_ERROR: usize = 3;
    pub(crate) const BUFFER_ERROR: usize = 4;
    pub(crate) const FPD3_ENCODE_ERROR: usize = 5;
    pub(crate) const LINE_LEN_CHG: usize = 6;
    pub(crate) const LINE_LEN_UNSTABLE: usize = 7;

    pub(crate) const REG_RX_FREQ_HIGH: usize = 0x4f;
    pub(crate) const FREQ_CNT_HIGH: usize = 0;

    pub(crate) const REG_RX_FERQ_LOQ: usize = 0x50;
    pub(crate) const FREQ_CNT_LOW: usize = 0;

    pub(crate) const REG_SENSOR_STS_0: usize = 0x51;
    pub(crate) const VOLT0_SENSE_ALARM: usize = 0;
    pub(crate) const VOLT1_SENSE_ALARM: usize = 1;
    pub(crate) const TEMP_SENSE_ALARM: usize = 2;
    pub(crate) const LINK_DETECT_ALARM: usize = 3;
    pub(crate) const BCC_ALARM: usize = 4;
    pub(crate) const CSI_ALARM: usize = 5;

    pub(crate) const REG_SENSOR_STS_1: usize = 0x52;
    pub(crate) const VOLT0_SENSE_LEVEL: usize = 0;
    pub(crate) const VOLT1_SENSE_LEVEL: usize = 4;

    pub(crate) const REG_SENSOR_STS_2: usize = 0x53;
    pub(crate) const TEMP_SENSE_LEVEL: usize = 0;

    pub(crate) const REG_SENSOR_ST_3: usize = 0x54;
    pub(crate) const CSI_CNTRL_ERR: usize = 0;
    pub(crate) const CSI_SYNC_ERR: usize = 1;
    pub(crate) const CSI_SOT_ERR: usize = 2;
    pub(crate) const CSI_CHKSUM_ERR: usize = 3;
    pub(crate) const CSI_ECC_2BIT_ERR: usize = 4;

    pub(crate) const REG_RX_PAR_ERR_HI: usize = 0x55;
    pub(crate) const PAR_ERROR_BYTE_1: usize = 0;

    pub(crate) const REG_RX_PAR_ERR_LO: usize = 0x56;
    pub(crate) const PAR_ERROR_BYTE_0: usize = 0;

    pub(crate) const REG_BIST_ERR_COUNT: usize = 0x57;
    pub(crate) const BIST_ERROR_COUNT: usize = 0;

    pub(crate) const REG_BCC_CONFIG: u32 = 0x58;
    pub(crate) const BC_FREQ_SELECT: usize = 0;
    pub(crate) const BC_CRC_GENERAOTR_ENABLE: usize = 3;
    pub(crate) const BC_ALWAYS_ON: usize = 4;
    pub(crate) const AUTO_ACK_ALL: usize = 5;
    pub(crate) const I2C_PASS_THROUGH: usize = 6;
    pub(crate) const I2C_PASS_THROUGH_ALL: usize = 7;
    pub(crate) const BC_FREQ_2M5: usize = 0;
    pub(crate) const BC_FREQ_1M: usize = 2;
    pub(crate) const BC_FREQ_25M: usize = 5;
    pub(crate) const BC_FREQ_50M: u32 = 6;
    pub(crate) const BC_FREQ_250: usize = 7;

    pub(crate) const REG_DATAPATH_CTL1: usize = 0x59;
    pub(crate) const FC_GPIO_EN: usize = 0;
    pub(crate) const OVERRIDE_FC_CONFIG: usize = 7;

    pub(crate) const REG_DATAPATH_CTL2: usize = 0x5a;

    pub(crate) const REG_SER_ID: usize = 0x5b;
    pub(crate) const FREEZE_DEVICE_ID: usize = 0;
    pub(crate) const SER_ID: usize = 1;

    pub(crate) const REG_SER_ALIAS_ID: u32 = 0x5c;
    pub(crate) const SER_AUTO_ACK: usize = 0;
    pub(crate) const SER_ALIAS_ID: usize = 1;

    pub(crate) const REG_SLAVE_ID0: usize = 0x5d;
    pub(crate) const SLAVE_ID0: usize = 1;
    pub(crate) const REG_SLAVE_ID1: usize = 0x5e;
    pub(crate) const SLAVE_ID1: usize = 1;
    pub(crate) const REG_SLAVE_ID2: usize = 0x5f;
    pub(crate) const SLAVE_ID2: usize = 1;
    pub(crate) const REG_SLAVE_ID3: usize = 0x60;
    pub(crate) const SLAVE_ID3: usize = 1;
    pub(crate) const REG_SLAVE_ID4: usize = 0x61;
    pub(crate) const SLAVE_ID4: usize = 1;
    pub(crate) const REG_SLAVE_ID5: usize = 0x62;
    pub(crate) const SLAVE_ID5: usize = 1;
    pub(crate) const REG_SLAVE_ID6: usize = 0x63;
    pub(crate) const SLAVE_ID6: usize = 1;
    pub(crate) const REG_SLAVE_ID7: usize = 0x64;
    pub(crate) const SLAVE_ID7: usize = 1;

    pub(crate) const REG_ALIAS_ID0: usize = 0x65;
    pub(crate) const ALIAS_ID0: usize = 1;
    pub(crate) const REG_ALIAS_ID1: usize = 0x66;
    pub(crate) const ALIAS_ID1: usize = 1;
    pub(crate) const REG_ALIAS_ID2: usize = 0x67;
    pub(crate) const ALIAS_ID2: usize = 1;
    pub(crate) const REG_ALIAS_ID3: usize = 0x68;
    pub(crate) const ALIAS_ID3: usize = 1;
    pub(crate) const REG_ALIAS_ID4: usize = 0x644;
    pub(crate) const ALIAS_ID4: usize = 1;
    pub(crate) const REG_ALIAS_ID5: usize = 0x6a;
    pub(crate) const ALIAS_ID5: usize = 1;
    pub(crate) const REG_ALIAS_ID6: usize = 0x6b;
    pub(crate) const ALIAS_ID6: usize = 1;
    pub(crate) const REG_ALIAS_ID7: usize = 0x6c;
    pub(crate) const ALIAS_ID7: usize = 1;

    pub(crate) const REG_PORT_CONFIG: usize = 0x6d;
    pub(crate) const FPD3_MODE: usize = 0;
    pub(crate) const COAX_MODE: usize = 2;
    pub(crate) const CSI_FWD_LEN: usize = 3;
    pub(crate) const CSI_FWD_ECC: usize = 4;
    pub(crate) const CSI_FWD_CKSUM: usize = 5;
    pub(crate) const CSI_WAIT_FS: usize = 6;
    pub(crate) const CSI_WAIT_FS1: usize = 7;

    pub(crate) const REG_BC_GPIO_CTL0: u32 = 0x6e;
    pub(crate) const BC_GPIO0_SEL: usize = 0;
    pub(crate) const BC_GPIO1_SEL: usize = 4;

    pub(crate) const REG_BC_GPIO_CTL1: u32 = 0x6f;
    pub(crate) const BC_GPIO2_SEL: usize = 0;
    pub(crate) const BC_GPIO3_SEL: usize = 4;

    pub(crate) const REG_RAW10_ID: usize = 0x70;
    pub(crate) const RAW10_DT: usize = 0;
    pub(crate) const RAW10_VC: usize = 6;

    pub(crate) const REG_RAW12_ID: usize = 0x71;
    pub(crate) const RAW12_DT: usize = 0;
    pub(crate) const RAW12_VC: usize = 6;

    pub(crate) const REG_CSI_VC_MAP: u32 = 0x72;
    pub(crate) const CSI_VC_MAP: usize = 0;

    pub(crate) const REG_LINE_COUNT_HI: usize = 0x73;
    pub(crate) const LINE_COUNT_HI: usize = 0;

    pub(crate) const REG_LINE_COUNT_LO: usize = 0x74;
    pub(crate) const LINE_COUNT_LO: usize = 0;

    pub(crate) const REG_LINE_LEN_1: usize = 0x750;
    pub(crate) const LINE_LEN_HI: usize = 0;

    pub(crate) const REG_LINE_LEN_0: usize = 0x76;
    pub(crate) const LINE_LEN_LO: usize = 0;

    pub(crate) const REG_FREQ_DET_CTL: usize = 0x77;
    pub(crate) const FREW_LO_THR: usize = 0;
    pub(crate) const FREQ_STABLE_THR: usize = 4;
    pub(crate) const FREQ_HYST: usize = 6;

    pub(crate) const REG_MAILBOX_1: usize = 0x78;
    pub(crate) const MAILBOX_0: usize = 0;

    pub(crate) const REG_MAILBOX_2: usize = 0x79;
    pub(crate) const MAILBOX_1: usize = 0;

    pub(crate) const REG_CSI_RX_STS: usize = 0x7a;
    pub(crate) const ECC1_ERR: usize = 0;
    pub(crate) const ECC2_ERR: usize = 1;
    pub(crate) const CKSUM_ERR: usize = 2;
    pub(crate) const LENGTH_ERR: usize = 3;

    pub(crate) const REG_CSI_ERR_COUNTER: usize = 0x7b;
    pub(crate) const CSI_ERR_CNT: usize = 0;

    pub(crate) const REG_PORT_CONFIG2: usize = 0x7c;
    pub(crate) const FV_POLARITY: usize = 0;
    pub(crate) const LV_POLARITY: usize = 1;
    pub(crate) const DISCARD_ON_FRAME_SIZE: usize = 3;
    pub(crate) const DISCARD_ON_LINE_SIZE: usize = 4;
    pub(crate) const DISCARD_ON_PAR_ERR: usize = 5;
    pub(crate) const RAW10_8BIT_CTL: usize = 6;

    pub(crate) const REG_PORT_PASS_CTL: usize = 0x7d;
    pub(crate) const PASS_THRESHOLD: usize = 0;
    pub(crate) const PASS_WDOG_DIS: usize = 2;
    pub(crate) const PASS_PARITY_ERR: usize = 3;
    pub(crate) const PASS_LINE_SIZE: usize = 4;
    pub(crate) const PASS_LINE_CNT: usize = 5;
    pub(crate) const PASS_DISCARD_EN: usize = 7;

    pub(crate) const REG_SEN_INT_RISE_CTL: usize = 0x7e;
    pub(crate) const SEN_INT_RISE_MASK: usize = 0;

    pub(crate) const REG_SEN_INT_FALL_CTL: usize = 0x7f;
    pub(crate) const SEN_INT_FALL_MASK: usize = 0;

    pub(crate) const REG_REFCLK_FREQ: usize = 0xa5;
    pub(crate) const REFCLK_FREQ: usize = 0;

    pub(crate) const REG_IND_ACC_CTL: usize = 0xb0;
    pub(crate) const IA_READ: usize = 0;
    pub(crate) const IA_AUTO_INC: usize = 1;
    pub(crate) const IA_SEL: usize = 2;

    pub(crate) const REG_IND_ACC_ADDR: usize = 0xb1;
    pub(crate) const IA_ADDR: usize = 0;

    pub(crate) const REG_IND_ACC_DATA: usize = 0xb2;
    pub(crate) const IA_DATA: usize = 0;

    pub(crate) const REG_BIST_CONTROL: u32 = 0xb3;
    pub(crate) const BIST_EN: usize = 0;
    pub(crate) const BIST_CLOCK_SOURCE: usize = 1;
    pub(crate) const BIST_PIN_CONFIG: usize = 3;
    pub(crate) const BIST_OUT_MODE: usize = 6;

    pub(crate) const REG_MODE_IDX_STS: usize = 0xb8;
    pub(crate) const MODE: usize = 0;
    pub(crate) const MODE_DONE: usize = 1;
    pub(crate) const IDX: usize = 4;
    pub(crate) const IDX_DONE: usize = 7;

    pub(crate) const REG_LINK_ERROR_COUNT: usize = 0xb9;
    pub(crate) const LINK_ERR_THRESH: usize = 0;
    pub(crate) const LINK_ERR_COUNT_EN: usize = 4;
    pub(crate) const LINK_SFIL_WAIT: usize = 5;

    pub(crate) const REG_FPD3_ENC_CTL: usize = 0xba;
    pub(crate) const FPD3_ENC_CRC_DIS: usize = 7;

    pub(crate) const REG_FV_MIN_TIME: usize = 0xbc;
    pub(crate) const FRAME_VALID_MIN: usize = 0;

    pub(crate) const REG_GPIO_PD_CTL: usize = 0xbe;
    pub(crate) const GPIO0_PD_DIS: usize = 0;
    pub(crate) const GPIO1_PD_DIS: usize = 1;
    pub(crate) const GPIO2_PD_DIS: usize = 2;
    pub(crate) const GPIO3_PD_DIS: usize = 3;
    pub(crate) const GPIO4_PD_DIS: usize = 4;
    pub(crate) const GPIO5_PD_DIS: usize = 5;
    pub(crate) const GPIO6_PD_DIS: usize = 6;

    pub(crate) const REG_PORT_DEBUG: usize = 0xd0;
    pub(crate) const FORCE_1_BC_ERROR: usize = 0;
    pub(crate) const FORCE_BC_ERRORS: usize = 1;
    pub(crate) const SER_BIST_ACT: usize = 5;

    pub(crate) const REG_AEQ_CTL2: usize = 0xd2;
    pub(crate) const SET_AEQ_FLOOR: usize = 2;
    pub(crate) const AEQ_RESTART: usize = 3;
    pub(crate) const AEQ_1ST_LOCK_MODE: usize = 4;
    pub(crate) const ADAPTIVE_EQ_RELOCK_TIME: usize = 5;

    pub(crate) const REG_AEQ_STATUS: usize = 0xd3;
    pub(crate) const EQ_STATUS: usize = 0;

    pub(crate) const REG_ADAPTIVE_EQ_BYPASS: usize = 0xd4;
    pub(crate) const ADAPTIVE_EQ_BYPASS: usize = 0;
    pub(crate) const EQ_STAGE_2_SELECT_VALUE: usize = 1;
    pub(crate) const AE_LOCK_MODE: usize = 4;
    pub(crate) const EQ_STAGE_1_SELECT_VALUE: usize = 5;

    pub(crate) const REG_AEQ_MIN_MAX: usize = 0xd5;
    pub(crate) const ADAPTIVE_EQ_FLOOR_VALUE: usize = 0;
    pub(crate) const AEQ_MAX: usize = 4;

    pub(crate) const REG_PRT_ICR_HI: usize = 0xd8;
    pub(crate) const IE_BC_CRC_ERR: usize = 0;
    pub(crate) const IE_BCC_SEQ_ERR: usize = 1;
    pub(crate) const IE_FPD3_ENC_ERR: usize = 2;

    pub(crate) const REG_PORT_ICR_LO: usize = 0xd9;
    pub(crate) const IE_LOCK_STS: usize = 0;
    pub(crate) const IE_PORT_PASS: usize = 1;
    pub(crate) const IE_FPD3_PAR_ERR: usize = 2;
    pub(crate) const IE_CSI_RX_ERR: usize = 3;
    pub(crate) const IE_BUFFER_ERR: usize = 4;
    pub(crate) const IE_LINE_CNT_CHG: usize = 5;
    pub(crate) const IE_LINE_LNE_CHG: usize = 6;

    pub(crate) const REG_PORT_ISR_HI: usize = 0xda;
    pub(crate) const IS_BCC_CRC_ERR: usize = 0;
    pub(crate) const IS_BCC_CEQ_ERR: usize = 1;
    pub(crate) const IS_FPD3_ENC_ERR: usize = 2;
    pub(crate) const IS_FC_SENS_STS: usize = 3;
    pub(crate) const IE_FC_GPIO: usize = 4;

    pub(crate) const REG_PORT_ISR_LO: usize = 0xdb;
    pub(crate) const IS_LOCK_STS: usize = 0;
    pub(crate) const IS_PORT_PASS: usize = 1;
    pub(crate) const IS_PFD3_PAR_ERR: usize = 2;
    pub(crate) const IS_SCI_RX_ERR: usize = 3;
    pub(crate) const IS_BUFFER_ERR: usize = 4;
    pub(crate) const IS_LINE_CNT_CHG: usize = 5;
    pub(crate) const IS_LINE_LEN_CHG: usize = 6;

    pub(crate) const REG_FC_GPIO_STS: usize = 0xdc;
    pub(crate) const FC_GPIO0_STS: usize = 0;
    pub(crate) const FC_GPIO1_STS: usize = 1;
    pub(crate) const FC_GPIO2_STS: usize = 2;
    pub(crate) const FC_GPIO3_STS: usize = 3;
    pub(crate) const GPIO0_INT_STS: usize = 4;
    pub(crate) const GPIO1_INT_STS: usize = 5;
    pub(crate) const GPIO2_INT_STS: usize = 6;
    pub(crate) const GPIO3_INT_STS: usize = 7;

    pub(crate) const REG_FC_GPIO_ICR: usize = 0xdd;
    pub(crate) const GPIO0_RISE_IE: usize = 0;
    pub(crate) const GPIO0_FALL_IE: usize = 1;
    pub(crate) const GPIO1_RISE_IE: usize = 2;
    pub(crate) const GPIO1_FALL_IE: usize = 3;
    pub(crate) const GPIO2_RISE_IE: usize = 4;
    pub(crate) const GPIO2_FALL_IE: usize = 5;
    pub(crate) const GPIO3_RISE_IE: usize = 6;
    pub(crate) const GPIO3_FALL_IE: usize = 7;

    pub(crate) const REG_SEN_INT_RISE_STS: usize = 0xde;
    pub(crate) const SEN_INT_RISE: usize = 0;

    pub(crate) const REG_SEN_INT_FALL_STS: usize = 0xdf;
    pub(crate) const SEN_INT_FALL: usize = 0;

    pub(crate) const REG_FPD3_RX_ID0: u32 = 0xf0;
    pub(crate) const FPD3_RX_ID0: usize = 0;
    pub(crate) const REG_FPD3_RX_ID1: usize = 0xf1;
    pub(crate) const FPD3_RX_ID1: usize = 0;
    pub(crate) const REG_FPD3_RX_ID2: usize = 0xf2;
    pub(crate) const FPD3_RX_ID2: usize = 0;
    pub(crate) const REG_FPD3_RX_ID3: usize = 0xf3;
    pub(crate) const FPD3_RX_ID3: usize = 0;
    pub(crate) const REG_FPD3_RX_ID4: usize = 0xf4;
    pub(crate) const FPD3_RX_ID4: usize = 0;
    pub(crate) const REG_FPD3_RX_ID5: usize = 0xf5;
    pub(crate) const FPD3_RX_ID5: usize = 0;
    pub(crate) const RX_ID_LENGTH: usize = 6;

    pub(crate) const REG_I2C_RX0_ID: usize = 0xf8;
    pub(crate) const RX_PORT0_ID: usize = 1;

    pub(crate) const REG_I2C_RX1_ID: usize = 0xf9;
    pub(crate) const RX_PORT1_ID: usize = 1;

    // Indirect Register Map Description
    pub(crate) const REG_IA_PATTERN_GEN_PAGE_BLOCK_SELECT: usize = 0x0;

    pub(crate) const REG_IA_PGEN_CTL: u8 = 0x01;
    pub(crate) const PGEB_ENABLE: u8 = 0;

    pub(crate) const REG_IA_PGEB_CFG: u8 = 0x02;
    pub(crate) const BLOCK_SIZE: usize = 0;
    pub(crate) const NUM_CBARS: usize = 4;
    pub(crate) const PGEN_FIXED_EN: usize = 7;

    pub(crate) const REG_IA_PGEN_CSI_DI: u8 = 0x03;
    pub(crate) const PGEN_CSI_DT: usize = 0;
    pub(crate) const PGEN_CSI_VC: usize = 6;

    pub(crate) const REG_IA_PGEN_LINE_SIZE1: u8 = 0x04;
    pub(crate) const PGEN_LINE_SIZE1: usize = 0;

    pub(crate) const REG_IA_PGEN_LINE_SIZE0: u8 = 0x05;
    pub(crate) const PGEN_LINE_SIZE0: usize = 0;

    pub(crate) const REG_IA_PGEN_BAR_SIZE1: u8 = 0x06;
    pub(crate) const PGEN_BAR_SIZE1: usize = 0;

    pub(crate) const REG_IA_PGEN_BAR_SIZE0: u8 = 0x07;
    pub(crate) const PGEN_BAR_SIZE0: usize = 0;

    pub(crate) const REG_IA_PGEN_ACT_LPF1: u8 = 0x08;
    pub(crate) const PGEN_ACT_LPF1: usize = 0;

    pub(crate) const REG_IA_PGEN_ACT_LPF0: u8 = 0x09;
    pub(crate) const PGEN_ACT_LPF0: usize = 0;

    pub(crate) const REG_IA_PGEN_TOT_LPF1: u8 = 0x0a;
    pub(crate) const PGEN_TOT_LPF1: usize = 0;

    pub(crate) const REG_IA_PGEN_TOT_LPF0: u8 = 0x0b;
    pub(crate) const PGEN_TOT_LPF0: usize = 0;

    pub(crate) const REG_IA_PGEN_LINE_PD1: u8 = 0x0c;
    pub(crate) const PGEN_LINE_PD1: usize = 0;

    pub(crate) const REG_IA_PGEN_LINE_PD0: u8 = 0x0d;
    pub(crate) const PGEN_LINE_PD0: usize = 0;

    pub(crate) const REG_IA_PGEN_VBP: u8 = 0x0e;
    pub(crate) const PGEN_VBP: usize = 0;

    pub(crate) const REG_IA_PGEN_VFP: u8 = 0x0f;
    pub(crate) const PGEN_VFP: usize = 0;

    pub(crate) const REG_IA_PGEN_COLOR0: usize = 0x10;
    pub(crate) const PGEN_COLOR0: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR1: usize = 0x11;
    pub(crate) const PGEN_COLOR1: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR2: usize = 0x12;
    pub(crate) const PGEN_COLOR2: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR3: usize = 0x13;
    pub(crate) const PGEN_COLOR3: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR4: usize = 0x14;
    pub(crate) const PGEN_COLOR4: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR5: usize = 0x15;
    pub(crate) const PGEN_COLOR5: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR6: usize = 0x16;
    pub(crate) const PGEN_COLOR6: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR7: usize = 0x17;
    pub(crate) const PGEN_COLOR7: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR8: usize = 0x18;
    pub(crate) const PGEN_COLOR8: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR9: usize = 0x19;
    pub(crate) const PGEN_COLOR9: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR10: usize = 0x1a;
    pub(crate) const PGEN_COLOR10: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR11: usize = 0x1b;
    pub(crate) const PGEN_COLOR11: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR12: usize = 0x1c;
    pub(crate) const PGEN_COLOR12: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR13: usize = 0x1d;
    pub(crate) const PGEN_COLOR13: usize = 0;
    pub(crate) const REG_IA_PGEN_COLOR14: usize = 0x1e;
    pub(crate) const PGEN_COLOR14: usize = 0;

    pub(crate) const REG_IA_CSI0_TCK_PREP: usize = 0x40;
    pub(crate) const MC_TCK_PREP: usize = 0;
    pub(crate) const MC_TCK_PREP_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_TCK_ZERO: usize = 0x41;
    pub(crate) const MC_TCK_ZERO: usize = 0;
    pub(crate) const MC_TCK_ZERO_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_TCK_TRAIL: usize = 0x42;
    pub(crate) const MR_TCK_TRAIL: usize = 0;
    pub(crate) const MR_TCK_TRAIL_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_TCK_POST: usize = 0x43;
    pub(crate) const MR_TCK_POST: usize = 0;
    pub(crate) const MR_TCK_POST_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_THS_PREP: usize = 0x44;
    pub(crate) const MR_THS_PREP: usize = 0;
    pub(crate) const MR_THS_PREP_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_THS_ZERO: usize = 0x45;
    pub(crate) const MR_THS_ZERO: usize = 0;
    pub(crate) const MR_THS_ZERO_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_THS_TRAIL: usize = 0x46;
    pub(crate) const MR_THS_TRAIL: usize = 0;
    pub(crate) const MR_THS_TRIAL_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_THS_EXIT: usize = 0x47;
    pub(crate) const MR_THS_EXIT: usize = 0;
    pub(crate) const MR_THS_EXIT_OV: usize = 7;

    pub(crate) const REG_IA_CSI0_TPLX: usize = 0x48;
    pub(crate) const MR_TPLX: usize = 0;
    pub(crate) const MR_TPLX_OV: usize = 7;
}

///  Serializer registers
#[allow(unused)]
mod ti953 {
    pub(crate) const REG_I2C_DEV_ID: usize = 0x00;
    pub(crate) const SER_ID_OVERRIDE: usize = 0;
    pub(crate) const DEVICE_ID: usize = 1;

    pub(crate) const REG_RESET: usize = 0x01;
    pub(crate) const DIGITAL_RESET_0: usize = 0;
    pub(crate) const DIGITAL_RESET_1: usize = 1;
    pub(crate) const RESTART_AUTOLOAD: usize = 2;

    pub(crate) const REG_GENERAL_CFG: usize = 0x02;
    pub(crate) const I2C_STRAP_MODE: usize = 0;
    pub(crate) const CRC_TX_GEN_ENABLE: usize = 1;
    pub(crate) const CSI_LANE_SEL: usize = 4;
    pub(crate) const CONTS_CLK: usize = 6;
    pub(crate) const CSI_LANE_SEL1: usize = 0;
    pub(crate) const CSI_LANE_SEL2: usize = 1;
    pub(crate) const CSI_LANE_SEL4: usize = 3;

    pub(crate) const REG_MODE_SEL: usize = 0x03;
    pub(crate) const MODE: usize = 0;
    pub(crate) const MODE_DONE: usize = 3;
    pub(crate) const MODE_OV: usize = 4;

    pub(crate) const REG_BC_MODE_SELECT: usize = 0x04;
    pub(crate) const DVP_MODE_OVER_EN: usize = 0;
    pub(crate) const MODE_OVERWRITE_75M: usize = 1;
    pub(crate) const MODE_OVERWRITE_100M: usize = 2;

    pub(crate) const REG_PLLCLK_CTL: usize = 0x05;
    pub(crate) const OSCCLO_SEL: usize = 3;
    pub(crate) const CLKIN_DIV: usize = 4;

    pub(crate) const REG_CLKOUT_CTRL0: usize = 0x06;
    pub(crate) const DIV_M_VAL: usize = 0;
    pub(crate) const HS_CLK_DIV: usize = 5;
    pub(crate) const HS_CLK_DIV_1: usize = 0;
    pub(crate) const HS_CLK_DIV_2: usize = 1;
    pub(crate) const HS_CLK_DIV_4: usize = 2;
    pub(crate) const HS_CLK_DIV_8: usize = 3;
    pub(crate) const HS_CLK_DIV_16: usize = 4;

    pub(crate) const REG_CLKOUT_CTRL1: usize = 0x07;
    pub(crate) const DIV_N_VAL: usize = 0;

    pub(crate) const REG_BBC_WATCHDOG: usize = 0x08;
    pub(crate) const BCC_WD_TIMER_DISABLE: usize = 0;
    pub(crate) const BCC_WD_TIMER: usize = 1;

    pub(crate) const REG_I2C_CONTROL1: usize = 0x09;
    pub(crate) const I2C_FILTER_DEPTH: usize = 0;
    pub(crate) const I2C_SDA_HOLD: usize = 4;
    pub(crate) const LCL_WRITE_DISABLE: usize = 7;

    pub(crate) const REG_I2C_CONTROL2: usize = 0x0a;
    pub(crate) const I2C_BUS_TIMER_DISABLE: usize = 0;
    pub(crate) const I2C_BUS_TIMER_SPEEDUP: usize = 1;
    pub(crate) const SDA_OUTPUT_DELAY: usize = 2;
    pub(crate) const SDA_OUTPUT_SETUP: usize = 4;

    pub(crate) const REG_SCL_HIGH_TIME: usize = 0x0b;
    pub(crate) const SCL_HIGH_TIME: usize = 0;

    pub(crate) const REG_SCL_LOW_TIME: usize = 0x0c;
    pub(crate) const SCL_LOW_TIME: usize = 0;

    pub(crate) const REG_LOCAL_GPIO_DATA: usize = 0x0d;
    pub(crate) const GPIO_OUT_SRC: usize = 0;
    pub(crate) const GPIO_RMTEN: usize = 4;

    pub(crate) const REG_GPIO_CTRL: usize = 0x0e;
    pub(crate) const GPIO0_INPUT_EN: usize = 0;
    pub(crate) const GPIO1_INPUT_EN: usize = 1;
    pub(crate) const GPIO2_INPUT_EN: usize = 2;
    pub(crate) const GPIO3_INPUT_EN: usize = 3;
    pub(crate) const GPIO0_OUT_EN: usize = 4;
    pub(crate) const GPIO1_OUT_EN: usize = 5;
    pub(crate) const GPIO2_OUT_EN: usize = 6;
    pub(crate) const GPIO3_OUT_EN: usize = 7;

    pub(crate) const REG_DVP_CFG: usize = 0x10;
    pub(crate) const DVP_LV_INV: usize = 0;
    pub(crate) const DVP_FV_IN: usize = 1;
    pub(crate) const DVP_DT_YUV_EN: usize = 2;
    pub(crate) const DVP_DT_MATH_EN: usize = 3;
    pub(crate) const DVP_DT_ANY_EN: usize = 4;

    pub(crate) const REG_DVP_DT: usize = 0x11;
    pub(crate) const DVP_DT_MATCH_VAL: usize = 0;

    pub(crate) const REG_FORCE_BIST_EN: usize = 0x13;
    pub(crate) const FORCE_FC_CNT: usize = 0;
    pub(crate) const FORCE_FC_ERR: usize = 7;

    pub(crate) const REG_REMOTE_BIST_CTRL: usize = 0x14;
    pub(crate) const REMOTE_BIST_EN: usize = 0;
    pub(crate) const BIST_CLOCK: usize = 1;
    pub(crate) const LOCAL_BIST_EN: usize = 3;
    pub(crate) const FORCE_ERR_CNT: usize = 4;

    pub(crate) const REG_SENSOR_VGAIN: usize = 0x15;
    pub(crate) const VOLT_GAIN: usize = 0;

    pub(crate) const REG_SENSOR_CTRL0: usize = 0x17;
    pub(crate) const SENSE_V_GPIO: usize = 0;
    pub(crate) const SENSOR_ENABLE: usize = 2;

    pub(crate) const REG_SENSOR_CTRL1: usize = 0x18;
    pub(crate) const SENSE_GAIN_EN: usize = 7;

    pub(crate) const REG_SENSOR_V0_THRESH: usize = 0x19;
    pub(crate) const SENSE_V0_LO: usize = 0;
    pub(crate) const SENSE_V0_HI: usize = 4;

    pub(crate) const REG_SENSOR_V1_THRESH: usize = 0x1a;
    pub(crate) const SENSE_V1_LO: usize = 0;
    pub(crate) const SENSE_V1_HI: usize = 4;

    pub(crate) const REG_SENSOR_T_THRESH: usize = 0x1b;
    pub(crate) const SENSE_T_LO: usize = 0;
    pub(crate) const SENSE_T_HI: usize = 4;

    pub(crate) const REG_ALARM_CSI_EN: usize = 0x1c;
    pub(crate) const CSI_LENGTH_ERR_EN: usize = 0;
    pub(crate) const CSI_CHKSUM_ERR_EN: usize = 1;
    pub(crate) const CSI_ECC_2_EN: usize = 2;
    pub(crate) const DPHY_CTRL_ERR_EN: usize = 3;
    pub(crate) const CSI_NO_FV_EN: usize = 5;

    pub(crate) const REG_SENSE_EN: usize = 0x1d;
    pub(crate) const V0_UNDER: usize = 0;
    pub(crate) const V0_OVER: usize = 1;
    pub(crate) const V1_UNSER: usize = 2;
    pub(crate) const V1_OVER: usize = 3;
    pub(crate) const T_UNDER: usize = 4;
    pub(crate) const T_OVER: usize = 5;

    pub(crate) const REG_ALARM_BC_EN: usize = 0x1e;
    pub(crate) const LINK_DETECT_EN: usize = 0;
    pub(crate) const CRC_ER_EN: usize = 1;

    pub(crate) const REG_CSI_POL_SEL: usize = 0x20;
    pub(crate) const POLARITY_D0: usize = 0;
    pub(crate) const POLARITY_D1: usize = 1;
    pub(crate) const POLARITY_D2: usize = 2;
    pub(crate) const POLARITY_D3: usize = 3;
    pub(crate) const POLARITY_CK0: usize = 4;

    pub(crate) const REG_CSI_LP_POLARITY: usize = 0x21;
    pub(crate) const POL_LP_DATA: usize = 0;
    pub(crate) const POL_LP_CLK0: usize = 4;

    pub(crate) const REG_CSI_EN_RXTERM: usize = 0x24;
    pub(crate) const EN_RXTERM_D0: usize = 0;
    pub(crate) const EN_RXTERM_D1: usize = 1;
    pub(crate) const EN_RXTERM_D2: usize = 2;
    pub(crate) const EN_RXTERM_D3: usize = 3;

    pub(crate) const REG_CSI_PKT_HDR_TINT_CTRL: usize = 0x31;
    pub(crate) const TINIT_TIME: usize = 0;
    pub(crate) const PKT_HDR_VCI_ENABLE: usize = 4;
    pub(crate) const PKT_HDR_CORRECTED: usize = 5;
    pub(crate) const PKT_HDR_SEL_VC: usize = 6;

    pub(crate) const REG_BCC_CONFIG: usize = 0x32;
    pub(crate) const RX_PARITY_CHECKER_ENABLE: usize = 3;
    pub(crate) const AUTO_ACK_ALL: usize = 5;
    pub(crate) const I2C_PASS_THROUGH: usize = 6;
    pub(crate) const I2C_PASS_THROUGH_ALL: usize = 7;

    pub(crate) const REG_DATAPATH_CTL1: usize = 0x33;
    pub(crate) const FC_GPIO_EN: usize = 0;
    pub(crate) const DCA_CRC_EN: usize = 2;

    pub(crate) const REG_DES_PAR_CAP1: usize = 0x35;
    pub(crate) const PORT_NUM: usize = 0;
    pub(crate) const MPORT: usize = 4;
    pub(crate) const BIST_EN: usize = 5;
    pub(crate) const FREEZE_DES_CAP: usize = 7;

    pub(crate) const REG_DES_ID: usize = 0x37;
    pub(crate) const FREEZE_DEVICE_ID: usize = 0;
    pub(crate) const DES_ID: usize = 1;

    pub(crate) const REG_SLAVE_ID_0: usize = 0x39;
    pub(crate) const SLAVE_ID_0: usize = 1;
    pub(crate) const REG_SLAVE_ID_1: usize = 0x3a;
    pub(crate) const SLAVE_ID_1: usize = 1;
    pub(crate) const REG_SLAVE_ID_2: usize = 0x3b;
    pub(crate) const SLAVE_ID_2: usize = 1;
    pub(crate) const REG_SLAVE_ID_3: usize = 0x3c;
    pub(crate) const SLAVE_ID_3: usize = 1;
    pub(crate) const REG_SLAVE_ID_4: usize = 0x3d;
    pub(crate) const SLAVE_ID_4: usize = 1;
    pub(crate) const REG_SLAVE_ID_5: usize = 0x3e;
    pub(crate) const SLAVE_ID_5: usize = 1;
    pub(crate) const REG_SLAVE_ID_6: usize = 0x3f;
    pub(crate) const SLAVE_ID_6: usize = 1;
    pub(crate) const REG_SLAVE_ID_7: usize = 0x40;
    pub(crate) const SLAVE_ID_7: usize = 1;

    pub(crate) const REG_SLAVE_ID_ALIAS_0: usize = 0x41;
    pub(crate) const SLAVE_ID_ALIAS_0: usize = 1;
    pub(crate) const REG_SLAVE_ID_ALIAS_1: usize = 0x42;
    pub(crate) const SLAVE_ID_ALIAS_1: usize = 1;
    pub(crate) const REG_SLAVE_ID_ALIAS_2: usize = 0x43;
    pub(crate) const SLAVE_ID_ALIAS_2: usize = 1;
    pub(crate) const REG_SLAVE_ID_ALIAS_3: usize = 0x44;
    pub(crate) const SLAVE_ID_ALIAS_3: usize = 1;
    pub(crate) const REG_SLAVE_ID_ALIAS_4: usize = 0x45;
    pub(crate) const SLAVE_ID_ALIAS_4: usize = 1;
    pub(crate) const REG_SLAVE_ID_ALIAS_5: usize = 0x46;
    pub(crate) const SLAVE_ID_ALIAS_5: usize = 1;
    pub(crate) const REG_SLAVE_ID_ALIAS_6: usize = 0x47;
    pub(crate) const SLAVE_ID_ALIAS_6: usize = 1;
    pub(crate) const REG_SLAVE_ID_ALIAS_7: usize = 0x48;
    pub(crate) const SLAVE_ID_ALIAS_7: usize = 1;

    pub(crate) const REG_CB_CTRL: usize = 0x49;
    pub(crate) const LINK_DET_TIMER: usize = 0;
    pub(crate) const CRC_ERR_CLR: usize = 3;
    pub(crate) const BIST_CRC_ERR_CLR: usize = 5;

    pub(crate) const REG_REV_MASK_ID: usize = 0x50;
    pub(crate) const MASK_ID: usize = 0;
    pub(crate) const REVISION_ID: usize = 4;

    pub(crate) const REG_DEVICE_STS: usize = 0x51;
    pub(crate) const CFG_INIT_DONE: usize = 6;
    pub(crate) const CFG_CKSUM_STS: usize = 7;

    pub(crate) const REG_GENERAL_STATUS: usize = 0x52;
    pub(crate) const LINK_DET: usize = 0;
    pub(crate) const CRC_ERR: usize = 1;

    pub(crate) const REG_GPIO_PIN_STS: usize = 0x53;
    pub(crate) const GPIO_STS: usize = 0;

    pub(crate) const REG_BIST_ERR_CNT: usize = 0x54;
    pub(crate) const BIST_BC_ERRCNT: usize = 0;

    pub(crate) const REG_CRC_ERR_CNT1: usize = 0x55;
    pub(crate) const CRC_ERR_CNT1: usize = 0;

    pub(crate) const REG_CRC_ERR_CNT2: usize = 0x56;
    pub(crate) const CRC_ERR_CNT2: usize = 0;

    pub(crate) const REG_SENSOR_STATUS: usize = 0x57;
    pub(crate) const V0_SENSOR_LOW: usize = 0;
    pub(crate) const V0_SENOSR_HI: usize = 1;
    pub(crate) const V1_SENSOR_LOW: usize = 2;
    pub(crate) const V1_SENSOR_HI: usize = 3;
    pub(crate) const T_SENSOR_LOW: usize = 4;
    pub(crate) const T_SENSOR_HI: usize = 5;

    pub(crate) const REG_SENSOR_V0: usize = 0x58;
    pub(crate) const VOLTAGE_SENSOR_V0_MIN: usize = 0;
    pub(crate) const VOLTAGE_SENSOR_V0_MAX: usize = 4;

    pub(crate) const REG_SENSOR_V1: usize = 0x59;
    pub(crate) const VOLTAGE_SENOSR_V1_MIN: usize = 0;
    pub(crate) const VOLTAGE_SENSOR_V1_MAX: usize = 4;

    pub(crate) const REG_SENSOR_T: usize = 0x5a;
    pub(crate) const TEMP_MIN: usize = 0;
    pub(crate) const TMEP_MAX: usize = 4;

    pub(crate) const REG_CSI_ERR_CNT: usize = 0x5c;
    pub(crate) const CSI_ERR_CNT: usize = 0;

    pub(crate) const REG_CSI_ERR_STATUS: usize = 0x5d;
    pub(crate) const ECC_1BIT_ERR: usize = 0;
    pub(crate) const ECC_2BIT_ERR: usize = 1;
    pub(crate) const CHKSUM_ERR: usize = 2;
    pub(crate) const LINE_LEN_MISMATCH: usize = 3;

    pub(crate) const REG_CSI_ERR_DLANE01: usize = 0x5e;
    pub(crate) const CNTRL_ERR_HSRQST_0: usize = 1;
    pub(crate) const SOT_SYNC_ERROR_0: usize = 2;
    pub(crate) const SOT_ERROR_0: usize = 3;
    pub(crate) const CNTRL_ERR_HSRQST_1: usize = 5;
    pub(crate) const SOT_SYNC_ERROR_1: usize = 6;
    pub(crate) const SOT_ERROR_1: usize = 7;

    pub(crate) const REG_CSI_ERR_DLANE23: usize = 0x5f;
    pub(crate) const CNTRL_ERR_HSRQST_2: usize = 1;
    pub(crate) const SOT_SYNC_ERROR_2: usize = 2;
    pub(crate) const SOT_ERROR_2: usize = 3;
    pub(crate) const CNTRL_ERR_HSRQST_3: usize = 5;
    pub(crate) const SOT_SYNC_ERROR_3: usize = 6;
    pub(crate) const SOT_ERROR_3: usize = 7;

    pub(crate) const REG_CSI_ERR_CLK_LANE: usize = 0x60;
    pub(crate) const CNTRL_ERR_HSRQST_CK0: usize = 1;

    pub(crate) const REG_CSI_PKT_HDR_VC_ID: usize = 0x61;
    pub(crate) const LONG_PKT_DATA_ID: usize = 0;
    pub(crate) const LONG_PKT_VCHNL_ID: usize = 6;

    pub(crate) const REG_PKT_HDR_WC_LSB: usize = 0x62;
    pub(crate) const LONG_PKT_WRD_CNT_LSB: usize = 0;

    pub(crate) const REG_PKT_HDR_WC_MSB: usize = 0x63;
    pub(crate) const LONG_PKT_WRD_CNT_MSB: usize = 0;

    pub(crate) const REG_CSI_ECC: usize = 0x64;
    pub(crate) const CSI2_ECC: usize = 0;
    pub(crate) const LINE_LENGTH_CHANGE: usize = 7;

    pub(crate) const REG_IND_ACC_CTL: usize = 0xb0;
    pub(crate) const IA_READ: usize = 0;
    pub(crate) const IA_AUTO_INC: usize = 1;
    pub(crate) const IA_SEL: usize = 2;

    pub(crate) const REG_IND_ACC_ADDR: usize = 0xb1;
    pub(crate) const IND_ACC_ADDR: usize = 0;

    pub(crate) const REG_IND_ACC_DATA: usize = 0xb2;
    pub(crate) const IND_ACC_DATA: usize = 0;

    pub(crate) const REG_FPD3_RX_ID0: usize = 0xf0;
    pub(crate) const FPD3_RX_ID0: usize = 0;
    pub(crate) const REG_FPD3_RX_ID1: usize = 0xf1;
    pub(crate) const FPD3_RX_ID1: usize = 0;
    pub(crate) const REG_FPD3_RX_ID2: usize = 0xf2;
    pub(crate) const FPD3_RX_ID2: usize = 0;
    pub(crate) const REG_FPD3_RX_ID3: usize = 0xf3;
    pub(crate) const FPD3_RX_ID3: usize = 0;
    pub(crate) const REG_FPD3_RX_ID4: usize = 0xf4;
    pub(crate) const FPD3_RX_ID4: usize = 0;
    pub(crate) const REG_FPD3_RX_ID5: usize = 0xf5;
    pub(crate) const FPD3_RX_ID5: usize = 0;
    pub(crate) const RX_ID_LENGTH: usize = 6;
}

const NUM_SERIALIZER: usize = 2;

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

const REGMAP_CONFIG: regmap::Config = regmap::Config::new(8, 8);

#[rustfmt::skip]
static DS90UB95X_TP_REG_VAL: [u8; 62] = [
	// Indirect Pattern Gen Registers
	0xB0, 0x00,
	0xB1, ti954::REG_IA_PGEN_CTL,
	0xB2, (1<<ti954::PGEB_ENABLE),
	0xB1, ti954::REG_IA_PGEB_CFG,
	0xB2, 0x35,
	0xB1, ti954::REG_IA_PGEN_CSI_DI,
	0xB2, 0x2B,
	0xB1, ti954::REG_IA_PGEN_LINE_SIZE1,
	0xB2, 0x14,
	0xB1, ti954::REG_IA_PGEN_LINE_SIZE0,
	0xB2, 0x00,
	0xB1, ti954::REG_IA_PGEN_BAR_SIZE1,
	0xB2, 0x02,
	0xB1, ti954::REG_IA_PGEN_BAR_SIZE0,
	0xB2, 0x80,
	0xB1, ti954::REG_IA_PGEN_ACT_LPF1,
	0xB2, 0x08,
	0xB1, ti954::REG_IA_PGEN_ACT_LPF0,
	0xB2, 0x70,
	0xB1, ti954::REG_IA_PGEN_TOT_LPF1,
	0xB2, 0x08,
	0xB1, ti954::REG_IA_PGEN_TOT_LPF0,
	0xB2, 0x70,
	0xB1, ti954::REG_IA_PGEN_LINE_PD1,
	0xB2, 0x0B,
	0xB1, ti954::REG_IA_PGEN_LINE_PD0,
	0xB2, 0x93,
	0xB1, ti954::REG_IA_PGEN_VBP,
	0xB2, 0x21,
	0xB1, ti954::REG_IA_PGEN_VFP,
	0xB2, 0x0A,
];

struct Ds90ub954 {
    i2c_client: i2c::Client,
    pass_gpio: Option<gpio::Desc>,
    lock_gpio: Option<gpio::Desc>,
    pdb_gpio: Option<gpio::Desc>,
    regmap: regmap::Regmap,
    serializers: [Option<Ds90ub953>; NUM_SERIALIZER],
    selected_rx_port: Option<RxPort>,
    selected_ia_config: Option<u32>,
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
        pr_info!("probing ds90ub954\n");

        let dev = client.as_ref();
        let Some(_id_info) = id_info else {
            dev_err!(dev, "Failed to find matching dt id\n");
            return Err(ENODEV);
        };

        let selected_rx_port = None;
        let selected_ia_config = None;

        let Ds90ub954ParseDtReturn {
            pass_gpio,
            lock_gpio,
            pdb_gpio,
            csi_lane_count,
            csi_lane_speed,
            test_pattern,
            continuous_clock,
        } = ds90ub954_parse_dt(dev).map_err(|err| {
            dev_err!(dev, "error parsing device tree\n");
            err
        })?;

        let regmap = regmap::Regmap::init_i2c(client, &REGMAP_CONFIG).map_err(|err| {
            dev_err!(dev, "regmap init failed ({})\n", err.to_errno());
            err
        })?;

        let serializers = ds90ub953_parse_dt(dev).map_err(|err| {
            dev_err!(dev, "error parsing device tree\n");
            err
        })?;

        let driver_data = Self {
            i2c_client: client.clone(),
            pass_gpio,
            lock_gpio,
            pdb_gpio,
            regmap,
            serializers,
            selected_rx_port,
            selected_ia_config,
            csi_lane_count,
            csi_lane_speed,
            test_pattern,
            continuous_clock,
        };
        let mut driver_data = KBox::new(driver_data, GFP_KERNEL)?;

        driver_data.pwr_enable();

        kernel::delay::msleep(6); // wait for sensor to start

        driver_data.init()?;

        kernel::delay::msleep(500);

        pr_info!("done probing ds90ub954\n");
        Ok(driver_data.into())
    }
}

impl Ds90ub954 {
    fn pwr_enable(&mut self) {
        if let Some(pdb_gpio) = &mut self.pdb_gpio {
            pdb_gpio.set_value_cansleep(1);
        }
    }

    fn pwr_disable(&mut self) {
        if let Some(pdb_gpio) = &mut self.pdb_gpio {
            pdb_gpio.set_value_cansleep(0);
        }
    }

    fn init(&mut self) -> Result<()> {
        let i2c_client = self.i2c_client.clone();
        let dev = i2c_client.as_ref();
        dev_info!(dev, "starting init ds90ub954\n");

        let dev_id = self.read(ti954::REG_I2C_DEV_ID)?;
        let rev = self.read(ti954::REG_REVISION)?;

        let mut id_code = [0; ti954::RX_ID_LENGTH];
        for (i, byte) in id_code.iter_mut().enumerate() {
            *byte = self.read(ti954::REG_FPD3_RX_ID0 + i as u32)? as u8;
        }
        let id_code = BStr::from_bytes(&id_code);

        dev_info!(
            dev,
            "device ID: 0x{dev_id:x}, code: {id_code}, revision: 0x{rev:x}\n"
        );

        // disable builtin self test
        self.write(ti954::REG_BIST_CONTROL, 0)?;

        // set CSI speed (REFCLK 25 MHz)
        //  00 : 1.6 Gbps serial rate
        //  01 : Reserved
        //  10 : 800 Mbps serial rate
        //  11 : 400 Mbps serial rate
        let value = match self.csi_lane_speed {
            400 => 0x3,

            800 => 0x2,
            _ => 0x0,
        };
        self.write(ti954::REG_CSI_PLL_CTL, value << ti954::CSI_TX_SPEED)?;

        // TODO add debug stuff? or just omit it?

        // set number of csi lanes
        let value = match self.csi_lane_count {
            1 => ti954::CSI_1_LANE,
            2 => ti954::CSI_2_LANE,
            3 => ti954::CSI_3_LANE,
            _ => ti954::CSI_4_LANE,
        };
        self.write(
            ti954::REG_CSI_CTL,
            (1 << ti954::CSI_ENABLE)
                | (if self.continuous_clock { 1 } else { 0 } << ti954::CSI_CONTS_CLOCK)
                | (value << ti954::CSI_LANE_COUNT)
                | (1 << ti954::CSI_CAL_EN),
        )?;

        kernel::delay::msleep(500);

        // check if test pattern should be turned on
        if self.test_pattern {
            dev_info!(dev, "deserializer init testpattern\n");
            let _ = self.init_testpattern().map_err(|_| {
                dev_info!(dev, "deserializer init testpattern failed\n");
            });
        }

        // Setting PASS and LOCK to "all enabled receiver ports
        let value = 0b00111100;
        self.write(ti954::REG_RX_PORT_CTL, value)?;

        // for loop goes through each serializer
        for i in 0..self.serializers.len() {
            let Some(ds90ub953) = self.serializers[i] else {
                continue;
            };
            let rx_port = ds90ub953.rx_channel;
            dev_info!(dev, "start init of serializer rx_port: {rx_port}\n");

            // Use closure for scoped early return and easy error-path cleanup.
            let mut init_serializer = || -> Result<()> {
                // Get TI954_REG_RX_PORT_CTL and enable receiver rx_port
                let mut value = self.read(ti954::REG_RX_PORT_CTL)?;

                value |= 1 << (ti954::PORT0_EN + rx_port.to_u32());
                self.write(ti954::REG_RX_PORT_CTL, value)?;

                // wait for receiver to calibrate link
                kernel::delay::msleep(400);

                // enable csi forwarding
                let mut value = self.read(ti954::REG_FWD_CTL1)?;

                value &= 0xEF << rx_port.to_u32();
                self.write(ti954::REG_FWD_CTL1, value)?;

                kernel::delay::msleep(500);

                // config back channel RX port [specific register]
                self.write_rx_port(
                    rx_port,
                    ti954::REG_BCC_CONFIG,
                    (ti954::BC_FREQ_50M << ti954::BC_FREQ_SELECT)
                        | (1 << ti954::BC_CRC_GENERAOTR_ENABLE)
                        | (1 << ti954::BC_ALWAYS_ON)
                        | (if ds90ub953.i2c_pass_through_all { 1 } else { 0 }
                            << ti954::I2C_PASS_THROUGH_ALL)
                        | (1 << ti954::I2C_PASS_THROUGH),
                )?;

                // wait for back channel
                let mut backchannel_setup_failed = true;
                for i in 0..50 {
                    kernel::delay::msleep(10);
                    let value = self.read(ti954::REG_DEVICE_STS)?;
                    dev_info!(dev, "DEVICE STS: 0x{value:02x}, id={i} x 10ms\n",);
                    if (value & 0xff) == 0xdf {
                        backchannel_setup_failed = false;
                        dev_info!(dev, "backchannel is ready\n",);
                        break;
                    }
                }
                if backchannel_setup_failed {
                    dev_err!(dev, "Backchannel setup failed!\n");
                    return Err(EIO);
                }

                // setup i2c forwarding
                self.write_rx_port(
                    rx_port,
                    ti954::REG_SER_ALIAS_ID,
                    ds90ub953.i2c_address << ti954::SER_ALIAS_ID,
                )?;

                // Serializer GPIO control
                match self.write_rx_port(
                    rx_port,
                    ti954::REG_BC_GPIO_CTL0,
                    (ds90ub953.gpio[0].control << ti954::BC_GPIO0_SEL)
                        | (ds90ub953.gpio[1].control << ti954::BC_GPIO1_SEL),
                ) {
                    Err(_) => dev_info!(dev, "could not set ti954::REG_BC_GPIO_CTL0\n",),
                    _ => dev_info!(dev, "Successfully set ti954::REG_BC_GPIO_CTL0\n",),
                }

                match self.write_rx_port(
                    rx_port,
                    ti954::REG_BC_GPIO_CTL1,
                    (ds90ub953.gpio[2].control << ti954::BC_GPIO2_SEL)
                        | (ds90ub953.gpio[3].control << ti954::BC_GPIO3_SEL),
                ) {
                    Err(_) => dev_info!(dev, "could not set ti954::REG_BC_GPIO_CTL1\n",),
                    _ => dev_info!(dev, "Successfully set ti954::REG_BC_GPIO_CTL1\n",),
                }

                // TODO: set i2c slave ids and aliases
                // for(i=0; (i < serializer.i2c_alias_num) && (i < NUM_ALIAS); i++) {
                // 	val = serializer.i2c_slave[i];
                // 	if(val == 0) {
                // 		continue;
                // 	}
                // 	err = ds90ub954_write_rx_port(priv, rx_port,
                // 				      ti954::REG_SLAVE_ID0+i,
                // 				      (val<<ti954::ALIAS_ID0));
                // 	if(unlikely(err))
                // 		goto ser_init_failed;
                // 	dev_info(dev, "%s: slave id %i: 0x%X\n", __func__, i, val);

                // 	val = serializer.i2c_alias[i];
                // 	if(val == 0) {
                // 		continue;
                // 	}
                // 	err = ds90ub954_write_rx_port(priv, rx_port,
                // 				      ti954::REG_ALIAS_ID0+i,
                // 				      (val<<ti954::ALIAS_ID0));
                // 	if(unlikely(err))
                // 		goto ser_init_failed;
                // 	dev_info(dev, "%s: alias id %i: 0x%X\n", __func__, i, val);
                // }

                // TODO: need vc_map from devicetree first
                //
                // // set virtual channel id mapping
                // self.write_rx_port(rx_port, ti954::REG_CSI_VC_MAP, ds90ub953.vc_map)?;
                //
                // let val = ds90ub953.vc_map & 0b11;
                // dev_info!(dev, "VC-ID 0 mapped to {val}\n");
                // let val = (ds90ub953.vc_map & 0b1100) >> 2;
                // dev_info!(dev, "VC-ID 1 mapped to {val}\n");
                // let val = (ds90ub953.vc_map & 0b110000) >> 4;
                // dev_info!(dev, "VC-ID 2 mapped to {val}\n");
                // let val = (ds90ub953.vc_map & 0b11000000) >> 6;
                // dev_info!(dev, "VC-ID 3 mapped to {val}\n");

                // all rx_port specific registers set for rx_port X
                dev_info!(dev, "init of deserializer rx_port {rx_port} successful\n");
                Ok(())
            };

            if init_serializer().is_err() {
                dev_err!(dev, "init deserializer rx_port {rx_port} failed\n");
                dev_err!(dev, "deserializer rx_port {rx_port} is deactivated\n");

                self.serializers[i] = None;

                // DISABLE RX PORT
                let Ok(mut val) = self.read(ti954::REG_RX_PORT_CTL) else {
                    continue;
                };
                val &= 0xFF ^ (1 << (ti954::PORT0_EN + rx_port.to_u32()));
                if self.write(ti954::REG_RX_PORT_CTL, val).is_err() {
                    continue;
                }
                // DISABLE CSI FORWARDING
                let Ok(mut val) = self.read(ti954::REG_FWD_CTL1) else {
                    continue;
                };
                val |= 1 << (ti954::FWD_PORT0_DIS + rx_port.to_u32());
                let _ = self.write(ti954::REG_FWD_CTL1, val);
            }
        }

        // setup gpio forwarding, default all input
        self.write(
            ti954::REG_GPIO_INPUT_CTL,
            (1 << ti954::GPIO6_INPUT_EN)
                | (1 << ti954::GPIO5_INPUT_EN)
                | (1 << ti954::GPIO4_INPUT_EN)
                | (1 << ti954::GPIO3_INPUT_EN)
                | (1 << ti954::GPIO2_INPUT_EN)
                | (1 << ti954::GPIO1_INPUT_EN)
                | (1 << ti954::GPIO0_INPUT_EN),
        )?;
        self.write(ti954::REG_GPIO0_PIN_CTL, 0)?;
        self.write(ti954::REG_GPIO1_PIN_CTL, 0)?;
        self.write(ti954::REG_GPIO2_PIN_CTL, 0)?;
        self.write(ti954::REG_GPIO3_PIN_CTL, 0)?;
        self.write(ti954::REG_GPIO4_PIN_CTL, 0)?;
        self.write(ti954::REG_GPIO5_PIN_CTL, 0)?;
        self.write(ti954::REG_GPIO6_PIN_CTL, 0)?;

        dev_info!(dev, "init ds90ub954 done\n");
        Ok(())
    }

    fn read(&mut self, register: u32) -> Result<u32> {
        self.regmap.read(register).map_err(|err| {
            dev_err!(
                self.i2c_client.as_ref(),
                "cannot read register 0x{register:02x} ({err:?})!\n"
            );
            err
        })
    }

    fn write(&mut self, register: u32, value: u32) -> Result<()> {
        self.regmap.write(register, value).map_err(|err| {
            dev_err!(
                self.i2c_client.as_ref(),
                "cannot write register 0x{register:02x} ({err:?})!\n"
            );
            err
        })
    }

    fn read_rx_port(&mut self, rx_port: RxPort, addr: u32) -> Result<u32> {
        let i2c_client = self.i2c_client.clone();
        let dev = i2c_client.as_ref();

        // Check if port is selected, select port if needed
        if self.selected_rx_port != Some(rx_port) {
            let port_reg = match rx_port {
                RxPort::Zero => 0b1, // leave ti954::RX_READ_PORT at 0
                RxPort::One => 0b10 | (1 << ti954::RX_READ_PORT),
                RxPort::Both => {
                    dev_err!(
                        dev,
                        "attempted to read from both rx ports at the same time\n"
                    );
                    0b1 // fallback to port 0
                }
            };

            self.write(ti954::REG_FPD3_PORT_SEL, port_reg)
                .map_err(|err| {
                    dev_err!(dev, "error writing register ti954::REG_FPD3_PORT_SEL\n",);
                    err
                })?;

            self.selected_rx_port = Some(rx_port);
        }
        self.read(addr).map_err(|err| {
            dev_err!(dev, "error read register (0x{:02x})\n", addr);
            err
        })
    }

    fn write_rx_port(&mut self, rx_port: RxPort, addr: u32, value: u32) -> Result<()> {
        let i2c_client = self.i2c_client.clone();
        let dev = i2c_client.as_ref();

        // Check if port is selected, select port if needed
        if self.selected_rx_port != Some(rx_port) {
            let port_reg = match rx_port {
                RxPort::Zero => 0b01, // set RX_WRITE_PORT_0
                RxPort::One => 0b10,  // set RX_WRITE_PORT_1
                RxPort::Both => 0b11, // set RX_WRITE_PORT_0 & 1
            };

            self.write(ti954::REG_FPD3_PORT_SEL, port_reg)
                .map_err(|err| {
                    dev_err!(dev, "error writing register ti954::REG_FPD3_PORT_SEL\n",);
                    err
                })?;

            self.selected_rx_port = Some(rx_port);
        }
        self.write(addr, value).map_err(|err| {
            dev_err!(dev, "error writing register (0x{:02x})\n", addr);
            err
        })
    }

    fn init_testpattern(&mut self) -> Result<()> {
        for i in (0..DS90UB95X_TP_REG_VAL.len()).step_by(2) {
            self.write(
                DS90UB95X_TP_REG_VAL[i].into(),
                DS90UB95X_TP_REG_VAL[i + 1].into(),
            )
            .map_err(|err| {
                dev_info!(
                    self.i2c_client.as_ref(),
                    "954: enable test pattern failed\n"
                );
                err
            })?;
        }
        dev_info!(self.i2c_client.as_ref(), "enable test pattern successful\n");
        Ok(())
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

#[derive(Debug, Clone, Copy)]
struct Ds90ub953 {
    // struct i2c_client *client;
    // struct regmap *regmap;
    rx_channel: RxPort,
    test_pattern: bool,
    i2c_address: u32,
    csi_lane_count: u32,
    // int i2c_alias_num; // number of slave alias pairs
    // int i2c_slave[NUM_ALIAS]; // array with the i2c slave addresses
    // int i2c_alias[NUM_ALIAS]; // array with the i2c alias addresses
    continuous_clock: bool,
    i2c_pass_through_all: bool,

    gpio: [Ds90ub953GpioConfig; 4],

    // reference output clock control parameters
    hs_clk_div: u32,
    div_m_val: u32,
    div_n_val: u32,

    virtual_channel_map: u32,
}
#[derive(Debug, Clone, Copy)]
struct Ds90ub953GpioConfig {
    output_enable: u32,
    control: u32,
}
fn ds90ub953_parse_dt(dev: &kernel::device::Device) -> Result<[Option<Ds90ub953>; NUM_SERIALIZER]> {
    // TODO: This function body is pseudo-code.
    // There isn't yet a Rust abstraction for parsing nested device tree nodes.

    dev_warn!(dev, "ds90ub953_parse_dt is not yet implemented\n");

    let mut res = [const { None }; NUM_SERIALIZER];

    // TODO: needs Rust abstraction for iterating over devicetree nodes
    // let serializers = of_get_child_by_name(des, "serializers");
    for i in 0..NUM_SERIALIZER {
        let serializer = dev; // FIXME

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

        let rx_channel = RxPort::from(get_u32(c_str!("rx-channel"), 0), dev);

        let test_pattern = serializer.property_read_bool(c_str!("test-pattern"));
        if test_pattern {
            dev_info!(dev, "test-pattern enabled\n");
        } else {
            dev_info!(dev, "test-pattern disabled\n");
        }

        let csi_lane_count = get_u32(c_str!("csi-lane-count"), 4);

        let gpio = [
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

        res[i] = Some(Ds90ub953 {
            gpio,
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
        });
    }

    dev_info!(dev, "ds90ub953_parse_dt done\n");

    Ok(res)
}

impl Drop for Ds90ub954 {
    fn drop(&mut self) {
        // TODO
        //
        // ds90ub953_free(priv);
        self.pwr_disable();

        pr_info!("Goodbye from DS90UB954 driver\n");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RxPort {
    Zero = 0,
    One = 1,
    Both = 2,
}

impl RxPort {
    fn from(value: u32, dev: &kernel::device::Device) -> RxPort {
        match value {
            x if x == RxPort::Zero as _ => RxPort::Zero,
            x if x == RxPort::One as _ => RxPort::One,
            x if x == RxPort::Both as _ => RxPort::Both,
            _ => {
                dev_err!(dev, "invalid rx port ({value}), fallback to 0");
                RxPort::Zero
            }
        }
    }

    fn to_u32(self) -> u32 {
        self as u32
    }
}

impl core::fmt::Display for RxPort {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.to_u32())
    }
}
