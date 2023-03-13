pub struct PhysicalAddress(usize);

pub struct VirtualAddress(usize);

pub struct InvalidPhysicalAddress(usize);

pub struct InvalidVirtualAddress(usize);

// impl PhysicalAddress {
//     pub unsafe fn new(addr: usize) -> Self {
//     }

//     pub fn try_new(addr: usize) -> Result<Self, InvalidPhysicalAddress> {

//     }
// }
