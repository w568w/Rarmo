use alloc::boxed::Box;
use core::mem::MaybeUninit;
use core::ptr;
use crate::aarch64::mmu::PAGE_SIZE;
use crate::common::list::ListLink;
use crate::common::sem::Semaphore;
use crate::define_init;
use crate::kernel::kernel_entry;
use crate::kernel::mem::kalloc_page;
use crate::kernel::sched::{activate, SchInfo, thisproc};

static mut ROOT_PROC: MaybeUninit<Process> = MaybeUninit::uninit();

pub enum ProcessState {
    Unused,
    Runnable,
    Running,
    Sleeping,
    Zombie,
}

#[repr(C)]
pub struct UserContext {}

#[repr(C)]
pub struct KernelContext {
    // x19-x30
    pub x: [usize; 12],
}

#[repr(C)]
pub struct Process {
    pub pid: usize,
    pub killed: bool,
    pub idle: bool,
    pub exit_code: usize,
    pub state: ProcessState,
    pub child_exit: Option<Semaphore>,
    pub children: ListLink,
    pub ptnode: ListLink,
    pub parent: Option<*mut Process>,
    pub sch_info: SchInfo,
    pub kernel_stack: *mut u8,
    pub user_context: *mut UserContext,
    pub kernel_context: *mut KernelContext,
}

impl Process {
    // Fill base fields of a process.
    pub fn fill_default_fields(&mut self) {
        self.pid = 0;
        self.killed = false;
        self.idle = false;
        self.exit_code = 0;
        self.state = ProcessState::Unused;
        self.child_exit = None;
        self.children = ListLink::new();
        self.children.init();
        self.ptnode = ListLink::new();
        self.ptnode.init();
        self.parent = None;
        self.sch_info = SchInfo {};
        self.kernel_stack = ptr::null_mut();
        self.user_context = ptr::null_mut();
        self.kernel_context = ptr::null_mut();
    }
}

impl Default for Box<Process> {
    fn default() -> Self {
        let mut proc = Self::new(unsafe { core::mem::zeroed() });
        proc.fill_default_fields();
        proc
    }
}

pub fn exit(code: usize) -> ! {
    // todo concurrency
    let proc = thisproc() as &mut Process;
    proc.exit_code = code as usize;
    todo!()
}

pub fn wait(code: usize) -> usize {
    todo!()
}

pub unsafe fn init_proc(p: *mut Process) {
    let mut proc = &mut *p;
    proc.fill_default_fields();
    let stack_top = kalloc_page();
    proc.kernel_stack = stack_top.byte_add(PAGE_SIZE);
    proc.user_context = proc.kernel_stack.byte_sub(16).byte_sub(core::mem::size_of::<UserContext>()) as *mut UserContext;
    proc.kernel_context = proc.user_context.byte_sub(core::mem::size_of::<KernelContext>()) as *mut KernelContext;
    // todo PID
    // todo concurrency
}

pub fn create_proc() -> *mut Process {
    let mut p: Box<Process> = Default::default();
    unsafe {
        init_proc(p.as_mut());
    }
    Box::leak(p)
}

pub fn start_proc(p: &mut Process, entry: *const fn(), arg: usize) -> usize {
    // todo concurrency
    // If the process does not have a parent, its parent is the root process.
    if p.parent.is_none() {
        p.parent = Some(unsafe { ROOT_PROC.as_mut_ptr() });
    }
    // todo setup the kernel context

    let pid = p.pid;
    activate(p);
    pid
}

pub unsafe extern "C" fn init_root_process() {
    init_proc(ROOT_PROC.as_mut_ptr());
    ROOT_PROC.assume_init_mut().parent = Some(ROOT_PROC.as_mut_ptr());
    start_proc(ROOT_PROC.assume_init_mut(), kernel_entry as *const fn(), 123456);
}

define_init!(init_root_process);