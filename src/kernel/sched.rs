use crate::kernel::proc::{exit, KernelContext, Process, ProcessState};
use core::arch::global_asm;
use spin::{Mutex, MutexGuard};

use super::cpu::get_cpu_info;

pub struct Sched {
    pub cur_proc: Option<*mut Process>,
    pub idle_proc: Option<*mut Process>,
}

impl Sched {
    pub const fn uninit() -> Self {
        Sched {
            cur_proc: None,
            idle_proc: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SchInfo {}

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
            proc.state = ProcessState::Runnable;
            // todo add it to the sched queue
        }
        ProcessState::Runnable | ProcessState::Running => {}
        ProcessState::Zombie => {
            panic!("attempt to activate zombie process");
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
    // todo
    let mut proc = get_cpu_sched().idle_proc.unwrap();
    unsafe { &mut *proc }
}

fn update_this_state(state: ProcessState) {
    thisproc().state = state;
    // todo if state == ProcessState::Runnable, add thisproc() to the sched queue
    // todo if state == ProcessState::Zombie, remove thisproc() from the sched queue
}

pub fn sched(sched_lock: MutexGuard<()>, new_state: ProcessState) {
    let this = thisproc();
    assert!(matches!(this.state, ProcessState::Running));
    update_this_state(new_state);
    let next = pick_next();
    update_this_proc(next);
    assert!(matches!(next.state, ProcessState::Runnable));
    next.state = ProcessState::Running;
    if next.pid != this.pid {
        unsafe {
            swtch(next.kernel_context, &mut this.kernel_context);
        }
    }
    // When executing this line, we have been back to the process that was running before the call to `sched`.
    release_sched_lock(sched_lock);
}

pub extern "C" fn proc_entry(real_entry: *const fn(usize), arg: usize) -> ! {
    unsafe {
        force_release_sched_lock();
        (*real_entry)(arg);
    }
    exit(0);
}

