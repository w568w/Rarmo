use crate::kernel::proc::UserContext;

const MAX_SYSCALLS: usize = 256;
static SYSCALL_TABLE: [Option<fn() -> u64>; MAX_SYSCALLS] = [None; MAX_SYSCALLS];

pub fn syscall_entry(context: *mut UserContext) {
    // todo pass args

    let syscall_id = unsafe { (*context).x[8] as usize };
    if syscall_id >= MAX_SYSCALLS {
        panic!("Invalid syscall id {}", syscall_id);
    }
    let syscall = SYSCALL_TABLE[syscall_id];
    if syscall.is_none() {
        panic!("Unimplemented syscall id {}", syscall_id);
    }
    let syscall = syscall.unwrap();
    let ret = syscall();
    unsafe { (*context).x[0] = ret };
}