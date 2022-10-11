#![allow(dead_code)]

use crate::aarch64::intrinsic::*;
use crate::driver::interrupt::interrupt_global_handler;
use crate::kernel::proc::UserContext;
use crate::kernel::sched::thisproc;
use core::arch::global_asm;

const ESR_EC_SHIFT: i8 = 26;
const ESR_ISS_MASK: u64 = 0xFFFFFF;
const ESR_IR_MASK: u64 = 1 << 25;

const ESR_EC_UNKNOWN: u64 = 0x00;
const ESR_EC_SVC64: u64 = 0x15;
const ESR_EC_IABORT_EL0: u64 = 0x20;
const ESR_EC_IABORT_EL1: u64 = 0x21;
const ESR_EC_DABORT_EL0: u64 = 0x24;
const ESR_EC_DABORT_EL1: u64 = 0x25;

global_asm!(include_str!("trap.asm"));
global_asm!(include_str!("exception_vector.asm"));

#[no_mangle]
pub extern "C" fn trap_global_handler(context: *mut UserContext) {
    thisproc().user_context = context;
    let context = unsafe { &mut *context };
    let esr = get_esr_el1();
    let exception_class = esr >> ESR_EC_SHIFT;
    let ir = esr & ESR_IR_MASK;

    reset_esr_el1();
    match exception_class {
        ESR_EC_UNKNOWN => {
            if ir != 0 {
                panic!("Unknown exception class: {:x}, at {:x}", esr, context.elr_el1);
            } else {
                interrupt_global_handler();
            }
        }
        ESR_EC_SVC64 => {
            panic!("SVC64 exception");
        }
        ESR_EC_IABORT_EL0 => {
            panic!("IABORT_EL0 exception, at {:x}", context.elr_el1);
        }
        ESR_EC_IABORT_EL1 => {
            panic!("IABORT_EL1 exception, at {:x}", context.elr_el1);
        }
        ESR_EC_DABORT_EL0 => {
            panic!("DABORT_EL0 exception, at {:x}", context.elr_el1);
        }
        ESR_EC_DABORT_EL1 => {
            panic!("DABORT_EL1 exception, at {:x}", context.elr_el1);
        }
        _ => {
            panic!("Unknown exception");
        }
    }
}

#[no_mangle]
pub extern "C" fn trap_error_handler(typ: u64) -> ! {
    panic!("trap error: {}", typ);
}
