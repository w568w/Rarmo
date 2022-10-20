use core::mem::{MaybeUninit, size_of};
use field_offset::offset_of;
use crate::{aarch64::mmu::PAGE_SIZE, common::round_down};
use crate::aarch64::mmu::{_kernel2physical_mut, _physical2kernel_mut};
use crate::common::buddy::RawBuddies;
use crate::common::{round_up, round_up_to_2n};
use crate::common::list::{ListLink, ListNode};

pub struct PhysicalMemory<T>
    where
        T: PhysicalMemoryTable,
{
    pub table: T,
}

pub trait PhysicalMemoryTable {
    fn page_alloc(&mut self, num: usize) -> *mut u8;
    fn page_free(&mut self, page_addr: *mut u8, num: usize);
}

// Provide proxy methods of `table` in `PhysicalMemory`.
impl<T: PhysicalMemoryTable> PhysicalMemoryTable for PhysicalMemory<T> {

    fn page_alloc(&mut self, num: usize) -> *mut u8 {
        self.table.page_alloc(num)
    }

    fn page_free(&mut self, page_addr: *mut u8, num: usize) {
        self.table.page_free(page_addr, num)
    }
}

#[repr(C)]
struct Page {
    buddy_link: ListLink,
    _padding: [u8; PAGE_SIZE - size_of::<ListLink>()],
}

impl ListNode<ListLink> for Page {
    fn get_link_offset() -> usize { offset_of!(Page => buddy_link).get_byte_offset() }
}

pub struct BuddyPageAllocation {
    buddy: MaybeUninit<RawBuddies<Page>>,
}

unsafe impl Send for BuddyPageAllocation {}

unsafe impl Sync for BuddyPageAllocation {}

impl BuddyPageAllocation {
    pub const fn uninitialized() -> Self {
        Self {
            buddy: MaybeUninit::uninit(),
        }
    }

    pub fn init(&mut self, start: *mut u8, end: *mut u8) {
        let start = round_up(start as usize, PAGE_SIZE) as usize;
        let end = round_down(end as usize, PAGE_SIZE) as usize;
        let len = (end - start) * 8 / (PAGE_SIZE * 8 + 1);
        let len = len * PAGE_SIZE;
        let mut highest_order = 8 * size_of::<usize>() - len.leading_zeros() as usize + 1;
        while round_down(end, 1 << highest_order) <= start ||
            round_down(end, 1 << highest_order) - (1 << highest_order) <= start {
            if highest_order <= 12 {
                panic!("Not enough memory for buddy system");
            }
            highest_order -= 1;
        }
        let real_end = round_down(end, 1 << highest_order);
        while real_end >= (1 << highest_order) && real_end - (1 << highest_order) >= start {
            highest_order += 1;
        }
        let real_start = real_end - (1 << (highest_order - 1));
        self.buddy = MaybeUninit::new(RawBuddies::uninit(
            highest_order - 12,
            _physical2kernel_mut(real_start as *mut Page),
            _physical2kernel_mut(start as *mut u8),
        ));
        unsafe {
            self.buddy.assume_init_mut().init();
        }
    }
}

impl PhysicalMemoryTable for BuddyPageAllocation {

    fn page_alloc(&mut self, num: usize) -> *mut u8 {
        let buddy = unsafe { self.buddy.assume_init_mut() };
        let highest_order = round_up_to_2n(num);
        let (ret, _) = buddy.allocate(highest_order as usize).unwrap();
        _kernel2physical_mut(ret as *mut u8)
    }

    fn page_free(&mut self, page_addr: *mut u8, num: usize) {
        let page_addr = _physical2kernel_mut(page_addr);
        let highest_order = round_up_to_2n(num);
        let buddy = unsafe { self.buddy.assume_init_mut() };
        buddy.free(highest_order as usize, buddy.pos(highest_order as usize, page_addr as *mut Page));
    }
}