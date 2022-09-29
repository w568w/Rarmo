use core::mem::MaybeUninit;
use crate::common::sem::Semaphore;
use crate::kernel::proc::{create_proc, exit, start_proc, wait};
use crate::kernel::sched::yield_;
use crate::println;

static mut Semaphores: [MaybeUninit<Semaphore>; 6] = MaybeUninit::uninit_array();

unsafe fn proc_test_1b(a: usize) {
    match a / 10 - 1 {
        1 => {
            yield_();
            yield_();
            yield_();
        }
        2 => {
            Semaphores[0].assume_init_mut().post();
        }
        3..=7 => {
            if a & 1 != 0 {
                Semaphores[1].assume_init_mut().post();
            } else {
                Semaphores[1].assume_init_mut().get_or_wait();
            }
        }
        8 => {
            Semaphores[2].assume_init_mut().get_or_wait();
            Semaphores[3].assume_init_mut().post();
        }
        9 => {
            Semaphores[4].assume_init_mut().post();
            Semaphores[5].assume_init_mut().get_or_wait();
        }
        _ => {}
    }
    exit(a);
}

unsafe fn proc_test_1a(a: usize) {
    for i in 0..10 {
        let p = create_proc();
        start_proc(p, proc_test_1b as *const fn(usize), a * 10 + i + 10);
    }
    match a {
        0 => {
            let mut t = 0;
            for _ in 0..10 {
                let (_, ret) = wait().unwrap();
                t |= 1 << (ret - 10);
            }
            assert_eq!(t, 1023);
            assert!(matches!(wait(),None));
        }
        2 => {
            for _ in 0..10 {
                assert!(Semaphores[0].assume_init_mut().get_or_wait());
            }
            assert!(!Semaphores[0].assume_init_mut().try_get());
        }
        3..=7 => {
            for _ in 0..10 {
                let _ = wait();
            }
            assert!(matches!(wait(),None));
        }
        8 => {
            for _ in 0..10 {
                Semaphores[2].assume_init_mut().post();
            }
            for _ in 0..10 {
                let _ = wait();
            }
            assert!(matches!(wait(),None));
            assert_eq!(Semaphores[2].assume_init_mut().value, 0);
            assert_eq!(Semaphores[3].assume_init_mut().try_get_all(), 10);
        }
        9 => {
            for _ in 0..10 {
                Semaphores[4].assume_init_mut().get_or_wait();
            }
            for _ in 0..10 {
                Semaphores[5].assume_init_mut().post();
            }
            for _ in 0..10 {
                let _ = wait();
            }
            assert!(matches!(wait(),None));
            assert_eq!(Semaphores[4].assume_init_mut().value, 0);
            assert_eq!(Semaphores[5].assume_init_mut().value, 0);
        }
        _ => {}
    }
    exit(a);
}

unsafe fn proc_test_1(_: usize) {
    println!("proc_test_1");
    for i in 0..6 {
        Semaphores[i] = MaybeUninit::new(Semaphore::uninit(0));
        Semaphores[i].assume_init_mut().init();
    }
    let mut pid: [usize; 10] = [0; 10];
    for i in 0..10 {
        let p = create_proc();
        pid[i] = start_proc(p, proc_test_1a as *const fn(usize), i);
    }
    for i in 0..10 {
        let (id, ret) = wait().unwrap();
        assert_eq!(id, pid[ret]);
        println!("proc_test_1: proc {} exit", ret);
    }
    exit(0);
}

pub fn proc_test() {
    println!("proc_test");
    let p = create_proc();
    let pid = start_proc(p, proc_test_1 as *const fn(usize), 0);
    let mut t = 0;
    loop {
        let ret = wait();
        if ret.is_none() {
            break;
        }
        let (id, code) = ret.unwrap();
        if id == pid {
            assert_eq!(code, 0);
        } else {
            t |= 1 << (code - 20);
        }
    }
    assert_eq!(t, 1048575);
    println!("proc_test: pass");
}