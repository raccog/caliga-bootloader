#![no_std]
#![no_main]
#![feature(core_intrinsics)]

use core::arch::global_asm;

global_asm!(include_str!("start.S"));

#[panic_handler]
fn handle_panic(info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
#[link_section = ".text.boot"]
pub extern "C" fn qemu_entry() {
    // Taken from https://lowenware.com/blog/aarch64-bare-metal-program-in-rust/
    let uart = 0x0900_0000 as *mut u8;
    let out_str = b"Hello aarch64\n";
    for byte in out_str {
        unsafe {
            core::ptr::write_volatile(uart, *byte);
        }
    }

    loop {}
}
