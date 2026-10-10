//! Handle-anchored Windows directory operations for the scratch namespace.
//!
//! `cap_std::fs::Dir::remove_open_dir_all` ultimately uses a path on Windows.
//! This module keeps every scratch descendant operation relative to an already
//! opened directory handle and marks objects delete-pending by handle.  The
//! caller supplies the only ambient path when it establishes the trusted
//! system temporary-directory boundary.

use std::{
    ffi::{OsStr, OsString},
    fs, io,
    mem::{self, size_of},
    os::windows::{
        ffi::{OsStrExt, OsStringExt},
        fs::{MetadataExt, OpenOptionsExt},
        io::{AsRawHandle, FromRawHandle, RawHandle},
    },
    path::{Component, Path},
};

use windows_sys::{
    Wdk::{
        Foundation::OBJECT_ATTRIBUTES,
        Storage::FileSystem::{
            FileDispositionInformation, FileNamesInformation, NtCreateFile, NtQueryDirectoryFile,
            NtSetInformationFile, FILE_CREATE, FILE_DIRECTORY_FILE, FILE_DISPOSITION_INFORMATION,
            FILE_OPEN, FILE_OPEN_IF, FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
        },
    },
    Win32::{
        Foundation::{
            RtlNtStatusToDosError, HANDLE, STATUS_NO_MORE_FILES, STATUS_SUCCESS, UNICODE_STRING,
        },
        Storage::FileSystem::{
            DELETE, FILE_ADD_SUBDIRECTORY, FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL,
            FILE_ATTRIBUTE_REPARSE_POINT, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES,
            FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FILE_TRAVERSE, SYNCHRONIZE,
        },
        System::IO::IO_STATUS_BLOCK,
    },
};

use super::scratch_windows_records::DirectoryBudget;

const MAX_DEPTH: usize = 64;
const MAX_ENTRIES: usize = 16_384;
const MAX_NAME_BYTES: usize = 4 * 1024 * 1024;
const READ_BUFFER_BYTES: usize = 64 * 1024;
const CLEANUP_ACCESS: u32 = FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | DELETE | SYNCHRONIZE;
const DIRECTORY_ACCESS: u32 = FILE_LIST_DIRECTORY
    | FILE_ADD_SUBDIRECTORY
    | FILE_TRAVERSE
    | FILE_READ_ATTRIBUTES
    | DELETE
    | SYNCHRONIZE;
const AMBIENT_DIRECTORY_ACCESS: u32 = FILE_LIST_DIRECTORY
    | FILE_ADD_SUBDIRECTORY
    | FILE_TRAVERSE
    | FILE_READ_ATTRIBUTES
    | SYNCHRONIZE;
const ALL_SHARE: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
const LOCK_SHARE: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE;

/// A directory opened with all share modes. Owned scratch descendants retain
/// DELETE access, while the ambient system-temp parent deliberately does not.
pub(crate) struct Dir {
    file: fs::File,
}

pub(crate) struct Entry {
    name: OsString,
}

impl Entry {
    pub(crate) fn file_name(&self) -> OsString {
        self.name.clone()
    }
}

impl Dir {
    /// Establishes the one ambient boundary. Subsequent calls stay relative to
    /// this handle. The caller explicitly trusts the OS-selected temporary
    /// directory, including its final reparse resolution. All owned children
    /// below this boundary are opened as reparse points and then rejected.
    pub(crate) fn open_ambient_dir(path: &Path, _: cap_std::AmbientAuthority) -> io::Result<Self> {
        let mut options = fs::OpenOptions::new();
        options
            .read(true)
            .access_mode(AMBIENT_DIRECTORY_ACCESS)
            .share_mode(ALL_SHARE)
            .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS);
        let file = options.open(path)?;
        Self::from_directory_file(file)
    }

    pub(crate) fn create_dir(&self, name: &str) -> io::Result<()> {
        let file = open_relative(
            &self.file,
            name,
            DIRECTORY_ACCESS,
            ALL_SHARE,
            FILE_CREATE,
            FILE_DIRECTORY_FILE | FILE_SYNCHRONOUS_IO_NONALERT,
            FILE_ATTRIBUTE_DIRECTORY,
        )?;
        Self::from_directory_file(file).map(drop)
    }

    pub(crate) fn open_dir_nofollow(&self, name: &str) -> io::Result<Self> {
        let file = open_relative(
            &self.file,
            name,
            DIRECTORY_ACCESS,
            ALL_SHARE,
            FILE_OPEN,
            FILE_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            FILE_ATTRIBUTE_NORMAL,
        )?;
        Self::from_directory_file(file)
    }

    pub(crate) fn entries(&self) -> io::Result<std::vec::IntoIter<io::Result<Entry>>> {
        self.entries_with_budget(&mut DirectoryBudget::new(MAX_ENTRIES, MAX_NAME_BYTES))
    }

    fn entries_with_budget(
        &self,
        budget: &mut DirectoryBudget,
    ) -> io::Result<std::vec::IntoIter<io::Result<Entry>>> {
        let mut result = Vec::new();
        let mut restart = true;
        loop {
            // NtQueryDirectoryFile writes C records into this buffer. `u64`
            // storage supplies more than the four-byte alignment the record layouts
            // require; the byte view has exactly the same initialized range.
            let mut words = vec![0_u64; READ_BUFFER_BYTES.div_ceil(size_of::<u64>())];
            let buffer = unsafe {
                std::slice::from_raw_parts_mut(
                    words.as_mut_ptr().cast::<u8>(),
                    words.len() * size_of::<u64>(),
                )
            };
            let mut status = unsafe { mem::zeroed::<IO_STATUS_BLOCK>() };
            // The handle is synchronous and `buffer` plus `status` remain
            // valid until the kernel returns. No APC or event is supplied.
            let code = unsafe {
                NtQueryDirectoryFile(
                    raw_handle(&self.file),
                    std::ptr::null_mut(),
                    None,
                    std::ptr::null(),
                    &mut status,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    FileNamesInformation,
                    false,
                    std::ptr::null(),
                    restart,
                )
            };
            restart = false;
            if code == STATUS_NO_MORE_FILES {
                break;
            }
            if code != STATUS_SUCCESS {
                return Err(nt_error(code));
            }
            let bytes = status.Information;
            if bytes == 0 || bytes > buffer.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid directory result",
                ));
            }
            for units in budget.parse_page(&buffer[..bytes])? {
                let name = OsString::from_wide(&units);
                if name != OsStr::new(".") && name != OsStr::new("..") {
                    result.push(Entry { name });
                }
            }
        }
        Ok(result.into_iter().map(Ok).collect::<Vec<_>>().into_iter())
    }

    /// Inspects a direct child without following a reparse point.
    pub(crate) fn symlink_metadata(&self, name: &str) -> io::Result<fs::Metadata> {
        let file = open_relative(
            &self.file,
            name,
            FILE_READ_ATTRIBUTES | SYNCHRONIZE,
            ALL_SHARE,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            FILE_ATTRIBUTE_NORMAL,
        )?;
        file.metadata()
    }

    /// Lock files intentionally omit `FILE_SHARE_DELETE`. Recovery drops the
    /// lock before it removes an abandoned session, preserving lock identity.
    pub(crate) fn open_lock(&self, name: &str, create: bool) -> io::Result<fs::File> {
        let disposition = if create { FILE_OPEN_IF } else { FILE_OPEN };
        let file = open_relative(
            &self.file,
            name,
            windows_sys::Win32::Foundation::GENERIC_READ
                | windows_sys::Win32::Foundation::GENERIC_WRITE
                | SYNCHRONIZE,
            LOCK_SHARE,
            disposition,
            FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            FILE_ATTRIBUTE_NORMAL,
        )?;
        if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "scratch lock cannot be a reparse point",
            ));
        }
        Ok(file)
    }

    pub(crate) fn remove_open_dir_all(self) -> io::Result<()> {
        remove_tree(
            self,
            0,
            &mut DirectoryBudget::new(MAX_ENTRIES, MAX_NAME_BYTES),
        )
    }

    fn from_directory_file(file: fs::File) -> io::Result<Self> {
        let metadata = file.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "scratch directory must not be a reparse point",
            ));
        }
        Ok(Self { file })
    }
}

fn remove_tree(directory: Dir, depth: usize, budget: &mut DirectoryBudget) -> io::Result<()> {
    if depth >= MAX_DEPTH {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "scratch directory depth limit exceeded",
        ));
    }
    let entries = directory.entries_with_budget(budget)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let opened = open_relative_os(
            &directory.file,
            &name,
            CLEANUP_ACCESS,
            ALL_SHARE,
            FILE_OPEN,
            FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT,
            FILE_ATTRIBUTE_NORMAL,
        )?;
        let metadata = opened.metadata()?;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0 && metadata.is_dir() {
            remove_tree(Dir::from_directory_file(opened)?, depth + 1, budget)?;
        } else {
            mark_delete(opened)?;
        }
    }
    mark_delete(directory.file)
}

fn mark_delete(file: fs::File) -> io::Result<()> {
    let mut status = unsafe { mem::zeroed::<IO_STATUS_BLOCK>() };
    let disposition = FILE_DISPOSITION_INFORMATION { DeleteFile: true };
    // `file` owns DELETE access. The information buffer is exactly the C ABI
    // structure, and both pointers are live for the synchronous call.
    let code = unsafe {
        NtSetInformationFile(
            raw_handle(&file),
            &mut status,
            (&disposition as *const FILE_DISPOSITION_INFORMATION).cast(),
            size_of::<FILE_DISPOSITION_INFORMATION>() as u32,
            FileDispositionInformation,
        )
    };
    if code == STATUS_SUCCESS {
        drop(file);
        Ok(())
    } else {
        Err(nt_error(code))
    }
}

fn open_relative(
    parent: &fs::File,
    name: &str,
    access: u32,
    share: u32,
    disposition: u32,
    options: u32,
    attributes: u32,
) -> io::Result<fs::File> {
    open_relative_os(
        parent,
        OsStr::new(name),
        access,
        share,
        disposition,
        options,
        attributes,
    )
}

fn open_relative_os(
    parent: &fs::File,
    name: &OsStr,
    access: u32,
    share: u32,
    disposition: u32,
    options: u32,
    attributes: u32,
) -> io::Result<fs::File> {
    validate_component(name)?;
    let wide: Vec<u16> = name.encode_wide().collect();
    if wide.len() > u16::MAX as usize / 2 || wide.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid scratch path component",
        ));
    }
    let mut unicode = UNICODE_STRING {
        Length: (wide.len() * 2) as u16,
        MaximumLength: (wide.len() * 2) as u16,
        Buffer: wide.as_ptr() as *mut u16,
    };
    let mut object = unsafe { mem::zeroed::<OBJECT_ATTRIBUTES>() };
    object.Length = size_of::<OBJECT_ATTRIBUTES>() as u32;
    object.RootDirectory = raw_handle(parent);
    object.ObjectName = &mut unicode;
    let mut status = unsafe { mem::zeroed::<IO_STATUS_BLOCK>() };
    let mut handle: HANDLE = std::ptr::null_mut();
    // The name contains one validated component and is interpreted relative to
    // `RootDirectory`; no full child path is ever passed to Windows.
    let code = unsafe {
        NtCreateFile(
            &mut handle,
            access,
            &object,
            &mut status,
            std::ptr::null_mut(),
            attributes,
            share,
            disposition,
            options,
            std::ptr::null_mut(),
            0,
        )
    };
    if code != STATUS_SUCCESS {
        return Err(nt_error(code));
    }
    // NtCreateFile returned a valid owned kernel handle on success.
    Ok(unsafe { fs::File::from_raw_handle(handle as RawHandle) })
}

fn validate_component(name: &OsStr) -> io::Result<()> {
    let path = Path::new(name);
    let has_windows_separator_or_stream = name
        .encode_wide()
        .any(|unit| [b'/' as u16, b'\\' as u16, b':' as u16].contains(&unit));
    match (path.components().next(), path.components().nth(1)) {
        (Some(Component::Normal(_)), None)
            if !name.is_empty() && !has_windows_separator_or_stream =>
        {
            Ok(())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scratch path must be one component",
        )),
    }
}

fn raw_handle(file: &fs::File) -> HANDLE {
    file.as_raw_handle() as HANDLE
}

fn nt_error(status: i32) -> io::Error {
    io::Error::from_raw_os_error(unsafe { RtlNtStatusToDosError(status) } as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_component_child_names() {
        let temp = tempfile::tempdir().unwrap();
        let root = Dir::open_ambient_dir(temp.path(), cap_std::ambient_authority()).unwrap();
        for name in ["", ".", "..", "a/b", r"a\b"] {
            assert!(root.create_dir(name).is_err(), "{name:?}");
        }
        assert!(root.create_dir("payload:alternate-stream").is_err());
    }

    #[test]
    fn delete_original_directory_after_rename_preserves_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let root = Dir::open_ambient_dir(temp.path(), cap_std::ambient_authority()).unwrap();
        root.create_dir("original").unwrap();
        let original = root.open_dir_nofollow("original").unwrap();
        fs::create_dir(temp.path().join("original").join("nested")).unwrap();
        fs::write(
            temp.path().join("original").join("nested").join("payload"),
            b"x",
        )
        .unwrap();
        fs::rename(temp.path().join("original"), temp.path().join("moved")).unwrap();
        fs::create_dir(temp.path().join("original")).unwrap();
        let replacement = temp.path().join("original").join("marker");
        fs::write(&replacement, b"replacement").unwrap();
        original.remove_open_dir_all().unwrap();
        assert!(replacement.exists());
        assert!(!temp.path().join("moved").exists());
    }

    #[test]
    fn delete_original_directory_after_ancestor_replacement_preserves_replacement() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("parent")).unwrap();
        let parent =
            Dir::open_ambient_dir(&temp.path().join("parent"), cap_std::ambient_authority())
                .unwrap();
        parent.create_dir("owned").unwrap();
        fs::write(
            temp.path().join("parent").join("owned").join("payload"),
            b"x",
        )
        .unwrap();
        fs::rename(temp.path().join("parent"), temp.path().join("moved-parent")).unwrap();
        fs::create_dir(temp.path().join("parent")).unwrap();
        fs::create_dir(temp.path().join("parent").join("owned")).unwrap();
        let marker = temp.path().join("parent").join("owned").join("marker");
        fs::write(&marker, b"replacement").unwrap();
        // Windows forbids renaming an ancestor while a descendant handle is
        // open. Keep the parent anchor across replacement, then resolve its
        // original child through that retained handle.
        let owned = parent.open_dir_nofollow("owned").unwrap();
        owned.remove_open_dir_all().unwrap();
        assert!(marker.exists());
        assert!(!temp.path().join("moved-parent").join("owned").exists());
    }

    #[test]
    fn held_file_stops_cleanup_until_the_file_is_closed() {
        let temp = tempfile::tempdir().unwrap();
        let root = Dir::open_ambient_dir(temp.path(), cap_std::ambient_authority()).unwrap();
        root.create_dir("owned").unwrap();
        fs::write(temp.path().join("owned").join("held"), b"x").unwrap();
        let mut options = fs::OpenOptions::new();
        options
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE);
        let held = options
            .open(temp.path().join("owned").join("held"))
            .unwrap();
        assert!(root
            .open_dir_nofollow("owned")
            .unwrap()
            .remove_open_dir_all()
            .is_err());
        assert!(temp.path().join("owned").exists());
        drop(held);
        root.open_dir_nofollow("owned")
            .unwrap()
            .remove_open_dir_all()
            .unwrap();
        assert!(!temp.path().join("owned").exists());
    }

    #[test]
    #[ignore = "creating directory symlinks requires Windows Developer Mode or SeCreateSymbolicLinkPrivilege"]
    fn deleting_a_reparse_child_never_traverses_its_target() {
        use std::os::windows::fs::symlink_dir;

        let temp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let marker = outside.path().join("marker");
        fs::write(&marker, b"outside").unwrap();
        let root = Dir::open_ambient_dir(temp.path(), cap_std::ambient_authority()).unwrap();
        root.create_dir("owned").unwrap();
        symlink_dir(outside.path(), temp.path().join("owned").join("link")).unwrap();
        root.open_dir_nofollow("owned")
            .unwrap()
            .remove_open_dir_all()
            .unwrap();
        assert!(marker.exists());
    }
}
