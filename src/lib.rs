#![cfg_attr(not(test), no_std)]
#![feature(allocator_api)]
#![feature(negative_impls)]
#![feature(ptr_sub_ptr)]
#![feature(pointer_byte_offsets)]
#![feature(pointer_is_aligned)]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

pub mod io;

pub mod common;
