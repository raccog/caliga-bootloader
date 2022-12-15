#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

use core::ptr;
use log::info;

pub mod firmware;

pub enum FileKind {
    Config,
    InitRamFs,
    Kernel,
}

pub trait BootLoaderInterface {
    fn get_memory_map(&self) -> (*const u8, usize) {
        panic!("get_memory_map() not implemented");
    }

    fn read_file(&self, _file: FileKind) -> (*const u8, usize) {
        panic!("read_file() not implemented");
    }
}

pub fn caliga_main<Interface: BootLoaderInterface>(boot: Interface) -> ! {
    let (config_base, config_size) = boot.read_file(FileKind::Config);

    info!("Config: {:p}, {}", config_base, config_size);

    panic!("End of bootloader.");
}
