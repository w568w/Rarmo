use crate::aarch64::intrinsic::{disable_trap, reset_esr_el1, set_ttbr0_el1, set_vbar_el1};
use crate::aarch64::kernel_pt::invalid_pt;
use crate::aarch64::mmu::kernel2physical;
use crate::{define_early_init, get_cpu_id};
use crate::driver::clock::{init_clock, set_clock_handler};
use crate::kernel::sched::Sched;

const CPU_NUM: usize = 4;

#[derive(Debug, Clone, Copy)]
pub struct CPU {
    pub online: bool,
    pub sched: Option<Sched>,
}

static mut CPUS: [CPU; CPU_NUM] = [CPU { online: false, sched: None }; CPU_NUM];

fn cpu_clock_handler() {}

pub extern "C" fn init_cpu_clock_handler() {
    set_clock_handler(cpu_clock_handler);
}

define_early_init!(init_cpu_clock_handler);

extern "C" {
    pub fn exception_vector();
}

pub fn set_cpu_on() {
    assert!(!disable_trap());
    set_ttbr0_el1(kernel2physical(&invalid_pt as *const _ as u64));
    set_vbar_el1(exception_vector as *const u8 as u64);
    reset_esr_el1();
    unsafe {
        init_clock();
        CPUS[get_cpu_id()].online = true;
    }
}

pub fn set_cpu_off() {
    disable_trap();
    unsafe {
        CPUS[get_cpu_id()].online = false;
    }
}

pub fn wait_all_cpu_off() {
    let mut id = 0;
    while id < CPU_NUM {
        if !unsafe { CPUS[id].online } {
            id += 1;
        }
    }
}