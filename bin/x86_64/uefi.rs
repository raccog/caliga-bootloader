#![no_std]
#![no_main]
#![feature(abi_efiapi)]

extern crate alloc;

use core::{ops::DerefMut, panic::PanicInfo};
use log::{error, debug, info, warn};
use uefi::{self, prelude::*, proto::loaded_image::LoadedImage};
use uefi_services::println;

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
        "Firmware Vendor: {} Revision {:#x}",
        system_table.firmware_vendor(),
        firmware_revision,
    );
    info!(
        "UEFI Revision {}.{}",
        uefi_revision.major(),
        uefi_revision.minor()
    );

    // Output program info
    {
        // `loaded_image` needs to be dropped at the end of this inner block so that it can be opened again
        // later
        let loaded_image = bt.open_protocol_exclusive::<LoadedImage>(image_handle)
            .expect("Could not open UEFI image handle");
        let (image_addr, image_size) = loaded_image.info();
        debug!("PROGRAM_START: {:p}", image_addr);
        debug!("PROGRAM_END  : {:#x}", image_addr as usize + image_size as usize);
        debug!("PROGRAM_SIZE : {:#x}", image_size);
    }

    let _root_directory = {
        let bt = system_table.boot_services();
        // Get the file system that the bootloader image was loaded from
        // NOTE: This type of `expect`-based error logging is quick to write, but
        // does not provide explicit logs for different error cases. It should
        // eventually be converted to `match`-based logging.
        // TODO: Switch to `match`-based logging.
        let mut uefi_fs = bt
            .get_image_file_system(image_handle.clone())
            .expect("Could not get boot image's file system!");
        let uefi_fs = uefi_fs.deref_mut();

        // Open root directory
        uefi_fs
            .open_volume()
            .expect("Could not get root directory of boot image's file system!")
    };

    panic!("End of bootloader reached");
}
