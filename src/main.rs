#![no_std]
#![no_main]

use core::panic::PanicInfo;
use core::arch::global_asm;
global_asm!(include_str!("entry.S"));

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}