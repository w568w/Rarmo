use crate::aarch64::mmu::PAGE_SIZE;

// We will be started from 128 MB memory management.
const SMALL_MEMORY_SIZE_IN_MB: usize = 128;
const SMALL_PAGE_NUM: usize = (SMALL_MEMORY_SIZE_IN_MB * 1024 * 1024 / PAGE_SIZE) as usize;

pub struct PhysicalMemory<T>
    where T: PhysicalMemoryTable {
    // lock: Mutex<u32>,
    table: T,
}

pub trait PhysicalMemoryTable {
    fn new(start: *mut u8, end: *mut u8) -> Self;
    fn page_alloc(&mut self) -> *mut u8;
    fn page_free(&mut self, page_addr: *mut u8);
}

// Provide proxy methods of `table` in `PhysicalMemory`.
impl<T: PhysicalMemoryTable> PhysicalMemoryTable for PhysicalMemory<T> {
    fn new(start: *mut u8, end: *mut u8) -> Self {
        PhysicalMemory {
            table: T::new(start, end)
        }
    }

    fn page_alloc(&mut self) -> *mut u8 {
        self.table.page_alloc()
    }

    fn page_free(&mut self, page_addr: *mut u8) {
        self.table.page_free(page_addr)
    }
}

pub struct LinkedMemoryTable {
    free_page_stack: [*mut u8; SMALL_PAGE_NUM],
    stack_top: usize,
}

impl PhysicalMemoryTable for LinkedMemoryTable {
    fn new(start: *mut u8, end: *mut u8) -> Self {
        let mut free_page_stack = [0 as *mut u8; SMALL_PAGE_NUM];
        let mut stack_top = 0usize;
        let mut page_addr = start;
        while page_addr < end {
            free_page_stack[stack_top] = page_addr;
            page_addr = unsafe { page_addr.offset(PAGE_SIZE as isize) };
            stack_top += 1;
        }
        Self { free_page_stack, stack_top }
    }

    fn page_alloc(&mut self) -> *mut u8 {
        if self.stack_top == 0 {
            panic!("No more physical memory!");
        }
        self.stack_top -= 1;
        self.free_page_stack[self.stack_top]
    }

    fn page_free(&mut self, page_addr: *mut u8) {
        if self.stack_top == SMALL_PAGE_NUM {
            panic!("Full page table, you may be inserting a freed page!");
        }
        self.free_page_stack[self.stack_top] = page_addr;
        self.stack_top += 1;
    }
}