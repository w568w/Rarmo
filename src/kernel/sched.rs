use core::arch::global_asm;
use spin::{Mutex, MutexGuard};
use crate::kernel::proc::{KernelContext, Process, ProcessState};

#[derive(Debug, Clone, Copy)]
pub struct Sched {}

#[derive(Debug, Clone, Copy)]
pub struct SchInfo {}

static SCHED_LOCK: Mutex<()> = Mutex::new(());

global_asm!(include_str!("../aarch64/swtch.asm"));
extern "C" {
    fn swtch(new: *mut KernelContext, old: *mut *mut KernelContext);
}

pub fn yield_() {
    let _lock = acquire_sched_lock();
    sched(_lock, ProcessState::Runnable);
}

pub fn thisproc() -> &'static mut Process {
    todo!("thisproc");
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

fn update_this_proc(proc: &mut Process) {
    todo!();
}

fn pick_next() -> &'static mut Process {
    todo!()
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