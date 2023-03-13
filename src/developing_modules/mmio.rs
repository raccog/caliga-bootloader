/// This implementation is a shortened version of the RedoxOS implementation found here:
///
/// https://gitlab.redox-os.org/redox-os/syscall/-/blob/master/src/io/mmio.rs
use core::{
    mem::MaybeUninit,
    ptr::{addr_of, addr_of_mut, read_volatile, write_volatile},
};

use crate::developing_modules::io::Io;

#[repr(packed)]
pub struct Mmio<T> {
    value: MaybeUninit<T>,
}

impl<T> Io for Mmio<T>
where
    T: Copy + PartialEq,
{
    type Value = T;

    fn read(&self) -> T {
        unsafe { read_volatile(addr_of!(self.value).cast::<T>()) }
    }

    fn write(&mut self, value: T) {
        unsafe { write_volatile(addr_of_mut!(self.value).cast::<T>(), value) };
    }
}
