use core::arch::asm;

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