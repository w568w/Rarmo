use core::arch::global_asm;
use core::ptr;
use crate::aarch64::mmu::{kernel2physical, physical2kernel};
use crate::common::sem::Semaphore;
use crate::cores::virtual_memory::{PageTableDirectory, pte_flags, VirtualMemoryPageTable};
use crate::kernel::mem::{kalloc_page, kfree_page};
use crate::kernel::proc::{create_proc, kill, start_proc, wait};
use crate::println;

static mut DONE: Semaphore = Semaphore::uninit(0);

global_asm!(include_str!("user/loop.asm"));

extern "C" {
    fn loop_start();
    fn loop_end();
    fn trap_return(_: usize);
}

const BASE_ADDR: usize = 0x400000;

static mut P: [*mut u8; 100000] = [ptr::null_mut(); 100000];

#[test_case]
pub fn vm_test() {
    println!("vm_test");
    let mut pgdir = PageTableDirectory::new();
    unsafe {
        for i in 0..P.len() {
            P[i] = kalloc_page(1);
            let pte = pgdir.walk(i << 12, true).unwrap();
            (*pte).set_addr(kernel2physical(P[i] as u64) as usize, 3);
            pte_flags::user_page(unsafe { &mut *pte });
            (P[i] as *mut i32).write(i as i32);
        }
    }
    pgdir.attach();
    unsafe {
        for i in 0..P.len() {
            assert_eq!(((i << 12) as *const i32).read(), i as i32);
            let pte = pgdir.walk(i << 12, false).unwrap();
            let addr = physical2kernel((*pte).addr(3) as u64) as *mut i32;
            assert_eq!(addr.read(), i as i32);
        }
    }
    pgdir.free();
    pgdir.attach();
    for p in unsafe { P } {
        kfree_page(p, 1);
    }
    println!("vm_test PASS");
}

pub fn user_proc_test() {
    println!("user proc test");
    unsafe {
        DONE.init();
    }
    let mut pids = [0usize; 22];
    for i in 0..pids.len() {
        let proc = create_proc();
        let q = loop_start as usize;
        let p = loop_end as usize;
        while q < p {
            let pte = proc.pgdir.walk(BASE_ADDR + q - loop_start as usize, true).unwrap();
            unsafe {
                (*pte).set_addr(kernel2physical(q as u64) as usize, 3);
                // todo set flags
            }
        }
        unsafe {
            (*proc.user_context).x[0] = i as u64;
            (*proc.user_context).elr_el1 = BASE_ADDR as u64;
            (*proc.user_context).spsr_el1 = 0;
        }
        pids[i] = start_proc(proc, trap_return as *const fn(usize), 0);
        println!("pids[{}] = {}", i, pids[i]);
    }
    unsafe {
        assert!(DONE.get_or_wait());
    }
    println!("done");
    for pid in pids {
        assert!(kill(pid));
    }
    for _ in pids {
        let (pid, code) = wait().unwrap();
        println!("pid {} killed", pid);
        assert_eq!(code, -1);
    }
    println!("user proc test PASS");
}