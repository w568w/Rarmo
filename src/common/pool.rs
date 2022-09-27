use core::mem::MaybeUninit;
use spin::RwLock;

pub struct LockedArrayPool<T: Copy, const len: usize> (RwLock<ArrayPool<T, len>>);

pub struct ArrayPool<T: Copy, const len: usize> {
    array: [MaybeUninit<T>; len],
    tail: usize,
    fill_count: usize,
}

impl<T: Copy, const len: usize> ArrayPool<T, len> {
    pub const fn new() -> Self {
        Self {
            array: [MaybeUninit::uninit(); len],
            tail: 0,
            fill_count: 0,
        }
    }

    fn fill_with(&mut self, generator: fn(usize, usize) -> T) {
        for i in 0..len {
            self.array[i] = MaybeUninit::new(generator(self.fill_count, i));
        }
        self.tail = len;
        self.fill_count += 1;
    }

    pub fn alloc(&mut self, generator: fn(usize, usize) -> T) -> Option<T> {
        if self.tail == 0 {
            self.fill_with(generator);
        }
        self.tail -= 1;
        let ret = &mut self.array[self.tail];
        Some(unsafe { ret.assume_init() })
    }
    pub fn free(&mut self, val: T) {
        if self.tail < len {
            self.array[self.tail] = MaybeUninit::new(val);
            self.tail += 1;
        }
    }
}

impl<T: Copy, const len: usize> LockedArrayPool<T, len> {
    pub const fn new() -> Self {
        Self(RwLock::new(ArrayPool::new()))
    }

    pub fn alloc(&self, generator: fn(usize, usize) -> T) -> Option<T> {
        self.0.write().alloc(generator)
    }
    pub fn free(&self, val: T) {
        self.0.write().free(val)
    }
}