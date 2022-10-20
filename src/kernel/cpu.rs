use core::mem::MaybeUninit;

use alloc::boxed::Box;
use field_offset::offset_of;

use crate::aarch64::intrinsic::{disable_trap, get_time_ms, reset_esr_el1, set_ttbr0_el1, set_vbar_el1};
use crate::aarch64::kernel_pt::invalid_pt;
use crate::aarch64::mmu::kernel2physical;
use crate::driver::clock::{init_clock, set_clock_handler, reset_clock};
use crate::kernel::proc::create_idle_process;
use crate::kernel::sched::{start_idle_proc, Sched, preemptive_sched};
use crate::{define_early_init, get_cpu_id, println};
use crate::common::list::ListNode;
use crate::common::tree::{RbTree, RbTreeLink};

const CPU_NUM: usize = 4;

pub struct Timer {
    triggered: bool,
    elapsed: u64,
    key: u64,
    link: RbTreeLink,
    handler: fn(&mut Timer),
    data: u64,
}

impl ListNode<RbTreeLink> for Timer {
    fn get_link_offset() -> usize { offset_of!(Timer => link).get_byte_offset() }
}

impl Timer {
    pub fn new(handler: fn(&mut Timer), data: u64, elapse_ms: u64) -> Self {
        Self {
            triggered: false,
            elapsed: elapse_ms,
            key: 0,
            link: RbTreeLink::new(),
            handler,
            data,
        }
    }
}

pub struct CPU {
    pub online: bool,
    pub timers: RbTree<Timer>,
    pub sched: Sched,
}

static mut CPUS: [CPU; CPU_NUM] = {
    let mut cpus: [MaybeUninit<CPU>; CPU_NUM] = MaybeUninit::uninit_array();
    let mut i = 0;
    while i < CPU_NUM {
        cpus[i] = MaybeUninit::new(CPU {
            online: false,
            sched: Sched::uninit(),
            timers: RbTree::new(timer_cmp),
        });
        i += 1;
    }
    unsafe { core::mem::transmute(cpus) }
};

pub unsafe extern "C" fn init_sched() {
    for info in CPUS.iter_mut() {
        info.sched.init();
    }
}

define_early_init!(init_sched);

const fn timer_cmp(a: &mut Timer, b: &mut Timer) -> bool {
    a.key < b.key
}

fn refresh_clock_by_timer() {
    match get_cpu_info().timers.minimum() {
        None => {
            reset_clock(1000);
        }
        Some(timer) => {
            let cur = get_time_ms();
            if timer.key < cur {
                reset_clock(0);
            } else {
                reset_clock(timer.key - cur);
            }
        }
    }
}

fn cpu_clock_handler() {
    reset_clock(1000);
    loop {
        let node = get_cpu_info().timers.minimum();
        if node.is_none() {
            break;
        }
        let timer = node.unwrap();
        if get_time_ms() < timer.key {
            break;
        }
        cancel_cpu_timer(timer);
        timer.triggered = true;
        (timer.handler)(timer);
    }
}

pub fn add_cpu_timer(timer: &mut Timer) {
    timer.triggered = false;
    timer.key = get_time_ms() + timer.elapsed;
    get_cpu_info().timers.insert(timer);
    refresh_clock_by_timer();
}

fn cancel_cpu_timer(timer: &mut Timer) {
    assert!(!timer.triggered);
    get_cpu_info().timers.delete(timer);
    refresh_clock_by_timer();
}

pub extern "C" fn init_cpu_clock_handler() {
    set_clock_handler(cpu_clock_handler);
}

define_early_init!(init_cpu_clock_handler);



// Get current CPU's Info.
// It is safe because it only returns a reference to the *current* CPU's, not others'.
pub fn get_cpu_info() -> &'static mut CPU {
    unsafe { &mut CPUS[get_cpu_id()] }
}

extern "C" {
    pub fn exception_vector();
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
    // After initializing the IDLE process, the scheduler will be started.
    let timer = Box::new(Timer::new(preemptive_sched, 0, 10));
    add_cpu_timer(Box::leak(timer));
    let timer = Box::new(Timer::new(watch_dog, 0, 5000));
    add_cpu_timer(Box::leak(timer));
}

pub fn watch_dog(timer: &mut Timer) {
    add_cpu_timer(timer);
    println!("CPU{}: Watch dog triggered!", get_cpu_id());
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
