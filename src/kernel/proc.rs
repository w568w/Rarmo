use crate::aarch64::mmu::PAGE_SIZE;
use crate::common::list::ListLink;
use crate::common::sem::Semaphore;
use crate::define_init;
use crate::kernel::{get_kernel_stack_bottom, kernel_entry};
use crate::kernel::mem::kalloc_page;
use crate::kernel::sched::{activate, thisproc, SchInfo, proc_entry};
use alloc::boxed::Box;
use core::mem::MaybeUninit;
use core::ptr;
use crate::common::pool::LockedArrayPool;

static mut ROOT_PROC: MaybeUninit<Process> = MaybeUninit::uninit();

pub enum ProcessState {
    Unused,
    Runnable,
    Running,
    Sleeping,
    Zombie,
}

const PID_POOL_SIZE: usize = 1000;
static PID_POOL: LockedArrayPool<usize, PID_POOL_SIZE> = LockedArrayPool::new();

const fn pid_generator(fill_count: usize, i: usize) -> usize {
    PID_POOL_SIZE * fill_count + i + 1
}

#[repr(C)]
pub struct UserContext {
    pub spsr_el1: u64,
    pub elr_el1: u64,
    // q0-q31
    pub q: [f64; 64],
    // x0-x31
    pub x: [u64; 32],
}

#[repr(C)]
pub struct KernelContext {
    // q0-q31
    pub q: [f64; 64],
    // x0-x7
    pub x0: [u64; 8],
    // x19-x30
    pub x19: [u64; 12],
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

pub unsafe fn init_proc(p: &mut Process) {
    let mut proc = &mut *p;
    proc.fill_default_fields();
    let stack_top = kalloc_page();
    proc.kernel_stack = stack_top.byte_add(PAGE_SIZE);
    proc.user_context = proc
        .kernel_stack
        .byte_sub(16)
        .byte_sub(core::mem::size_of::<UserContext>()) as *mut UserContext;
    proc.kernel_context =
        proc.user_context
            .byte_sub(core::mem::size_of::<KernelContext>()) as *mut KernelContext;
    proc.pid = PID_POOL.alloc(pid_generator).unwrap();
    // todo concurrency
}

pub fn create_proc() -> *mut Process {
    let mut p: Box<Process> = Default::default();
    unsafe {
        init_proc(p.as_mut());
    }
    Box::leak(p)
}

pub fn start_proc(p: &mut Process, entry: *const fn(usize), arg: usize) -> usize {
    // todo concurrency
    // If the process does not have a parent, its parent is the root process.
    if p.parent.is_none() {
        p.parent = Some(unsafe { ROOT_PROC.as_mut_ptr() });
    }
    // Set the entry point of the process.
    let kcontext = unsafe { &mut *(p.kernel_context) };
    kcontext.x0[0] = entry as u64;
    kcontext.x0[1] = arg as u64;
    kcontext.x19[11] = proc_entry as *const extern "C" fn(*const fn(usize), usize) as u64;

    let pid = p.pid;
    activate(p);
    pid
}

pub fn create_idle_process() -> Box<Process> {
    let mut proc: Box<Process> = Default::default();
    proc.state = ProcessState::Running;
    proc.idle = true;
    proc.kernel_stack = get_kernel_stack_bottom();
    proc
}

pub unsafe extern "C" fn init_root_process() {
    let root = ROOT_PROC.assume_init_mut();
    init_proc(root);
    root.parent = Some(ROOT_PROC.as_mut_ptr());
    start_proc(root, kernel_entry as *const fn(usize), 123456);
}

define_init!(init_root_process);
