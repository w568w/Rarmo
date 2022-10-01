extern "C" {
    fn early_init();
    fn rest_init();
    fn init();
    fn einit();
}

pub fn do_early_init() {
    let mut start = early_init as *const fn();
    let end = rest_init as *const fn();
    while start != end {
        unsafe {
            (*start)();
        }
        start = unsafe { start.add(1) };
    }
}

pub fn do_init() {
    let mut start = init as *const fn();
    let end = einit as *const fn();
    while start != end {
        unsafe {
            (*start)();
        }
        start = unsafe { start.add(1) };
    }
}