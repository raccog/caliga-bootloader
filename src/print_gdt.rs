use core::{convert::TryFrom, ptr, slice};
use log::debug;

#[derive(Debug, Copy, Clone)]
enum GdtrError {
    InvalidPointer,
    InvalidLimit
}

#[derive(Debug, Copy, Clone)]
#[repr(packed)]
struct Gdtr {
    limit: u16,
    base: u64
}

impl Gdtr {
    pub fn base(&self) -> u64 {
        self.base
    }

    pub unsafe fn from_array_unchecked(gdtr: [u8; 10]) -> Self {
        *( gdtr.as_ptr() as *const Gdtr )
    }

    pub fn limit(&self) -> u16 {
        self.limit
    }
}

impl TryFrom<[u8; 10]> for Gdtr {
    type Error = GdtrError;

    fn try_from(gdtr: [u8; 10]) -> Result<Self, Self::Error> {
        let gdtr = unsafe { Self::from_array_unchecked(gdtr) };
        if gdtr.limit % 8 != 7 {
            return Err(GdtrError::InvalidLimit);
        }
        if gdtr.base == 0 {
            return Err(GdtrError::InvalidPointer);
        }

        Ok(gdtr)
    }
}

pub fn print_gdt(gdtr: [u8; 10]) {
    let gdtr = Gdtr::try_from(gdtr).expect("Tried to print invalid GDT");
    debug!("GDTR Base: {:#x}, Limit: {:#x}", gdtr.base(), gdtr.limit());

    unsafe {
        let gdt: *const u64 = (ptr::null() as *const u64).with_addr(gdtr.base() as usize);
        debug!("GDT: {:?}", gdt);
        let gdt = slice::from_raw_parts(gdt, (gdtr.limit() as usize + 1) / 8);
        for (i, descriptor) in gdt.iter().enumerate() {
            debug!("GDT[{}]: {:#x}", i + 1, descriptor);
        }
    }
}
