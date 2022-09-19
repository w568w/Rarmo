use core::mem::size_of;
use core::ptr;
use crate::aarch64::mmu::PAGE_SIZE;
use crate::common::{padding, round_up};
use crate::get_cpu_id;
use crate::kernel::mem::{kalloc_page, kfree_page};

/**
 * A simple SLOB implementation.
 * The design is based on the Linux kernel's.
 *
 * Ref:
 * https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/mm/slob.c?h=linux-2.6.33.y
 */

// todo: the type should be decided by page size.
type SlobUnit = i16;

const UNIT_SIZE: usize = size_of::<SlobUnit>();

const PAGE_FREE_SIZE: usize = PAGE_SIZE - size_of::<SlobPage>();

pub const fn need_units(x: usize) -> SlobUnit {
    if x % UNIT_SIZE == 0 {
        (x / UNIT_SIZE + 1) as SlobUnit
    } else {
        (x / UNIT_SIZE + 2) as SlobUnit
    }
}

pub const fn contain_units(x: usize) -> SlobUnit {
    (x / UNIT_SIZE) as SlobUnit
}

const SLOB_BREAK1: usize = 64;
const SLOB_BREAK2: usize = 256;

static mut FREE_SLOB_SMALL: [SlobPageList; 4] = [SlobPageList { prev: None, next: None }; 4];
static mut FREE_SLOB_MEDIUM: [SlobPageList; 4] = [SlobPageList { prev: None, next: None }; 4];
static mut FREE_SLOB_LARGE: [SlobPageList; 4] = [SlobPageList { prev: None, next: None }; 4];


pub struct KMemCache {
    pub name: &'static str,
    pub size: usize,
    pub align: usize,
}

#[derive(Clone, Copy)]
pub struct SlobPageList {
    pub prev: Option<*mut SlobPage>,
    pub next: Option<*mut SlobPage>,
}

impl SlobPageList {
    pub unsafe fn add_free_page(&mut self, page: *mut SlobPage) {
        (*page).list.prev = None;
        if let Some(original_next) = self.next {
            (*original_next).list.prev = Some(page);
        }
        (*page).list.next = self.next;
        self.next = Some(page);
    }
}


pub struct SlobPage {
    pub free_units: SlobUnit,
    pub free: *mut SlobUnit,
    pub list: SlobPageList,
}

impl SlobPage {
    pub fn is_last(page_addr: *mut SlobPage, obj: *mut SlobUnit) -> bool {
        let addr = unsafe { slob_next(obj) } as usize;
        let end = unsafe { page_addr.byte_offset(PAGE_SIZE as isize) } as usize;
        addr >= end
    }
    pub unsafe fn detach_self(page: *mut SlobPage) {
        if let Some(prev) = (*page).list.prev {
            (*prev).list.next = (*page).list.next;
        }
        if let Some(next) = (*page).list.next {
            (*next).list.prev = (*page).list.prev;
        }
        let mut a = [&mut FREE_SLOB_SMALL[get_cpu_id()], &mut FREE_SLOB_MEDIUM[get_cpu_id()], &mut FREE_SLOB_LARGE[get_cpu_id()]];
        for i in a.iter_mut() {
            if let Some(next) = i.next {
                if next == page {
                    i.next = (*page).list.next;
                }
            }
        }
        (*page).list.prev = None;
        (*page).list.next = None;
    }
}


pub fn kmem_cache_alloc_node(cache: &KMemCache) -> Option<*mut u8> {
    if cache.size < PAGE_FREE_SIZE {
        unsafe { slob_alloc(cache.size, cache.align) }
    } else {
        todo!()
    }
}

pub fn dealloc_node(block: *mut u8) -> usize {
    unsafe {
        let start_of_block = (block as *mut SlobUnit).offset(-1);
        slob_free(start_of_block, *start_of_block) as usize
    }
}

unsafe fn slob_alloc(size: usize, align: usize) -> Option<*mut u8> {
    // We do not need a lock mechanism here, since we will allocate the page for each CPU hart.
    let slob_list =
        if size <= SLOB_BREAK1 {
            &mut FREE_SLOB_SMALL[get_cpu_id()]
        } else if size <= SLOB_BREAK2 {
            &mut FREE_SLOB_MEDIUM[get_cpu_id()]
        } else {
            &mut FREE_SLOB_LARGE[get_cpu_id()]
        };
    let mut slob_pointer = slob_list as *mut SlobPageList;
    let mut block: Option<*mut u8> = None;
    while let Some(page) = (*slob_pointer).next {
        slob_pointer = &mut (*page).list as *mut SlobPageList;
        if (*page).free_units < need_units(size) {
            continue;
        }
        block = slob_page_alloc(page, size, align);
        if block.is_none() {
            continue;
        }
        if slob_list.next != Some(page) {
            if let Some(first_page) = slob_list.next {
                if (*first_page).free_units < (*page).free_units {
                    SlobPage::detach_self(page);
                    slob_list.add_free_page(page);
                }
            }
        }
        break;
    }
    if block.is_none() {
        let page = slob_new_pages();
        slob_list.add_free_page(page);
        block = slob_page_alloc(page, size, align);
    }
    block
}

// Allocate a new page, but do not add it to the free list.
unsafe fn slob_new_pages() -> *mut SlobPage {
    let b = kalloc_page();
    let page = b as *mut SlobPage;
    (*page).free_units = contain_units(PAGE_FREE_SIZE);
    let first_free = b.add(size_of::<SlobPage>()) as *mut SlobUnit;
    set_slob(first_free, (*page).free_units, first_free.byte_add(PAGE_FREE_SIZE));
    (*page).free = first_free;
    (*page).list = SlobPageList { prev: None, next: None };
    page
}

unsafe fn set_slob(unit: *mut SlobUnit, unit_size: SlobUnit, next_free: *mut SlobUnit) {
    let offset = next_free.offset_from(unit);
    if offset < 0 {
        panic!("Illegal arguments. The next free unit is before the current unit!");
    }
    if unit_size > 1 {
        unit.write(unit_size);
        unit.offset(1).write(offset as SlobUnit);
    } else {
        unit.write(-(offset as SlobUnit));
    }
}

unsafe fn slob_page_alloc(page: *mut SlobPage, size: usize, align: usize) -> Option<*mut u8> {
    let mut prev: *mut SlobUnit = ptr::null_mut();
    let mut cur = (*page).free;
    let mut aligned: *mut SlobUnit = ptr::null_mut();
    let mut delta: isize = 0;
    let units = need_units(size);
    loop {
        let mut avail = slob_block_size(cur);
        if align > 0 {
            aligned = align_up(cur, align);
            delta = aligned.offset_from(cur);
        }
        if avail >= units + delta as SlobUnit {
            let mut next: *mut SlobUnit = ptr::null_mut();
            // If we really need to align...
            if delta != 0 {
                next = slob_next(cur);
                // ... we need to split the current block into two ...
                // From:
                // [cur                 ] -> [next     ]
                // To:
                // [cur  ] -> [aligned  ] -> [next     ]
                set_slob(cur, delta as SlobUnit, aligned);
                set_slob(aligned, avail - delta as SlobUnit, next);
                // ... and tell the following codes to look at the aligned block first!
                prev = cur;
                cur = aligned;
                avail = slob_block_size(cur);
            }
            next = slob_next(cur);
            if avail == units {
                // If the current block is exactly the size we need, we can just use it.

                // Let the previous block point to the next block.
                if prev.is_null() {
                    (*page).free = next;
                } else {
                    set_slob(prev, slob_block_size(prev), next);
                }
            } else {
                // Or, we need to split the current block into two and let the previous block point to its second part.
                if prev.is_null() {
                    (*page).free = cur.offset(units as isize);
                } else {
                    set_slob(prev, slob_block_size(prev), cur.offset(units as isize));
                }
                // The second part will point to the next block.
                set_slob(cur.offset(units as isize), avail - units, next);
            }
            // Update the free units we have.
            (*page).free_units -= units;
            if (*page).free_units == 0 {
                SlobPage::detach_self(page);
            }
            cur.write(units);
            // We should return the address of the second unit of the block,
            // because first unit is used to record the block's size.
            return Some(cur.offset(1) as *mut u8);
        }
        // If we have gone over the whole page, we should return None.
        if SlobPage::is_last(page, cur) {
            return None;
        } else {
            prev = cur;
            cur = slob_next(cur);
        }
    }
}

unsafe fn slob_free(block: *mut SlobUnit, size: SlobUnit) -> SlobUnit {
    let original_size = size;
    let mut size = size;
    let page = slob_page(block);
    let mut prev: *mut SlobUnit = ptr::null_mut();
    let mut next: *mut SlobUnit = ptr::null_mut();
    if (*page).free_units + size >= contain_units(PAGE_FREE_SIZE) {
        SlobPage::detach_self(page);
        kfree_page(page as *mut u8);
        return original_size;
    }
    if (*page).free_units == 0 {
        // if the page is full, it is so easy to free the block.
        (*page).free = block;
        (*page).free_units = size;
        set_slob(block, size, page.byte_add(PAGE_SIZE) as *mut SlobUnit);
        // todo: if we drop full page before, add it to the free list again here.
        return original_size;
    }
    // Otherwise, the page is not full, we need to find the right place to insert the block, and merge it with its neighbors.
    // Be ready to get your hands dirty! We will deal with two conditions:
    (*page).free_units += size;
    if block < (*page).free {
        // 1, if the block is before the first free block:
        if block.offset(size as isize) == (*page).free {
            // if the block is adjacent to the first free block, we can merge them
            // by increasing the size of our `block`.
            size += slob_block_size((*page).free);
            (*page).free = slob_next((*page).free);
        }
        // then set the first free block of the page to be our `block`.
        set_slob(block, size, (*page).free);
        (*page).free = block;
    } else {
        // 2, or, the block is after the first free block:
        // find the right place to insert the block.
        prev = (*page).free;
        next = slob_next(prev);
        while block > next {
            prev = next;
            next = slob_next(prev);
        }
        // now, we have: prev < block < next,
        // so insert `block` between them.

        // Deal with `next`:
        if !SlobPage::is_last(page, prev) && block.offset(size as isize) == next {
            // if prev is not the last block, and the block is adjacent to the next block,
            // merge `block` with `next`.
            size += slob_block_size(next);
            set_slob(block, size, slob_next(next));
        } else {
            // or, just set the next block of `block` to be `next`.
            set_slob(block, size, next);
        }

        // Deal with `prev`:
        if prev.offset(slob_block_size(prev) as isize) == block {
            // if the block is adjacent to the `previous` block,
            // merge `prev` with `block`.
            size = slob_block_size(prev) + slob_block_size(block);
            set_slob(prev, size, slob_next(block));
        } else {
            // or, just set the next block of `prev` to be `block`.
            set_slob(prev, slob_block_size(prev), block);
        }
    }
    original_size
}

fn slob_page(block: *mut SlobUnit) -> *mut SlobPage {
    let page = block as usize & !(PAGE_SIZE - 1);
    page as *mut SlobPage
}

// Calculate aligned block, so that `returned value + 1` is a multiple of `align`.
fn align_up(x: *mut SlobUnit, align: usize) -> *mut SlobUnit {
    let aligned = round_up(unsafe { x.offset(1) } as usize, align) as *mut SlobUnit;
    unsafe { aligned.offset(-1) }
}

// Get the next block of `cur`.
unsafe fn slob_next(cur: *mut SlobUnit) -> *mut SlobUnit {
    let avail = cur.read();
    if avail >= 0 {
        let avail = cur.offset(1).read();
        cur.offset(avail as isize)
    } else {
        cur.offset((-avail) as isize)
    }
}

// Get the block size of `cur`.
fn slob_block_size(cur: *mut SlobUnit) -> SlobUnit {
    let avail = unsafe { cur.read() };
    if avail >= 0 {
        avail
    } else {
        1
    }
}
