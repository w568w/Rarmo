use crate::aarch64::intrinsic::addr::{EMMC_BLKSIZECNT, EMMC_DATA, EMMC_INTERRUPT};
use crate::aarch64::intrinsic::{get_u32, put_u32};
use crate::common::list::{ListLink, ListNode};
use crate::common::sem::Semaphore;
use crate::driver::interrupt::{set_interrupt_handler, InterruptType};
use crate::kernel::sd_def::{sd_send_command_a, sd_wait_for_interrupt, INT_WRITE_RDY, IX_READ_SINGLE, IX_WRITE_SINGLE, SD_CARD, SD_TYPE_2_HC};
use crate::{define_rest_init, dsb_sy, println};
use field_offset::offset_of;
use spin::Mutex;

use super::mbr::MBR;
use super::sd_def::sd_init;

pub const B_VALID: u32 = 0x2; /* Buffer has been read from disk. */
pub const B_DIRTY: u32 = 0x4; /* Buffer needs to be written to disk. */
#[repr(C)]
pub struct Buffer {
    pub flags: u32,
    pub block_no: u32,
    pub data: [u8; 512],
    link: ListLink,
    sleep: Semaphore,
}

impl Buffer {
    pub const fn uninit(flags: u32, block_no: u32) -> Self {
        Self {
            flags,
            block_no,
            data: [0; 512],
            link: ListLink::uninit(),
            sleep: Semaphore::uninit(0),
        }
    }

    pub const fn read_uninit(block_no: u32) -> Self {
        Self::uninit(0, block_no)
    }

    pub const fn write_uninit(block_no: u32) -> Self {
        Self::uninit(B_DIRTY, block_no)
    }

    pub fn init(&mut self) {
        self.link.init();
        self.sleep.init();
    }
}

impl ListNode<ListLink> for Buffer {
    fn get_link_offset() -> usize {
        offset_of!(Buffer => link).get_byte_offset()
    }
}

static mut BUF_QUEUE: ListLink = ListLink::uninit();
static SD_LOCK: Mutex<()> = Mutex::new(());
/*
 * Initialize SD card and parse MBR.
 * 1. The first partition should be FAT and is used for booting.
 * 2. The second partition is used by our file system.
 *
 * See https://en.wikipedia.org/wiki/Master_boot_record
 */
pub fn init_sd() {
    // * 1.call sdInit.
    // * 2.Initialize the lock and request queue if any.
    // * 3.Read and parse 1st block (MBR) and collect whatever
    // * information you want.
    // * 4.set interrupt handler for IRQ_SDIO,IRQ_ARASANSDIO
    // *
    // * Hint:
    // * 1.Maybe need to use sd_start for reading, and
    // * sdWaitForInterrupt for clearing certain interrupt.
    // * 2.Remember to call init_sd() at somewhere.
    // * 3.the first number is 0.
    // * 4.don't forget to call this function somewhere
    // * TODO: Lab5 driver.
    let lock = SD_LOCK.lock();
    unsafe {
        BUF_QUEUE.init();
        sd_init().expect("sd init failed");
    }
    set_interrupt_handler(InterruptType::IRQ_SDIO, sd_interrupt_handler);
    set_interrupt_handler(InterruptType::IRQ_ARASANSDIO, sd_interrupt_handler);
    drop(lock);
    let mut buf = Buffer::read_uninit(0);
    buf.init();
    sd_rw(&mut buf);
    let mbr = MBR::parse(&buf.data);
    println!("MBR: {:?}", mbr);
}
define_rest_init!(init_sd);

fn sd_start(buf: &Buffer) {
    let block_no = if unsafe { SD_CARD.typ } == SD_TYPE_2_HC {
        buf.block_no
    } else {
        buf.block_no << 9
    };
    let write = buf.flags & B_DIRTY != 0;
    dsb_sy();
    if get_u32(EMMC_INTERRUPT) != 0 {
        panic!("sd_start: interrupt before start");
    }
    dsb_sy();
    let cmd_index = if write {
        IX_WRITE_SINGLE
    } else {
        IX_READ_SINGLE
    };
    put_u32(EMMC_BLKSIZECNT, 512);
    let resp = unsafe { sd_send_command_a(cmd_index, block_no) };
    if resp.is_err() {
        panic!("sd_start: sd_send_command_a failed");
    }

    let data_ptr = &buf.data as *const _ as usize;
    assert_eq!(
        data_ptr & 0x3,
        0,
        "sd_start: data_ptr is not 4-byte aligned"
    );

    if write {
        let resp = sd_wait_for_interrupt(INT_WRITE_RDY);
        if resp.is_err() {
            panic!("sd_start: sd_wait_for_interrupt timeout");
        }
        if get_u32(EMMC_INTERRUPT) != 0 {
            panic!("sd_start: interrupt before start");
        }
        let data = unsafe { &*(data_ptr as *const [u32; 512 / (32 / 8)]) };
        for qword in data {
            put_u32(EMMC_DATA, *qword);
        }
    }
}

pub fn sd_interrupt_handler() {
    /*
     * Pay attention to whether there is any element in the buflist.
     * Understand the meanings of EMMC_INTERRUPT, EMMC_DATA, INT_DATA_DONE,
     * INT_READ_RDY, B_DIRTY, B_VALID and some other flags.
     *
     * Notice that reading and writing are different, you can use flags
     * to identify.
     *
     * If B_DIRTY is set, write buf to disk, clear B_DIRTY, set B_VALID.
     * Else if B_VALID is not set, read buf from disk, set B_VALID.
     *
     * Remember to clear the flags after reading/writing.
     *
     * When finished, remember to use pop and check whether the list is
     * empty, if not, continue to read/write.
     *
     * You may use some buflist functions, arch_dsb_sy(), sd_start(), post_sem()
     * and sdWaitForInterrupt() to complete this function.
     *
     * TODO: Lab5 driver.
     */
    let _lock = SD_LOCK.lock();
    let buf = unsafe { BUF_QUEUE.prev::<Buffer>().unwrap() };
    let mut done = false;
    if (buf.flags & B_DIRTY) != 0 {
        done = true;
    } else if buf.flags & B_VALID == 0 {
        // Read data from EMMC
        let data_ptr = &mut buf.data as *mut _ as usize;
        let data = unsafe { &mut *(data_ptr as *mut [u32; 512 / (32 / 8)]) };
        for qword in data {
            *qword = get_u32(EMMC_DATA);
        }
        done = true;
    }
    if done {
        put_u32(EMMC_INTERRUPT, get_u32(EMMC_INTERRUPT));
        buf.flags &= !B_DIRTY;
        buf.flags |= B_VALID;
        buf.link.detach();
        buf.sleep.post();
    }
    if let Some(next) = unsafe { BUF_QUEUE.prev::<Buffer>() } {
        sd_start(next);
    }
}

pub fn sd_rw(buf: &mut Buffer) {
    // * 1.add buf to the queue
    //  * 2.if no buf in queue before,send request now
    //  * 3.'loop' until buf flag is modified
    //  *
    //  * You may use some buflist functions, arch_dsb_sy(),
    //  * sd_start(), wait_sem() to complete this function.
    //  *  TODO: Lab5 driver.
    let lock = SD_LOCK.lock();
    let single = unsafe { BUF_QUEUE.is_single() };
    unsafe {
        BUF_QUEUE.insert_at_first(buf);
    }
    if single {
        sd_start(buf);
    }
    drop(lock);
    buf.sleep.get_or_wait();
}