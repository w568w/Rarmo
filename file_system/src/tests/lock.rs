use std::{thread::sleep, time::{self, Duration}};

use parking_lot::{Mutex, RawMutex};
use parking_lot::lock_api::RawMutex as RawMutexTrait;

pub struct Semaphore {
    lock: RawMutex,
    request: isize,
    response: isize,
}

impl Semaphore {
    pub const fn uninit(value: isize) -> Self {
        Self {
            lock: RawMutex::INIT,
            request: 0,
            response: value,
        }
    }
    pub fn init(&mut self) {}
    pub fn try_get(&mut self) -> bool {
        self.lock.lock();
        if self.request < self.response {
            self.request += 1;
            unsafe { self.lock.unlock(); }
            true
        } else {
            unsafe { self.lock.unlock(); }
            false
        }
    }

    pub fn get_or_wait(&mut self) -> bool {
        self.lock.lock();
        let t = self.request;
        self.request += 1;
        let start_time = time::SystemTime::now();
        loop {
            let now = time::SystemTime::now();
            let duration = now.duration_since(start_time).unwrap().as_secs();
            if duration > 3 {
                unsafe { self.lock.unlock(); }
                return false;
            }
            if self.response > t {
                break;
            }
            unsafe { self.lock.unlock(); }
            sleep(Duration::from_millis(10));
            self.lock.lock();
        }
        unsafe { self.lock.unlock(); }
        true
    }

    pub fn try_get_all(&mut self) -> isize {
        self.lock.lock();
        let val = self.response - self.request;
        if val > 0 {
            self.request = self.response;
            unsafe { self.lock.unlock(); }
            val
        } else {
            unsafe { self.lock.unlock(); }
            0
        }
    }
    pub fn post(&mut self) {
        self.lock.lock();
        self.response += 1;
        unsafe { self.lock.unlock(); }
    }
}