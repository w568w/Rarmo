use parking_lot::Mutex;
use crate::block_device::BlockDevice;
use crate::defines::{BLOCK_SIZE, SuperBlock};

pub const OP_MAX_NUM_BLOCKS: usize = 10;
pub const EVICTION_THRESHOLD: usize = 10;

pub struct OpContext {
    pub ts: usize,
}

impl OpContext {
    pub const fn new() -> Self {
        Self {
            ts: 0,
        }
    }
}

static CACHE_LOCK: Mutex<()> = Mutex::new(());

pub struct Block {
    pub block_no: usize,
    pub acquired: bool,
    pub pinned: bool,

    // todo sleeplock
    pub valid: bool,
    pub data: [u8; BLOCK_SIZE],
}
impl Block{
    pub const fn new() -> Self {
        Self {
            block_no: 0,
            acquired: false,
            pinned: false,
            valid: false,
            data: [0; BLOCK_SIZE],
        }
    }

    pub fn fill_zero(&mut self) {
        for i in 0..BLOCK_SIZE {
            self.data[i] = 0;
        }
    }
}
pub trait BlockCache {
    fn get_num_cached_blocks(&self) -> usize;

    fn acquire(&mut self, block_no: usize) -> *mut Block;

    fn release(&mut self, block: *mut Block);

    fn begin_op(&mut self, ctx: &mut OpContext);


    fn sync(&mut self, ctx: *mut OpContext, block: *mut Block);

    fn end_op(&mut self, ctx: &mut OpContext);

    fn alloc(&mut self, ctx: *mut OpContext) -> usize;

    fn free(&mut self, ctx: *mut OpContext, block_no: usize);
}

pub struct BlockCacheImpl;

impl BlockCache for BlockCacheImpl {
    fn get_num_cached_blocks(&self) -> usize {
        todo!()
    }

    fn acquire(&mut self, block_no: usize) -> *mut Block {
        todo!()
    }

    fn release(&mut self, block: *mut Block) {
        todo!()
    }

    fn begin_op(&mut self, ctx: &mut OpContext) {
        todo!()
    }

    fn sync(&mut self, ctx: *mut OpContext, block: *mut Block) {
        todo!()
    }

    fn end_op(&mut self, ctx: &mut OpContext) {
        todo!()
    }

    fn alloc(&mut self, ctx: *mut OpContext) -> usize {
        todo!()
    }

    fn free(&mut self, ctx: *mut OpContext, block_no: usize) {
        todo!()
    }
}

pub fn init_bcache(sblock: &SuperBlock, device: &dyn BlockDevice) {
    todo!()
}

pub static mut SCACHE: BlockCacheImpl = BlockCacheImpl;