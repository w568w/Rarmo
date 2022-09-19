use core::sync::atomic::AtomicUsize;
use crate::{CONSOLE, dsb_sy, get_cpu_id};
use crate::aarch64::mmu::PAGE_SIZE;
use crate::kernel::mem::{ALLOC_PAGE_CNT, kfree};
use rand::prelude::*;
use crate::common::round_up;
use core::fmt::Write;
use rand::distributions::Uniform;

static mut p: [[*mut u8; 10000]; CPU_NUM] = [[0 as *mut u8; 10000]; CPU_NUM];
static mut sz: [[u8; 10000]; CPU_NUM] = [[0; 10000]; CPU_NUM];
static BARRIER: AtomicUsize = AtomicUsize::new(0);

const CPU_NUM: usize = 4;

const RAND_MAX: i32 = 32768;

#[inline(always)]
pub fn sync(stage: usize) {
    BARRIER.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    while BARRIER.load(core::sync::atomic::Ordering::Acquire) < CPU_NUM * stage {}
    dsb_sy();
}

pub unsafe fn alloc_test() {
    let mut rng = SmallRng::seed_from_u64(0xdeadbeef);
    let between = Uniform::from(0..RAND_MAX);
    let mut rand = || between.sample(&mut rng);

    let i = get_cpu_id();
    let r = ALLOC_PAGE_CNT.load(core::sync::atomic::Ordering::Relaxed);
    let y = 10000 - i * 500;

    if i == 0 {
        let mut binding = CONSOLE.write();
        let writer = binding.as_mut().unwrap();
        write!(writer, "alloc_test\n").unwrap();
        drop(binding);
    }
    sync(1);
    for j in 0..y {
        p[i][j] = crate::kernel::mem::kalloc_page();
        if p[i][j].is_null() || p[i][j] as usize & 1023 != 0 {
            panic!("kalloc_page failed");
        }
        p[i][j].write_bytes((i ^ j) as u8, PAGE_SIZE);
    }
    for j in 0..y {
        let m = ((i ^ j) & 255) as u8;
        for k in 0..PAGE_SIZE {
            if p[i][j].add(k).read() != m {
                panic!("page is wrong");
            }
        }
        crate::kernel::mem::kfree_page(p[i][j]);
    }
    sync(2);
    if ALLOC_PAGE_CNT.load(core::sync::atomic::Ordering::Relaxed) != r {
        panic!("ALLOC_PAGE_CNT changed")
    }

    sync(3);
    let mut j = 0;
    while j < 10000 {
        if j < 1000 || rand() > RAND_MAX / 16 * 7 {
            let mut z ;
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
    if i == 0 {
        let mut binding = CONSOLE.write();
        let writer = binding.as_mut().unwrap();
        write!(writer, "Usage: {}\n", ALLOC_PAGE_CNT.load(core::sync::atomic::Ordering::Relaxed) - r).unwrap();
        drop(binding);
    }
    sync(5);
    for j in 0..10000 {
        kfree(p[i][j]);
    }
    sync(6);
    if i == 0 {
        let mut binding = CONSOLE.write();
        let writer = binding.as_mut().unwrap();
        write!(writer, "alloc_test PASS\n").unwrap();
        drop(binding);
    }
}