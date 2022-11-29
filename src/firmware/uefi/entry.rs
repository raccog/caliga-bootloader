#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use caliga_bootloader::BootLoaderInterface;

use core::{ops::DerefMut, panic::PanicInfo, ptr};
use log::{error, info, warn};
use uefi::{self, prelude::*};
use uefi_services::println;

struct UefiInterface {}

impl BootLoaderInterface for UefiInterface {
    fn read_config(&self) -> (*const u8, usize) {
        (ptr::null(), 0)
    }
}

#[panic_handler]
fn handle_panic(info: &PanicInfo) -> ! {
    println!("[PANIC]: {}", info);
    loop {}
}

#[entry]
fn boot_uefi_entry(image_handle: Handle, mut st: SystemTable<Boot>) -> Status {
    // Initialize UEFI
    uefi_services::init(&mut st).unwrap();
    let bt = st.boot_services();

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
    let firmware_revision = st.firmware_revision();
    let uefi_revision = st.uefi_revision();
    info!(
        "Firmware Vendor: {} Revision {}.{}",
        st.firmware_vendor(),
        firmware_revision.major(),
        firmware_revision.minor()
    );
    info!(
        "UEFI Revision {}.{}",
        uefi_revision.major(),
        uefi_revision.minor()
    );

    // Get the file system that the bootloader image was loaded from
    // NOTE: This type of `expect`-based error logging is quick to write, but
    // does not provide explicit logs for different error cases. It should
    // eventually be converted to `match`-based logging.
    // TODO: Switch to `match`-based logging.
    let mut fs = bt
        .get_image_file_system(image_handle)
        .expect("Could not get boot image's file system!");
    let fs = fs.deref_mut();

    // Open root directory
    let mut _root_dir = fs
        .open_volume()
        .expect("Could not get root directory of boot image's file system!");

    let interface = UefiInterface {};
    caliga_bootloader::caliga_main(interface);
}
