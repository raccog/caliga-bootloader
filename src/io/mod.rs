/// Implementation of IO is inspired by the following:
///
/// * IO trait from RedoxOS https://gitlab.redox-os.org/redox-os/syscall/-/blob/master/src/io
/// * volatile_register crate https://docs.rs/volatile-register/latest/volatile_register/
/// * Tock's tock_register crate https://github.com/tock/tock/blob/master/libraries/tock-register-interface

pub mod io;
pub mod mmio;
