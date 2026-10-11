//! Descriptor-first access to user-selected regular files.

use std::{fs::File, io, path::Path};

/// Opens a path, then validates the opened descriptor is a regular file.
///
/// On Unix, `O_NONBLOCK` prevents opening a FIFO from waiting for a writer.
/// The caller may still intentionally select a symlink; validation applies to
/// its resolved target and happens on the descriptor, avoiding a check/open
/// replacement window.
pub(crate) fn open_regular_file(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)?
    };
    #[cfg(not(unix))]
    let file = {
        // Keep the existing pre-open rejection for Windows special paths. The
        // descriptor check below still validates the object actually opened.
        if !std::fs::metadata(path)?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "selected path is not a regular file",
            ));
        }
        File::open(path)?
    };

    if file.metadata()?.is_file() {
        Ok(file)
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "selected path is not a regular file",
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn opens_regular_files() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), b"regular").unwrap();
        assert_eq!(
            super::open_regular_file(file.path())
                .unwrap()
                .metadata()
                .unwrap()
                .len(),
            7
        );
    }

    #[cfg(unix)]
    #[test]
    fn follows_selected_file_symlinks() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        let link = root.path().join("selected.gguf");
        std::fs::write(&target, b"regular").unwrap();
        symlink(&target, &link).unwrap();
        assert_eq!(
            super::open_regular_file(&link)
                .unwrap()
                .metadata()
                .unwrap()
                .len(),
            7
        );
    }

    #[cfg(unix)]
    #[test]
    fn fifo_is_rejected_without_blocking() {
        use std::{
            ffi::CString,
            process::Command,
            thread,
            time::{Duration, Instant},
        };

        const CHILD: &str = "CACHELLA_REGULAR_FILE_FIFO_CHILD";
        const PATH: &str = "CACHELLA_REGULAR_FILE_FIFO_PATH";
        if let (Some(_), Some(path)) = (std::env::var_os(CHILD), std::env::var_os(PATH)) {
            assert!(super::open_regular_file(std::path::Path::new(&path)).is_err());
            return;
        }

        let root = tempfile::tempdir().unwrap();
        let fifo = root.path().join("blocked.gguf");
        let path = CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "regular_file::tests::fifo_is_rejected_without_blocking",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env(PATH, &fifo)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                break child.wait().unwrap();
            }
            thread::sleep(Duration::from_millis(10));
        };
        assert!(
            status.success(),
            "special-file validation blocked or failed: {status}"
        );
    }
}
