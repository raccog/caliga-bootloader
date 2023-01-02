#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

pub mod arch;
pub mod filesystem;
pub mod firmware;

use crate::filesystem::{FileSystem, FileSystemInterface};
use alloc::{boxed::Box, string::String, vec, vec::Vec};
use log::info;

pub struct CrossPlatformHeader {
    pub storage_devices: Vec<StorageDevice>,
    pub partition_tables: Vec<PartitionTable>,
    pub partitions: Vec<Partition>,
    pub file_systems: Vec<FileSystem>,
    pub boot_file_system_index: usize,
}

impl CrossPlatformHeader {
    pub fn get_boot_filesystem(&self) -> *mut FileSystem {
        assert!(self.boot_file_system_index < self.file_systems.len());
        &self.file_systems[self.boot_file_system_index] as *const FileSystem as *mut FileSystem
    }
}

pub trait BlockDeviceInterface {
    // TODO: Determine whether it would be useful to read many sectors at a time
    fn read(&self, _sector: u64) -> [u8; 512] {
        panic!("NOT IMPLEMENTED");
    }

    fn sector_count(&self) -> u64 {
        panic!("NOT IMPLEMENTED");
    }
}

pub struct StorageDevice {
    _index: u32,
    driver: Box<dyn BlockDeviceInterface>,
}

impl BlockDeviceInterface for StorageDevice {
    fn read(&self, sector: u64) -> [u8; 512] {
        self.driver.read(sector)
    }

    fn sector_count(&self) -> u64 {
        self.driver.sector_count()
    }
}

pub struct PartitionTable {
    _index: u32,
}

pub struct Partition {
    _index: u32,
}

pub unsafe fn caliga_main(boot: CrossPlatformHeader) -> ! {
    let filesystem = boot.get_boot_filesystem();

    info!("Opening config file");
    let descriptor = {
        let fs_result = (*filesystem).open_file("/config.txt");
        if let Err(err) = fs_result {
            panic!("Could not open config file: {:?}", err);
        }
        fs_result.unwrap()
    };

    let file_size = (*filesystem).get_size(descriptor).unwrap_or_else(|_| {
        panic!("Could not get size for file");
    }) as usize;
    info!("File size: {}", file_size);

    match (*filesystem).seek_file(descriptor, 1) {
        Ok(_) => {
            info!("Set file position to the second byte");
        }
        Err(_) => {
            panic!("Could not set file position");
        }
    }
    let mut buf = vec![0; file_size];
    let read_result = (*filesystem).read_file(descriptor, &mut buf);
    let read_size = read_result.unwrap_or_else(|bytes_read| {
        panic!(
            "Could not read config file in full; only read {} bytes",
            bytes_read
        );
    });

    if let Err(_) = (*filesystem).close_file(descriptor) {
        panic!("Could not close file");
    }

    info!("Requested_size: {}, Read_size: {}", file_size, read_size,);
    buf.truncate(read_size);

    if let Ok(config_contents) = String::from_utf8(buf) {
        info!("File contents: {}", config_contents);
    } else {
        panic!("Could not print file contents as UTF8");
    }

    panic!("End of bootloader.");
}
