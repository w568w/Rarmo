pub mod sem;
pub mod list;
pub mod pool;
pub mod lock;
pub mod tree;
pub mod bitmap;
pub mod ipc;

use core::ops::{Add, Rem, Sub};

pub fn round_down<T: Copy + Rem<Output=T> + Sub<Output=T>>(addr: T, align: T) -> T {
    addr - (addr % align)
}

pub fn round_up<T: Copy + Rem<Output=T> + Sub<Output=T> + Add<Output=T> + From<u8>>(
    addr: T,
    align: T,
) -> T {
    round_down(addr + align - T::from(1), align)
}

const fn simple_shl(x: u64, bits: u8) -> u64 {
    if bits == 64 {
        0
    } else {
        x << bits
    }
}

pub const fn set_bits(num: &mut u64, mut bits: u64, start: u8, end: u8) {
    assert!(start <= end);
    if start == end {
        return;
    }
    let len = end - start;
    let mask = simple_shl(1, len).wrapping_sub(1u64);
    bits &= mask;
    let mask = simple_shl(1, end).wrapping_sub(simple_shl(1, start));
    *num &= !mask;
    *num |= bits << start;
}

pub const fn get_bits(num: u64, start: u8, end: u8) -> u64 {
    assert!(start <= end);
    if start == end {
        return 0;
    }
    let len = end - start;
    let mask = simple_shl(1, len).wrapping_sub(1u64);
    (num >> start) & mask
}

#[allow(dead_code)]
pub const fn padding(size: usize, align: usize) -> usize {
    if size % align == 0 {
        0
    } else {
        align - size % align
    }
}

pub trait Container<T> {
    fn get_child_ptr(&mut self) -> *mut T {
        let ptr = self as *mut Self as *mut T;
        unsafe { ptr.byte_add(Self::get_child_offset()) }
    }

    fn get_child(&mut self) -> &mut T {
        unsafe { &mut *self.get_child_ptr() }
    }

    fn get_parent_ptr<T2: Container<T>>(this: *mut T) -> *mut T2 {
        let ptr = this as *mut T2;
        unsafe { ptr.byte_sub(T2::get_child_offset()) }
    }

    fn get_parent<T2: Container<T>>(this: *mut T) -> &'static mut T2 {
        unsafe { &mut *Self::get_parent_ptr::<T2>(this) }
    }


    fn get_child_offset() -> usize;
}

#[repr(transparent)]
pub struct StaticSafe<T>(pub T);

unsafe impl<T> Sync for StaticSafe<T> {}