pub mod sem;
pub mod list;
pub mod pool;
pub mod lock;

use core::ops::{Add, Rem, Sub};

pub fn round_down<T: Copy + Rem<Output = T> + Sub<Output = T>>(addr: T, align: T) -> T {
    addr - (addr % align)
}

pub fn round_up<T: Copy + Rem<Output = T> + Sub<Output = T> + Add<Output = T> + From<u8>>(
    addr: T,
    align: T,
) -> T {
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

pub trait Container<T> {
    fn get_child_ptr(&mut self) -> *mut T {
        let ptr = self as *mut Self as *mut T;
        unsafe { ptr.byte_add(Self::get_child_offset()) }
    }

    fn get_child(&mut self) -> &mut T {
        unsafe { &mut *self.get_child_ptr() }
    }

    fn get_parent_ptr<T2: Container<T>>(this:*mut T) -> *mut T2 {
        let ptr = this as *mut T2;
        unsafe { ptr.byte_sub(T2::get_child_offset()) }
    }

    fn get_parent<T2: Container<T>>(this: *mut T) -> &'static mut T2 {
        unsafe { &mut *Self::get_parent_ptr::<T2>(this) }
    }
    
    
    fn get_child_offset() -> usize;
}
