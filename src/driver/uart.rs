use core::convert::TryInto;

use crate::aarch64::intrinsic::addr::*;
use crate::aarch64::intrinsic::aux::*;
use crate::aarch64::intrinsic::delay_us;
use crate::aarch64::intrinsic::get_u32;
use crate::aarch64::intrinsic::put_u32;

use super::CharDevice;
pub struct UartDevice;
impl CharDevice for UartDevice {
    fn init(&self) {
        put_u32(GPPUD, 0);
        delay_us(5);
        put_u32(GPPUDCLK0, (1 << 14) | (1 << 15));
        delay_us(5);
        put_u32(GPPUDCLK0, 0);

        put_u32(AUX_ENABLES, 1);
        put_u32(AUX_MU_CNTL_REG, 0);
        // enable receiving interrupts.
        put_u32(AUX_MU_IER_REG, 3 << 2 | 1);
        // enable 8-bit mode.
        put_u32(AUX_MU_LCR_REG, 3);
        // set RTS line to always high.
        put_u32(AUX_MU_MCR_REG, 0);
        // set baud rate to 115200.
        put_u32(AUX_MU_BAUD_REG, aux_mu_baud(115_200));
        // clear receive and transmit FIFO.
        put_u32(AUX_MU_IIR_REG, 6);
        // finally, enable receiver and transmitter.
        put_u32(AUX_MU_CNTL_REG, 3);
    }

    fn put_char(&self,c: u8) {
        while get_u32(AUX_MU_LSR_REG) & 0x20 == 0 {}
        put_u32(AUX_MU_IO_REG, c.into());
        if c == 10 {
            self.put_char(13);
        }
    }

    fn get_char(&self) -> u8 {
        let state = get_u32(AUX_MU_IIR_REG);
        if (state & 1) != 0 || (state & 6) != 4 {
            return u8::MAX;
        }
        let result = get_u32(AUX_MU_IO_REG) & 0xff;
        result.try_into().unwrap()
    }
}
