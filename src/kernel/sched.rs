use crate::{common::{
    list::ListNode,
    Container,
}, define_early_init, kernel::proc::{exit, KernelContext, Process, ProcessState}, println};
use core::arch::global_asm;
use core::mem::MaybeUninit;
use field_offset::offset_of;
use spin::{Mutex, MutexGuard};
use crate::common::tree::{RbTree, RbTreeLink};

use super::cpu::get_cpu_info;

pub struct Sched {
    pub cur_proc: Option<*mut Process>,
    pub idle_proc: Option<*mut Process>,
}

static mut RUN_QUEUE: MaybeUninit<RbTree<SchInfo>> = MaybeUninit::uninit();

pub extern "C" fn init_run_queue() {
    unsafe {
        RUN_QUEUE = MaybeUninit::new(RbTree::new(|a, b| true));
    }
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
}

impl SchInfo {
    pub fn uninit() -> Self {
        Self {
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
    let _lock = acquire_sched_lock();
    sched(_lock, ProcessState::Runnable);
}

fn get_cpu_sched() -> &'static mut Sched {
    &mut get_cpu_info().sched
}

#[inline(always)]
pub fn is_dead(proc: &Process) -> bool {
    let lock = acquire_sched_lock();
    let ret = matches!(proc.state, ProcessState::Zombie) || proc.killed;
    release_sched_lock(lock);
    ret
}

pub fn try_thisproc() -> Option<&'static mut Process> {
    let proc = get_cpu_sched().cur_proc;
    proc.map(|proc| unsafe { &mut *proc })
}

pub fn thisproc() -> &'static mut Process {
    try_thisproc().unwrap()
}

pub fn activate(proc: &mut Process) {
    let lock = acquire_sched_lock();
    match proc.state {
        ProcessState::Unused | ProcessState::Sleeping => {
            update_proc_state(proc, ProcessState::Runnable);
        }
        ProcessState::Runnable | ProcessState::Running => {}
        ProcessState::Zombie => {
            panic!("activate zombie process");
        }
    }
    release_sched_lock(lock);
}

pub fn acquire_sched_lock<'a>() -> MutexGuard<'a, ()> {
    SCHED_LOCK.lock()
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

// Choose the next process to run.
fn pick_next() -> &'static mut Process {
    let sch_info = unsafe { RUN_QUEUE.assume_init_mut().head() };
    if let Some(sch_info) = sch_info {
        let proc = Process::get_parent::<Process>(sch_info);
        proc
    } else {
        let idle_proc = get_cpu_sched().idle_proc.unwrap();
        unsafe { &mut *idle_proc }
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

pub fn sched(sched_lock: MutexGuard<()>, new_state: ProcessState) {
    let this = thisproc();
    assert!(matches!(this.state, ProcessState::Running));
    update_this_state(new_state);
    let next = pick_next();
    update_this_proc(next);
    assert!(matches!(next.state, ProcessState::Runnable));
    update_proc_state(next, ProcessState::Running);
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
