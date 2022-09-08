use spin::{Mutex};

pub struct ConsoleContext {
    pub lock: Mutex<u32>,
}

impl ConsoleContext{
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(0),
        }
    }
}