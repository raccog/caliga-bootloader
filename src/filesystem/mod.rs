use alloc::{boxed::Box, string::String};

pub trait FileDescriptorInterface {}

pub struct FileDescriptor {
    pub index: usize,
    pub offset: u64,
    pub path: String,
    pub driver: Box<dyn FileDescriptorInterface>,
}

pub trait FileSystemInterface {
    unsafe fn open_file(&mut self, _path: &str) -> Result<*mut FileDescriptor, OpenFileError> {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn close(&mut self, _fd: *mut FileDescriptor) -> Result<(), ()> {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn read_file(&self, _fd: *mut FileDescriptor, _buf: &mut [u8]) -> Result<usize, usize> {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn seek(&self, _fd: *mut FileDescriptor, _location: u64) -> Result<(), ()> {
        panic!("NOT IMPLEMENTED");
    }

    unsafe fn get_size(&self, _fd: *mut FileDescriptor) -> Result<u64, ()> {
        panic!("NOT IMPLEMENTED");
    }
}

pub struct FileSystem {
    pub index: u32,
    pub driver: Box<dyn FileSystemInterface>,
}

impl FileSystemInterface for FileSystem {
    unsafe fn open_file(&mut self, path: &str) -> Result<*mut FileDescriptor, OpenFileError> {
        self.driver.open_file(path)
    }

    unsafe fn close(&mut self, fd: *mut FileDescriptor) -> Result<(), ()> {
        self.driver.close(fd)
    }

    unsafe fn read_file(&self, fd: *mut FileDescriptor, buf: &mut [u8]) -> Result<usize, usize> {
        self.driver.read_file(fd, buf)
    }

    unsafe fn seek(&self, fd: *mut FileDescriptor, location: u64) -> Result<(), ()> {
        self.driver.seek(fd, location)
    }

    unsafe fn get_size(&self, fd: *mut FileDescriptor) -> Result<u64, ()> {
        self.driver.get_size(fd)
    }
}

/// An error returned from opening a file.
#[derive(Debug)]
pub enum OpenFileError {
    /// The opened path is too long to be valid for this filesystem.
    PathTooLong,
    /// One of the path's components is too long to be valid for this filesystem.
    ComponentTooLong,
    /// The opened path cannot be converted into the proper charset.
    InvalidCharset,
    /// The opened file was not found on the filesystem.
    FileNotFound,
    /// An error occurred while reading from this filesystem's device.
    DeviceError,
    /// Access was denied to this file
    AccessDenied,
    /// The filesystem state has been corrupted.
    FileSystemCorrupted,
    /// A directory on the path to the opened file was not found on the filesystem.
    DirectoryNotFound,
    /// Tried to open a file as a directory.
    IsFile,
    /// Tried to open a directory as a normal file.
    IsDirectory,
    /// Tried to open a file when the maximum number of files are already opened.
    TooManyOpenFiles,
    /// This file is already opened and has not yet been closed
    AlreadyOpen,
}
