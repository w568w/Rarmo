use field_offset::offset_of;
use crate::aarch64::intrinsic::addr::{EMMC_BLKSIZECNT, EMMC_DATA, EMMC_INTERRUPT};
use crate::aarch64::intrinsic::{get_u32, put_u32};
use crate::common::list::{ListLink, ListNode};
use crate::common::sem::Semaphore;
use crate::dsb_sy;
use crate::kernel::sd_def::{INT_WRITE_RDY, IX_READ_SINGLE, IX_WRITE_SINGLE, SD_CARD, SD_OK, sd_send_command_a, SD_TYPE_2_HC, sd_wait_for_interrupt};

pub const B_VALID: u32 = 0x2; /* Buffer has been read from disk. */
pub const B_DIRTY: u32 = 0x4; /* Buffer needs to be written to disk. */
#[repr(C)]
pub struct Buffer {
    flags: u32,
    block_no: u32,
    data: [u8; 512],
    link: ListLink,
    sleep: Semaphore,
}

impl ListNode<ListLink> for Buffer {
    fn get_link_offset() -> usize { offset_of!(Buffer => link).get_byte_offset() }
}
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
    // * 2.Remember to call sd_init() at somewhere.
    // * 3.the first number is 0.
    // * 4.don't forget to call this function somewhere
    // * TODO: Lab5 driver.
}

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
    let cmd_index = if write { IX_WRITE_SINGLE } else { IX_READ_SINGLE };
    put_u32(EMMC_BLKSIZECNT, 512);
    let resp = unsafe { sd_send_command_a(cmd_index, block_no) };
    if resp != SD_OK {
        panic!("sd_start: sd_send_command_a failed");
    }

    let data_ptr = &buf.data as *const _ as u64;
    assert_eq!(data_ptr & 0x3, 0, "sd_start: data_ptr is not 4-byte aligned");

    if write {
        let resp = unsafe { sd_wait_for_interrupt(INT_WRITE_RDY) };
        if resp != SD_OK {
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
}

pub fn sd_rw(buf: *mut Buffer) {}

// TODO sd_test