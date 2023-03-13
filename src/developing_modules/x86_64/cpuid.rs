// TODO: Ensure CPUID is supported by attempting to set bit 21 of EFLAGS
use core::arch::x86_64::__get_cpuid_max;

#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

#[derive(Debug)]
pub struct CpuidMaxValues {
    pub basic: u32,
    pub extended: u32
}

pub unsafe fn cpuid_max_values() -> CpuidMaxValues {
    let (basic, _) = __get_cpuid_max(0);
    let (extended, _) = __get_cpuid_max(0x8000_0000);

    CpuidMaxValues { basic, extended }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple check to ensure that `cpuid_max_values`'s result is in a valid range.
    /// 
    /// Since the max values are different on each CPU, this just ensures that the basic
    /// max value is not 0 and is less than 0x8000_0000 (start of extended values). The
    /// extended max value is checked to be greater than 0x8000_0000, but less than 0x9000_0000.
    /// The value 0x9000_0000 was chosen because it seems that the latest Intel and AMD processors
    /// don't use anything past that value.
    #[test]
    fn cpuid_max() {
        let CpuidMaxValues {basic, extended} = unsafe { cpuid_max_values() };
        debug!("CPUID Max Values {{ basic: {:#x}, extended: {:#x} }}", basic, extended);
        const EXTENDED_MIN: u32 = 0x8000_0000;
        const EXTENDED_MAX: u32 = 0x9000_0000;
        assert!(basic > 0 && basic < EXTENDED_MIN);
        assert!(extended > EXTENDED_MIN && extended < EXTENDED_MAX);
    }
}