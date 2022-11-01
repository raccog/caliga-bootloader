#![no_std]

extern crate alloc;
extern crate lazy_static;

pub mod file_system;

pub use file_system::{open_file, OpenFileError};
