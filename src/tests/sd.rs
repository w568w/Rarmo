use core::mem::MaybeUninit;
use crate::aarch64::intrinsic::{get_timer_freq, get_timestamp};
use crate::kernel::sd::{B_DIRTY, Buffer, sd_rw};
use crate::{dsb_sy, print, println};

static mut BS: [Buffer; 1 << 11] = {
    let mut bs: [MaybeUninit<Buffer>; 1 << 11] = MaybeUninit::uninit_array();
    let mut i = 0;
    while i < 1 << 11 {
        bs[i] = MaybeUninit::new(Buffer::uninit(0, 0));
        i += 1;
    }
    unsafe { core::mem::transmute(bs) }
};

pub unsafe fn sd_test() {
    for b in BS.iter_mut() {
        b.init();
    }
    let size_in_mb = (BS.len() * BS[0].data.len()) >> 20;
    println!("sd_test: {} MB", size_in_mb);
    let freq = get_timer_freq();

    print!("sd_test: checking rw...  ");
    for i in 1..BS.len() {
        BS[0].flags = 0;
        BS[0].block_no = i as u32;
        sd_rw(&mut BS[0]);

        BS[i].flags = B_DIRTY;
        BS[i].block_no = i as u32;
        for (j, d) in BS[i].data.iter_mut().enumerate() {
            *d = ((i * j) & 0xff) as u8;
        }
        sd_rw(&mut BS[i]);

        BS[i].data.fill(0);
        BS[i].flags = 0;
        sd_rw(&mut BS[i]);
        for (j, d) in BS[i].data.iter().enumerate() {
            assert_eq!(*d, ((i * j) & 0xff) as u8);
        }

        BS[0].flags = B_DIRTY;
        sd_rw(&mut BS[0]);
    }
    println!("OK");

    print!("sd_test: Sequential read... ");
    dsb_sy();
    let t = get_timestamp();
    dsb_sy();
    for (i, b) in BS.iter_mut().enumerate() {
        b.flags = 0;
        b.block_no = i as u32;
        sd_rw(b);
    }
    dsb_sy();
    let t = get_timestamp() - t;
    dsb_sy();
    let speed: f64 = ((size_in_mb as u64) * freq) as f64;
    println!("{} MB/s", speed / (t as f64));

    print!("sd_test: Sequential write... ");
    dsb_sy();
    let t = get_timestamp();
    dsb_sy();
    for (i, b) in BS.iter_mut().enumerate() {
        b.flags = B_DIRTY;
        b.block_no = i as u32;
        sd_rw(b);
    }
    dsb_sy();
    let t = get_timestamp() - t;
    dsb_sy();
    let speed: f64 = ((size_in_mb as u64) * freq) as f64;
    println!("{} MB/s", speed / (t as f64));
}