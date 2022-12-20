## File System Abstractions

Many layers of abstractions are needed to get multiple file systems and disk types working:

1. Storage Device Controller - Returns a list of each Storage Device connected to the computer
2. Storage Device - Contains info about the Storage Device's hardware. It's also a Block Device
3. Block Device - A collection of block sectors that can be read from or written to; can be a Storage Device, a Partition, RAM, or really any memory that can be divided into block sectors
4. Partition Table - A table that can be read from a Block Device; returns a list of Partitions from the table
5. Partition - Contains info about the Partition. It's also a Block Device.
6. File System - Allows files to be read from a Block Device using different file system types. When a file is opened, returns a File Descriptor
7. File Descriptor - Allows for many operations on a file such as: opening, closing, reading data, seeking, and reading metadata

Here is a flow chart of how each abstraction might connect:

Storage Device Controller -> Storage Device -> Block Device -> Partition Table -> Partition -> Block Device -> File System -> File Descriptor

### Note

A Block device is deliberately returned from either Storage Device or a Partition. This decision was made so that a File System can be stored on either an entire Storage Device, or a Partition.

I might extend this further so that a File System can also be stored with a File Descriptor. This could allow file system images to be read seamlessly.

## How does this connect to the cross-platform interface?

The following is a possible representation of the connections between these data structures in Rust.

It makes heavy use of lifetime parameters so that each data structure can contain pointers to other structures. I haven't fully implemented something like this yet, so I'm not quite sure it will work. But it seems like a good start.

Note that the structs that contain a boxed driver will likely have the `Box` replaced with a mutable pointer in the future. This is because I probably don't want the structs to own the drivers; the drivers should be stored elsewhere and only pointed to by each struct.

### WARNING

After doing some testing with these data structures, I have realized that this example implementation would not work.

I tried to use different lifetimes for each data structure (Storage Devices, Partition Tables, and File Systems) in an attempt to differentiate between Block Device types at compile time, but it turns out lifetimes are not useful for this. Looks like I need to read more about the Rust borrow checker. :P

I'm leaving this failed example implementation here for now so that I can compare it with my future implementation attempts.

```rust,ignore
trait CrossPlatformInterface {
    fn get_all_partitions(&self) -> &[Partition];
    fn get_storage_devices(&self) -> &[StorageDevice];
    fn get_boot_filesystem(&self) -> &FileSystemInterface;
}

enum BlockDeviceType<'device, 'parttable, 'filesystem> {
    Storage(&'device StorageDevice),
    Part(&'device Partition<'device, 'parttable>),
    File(&'device FileDescriptor<'device, 'filesystem>),
}

struct Block;
trait BlockDevice {
    fn read(&self, sector: u64) -> Block;
    fn sector_count(&self) -> u64;
    fn get_device_type(&self) -> BlockDeviceType;
    fn get_parent_device(&self) -> Option<BlockDeviceType>;
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
    fn from_block_device(block_device: &dyn BlockDevice) -> Self;
    fn get_partitions(&self) -> &[Partition];
    fn get_block_device(&self) -> BlockDeviceType;
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

trait FileSystemDriver {}

struct FileSystemInterface<'device> {
    device: &'device dyn BlockDevice,
    driver: Box<dyn FileSystemDriver>
}
impl<'device> FileSystemInterface<'device> {
    fn from_block_device(block_device: &dyn BlockDevice) -> Self;
    fn get_device(&self) -> &'device dyn BlockDevice;
    fn open(&self, path: &str) -> FileDescriptor;
    fn close(&self, fd: FileDescriptor);
    fn read(&self, fd: &mut FileDescriptor, buf: &mut [u8]);
    fn seek(&self, fd: &mut FileDescriptor, location: u64);
}

trait FileMetadata {}

struct FileDescriptor<'device, 'filesystem> {
    filesystem: &'filesystem FileSystemInterface<'device>,
    driver: &'filesystem dyn FileSystemDriver,
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
```