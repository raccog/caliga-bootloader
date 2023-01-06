#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

pub mod arch;
pub mod filesystem;
pub mod firmware;

use crate::filesystem::{FileSystemInterface};
use alloc::{boxed::Box, vec::Vec};
use log::info;

pub struct BootConfig {

}

pub struct BootInterface {
    pub config_buffer: BootConfig,
    pub block_devices: Vec<Box<dyn BlockDeviceInterface>>,
    pub file_systems: Vec<Box<dyn FileSystemInterface>>,
}

pub trait BlockDeviceInterface {
    // TODO: Determine whether it would be useful to read many sectors at a time
    fn read(&self, _sector: u64) -> [u8; 512] {
        unimplemented!();
    }

    fn sector_count(&self) -> u64 {
        unimplemented!();
    }
}

pub unsafe fn caliga_main(boot: BootInterface) -> ! {

    panic!("End of bootloader.");
}
