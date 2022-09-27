use core::mem::MaybeUninit;
use spin::RwLock;

pub struct LockedArrayPool<T: Copy, const len: usize> (RwLock<ArrayPool<T, len>>);

pub struct ArrayPool<T: Copy, const len: usize> {
    array: [MaybeUninit<T>; len],
    tail: usize,
}

impl<T: Copy, const len: usize> ArrayPool<T, len> {
    pub const fn new() -> Self {
        Self {
            array: [MaybeUninit::uninit(); len],
            tail: 0,
        }
    }

    pub fn fill_with(&mut self, generator: fn(usize) -> T) {
        for i in 0..len {
            self.array[i] = MaybeUninit::new(generator(i));
        }
    }

    pub fn alloc(&mut self) -> Option<T> {
        if self.tail == 0 {
            None
        } else {
            self.tail -= 1;
            let ret = &mut self.array[self.tail];
            Some(unsafe { ret.assume_init() })
        }
    }
    pub fn free(&mut self, val: T) {
        if self.tail == len {
            panic!("ArrayPool is full");
        }
        self.array[self.tail] = MaybeUninit::new(val);
        self.tail += 1;
    }
}

impl <T: Copy, const len: usize> LockedArrayPool<T, len> {
    pub const fn new() -> Self {
        Self(RwLock::new(ArrayPool::new()))
    }

    pub fn fill_with(&self, generator: fn(usize) -> T) {
        self.0.write().fill_with(generator);
    }

    pub fn alloc(&self) -> Option<T> {
        self.0.write().alloc()
    }
    pub fn free(&self, val: T) {
        self.0.write().free(val);
    }
}