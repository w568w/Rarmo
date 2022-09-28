use crate::aarch64::mmu::PAGE_SIZE;
use crate::common::list::{ListLink, ListNode};
use crate::common::sem::Semaphore;
use crate::define_init;
use crate::kernel::{get_kernel_stack_bottom, kernel_entry};
use crate::kernel::mem::{kalloc_page, kfree_page};
use crate::kernel::sched::{activate, thisproc, SchInfo, proc_entry, try_thisproc, sched, acquire_sched_lock};
use alloc::boxed::Box;
use core::mem::MaybeUninit;
use core::ptr;
use field_offset::offset_of;
use spin::Mutex;
use crate::common::pool::LockedArrayPool;

static mut ROOT_PROC: MaybeUninit<Process> = MaybeUninit::uninit();

pub fn root_proc() -> &'static mut Process {
    unsafe { ROOT_PROC.assume_init_mut() }
}

pub enum ProcessState {
    Unused,
    Runnable,
    Running,
    Sleeping,
    Zombie,
}

static PROC_LOCK: Mutex<()> = Mutex::new(());

const PID_POOL_SIZE: usize = 1000;
static PID_POOL: LockedArrayPool<usize, PID_POOL_SIZE> = LockedArrayPool::new();

const fn pid_generator(fill_count: usize, i: usize) -> usize {
    // The PID will start from 1, so that 0 can be used as idle process's PID.
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
    pub child_exit: Semaphore,
    pub first_child: Option<*mut Process>,
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
        self.child_exit = Semaphore::new(0);
        self.first_child = None;
        self.ptnode = ListLink::new();
        self.ptnode.init();
        self.parent = None;
        self.sch_info = SchInfo {};
        self.kernel_stack = ptr::null_mut();
        self.user_context = ptr::null_mut();
        self.kernel_context = ptr::null_mut();
    }

    // All functions below do not acquire the lock. You MUST lock the process tree before calling them!

    // Attach a new child to the process.
    // It will also set the child's parent to this process.
    pub fn attach_child(&mut self, child: &mut Process) {
        child.parent = Some(self);
        if let Some(first_child) = self.first_child {
            let mut first_child = unsafe { &mut *first_child };
            first_child.ptnode.insert_at_first(child);
        } else {
            self.first_child = Some(child);
        }
    }

    // This function is private, and should only be called when proc tree being locked.
    // It does not change the `parent` of each child, you need to do it yourself.
    fn attach_children(&mut self, first_child: &mut Process) {
        if let Some(my_first_child) = self.first_child {
            let my_first_child = unsafe { &mut *my_first_child };
            my_first_child.ptnode.merge(first_child.link());
        } else {
            self.first_child = Some(first_child);
        }
    }

    pub fn transfer_all_children_to_root(&mut self) {
        // If I am the root, I don't need to do anything.
        if self.pid == root_proc().pid {
            return;
        }
        if let Some(first_child) = self.first_child {
            let mut first_child = unsafe { &mut *first_child };
            // Merge the child list to the root process's child list.
            for child in first_child.link().iter::<Process>(false) {
                child.parent = Some(root_proc());
            }
            root_proc().attach_children(first_child);
        }
        self.first_child = None;
    }

    pub fn detach_child(&mut self, child: &mut Process) {
        child.parent = None;
        if let Some(first_child) = self.first_child {
            let mut first_child = unsafe { &mut *first_child };
            if first_child.link().is_single() {
                // If the first child is the only child, we can just set the first child to `None`.
                self.first_child = None;
            } else {
                // or, there is more than one child.

                // If the first child is the one to be detached, we need to update `first_child`.
                if first_child.pid == child.pid {
                    self.first_child = first_child.link().next_ptr::<Process>();
                }
                // Remove the child from the list.
                child.ptnode.detach();
            }
        }
    }

    pub fn killable(&self) -> bool {
        !self.idle && !self.killed && self.pid != root_proc().pid
    }
}

impl ListNode for Process {
    fn get_link_offset() -> usize { offset_of!(Process => ptnode).get_byte_offset() }
}

impl Default for Box<Process> {
    fn default() -> Self {
        let mut proc = Self::new(unsafe { core::mem::zeroed() });
        proc.fill_default_fields();
        proc
    }
}

pub fn exit(code: usize) -> ! {
    let proc = thisproc();
    proc.exit_code = code;

    // Clean up stack and context.
    if proc.killable() {
        kfree_page(unsafe { proc.kernel_stack.byte_sub(PAGE_SIZE) });
        proc.kernel_context = ptr::null_mut();
        proc.user_context = ptr::null_mut();
    }
    proc.killed = true;

    // Transfer all children to the root process.
    let p_lock = PROC_LOCK.lock();
    proc.transfer_all_children_to_root();
    drop(p_lock);
    // Notify the parent that it is exiting.
    if let Some(parent) = proc.parent {
        unsafe { (*parent).child_exit.post() };
    }

    // This process is a zombie, and will be cleaned up by the parent's wait().
    let lock = acquire_sched_lock();

    sched(lock, ProcessState::Zombie);

    panic!("Zombie process should not be scheduled");
}

pub fn wait() -> Option<(usize, usize)> {
    let proc = thisproc();
    if proc.first_child.is_none() {
        return None;
    }
    // Wait for a child to exit.
    let _ = proc.child_exit.get_or_wait();

    let lock = PROC_LOCK.lock();
    let child = proc.first_child.unwrap();
    let child = unsafe { &mut *child };
    for x in child.link().iter::<Process>(false) {
        if matches!(x.state, ProcessState::Zombie) {
            let exit_code = x.exit_code;
            let pid = x.pid;
            proc.detach_child(x);
            // Scheduler has removed it and parent has detached it, so we can free it.
            let _proc_to_be_dropped = unsafe { Box::from_raw(x) };
            return Some((pid, exit_code));
        }
    }
    drop(lock);
    panic!("child_exit is posted, but no zombie child is found!");
}

// Create a new process.
// It will allocate stack and pid for `p`, and fill default fields.
// If the caller is a running process, it will also attach `p` to the caller.
pub unsafe fn init_proc(p: &mut Process) {
    let mut proc = &mut *p;
    proc.fill_default_fields();
    let stack_top = kalloc_page();
    proc.kernel_stack = stack_top.byte_add(PAGE_SIZE);
    proc.user_context = proc
        .kernel_stack
        .byte_sub(core::mem::size_of::<UserContext>()) as *mut UserContext;
    proc.kernel_context =
        proc.user_context
            .byte_sub(core::mem::size_of::<KernelContext>()) as *mut KernelContext;
    proc.pid = PID_POOL.alloc(pid_generator).unwrap();
    // Set up the proc tree, if the caller is a running process.
    if let Some(parent) = try_thisproc() {
        let _lock = PROC_LOCK.lock();
        parent.attach_child(proc);
    }
}

pub fn create_proc() -> *mut Process {
    let mut p: Box<Process> = Default::default();
    unsafe {
        init_proc(p.as_mut());
    }
    Box::leak(p)
}

// Start a process.
// It will set `p`'s state to runnable, and push it to the scheduler.
// If `p` still does not have a parent, it will be attached to the root process.
pub fn start_proc(p: &mut Process, entry: *const fn(usize), arg: usize) -> usize {
    if p.pid == 0 {
        panic!("cannot start IDLE process");
    }
    // If the process does not have a parent, its parent is the root process.
    let lock = PROC_LOCK.lock();
    if p.parent.is_none() {
        p.parent = Some(root_proc());
        // If `p` itself is not the root process, attach it to the root process.
        if p.pid != root_proc().pid {
            root_proc().attach_child(p);
        }
    }
    drop(lock);
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
    proc.state = ProcessState::Runnable;
    proc.idle = true;
    proc.kernel_stack = get_kernel_stack_bottom();
    proc
}

pub unsafe extern "C" fn init_root_process() {
    let root = root_proc();
    init_proc(root);
    root.parent = Some(root_proc());
    start_proc(root, kernel_entry as *const fn(usize), 123456);
}

define_init!(init_root_process);
