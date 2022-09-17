use core::mem::size_of;
use core::ptr;
use spin::Mutex;
use crate::aarch64::mmu::PAGE_SIZE;
use crate::common::round_up;
use crate::kernel::mem::kalloc_page;

// todo: the type should be decided by page size.
pub type SlobUnit = i16;

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

const SLOB_BREAK1: usize = 256;
const SLOB_BREAK2: usize = 1024;

static mut free_slob_small: SlobPageList = SlobPageList { prev: None, next: None };
static mut free_slob_medium: SlobPageList = SlobPageList { prev: None, next: None };
static mut free_slob_large: SlobPageList = SlobPageList { prev: None, next: None };
static SLOB_LOCK: Mutex<()> = Mutex::new(());

pub struct KMemCache {
    pub name: &'static str,
    pub size: usize,
    pub align: usize,
}


pub struct SlobPageList {
    pub prev: Option<*mut SlobPage>,
    pub next: Option<*mut SlobPage>,
}

impl SlobPageList {
    pub unsafe fn add_free_page(&mut self, page: *mut SlobPage) {
        let mut page = page;
        (*page).list.prev = None;
        if let Some(original_next) = self.next {
            (*original_next).list.prev = Some(page);
        }
        (*page).list.next = self.next;
        self.next = Some(page);
    }
    pub unsafe fn detach_self(&mut self) {
        if let Some(prev) = self.prev {
            (*prev).list.next = self.next;
        }
        if let Some(next) = self.next {
            (*next).list.prev = self.prev;
        }
        // todo: what if `self` is the first element?
    }
}

pub struct SlobPage {
    pub free_units: SlobUnit,
    pub free: *mut SlobUnit,
    pub list: SlobPageList,
}

impl SlobPage {
    pub fn is_last(page_addr:*mut SlobPage, obj: *mut SlobUnit) -> bool {
        let addr = unsafe { slob_next(obj) } as usize;
        let end = unsafe { page_addr.byte_offset(PAGE_SIZE as isize) } as usize;
        addr >= end
    }
}

pub struct SlobBlock {
    pub unit: SlobUnit,
}

pub fn kmem_cache_alloc_node(cache: &KMemCache) -> Option<*mut u8> {
    if cache.size < PAGE_FREE_SIZE {
        unsafe { slob_alloc(cache.size, cache.align) }
    } else {
        todo!("We don't support large object allocation yet.")
    }
}

unsafe fn slob_alloc(size: usize, align: usize) -> Option<*mut u8> {
    // Since we... (see below)
    let slob_list =
        if size <= SLOB_BREAK1 {
            &mut free_slob_small
        } else if size <= SLOB_BREAK2 {
            &mut free_slob_medium
        } else {
            &mut free_slob_large
        };
    let mut slob_pointer = slob_list as *mut SlobPageList;
    // ... lock it here, the above code is thread safe!
    let lock = SLOB_LOCK.lock();
    let mut block: Option<*mut u8> = None;
    while let Some(page) = (*slob_pointer).next {
        slob_pointer = &mut (*page).list as *mut SlobPageList;
        if (*page).free_units < need_units(size) {
            continue;
        }
        // let prev = (*page).list.prev.expect("Slob page list is corrupted. Some page does not have a prev pointer!");
        block = slob_page_alloc(page, size, align);
        if block.is_none() {
            continue;
        }
        // todo: Improve fragment distribution and reduce our average search time by starting our next search here.
    }
    drop(lock);
    if block.is_none() {
        let _ = SLOB_LOCK.lock();
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
    let first_free = b.offset(size_of::<SlobPage>() as isize) as *mut SlobUnit;
    set_slob(first_free, (*page).free_units, first_free.byte_offset(PAGE_FREE_SIZE as isize));
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
                avail =slob_block_size(cur);
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
                (*page).list.detach_self();
            }
            // We should return the address of the second unit of the block,
            // because first unit is used to record the block's size.
            return Some(cur.offset(1) as *mut u8);
        }
        // If we have gone over the whole page, we should return None.
        if SlobPage::is_last(page,cur) {
            return None;
        } else {
            prev = cur;
            cur = slob_next(cur);
        }
    }
}

// Align so that `returned value + 1` is a multiple of `align`.
fn align_up(x: *mut SlobUnit, align: usize) -> *mut SlobUnit {
    let aligned = round_up(unsafe { x.offset(1) } as usize, align) as *mut SlobUnit;
    unsafe { aligned.offset(-1) }
}

unsafe fn slob_next(cur: *mut SlobUnit) -> *mut SlobUnit {
    let avail = cur.read();
    if avail >= 0 {
        let avail = cur.offset(1).read();
        cur.offset(avail as isize)
    } else {
        cur.offset((-avail) as isize)
    }
}

fn slob_block_size(cur: *mut SlobUnit) -> SlobUnit {
    let avail = unsafe { cur.read() };
    if avail >= 0 {
        avail
    } else {
        1
    }
}