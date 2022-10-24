#![feature(map_try_insert)]
#![feature(maybe_uninit_uninit_array)]
#![feature(pointer_byte_offsets)]

mod tests;
mod defines;
mod cache;
mod block_device;

pub trait Container<T> {
    fn get_child_ptr(&mut self) -> *mut T {
        let ptr = self as *mut Self as *mut T;
        unsafe { ptr.byte_add(Self::get_child_offset()) }
    }

    fn get_child(&mut self) -> &mut T {
        unsafe { &mut *self.get_child_ptr() }
    }

    fn get_parent_ptr_mut<T2: Container<T>>(this: *mut T) -> *mut T2 {
        let ptr = this as *mut T2;
        unsafe { ptr.byte_sub(T2::get_child_offset()) }
    }
    fn get_parent_ptr<T2: Container<T>>(this: *const T) -> *const T2 {
        let ptr = this as *const T2;
        unsafe { ptr.byte_sub(T2::get_child_offset()) }
    }
    fn get_parent_mut<T2: Container<T>>(this: *mut T) -> &'static mut T2 {
        unsafe { &mut *Self::get_parent_ptr_mut::<T2>(this) }
    }

    fn get_parent<T2: Container<T>>(this: *const T) -> &'static T2 {
        unsafe { &*Self::get_parent_ptr::<T2>(this) }
    }

    fn get_child_offset() -> usize;
}