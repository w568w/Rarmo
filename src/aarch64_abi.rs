use core::{
    arch::asm,
    ptr::{read_volatile, write_volatile},
};

// Some useful functions for `AArch64`.

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

pub fn get_timestamp() -> u64 {
    let mut ret;
    unsafe {
        asm!(
        "mrs x0, cntpct_el0",
        out("x0") ret,
        options(nomem, nostack, preserves_flags)
        );
    };
    ret
}

pub fn put_u32(address: u64, value: u32) {
    let ptr = unsafe { &mut *(address as *mut u32) };
    unsafe {
        write_volatile(ptr, value);
    }
}

pub fn get_u32(address: u64) -> u32 {
    let ptr = unsafe { &mut *(address as *mut u32) };
    unsafe { read_volatile(ptr) }
}

pub mod addr {
    pub const KERNEL_BASE: u64 = 0xffff000000000000;
    pub const MMIO_BASE: u64 = KERNEL_BASE + 0x3F000000;
    // GPIO Address definition
    pub const GPIO_BASE: u64 = MMIO_BASE + 0x200000;
    pub const GPFSEL0: u64 = GPIO_BASE + 0x00;
    pub const GPFSEL1: u64 = GPIO_BASE + 0x04;
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
}

pub mod aux {
    pub const AUX_UART_CLOCK: u32 = 250_000_000;

    pub fn AUX_MU_BAUD(bandrate: u32) -> u32 {
        (AUX_UART_CLOCK / ((bandrate) * 8)) - 1
    }
}
