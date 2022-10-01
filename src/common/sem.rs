use alloc::boxed::Box;
use field_offset::offset_of;
use spin::Mutex;
use crate::common::list::{ListLink, ListNode};
use crate::kernel::proc::Process;
use crate::kernel::proc::ProcessState::Sleeping;
use crate::kernel::sched::{acquire_sched_lock, activate, sched, thisproc};

pub struct Semaphore {
    lock: Mutex<()>,
    pub value: isize,
    sleep_list: ListLink,
}

#[repr(C)]
pub struct WaitData {
    sibling: ListLink,
    up: bool,
    proc: *mut Process,
}

impl ListNode<ListLink> for WaitData {
    fn get_link_offset() -> usize {
        offset_of!(WaitData => sibling).get_byte_offset()
    }
}

impl Semaphore {
    pub fn uninit(value: isize) -> Self {
        Self {
            lock: Mutex::new(()),
            value,
            sleep_list: ListLink::uninit(),
        }
    }
    pub fn init(&mut self) {
        self.sleep_list.init();
    }
    pub fn try_get(&mut self) -> bool {
        let _lock = self.lock.lock();
        if self.value > 0 {
            self.value -= 1;
            true
        } else {
            false
        }
    }

    pub fn get_or_wait(&mut self) -> bool {
        let lock = self.lock.lock();
        self.value -= 1;
        if self.value >= 0 {
            // We have enough resources, so we can return immediately!
            return true;
        }
        // Create a WaitData, representing that the current process is in the wait list.
        let mut wait_data = Box::new(WaitData::uninit());
        wait_data.sibling.init();
        self.sleep_list.insert_at_first(wait_data.as_mut());
        // Lock for the scheduler, and tell it that the process is going to sleep.
        let sched_lock = acquire_sched_lock();
        drop(lock);
        sched(sched_lock, Sleeping);

        // Now back from the scheduler...
        // ... lock self again, since we are going to modify this semaphore.
        let _lock = self.lock.lock();
        // Are we woken up by other processes?
        if !wait_data.up {
            self.value += 1;
            assert!(self.value <= 0);
            wait_data.link().detach();
        }
        wait_data.up
    }

    pub fn try_get_all(&mut self) -> isize {
        let _lock = self.lock.lock();
        let val = self.value;
        if self.value > 0 {
            self.value = 0;
            val
        } else {
            0
        }
    }
    pub fn post(&mut self) {
        let _lock = self.lock.lock();
        self.value += 1;
        if self.value <= 0 {
            assert!(!self.sleep_list.is_single());
            // If there is someone waiting, wake up the last one.
            let wait: &mut WaitData = self.sleep_list.prev().unwrap();
            wait.up = true;
            wait.link().detach();
            activate(unsafe { &mut *(wait.proc) });
        }
    }
}

impl WaitData {
    pub fn uninit() -> Self {
        Self {
            sibling: ListLink::uninit(),
            up: false,
            proc: thisproc(),
        }
    }
}