use core::fmt;

use spin::{Mutex, RwLock};

use crate::driver::CharDevice;
use crate::{define_early_init, UartDevice};

pub static CONSOLE: RwLock<Option<ConsoleContext<UartDevice>>> = RwLock::new(None);

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    CONSOLE.write().as_mut().unwrap().write_fmt(args).expect("print to console failed");
}

pub unsafe extern "C" fn init_console() {
    let mut binding = CONSOLE.write();
    *binding = Some(ConsoleContext::new(UartDevice));
}
define_early_init!(init_console);

pub struct ConsoleContext<T>
    where
        T: CharDevice,
{
    pub lock: Mutex<u32>,
    pub device: T,
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
