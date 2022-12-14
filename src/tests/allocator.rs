#![allow(non_upper_case_globals)]

use core::sync::atomic::AtomicUsize;
use crate::{dsb_sy, println};
use crate::aarch64::mmu::PAGE_SIZE;
use crate::kernel::mem::{ALLOC_PAGE_CNT, kfree};
use rand::prelude::*;
use crate::common::round_up;
use rand::distributions::Uniform;
use crate::aarch64::intrinsic::get_time_us;

static mut p: [[*mut u8; 10000]; CPU_NUM] = [[0 as *mut u8; 10000]; CPU_NUM];
static mut sz: [[u8; 10000]; CPU_NUM] = [[0; 10000]; CPU_NUM];
static BARRIER: AtomicUsize = AtomicUsize::new(0);

const CPU_NUM: usize = 1;

const RAND_MAX: i32 = 32768;

#[inline(always)]
pub fn sync(stage: usize) {
    BARRIER.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    while BARRIER.load(core::sync::atomic::Ordering::Acquire) < CPU_NUM * stage {}
    dsb_sy();
}

// #[test_case]
pub fn alloc_test() {
    unsafe {
        let mut rng = SmallRng::seed_from_u64(0xdeadbeef);
        let between = Uniform::from(0..RAND_MAX);
        let mut rand = || between.sample(&mut rng);

        let i = 0;
        let r = ALLOC_PAGE_CNT.load(core::sync::atomic::Ordering::Relaxed);
        let y = 1000 - i * 50;

        println!("alloc test start");
        sync(1);
        for j in 0..y {
            p[i][j] = crate::kernel::mem::kalloc_page(1);
            if p[i][j].is_null() || p[i][j] as usize & 1023 != 0 {
                panic!("kalloc_page failed");
            }
            p[i][j].write_bytes(((i ^ j) & 255) as u8, PAGE_SIZE);
        }
        for j in 0..y {
            let m = ((i ^ j) & 255) as u8;
            for k in 0..PAGE_SIZE {
                if p[i][j].byte_add(k).read() != m {
                    panic!("page is wrong");
                }
            }
            crate::kernel::mem::kfree_page(p[i][j], 1);
        }
        sync(2);
        if ALLOC_PAGE_CNT.load(core::sync::atomic::Ordering::Relaxed) != r {
            panic!("ALLOC_PAGE_CNT changed")
        }

        sync(3);
        let mut start = 0;
        start = get_time_us();

        let mut j = 0;
        while j < 1000 {
            if j < 100 || rand() > RAND_MAX / 16 * 7 {
                let mut z;
                let r = rand() & 255;
                if r < 127 { // [17,64]
                    z = rand() % 48 + 17;
                    z = round_up(z, 4);
                } else if r < 181 { // [1,16]
                    z = rand() % 16 + 1;
                } else if r < 235 { // [65,256]
                    z = rand() % 192 + 65;
                    z = round_up(z, 8);
                } else if r < 255 { // [257,512]
                    z = rand() % 256 + 257;
                    z = round_up(z, 8);
                } else { // [513,2040]
                    z = rand() % 1528 + 513;
                    z = round_up(z, 8);
                }
                sz[i][j] = z as u8;
                p[i][j] = crate::kernel::mem::kmalloc(z as usize);
                let q = p[i][j] as u64;
                if p[i][j].is_null() || ((z & 1) == 0 && (q & 1) != 0) ||
                    ((z & 3) == 0 && (q & 3) != 0) ||
                    ((z & 7) == 0 && (q & 7) != 0) {
                    panic!("kalloc failed");
                }
                p[i][j].write_bytes((i ^ (z as usize)) as u8, z as usize);
                j += 1;
            } else {
                let k = (rand() as usize) % j;
                if p[i][k].is_null() {
                    panic!("kfree failed, some block null");
                }
                let m = (i as u8) ^ sz[i][k];
                for t in 0..sz[i][k] as usize {
                    if p[i][k].add(t).read() != m {
                        panic!("kfree failed, some block wrong");
                    }
                }
                kfree(p[i][k]);
                j -= 1;
                p[i][k] = p[i][j];
                sz[i][k] = sz[i][j];
            }
        }

        sync(4);
        println!("Usage: {}, time: {} us", ALLOC_PAGE_CNT.load(core::sync::atomic::Ordering::Relaxed) - r, get_time_us() - start);

        sync(5);
        for j in 0..1000 {
            kfree(p[i][j]);
        }
        sync(6);
        println!("alloc test PASS");
    }
}