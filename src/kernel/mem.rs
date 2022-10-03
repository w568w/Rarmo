use core::sync::atomic::AtomicUsize;
use spin::RwLock;
use crate::aarch64::mmu::{_kernel2physical_mut, _physical2kernel_mut, kernel2physical, PHYSICAL_TOP};
use crate::cores::physical_memory::{BuddyPageAllocation, PhysicalMemory, PhysicalMemoryTable};
use crate::cores::slob;
use crate::cores::slob::KMemCache;
use crate::define_early_init;

// Why cannot leave the value here as None, and then create it in `init_physical_page_table`?
//
// We have to allocate the page table in the .data section directly, rather than create on the stack and copy it to .data,
// which will blow up the tiny stack.
static KERNEL_PHYSICAL_PT: RwLock<PhysicalMemory<BuddyPageAllocation>> = RwLock::new(PhysicalMemory {
    table: BuddyPageAllocation::uninitialized()
});
// Record the number of allocated pages. Just for test.
pub static ALLOC_PAGE_CNT: AtomicUsize = AtomicUsize::new(0);

pub extern "C" fn init_physical_page_table() {
    extern "C" {
        fn ekernel();
    }
    let mut binding = KERNEL_PHYSICAL_PT.write();
    binding.table.init(kernel2physical(ekernel as u64) as *mut u8,
                       PHYSICAL_TOP as *mut u8);
}

define_early_init!(init_physical_page_table);

pub fn kalloc_page(page_num: usize) -> *mut u8 {
    ALLOC_PAGE_CNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    let mut binding = KERNEL_PHYSICAL_PT.write();
    _physical2kernel_mut(binding.table.page_alloc(page_num))
}

pub fn kfree_page(page_addr: *mut u8, page_num: usize) {
    ALLOC_PAGE_CNT.fetch_sub(1, core::sync::atomic::Ordering::AcqRel);
    let mut binding = KERNEL_PHYSICAL_PT.write();
    binding.page_free(_kernel2physical_mut(page_addr), page_num)
}

pub fn kmalloc(size: usize) -> *mut u8 {
    slob::kmem_cache_alloc_node(&KMemCache {
        size,
        align: align_num(size),
        name: "kmalloc",
    }).expect("Unable to allocate memory from SLOB")
}

pub fn kfree(obj: *mut u8) -> usize {
    slob::dealloc_node(obj)
}

const fn align_num(size: usize) -> usize {
    if size % 8 == 0 {
        8
    } else if size % 4 == 0 {
        4
    } else if size % 2 == 0 {
        2
    } else {
        0
    }
}