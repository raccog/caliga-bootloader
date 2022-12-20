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

    pub trait CrossPlatformInterface<'filesystem> {
        fn all_partitions(&self) -> &[Partition] {
            panic!("NOT IMPLEMENTED");
        }
        fn get_storage_devices(&self) -> &[StorageDevice] {
            panic!("NOT IMPLEMENTED");
        }
        fn get_boot_filesystem(&self) -> &'filesystem FileSystemInterface {
            panic!("NOT IMPLEMENTED");
        }
    }

    struct UefiInterface<'device, 'filesystem> {
        bootfs: FileSystemInterface<'device, 'filesystem>
    }
    impl<'device, 'filesystem> CrossPlatformInterface<'filesystem> for UefiInterface<'device, 'filesystem> {
        fn get_boot_filesystem(&self) -> &'filesystem FileSystemInterface {
            &self.bootfs
        }
    }

    const SECTOR_SIZE: usize = 512;

    trait BlockDevice {
        fn read(&self, sector: u64) -> [u8; SECTOR_SIZE] {
            panic!("NOT IMPLEMENTED");
        }

        fn sector_count(&self) -> u64 {
            panic!("NOT IMPLEMENTED");
        }

        fn get_device_type(&self) -> BlockDeviceType;
        fn get_parent_device(&self) -> Option<BlockDeviceType>;
    }

    enum BlockDeviceType<'device, 'parttable, 'filesystem> {
        Storage(&'device StorageDevice),
        Part(&'device Partition<'device, 'parttable>),
        File(&'filesystem FileDescriptor<'device, 'filesystem>),
    }

    trait StorageDeviceDriver {}

    struct StorageDevice {
        driver: Box<dyn StorageDeviceDriver>
    }

    impl BlockDevice for StorageDevice {
        fn get_device_type(&self) -> BlockDeviceType {
            BlockDeviceType::Storage(&self)
        }

        fn get_parent_device(&self) -> Option<BlockDeviceType> {
            None
        }
    }

    trait PartitionTableDriver {}

    struct PartitionTable<'device> {
        device: &'device dyn BlockDevice,
        driver: Box<dyn PartitionTableDriver>
    }

    impl<'device> PartitionTable<'device> {
        fn new(device: &'device dyn BlockDevice, driver: Box<dyn PartitionTableDriver>) -> Self {
            Self {
                device, driver
            }
        }

        fn get_partitions(&self) -> &[Partition] {
            panic!("NOT IMPLEMENTED");
        }

        fn get_block_device(&self) -> BlockDeviceType {
            self.device.get_device_type()
        }
    }

    struct Partition<'device, 'parttable> {
        table: &'parttable PartitionTable<'device>
    }

    impl<'device, 'parttable> BlockDevice for Partition<'device, 'parttable> {
        fn get_device_type(&self) -> BlockDeviceType {
            BlockDeviceType::Part(&self)
        }

        fn get_parent_device(&self) -> Option<BlockDeviceType> {
            Some(self.table.get_block_device())
        }
    }

    trait FileSystemDriver<'filesystem> {
        fn open(&'filesystem self, path: &str) -> &'filesystem mut FileDescriptor {
            panic!("NOT IMPLEMENTED");
        }
        fn close(&'filesystem self, fd: &'filesystem FileDescriptor) {
            panic!("NOT IMPLEMENTED");
        }
        fn read(&'filesystem self, fd: &'filesystem mut FileDescriptor, buf: &mut [u8]) {
            panic!("NOT IMPLEMENTED");
        }
        fn seek(&'filesystem self, fd: &'filesystem mut FileDescriptor, location: u64) {
            panic!("NOT IMPLEMENTED");
        }
    }

    struct FileSystemInterface<'device, 'filesystem> {
        device: &'device dyn BlockDevice,
        driver: Box<dyn FileSystemDriver<'filesystem>>
    }

    impl<'device, 'filesystem> FileSystemInterface<'device, 'filesystem> {
        fn new(device: &'device dyn BlockDevice, driver: Box<dyn FileSystemDriver<'filesystem>>) -> Self {
            Self {
                device, driver
            }
        }

        fn get_device(&self) -> &'device dyn BlockDevice {
            self.device
        }
    }

    impl<'device, 'filesystem> FileSystemDriver<'filesystem> for FileSystemInterface<'device, 'filesystem> {
        fn open(&'filesystem self, path: &str) -> &'filesystem mut FileDescriptor {
            self.driver.open(path)
        }
        fn close(&'filesystem self, fd: &'filesystem FileDescriptor) {
            self.driver.close(fd)
        }
        fn read(&'filesystem self, fd: &'filesystem mut FileDescriptor, buf: &mut [u8]) {
            self.driver.read(fd, buf)
        }
        fn seek(&'filesystem self, fd: &'filesystem mut FileDescriptor, location: u64) {
            self.driver.seek(fd, location)
        }
    }

    trait FileMetadata {}

    struct FileDescriptor<'device, 'filesystem> {
        filesystem: &'filesystem FileSystemInterface<'device, 'filesystem>,
        driver: &'filesystem dyn FileSystemDriver<'filesystem>,
        metadata: Box<dyn FileMetadata>
    }
    impl<'device, 'filesystem> BlockDevice for FileDescriptor<'device, 'filesystem> {
        fn get_device_type(&self) -> BlockDeviceType {
            BlockDeviceType::File(&self)
        }
        fn get_parent_device(&self) -> Option<BlockDeviceType> {
            Some(self.filesystem.get_device().get_device_type())
        }
    }

    fn caliga_main<'filesystem, Interface: CrossPlatformInterface<'filesystem>>(mut boot: Interface) -> ! {
        let filesystem = boot.get_boot_filesystem();
        let filesystem2 = boot.get_boot_filesystem();
        let mut fd = filesystem.open("/path/test");
        let mut buf = [0; 256];
        filesystem.read(&mut fd, &mut buf);
        filesystem2.read(&mut fd, &mut buf);
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
