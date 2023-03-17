use core::arch::asm;

/// Returns the physical addressing width in bits.
/// 
/// # Panic
/// 
/// Panics if the retrieved system addressing width is invalid.
pub unsafe fn physical_address_width() -> u8 {
    let result: u64;
    asm!("mrs {result}, ID_AA64MMFR0_EL1",
         result = out(reg) result);

    let [physical_range, ..] = result.to_le_bytes();
    let physical_range = physical_range & 0xf;
    match physical_range {
        0b0000 => 32,
        0b0001 => 36,
        0b0010 => 40,
        0b0011 => 42,
        0b0100 => 44,
        0b0101 => 48,
        0b0110 => 52,
        _ => panic!("Invalid address width for Aarch64: {}", physical_range)
    }
}

#[derive(Debug)]
pub enum ExceptionLevel {
    EL0 = 0b00,
    EL1 = 0b01,
    EL2 = 0b10,
    EL3 = 0b11
}

pub unsafe fn current_exception_level() -> ExceptionLevel {
    let result: u64;
    asm!("mrs {result}, CurrentEL",
         result = out(reg) result);

    let [exception_level, ..] = result.to_le_bytes();
    let exception_level = exception_level >> 2;
    match exception_level {
        0b00 => ExceptionLevel::EL0,
        0b01 => ExceptionLevel::EL1,
        0b10 => ExceptionLevel::EL2,
        0b11 => ExceptionLevel::EL3,
        _ => panic!("Invalid exception level for Aarch64: {}", exception_level)
    }
}