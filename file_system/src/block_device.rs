use crate::defines::SuperBlock;

pub fn init_block_device() {
    todo!()
}

pub fn get_super_block() -> &'static SuperBlock {
    todo!()
}
pub trait BlockDevice {
    fn write(&mut self, block_no: usize, buf: &[u8]);
    fn read(&mut self, block_no: usize, buf: &mut [u8]);
}