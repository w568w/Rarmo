use core::arch::asm;
use core::mem;
use core::ptr::read_volatile;
use crate::println;

#[inline(never)]
pub unsafe fn unwind_stack() {
    let mut fp: usize;
    asm!("mov {}, fp", out(reg) fp);
    println!("trace: {:x}", fp);
    for _frame in 0..64 {
        if let Some(pc_fp) = fp.checked_add(mem::size_of::<usize>()) {
            let pc = read_volatile(pc_fp as *const usize);
            if pc == 0 {
                println!("{:x}: NO RETURN", fp);
                break;
            }
            println!("{:x}: pc: {:x}", fp, pc);
            fp = read_volatile(fp as *const usize);
        } else {
            println!("{:x}: OVERFLOW", fp);
            break;
        }
    }
}