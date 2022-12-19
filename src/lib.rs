#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

use alloc::vec;

pub mod filesystem;
pub mod firmware;

use filesystem::FileSystem;

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
