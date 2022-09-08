#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::global_asm;
global_asm!(include_str!("entry.asm"));

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub fn main() -> ! {
    clear_bss();

    loop {}
}

// Clean up the BSS section with zero.
fn clear_bss() {
    // These two symbols are provided by the linker script.
    // See `linker.ld` for more information.
    extern "C" {
        fn sbss();
        fn ebss();
    }
    (sbss as usize..ebss as usize).for_each(|a| {
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}