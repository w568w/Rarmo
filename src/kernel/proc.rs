use crate::common::list::ListNode;

pub enum ProcessState {
    Unused,
    Runnable,
    Running,
    Sleeping,
    Zombie,
}
pub struct UserContext{

}

pub struct KernelContext{

}

pub struct Process{
    pub pid: usize,
    pub killed: bool,
    pub exit_code: usize,
    pub state: ProcessState,
    pub children: ListNode<Process>,
    pub ptnode: ListNode<Process>,
    pub parent: Option<*mut Process>,
    pub kernel_stack: *mut u8,
    pub user_context: *mut UserContext,
    pub kernel_context: *mut KernelContext,
}