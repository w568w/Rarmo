#![feature(pointer_byte_offsets)]
#![feature(const_pointer_byte_offsets)]
#![feature(maybe_uninit_uninit_array)]
#![feature(const_maybe_uninit_uninit_array)]
#![feature(const_mut_refs)]
#![feature(assert_matches)]
#![feature(link_llvm_intrinsics)]
#![feature(cstr_from_bytes_until_nul)]

#![no_std]
#![no_main]

#![feature(custom_test_frameworks)]
#![test_runner(start_test)]
#![reexport_test_harness_main = "test_main"]

#![feature(default_alloc_error_handler)]
#![feature(is_some_with)]
extern crate alloc;

mod cores;
mod driver;
mod kernel;
mod aarch64;
mod common;
mod tests;

use core::arch::global_asm;
use core::panic::PanicInfo;
use core::sync::atomic::AtomicBool;
use spin::Mutex;
use driver::uart::UartDevice;
use aarch64::intrinsic::{get_cpu_id, stop_cpu};
use crate::aarch64::intrinsic::dsb_sy;
use crate::aarch64::trace::unwind_stack;
use crate::cores::console::CONSOLE;
use crate::driver::power::power_off;
use crate::kernel::cpu::{set_cpu_off, wait_all_cpu_off};
use crate::kernel::{idle_entry, PANIC_FLAG};

// This file is generated by Makefile from `entry.S`.
global_asm!(include_str!("entry.asm"));

static PANIC_LOCK: Mutex<()> = Mutex::new(());

#[cfg(test)]
fn run_test() {
    test_main();
}

#[cfg(test)]
fn start_test(tests: &[&dyn Fn()]) {
    println!("------------------------");
    println!("Running {} test(s)", tests.len());
    println!("------------------------");
    println!();
    for test in tests {
        println!("Running test at: {:p}", test);
        println!("------TEST START------");
        test();
        println!("------TEST END------");
    }
    power_off();
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    let lock = PANIC_LOCK.lock();
    // Force to unlock the write lock on console.
    unsafe { CONSOLE.force_write_unlock() };
    PANIC_FLAG.store(true, core::sync::atomic::Ordering::Relaxed);
    println!("\n\nKernel panic: {:?}", _info);
    unsafe { unwind_stack(); }
    drop(lock);
    set_cpu_off();
    wait_all_cpu_off();
    stop_cpu();
}


static KERNEL_INITED: AtomicBool = AtomicBool::new(false);

fn kernel_init() {
    clear_bss();
    kernel::init::do_early_init();
    kernel::init::do_init();
    KERNEL_INITED.store(true, core::sync::atomic::Ordering::Release);
}

#[no_mangle]
pub fn main() -> ! {
    // We will only use the first core.
    if get_cpu_id() == 0 {
        kernel_init();
    } else {
        while !KERNEL_INITED.load(core::sync::atomic::Ordering::Acquire) {}
        dsb_sy();
    }
    idle_entry();
}


// Clean up the BSS section with zero.
fn clear_bss() {
    // These two symbols are provided by the linker script.
    // See `linker.ld` for more information.
    extern "C" {
        fn sbss();
        fn ebss();
    }
    let start = sbss as usize;
    let end = ebss as usize;
    (start..end).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
}
