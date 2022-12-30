#![no_std]
#![no_main]
use core::panic::PanicInfo;

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn _start() {

    let uart = 0x09000000 as *mut u8;
    unsafe {core::ptr::write_volatile(uart, 'a' as u8)};

    loop {}
}
