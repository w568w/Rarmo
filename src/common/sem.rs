use spin::Mutex;
use crate::common::list::ListNode;

pub struct Semaphore {
    lock: Mutex<()>,
    value: usize,
    sleep_list: ListNode<WaitData>,
}

pub struct WaitData {
    slnode: ListNode<WaitData>,
    up: bool,
}

