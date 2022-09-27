use core::mem::MaybeUninit;

use alloc::boxed::Box;

use crate::aarch64::intrinsic::{disable_trap, reset_esr_el1, set_ttbr0_el1, set_vbar_el1};
use crate::aarch64::kernel_pt::invalid_pt;
use crate::aarch64::mmu::kernel2physical;
use crate::driver::clock::{init_clock, set_clock_handler, reset_clock};
use crate::kernel::proc::create_idle_process;
use crate::kernel::sched::{start_idle_proc, yield_,Sched};
use crate::{define_early_init, get_cpu_id};

const CPU_NUM: usize = 4;

pub struct CPU {
    pub online: bool,
    pub sched: Sched,
}

static mut CPUS: [CPU; CPU_NUM] = {
    let mut cpus: [MaybeUninit<CPU>; CPU_NUM] = MaybeUninit::uninit_array();
    let mut i = 0;
    while i < CPU_NUM {
        cpus[i] = MaybeUninit::new(CPU {
            online: false,
            sched: Sched::uninit(),
        });
        i += 1;
    }
    unsafe { core::mem::transmute(cpus) }
};

fn cpu_clock_handler() {
    yield_();
    reset_clock(1000);
}

pub extern "C" fn init_cpu_clock_handler() {
    set_clock_handler(cpu_clock_handler);
}

define_early_init!(init_cpu_clock_handler);

extern "C" {
    pub fn exception_vector();
}

// Get current CPU's CPUInfo.
// It is safe because it only returns a reference to the *current* CPU's, not others'.
pub fn get_cpu_info() -> &'static mut CPU {
    unsafe { &mut CPUS[get_cpu_id()] }
}

// Set the cpu on.
// This function should be only called by IDLE process when it starts,
// and should be invoked only once for each hart, since it will initialize the IDLE process too.
pub fn set_cpu_on() {
    assert!(!disable_trap());
    set_ttbr0_el1(kernel2physical(&invalid_pt as *const _ as u64));
    set_vbar_el1(exception_vector as *const u8 as u64);
    reset_esr_el1();
    init_clock();
    get_cpu_info().online = true;
    // Init IDLE process
    let idle_proc = Box::leak(create_idle_process());
    get_cpu_info().sched.idle_proc = Some(idle_proc);
    start_idle_proc();
}

pub fn set_cpu_off() {
    disable_trap();
    get_cpu_info().online = false;
}

pub fn wait_all_cpu_off() {
    let mut id = 0;
    while id < CPU_NUM {
        if !unsafe { CPUS[id].online } {
            id += 1;
        }
    }
}
