use core::arch::asm;
use spin::RwLock;
use crate::aarch64::intrinsic::addr::LOCAL_BASE;
use crate::aarch64::intrinsic::{get_timer_freq, put_u32};
use crate::get_cpu_id;

struct Clock {
    one_ms: u64,
    handler: Option<fn()>,
}

static CLOCK: RwLock<Clock> = RwLock::new(Clock {
    one_ms: 0,
    handler: None,
});

const CORE_CLOCK_ENABLE: u32 = 1 << 1;

const fn core_clock_ctrl(id: usize) -> u64 {
    LOCAL_BASE + 0x40 + (id as u64) * 4
}

pub fn init_clock() {
    CLOCK.write().one_ms = get_timer_freq() / 1000;
    unsafe {
        // reserve 1s for timer interrupt
        asm!("msr cntp_ctl_el0, {}", in(reg) 1u64);
    }
    reset_clock(1000);
    put_u32(core_clock_ctrl(get_cpu_id()), CORE_CLOCK_ENABLE);
}

pub fn reset_clock(countdown_ms: u64) {
    unsafe {
        asm!("msr cntp_tval_el0, {}", in(reg) (CLOCK.read().one_ms * countdown_ms));
    }
}

pub fn set_clock_handler(handler: fn()) {
    CLOCK.write().handler = Some(handler);
}

pub fn clock_handler() {
    if let Some(func) = CLOCK.read().handler {
        func();
    } else {
        panic!("clock handler is null");
    }
}