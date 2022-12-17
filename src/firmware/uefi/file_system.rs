use crate::filesystem::{FileDescriptor, FileSystem, OpenFileError};

use alloc::{vec, vec::Vec};
use log::debug;
use uefi::{
    data_types::chars::NUL_16,
    proto::media::file::{Directory, File, FileAttribute, FileMode, FileType},
    CStr16, CString16, Char16, Status,
};

// Both of these lengths do not include the null terminator
const MAX_PATH_LEN: usize = 32760;
const COMPONENT_MAX_LEN: usize = 255;

/// A [`FileSystem`] implementation for the UEFI protocol EFI_SIMPLE_FILE_SYSTEM_PROTOCOL.
///
/// This implementation reads the FAT filesystem that was booted from. All paths are converted to the
/// UCS2 charset.
pub struct UefiSimpleFilesystem {
    pub root_dir: Directory,
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

impl FileSystem for UefiSimpleFilesystem {
    fn open_file(&mut self, path: &str) -> Result<FileDescriptor, OpenFileError> {
        // Convert to UCS2
        let uefi_path = CString16::try_from(path).map_err(|_| OpenFileError::InvalidCharset)?;
        let path_slice = uefi_path.as_slice_with_nul();

        // Ensure path is not too long
        if path_slice.len() > MAX_PATH_LEN + 1 {
            return Err(OpenFileError::PathTooLong);
        }

        // Split path into components
        let path_components = split_path(&path_slice);
        debug!("Path Vector: {:?}", path_components);

        let mut dirname_buf: [Char16; COMPONENT_MAX_LEN + 1] = [NUL_16; COMPONENT_MAX_LEN + 1];
        let mut current_dir_owned: Directory;
        let mut current_dir = &mut self.root_dir;
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
                log::debug!("Dir: {}", filename);
                if should_be_file {
                    return Err(OpenFileError::IsDirectory);
                }
                current_dir_owned = next_dir;
                current_dir = &mut current_dir_owned;
            } else if let FileType::Regular(_file) = handle {
                log::debug!("File: {}", filename);
                if !should_be_file {
                    return Err(OpenFileError::IsFile);
                }
                // TODO: Include UEFI file protocol in FileDescriptor struct
                return Ok(FileDescriptor { size: 0 });
            }
        }

        Err(OpenFileError::FileNotFound)
    }

    fn close_file(&mut self, _descriptor: FileDescriptor) {
        panic!("NOT IMPLEMENTED");
    }

    fn read_file(&mut self, _descriptor: &mut FileDescriptor, _buf: &mut [u8], _count: usize) {
        panic!("NOT IMPLEMENTED");
    }

    fn seek_file(&mut self, _descriptor: &mut FileDescriptor, _location: u64) {
        panic!("NOT IMPLEMENTED");
    }
}
