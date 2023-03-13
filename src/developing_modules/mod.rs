//! These modules are all in development.
//!
//! They will likely go through many changes before being included included in the main module tree.

pub mod addressing;
pub mod io;
pub mod mmio;
//pub mod physical_allocator;
pub mod page_frame_allocator;
pub mod slab_allocator;

#[cfg(target_arch = "x86_64")]
pub mod x86_64;