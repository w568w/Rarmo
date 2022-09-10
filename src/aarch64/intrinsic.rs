use core::{
    arch::asm,
    ptr::{read_volatile, write_volatile},
};

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
