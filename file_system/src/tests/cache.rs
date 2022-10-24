use std::cmp::{max, min};
use std::collections::HashMap;
use std::mem::{MaybeUninit, size_of, transmute};
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;
use parking_lot::{Condvar, Mutex, RawMutex};
use parking_lot::lock_api::RawMutex as RawMutexTrait;
use prng_mt::MT19937;
use crate::cache::{Block, BlockCache, OpContext};
use crate::Container;
use crate::defines::{BLOCK_SIZE, INODE_DIRECTORY, INODE_INVALID, INODE_NUM_DIRECT, INODE_PER_BLOCK, InodeEntry, SuperBlock};

const NUM_BLOCKS: usize = 2000;
const INODE_START: usize = 200;
const BLOCK_START: usize = 1000;
const NUM_INODES: usize = 21000;

pub struct MockBlockCache {
    mutex: Mutex<()>,
    cv: Condvar,
    oracle: AtomicUsize,
    top_oracle: AtomicUsize,
    score_board: HashMap<usize, bool>,
    mbit: [Meta; NUM_BLOCKS],
    sbit: [Meta; NUM_BLOCKS],
    mblk: [Cell; NUM_BLOCKS],
    sblk: [Cell; NUM_BLOCKS],
}

impl MockBlockCache {
    fn get_sblock() -> SuperBlock {
        SuperBlock {
            num_blocks: NUM_BLOCKS as u32,
            num_inodes: NUM_INODES as u32,
            inode_start: INODE_START as u32,
            num_data_blocks: (NUM_BLOCKS - BLOCK_START) as u32,
            log_start: 2,
            num_log_blocks: 50,
            bitmap_start: 900,
        }
    }

    pub unsafe fn new() -> Self {
        let mut gen = MT19937::new(0);
        let mut mbit: [MaybeUninit<Meta>; NUM_BLOCKS] = MaybeUninit::uninit_array();
        let mut sbit: [MaybeUninit<Meta>; NUM_BLOCKS] = MaybeUninit::uninit_array();
        let mut mblk: [MaybeUninit<Cell>; NUM_BLOCKS] = MaybeUninit::uninit_array();
        let mut sblk: [MaybeUninit<Cell>; NUM_BLOCKS] = MaybeUninit::uninit_array();
        for i in 0..NUM_BLOCKS {
            mbit[i] = MaybeUninit::new(Meta::new());
            sbit[i] = MaybeUninit::new(Meta::new());
            mblk[i] = MaybeUninit::new(Cell::new(i));
            mblk[i].assume_init_mut().random(&mut gen);
            sblk[i] = MaybeUninit::new(Cell::new(i));
            sblk[i].assume_init_mut().random(&mut gen);
        }

        let sblock = Self::get_sblock();

        let buf: *mut u8 = transmute(&sblock);
        for i in 0..size_of::<SuperBlock>() {
            sblk[1].assume_init_mut().block.data[i] = *(buf.add(i));
        }

        let mut node: [MaybeUninit<InodeEntry>; NUM_INODES] = MaybeUninit::uninit_array();
        for i in 0..NUM_INODES {
            let node = node[i].assume_init_mut();
            node.typ = INODE_INVALID;
            node.major = (gen.next() & 0xffff) as u16;
            node.minor = (gen.next() & 0xffff) as u16;
            node.num_links = (gen.next() & 0xffff) as u16;
            node.num_bytes = (gen.next() & 0xffff) as u32;
            for j in 0..INODE_NUM_DIRECT {
                node.addrs[j] = gen.next();
            }
            node.indirect = gen.next();
        }

        let root_node = node[1].assume_init_mut();
        root_node.typ = INODE_DIRECTORY;
        root_node.major = 0;
        root_node.minor = 0;
        root_node.num_links = 1;
        root_node.num_bytes = 0;
        for j in 0..INODE_NUM_DIRECT {
            root_node.addrs[j] = 0;
        }
        root_node.indirect = 0;

        let mut step = 0;
        let mut i = 0;
        let mut j = INODE_START;
        while i < NUM_INODES {
            step = min(NUM_INODES - i, INODE_PER_BLOCK);
            let buf: *mut u8 = transmute(&node[i]);
            for k in 0..(step * size_of::<InodeEntry>()) {
                sblk[j].assume_init_mut().block.data[k] = *(buf.add(k));
            }
            i += step;
            j += 1;
        }

        let mut cache = MockBlockCache {
            mutex: Mutex::new(()),
            cv: Condvar::new(),
            oracle: AtomicUsize::new(1),
            top_oracle: AtomicUsize::new(0),
            score_board: HashMap::new(),
            mbit: transmute(mbit),
            sbit: transmute(sbit),
            mblk: transmute(mblk),
            sblk: transmute(sblk),
        };

        cache
    }

    pub fn fill_junk(&mut self) {
        let mut gen = MT19937::new(0xdeadbeef);
        for i in 0..NUM_BLOCKS {
            self.mbit[i].mutex.lock();
            assert!(!self.mbit[i].mark);
            unsafe { self.mbit[i].mutex.unlock(); }
        }
        for i in 0..NUM_BLOCKS {
            self.mblk[i].mutex.lock();
            assert!(!self.mblk[i].mark);
            self.mblk[i].random(&mut gen);
            unsafe { self.mblk[i].mutex.unlock(); }
        }
    }

    pub unsafe fn count_inodes(&self) -> usize {
        let _lock = self.mutex.lock();
        let mut count = 0;
        let mut step = 0;
        let mut i = 0;
        let mut j = INODE_START;

        while i < NUM_INODES {
            step = min(NUM_INODES - i, INODE_PER_BLOCK);
            let inodes: *const InodeEntry = transmute(&self.sblk[j].block.data);
            for k in 0..step {
                if (*inodes.add(k)).typ != INODE_INVALID {
                    count += 1;
                }
            }
            i += step;
            j += 1;
        }

        count
    }

    pub fn count_blocks(&self) -> usize {
        let _lock = self.mutex.lock();
        let mut count = 0;
        for i in BLOCK_START..NUM_BLOCKS {
            self.sbit[i].mutex.lock();
            if self.sbit[i].used {
                count += 1;
            }
            unsafe { self.sbit[i].mutex.unlock(); }
        }
        count
    }
    pub unsafe fn inspect(&self, i: usize) -> *const InodeEntry {
        let j = INODE_START + i / INODE_PER_BLOCK;
        let k = i % INODE_PER_BLOCK;
        let arr: *const InodeEntry = transmute(&self.sblk[j].block.data);
        arr.add(k)
    }
    pub unsafe fn check_and_get_cell(&self, b: &Block) -> *const Cell {
        let cell: *const Cell = Cell::get_parent_ptr(b);
        let offset = cell.byte_offset_from(self.mblk.as_ptr());
        assert_eq!(offset % (size_of::<Cell>() as isize), 0);
        let i = cell.offset_from(self.mblk.as_ptr());
        assert!(i >= 0);
        assert!(i < NUM_BLOCKS as isize);
        cell
    }
}

impl BlockCache for MockBlockCache {
    fn get_num_cached_blocks(&self) -> usize {
        todo!()
    }

    fn acquire(&mut self, i: usize) -> *mut Block {
        self.mblk[i].mutex.lock();
        self.sblk[i].mutex.lock();
        Cell::load(&mut self.mblk[i], &self.sblk[i]);
        unsafe { self.sblk[i].mutex.unlock(); }
        &mut self.mblk[i].block
    }

    fn release(&mut self, b: *mut Block) {
        unsafe {
            let cell = self.check_and_get_cell(&*b);
            (*cell).mutex.unlock();
        }
    }

    fn begin_op(&mut self, ctx: &mut OpContext) {
        let _lock = self.mutex.lock();
        ctx.ts = self.oracle.fetch_add(1, SeqCst);
        self.score_board.insert(ctx.ts, false);
    }

    fn sync(&mut self, ctx: *mut OpContext, block: *mut Block) {
        let p = unsafe { &*self.check_and_get_cell(&*block) };
        let i = p.index;
        if ctx.is_null(){
            self.sblk[i].mutex.lock();
            Cell::store(&mut self.mblk[i], &mut self.sblk[i]);
            unsafe { self.sblk[i].mutex.unlock(); }
        }
    }

    fn end_op(&mut self, ctx: &mut OpContext) {
        let mut lock = self.mutex.lock();
        self.score_board.insert(ctx.ts, true);

        let mut do_checkpoint = true;
        for (_, v) in &self.score_board {
            if !*v {
                do_checkpoint = false;
                break;
            }
        }
        if do_checkpoint {
            for i in 0..NUM_BLOCKS {
                self.mbit[i].mutex.lock();
                self.sbit[i].mutex.lock();
                Meta::store(&mut self.mbit[i], &mut self.sbit[i]);
                unsafe { self.mbit[i].mutex.unlock(); }
                unsafe { self.sbit[i].mutex.unlock(); }
            }
            for i in 0..NUM_BLOCKS {
                self.mblk[i].mutex.lock();
                self.sblk[i].mutex.lock();
                Cell::store(&mut self.mblk[i], &mut self.sblk[i]);
                unsafe { self.mblk[i].mutex.unlock(); }
                unsafe { self.sblk[i].mutex.unlock(); }
            }
            let mut max_oracle = 0;
            for (k, _) in &self.score_board {
                max_oracle = max(max_oracle, *k);
            }
            self.top_oracle.store(max_oracle, SeqCst);
            self.score_board.clear();
            self.cv.notify_all();
        } else {
            self.cv.wait_while(&mut lock, |&mut ()| {
                ctx.ts <= self.top_oracle.load(SeqCst)
            });
        }
    }

    fn alloc(&mut self, ctx: *mut OpContext) -> usize {
        for i in BLOCK_START..NUM_BLOCKS {
            self.mbit[i].mutex.lock();
            self.sbit[i].mutex.lock();
            Meta::load(&mut self.mbit[i], &self.sbit[i]);

            if !self.mbit[i].used {
                self.mbit[i].used = true;
                if ctx.is_null() {
                    Meta::store(&mut self.mbit[i], &mut self.sbit[i]);
                }
                self.mblk[i].mutex.lock();
                self.sblk[i].mutex.lock();
                Cell::load(&mut self.mblk[i], &self.sblk[i]);
                self.mblk[i].zero();
                if ctx.is_null() {
                    Cell::store(&mut self.mblk[i], &mut self.sblk[i]);
                }

                unsafe { self.sblk[i].mutex.unlock(); }
                unsafe { self.mblk[i].mutex.unlock(); }
                unsafe { self.sbit[i].mutex.unlock(); }
                unsafe { self.mbit[i].mutex.unlock(); }
                return i;
            }
            unsafe { self.sbit[i].mutex.unlock(); }
            unsafe { self.mbit[i].mutex.unlock(); }
        }
        panic!("no free block");
    }

    fn free(&mut self, ctx: *mut OpContext, block_no: usize) {
        self.mbit[block_no].mutex.lock();
        self.sbit[block_no].mutex.lock();
        Meta::load(&mut self.mbit[block_no], &self.sbit[block_no]);
        if !self.mbit[block_no].used {
            panic!("freeing unused block");
        }
        self.mbit[block_no].used = false;
        if ctx.is_null() {
            Meta::store(&mut self.mbit[block_no], &mut self.sbit[block_no]);
        }
        unsafe { self.sbit[block_no].mutex.unlock(); }
        unsafe { self.mbit[block_no].mutex.unlock(); }
    }
}

#[repr(C)]
pub struct Meta {
    mark: bool,
    mutex: RawMutex,
    used: bool,
}

impl Meta {
    pub fn new() -> Self {
        Meta {
            mark: false,
            mutex: RawMutex::INIT,
            used: false,
        }
    }
    pub fn load(a: &mut Meta, b: &Meta) {
        if !a.mark {
            a.used = b.used;
            a.mark = true;
        }
    }
    pub fn store(a: &mut Meta, b: &mut Meta) {
        if a.mark {
            b.used = a.used;
            a.mark = false;
        }
    }
}

#[repr(C)]
pub struct Cell {
    mark: bool,
    index: usize,
    mutex: RawMutex,
    block: Block,
}

impl Container<Block> for Cell {
    fn get_child_offset() -> usize {
        field_offset::offset_of!(Cell => block).get_byte_offset()
    }
}

impl Cell {
    pub fn new(index: usize) -> Self {
        Cell {
            mark: false,
            index,
            mutex: RawMutex::INIT,
            block: Block::new(),
        }
    }

    pub fn zero(&mut self) {
        self.block.fill_zero();
    }

    pub fn random(&mut self, gen: &mut MT19937) {
        for i in 0..BLOCK_SIZE {
            self.block.data[i] = (gen.next() & 0xff) as u8;
        }
    }
    pub fn load(a: &mut Cell, b: &Cell) {
        if !a.mark {
            a.block.data.copy_from_slice(&b.block.data);
            a.mark = true;
        }
    }
    pub fn store(a: &mut Cell, b: &mut Cell) {
        if a.mark {
            b.block.data.copy_from_slice(&a.block.data);
            a.mark = false;
        }
    }
}