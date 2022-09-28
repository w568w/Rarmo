pub mod console;
pub mod physical_memory;
pub mod virtual_memory;
pub mod slob;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::cores::console::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

