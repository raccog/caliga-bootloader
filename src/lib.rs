#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

pub mod filesystem;
pub mod firmware;

use crate::filesystem::OpenFileError;
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

pub trait FileDescriptorInterface {}

pub struct FileDescriptor {
    pub index: usize,
    pub offset: u64,
    pub path: String,
    pub driver: Box<dyn FileDescriptorInterface>,
}

pub trait FileSystemInterface {
    unsafe fn open_file(&mut self, _path: &str) -> Result<*mut FileDescriptor, OpenFileError> {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn close(&self, _fd: *mut FileDescriptor) {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn read_file(
        &self,
        _fd: *mut FileDescriptor,
        _buf: &mut [u8],
        _size: usize,
    ) -> Result<usize, usize> {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn seek(&self, _fd: *mut FileDescriptor, _location: u64) -> Result<usize, usize> {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn get_size(&self, _fd: *mut FileDescriptor) -> u64 {
        panic!("NOT IMPLEMENTED");
    }
}

pub struct FileSystem {
    pub index: u32,
    pub driver: Box<dyn FileSystemInterface>,
}

impl FileSystemInterface for FileSystem {
    unsafe fn open_file(&mut self, path: &str) -> Result<*mut FileDescriptor, OpenFileError> {
        self.driver.open_file(path)
    }

    unsafe fn close(&self, fd: *mut FileDescriptor) {
        self.driver.close(fd)
    }

    unsafe fn read_file(
        &self,
        fd: *mut FileDescriptor,
        buf: &mut [u8],
        size: usize,
    ) -> Result<usize, usize> {
        self.driver.read_file(fd, buf, size)
    }

    unsafe fn seek(&self, fd: *mut FileDescriptor, location: u64) -> Result<usize, usize> {
        self.driver.seek(fd, location)
    }

    unsafe fn get_size(&self, fd: *mut FileDescriptor) -> u64 {
        self.driver.get_size(fd)
    }
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
    let file_size = (*filesystem).get_size(descriptor) as usize;
    info!("File size: {}", file_size);
    let mut buf = vec![0; file_size];
    if let Ok(read_size) = (*filesystem).read_file(descriptor, &mut buf, file_size) {
        info!(
            "Requested_size: {}, Read_size: {}, Buf_size: {}",
            file_size,
            read_size,
            buf.len()
        );
    } else {
        panic!("Could not read config file");
    }
    if let Ok(config_contents) = String::from_utf8(buf) {
        info!("File contents: {}", config_contents);
    } else {
        panic!("Could not print file contents as UTF8");
    }

    panic!("End of bootloader.");
}
