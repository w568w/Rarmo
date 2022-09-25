use core::alloc::{GlobalAlloc, Layout};
use crate::cores::slob;

struct KernelSlobAllocator;

#[global_allocator]
static KERNEL_ALLOCATOR: KernelSlobAllocator = KernelSlobAllocator;

unsafe impl GlobalAlloc for KernelSlobAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        slob::kmem_cache_alloc_node(&slob::KMemCache {
            size: layout.size(),
            align: layout.align(),
            name: "KernelAllocator",
        }).expect("Unable to allocate memory from SLOB")
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
        slob::dealloc_node(ptr);
    }
}