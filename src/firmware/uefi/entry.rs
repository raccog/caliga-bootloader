#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use caliga_bootloader::{firmware::uefi::file_system, BootLoaderInterface, FileKind};

use core::{ops::DerefMut, panic::PanicInfo, ptr};
use log::{error, info, warn};
use uefi::{self, prelude::*, proto::media::file::Directory, CString16};
use uefi_services::println;

struct UefiInterface<'a> {
    image_handle: &'a Handle,
    system_table: &'a mut SystemTable<Boot>,
}

impl<'a> BootLoaderInterface for UefiInterface<'a> {
    fn read_file(&self, file: FileKind) -> (*const u8, usize) {
        let mut esp_root_dir = self.get_root_dir();
        let path = match file {
            FileKind::Config => CString16::try_from("/caliga.txt").unwrap(),
            FileKind::InitRamFs => CString16::try_from("/initramfs.img").unwrap(),
            FileKind::Kernel => CString16::try_from("/kernel.elf").unwrap(),
        };
        let file_kind = match file {
            FileKind::Config => "config",
            FileKind::InitRamFs => "initramfs",
            FileKind::Kernel => "kernel",
        };

        match file_system::open_file(&mut esp_root_dir, &path) {
            Ok(_file) => {
                info!("Found {} file!", file_kind);
            }
            Err(_) => {
                panic!("Could not open {} file at {}", file_kind, path);
            }
        }

        (ptr::null(), 0)
    }
}

impl<'a> UefiInterface<'a> {
    fn get_root_dir(&self) -> Directory {
        let bt = self.system_table.boot_services();
        // Get the file system that the bootloader image was loaded from
        // NOTE: This type of `expect`-based error logging is quick to write, but
        // does not provide explicit logs for different error cases. It should
        // eventually be converted to `match`-based logging.
        // TODO: Switch to `match`-based logging.
        let mut fs = bt
            .get_image_file_system(self.image_handle.clone())
            .expect("Could not get boot image's file system!");
        let fs = fs.deref_mut();

        // Open root directory
        fs.open_volume()
            .expect("Could not get root directory of boot image's file system!")
    }
}

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    println!("[PANIC]: {}", info);
    loop {}
}

#[entry]
fn boot_uefi_entry(image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    // Initialize UEFI
    uefi_services::init(&mut system_table).unwrap();
    let bt = system_table.boot_services();

    println!("Today is the shadow of tomorrow");

    // Disable watchdog timer
    // NOTE: This type of explicit error logging is nice and could be expanded to
    // all possible bootloader errors.
    // TODO: Rewrite error messages
    if let Err(e) = bt.set_watchdog_timer(0, 0x10000, None) {
        match e.status() {
            Status::INVALID_PARAMETER => {
                error!("The supplied watchdog code is invalid!");
            }
            Status::UNSUPPORTED => {
                info!("This system does not have a watchdog timer.");
            }
            Status::DEVICE_ERROR => {
                warn!("The watchdog could not be set due to a hardware error.");
            }
            _ => warn!(
                "SetWatchdogTimer() returned an invalid error code: {}",
                e.status().0
            ),
        }
    }

    // Log UEFI information
    // TODO: Change `info!` to `debug!`
    let firmware_revision = system_table.firmware_revision();
    let uefi_revision = system_table.uefi_revision();
    info!(
        "Firmware Vendor: {} Revision {}.{}",
        system_table.firmware_vendor(),
        firmware_revision.major(),
        firmware_revision.minor()
    );
    info!(
        "UEFI Revision {}.{}",
        uefi_revision.major(),
        uefi_revision.minor()
    );

    let interface = UefiInterface {
        image_handle: &image_handle,
        system_table: &mut system_table,
    };
    caliga_bootloader::caliga_main(interface);
}
