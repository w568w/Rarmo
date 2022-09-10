use core::fmt;

use spin::Mutex;

use crate::driver::CharDevice;
use crate::UartDevice;

pub struct ConsoleContext<T>
    where
        T: CharDevice,
{
    pub lock: Mutex<u32>,
    pub device: T,
}

// Static variable for console.
pub static DEFAULT_CONSOLE: Mutex<ConsoleContext<UartDevice>> = Mutex::new(ConsoleContext {
    lock: Mutex::new(0),
    device: UartDevice,
});
impl ConsoleContext<UartDevice>
{

}


impl<T> ConsoleContext<T>
    where
        T: CharDevice,
{
    pub fn new(device: T) -> Self {
        device.init();
        Self {
            lock: Mutex::new(0),
            device,
        }
    }

    pub fn put_char(&self, chr: u8) {
        self.device.put_char(chr);
    }
}

impl<T: CharDevice> fmt::Write for ConsoleContext<T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _ = self.lock.lock();
        for byte in s.bytes() {
            self.put_char(byte);
        }
        Ok(())
    }
}
