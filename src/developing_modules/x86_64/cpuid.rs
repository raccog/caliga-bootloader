// TODO: Ensure CPUID is supported by attempting to set bit 21 of EFLAGS
use core::arch::x86_64::__get_cpuid_max;

#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

#[derive(Debug)]
pub struct CpuidMaxValues {
    pub basic: u32,
    pub extended: u32,
}

/// Returns the maximum values for CPUID basic and extended functions.
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
    /// max value is greater than 0 and is less than 0x1000_0000. The
    /// extended max value is checked to be greater than 0x8000_0000 and less than 0x9000_0000.
    ///
    /// These ranges were chosen because the Intel and AMD manuals do not list any CPUID leaves
    /// that are out of range.
    #[test]
    fn cpuid_max() {
        let CpuidMaxValues { basic, extended } = unsafe { cpuid_max_values() };
        debug!(
            "CPUID Max Values {{ basic: {:#x}, extended: {:#x} }}",
            basic, extended
        );
        const BASIC_MIN: u32 = 0;
        const BASIC_MAX: u32 = 0x1000_0000;
        const EXTENDED_MIN: u32 = 0x8000_0000;
        const EXTENDED_MAX: u32 = 0x9000_0000;
        assert!(basic > BASIC_MIN && basic < BASIC_MAX);
        assert!(extended > EXTENDED_MIN && extended < EXTENDED_MAX);
    }
}
