#![allow(dead_code)]
use crate::aarch64::intrinsic::addr::{GPFSEL0, GPFSEL1, GPPUD, GPPUDCLK0, GPPUDCLK1, MMIO_BASE};
use crate::aarch64::intrinsic::{delay_us, get_u32, put_u32};
use crate::driver::mbox::set_power_state;

const PM_RSTC: u64 = MMIO_BASE + 0x0010001c;
const PM_RSTS: u64 = MMIO_BASE + 0x00100020;
const PM_WDOG: u64 = MMIO_BASE + 0x00100024;
const PM_WDOG_MAGIC: u32 = 0x5a000000;
const PM_RSTC_FULLRST: u32 = 0x00000020;

// Shut down the QEMU machine.
pub fn power_off() {
    for r in 0u32..16 {
        set_power_state(r, 0);
    }
    // power off gpio pins (but not VCC pins)
    put_u32(GPFSEL0, 0);
    put_u32(GPFSEL1, 0);
    put_u32(GPPUD, 0);
    delay_us(150);
    put_u32(GPPUDCLK0, 0xffffffff);
    put_u32(GPPUDCLK1, 0xffffffff);
    delay_us(150);
    put_u32(GPPUDCLK0, 0);
    put_u32(GPPUDCLK1, 0);

    // power off the SoC (GPU + CPU)
    let mut rsts = get_u32(PM_RSTS);
    rsts &= !0xfffffaaa;
    rsts |= 0x555;
    put_u32(PM_RSTS, PM_WDOG_MAGIC | rsts);
    put_u32(PM_WDOG, PM_WDOG_MAGIC | 10);
    put_u32(PM_RSTC, PM_WDOG_MAGIC | PM_RSTC_FULLRST);
}