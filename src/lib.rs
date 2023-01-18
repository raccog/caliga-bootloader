#![cfg_attr(not(test), no_std)]
#![feature(strict_provenance)]

extern crate alloc;
extern crate lazy_static;

pub mod arch;
pub mod filesystem;
pub mod firmware;
pub mod io;

pub mod intrusive_list;
