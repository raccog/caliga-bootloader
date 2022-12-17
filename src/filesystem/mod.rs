pub struct FileDescriptor {
    pub size: u64,
}

pub trait FileSystem {
    fn open_file(&mut self, filename: &str) -> FileDescriptor;

    fn close_file(&mut self, descriptor: FileDescriptor);

    fn read_file(&mut self, descriptor: &mut FileDescriptor, buf: &mut [u8], count: usize);

    fn seek_file(&mut self, descriptor: &mut FileDescriptor, location: u64);
}
