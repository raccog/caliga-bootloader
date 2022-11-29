#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

use core::ptr;

pub mod firmware;

pub trait BootLoaderInterface {
    fn get_memory_map(&self) -> ( *const u8, usize ) {
        (ptr::null(), 0)
    }

    fn read_config(&self) -> ( *const u8, usize ) {
        (ptr::null(), 0)
    }

    fn read_initramfs(&self) -> ( *const u8, usize ) {
        (ptr::null(), 0)
    }

    fn read_kernel(&self) -> ( *const u8, usize ) {
        (ptr::null(), 0)
    }
}

pub fn caliga_main<Interface: BootLoaderInterface>(boot: Interface) -> ! {
    let _config = boot.read_config();

    panic!("End of bootloader.");
}
