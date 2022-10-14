#![allow(unused_variables)]

use core::ptr;
use crate::aarch64::intrinsic::set_ttbr0_el1;
use crate::aarch64::kernel_pt::invalid_pt;
use crate::aarch64::mmu::{kernel2physical, N_PTE_PER_TABLE, physical2kernel};
use crate::common::{get_bits, set_bits};
use crate::kernel::mem::{kalloc_page, kfree_page};

#[repr(transparent)]
pub struct PageTableEntry(pub u64);

pub enum PageTableEntryType {
    Block,
    Table,
}

impl PageTableEntry {
    pub const fn new() -> Self {
        Self(0)
    }
    pub const fn valid(&self) -> bool {
        get_bits(self.0, 0, 1) == 1
    }

    pub const fn set_valid(&mut self, valid: bool) {
        let valid = if valid { 1 } else { 0 };
        set_bits(&mut self.0, valid, 0, 1);
    }

    pub const fn type_(&self) -> PageTableEntryType {
        if get_bits(self.0, 1, 2) == 1 {
            PageTableEntryType::Table
        } else {
            PageTableEntryType::Block
        }
    }

    pub const fn set_type(&mut self, type_: PageTableEntryType) {
        let type_ = match type_ {
            PageTableEntryType::Block => 0,
            PageTableEntryType::Table => 1,
        };
        set_bits(&mut self.0, type_, 1, 2);
    }

    const fn addr_offset(&self, level: u8) -> u8 {
        match self.type_() {
            PageTableEntryType::Block => {
                12 + 9 * (3 - level)
            }
            PageTableEntryType::Table => {
                12
            }
        }
    }
    pub const fn addr(&self, level: u8) -> usize {
        let offset = self.addr_offset(level);
        (get_bits(self.0, offset, 64 - offset) << offset) as usize
    }

    const fn kernel_addr(&self, level: u8) -> u64 {
        physical2kernel(self.addr(level) as u64)
    }

    pub const fn set_addr(&mut self, addr: usize, level: u8) {
        let offset = self.addr_offset(level);
        set_bits(&mut self.0, (addr >> offset) as u64, offset, 64 - offset);
    }

    pub fn free(&mut self, level: u8) {
        if self.valid() {
            if matches!(self.type_(), PageTableEntryType::Table) {
                let addr = self.kernel_addr(level);
                let table = unsafe { &mut *(addr as *mut PageTable) };
                for entry in table.iter_mut() {
                    entry.free(level + 1);
                }
            }
            self.set_valid(false);
            kfree_page(self.kernel_addr(level) as *mut u8, 1);
        }
    }
}

pub type PageTable = [PageTableEntry; N_PTE_PER_TABLE];

#[repr(transparent)]
pub struct FourLevelVirtualAddress(pub u64);

impl From<u64> for FourLevelVirtualAddress {
    fn from(addr: u64) -> Self {
        Self(addr)
    }
}

impl FourLevelVirtualAddress {
    pub fn level_index(&self, level: u8) -> u64 {
        let offset = 12 + 9 * (3 - level);
        get_bits(self.0, offset, offset + 9)
    }
}

pub trait VirtualMemoryPageTable {
    fn new() -> Self;
    fn walk(&mut self, virtual_addr: usize, alloc_if_not_exist: bool) -> Option<*mut PageTableEntry>;
    fn free(&mut self);
    fn attach(&self);
}

pub struct PageTableDirectory {
    page_table: *mut PageTable,
}

impl PageTableDirectory {
    pub const fn uninit() -> Self {
        Self {
            page_table: ptr::null_mut(),
        }
    }
    pub fn init(&mut self) {
        self.page_table = kalloc_page(1) as *mut PageTable;
        // Clear the page table with zeros.
        unsafe {
            ptr::write_bytes(self.page_table, 0, 1);
        }
    }

    pub fn get_page_table(&self) -> &mut PageTable {
        unsafe { &mut *self.page_table }
    }
}

impl VirtualMemoryPageTable for PageTableDirectory {
    fn new() -> Self {
        let mut ptd = PageTableDirectory::uninit();
        ptd.init();
        ptd
    }

    fn walk(&mut self, virtual_addr: usize, alloc_if_not_exist: bool) -> Option<*mut PageTableEntry> {
        // Return a pointer to the PTE (Page Table Entry) for virtual address 'va'
        // If the entry not exists (NEEDN'T BE VALID), allocate it if alloc=true, or return NULL if false.
        // THIS ROUTINUE GETS THE PTE, NOT THE PAGE DESCRIBED BY PTE.
        let virtual_addr = FourLevelVirtualAddress::from(virtual_addr as u64);
        let mut current_page_table = self.get_page_table();
        for level in 0..4 {
            let index = virtual_addr.level_index(level);
            let pte = current_page_table.get_mut(index as usize)?;
            if !pte.valid() {
                if alloc_if_not_exist {
                    let new_page_table = kalloc_page(1) as *mut PageTable;
                    unsafe {
                        ptr::write_bytes(new_page_table, 0, 1);
                    }
                    pte.set_valid(true);
                    pte.set_type(if level == 3 {
                        PageTableEntryType::Block
                    } else {
                        PageTableEntryType::Table
                    });
                    pte.set_addr(kernel2physical(new_page_table as u64) as usize, level);
                } else {
                    return None;
                }
            }
            if matches!(pte.type_(), PageTableEntryType::Block) {
                return Some(pte);
            }
            if level < 3 {
                current_page_table = unsafe { &mut *(pte.kernel_addr(level) as *mut PageTable) };
            }
        }
        None
    }

    /// Free the page table and its sub page tables.
    /// The page pointed by the page table directory will NOT be freed.
    ///
    /// Note: This function should be called when the page table is not used anymore,
    /// and the page table should be detached before calling this function.
    ///
    fn free(&mut self) {
        for entry in self.get_page_table().iter_mut() {
            entry.free(0);
        }
        kfree_page(self.page_table as *mut u8, 1);
        self.page_table = ptr::null_mut();
    }

    fn attach(&self) {
        if self.page_table.is_null() {
            set_ttbr0_el1(kernel2physical(&invalid_pt as *const _ as u64));
        } else {
            set_ttbr0_el1(kernel2physical(self.page_table as u64));
        }
    }
}