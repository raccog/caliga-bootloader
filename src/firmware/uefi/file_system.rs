use crate::filesystem::{FileDescriptor, FileSystem};

pub struct UefiSimpleFilesystem {}

impl FileSystem for UefiSimpleFilesystem {
    fn open_file(&mut self, filename: &str) -> FileDescriptor {
        panic!("NOT IMPLEMENTED");
    }

    fn close_file(&mut self, descriptor: FileDescriptor) {
        panic!("NOT IMPLEMENTED");
    }

    fn read_file(&mut self, descriptor: &mut FileDescriptor, buf: &mut [u8], count: usize) {
        panic!("NOT IMPLEMENTED");
    }

    fn seek_file(&mut self, descriptor: &mut FileDescriptor, location: u64) {
        panic!("NOT IMPLEMENTED");
    }
}

use alloc::{fmt, vec::Vec};
use log::debug;
use uefi::{
    self,
    data_types::chars::Char16,
    proto::media::file::{Directory, File, FileAttribute, FileMode, FileType, RegularFile},
    CStr16, CString16,
};

/// The error type returned from [`open_file`].
#[derive(Debug)]
pub enum OpenFileError<'a> {
    /// A UEFI error occurred while opening a file.
    ///
    /// Contains an error returned from a UEFI protocol and the path it occurred at.
    UefiFailure(uefi::Error, &'a [Char16]),
    /// The path contains an empty file name.
    EmptyFileName,
    /// The path contains a file (or directory) name that is longer than 255 characters.
    ///
    /// Contains a path to the long file name.
    FileNameTooLong(&'a [Char16]),
    /// A file was attemepted to be opened as a directory.
    ///
    /// Contains a path to this invalid directory.
    NotADirectory(&'a [Char16]),
    /// A directory was attempted to be opened as a file.
    NotAFile,
}

impl fmt::Display for OpenFileError<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: Question: What happens during an out-of-memory error?
        use OpenFileError::*;
        match self {
            UefiFailure(error, path) => {
                let path_str = allocate_str(path);
                // TODO: Properly format UEFI error
                write!(f, "UEFI error at {}: {:?}", path_str, error)
            }
            EmptyFileName => {
                write!(f, "Attempted to open a path with an empty file name")
            }
            FileNameTooLong(path) => {
                let path_str = allocate_str(path);
                write!(f, "File name too long: {}", path_str)
            }
            NotADirectory(path) => {
                let path_str = allocate_str(path);
                write!(
                    f,
                    "A file was attempted to be opened as a directory: {}",
                    path_str
                )
            }
            NotAFile => {
                write!(f, "A directory was attempted to be opened as a file")
            }
        }
    }
}

// TODO: Allow non-abolute paths with `root_dir` as the current working directory.
/// Attempts to open a file using an absolute `path` starting from `root_dir`.
///
/// # Errors
///
/// Each [`OpenFileError`] variant could be returned. See [`OpenFileError`] for more information about these variants.
pub fn open_file<'a>(
    root_dir: &mut Directory,
    path: &'a CStr16,
) -> Result<RegularFile, OpenFileError<'a>> {
    // TODO: Question: Can a Cstr16 not have a null terminator? If so, I should add code to ensure that it is.
    const FILE_NAME_MAX_LEN: usize = 255;
    debug!("Path: {}", path);

    let path = path.as_slice_with_nul();

    // Ensure path is not an empty string
    // TODO: Change this to a returned error if I ever open file names from user input
    assert!(path.len() > 1);

    // State variables for parsing
    let mut current_dir_owned: Directory; // Subdirectory handles (not root dir) need to be owned
    let mut current_dir = root_dir;
    let mut start_idx: usize = 0;
    let mut next_separator: Option<usize> = find_next_separator(path, start_idx);
    // TODO: Move Char16 null character to a constant
    let mut dirname_buf: [Char16; FILE_NAME_MAX_LEN + 1] =
        [Char16::try_from(0).unwrap(); FILE_NAME_MAX_LEN + 1];

    // Open each directory in the path until the file is reached
    while let Some(end_idx) = next_separator {
        // Continue to next separator if directory name is empty (2 path separators in a row '//')
        if start_idx == end_idx {
            start_idx += 1;
            next_separator = find_next_separator(path, start_idx);
            continue;
        }

        // Ensure directory name is not too long
        let name_len = end_idx - start_idx;
        if name_len > FILE_NAME_MAX_LEN {
            return Err(OpenFileError::FileNameTooLong(&path[..end_idx]));
        }

        // Copy name of next directory into buffer
        let name = &path[start_idx..end_idx];
        dirname_buf[..name_len].copy_from_slice(name);
        dirname_buf[name_len] = Char16::try_from(0).unwrap();
        let name = unsafe {
            CStr16::from_u16_with_nul_unchecked(
                &*(&dirname_buf[..name_len + 1] as *const [Char16] as *const [u16]),
            )
        };

        // Open next directory in path
        let handle = open_handle(current_dir, name, &path[..end_idx])?;

        match handle {
            FileType::Regular(_) => {
                // Ensure file is not a directory
                return Err(OpenFileError::NotADirectory(&path[..end_idx]));
            }
            FileType::Dir(subdir) => {
                // Set subdirectory as current
                current_dir_owned = subdir;
                current_dir = &mut current_dir_owned;
                // Update start index to be after the previous path separator '/'
                start_idx = end_idx + 1;
                // Find the next path separator
                next_separator = find_next_separator(path, start_idx);
            }
        }
    }

    // Ensure file name is not empty
    if path[start_idx] == Char16::try_from(0).unwrap() {
        return Err(OpenFileError::EmptyFileName);
    }

    // Ensure file name is not too long
    if path.len() - start_idx > FILE_NAME_MAX_LEN + 1 {
        return Err(OpenFileError::FileNameTooLong(path));
    }

    // Get filename
    let name = unsafe {
        CStr16::from_u16_with_nul_unchecked(
            &*(&path[start_idx..] as *const [Char16] as *const [u16]),
        )
    };

    // Return file handle
    let handle = open_handle(current_dir, name, path)?;

    match handle {
        FileType::Regular(file_handle) => Ok(file_handle),
        FileType::Dir(_) => Err(OpenFileError::NotAFile),
    }
}

/// Returns the index of the first "/" separator in the `path`. Returns `None` if there
/// are no more separators.
///
/// The search starts at `start_idx`.
fn find_next_separator(path: &[Char16], start_idx: usize) -> Option<usize> {
    if start_idx >= path.len() {
        return None;
    }

    let separator = Char16::try_from('/').unwrap();

    for (i, c) in path[start_idx..].iter().enumerate() {
        if *c == separator {
            return Some(start_idx + i);
        }
    }

    return None;
}

fn open_handle<'a>(
    dir: &mut Directory,
    name: &CStr16,
    path: &'a [Char16],
) -> Result<FileType, OpenFileError<'a>> {
    let handle = dir
        .open(name, FileMode::Read, FileAttribute::READ_ONLY)
        .map_err(|uefi_err| OpenFileError::UefiFailure(uefi_err, path))?;

    handle
        .into_type()
        .map_err(|uefi_err| OpenFileError::UefiFailure(uefi_err, path))
}

fn allocate_str(data: &[Char16]) -> CString16 {
    let buf_len = data.len();
    let mut buf: Vec<u16> = Vec::with_capacity(buf_len + 1);
    buf.extend_from_slice(unsafe { &*(data as *const [Char16] as *const [u16]) });
    buf.push(0);
    CString16::try_from(buf).unwrap()
}
