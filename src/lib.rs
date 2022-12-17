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
    fn get_memory_map(&self) -> (*const u8, usize) {
        panic!("get_memory_map() not implemented");
    }

    fn get_boot_filesystem(&mut self) -> &mut dyn FileSystem {
        panic!("get_boot_filesystem() not implemented")
    }

    fn read_file(&self, _file: FileKind) -> (*const u8, usize) {
        panic!("read_file() not implemented");
    }
}

pub fn caliga_main<Interface: BootLoaderInterface>(mut boot: Interface) -> ! {
    let filesystem = boot.get_boot_filesystem();
    let mut descriptor = {
        let fs_result = filesystem.open_file("/efi/boot////bootx64.efi");
        if let Ok(fs) = fs_result {
            fs
        } else {
            panic!("Could not open config file: {:?}", fs_result);
        }
    };
    let file_size = descriptor.size as usize;
    let mut buf = vec![0; file_size];
    filesystem.read_file(&mut descriptor, &mut buf, file_size);

    panic!("End of bootloader.");
}
