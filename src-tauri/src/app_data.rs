//! Application-data directory boundary. The OS-provided parent is trusted.

use std::{io, path::Path};

#[cfg(unix)]
pub(crate) fn prepare(path: &Path) -> io::Result<()> {
    use cap_fs_ext::DirExt;
    use cap_std::fs::{Dir, DirBuilder, DirBuilderExt};
    use std::os::unix::fs::PermissionsExt;

    let parent = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "App data must have a parent directory.",
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "App data must have a directory name.",
        )
    })?;
    std::fs::create_dir_all(parent)?;
    let parent = Dir::open_ambient_dir(parent, cap_std::ambient_authority())?;
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    match parent.create_dir_with(name, &builder) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    // Refuse a leaf symlink and apply permissions through its opened handle.
    // The trusted parent and same-user processes remain outside this boundary.
    let directory = parent.open_dir_nofollow(name)?.into_std_file();
    #[cfg(target_os = "macos")]
    reject_extended_acl(&directory)?;
    directory.set_permissions(std::fs::Permissions::from_mode(0o700))?;
    let mode = directory.metadata()?.permissions().mode();
    if mode & 0o777 != 0o700 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot make app data private to this account.",
        ));
    }
    Ok(())
}

// Darwin extended ACLs can grant access independently of POSIX mode bits.
// Reject custom policies without silently changing the user's ACL. Read the ACL
// through the same descriptor used for chmod, never through a replacement path.
#[cfg(target_os = "macos")]
fn reject_extended_acl(directory: &std::fs::File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    // sys/acl.h: acl_type_t is a C enum and ACL_TYPE_EXTENDED is 0x100.
    unsafe extern "C" {
        fn acl_get_fd_np(fd: libc::c_int, kind: libc::c_int) -> *mut libc::c_void;
        fn acl_free(acl: *mut libc::c_void) -> libc::c_int;
    }
    // SAFETY: the borrowed file keeps this descriptor live. The opaque returned
    // ACL is never dereferenced and is freed exactly once with its paired API.
    let acl = unsafe { acl_get_fd_np(directory.as_raw_fd(), 0x100) };
    if acl.is_null() {
        let error = io::Error::last_os_error();
        return if error.raw_os_error() == Some(libc::ENOENT) {
            Ok(())
        } else {
            Err(error)
        };
    }
    // SAFETY: acl is the non-null allocation returned by acl_get_fd_np above.
    unsafe { acl_free(acl) };
    Err(io::Error::new(io::ErrorKind::PermissionDenied,
        "App data has an extended access policy. Ask your administrator to review its ACL before opening Cachella."))
}

#[cfg(not(unix))]
pub(crate) fn prepare(path: &Path) -> io::Result<()> {
    // Retain the OS per-user directory ACL. Windows ACL verification is a
    // separate qualification requirement; Unix mode bits do not model it.
    std::fs::create_dir_all(path)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::{symlink, PermissionsExt},
    };

    #[test]
    fn new_app_data_is_private() {
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("new-parent").join("app");
        prepare(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn existing_app_data_is_hardened_without_changing_contents() {
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("app");
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(path.join("workspace.sqlite"), b"existing data").unwrap();
        prepare(&path).unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::read(path.join("workspace.sqlite")).unwrap(),
            b"existing data"
        );
        prepare(&path).unwrap();
    }

    #[test]
    fn symlink_app_data_is_rejected_without_modifying_target() {
        let parent = tempfile::tempdir().unwrap();
        let outside = parent.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o755)).unwrap();
        let path = parent.path().join("app");
        symlink(&outside, &path).unwrap();
        assert!(prepare(&path).is_err());
        assert_eq!(
            fs::metadata(&outside).unwrap().permissions().mode() & 0o777,
            0o755
        );
        assert!(fs::symlink_metadata(path).unwrap().file_type().is_symlink());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extended_acl_is_rejected_without_removing_user_policy() {
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("app");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("marker"), b"preserve").unwrap();
        let status = std::process::Command::new("/bin/chmod")
            .args([
                "+a",
                "everyone allow list,search,readattr,readextattr,readsecurity",
            ])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        let result = prepare(&path);
        // Always clear our fixture ACL even on the expected red-test failure.
        assert!(std::process::Command::new("/bin/chmod")
            .arg("-N")
            .arg(&path)
            .status()
            .unwrap()
            .success());
        assert!(result.is_err(), "extended ACL must block startup");
        assert_eq!(fs::read(path.join("marker")).unwrap(), b"preserve");
    }

    #[test]
    fn regular_file_at_app_data_path_is_rejected() {
        let parent = tempfile::tempdir().unwrap();
        let path = parent.path().join("app");
        fs::write(&path, b"preserve").unwrap();
        assert!(prepare(&path).is_err());
        assert_eq!(fs::read(path).unwrap(), b"preserve");
    }
}
