use spin::{Mutex, MutexGuard};
use crate::kernel::proc::{Process, ProcessState};

#[derive(Debug, Clone, Copy)]
pub struct Sched {}

#[derive(Debug, Clone, Copy)]
pub struct SchInfo {}

static SCHED_LOCK: Mutex<()> = Mutex::new(());
pub fn yield_(){
    let _lock = acquire_sched_lock();
    sched(_lock, ProcessState::Runnable);
}
pub fn thisproc() -> *mut Process {
    todo!("thisproc");
}
pub fn activate(proc: *mut Process) {
    todo!("activate");
}
pub fn acquire_sched_lock<'a>() -> MutexGuard<'a, ()> {
    SCHED_LOCK.lock()
}

pub fn release_sched_lock(sched_lock: MutexGuard<()>) {
    // We don't need to do anything here, since the MutexGuard will be dropped at the end of the scope.
}

pub fn sched(sched_lock: MutexGuard<()>, new_state: ProcessState) {
    release_sched_lock(sched_lock);
}