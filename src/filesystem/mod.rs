#[derive(Debug)]
pub struct FileDescriptor {
    pub size: u64,
}

pub trait FileSystem {
    fn open_file(&mut self, path: &str) -> Result<FileDescriptor, OpenFileError>;

    fn close_file(&mut self, descriptor: FileDescriptor);

    fn read_file(&mut self, descriptor: &mut FileDescriptor, buf: &mut [u8], count: usize);

    fn seek_file(&mut self, descriptor: &mut FileDescriptor, location: u64);
}

#[derive(Debug)]
pub enum OpenFileError {
    InvalidPath,
    InvalidCharset,
    NotFound,
    DeviceError,
    FileSystemCorrupted,
    AccessDenied,
}
