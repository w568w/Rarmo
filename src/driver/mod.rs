pub mod uart;
pub mod power;
pub mod clock;
pub mod interrupt;

pub trait CharDevice {
    fn init(&self);
    fn put_char(&self,c: u8);
    fn get_char(&self) -> u8;
}
