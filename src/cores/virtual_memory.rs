#![allow(unused_variables)]

use core::ptr;
use crate::aarch64::intrinsic::set_ttbr0_el1;
use crate::aarch64::kernel_pt::invalid_pt;
use crate::aarch64::mmu::{kernel2physical, N_PTE_PER_TABLE, physical2kernel};
use crate::common::{get_bits, set_bits};
use crate::kernel::mem::{kalloc_page, kfree_page};

pub mod pte_flags {
    use crate::cores::virtual_memory::{AccessPermission, PageTableEntry, PageTableEntryType, Shareability};

    pub fn user(pte: &mut PageTableEntry) {
        pte.set_access_permission(AccessPermission::El1rwEl0rw)
    }

    pub fn normal(pte: &mut PageTableEntry) {
        pte.set_attr_index(1);
        pte.set_accessed(true);
        pte.set_shareability(Shareability::OuterShareable);
    }

    pub fn user_page(pte: &mut PageTableEntry) {
        user(pte);
        normal(pte);
        pte.set_type(PageTableEntryType::TableOrPage);
    }
}

#[repr(transparent)]
pub struct PageTableEntry(pub u64);

pub enum PageTableEntryType {
    Block,
    TableOrPage,
}

pub enum Shareability {
    NonShareable,
    OuterShareable,
    InnerShareable,
}

pub enum AccessPermission {
    El1rwEl0n,
    El1rwEl0rw,
    EL1rEL0n,
    EL1rEL0r,
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
            PageTableEntryType::TableOrPage
        } else {
            PageTableEntryType::Block
        }
    }

    pub const fn set_type(&mut self, type_: PageTableEntryType) {
        let type_ = match type_ {
            PageTableEntryType::Block => 0,
            PageTableEntryType::TableOrPage => 1,
        };
        set_bits(&mut self.0, type_, 1, 2);
    }

    const fn addr_offset(&self, level: u8) -> u8 {
        match self.type_() {
            PageTableEntryType::Block => {
                12 + 9 * (3 - level)
            }
            PageTableEntryType::TableOrPage => {
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

    /// Attributes
    pub const fn execute_never(&self) -> bool {
        get_bits(self.0, 54, 55) == 1
    }

    pub const fn set_execute_never(&mut self, execute_never: bool) {
        let execute_never = if execute_never { 1 } else { 0 };
        set_bits(&mut self.0, execute_never, 54, 55);
    }

    pub const fn privileged_execute_never(&self) -> bool {
        get_bits(self.0, 53, 54) == 1
    }

    pub const fn set_privileged_execute_never(&mut self, privileged_execute_never: bool) {
        let privileged_execute_never = if privileged_execute_never { 1 } else { 0 };
        set_bits(&mut self.0, privileged_execute_never, 53, 54);
    }

    pub const fn accessed(&self) -> bool {
        get_bits(self.0, 10, 11) == 1
    }

    pub const fn set_accessed(&mut self, accessed: bool) {
        let accessed = if accessed { 1 } else { 0 };
        set_bits(&mut self.0, accessed, 10, 11);
    }

    pub const fn shareability(&self) -> Shareability {
        match get_bits(self.0, 8, 10) {
            0 => Shareability::NonShareable,
            2 => Shareability::OuterShareable,
            3 => Shareability::InnerShareable,
            _ => unreachable!(),
        }
    }

    pub const fn set_shareability(&mut self, shareability: Shareability) {
        let shareability = match shareability {
            Shareability::NonShareable => 0,
            Shareability::OuterShareable => 2,
            Shareability::InnerShareable => 3,
        };
        set_bits(&mut self.0, shareability, 8, 10);
    }

    pub const fn access_permission(&self) -> AccessPermission {
        match get_bits(self.0, 6, 8) {
            0 => AccessPermission::El1rwEl0n,
            1 => AccessPermission::El1rwEl0rw,
            2 => AccessPermission::EL1rEL0n,
            3 => AccessPermission::EL1rEL0r,
            _ => unreachable!(),
        }
    }

    pub const fn set_access_permission(&mut self, access_permission: AccessPermission) {
        let access_permission = match access_permission {
            AccessPermission::El1rwEl0n => 0,
            AccessPermission::El1rwEl0rw => 1,
            AccessPermission::EL1rEL0n => 2,
            AccessPermission::EL1rEL0r => 3,
        };
        set_bits(&mut self.0, access_permission, 6, 8);
    }

    pub const fn non_secure(&self) -> bool {
        get_bits(self.0, 5, 6) == 1
    }

    pub const fn set_non_secure(&mut self, non_secure: bool) {
        let non_secure = if non_secure { 1 } else { 0 };
        set_bits(&mut self.0, non_secure, 5, 6);
    }

    pub const fn attr_index(&self) -> u8 {
        get_bits(self.0, 2, 5) as u8
    }

    pub const fn set_attr_index(&mut self, attr_index: u8) {
        set_bits(&mut self.0, attr_index as u64, 2, 5);
    }

    pub fn free(&mut self, level: u8) {
        if self.valid() {
            if level < 3 && matches!(self.type_(), PageTableEntryType::TableOrPage) {
                let addr = self.kernel_addr(level);
                let table = unsafe { &mut *(addr as *mut PageTable) };
                for entry in table.iter_mut() {
                    entry.free(level + 1);
                }
                kfree_page(self.kernel_addr(level) as *mut u8, 1);
            }
            self.set_valid(false);
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
                    pte.set_type(PageTableEntryType::TableOrPage);
                    pte.set_addr(kernel2physical(new_page_table as u64) as usize, level);
                } else {
                    return None;
                }
            }
            if level == 3 {
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