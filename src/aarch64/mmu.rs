#![allow(dead_code)]
#![allow(unused)]
#![allow(non_upper_case_globals)]

pub const PAGE_SIZE: usize = 4096;

#[derive(Clone, Copy)]
#[repr(align(4096))]
pub struct APageSize;

/* memory region attributes */
pub const MT_DEVICE_nGnRnE: u64 = 0x0;
pub const MT_NORMAL: u64 = 0x1;
pub const MT_NORMAL_NC: u64 = 0x2;
pub const MT_DEVICE_nGnRnE_FLAGS: u64 = 0x00;
pub const MT_NORMAL_FLAGS: u64 = 0xFF; /* Inner/Outer Write-Back Non-Transient RW-Allocate */
pub const MT_NORMAL_NC_FLAGS: u64 = 0x44;/* Inner/Outer Non-Cacheable */

pub const SH_OUTER: u64 = 2 << 8;
pub const SH_INNER: u64 = 3 << 8;

pub const AF_USED: u64 = 1 << 10;

pub const PTE_NORMAL_NC: u64 = (MT_NORMAL_NC << 2) | AF_USED | SH_OUTER;
pub const PTE_NORMAL: u64 = (MT_NORMAL << 2) | AF_USED | SH_OUTER;
pub const PTE_DEVICE: u64 = (MT_DEVICE_nGnRnE << 2) | AF_USED;

pub const PTE_VALID: u64 = 0x1;

pub const PTE_TABLE: u64 = 0x3;
pub const PTE_BLOCK: u64 = 0x1;
pub const PTE_PAGE: u64 = 0x3;

pub const PTE_KERNEL: u64 = 0 << 6;
pub const PTE_USER: u64 = 1 << 6;

pub const PTE_KERNEL_DATA: u64 = PTE_KERNEL | PTE_NORMAL | PTE_BLOCK;
pub const PTE_KERNEL_DEVICE: u64 = PTE_KERNEL | PTE_DEVICE | PTE_BLOCK;
pub const PTE_USER_DATA: u64 = PTE_USER | PTE_NORMAL | PTE_PAGE;

pub const N_PTE_PER_TABLE: usize = 512;

pub type PageTable = [*const u8; N_PTE_PER_TABLE];

// Another type of page table, which is used to initialize a page table with `u64`.
pub type RawPageTable = [u64; N_PTE_PER_TABLE];

pub const KSPACE_MASK: u64 = 0xffff000000000000;

pub const PHYSICAL_TOP: u64 = 0x3f000000;

pub const fn kernel2physical(addr: u64) -> u64 {
    addr - KSPACE_MASK
}

pub const unsafe fn _kernel2physical<T>(addr: *const T) -> *const T {
    addr.wrapping_offset(KSPACE_MASK.wrapping_neg() as isize)
}

pub const unsafe fn _kernel2physical_mut<T>(addr: *mut T) -> *mut T {
    addr.wrapping_offset(KSPACE_MASK.wrapping_neg() as isize)
}


pub const fn physical2kernel(addr: u64) -> u64 {
    addr + KSPACE_MASK
}

pub const unsafe fn _physical2kernel<T>(addr: *const T) -> *const T {
    addr.wrapping_offset(KSPACE_MASK as isize)
}

pub const unsafe fn _physical2kernel_mut<T>(addr: *mut T) -> *mut T {
    addr.wrapping_offset(KSPACE_MASK as isize)
}

pub const fn into_kernel_addr(addr: u64) -> u64 {
    addr | KSPACE_MASK
}

pub const fn into_physical_addr(addr: u64) -> u64 {
    addr & (!KSPACE_MASK)
}