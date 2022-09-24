use core::mem::MaybeUninit;
use crate::common::list::ListLink;
use crate::common::sem::Semaphore;
use crate::define_init;
use crate::kernel::kernel_entry;
use crate::kernel::sched::SchInfo;

static mut ROOT_PROC: MaybeUninit<Process> = MaybeUninit::uninit();

pub enum ProcessState {
    Unused,
    Runnable,
    Running,
    Sleeping,
    Zombie,
}

pub struct UserContext {}

pub struct KernelContext {}

pub struct Process {
    pub pid: usize,
    pub killed: bool,
    pub idle: bool,
    pub exit_code: usize,
    pub state: ProcessState,
    pub child_exit: Semaphore,
    pub children: ListLink,
    pub ptnode: ListLink,
    pub parent: Option<*mut Process>,
    pub sch_info: SchInfo,
    pub kernel_stack: *mut u8,
    pub user_context: *mut UserContext,
    pub kernel_context: *mut KernelContext,
}

pub fn exit(code: i32) -> ! {
    todo!()
}

pub fn wait(code: i32) -> i32 {
    todo!()
}

pub fn init_proc(p: *mut Process) {
    todo!()
}

pub fn start_proc(p: *mut Process, entry: *const fn(), arg: usize) {
    todo!()
}

pub unsafe extern "C" fn init_root_process() {
    init_proc(ROOT_PROC.as_mut_ptr());
    ROOT_PROC.assume_init_mut().parent = Some(ROOT_PROC.as_mut_ptr());
    start_proc(ROOT_PROC.as_mut_ptr(), kernel_entry as *const fn(), 123456);
}

define_init!(init_root_process);