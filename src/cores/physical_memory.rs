use crate::aarch64::mmu::PAGE_SIZE;

// We will be started from up to 1024 MB memory management.
const SMALL_MEMORY_SIZE_IN_MB: usize = 1024;
const SMALL_PAGE_NUM: usize = (SMALL_MEMORY_SIZE_IN_MB * 1024 * 1024 / PAGE_SIZE) as usize;

pub struct PhysicalMemory<T>
    where T: PhysicalMemoryTable {
    pub table: T,
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
    free_page_stack: [usize; SMALL_PAGE_NUM],
    stack_top: usize,
}

impl LinkedMemoryTable {
    // Use to allocate a `LinkedMemoryTable` in the `.data` section.
    pub const fn uninitialized() -> Self {
        LinkedMemoryTable {
            free_page_stack: [0; SMALL_PAGE_NUM],
            stack_top: 0,
        }
    }
    pub fn init(&mut self, start: *mut u8, end: *mut u8) {
        self.stack_top = 0usize;
        let mut page_addr = start;
        while page_addr < end && self.stack_top < SMALL_PAGE_NUM {
            self.free_page_stack[self.stack_top] = page_addr as usize;
            page_addr = unsafe { page_addr.offset(PAGE_SIZE as isize) };
            self.stack_top += 1;
        }
    }
}

impl PhysicalMemoryTable for LinkedMemoryTable {
    fn new(start: *mut u8, end: *mut u8) -> Self {
        let mut table = LinkedMemoryTable::uninitialized();
        table.init(start, end);
        table
    }

    fn page_alloc(&mut self) -> *mut u8 {
        if self.stack_top == 0 {
            panic!("No more physical memory!");
        }
        self.stack_top -= 1;
        self.free_page_stack[self.stack_top] as *mut u8
    }

    fn page_free(&mut self, page_addr: *mut u8) {
        if self.stack_top == SMALL_PAGE_NUM {
            panic!("Full page table, you may be inserting a freed page!");
        }
        self.free_page_stack[self.stack_top] = page_addr as usize;
        self.stack_top += 1;
    }
}