use core::mem::MaybeUninit;

use field_offset::offset_of;

use crate::{aarch64::mmu::PAGE_SIZE, common::{
    list::{ListLink, ListNode},
    round_down,
}};
use crate::aarch64::mmu::{_kernel2physical_mut, _physical2kernel_mut};

// We will be started from up to 1024 MB memory management.
const SMALL_MEMORY_SIZE_IN_MB: usize = 1024;
const SMALL_PAGE_NUM: usize = (SMALL_MEMORY_SIZE_IN_MB * 1024 * 1024 / PAGE_SIZE) as usize;

pub struct PhysicalMemory<T>
    where
        T: PhysicalMemoryTable,
{
    pub table: T,
}

pub trait PhysicalMemoryTable {
    fn new(start: *mut u8, end: *mut u8) -> Self;
    fn page_alloc(&mut self, num: usize) -> *mut u8;
    fn page_free(&mut self, page_addr: *mut u8, num: usize);
}

// Provide proxy methods of `table` in `PhysicalMemory`.
impl<T: PhysicalMemoryTable> PhysicalMemoryTable for PhysicalMemory<T> {
    fn new(start: *mut u8, end: *mut u8) -> Self {
        Self {
            table: T::new(start, end),
        }
    }

    fn page_alloc(&mut self, num: usize) -> *mut u8 {
        self.table.page_alloc(num)
    }

    fn page_free(&mut self, page_addr: *mut u8, num: usize) {
        self.table.page_free(page_addr, num)
    }
}

const GF_ORDER: usize = 16;

#[repr(C)]
pub struct BuddyPage {
    pub link: ListLink,
}

impl ListNode for BuddyPage {
    fn get_link_offset() -> usize {
        offset_of!(BuddyPage => link).get_byte_offset()
    }
}

pub struct BuddyPageAllocation {
    free_list: [ListLink; GF_ORDER],
    start: *mut u8,
    end: *mut u8,
    page_num: usize,
    page_map: &'static mut [usize],
}

unsafe impl Send for BuddyPageAllocation {}

unsafe impl Sync for BuddyPageAllocation {}

impl BuddyPageAllocation {
    pub const fn uninitialized() -> Self {
        let mut free_list: [MaybeUninit<ListLink>; GF_ORDER] = MaybeUninit::uninit_array();

        let mut i = 0;
        while i < GF_ORDER {
            free_list[i] = MaybeUninit::new(ListLink::uninit());
            i += 1;
        }

        let alloc = Self {
            free_list: unsafe { core::mem::transmute(free_list) },
            page_map: &mut [],
            start: core::ptr::null_mut(),
            end: core::ptr::null_mut(),
            page_num: 0,
        };
        alloc
    }

    pub fn init(&mut self, start: *mut u8, end: *mut u8) {
        let start = _physical2kernel_mut(start);
        let end = _physical2kernel_mut(end);
        let total_size = unsafe { end.byte_offset_from(start) } as usize;
        let aligned_size = round_down(total_size, core::mem::align_of::<usize>());
        let bit_unit_size = core::mem::size_of::<usize>();
        // Calculate the number of pages.
        let page_num = aligned_size / (PAGE_SIZE + bit_unit_size);
        let real_start = unsafe { end.byte_sub(page_num * (PAGE_SIZE + bit_unit_size)) };

        self.page_map =
            unsafe { core::slice::from_raw_parts_mut(real_start as *mut usize, page_num) };
        // Clean the page map.
        self.page_map.iter_mut().for_each(|x| *x = GF_ORDER);
        self.start = unsafe { real_start.byte_add(page_num * bit_unit_size) };
        self.end = end;
        self.page_num = page_num;
        self.init_free_list(self.start, self.end, page_num);
    }

    fn init_free_list(&mut self, start: *mut u8, end: *mut u8, page_num: usize) {
        for i in 0..GF_ORDER {
            self.free_list[i].init();
        }
        Self::iter_by_order(start, page_num, |page, order| {
            self.insert_page(page, order);
        })
    }

    fn get_page_map_index(&self, page_addr: *mut u8) -> usize {
        unsafe { page_addr.byte_offset_from(self.start) as usize / PAGE_SIZE }
    }

    // Iterate over some continuous pages by a order split.
    fn iter_by_order<F>(start_page_addr: *mut u8, size: usize, mut f: F)
        where
            F: FnMut(&mut BuddyPage, usize),
    {
        let mut size = size;
        let mut page_addr = start_page_addr;
        let mut order = GF_ORDER - 1;
        while size > 0 {
            let page_num_in_order = 1 << order;
            if size >= page_num_in_order {
                f(unsafe { &mut *(page_addr as *mut BuddyPage) }, order);
                page_addr = unsafe { page_addr.byte_add(page_num_in_order * PAGE_SIZE) };
                size -= page_num_in_order;
            } else {
                order -= 1;
            }
        }
    }

    fn declare_page_used(&mut self, page_addr: *mut u8, size: usize) {
        let page_map_index = self.get_page_map_index(page_addr);
        for i in 0..size {
            self.page_map[page_map_index + i] = GF_ORDER;
        }
    }
    fn declare_page_free(&mut self, page_addr: *mut u8, order: usize) {
        let page_map_index = self.get_page_map_index(page_addr);
        let page_num = 1 << order;
        for i in 0..page_num {
            self.page_map[page_map_index + i] = order;
        }
    }

    // Insert a page into the free list. Merge the page with its buddy if possible.
    fn insert_page(&mut self, page: &mut BuddyPage, order: usize) {
        page.link().init();
        let page_addr = page as *mut _ as *mut u8;
        self.declare_page_free(page_addr, order);
        self.free_list[order].insert_at_first(page);
        if order >= GF_ORDER - 1 {
            return;
        }
        let prev_page_addr = unsafe { page_addr.byte_sub(PAGE_SIZE * (1 << order)) };
        let next_page_addr = unsafe { page_addr.byte_add(PAGE_SIZE * (1 << order)) };
        if prev_page_addr >= self.start {
            let prev_page = unsafe { &mut *(prev_page_addr as *mut BuddyPage) };
            if self.page_map[self.get_page_map_index(prev_page_addr)] == order {
                page.link().detach();
                prev_page.link().detach();
                self.insert_page(prev_page, order + 1);
                return;
            }
        }

        if next_page_addr < self.end {
            if self.page_map[self.get_page_map_index(next_page_addr)] == order {
                let next_page = unsafe { &mut *(next_page_addr as *mut BuddyPage) };
                next_page.link().detach();
                page.link().detach();
                self.insert_page(page, order + 1);
                return;
            }
        }
    }

    // Split a page into two pages.
    // The first part should have been removed from the free list.
    fn split_page(&mut self, page_addr: *mut u8, order: usize, target_size: usize) {
        let free_pages_start = unsafe { page_addr.byte_add(PAGE_SIZE * target_size) };
        let free_size = (1usize << order) - target_size;
        Self::iter_by_order(free_pages_start, free_size, |page, order| {
            self.insert_page(page, order);
        });
    }

    fn alloc_page(&mut self, page_num: usize) -> Option<*mut u8> {
        if page_num > 1 << (GF_ORDER - 1) || page_num < 1 {
            return None;
        }
        // get the minimum feasible order for page_num.
        let mut order = 0;
        while (1 << order) < page_num {
            order += 1;
        }

        // Try each order for a free buddy.
        while order < GF_ORDER {
            let page = self.free_list[order].prev::<BuddyPage>();
            if let Some(page) = page {
                let page_addr = page as *mut BuddyPage as *mut u8;
                page.link().detach();
                self.declare_page_used(page_addr, 1 << order);
                if order > 0 {
                    self.split_page(page_addr, order, page_num);
                }
                return Some(_kernel2physical_mut(page_addr));
            }
            order += 1;
        }
        None
    }
}

impl PhysicalMemoryTable for BuddyPageAllocation {
    fn new(start: *mut u8, end: *mut u8) -> Self {
        let mut table = BuddyPageAllocation::uninitialized();
        table.init(start, end);
        table
    }

    fn page_alloc(&mut self, num: usize) -> *mut u8 {
        self.alloc_page(num).unwrap()
    }

    fn page_free(&mut self, page_addr: *mut u8, num: usize) {
        let page_addr = _physical2kernel_mut(page_addr);
        BuddyPageAllocation::iter_by_order(page_addr, num, |page, order| {
            self.insert_page(page, order);
        });
    }
}
