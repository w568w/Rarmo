use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicUsize, Ordering};
use crate::{dsb_sy, get_cpu_id};

pub struct CPUReentrantMutex<T> {
    data: UnsafeCell<T>,
    owner: AtomicUsize,
    count: AtomicUsize,
}

impl<T> CPUReentrantMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            owner: AtomicUsize::new(0),
            count: AtomicUsize::new(0),
        }
    }

    pub fn lock(&self) -> CPUReentrantMutexGuard<T> {
        if self.count.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
            self.owner.store(get_cpu_id(), Ordering::Release);
        } else if self.owner.load(Ordering::Acquire) == get_cpu_id() {
            self.count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.spin_for_lock();
        }
        CPUReentrantMutexGuard {
            lock: &self.count,
            data: unsafe { &mut *self.data.get() },
        }
    }
    pub fn force_unlock(&self) {
        self.count.store(0, Ordering::Release);
    }

    fn spin_for_lock(&self) {
        loop {
            dsb_sy();
            if self.count.compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed).is_ok() {
                self.owner.store(get_cpu_id(), Ordering::Release);
                break;
            }
        }
    }
}

pub struct CPUReentrantMutexGuard<'a, T: 'a> {
    lock: &'a AtomicUsize,
    data: &'a mut T,
}

impl<'a, T:  'a> Drop for CPUReentrantMutexGuard<'a, T> {
    fn drop(&mut self) {
        if self.lock.load(Ordering::SeqCst) > 0 {
            self.lock.fetch_sub(1, Ordering::Release);
        }
    }
}

impl<'a, T: 'a> Deref for CPUReentrantMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: 'a> DerefMut for CPUReentrantMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}