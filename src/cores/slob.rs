use crate::aarch64::mmu::PAGE_SIZE;
use crate::common::round_up;
use crate::get_cpu_id;
use crate::kernel::mem::{kalloc_page, kfree_page};
use core::mem::size_of;
use core::ptr;
use spin::Mutex;
use crate::common::list::ListNode;

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

// How many free units in a page?
const PAGE_FREE_SIZE: usize = PAGE_SIZE - size_of::<SlobPage>();

// To allocate a `x` bytes memory, how many units do we need?
pub const fn need_units(x: usize) -> SlobUnit {
    if x % UNIT_SIZE == 0 {
        (x / UNIT_SIZE + 1) as SlobUnit
    } else {
        (x / UNIT_SIZE + 2) as SlobUnit
    }
}

// How many units can be allocated within `x` bytes?
pub const fn contain_units(x: usize) -> SlobUnit {
    (x / UNIT_SIZE) as SlobUnit
}

// How many bytes does a block of `x` units contain?
pub const fn unit_to_size(x: SlobUnit) -> usize {
    (x - 1) as usize * UNIT_SIZE
}

const SLOB_BREAK1: usize = 64;
const SLOB_BREAK2: usize = 256;

static SLOB_LOCK: Mutex<()> = Mutex::new(());
// The list heads of the SLOB page list.
// We will maintain three lists for each CPU hart.
static mut FREE_SLOB_SMALL: SlobPageList = SlobPageList {
    prev: None,
    next: None,
};
static mut FREE_SLOB_MEDIUM: SlobPageList = SlobPageList {
    prev: None,
    next: None,
};
static mut FREE_SLOB_LARGE: SlobPageList = SlobPageList {
    prev: None,
    next: None,
};

pub struct KMemCache {
    pub name: &'static str,
    pub size: usize,
    pub align: usize,
}

// A structure to describe a linked-list node.
type SlobPageList = ListNode<SlobPage>;

impl SlobPageList {
    // Add a page to the head of this list.
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
    // How many free units are left on the page?
    pub free_units: SlobUnit,
    // Point to the first free block on the page.
    pub free: *mut SlobUnit,
    // The list node of this page.
    pub list: SlobPageList,
    // The max units that can be allocated on this page.
    // It can be `None` if we don't know the max unit exactly.
    pub max_free: Option<SlobUnit>,
}

impl SlobPage {
    // Is the `obj` the last object on the `page_addr`?
    pub fn is_last(page_addr: *mut SlobPage, obj: *mut SlobUnit) -> bool {
        let addr = unsafe { slob_next(obj) } as usize;
        let end = unsafe { page_addr.byte_add(PAGE_SIZE) } as usize;
        addr >= end
    }
    // Detach the `page` from the list.
    pub unsafe fn detach_self(page: *mut SlobPage) {
        if let Some(prev) = (*page).list.prev {
            (*prev).list.next = (*page).list.next;
        }
        if let Some(next) = (*page).list.next {
            (*next).list.prev = (*page).list.prev;
        }
        // Special case: if the page is the first one in the list,
        // we should update the list head.
        let mut slob_lists = [
            &mut FREE_SLOB_SMALL,
            &mut FREE_SLOB_MEDIUM,
            &mut FREE_SLOB_LARGE,
        ];
        for slob_list in slob_lists.iter_mut() {
            if let Some(next) = slob_list.next {
                if next == page {
                    slob_list.next = (*page).list.next;
                }
            }
        }

        (*page).list.prev = None;
        (*page).list.next = None;
    }
}

pub fn kmem_cache_alloc_node(cache: &KMemCache) -> Option<*mut u8> {
    // We can only allocate a size smaller than `PAGE_FREE_SIZE`.
    if cache.size < PAGE_FREE_SIZE {
        unsafe { slob_alloc(cache.size, cache.align) }
    } else {
        todo!()
    }
}

pub fn dealloc_node(block: *mut u8) -> usize {
    unsafe {
        // `block - 1` is the start of the block.
        let start_of_block = (block as *mut SlobUnit).offset(-1);
        slob_free(start_of_block, *start_of_block) as usize
    }
}

// ==================================
// SLOB internal functions
//
// Most of them are unsafe. Be ready to get your hands dirty with raw pointers.
// ==================================

// Choose the appropriate list to allocate from.
unsafe fn select_slob_list(size: usize) -> *mut SlobPageList {
    let a = if size <= SLOB_BREAK1 {
        &mut FREE_SLOB_SMALL
    } else if size <= SLOB_BREAK2 {
        &mut FREE_SLOB_MEDIUM
    } else {
        &mut FREE_SLOB_LARGE
    };
    a as *mut SlobPageList
}

unsafe fn slob_alloc(size: usize, align: usize) -> Option<*mut u8> {
    let lock = SLOB_LOCK.lock();
    // We do not need a lock mechanism here, since we will allocate the page for each CPU hart.
    let slob_list = select_slob_list(size);
    let mut slob_pointer = slob_list;
    let mut block: Option<*mut u8> = None;
    // Walk through the slob_list to find a suitable page.
    while let Some(page) = (*slob_pointer).next {
        slob_pointer = &mut (*page).list as *mut SlobPageList;

        // If the page has less space than the requested size, we will skip it...
        if (*page).free_units < need_units(size) {
            continue;
        }
        if let Some(max_size) = (*page).max_free {
            if max_size < need_units(size) {
                continue;
            }
        }
        // ... or, we will try to allocate from this page.
        block = slob_page_alloc(page, size, align);
        // Cannot find a suitable block on this page? We will try the next one.
        if block.is_none() {
            continue;
        }
        // A optimization: if the page has more free space than the first page in the list,
        // we will move it to the head of the list.
        if (*slob_list).next != Some(page) {
            if let Some(first_page) = (*slob_list).next {
                if (*first_page).free_units < (*page).free_units {
                    SlobPage::detach_self(page);
                    (*slob_list).add_free_page(page);
                }
            }
        }
        break;
    }
    drop(lock);
    // If we still cannot find a suitable page, we will allocate a new one.
    if block.is_none() {
        let _lock = SLOB_LOCK.lock();
        let page = slob_new_pages();
        (*slob_list).add_free_page(page);
        block = slob_page_alloc(page, size, align);
    }
    block
}

// Allocate a new page, but do not add it to the free list.
unsafe fn slob_new_pages() -> *mut SlobPage {
    let b = kalloc_page();
    let page = b as *mut SlobPage;
    (*page).free_units = contain_units(PAGE_FREE_SIZE);
    (*page).max_free = Some((*page).free_units);
    let first_free = b.add(size_of::<SlobPage>()) as *mut SlobUnit;
    set_slob(
        first_free,
        (*page).free_units,
        first_free.byte_add(PAGE_FREE_SIZE),
    );
    (*page).free = first_free;
    (*page).list = SlobPageList {
        prev: None,
        next: None,
    };
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
    let mut max_block_size: SlobUnit = 0;
    // Walk through the free list to find a suitable block.
    loop {
        let mut avail = slob_block_size(cur);
        max_block_size = max_block_size.max(avail);
        // Calculate the alignment offset, if needed.
        if align > 0 {
            aligned = align_up(cur, align);
            delta = aligned.offset_from(cur);
        }
        // If this block has enough space, we will allocate on it.
        if avail >= units + delta as SlobUnit {
            let mut next: *mut SlobUnit;
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
                // The second part of the block will point to the next block.
                set_slob(cur.offset(units as isize), avail - units, next);
            }
            // Update the free units we have on the page.
            (*page).free_units -= units;
            if (*page).free_units == 0 {
                SlobPage::detach_self(page);
            }
            cur.write(units);

            // Since we have not gone over the whole page, we cannot update the max free units
            // with `max_block_size` here. So we need to invalidate the max free units.
            (*page).max_free = None;
            // We should return the address of the second unit of the block,
            // because first unit has been used to record the block's size.
            return Some(cur.offset(1) as *mut u8);
        }
        // If we have gone over the whole page but still cannot find a suitable block, we should return None.
        if SlobPage::is_last(page, cur) {
            // Since we have gone over the whole page, we can update the max free units here.
            (*page).max_free = Some(max_block_size);
            return None;
        } else {
            // We will try the next block.
            prev = cur;
            cur = slob_next(cur);
        }
    }
}

unsafe fn slob_free(block: *mut SlobUnit, size: SlobUnit) -> SlobUnit {
    let original_size = size;
    let mut size = size;
    let _lock = SLOB_LOCK.lock();
    let page = slob_page(block);
    // If the page will be empty after this free, we should remove it from the free list and
    // free the page by page allocator.
    if (*page).free_units + size >= contain_units(PAGE_FREE_SIZE) {
        SlobPage::detach_self(page);
        kfree_page(page as *mut u8);
        return original_size;
    }
    if (*page).free_units == 0 {
        // if the page is full, it is so easy to free the block.
        (*page).free = block;
        (*page).free_units = size;
        (*page).max_free = Some(size);
        set_slob(block, size, page.byte_add(PAGE_SIZE) as *mut SlobUnit);
        let slob_list = select_slob_list(unit_to_size(size));
        (*slob_list).add_free_page(page);
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
        let mut prev = (*page).free;
        let mut next = slob_next(prev);
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
    // We have changed some blocks, so we need to invalidate the max free units.
    (*page).max_free = None;
    original_size
}

// Get the page that contains the block.
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
