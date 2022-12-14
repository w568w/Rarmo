use core::sync::atomic::AtomicBool;
use crate::aarch64::intrinsic::{disable_trap, enable_trap, wfi};
use crate::kernel::cpu::set_cpu_on;
use crate::kernel::sched::yield_;
use crate::{get_cpu_id, set_cpu_off, stop_cpu};
use crate::kernel::init::do_rest_init;

pub mod init;
pub mod mem;
pub mod rust_allocator;
pub mod proc;
pub mod cpu;
pub mod sched;
pub mod syscall;
pub mod sd_def;
pub mod sd;
pub mod mbr;

pub static PANIC_FLAG: AtomicBool = AtomicBool::new(false);
pub const KERNEL_STACK_SIZE: usize = 65536;

#[macro_export]
macro_rules! define_early_init {
    ($func:ident) => {
        // This is a hack to get the linker to include the function pointer in the
        // early init section.
        //
        // Also notice that we use `paste!` to generate a unique name for the
        // function pointer.
        // `pub` is required to tell the compiler not to optimize the variable away.
        // You can use `#[used]` attribute to achieve the same effect too.
        paste::paste! {
            #[link_section = ".init.early"]
            #[no_mangle]
            pub static mut [<__early_init_ $func>] : *const () = $func as *const ();
        }
    };
}

#[macro_export]
macro_rules! define_init {
    ($func:ident) => {
        paste::paste! {
            #[link_section = ".init"]
            #[no_mangle]
            pub static mut [<__init_ $func>] : *const () = $func as *const ();
        }
    };
}

#[macro_export]
macro_rules! define_rest_init {
    ($func:ident) => {
        paste::paste! {
            #[link_section = ".init.rest"]
            #[no_mangle]
            pub static mut [<__rest_init_ $func>] : *const () = $func as *const ();
        }
    };
}

#[macro_export]
macro_rules! define_syscall {
    ($syscall_id:expr, $func:ident) => {
        paste::paste! {
            pub unsafe extern "C" fn [<__syscall_ $func>] () {
                $crate::kernel::syscall::register_syscall($syscall_id, $func);
            }
            $crate::define_early_init!([<__syscall_ $func>]);
        }
    };
}

extern "C" {
    fn boot_stack_top();
}

pub fn get_kernel_stack_bottom() -> *mut u8 {
    let stack_bottom = boot_stack_top as usize;
    (stack_bottom - get_cpu_id() * KERNEL_STACK_SIZE) as *mut u8
}

pub fn idle_entry() -> ! {
    set_cpu_on();
    loop {
        yield_();
        if PANIC_FLAG.load(core::sync::atomic::Ordering::Relaxed) {
            break;
        }
        enable_trap();
        wfi();
        disable_trap();
    }
    set_cpu_off();
    stop_cpu();
}

pub fn kernel_entry(_arg: usize) -> ! {
    do_rest_init();
    #[cfg(test)]
    {
        use crate::run_test;
        run_test();
    }
    loop {
        yield_();
    }
}