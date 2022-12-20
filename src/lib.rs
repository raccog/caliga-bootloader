#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

use alloc::vec;

pub mod filesystem;
pub mod firmware;

use filesystem::FileSystem;

mod tmp {
    use alloc::{boxed::Box, vec::Vec};

    struct CrossPlatformInterface {
        storage_devices: Vec<StorageDevice>,
        partition_tables: Vec<PartitionTable>,
        partitions: Vec<Partition>,
        file_systems: Vec<FileSystem>,
    }

    trait BlockDeviceInterface {
        fn read(&self, sector: u64) -> [u8; 512] {
            panic!("NOT IMPLEMENTED");
        }

        fn sector_count(&self) -> u64 {
            panic!("NOT IMPLEMENTED");
        }
    }

    struct StorageDevice {
        index: u32,
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

    struct UefiStorageDeviceDriver {
        system_table: usize,
    }

    impl BlockDeviceInterface for UefiStorageDeviceDriver {}

    struct PartitionTable {
        index: u32,
    }

    struct Partition {
        index: u32,
    }

    struct FileSystem {
        index: u32,
    }
}

pub enum FileKind {
    Config,
    InitRamFs,
    Kernel,
}

pub trait BootLoaderInterface {
    // TODO: Figure out how to not have this trait type here?
    type FileSystemData;

    fn get_memory_map(&self) -> (*const u8, usize) {
        panic!("get_memory_map() not implemented");
    }

    // TODO: Figure out how to make this return type more concise?
    fn get_boot_filesystem(&mut self) -> &mut dyn FileSystem<FileSystemData = Self::FileSystemData> {
        panic!("get_boot_filesystem() not implemented")
    }
}

pub fn caliga_main<Interface: BootLoaderInterface>(mut boot: Interface) -> ! {
    let filesystem = boot.get_boot_filesystem();
    let mut descriptor = {
        let fs_result = filesystem.open_file("/efi/boot////bootx64.efi");
        if let Err(err) = fs_result {
            panic!("Could not open config file: {:?}", err);
        }
        fs_result.unwrap()
    };
    let file_size = filesystem.get_size(&mut descriptor) as usize;
    let mut buf = vec![0; file_size];
    filesystem.read_file(&mut descriptor, &mut buf, file_size);

    panic!("End of bootloader.");
}
