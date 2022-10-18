use core::mem::{MaybeUninit, size_of};
use crate::{aarch64::mmu::PAGE_SIZE, common::round_down};
use crate::aarch64::mmu::_physical2kernel_mut;
use crate::common::buddy::RawBuddies;
use crate::common::{round_up, round_up_to_2n};

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

struct Page([u8; PAGE_SIZE]);

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
        self.buddy = MaybeUninit::new(RawBuddies::new(
            highest_order - 12,
            real_start as *mut Page,
            _physical2kernel_mut(start as *mut u8),
        ));
    }
}

impl PhysicalMemoryTable for BuddyPageAllocation {
    fn new(start: *mut u8, end: *mut u8) -> Self {
        let mut table = BuddyPageAllocation::uninitialized();
        table.init(start, end);
        table
    }

    fn page_alloc(&mut self, num: usize) -> *mut u8 {
        let buddy = unsafe { self.buddy.assume_init_mut() };
        let highest_order = round_up_to_2n(num);
        let (ret, _) = buddy.allocate(highest_order as usize).unwrap();
        ret as *mut u8
    }

    fn page_free(&mut self, page_addr: *mut u8, num: usize) {
        let highest_order = round_up_to_2n(num);
        let buddy = unsafe { self.buddy.assume_init_mut() };
        buddy.free(highest_order as usize, buddy.pos(highest_order as usize, page_addr as *mut Page));
    }
}