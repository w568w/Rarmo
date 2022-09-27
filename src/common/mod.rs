pub mod sem;
pub mod list;
mod pool;

use core::ops::{Add, Rem, Sub};

pub fn round_down<T: Copy + Rem<Output=T> + Sub<Output=T>>(addr: T, align: T) -> T {
    addr - (addr % align)
}

pub fn round_up<T: Copy + Rem<Output=T> + Sub<Output=T> + Add<Output=T> + From<u8>>(addr: T, align: T) -> T {
    round_down(addr + align - T::from(1), align)
}
#[allow(dead_code)]
pub const fn padding(size: usize, align: usize) -> usize {
    if size % align == 0 {
        0
    } else {
        align - size % align
    }
}