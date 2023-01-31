#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
#![feature(ptr_sub_ptr)]
#![feature(pointer_is_aligned)]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

pub mod arch;
pub mod filesystem;
pub mod firmware;
pub mod io;

pub mod common;
