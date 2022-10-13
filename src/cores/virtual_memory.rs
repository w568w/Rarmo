#![allow(unused_variables)]

use core::ptr;
use crate::aarch64::intrinsic::set_ttbr0_el1;
use crate::aarch64::kernel_pt::invalid_pt;
use crate::aarch64::mmu::{kernel2physical, RawPageTable};
use crate::kernel::mem::kalloc_page;

#[repr(transparent)]
pub struct PageTableEntry(pub u64);

pub trait VirtualMemoryPageTable {
    fn new() -> Self;
    fn walk(&mut self, virtual_addr: usize, alloc_if_not_exist: bool) -> Option<*mut PageTableEntry>;
    fn free(&mut self);
    fn attach(&self);
}

pub struct PageTableDirectory {
    page_table: *mut RawPageTable,
}

impl PageTableDirectory {
    pub const fn uninit() -> Self {
        Self {
            page_table: ptr::null_mut(),
        }
    }
    pub fn init(&mut self) {
        self.page_table = kalloc_page(1) as *mut RawPageTable;
        // Clear the page table with zeros.
        unsafe {
            ptr::write_bytes(self.page_table, 0, 1);
        }
    }
}

impl VirtualMemoryPageTable for PageTableDirectory {
    fn new() -> Self {
        let mut ptd = PageTableDirectory::uninit();
        ptd.init();
        ptd
    }

    fn walk(&mut self, kernel_addr: usize, alloc_if_not_exist: bool) -> Option<*mut PageTableEntry> {
        todo!()
        // Return a pointer to the PTE (Page Table Entry) for virtual address 'va'
        // If the entry not exists (NEEDN'T BE VALID), allocate it if alloc=true, or return NULL if false.
        // THIS ROUTINUE GETS THE PTE, NOT THE PAGE DESCRIBED BY PTE.
    }

    fn free(&mut self) {
        todo!()
        // Free pages used by the page table. If pgdir->pt=NULL, do nothing.
        // DONT FREE PAGES DESCRIBED BY THE PAGE TABLE
    }

    fn attach(&self) {
        if self.page_table.is_null() {
            set_ttbr0_el1(kernel2physical(&invalid_pt as *const _ as u64));
        } else {
            set_ttbr0_el1(kernel2physical(self.page_table as u64));
        }
    }
}