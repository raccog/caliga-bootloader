#![no_std]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

pub mod file_system;
pub mod print_gdt;

pub use file_system::{open_file, OpenFileError};
