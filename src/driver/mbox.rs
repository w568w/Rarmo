use aligned::{A16, Aligned};
use crate::aarch64::intrinsic::dc_civac;
use crate::aarch64::intrinsic::mbox::*;
use crate::aarch64::mmu::kernel2physical;
use crate::dsb_sy;

pub fn get_arm_memory() -> u32 {
    let mut buf: Aligned<A16, [u32; 8]> = Aligned([0; 8]);
    buf[0] = 36;
    buf[1] = MBOX_REQUEST;
    buf[2] = MBOX_TAG_GET_CLOCK_RATE;
    buf[3] = 8;
    buf[7] = MBOX_TAG_LAST;
    dsb_sy();
    dc_civac(&buf);
    dsb_sy();
    write(kernel2physical(&buf as *const _ as u64) as u32, MBOX_CH_PROP);
    dsb_sy();
    read(MBOX_CH_PROP);
    dsb_sy();
    dc_civac(&buf);
    dsb_sy();

    assert_eq!(buf[5], 0);
    assert_ne!(buf[6], 0);

    buf[6]
}

pub fn get_clock_rate() -> u32 {
    let mut buf: Aligned<A16, [u32; 8]> = Aligned([0; 8]);
    buf[0] = 36;
    buf[1] = MBOX_REQUEST;
    buf[2] = MBOX_TAG_GET_CLOCK_RATE;
    buf[3] = 8;
    buf[5] = 1;
    buf[7] = MBOX_TAG_LAST;
    dsb_sy();
    dc_civac(&buf);
    dsb_sy();
    write(kernel2physical(&buf as *const _ as u64) as u32, MBOX_CH_PROP);
    dsb_sy();
    read(MBOX_CH_PROP);
    dsb_sy();
    dc_civac(&buf);
    dsb_sy();

    buf[6]
}