use crate::{
    filesystem::OpenFileError, FileDescriptor, FileDescriptorInterface, FileSystemInterface,
};

use alloc::{boxed::Box, string::String, vec, vec::Vec};
use log::info;
use uefi::{
    data_types::chars::NUL_16,
    prelude::*,
    proto::media::file::{
        Directory, File, FileAttribute, FileInfo, FileMode, FileType, RegularFile,
    },
    CStr16, CString16, Char16,
};

// Both of these lengths do not include the null terminator
const MAX_PATH_LEN: usize = 32760;
const COMPONENT_MAX_LEN: usize = 255;

pub struct UefiFileDescriptorDriver {}

impl FileDescriptorInterface for UefiFileDescriptorDriver {}

// Start with a very low maximum
pub const MAX_OPENED_FILES: usize = 5;

pub struct UefiSimpleFileSystemDriver {
    pub root_directory: Directory,
    pub opened_files: [Option<FileDescriptor>; MAX_OPENED_FILES],
    pub uefi_descriptors: [Option<RegularFile>; MAX_OPENED_FILES],
}

impl FileSystemInterface for UefiSimpleFileSystemDriver {
    unsafe fn open_file(&mut self, path: &str) -> Result<*mut FileDescriptor, OpenFileError> {
        // Return error if file is already opened
        for slot in self.opened_files.iter() {
            if let Some(descriptor) = slot {
                if descriptor.path == path {
                    return Err(OpenFileError::AlreadyOpen);
                }
            }
        }

        // Convert to UCS2
        let uefi_path = CString16::try_from(path).map_err(|_| OpenFileError::InvalidCharset)?;
        let path_slice = uefi_path.as_slice_with_nul();

        // Ensure path is not too long
        if path_slice.len() > MAX_PATH_LEN + 1 {
            return Err(OpenFileError::PathTooLong);
        }

        // Split path into components
        let path_components = split_path(&path_slice);

        let mut dirname_buf: [Char16; COMPONENT_MAX_LEN + 1] = [NUL_16; COMPONENT_MAX_LEN + 1];
        let mut current_dir_owned: Directory;
        let mut current_dir = &mut self.root_directory;
        for (idx, &entry) in path_components.iter().enumerate() {
            // Ensure component name is not too long
            if entry.len() > COMPONENT_MAX_LEN {
                return Err(OpenFileError::ComponentTooLong);
            }

            // Copy next component into buffer
            dirname_buf[..entry.len()].copy_from_slice(entry);
            dirname_buf[entry.len()] = Char16::try_from(0).unwrap();
            let filename = unsafe {
                CStr16::from_u16_with_nul_unchecked(
                    &*(&dirname_buf[..entry.len() + 1] as *const [Char16] as *const [u16]),
                )
            };

            // Open file using UEFI protocol
            let should_be_file = idx == path_components.len() - 1;
            let handle = current_dir
                .open(filename, FileMode::Read, FileAttribute::READ_ONLY)
                .map_err(|open_err| match open_err.status() {
                    Status::NOT_FOUND => {
                        if should_be_file {
                            OpenFileError::FileNotFound
                        } else {
                            OpenFileError::DirectoryNotFound
                        }
                    }
                    Status::NO_MEDIA | Status::MEDIA_CHANGED | Status::DEVICE_ERROR => {
                        OpenFileError::DeviceError
                    }
                    Status::VOLUME_CORRUPTED => OpenFileError::FileSystemCorrupted,
                    Status::ACCESS_DENIED => OpenFileError::AccessDenied,
                    Status::OUT_OF_RESOURCES => {
                        panic!("Could not open file at {}, out of resources!", path);
                    }
                    _ => {
                        panic!("Unknown error: {:?}", open_err);
                    }
                })?
                .into_type()
                .map_err(|get_position_err| match get_position_err.status() {
                    Status::DEVICE_ERROR => OpenFileError::DeviceError,
                    _ => {
                        panic!("Unknown error: {:?}", get_position_err);
                    }
                })?;

            // Determine whether opened file is a directory or a regular file.
            // Return any unexpected errors
            if let FileType::Dir(next_dir) = handle {
                if should_be_file {
                    return Err(OpenFileError::IsDirectory);
                }
                current_dir_owned = next_dir;
                current_dir = &mut current_dir_owned;
            } else if let FileType::Regular(file) = handle {
                if !should_be_file {
                    return Err(OpenFileError::IsFile);
                }

                let mut descriptor_index: Option<usize> = None;
                for (index, slot) in self.opened_files.iter_mut().enumerate() {
                    if slot.is_none() {
                        descriptor_index = Some(index);
                        break;
                    }
                }
                let index = if descriptor_index.is_none() {
                    return Err(OpenFileError::TooManyOpenFiles);
                } else {
                    descriptor_index.unwrap()
                };
                self.opened_files[index] = Some(FileDescriptor {
                    index,
                    offset: 0,
                    path: String::from(path),
                    driver: Box::new(UefiFileDescriptorDriver {}),
                });
                self.uefi_descriptors[index] = Some(file);
                // TODO: Research safety of casting raw pointers to be mutable
                return Ok(
                    self.opened_files[index].as_ref().unwrap() as *const FileDescriptor
                        as *mut FileDescriptor,
                );
            }
        }

        Err(OpenFileError::FileNotFound)
    }

    unsafe fn close(&mut self, fd: *mut FileDescriptor) {
        assert!(!fd.is_null());
        let index = (*fd).index;
        assert!(index < MAX_OPENED_FILES);
        if let Some(other_fd) = &self.opened_files[index] {
            if other_fd.path == (*fd).path {
                self.opened_files[index] = None;
                self.uefi_descriptors[index] = None;
                return;
            }
        }
        info!("Could not close file at: {}", (*fd).path);
    }

    unsafe fn read_file(
        &self,
        fd: *mut FileDescriptor,
        buf: &mut [u8],
    ) -> Result<usize, usize> {
        assert!(!fd.is_null());
        let index = (*fd).index;
        assert!(index < MAX_OPENED_FILES);
        let uefi_descriptor = self.uefi_descriptors[index].as_ref().unwrap() as *const RegularFile
            as *mut RegularFile;
        let read_result = (*uefi_descriptor).read(buf);
        match read_result {
            Ok(bytes_read) => Ok(bytes_read),
            Err(maybe_bytes_read) => {
                if let Some(bytes_read) = maybe_bytes_read.data() {
                    Err(*bytes_read)
                } else {
                    Err(0)
                }
            }
        }
    }

    unsafe fn seek(&self, fd: *mut FileDescriptor, location: u64) -> Result<(), ()> {
        assert!(!fd.is_null());
        let index = (*fd).index;
        assert!(index < MAX_OPENED_FILES);
        let uefi_descriptor = self.uefi_descriptors[index].as_ref().unwrap() as *const RegularFile
            as *mut RegularFile;
        let set_position_result = (*uefi_descriptor).set_position(location);
        match set_position_result {
            Ok(_) => Ok(()),
            Err(_) => Err(())
        }
    }

    unsafe fn get_size(&self, descriptor: *mut FileDescriptor) -> Result<u64, ()> {
        assert!(!descriptor.is_null());
        let index = (*descriptor).index;
        assert!(index < MAX_OPENED_FILES);
        let uefi_descriptor = self.uefi_descriptors[index].as_ref().unwrap() as *const RegularFile
            as *mut RegularFile;
        let file_info_result = (*uefi_descriptor).get_boxed_info::<FileInfo>();
        match file_info_result {
            Ok(file_info) => Ok(file_info.file_size()),
            Err(_) => Err(()),
        }
    }
}

/// Split a path into its individual components.
///
/// Each component is split up by the '/' character. Empty components are omitted.
fn split_path(path_slice: &[Char16]) -> Vec<&[Char16]> {
    let separator = Char16::try_from('/').unwrap();
    let mut path_components: Vec<&[Char16]> = vec![];
    let mut prev_separator: usize = 0;
    for (idx, c) in path_slice.iter().enumerate() {
        if *c == separator || *c == NUL_16 {
            if idx > prev_separator + 1 {
                path_components.push(&path_slice[prev_separator + 1..idx]);
            }
            prev_separator = idx;
        }
    }
    path_components
}
