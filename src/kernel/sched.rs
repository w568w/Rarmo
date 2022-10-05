use crate::{common::{
    list::ListNode,
    Container,
}, define_early_init, kernel::proc::{exit, KernelContext, Process, ProcessState}};
use core::arch::global_asm;
use core::mem::MaybeUninit;
use field_offset::offset_of;
use spin::{Mutex, MutexGuard};
use crate::aarch64::intrinsic::get_time_us;
use crate::common::tree::{RbTree, RbTreeLink};
use crate::kernel::proc::guard::check_guard_bits;

use super::cpu::get_cpu_info;

pub struct Sched {
    pub cur_proc: Option<*mut Process>,
    pub idle_proc: Option<*mut Process>,
}

static mut RUN_QUEUE: MaybeUninit<RbTree<SchInfo>> = MaybeUninit::uninit();

pub unsafe extern "C" fn init_run_queue() {
    RUN_QUEUE = MaybeUninit::new(RbTree::new(|a, b| {
        if a.vruntime != b.vruntime {
            a.vruntime < b.vruntime
        } else {
            Process::get_parent::<Process>(a).pid < Process::get_parent::<Process>(b).pid
        }
    }));
}
define_early_init!(init_run_queue);

impl Sched {
    // Note: this function will not initialize the run queue. DO IT MANUALLY.
    pub const fn uninit() -> Self {
        Self {
            cur_proc: None,
            idle_proc: None,
        }
    }

    pub fn init(&mut self) {}
}

#[repr(C)]
pub struct SchInfo {
    pub ptnode: RbTreeLink,
    pub vruntime: u64,
    pub nice: usize,
    pub start_time: u64,
}

impl SchInfo {
    pub fn uninit() -> Self {
        Self {
            start_time: 0,
            nice: SCHED_MEDIUM_NICE,
            vruntime: 0,
            ptnode: RbTreeLink::new(),
        }
    }
    pub fn init(&mut self) {}
}

impl ListNode<RbTreeLink> for SchInfo {
    fn get_link_offset() -> usize {
        offset_of!(SchInfo => ptnode).get_byte_offset()
    }
}

static SCHED_LOCK: Mutex<()> = Mutex::new(());

global_asm!(include_str!("../aarch64/swtch.asm"));
extern "C" {
    fn swtch(new: *mut KernelContext, old: *mut *mut KernelContext);
}

#[inline(always)]
pub fn yield_() {
    if let Some(lock) = try_acquire_sched_lock() {
        sched(lock, ProcessState::Runnable);
    }
}

fn get_cpu_sched() -> &'static mut Sched {
    &mut get_cpu_info().sched
}

#[inline(always)]
pub fn is_zombie(proc: &Process) -> bool {
    let lock = acquire_sched_lock();
    let ret = matches!(proc.state, ProcessState::Zombie);
    release_sched_lock(lock);
    ret
}

#[inline(always)]
pub fn is_zombie_no_lock(proc: &Process) -> bool {
    matches!(proc.state, ProcessState::Zombie)
}

pub fn try_thisproc() -> Option<&'static mut Process> {
    let proc = get_cpu_sched().cur_proc;
    proc.map(|proc| unsafe { &mut *proc })
}

pub fn thisproc() -> &'static mut Process {
    try_thisproc().unwrap()
}

pub fn activate_no_lock(proc: &mut Process) {
    _activate(proc);
}

pub fn activate(proc: &mut Process) {
    let lock = acquire_sched_lock();
    _activate(proc);
    release_sched_lock(lock);
}

fn _activate(proc: &mut Process) {
    match proc.state {
        ProcessState::Unused | ProcessState::Sleeping => {
            update_proc_state(proc, ProcessState::Runnable);
        }
        ProcessState::Runnable | ProcessState::Running => {}
        ProcessState::Zombie => {
            panic!("activate zombie process");
        }
    }
}

pub fn acquire_sched_lock<'a>() -> MutexGuard<'a, ()> {
    SCHED_LOCK.lock()
}

pub fn try_acquire_sched_lock<'a>() -> Option<MutexGuard<'a, ()>> {
    SCHED_LOCK.try_lock()
}

pub fn release_sched_lock(_sched_lock: MutexGuard<()>) {
    // We don't need to do anything here, since the MutexGuard will be dropped at the end of this scope.
}

pub unsafe fn force_release_sched_lock() {
    SCHED_LOCK.force_unlock();
}

// Update the current CPU's process to the next process.
// Since all processes are in a single queue, we are more likely to pass in a pointer than a reference.
fn update_this_proc(proc: *mut Process) {
    get_cpu_sched().cur_proc = Some(proc);
}

pub fn start_idle_proc() {
    let lock = acquire_sched_lock();
    let idle_proc = get_cpu_sched().idle_proc.unwrap();
    unsafe { (*idle_proc).state = ProcessState::Running };
    update_this_proc(idle_proc);
    release_sched_lock(lock);
}

const SCHED_PRIO_TO_WEIGHT: [u64; 40] = [
    /* -20 */     88761, 71755, 56483, 46273, 36291,
    /* -15 */     29154, 23254, 18705, 14949, 11916,
    /* -10 */     9548, 7620, 6100, 4904, 3906,
    /*  -5 */     3121, 2501, 1991, 1586, 1277,
    /*   0 */     1024, 820, 655, 526, 423,
    /*   5 */     335, 272, 215, 172, 137,
    /*  10 */     110, 87, 70, 56, 45,
    /*  15 */     36, 29, 23, 18, 15,
];
const SCHED_MEDIUM_NICE: usize = 20;
const SCHED_MIN_GRANULARITY_US: u64 = 1000;

// Choose the next process to run.
fn pick_next() -> *mut Process {
    let sch_info = unsafe { RUN_QUEUE.assume_init_mut().minimum() };
    if let Some(sch_info) = sch_info {
        let proc = Process::get_parent(sch_info);
        proc
    } else {
        get_cpu_sched().idle_proc.unwrap()
    }
}

fn update_proc_state(proc: &mut Process, state: ProcessState) {
    if proc.state == state {
        return;
    }
    proc.state = state;
    if proc.idle {
        return;
    }
    match proc.state {
        ProcessState::Unused => panic!("Try to set a process to unused state"),
        ProcessState::Runnable => {
            unsafe {
                RUN_QUEUE.assume_init_mut().insert(&mut proc.sch_info);
            }
        }
        ProcessState::Running | ProcessState::Sleeping | ProcessState::Zombie => {
            unsafe {
                RUN_QUEUE.assume_init_mut().delete(&mut proc.sch_info);
            }
        }
    }
}

fn update_this_state(state: ProcessState) {
    update_proc_state(thisproc(), state)
}


fn stop_tick_and_update_vruntime(cur: &mut Process) {
    if cur.sch_info.start_time > 0 {
        let wall_time = get_time_us() - cur.sch_info.start_time;
        cur.sch_info.vruntime += wall_time / SCHED_PRIO_TO_WEIGHT[SCHED_MEDIUM_NICE] * SCHED_PRIO_TO_WEIGHT[cur.sch_info.nice];
    }
}

fn start_tick(cur: &mut Process) {
    cur.sch_info.start_time = get_time_us();
}

pub fn sched(sched_lock: MutexGuard<()>, new_state: ProcessState) {
    assert!(!matches!(new_state, ProcessState::Unused | ProcessState::Running));

    let this = thisproc();
    assert!(matches!(this.state, ProcessState::Running));
    stop_tick_and_update_vruntime(this);
    if !this.idle {
        assert!(unsafe { check_guard_bits(this.kernel_stack) }, "Proc {} has corrupted its kernel stack", this.pid);
    }
    update_this_state(new_state);
    let mut next: *mut Process = pick_next();

    // todo: move this to a better place
    if matches!(this.state,ProcessState::Runnable)
        && unsafe { (*next).sch_info.vruntime } + SCHED_MIN_GRANULARITY_US > this.sch_info.vruntime {
        // If next process only has a little less time than current process, we don't need to switch.
        next = this;
    }

    let next = unsafe { &mut *next };
    update_this_proc(next);
    assert!(matches!(next.state, ProcessState::Runnable));
    update_proc_state(next, ProcessState::Running);
    start_tick(next);
    if next.pid != this.pid {
        unsafe {
            swtch(next.kernel_context, &mut this.kernel_context);
        }
    }
    // When executing this line, we have been back to the process that was running before the call to `sched`.
    release_sched_lock(sched_lock);
}

pub extern "C" fn proc_entry(real_entry: extern "C" fn(usize), arg: usize) -> ! {
    unsafe {
        force_release_sched_lock();
        real_entry(arg);
    }
    exit(0);
}
