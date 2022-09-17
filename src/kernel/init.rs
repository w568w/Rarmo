extern "C" {
    fn early_init();
    fn rest_init();
    fn init();
    fn einit();
}

pub fn do_early_init() {
    let mut start = early_init as *const fn();
    let end = rest_init as *const fn();
    let a = super::mem::init_physical_page_table as *const fn();
    while start != end {
        unsafe {
            (*start)();
        }
        start = unsafe { start.offset(1) };
    }
}

pub fn do_init() {
    let mut start = init as *const fn();
    let end = einit as *const fn();
    while start != end {
        unsafe {
            (*start)();
        }
        start = unsafe { start.offset(1) };
    }
}

pub enum StackDirection {
    // From low address to high address.
    Up,
    // From high address to low address.
    Down,
}

pub fn check_stack_direction() -> StackDirection {
    let mut a = 0i32;
    _check_stack_direction(&mut a)
}

fn _check_stack_direction(parent: *mut i32) -> StackDirection {
    let mut b = 1i32;
    let c = &mut b as *mut i32;
    if c > parent {
        StackDirection::Up
    } else {
        StackDirection::Down
    }
}