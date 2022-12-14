#![allow(dead_code)]

use core::{
    arch::asm,
    ptr::{read_volatile, write_volatile},
};
use core::mem::size_of;

/* Some useful functions for `AArch64`. */

pub fn delay_us(us: u64) {
    let freq = get_timer_freq();
    let mut end = get_timestamp();
    let mut now;
    end += freq / 1_000_000 * us;
    loop {
        now = get_timestamp();
        if now >= end {
            break;
        }
    }
}

pub fn get_time_us() -> u64 {
    let freq = get_timer_freq();
    let now = get_timestamp();
    now * 1_000_000 / freq
}

pub fn get_time_ms() -> u64 {
    let freq = get_timer_freq();
    let now = get_timestamp();
    now * 1_000 / freq
}


#[inline(always)]
pub fn stop_cpu() -> ! {
    loop {
        unsafe { asm!("wfe"); }
    }
}

pub fn get_cpu_id() -> usize {
    let mut ret: usize;
    unsafe {
        asm!("mrs {}, mpidr_el1", out(reg) ret);
    }
    ret & 0xff
}

pub fn get_timer_freq() -> u64 {
    let mut ret;
    unsafe {
        asm!(
        "mrs x0, cntfrq_el0",
        out("x0") ret,
        options(nomem, nostack, preserves_flags)
        );
    };
    ret
}

#[inline(always)]
pub fn dc_civac<T>(addr: &T) {
    let addr = addr as *const T as usize;
    for i in 0..size_of::<T>() {
        _dc_civac(addr + i);
    }
}

#[inline(always)]
fn _dc_civac(addr: usize) {
    unsafe {
        asm!("dc civac, {}", in(reg) addr, options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn isb() {
    unsafe {
        asm!("isb", options(nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn dsb_sy() {
    unsafe {
        asm!("dsb sy", options(nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn arch_fence() {
    dsb_sy();
    isb();
}

#[inline(always)]
pub fn get_esr_el1() -> u64 {
    let mut ret;
    unsafe {
        asm!("mrs {}, esr_el1", out(reg) ret);
    }
    ret
}

#[inline(always)]
pub fn reset_esr_el1() {
    unsafe {
        asm!("msr esr_el1, xzr", options(nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn set_ttbr0_el1(val: u64) {
    unsafe {
        asm!("msr ttbr0_el1, {}", in(reg) val, options(nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn set_vbar_el1(val: u64) {
    unsafe {
        asm!("msr vbar_el1, {}", in(reg) val, options(nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn disable_trap() -> bool {
    let t: u64;
    unsafe {
        asm!("mrs {}, daif", out(reg) t, options(nostack, preserves_flags));
        if t != 0 {
            return false;
        }
        asm!("msr daif, {}", in(reg) 0xfu64 << 6, options(nostack, preserves_flags));
    }
    true
}

#[inline(always)]
pub fn enable_trap() -> bool {
    let t: u64;
    unsafe {
        asm!("mrs {}, daif", out(reg) t, options(nostack, preserves_flags));
        if t == 0 {
            return true;
        }
        asm!("msr daif, {}", in(reg) 0u64, options(nostack, preserves_flags));
    }
    false
}

#[inline(always)]
pub fn wfi() {
    unsafe {
        asm!("wfi", options(nostack, preserves_flags));
    }
}

// Why don't we need `::: "memory"` here, like what we did in C?
// Because Rust's `asm!` macro will automatically add a memory barrier for us. A perfect design!
// See: https://stackoverflow.com/questions/72823056/how-to-build-a-barrier-by-rust-asm.
// Only when you add `nomem` option, you are declaring that the assembly code will not access memory.
//
// p.s. to understand more about what "barrier" means, see:
// https://stackoverflow.com/questions/59596654/is-memory-fence-and-memory-barrier-same.
//
// There are four kinds of barriers:
// 1. Atomic fence: controls the order in which observers can see the effects of atomic memory operations.
// 2. Memory barrier: controls the order of actual operations against memory or memory-mapped I/O.
//      This is often a bigger hammer that can achieve similar results to an atomic fence, but at higher cost.
// 3. Compiler fence: controls the order of instructions the compiler generates. This is what `::: "memory"` in C and `compiler_fence` in Rust does.
// 4. Architectural barrier: controls the order of instructions the CPU executes. This differs in different architectures.
//      In ARM, it is called "memory barrier" and is implemented by `dmb`(Data Memory Barrier) instruction.
// Have a look at the ARMv8-A Architecture Reference Manual, https://developer.arm.com/documentation/100941/0101/Barriers.
// Also, take this discussion about Linux developers' talks on memory barriers as a reference:
// https://www.kernel.org/doc/Documentation/memory-barriers.txt.
pub fn get_timestamp() -> u64 {
    let mut ret;
    unsafe {
        asm!(
        "mrs x0, cntpct_el0",
        out("x0") ret,
        options(nostack, preserves_flags)
        );
    };
    ret
}


// For `get/put_*`, there's no need to protect them with architectural
// barriers, since they are intended to access device memory regions. These
// regions are already marked as nGnRnE in `kernel_pt`.
pub fn put_u32(address: u64, value: u32) {
    let ptr = address as *mut u32;
    unsafe {
        write_volatile(ptr, value);
    }
}

pub fn get_u32(address: u64) -> u32 {
    let ptr = address as *mut u32;
    unsafe { read_volatile(ptr) }
}

pub mod addr {
    pub const KERNEL_BASE: u64 = 0xffff000000000000;
    pub const MMIO_BASE: u64 = KERNEL_BASE + 0x3F000000;
    pub const LOCAL_BASE: u64 = KERNEL_BASE + 0x40000000;
    // GPIO Address definition
    pub const GPIO_BASE: u64 = MMIO_BASE + 0x200000;
    pub const GPFSEL0: u64 = GPIO_BASE + 0x00;
    pub const GPFSEL1: u64 = GPIO_BASE + 0x04;
    pub const GPFSEL2: u64 = GPIO_BASE + 0x08;
    pub const GPFSEL3: u64 = GPIO_BASE + 0x0C;
    pub const GPFSEL4: u64 = GPIO_BASE + 0x10;
    pub const GPFSEL5: u64 = GPIO_BASE + 0x14;
    pub const GPEDS0: u64 = GPIO_BASE + 0x40;
    pub const GPEDS1: u64 = GPIO_BASE + 0x44;
    pub const GPHEN0: u64 = GPIO_BASE + 0x64;
    pub const GPHEN1: u64 = GPIO_BASE + 0x68;
    pub const GPPUD: u64 = GPIO_BASE + 0x94;
    pub const GPPUDCLK0: u64 = GPIO_BASE + 0x98;
    pub const GPPUDCLK1: u64 = GPIO_BASE + 0x9C;
    // AUX Address definition
    pub const AUX_BASE: u64 = MMIO_BASE + 0x215000;
    pub const AUX_ENABLES: u64 = AUX_BASE + 0x04;
    pub const AUX_MU_IO_REG: u64 = AUX_BASE + 0x40;
    pub const AUX_MU_IER_REG: u64 = AUX_BASE + 0x44;
    pub const AUX_MU_IIR_REG: u64 = AUX_BASE + 0x48;
    pub const AUX_MU_LCR_REG: u64 = AUX_BASE + 0x4C;
    pub const AUX_MU_MCR_REG: u64 = AUX_BASE + 0x50;
    pub const AUX_MU_LSR_REG: u64 = AUX_BASE + 0x54;
    pub const AUX_MU_MSR_REG: u64 = AUX_BASE + 0x58;
    pub const AUX_MU_SCRATCH: u64 = AUX_BASE + 0x5C;
    pub const AUX_MU_CNTL_REG: u64 = AUX_BASE + 0x60;
    pub const AUX_MU_STAT_REG: u64 = AUX_BASE + 0x64;
    pub const AUX_MU_BAUD_REG: u64 = AUX_BASE + 0x68;
    // MailBox Address definition
    pub const VIDEOCORE_MBOX: u64 = MMIO_BASE + 0x0000B880;
    pub const MBOX_READ: u64 = VIDEOCORE_MBOX + 0x0;
    pub const MBOX_POLL: u64 = VIDEOCORE_MBOX + 0x10;
    pub const MBOX_SENDER: u64 = VIDEOCORE_MBOX + 0x14;
    pub const MBOX_STATUS: u64 = VIDEOCORE_MBOX + 0x18;
    pub const MBOX_CONFIG: u64 = VIDEOCORE_MBOX + 0x1C;
    pub const MBOX_WRITE: u64 = VIDEOCORE_MBOX + 0x20;
    // EEMC Address definition
    pub const EMMC_ARG2: u64 = MMIO_BASE + 0x00300000;
    pub const EMMC_BLKSIZECNT: u64 = MMIO_BASE + 0x00300004;
    pub const EMMC_ARG1: u64 = MMIO_BASE + 0x00300008;
    pub const EMMC_CMDTM: u64 = MMIO_BASE + 0x0030000C;
    pub const EMMC_RESP0: u64 = MMIO_BASE + 0x00300010;
    pub const EMMC_RESP1: u64 = MMIO_BASE + 0x00300014;
    pub const EMMC_RESP2: u64 = MMIO_BASE + 0x00300018;
    pub const EMMC_RESP3: u64 = MMIO_BASE + 0x0030001C;
    pub const EMMC_DATA: u64 = MMIO_BASE + 0x00300020;
    pub const EMMC_STATUS: u64 = MMIO_BASE + 0x00300024;
    pub const EMMC_CONTROL0: u64 = MMIO_BASE + 0x00300028;
    pub const EMMC_CONTROL1: u64 = MMIO_BASE + 0x0030002C;
    pub const EMMC_INTERRUPT: u64 = MMIO_BASE + 0x00300030;
    pub const EMMC_IRPT_MASK: u64 = MMIO_BASE + 0x00300034;
    pub const EMMC_IRPT_EN: u64 = MMIO_BASE + 0x00300038;
    pub const EMMC_CONTROL2: u64 = MMIO_BASE + 0x0030003C;
    pub const EMMC_SLOTISR_VER: u64 = MMIO_BASE + 0x003000fc;
}

pub mod aux {
    pub const AUX_UART_CLOCK: u32 = 250_000_000;

    pub fn aux_mu_baud(bandrate: u32) -> u32 {
        (AUX_UART_CLOCK / ((bandrate) * 8)) - 1
    }
}

pub mod mbox {
    pub const MBOX_RESPONSE: u32 = 0x80000000;
    pub const MBOX_FULL: u32 = 0x80000000;
    pub const MBOX_EMPTY: u32 = 0x40000000;
    pub const MBOX_REQUEST: u32 = 0;
    pub const MBOX_CH_POWER: u8 = 0;
    pub const MBOX_CH_FB: u8 = 1;
    pub const MBOX_CH_VUART: u8 = 2;
    pub const MBOX_CH_VCHIQ: u8 = 3;
    pub const MBOX_CH_LEDS: u8 = 4;
    pub const MBOX_CH_BTNS: u8 = 5;
    pub const MBOX_CH_TOUCH: u8 = 6;
    pub const MBOX_CH_COUNT: u8 = 7;
    pub const MBOX_CH_PROP: u8 = 8;

    // tags
    pub const MBOX_TAG_SETPOWER: u32 = 0x28001;
    pub const MBOX_TAG_SETCLKRATE: u32 = 0x38002;
    pub const MBOX_TAG_GET_ARM_MEMORY: u32 = 0x00010005;
    pub const MBOX_TAG_GET_CLOCK_RATE: u32 = 0x00030002;
    pub const MBOX_TAG_LAST: u32 = 0;

    use crate::aarch64::intrinsic::addr::{MBOX_READ, MBOX_STATUS, MBOX_WRITE};
    use crate::aarch64::intrinsic::{get_u32, put_u32};
    use crate::dsb_sy;

    pub fn write(buf_paddr: u32, channel: u8) {
        while get_u32(MBOX_STATUS) & MBOX_FULL != 0 {}
        dsb_sy();
        let channel_descriptor: u32 = <u8 as Into<u32>>::into(channel) & 0xf;
        let message_addr: u32 =
            ((buf_paddr & (!0xf)) | channel_descriptor) as u32;
        put_u32(MBOX_WRITE, message_addr);
        dsb_sy();
    }

    pub fn read(channel: u8) -> u32 {
        loop {
            dsb_sy();
            while get_u32(MBOX_STATUS) & MBOX_EMPTY != 0 {}
            dsb_sy();
            let r = get_u32(MBOX_READ);
            if (r & 0xf) == channel as u32 {
                return r >> 4;
            }
        }
    }
}