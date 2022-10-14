use crate::define_syscall;
use crate::kernel::proc::UserContext;

const MAX_SYSCALLS: usize = 256;
static mut SYSCALL_TABLE: [Option<fn([u64; 6]) -> u64>; MAX_SYSCALLS] = [None; MAX_SYSCALLS];

pub unsafe fn register_syscall(syscall: usize, func: fn([u64; 6]) -> u64) {
    if syscall >= MAX_SYSCALLS {
        panic!("Syscall number out of range");
    }
    if SYSCALL_TABLE[syscall].is_some() {
        panic!("Syscall number already registered");
    }
    SYSCALL_TABLE[syscall] = Some(func);
}

pub fn syscall_entry(context: *mut UserContext) {

    let syscall_id = unsafe { (*context).x[8] as usize };
    if syscall_id >= MAX_SYSCALLS {
        panic!("Invalid syscall id {}", syscall_id);
    }
    let syscall = unsafe { SYSCALL_TABLE[syscall_id] };
    if syscall.is_none() {
        panic!("Unimplemented syscall id {}", syscall_id);
    }
    let syscall = syscall.unwrap();
    // Get args from context
    let args: [u64; 6] = unsafe { (*context).x[0..6].try_into().unwrap() };
    let ret = syscall(args);
    unsafe { (*context).x[0] = ret };
}

pub fn hello_world(_args: [u64; 6]) -> u64 {
    0x114514
}
define_syscall!(0, hello_world);