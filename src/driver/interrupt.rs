use core::ptr;
use crate::aarch64::intrinsic::addr::{LOCAL_BASE, MMIO_BASE};
use crate::aarch64::intrinsic::{get_u32, put_u32};
use crate::{define_early_init, get_cpu_id};
use crate::driver::clock::clock_handler;

#[derive(Clone, Copy)]
pub enum InterruptType {
    IRQ_AUX = 29,
    IRQ_GPIO0 = 49,
    IRQ_SDIO = 56,
    IRQ_ARASANSDIO = 62,
}

const NUM_IRQ_TYPES: usize = 64;

const IRQ_BASIC_PENDING: u64 = MMIO_BASE + 0xB200;
const IRQ_PENDING_1: u64 = MMIO_BASE + 0xB204;
const IRQ_PENDING_2: u64 = MMIO_BASE + 0xB208;
const FIQ_CONTROL: u64 = MMIO_BASE + 0xB20C;
const ENABLE_IRQS_1: u64 = MMIO_BASE + 0xB210;
const ENABLE_IRQS_2: u64 = MMIO_BASE + 0xB214;
const ENABLE_BASIC_IRQS: u64 = MMIO_BASE + 0xB218;
const DISABLE_IRQS_1: u64 = MMIO_BASE + 0xB21C;
const DISABLE_IRQS_2: u64 = MMIO_BASE + 0xB220;
const DISABLE_BASIC_IRQS: u64 = MMIO_BASE + 0xB224;
// ARM Local Peripherals
const GPU_INT_ROUTE: u64 = LOCAL_BASE + 0xC;
const IRQ_SRC_TIMER: u32 = 1 << 11; /* Local Timer */
const IRQ_SRC_GPU: u32 = 1 << 8;
const IRQ_SRC_CNTPNSIRQ: u32 = 1 << 1; /* Core Timer */

const fn irq_src_core(i: usize) -> u64 {
    LOCAL_BASE + 0x60 + 4 * (i as u64)
}

static mut IRQ_HANDLERS: [*const fn(); NUM_IRQ_TYPES] = [ptr::null(); NUM_IRQ_TYPES];

pub fn init_interrupt() {
    put_u32(GPU_INT_ROUTE, 0);
}
define_early_init!(init_interrupt);

pub fn set_interrupt_handler(typ: InterruptType, handler: *const fn()) {
    put_u32(ENABLE_IRQS_1 + ((typ as u64) / 32) * 4, 1 << ((typ as usize) % 32));
    unsafe {
        IRQ_HANDLERS[typ as usize] = handler;
    }
}

pub fn interrupt_global_handler() {
    let mut source = get_u32(irq_src_core(get_cpu_id()));
    if source & IRQ_SRC_CNTPNSIRQ != 0 {
        source ^= IRQ_SRC_CNTPNSIRQ;
        unsafe { clock_handler(); }
    }
    if source & IRQ_SRC_GPU != 0 {
        source ^= IRQ_SRC_GPU;
        let map: u64 = (get_u32(IRQ_PENDING_1) as u64) | ((get_u32(IRQ_PENDING_2) as u64) << 32);
        for i in 0..NUM_IRQ_TYPES {
            if (map >> i) & 1 != 0 {
                unsafe {
                    if IRQ_HANDLERS[i] != ptr::null() {
                        (*(IRQ_HANDLERS[i]))();
                    } else {
                        panic!("Unknown interrupt {}", i);
                    }
                }
            }
        }
    }

    if source != 0 {
        panic!("Unknown interrupt source {}", source);
    }
}