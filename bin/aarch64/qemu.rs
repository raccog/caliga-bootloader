#![no_std]
#![no_main]

use core::{ptr, fmt::{self, Write}, arch::global_asm };

global_asm!(include_str!("start.S"));

pub const UART0_ADDR: usize = 0x0900_0000;

pub struct UartPl011 {
    base: *mut u8,
}

impl UartPl011 {
    pub fn new(base: *mut u8) -> Result<Self, ()> {
        match base as usize {
            UART0_ADDR => Ok(Self{base}),
            _ => Err(())
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        unsafe { ptr::write_volatile(self.base, byte); }
    }
}

impl fmt::Write for UartPl011 {
    fn write_str(&mut self, out_string: &str) -> fmt::Result {
        for out_byte in out_string.bytes() {
            self.write_byte(out_byte);
        }
        Ok(())
    }
}

#[panic_handler]
fn handle_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
#[link_section = ".text.boot"]
pub extern "C" fn qemu_entry() {
    let mut uart = UartPl011::new(UART0_ADDR as *mut u8).unwrap();
    uart.write_str("Hello aarch64\n").unwrap();

    loop {}
}
