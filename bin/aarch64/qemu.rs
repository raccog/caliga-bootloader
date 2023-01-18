#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(allocator_api)]
#![feature(panic_info_message)]

extern crate alloc;

use alloc::vec;
use core::{
    arch::global_asm,
    cell::UnsafeCell,
    fmt::{self, Write},
    ptr
};
use log::{self, debug, info, LevelFilter, Log, Metadata, Record};

use caliga_bootloader::io::{io::Io, mmio::Mmio};

// The start procedure
global_asm!(include_str!("start.S"));

/// Address of UART0 on default QEMU for aarch64
pub const UART0_ADDR: usize = 0x0900_0000;

// TODO: Move this to its own file
pub mod intrusive_list {
    pub struct IntrusiveList<T> {
        pub allocated_nodes: *const T,
        pub capacity: usize,
    }

    pub struct IntrusiveNode<T> {
        pub next: *const T,
    }
}

use intrusive_list::{IntrusiveNode, IntrusiveList};

struct FreeMemoryChunk {
    data: *const u8,
    node: Option<IntrusiveNode<FreeMemoryChunk>>
}

static mut FREE_MEMORY: Option<IntrusiveList<FreeMemoryChunk>> = None;

struct MemoryRange {
    pub start: usize,
    pub size: usize
}

// An unimplemented allocator to see how it may be structured
//mod bump_allocator {
use core::alloc::{GlobalAlloc, Layout};

#[global_allocator]
static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator;

// Note that these are linker-defined variables.
// Although they are declared as a `u8`, the address of each variable is the true value.
//
// So, to get PROGRAM_START as a `usize`, you need to do the following:
//
// ```
// let program_start = &PROGRAM_START as *const u8 as usize;
// ```
extern "C" {
    static PROGRAM_START: u8;
    pub static PROGRAM_END: u8;
    static PROGRAM_SIZE: u8;
}

/// The current pointer used by the bump allocator
static mut BUMP_ALLOC_PTR: Option<*const u8> = None;
const BUMP_ALLOC_ALIGNMENT: usize = 8;

/// An extremely simple bump allocator.
///
/// Starts at a base address and increments the current pointer for each allocation. Never frees the
/// allocations. Runs out of memory very quickly and should only be used for testing purposes.
struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if BUMP_ALLOC_PTR.is_none() {
            return ptr::null_mut();
        }

        let mut alloc_ptr = BUMP_ALLOC_PTR.unwrap();

        // TODO: This allocator needs to return a null pointer if it does not have enough memory for the
        //       allocation. This requires the boot loader to know where the memory ends. Not sure how this
        //       is done on ARM, yet.

        // Ensure pointer is aligned
        let offset = alloc_ptr.align_offset(BUMP_ALLOC_ALIGNMENT);
        alloc_ptr = alloc_ptr.add(offset);

        // Ensure that pointer is aligned according to `layout`
        if layout.align() > BUMP_ALLOC_ALIGNMENT {
            let offset = alloc_ptr.align_offset(layout.align());

            // Return null if the alignment is invalid
            if offset == usize::MAX {
                return ptr::null_mut();
            }

            // Offset the pointer so that it's properly aligned
            alloc_ptr = alloc_ptr.add(offset);
        }

        // Save the pointer to return later
        let allocated = alloc_ptr;

        // Bump the current pointer by the allocation's size
        // TODO: Panic if the end of RAM is reached
        BUMP_ALLOC_PTR = Some(alloc_ptr.add(layout.size()));

        debug!("ALLOC@{:p} with size: {:#x} and align: {}", allocated, layout.size(), layout.align());

        allocated as *mut u8
    }

    // No deallocations ever take place
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[repr(packed)]
pub struct Pl011Uart {
    data: Mmio<u8>,
}

impl Pl011Uart {
    /// Returns a [`Pl011Uart`] reference using a `base` address
    ///
    /// # Safety
    ///
    /// It is unsafe to use the referenced [`Pl011Uart`] because there could be an already existing reference.
    /// If multiple references to a single Uart exist, the owner of each reference could overwrite the registers
    /// used by the other reference.
    ///
    /// It should be ensured that when using this function, that another reference does not already exist.
    ///
    /// One exception to this rule is during a panic. As nothing else will be running, the panic handler
    /// is allowed to use this for re-initializing a Uart so the panic log can be somewhat reliably
    /// written to it. Note that this exception may not hold up if it's being used in multiple threads, as the
    /// threads might panic separately.
    pub unsafe fn new(base: usize) -> &'static mut Pl011Uart {
        &mut *(base as *mut Pl011Uart)
    }
}

impl Write for Pl011Uart {
    fn write_str(&mut self, out_string: &str) -> fmt::Result {
        for out_byte in out_string.bytes() {
            self.data.write(out_byte);
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
    uart: UnsafeCell<&'static mut Pl011Uart>,
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

        // Write log level and args
        write!(uart, "[{}] {}", record.level().as_str(), record.args()).unwrap();

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
    // Re-initialize UART0 and print a panic log
    // TODO: Try to use already existing UART so that no stack allocation occurs
    let uart = unsafe { Pl011Uart::new(UART0_ADDR) };
    // TODO: Maybe halt if this returns an error
    writeln!(uart, "[PANIC] {}", info).unwrap();
    loop {}
}

#[alloc_error_handler]
fn handle_out_of_memory(layout: core::alloc::Layout) -> ! {
    // Try to panic with the current logger
    panic!("Out of memory! {:#?}", layout);
}

// The default logger
static mut LOGGER: Option<UartPl011Logger> = None;

#[no_mangle]
#[link_section = ".text.boot"]
pub unsafe extern "C" fn qemu_entry() {
    // TODO: Read memory range from DTB file
    let memory_range: MemoryRange = MemoryRange {
        // Starts at the end of the bootloader program
        start: &PROGRAM_END as *const u8 as usize,
        // 128MB
        size: 0x800_0000
    };

    // Ensure that there's at least two 4KB pages of memory
    if memory_range.size < 0x2000 {
        panic!("There is not enough memory for any allocations");
    }

    // Initialize UART0
    // The only other place it should be initialized is during a panic for emergency serial output
    let uart = unsafe { Pl011Uart::new(UART0_ADDR) };

    // Initialize the free memory list
    FREE_MEMORY = {
        let mut free_memory = IntrusiveList {
            allocated_nodes: memory_range.start as *const FreeMemoryChunk,
            capacity: memory_range.size / 0x1000
        };

        let memory_list_size = free_memory.capacity * core::mem::size_of::<FreeMemoryChunk>();

        *(free_memory.allocated_nodes as *mut FreeMemoryChunk) = FreeMemoryChunk {
            data: (memory_range.start + memory_list_size) as *const u8,
            node: None
        };

        Some(free_memory)
    };

    // Initialize logger using UART0
    let logger = {
        LOGGER = Some(UartPl011Logger { uart: uart.into() });
        LOGGER.as_ref().unwrap()
    };
    log::set_logger(logger).unwrap();
    log::set_max_level(LevelFilter::Debug);
    info!("Default logger is UART at address: {:#x}", UART0_ADDR);

    // Print out program address and size
    debug!("PROGRAM_START: {:p}", &PROGRAM_START);
    debug!("PROGRAM_END  : {:p}", &PROGRAM_END);
    debug!("PROGRAM_SIZE : {:#x}", &PROGRAM_SIZE as *const u8 as usize);

    if let Some(free_memory) = &FREE_MEMORY {
        let first_chunk = (*free_memory.allocated_nodes).data;
        debug!("First chunk: {:#?}", first_chunk);

        BUMP_ALLOC_PTR = Some(first_chunk);
    }

    // Test out allocator
    let v1 = vec!['a', 'b', 'c', 'd'];
    for (i, n) in v1.iter().enumerate() {
        debug!("{} {}", i, n);
    }
    let v2 = vec!['w', 'x', 'y', 'z'];
    for (i, n) in v2.iter().enumerate() {
        debug!("{} {}", i, n);
    }
    debug!("Original:");
    for (i, n) in v1.iter().enumerate() {
        debug!("{} {}", i, n);
    }

    // TODO: Run kernel
    panic!("End of bootloader reached. Press 'CTRL+A' and then 'X' to exit.");
}
