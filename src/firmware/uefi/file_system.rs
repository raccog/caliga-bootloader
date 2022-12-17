use crate::filesystem::{self, FileDescriptor, FileSystem};

use alloc::{vec, vec::Vec};
use log::debug;
use uefi::{
    proto::media::file::{Directory, File, FileAttribute, FileMode, FileType},
    CStr16, CString16, Char16, Status,
};

pub struct UefiSimpleFilesystem {
    pub root_dir: Directory,
}

/// Split a path by the '/' character.
fn split_path(path: &CStr16) -> Vec<&[Char16]> {
    let path_slice = path.as_slice_with_nul();
    let separator = Char16::try_from('/').unwrap();
    let null_terminator = Char16::try_from('\0').unwrap();
    let mut path_vec: Vec<&[Char16]> = vec![];
    let mut prev_separator: usize = 0;
    for (idx, c) in path_slice.iter().enumerate() {
        if *c == separator || *c == null_terminator {
            if idx > prev_separator + 1 {
                path_vec.push(&path_slice[prev_separator + 1..idx]);
            }
            prev_separator = idx;
        }
    }
    path_vec
}

impl FileSystem for UefiSimpleFilesystem {
    fn open_file(&mut self, path: &str) -> Result<FileDescriptor, filesystem::OpenFileError> {
        // Convert to UCS2
        let uefi_path =
            CString16::try_from(path).map_err(|_| filesystem::OpenFileError::InvalidCharset)?;

        // Split path into array of file names
        let path_vec = split_path(&uefi_path);
        debug!("Path Vector: {:?}", path_vec);

        // TODO: Move Char16 null character to a constant
        const FILE_NAME_MAX_LEN: usize = 255;
        let mut dirname_buf: [Char16; FILE_NAME_MAX_LEN + 1] =
            [Char16::try_from(0).unwrap(); FILE_NAME_MAX_LEN + 1];

        let mut current_dir_owned: Directory;
        let mut current_dir = &mut self.root_dir;
        for (idx, entry) in path_vec.iter().enumerate() {
            // Copy next file name into buffer
            // TODO: Ensure that file name is not too long
            dirname_buf[..entry.len()].copy_from_slice(entry);
            dirname_buf[entry.len()] = Char16::try_from(0).unwrap();
            let filename = unsafe {
                CStr16::from_u16_with_nul_unchecked(
                    &*(&dirname_buf[..entry.len() + 1] as *const [Char16] as *const [u16]),
                )
            };

            let should_be_file = idx == path_vec.len() - 1;
            let handle = current_dir
                .open(filename, FileMode::Read, FileAttribute::READ_ONLY)
                .map_err(|uefi_err| match uefi_err.status() {
                    // TODO: Ensure all errors are accounted for
                    Status::NOT_FOUND => {
                        if should_be_file {
                            filesystem::OpenFileError::FileNotFound
                        } else {
                            filesystem::OpenFileError::DirectoryNotFound
                        }
                    }
                    Status::NO_MEDIA | Status::MEDIA_CHANGED | Status::DEVICE_ERROR => {
                        filesystem::OpenFileError::DeviceError
                    }
                    Status::VOLUME_CORRUPTED => filesystem::OpenFileError::FileSystemCorrupted,
                    Status::ACCESS_DENIED => filesystem::OpenFileError::AccessDenied,
                    Status::OUT_OF_RESOURCES => {
                        panic!("Could not open file at {}, out of resources!", path);
                    }
                    _ => {
                        panic!("Unknown error: {:?}", uefi_err);
                    }
                })?
                .into_type()
                .map_err(|uefi_err| match uefi_err.status() {
                    Status::DEVICE_ERROR => filesystem::OpenFileError::DeviceError,
                    _ => {
                        panic!("Unknown error: {:?}", uefi_err);
                    }
                })?;

            if let FileType::Dir(next_dir) = handle {
                log::debug!("Dir: {}", filename);
                if should_be_file {
                    return Err(filesystem::OpenFileError::IsDirectory);
                }
                current_dir_owned = next_dir;
                current_dir = &mut current_dir_owned;
            } else if let FileType::Regular(_file) = handle {
                log::debug!("File: {}", filename);
                if !should_be_file {
                    return Err(filesystem::OpenFileError::IsFile);
                }
                // TODO: Include UEFI file protocol in FileDescriptor struct
                return Ok(FileDescriptor { size: 0 });
            }
        }

        Err(filesystem::OpenFileError::FileNotFound)
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
