#![no_std]
#![no_main]
#![feature(allocator_api)]
#![feature(panic_info_message)]

use core::{
    arch::global_asm,
    cell::UnsafeCell,
    fmt::{self, Write},
    ptr::{self},
};
use log::{self, info, LevelFilter, Log, Metadata, Record};

// The start procedure
global_asm!(include_str!("start.S"));

/// Address of UART0 on default QEMU for aarch64
pub const UART0_ADDR: usize = 0x0900_0000;

// An unimplemented allocator to see how it may be structured
mod bump_allocator {
    use core::alloc::{GlobalAlloc, Layout};

    #[global_allocator]
    static GLOBAL_ALLOCATOR: Aarch64QemuAlloc = Aarch64QemuAlloc {};

    struct Aarch64QemuAlloc;

    unsafe impl GlobalAlloc for Aarch64QemuAlloc {
        unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
            unimplemented!();
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
            unimplemented!();
        }
    }
}

/// A very simple implementation of the PL011 UART peripheral
pub struct UartPl011 {
    base: usize,
}

impl UartPl011 {
    /// Creates a new UART with a `base` address
    ///
    /// # Errors
    ///
    /// Returns an error if `base` is an invalid UART address. It needs to be one of the following:
    ///
    /// * [`UART0_ADDR`]
    pub fn new(base: usize) -> Result<Self, ()> {
        match base as usize {
            UART0_ADDR => Ok(Self { base }),
            _ => Err(()),
        }
    }

    /// Writes a byte to the UART's output FIFO
    pub fn write_byte(&mut self, byte: u8) {
        unsafe {
            ptr::write_volatile(self.base as *mut u8, byte);
        }
    }
}

// Implemented so I can easily print strings to UART
impl Write for UartPl011 {
    fn write_str(&mut self, out_string: &str) -> fmt::Result {
        for out_byte in out_string.bytes() {
            self.write_byte(out_byte);
        }
        Ok(())
    }
}

/// A logger that outputs to a PL011 UART
///
/// This is a proof of concept to see what is necessary to set up a default logger.
///
/// # Interior Mutability
///
/// Internally, it uses an [`UnsafeCell`] to contain the UART struct because the method `log` would disallow
/// interior mutability, otherwise. Since this bootloader will always run on a single thread, there should be
/// no problems with race conditions.
struct UartPl011Logger {
    uart: UnsafeCell<UartPl011>,
}

// Implement traits that are needed for `Log`
unsafe impl Sync for UartPl011Logger {}
unsafe impl Send for UartPl011Logger {}

impl Log for UartPl011Logger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level().to_level_filter() <= log::max_level()
    }

    // A very basic logger. Only outputs the log if it's possible without any allocations
    //
    // I want to move this into a cross-architecture implementation so that all logs can be formatted
    // the same. Also, it might be useful to use this in the panic logs, too.
    //
    // TODO: Deal with all the calls to `unwrap`
    fn log(&self, record: &Record<'_>) {
        // Get a mutable reference to the UART
        let uart = unsafe { &mut *self.uart.get() };

        // Write log level
        write!(uart, "[{}] ", record.level().as_str()).unwrap();

        // Try to write log without any allocations
        if let Some(args) = record.args().as_str() {
            uart.write_str(args).unwrap();
        } else {
            uart.write_str("Could not get log; allocator needed")
                .unwrap();
        }

        // Try to write log file and line without any allocations
        if let (Some(file_name), Some(line)) = (record.file(), record.line()) {
            write!(uart, ", {}:{:?}", file_name, line).unwrap();
        }

        uart.write_char('\n').unwrap();
    }

    fn flush(&self) {}
}

#[panic_handler]
fn handle_panic(info: &core::panic::PanicInfo) -> ! {
    // Try to re-initialize UART0 and print a panic log
    if let Ok(mut uart) = UartPl011::new(UART0_ADDR) {
        // TODO: Maybe halt if this returns an error
        writeln!(&mut uart, "[PANIC] {}", info).unwrap();
    }
    loop {}
}

// The default logger
static mut LOGGER: Option<UartPl011Logger> = None;

#[no_mangle]
#[link_section = ".text.boot"]
pub unsafe extern "C" fn qemu_entry() {
    // Initialize UART0
    let uart = UartPl011::new(UART0_ADDR).unwrap();

    // Initialize logger using UART0
    let logger = {
        LOGGER = Some(UartPl011Logger { uart: uart.into() });
        LOGGER.as_ref().unwrap()
    };
    log::set_logger(logger).unwrap();
    log::set_max_level(LevelFilter::Debug);

    // Test out logger
    info!("Done with info log");

    // TODO: Run kernel
    panic!("End of bootloader reached");
}
