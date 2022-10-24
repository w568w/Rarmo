use std::mem::size_of;

pub const BLOCK_SIZE: usize = 512;
pub const LOG_MAX_SIZE: usize = (BLOCK_SIZE - size_of::<usize>()) / size_of::<usize>();

pub const INODE_NUM_DIRECT: usize = 12;
pub const INODE_NUM_INDIRECT: usize = BLOCK_SIZE / size_of::<u32>();
pub const INODE_PER_BLOCK: usize = BLOCK_SIZE / size_of::<InodeEntry>();
pub const INODE_MAX_BLOCKS: usize = INODE_NUM_DIRECT + INODE_NUM_INDIRECT;
pub const INODE_MAX_SIZE: usize = INODE_MAX_BLOCKS * BLOCK_SIZE;

pub const FILE_NAME_MAX_LENGTH: usize = 14;

pub const INODE_INVALID: u16 = 0;
pub const INODE_DIRECTORY: u16 = 1;
pub const INODE_REGULAR: u16 = 2;
pub const INODE_DEVICE: u16 = 3;

pub const BIT_PER_BLOCK: usize = BLOCK_SIZE * 8;
pub const ROOT_INODE_NO: usize = 1;

#[repr(C)]
#[derive(Clone)]
pub struct SuperBlock {
    pub num_blocks: u32,
    // total number of blocks in filesystem.
    pub num_data_blocks: u32,
    pub num_inodes: u32,
    pub num_log_blocks: u32,
    // number of blocks for logging, including log header.
    pub log_start: u32,
    // the first block of logging area.
    pub inode_start: u32,
    // the first block of inode area.
    pub bitmap_start: u32,    // the first block of bitmap area.
}

#[repr(C)]
pub struct InodeEntry {
    pub typ: u16,
    pub major: u16,
    // major device id, for INODE_DEVICE only.
    pub minor: u16,
    // minor device id, for INODE_DEVICE only.
    pub num_links: u16,
    // number of hard links to this inode in the filesystem.
    pub num_bytes: u32,
    // number of bytes in the file, i.e. the size of file.
    pub addrs: [u32; INODE_NUM_DIRECT],
    // direct addresses/block numbers.
    pub indirect: u32,                 // the indirect address block.
}

#[repr(C)]
pub struct IndirectBlock {
    pub addrs: [u32; INODE_NUM_INDIRECT],
}

#[repr(C)]
pub struct DirEntry {
    pub inode_no: u16,
    pub name: [u8; FILE_NAME_MAX_LENGTH],
}

#[repr(C)]
pub struct LogHeader {
    pub num_blocks: usize,
    pub block_no: [usize; LOG_MAX_SIZE],
}