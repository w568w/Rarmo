#![allow(unused_variables)]

use crate::aarch64::mmu::PageTable;
use crate::cores::physical_memory::{PhysicalMemory, PhysicalMemoryTable};

trait VirtualMemoryPageTable<T> where T: PhysicalMemoryTable {
    fn new(p_alloc: &mut PhysicalMemory<T>) -> Self;
    fn walk(&mut self, kernel_addr: usize, alloc_if_not_exist: bool) -> Option<*mut PageTable>;
    fn map(&mut self, virtual_addr: usize, size: usize, physical_addr: usize);
}

impl<T> VirtualMemoryPageTable<T> for *mut PageTable
    where T: PhysicalMemoryTable {
    fn new(p_alloc: &mut PhysicalMemory<T>) -> Self {
        let page_table = p_alloc.page_alloc() as *mut PageTable;
        // Clear the page table with zeros.
        unsafe {
            core::ptr::write_bytes(page_table, 0, 1);
        }
        page_table
    }

    fn walk(&mut self, kernel_addr: usize, alloc_if_not_exist: bool) -> Option<*mut PageTable> {
        todo!()
    }

    fn map(&mut self, virtual_addr: usize, size: usize, physical_addr: usize) {
        todo!()
    }
}