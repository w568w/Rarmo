use alloc::boxed::Box;
use core::mem::{MaybeUninit, size_of};
use crate::common::ipc::{AsMessageBuffer, IPC_CREATE, IPC_EXCL, IPC_RMID, sys_msgctl, sys_msgget, sys_msgrcv, sys_msgsend};
use crate::kernel::proc::{create_proc, exit, start_proc, wait};
use crate::println;

static mut MSG: [i32; 10001] = [0; 10001];

#[repr(C)]
struct Msg {
    mtype: i32,
    sum: i32,
}

impl AsMessageBuffer<Msg> for Msg {}

fn sender(start: usize) {
    let start = start as i32;
    let msg_id = sys_msgget(114514, 0);
    assert!(msg_id >= 0);
    for i in start..(start + 100) {
        let mut k = Box::new(Msg {
            mtype: i + 1,
            sum: -i - 1,
        });
        assert!(sys_msgsend(msg_id, k.as_message_buffer(), size_of::<Msg>() - size_of::<i32>(), 0) >= 0);
    }
    exit(0);
}

fn receiver(start: usize) {
    let start = start as i32;
    let msg_id = sys_msgget(114514, 0);
    assert!(msg_id >= 0);
    for _ in start..(start + 1000) {
        let mut k: MaybeUninit<Msg> = MaybeUninit::uninit();
        assert!(sys_msgrcv(msg_id, unsafe { k.assume_init_mut().as_message_buffer() }, size_of::<Msg>() - size_of::<i32>(), 0, 0) >= 0);
        unsafe { MSG[k.assume_init_mut().mtype as usize] = k.assume_init_mut().sum; }
    }
    exit(0);
}

#[test_case]
pub fn ipc_test() {
    println!("ipc test");
    let msg_id = sys_msgget(114514, IPC_CREATE | IPC_EXCL);
    for i in 0..100 {
        let proc = create_proc();
        start_proc(proc, sender as *const fn(usize), i * 100);
    }
    for i in 0..10 {
        let proc = create_proc();
        start_proc(proc, receiver as *const fn(usize), i * 1000);
    }
    while wait().is_some() {}

    assert!(sys_msgctl(msg_id, IPC_RMID) >= 0);
    for i in 1i32..10001 {
        assert_eq!(unsafe { MSG[i as usize] }, -i);
    }
    println!("ipc test PASS");
}