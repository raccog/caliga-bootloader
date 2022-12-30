use crate::BlockDeviceInterface;

struct UefiStorageDeviceDriver {
    system_table: usize,
}

impl BlockDeviceInterface for UefiStorageDeviceDriver {}