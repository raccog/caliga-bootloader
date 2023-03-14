// TODO: Ensure CPUID is supported by attempting to set bit 21 of EFLAGS
use core::arch::x86_64::{__get_cpuid_max, CpuidResult, __cpuid_count};

#[cfg(not(test))]
use log::debug;
#[cfg(test)]
use std::println as debug;

/// Returns the maximum values for CPUID basic and extended functions, respectively.
pub unsafe fn cpuid_max_values() -> (u32, u32) {
    let (basic, _) = __get_cpuid_max(0);
    let (extended, _) = __get_cpuid_max(0x8000_0000);

    (basic, extended)
}

/// Returns the addressing width in bits of the physical and linear addresses, respectively.
pub unsafe fn cpuid_address_width() -> (u8, u8) {
    let CpuidResult {eax, ..} = __cpuid_count(0x8000_0008, 0);
    let [physical, linear, ..] = eax.to_le_bytes();
    (physical, linear)
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
    fn max_values() {
        let (basic, extended) = unsafe { cpuid_max_values() };
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

    /// A simple check to ensure that `cpuid_address_width`'s result is in a valid range.
    /// 
    /// The physical addressing witdh should be in between 40 and 64 bits. Modern processors would likely use either
    /// 48 or 52 bits. Older AMD64 processors (and qemu-system-x86_64) might use 40 bits. The max is kept at 64 bits
    /// in case future processors use more than 52 bits.
    /// 
    /// The linear addressing width should be in between 48 and 64 bits. Just like the physical address witdh, the
    /// max is kept at 64 bits.
    #[test]
    fn address_width() {
        let (physical, linear) = unsafe { cpuid_address_width() };
        debug!("Addressing Width {{ physical: {}, linear: {} }}", physical, linear);
        const MIN_PHYSICAL: u8 = 40;
        const MAX_PHYSICAL: u8 = 64;
        const MIN_LINEAR: u8 = 48;
        const MAX_LINEAR: u8 = 64;
        assert!(physical >= MIN_PHYSICAL && physical <= MAX_PHYSICAL);
        assert!(linear >= MIN_LINEAR && linear <= MAX_LINEAR);
    }
}
