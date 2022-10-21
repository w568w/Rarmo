use core::mem::MaybeUninit;
use crate::aarch64::intrinsic::{get_timer_freq, get_timestamp};
use crate::kernel::sd::{B_DIRTY, Buffer, sd_rw};
use crate::{dsb_sy, print, println};
use crate::kernel::proc::{create_proc, exit, start_proc, wait};

static mut BS: [Buffer; 1 << 11] = {
    let mut bs: [MaybeUninit<Buffer>; 1 << 11] = MaybeUninit::uninit_array();
    let mut i = 0;
    while i < 1 << 11 {
        bs[i] = MaybeUninit::new(Buffer::uninit(0, 0));
        i += 1;
    }
    unsafe { core::mem::transmute(bs) }
};


fn rw_proc(start_buf_index: usize) {
    // println!("rw_proc start: {}", start_buf_index);
    unsafe {
        for i in start_buf_index..(start_buf_index + (1 << 4)) {
            BS[i].init();
        }
        for i in (start_buf_index + 1)..(start_buf_index + (1 << 4)) {
            BS[start_buf_index].flags = 0;
            BS[start_buf_index].block_no = i as u32;
            sd_rw(&mut BS[start_buf_index]);
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

            BS[start_buf_index].flags = B_DIRTY;
            sd_rw(&mut BS[start_buf_index]);
        }
    }
    exit(0);
}

#[test_case]
pub fn sd_multi_process_rw() {
    println!("sd_multi_process_rw_test: start");
    for i in 0..(1 << 7) {
        let proc = create_proc();
        start_proc(proc, rw_proc as *const fn(usize), i * (1 << 4));
    }
    for _ in 0..(1 << 7) {
        let _ = wait().unwrap();
    }
    println!("sd_multi_process_rw_test: PASS");
}

#[test_case]
pub fn sd_test() {
    unsafe {
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
}