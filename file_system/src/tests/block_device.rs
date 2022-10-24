use crate::defines::{LogHeader, SuperBlock, BIT_PER_BLOCK, BLOCK_SIZE, INODE_PER_BLOCK};
use parking_lot::Mutex;
use prng_mt::MT19937;
use std::mem::MaybeUninit;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use crate::block_device::BlockDevice;
use crate::cache::init_bcache;

pub struct MockBlockDevice {
    offline: AtomicBool,
    pub read_count: AtomicUsize,
    pub write_count: AtomicUsize,
    disk: Vec<Block>,
    pub on_read: Option<Box<dyn FnMut(usize, &[u8])>>,
    pub on_write: Option<Box<dyn FnMut(usize, &[u8])>>,
    sblock: Option<*mut SuperBlock>,
}

impl BlockDevice for MockBlockDevice {
    fn write(&mut self, block_no: usize, buf: &[u8]) {
        Self::check_offline(&self.offline);

        let mut block = &mut self.disk[block_no];
        let _lock = block.mutex.lock();
        if let Some(ref mut on_write) = self.on_write {
            on_write(block_no, buf);
        }

        Self::check_offline(&self.offline);

        for i in 0..BLOCK_SIZE {
            block.data[i] = buf[i];
        }

        self.write_count.fetch_add(1, SeqCst);

        Self::check_offline(&self.offline);
    }

    fn read(&mut self, block_no: usize, buf: &mut [u8]) {
        Self::check_offline(&self.offline);

        let block = &self.disk[block_no];
        let _lock = block.mutex.lock();
        if let Some(ref mut on_read) = self.on_read {
            on_read(block_no, buf);
        }

        Self::check_offline(&self.offline);

        for i in 0..BLOCK_SIZE {
            buf[i] = block.data[i];
        }

        self.read_count.fetch_add(1, SeqCst);

        Self::check_offline(&self.offline);
    }
}

impl MockBlockDevice {
    pub fn new(size: usize) -> Self {
        MockBlockDevice {
            offline: AtomicBool::new(false),
            read_count: AtomicUsize::new(0),
            write_count: AtomicUsize::new(0),
            disk: vec![],
            on_read: None,
            on_write: None,
            sblock: None,
        }
    }

    pub fn init(&mut self, sblock_: &mut SuperBlock) {
        self.sblock = Some(sblock_);
        self.offline.store(false, SeqCst);
        self.read_count.store(0, SeqCst);
        self.write_count.store(0, SeqCst);
        self.disk.clear();
        for _ in 0..sblock_.num_blocks {
            self.disk.push(Block::new());
        }
        for block in self.disk.iter_mut() {
            block.fill_junk();
        }
        assert!(sblock_.num_log_blocks >= 2);
        self.disk[sblock_.log_start as usize].fill_zero();

        let num_bitmap_blocks = (sblock_.num_inodes as usize + BIT_PER_BLOCK - 1) / BIT_PER_BLOCK;
        for i in 0..num_bitmap_blocks {
            self.disk[(sblock_.bitmap_start + i as u32) as usize].fill_zero();
        }

        let num_preallocated = 1
            + 1
            + sblock_.num_log_blocks
            + ((sblock_.num_inodes + (INODE_PER_BLOCK as u32) - 1) / (INODE_PER_BLOCK as u32))
            + (num_bitmap_blocks as u32);
        assert!(num_preallocated + sblock_.num_data_blocks <= sblock_.num_blocks);

        for i in 0..num_preallocated {
            let j = i as usize / BIT_PER_BLOCK;
            let k = i % (BIT_PER_BLOCK as u32);

            self.disk[sblock_.bitmap_start as usize + j].data[k as usize / 8] |= 1 << (k % 8);
        }
    }

    pub fn inspect(&mut self, block_no: usize) -> *mut u8 {
        self.disk[block_no].data.as_mut_ptr()
    }

    pub fn inspect_log(&mut self, index: usize) -> *mut u8 {
        let sblock_ = unsafe { &mut *self.sblock.unwrap() };
        self.inspect(sblock_.log_start as usize + 1 + index)
    }

    pub fn inspect_log_header(&mut self) -> *mut LogHeader {
        let sblock_ = unsafe { &mut *self.sblock.unwrap() };
        unsafe { std::mem::transmute(self.inspect(sblock_.log_start as usize)) }
    }

    pub fn check_offline(offline: &AtomicBool) {
        assert!(offline.load(SeqCst));
    }
}

pub struct Block {
    pub data: [u8; BLOCK_SIZE],
    mutex: Mutex<()>,
}

impl Block {
    pub fn new() -> Block {
        Block {
            data: [0; BLOCK_SIZE],
            mutex: Mutex::new(()),
        }
    }
    pub fn fill_junk(&mut self) {
        let mut gen = MT19937::new(0x19260817);
        for i in 0..BLOCK_SIZE {
            self.data[i] = (gen.next() & 0xff) as u8;
        }
    }

    pub fn fill_zero(&mut self) {
        self.data.fill(0);
    }
}

pub static mut sblock: MaybeUninit<SuperBlock> = MaybeUninit::uninit();
pub static mut mock: MaybeUninit<MockBlockDevice> = MaybeUninit::uninit();

pub unsafe fn initialize_mock(log_size: usize, num_data_blocks: usize, image_path: &str) {
    let sblock_ = sblock.assume_init_mut();
    sblock_.log_start = 2;
    sblock_.inode_start = sblock_.log_start + 1 + log_size as u32;
    sblock_.bitmap_start = sblock_.inode_start + 1;
    sblock_.num_inodes = 1;
    sblock_.num_log_blocks = 1 + log_size as u32;
    sblock_.num_data_blocks = num_data_blocks as u32;
    sblock_.num_blocks = (1
        + 1
        + 1
        + log_size
        + 1
        + ((num_data_blocks + BIT_PER_BLOCK - 1) / BIT_PER_BLOCK)
        + num_data_blocks) as u32;
    mock.assume_init_mut().init(sblock_);
    if !image_path.is_empty() {
        // todo load image
    }
}

pub unsafe fn initialize(log_size: usize, num_data_blocks: usize, image_path: &str) {
    initialize_mock(log_size, num_data_blocks, image_path);
    init_bcache(sblock.assume_init_ref(), mock.assume_init_ref());
}
