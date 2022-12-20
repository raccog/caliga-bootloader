#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

use alloc::vec;

pub mod filesystem;
pub mod firmware;

use filesystem::FileSystem;

mod tmp {
    use alloc::boxed::Box;

    pub trait CrossPlatformInterface<'bootmem> {
        fn all_partitions(&self) -> &[Partition] {
            panic!("NOT IMPLEMENTED");
        }
        fn get_storage_devices(&self) -> &[StorageDevice] {
            panic!("NOT IMPLEMENTED");
        }
        fn get_boot_filesystem(&'bootmem self) -> &'bootmem FileSystemInterface<'bootmem> {
            panic!("NOT IMPLEMENTED");
        }
    }

    struct UefiInterface<'bootmem> {
        bootfs: FileSystemInterface<'bootmem>
    }
    impl<'bootmem> CrossPlatformInterface<'bootmem> for UefiInterface<'bootmem> {
        fn get_boot_filesystem(&'bootmem self) -> &'bootmem FileSystemInterface<'bootmem> {
            &self.bootfs
        }
    }

    const SECTOR_SIZE: usize = 512;

    trait BlockDevice<'bootmem> {
        fn read(&self, sector: u64) -> [u8; SECTOR_SIZE] {
            panic!("NOT IMPLEMENTED");
        }

        fn sector_count(&self) -> u64 {
            panic!("NOT IMPLEMENTED");
        }

        fn get_device_type(&'bootmem self) -> BlockDeviceType;
        fn get_parent_device(&'bootmem self) -> Option<BlockDeviceType>;
    }

    enum BlockDeviceType<'bootmem> {
        Storage(&'bootmem StorageDevice),
        Part(&'bootmem Partition<'bootmem>),
        File(&'bootmem FileDescriptor<'bootmem>),
    }

    trait StorageDeviceDriver {}

    struct StorageDevice {
        driver: Box<dyn StorageDeviceDriver>
    }

    impl<'bootmem> BlockDevice<'bootmem> for StorageDevice {
        fn get_device_type(&'bootmem self) -> BlockDeviceType {
            BlockDeviceType::Storage(&self)
        }

        fn get_parent_device(&'bootmem self) -> Option<BlockDeviceType> {
            None
        }
    }

    trait PartitionTableDriver {}

    struct PartitionTable<'bootmem> {
        device: &'bootmem dyn BlockDevice<'bootmem>,
        driver: Box<dyn PartitionTableDriver>
    }

    impl<'bootmem> PartitionTable<'bootmem> {
        fn new(device: &'bootmem dyn BlockDevice<'bootmem>, driver: Box<dyn PartitionTableDriver>) -> Self {
            Self {
                device, driver
            }
        }

        fn get_partitions(&self) -> &[Partition] {
            panic!("NOT IMPLEMENTED");
        }

        fn get_block_device(&'bootmem self) -> BlockDeviceType {
            self.device.get_device_type()
        }
    }

    struct Partition<'bootmem> {
        table: &'bootmem PartitionTable<'bootmem>
    }

    impl<'bootmem> BlockDevice<'bootmem> for Partition<'bootmem> {
        fn get_device_type(&'bootmem self) -> BlockDeviceType {
            BlockDeviceType::Part(&self)
        }

        fn get_parent_device(&'bootmem self) -> Option<BlockDeviceType> {
            Some(self.table.get_block_device())
        }
    }

    trait FileSystemDriver {
        fn open(&self, path: &str) -> FileDescriptor {
            panic!("NOT IMPLEMENTED");
        }
        fn close(&self, fd: FileDescriptor) {
            panic!("NOT IMPLEMENTED");
        }
        fn read(&self, fd: &mut FileDescriptor, buf: &mut [u8]) {
            panic!("NOT IMPLEMENTED");
        }
        fn seek(&self, fd: &mut FileDescriptor, location: u64) {
            panic!("NOT IMPLEMENTED");
        }
    }

    struct FileSystemInterface<'bootmem> {
        device: &'bootmem dyn BlockDevice<'bootmem>,
        driver: Box<dyn FileSystemDriver>
    }

    impl<'bootmem> FileSystemInterface<'bootmem> {
        fn new(device: &'bootmem dyn BlockDevice<'bootmem>, driver: Box<dyn FileSystemDriver>) -> Self {
            Self {
                device, driver
            }
        }

        fn get_device(&self) -> &'bootmem dyn BlockDevice {
            self.device
        }
    }

    impl<'bootmem> FileSystemDriver for FileSystemInterface<'bootmem> {
        fn open(&self, path: &str) -> FileDescriptor {
            self.driver.open(path)
        }
        fn close(&self, fd: FileDescriptor) {
            self.driver.close(fd)
        }
        fn read(&self, fd: &mut FileDescriptor, buf: &mut [u8]) {
            self.driver.read(fd, buf)
        }
        fn seek(&self, fd: &mut FileDescriptor, location: u64) {
            self.driver.seek(fd, location)
        }
    }

    trait FileMetadata {}

    struct FileDescriptor<'bootmem> {
        filesystem: &'bootmem FileSystemInterface<'bootmem>,
        driver: &'bootmem dyn FileSystemDriver,
        metadata: Box<dyn FileMetadata>
    }
    impl<'bootmem> BlockDevice<'bootmem> for FileDescriptor<'bootmem> {
        fn get_device_type(&'bootmem self) -> BlockDeviceType {
            BlockDeviceType::File(&self)
        }
        fn get_parent_device(&'bootmem self) -> Option<BlockDeviceType> {
            Some(self.filesystem.get_device().get_device_type())
        }
    }

    fn caliga_main<'bootmem>(boot: &'bootmem mut dyn CrossPlatformInterface<'bootmem>) -> ! {
        let filesystem = boot.get_boot_filesystem();
        //let filesystem2 = boot.get_boot_filesystem();
        let mut fd = filesystem.open("/path/test");
        let mut buf = [0; 256];
        filesystem.read(&mut fd, &mut buf);
        //filesystem2.read(&mut fd, &mut buf);
        filesystem.close(fd);

        loop {}
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
