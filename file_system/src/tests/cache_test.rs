use std::{ptr, thread};
use std::sync::atomic::Ordering::SeqCst;
use prng_mt::MT19937;
use crate::cache::{Block, BlockCache, EVICTION_THRESHOLD, OP_MAX_NUM_BLOCKS, OpContext, SCACHE};

use crate::defines::{SuperBlock, BLOCK_SIZE};
use crate::tests::block_device::initialize_mock;

use super::{
    block_device::{initialize, mock, sblock, MockBlockDevice},
};

pub unsafe fn mock_() -> &'static mut MockBlockDevice {
    mock.assume_init_mut()
}

pub unsafe fn sblock_() -> &'static mut SuperBlock {
    sblock.assume_init_mut()
}

unsafe fn test_init() {
    initialize(1, 1, "");
}

unsafe fn test_read_write() {
    initialize(1, 1, "");

    let b = SCACHE.acquire(1);
    let d = mock_().inspect(1);

    assert_eq!((*b).block_no, 1);
    assert!((*b).valid);

    for i in 0..BLOCK_SIZE {
        assert_eq!((*b).data[i], d.add(i).read());
    }

    let value = (*b).data[128];
    (*b).data[128] = !value;
    SCACHE.sync(ptr::null_mut(), b);
    assert_eq!(d.add(128).read(), !value);

    SCACHE.release(b);
    let _ = SCACHE.acquire(1);
}

unsafe fn test_loop_read() {
    let num_rounds = 10;
    initialize(1, 128, "");
    for round in 0..num_rounds {
        let mut p: Vec<*mut Block> = vec![];
        for i in 0..(sblock_().num_blocks as usize) {
            p.push(SCACHE.acquire(i));
            assert_eq!((*p[i]).block_no, i);

            let d = mock_().inspect(i);
            for j in 0..BLOCK_SIZE {
                assert_eq!((*p[i]).data[j], d.add(j).read());
            }
        }
        for i in 0..(sblock_().num_blocks as usize) {
            assert!((*p[i]).valid);
            SCACHE.release(p[i]);
        }
    }
}

static mut rcnt: usize = 0;
static mut wcnt: usize = 0;
const BLOCKS: [usize; 5] = [1, 123, 233, 399, 415];
fn matched(block_no: usize) -> bool {
    for i in 0..BLOCKS.len() {
        if BLOCKS[i] == block_no {
            return true;
        }
    }
    false
}
unsafe fn test_reuse() {
    initialize(1, 500, "");
    let num_rounds = 200;
    let blocks = [1, 123, 233, 399, 415];

    mock_().on_read = Some(Box::new(|bno, _| {
        if matched(bno) {
            rcnt += 1;
        }
    }));
    mock_().on_write = Some(Box::new(|bno, _| {
        if matched(bno) {
            wcnt += 1;
        }
    }));

    for round in 0..num_rounds {
        let mut p: Vec<*mut Block> = vec![];
        for i in 0..blocks.len() {
            p.push(SCACHE.acquire(blocks[i]));
        }
        for i in 0..blocks.len() {
            assert!((*p[i]).valid);
            SCACHE.release(p[i]);
        }
    }
    assert!(rcnt <= num_rounds);
    assert_eq!(wcnt, 0);
}

unsafe fn test_lru() {
    let mut gen = MT19937::new(0xdeadbeef);

    let cold_size = 1000;
    let hot_size = EVICTION_THRESHOLD * 4 / 5;
    initialize(1, cold_size + hot_size, "");

    for i in 0..1000 {
        let hot = (gen.next() % 100) <= 90;
        let bno = if hot {
            gen.next() as usize % hot_size
        } else {
            hot_size + gen.next() as usize % cold_size
        };
        let b = SCACHE.acquire(bno);
        let d = mock_().inspect(bno);
        assert_eq!((*b).data[123], d.add(123).read());
        SCACHE.release(b);
    }

    println!("cached = {}, read = {}", SCACHE.get_num_cached_blocks(), mock_().read_count.load(SeqCst));
    assert!(mock_().read_count.load(SeqCst) < 233);
    assert!(mock_().write_count.load(SeqCst) < 5);
}

unsafe fn test_atomic_op() {
    initialize(32, 64, "");

    let mut ctx = OpContext::new();
    SCACHE.begin_op(&mut ctx);
    SCACHE.end_op(&mut ctx);

    SCACHE.begin_op(&mut ctx);
    let t = (sblock_().num_blocks - 1) as usize;
    let b = SCACHE.acquire(t);
    assert_eq!((*b).block_no, t as usize);
    assert!((*b).valid);
    let d = mock_().inspect(t);
    let v = *d.add(128);
    assert_eq!((*b).data[128], v);

    (*b).data[128] = !v;
    SCACHE.sync(&mut ctx, b);
    SCACHE.release(b);

    assert_eq!(*d.add(128), v);
    SCACHE.end_op(&mut ctx);
    assert_eq!(*d.add(128), !v);

    SCACHE.begin_op(&mut ctx);
    let b1 = SCACHE.acquire(t - 1);
    let b2 = SCACHE.acquire(t - 2);
    assert_eq!((*b1).block_no, t - 1);
    assert_eq!((*b2).block_no, t - 2);

    let d1 = mock_().inspect(t - 1);
    let d2 = mock_().inspect(t - 2);
    let v1 = *d1.add(500);
    let v2 = *d2.add(10);
    assert_eq!((*b1).data[500], v1);
    assert_eq!((*b1).data[10], v2);

    (*b1).data[500] = !v1;
    (*b1).data[10] = !v2;
    SCACHE.sync(&mut ctx, b1);
    SCACHE.release(b1);
    SCACHE.sync(&mut ctx, b2);
    SCACHE.release(b2);

    assert_eq!(*d1.add(500), v1);
    assert_eq!(*d2.add(10), v2);
    SCACHE.end_op(&mut ctx);
    assert_eq!(*d1.add(500), !v1);
    assert_eq!(*d2.add(10), !v2);
}

unsafe fn test_overflow() {
    // todo Can we catch a panic in Rust?
    // initialize(100,100,"");
    // let mut ctx = OpContext::new();
    //
    // SCACHE.begin_op(&mut ctx);
    // let t = (sblock_().num_blocks - 1) as usize;
    // for i in 0..OP_MAX_NUM_BLOCKS{
    //     let b=SCACHE.acquire(t-i);
    //     (*b).data[0]=0xaa;
    //     SCACHE.sync(&mut ctx,b);
    //     SCACHE.release(b);
    // }
    //
    // let b=SCACHE.acquire(t-OP_MAX_NUM_BLOCKS);
    // (*b).data[0]=0xbb;
    // SCACHE.sync(&mut ctx,b);
}

unsafe fn test_local_absorption() {
    let num_rounds = 1000;
    initialize(100, 100, "");
    let mut ctx = OpContext::new();
    SCACHE.begin_op(&mut ctx);
    let t = (sblock_().num_blocks - 1) as usize;
    for i in 0..num_rounds {
        for j in 0..OP_MAX_NUM_BLOCKS {
            let b = SCACHE.acquire(t - j);
            (*b).data[0] = 0xcd;
            SCACHE.sync(&mut ctx, b);
            SCACHE.release(b);
        }
    }
    SCACHE.end_op(&mut ctx);

    assert!(mock_().read_count.load(SeqCst) < OP_MAX_NUM_BLOCKS * 5);
    assert!(mock_().write_count.load(SeqCst) < OP_MAX_NUM_BLOCKS * 5);
    for j in 0..OP_MAX_NUM_BLOCKS {
        let b = mock_().inspect(t - j);
        assert_eq!(*b.add(0), 0xcd);
    }
}

unsafe fn test_global_absorption() {
    let op_size = 3;
    let num_workers = 100;

    initialize(2 * OP_MAX_NUM_BLOCKS + op_size, 100, "");
    let t = (sblock_().num_blocks - 1) as usize;

    let mut out = OpContext::new();
    SCACHE.begin_op(&mut out);

    for i in 0..OP_MAX_NUM_BLOCKS {
        let b = SCACHE.acquire(t - i);
        (*b).data[0] = 0xcc;
        SCACHE.sync(&mut out, b);
        SCACHE.release(b);
    }

    let mut workers = Vec::new();

    for i in 0..num_workers {
        let mut ctx = OpContext::new();
        SCACHE.begin_op(&mut ctx);
        for j in 0..op_size {
            let b = SCACHE.acquire(t - j);
            (*b).data[0] = 0xdd;
            SCACHE.sync(&mut ctx, b);
            SCACHE.release(b);
        }
        workers.push(thread::spawn(move || {
            SCACHE.end_op(&mut ctx);
        }));
    }

    workers.push(thread::spawn(move || {
        SCACHE.end_op(&mut out);
    }));
    for worker in workers {
        worker.join().unwrap();
    }
    for i in 0..op_size {
        let b = mock_().inspect(t - i);
        assert_eq!(*b.add(0), 0xdd);
    }
    for i in op_size..OP_MAX_NUM_BLOCKS {
        let b = mock_().inspect(t - i);
        assert_eq!(*b.add(0), 0xcc);
    }
}

unsafe fn test_replay() {
    initialize_mock(50, 1000, "");

    let header = mock_().inspect_log_header();
    (*header).num_blocks = 5;
    for i in 0..5 {
        let v = 500 + i;
        (*header).block_no[i] = v;
        let b = mock_().inspect_log(i);
        for j in 0..BLOCK_SIZE {
            *b.add(j) = (v & 0xff) as u8;
        }
    }
}