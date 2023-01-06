#![no_std]
#![no_main]

use core::{ptr, arch::global_asm};

global_asm!(include_str!("start.S"));

#[panic_handler]
fn handle_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

struct UartPl011 {
    pub base: *mut u8,
}

impl UartPl011 {
    pub fn write_byte(&mut self, byte: u8) {
        unsafe { ptr::write_volatile(self.base, byte); }
    }
}

#[no_mangle]
#[link_section = ".text.boot"]
pub extern "C" fn qemu_entry() {
    // Taken from https://lowenware.com/blog/aarch64-bare-metal-program-in-rust/
    const UART_ADDR: usize = 0x0900_0000;
    let mut uart = UartPl011 { base: UART_ADDR as *mut u8 };
    let out_str = b"Hello aarch64\n";
    for byte in out_str {
        uart.write_byte(*byte);
    }

    loop {}
}
