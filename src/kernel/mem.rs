use core::sync::atomic::{AtomicI64, AtomicUsize};
use spin::RwLock;
use crate::aarch64::mmu::{_kernel2physical, _kernel2physical_mut, _physical2kernel, _physical2kernel_mut, kernel2physical, PAGE_SIZE, physical2kernel, PHYSICAL_TOP};
use crate::common::round_up;
use crate::cores::physical_memory::{LinkedMemoryTable, PhysicalMemory, PhysicalMemoryTable};
use crate::cores::slob;
use crate::cores::slob::KMemCache;
use crate::{CONSOLE, define_early_init};
use core::fmt::Write;

// Why cannot leave the value here as None, and then create it in `init_physical_page_table`?
//
// We have to allocate the page table in the .data section directly, rather than create on the stack and copy it to .data,
// which will blow up the tiny stack.
static KERNEL_PHYSICAL_PT: RwLock<PhysicalMemory<LinkedMemoryTable>> = RwLock::new(PhysicalMemory {
    table: LinkedMemoryTable::uninitialized()
});
// Record the number of allocated pages. Just for test.
static ALLOC_PAGE_CNT: AtomicUsize = AtomicUsize::new(0);

pub extern "C" fn init_physical_page_table() {
    extern "C" {
        fn ekernel();
    }
    let mut binding = KERNEL_PHYSICAL_PT.write();
    binding.table.init(round_up(kernel2physical(ekernel as u64) as usize, PAGE_SIZE) as *mut u8,
                       PHYSICAL_TOP as *mut u8);
}

define_early_init!(init_physical_page_table);

pub fn kalloc_page() -> *mut u8 {
    ALLOC_PAGE_CNT.fetch_add(1, core::sync::atomic::Ordering::AcqRel);
    let mut binding = CONSOLE.write();
    write!(binding.as_mut().unwrap(), "kalloc_page: {} pages allocated\r", ALLOC_PAGE_CNT.load(core::sync::atomic::Ordering::Relaxed)).unwrap();
    let mut binding = KERNEL_PHYSICAL_PT.write();
    unsafe { _physical2kernel_mut(binding.table.page_alloc()) }
}

pub fn kfree_page(page_addr: *mut u8) {
    ALLOC_PAGE_CNT.fetch_sub(1, core::sync::atomic::Ordering::AcqRel);
    let mut binding = KERNEL_PHYSICAL_PT.write();
    binding.page_free(unsafe { _kernel2physical_mut(page_addr) })
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

fn align_num(size: usize) -> usize {
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

