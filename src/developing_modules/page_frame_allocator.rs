use crate::developing_modules::addressing::PhysicalAddress;

pub struct PageSize {
    exponent: usize
}

pub struct PageFrame {
    start: PhysicalAddress,

}