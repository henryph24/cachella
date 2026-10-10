//! Atomic settings publication with leased, recoverable staging.
//!
//! The destination parent is trusted application data. Staging lives on the
//! same filesystem, and the committed file is replaced only after its new
//! contents are written and synced. This covers process interruption; it does
//! not claim power-loss durability for the parent directory entry.

use crate::scratch::Workspace;
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

type Result<T> = std::result::Result<T, String>;

pub fn recover(directory: &Path) -> Result<()> {
    crate::scratch::recover_publications_in(directory)
}

pub fn save_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| "Could not encode settings.".to_string())?;
    Publication::prepare(path, &bytes)?.publish()
}

struct Publication {
    // Close/remove the staged file before dropping its workspace on Windows.
    file: tempfile::NamedTempFile,
    _workspace: Workspace,
    destination: PathBuf,
}

impl Publication {
    fn prepare(path: &Path, bytes: &[u8]) -> Result<Self> {
        let directory = path.parent().ok_or("Settings directory is unavailable.")?;
        let workspace = Workspace::publication_in(directory)?;
        let mut file = tempfile::NamedTempFile::new_in(workspace.path())
            .map_err(|_| "Could not create settings staging file.".to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.as_file().sync_all())
            .map_err(|_| "Could not write settings staging file.".to_string())?;
        Ok(Self {
            file,
            _workspace: workspace,
            destination: path.to_owned(),
        })
    }

    fn publish(self) -> Result<()> {
        self.file
            .persist(&self.destination)
            .map_err(|_| "Could not publish settings.".to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn readers_keep_committed_settings_until_publication() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("runtime.json");
        fs::write(&path, br#"{"version":1}"#).unwrap();
        let staged = Publication::prepare(&path, br#"{"version":2}"#).unwrap();
        let workspace = staged.file.path().parent().unwrap().to_owned();
        recover(root.path()).unwrap();
        assert!(
            staged.file.path().exists(),
            "active staging must survive recovery"
        );
        assert_eq!(fs::read(&path).unwrap(), br#"{"version":1}"#);
        staged.publish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), br#"{"version":2}"#);
        assert!(!workspace.exists(), "normal publication must clear staging");
    }

    #[test]
    fn failed_publication_preserves_destination_and_removes_staging() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("runtime.json");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("marker"), b"preserve").unwrap();
        let staged = Publication::prepare(&path, b"replacement").unwrap();
        let workspace = staged.file.path().parent().unwrap().to_owned();
        assert!(staged.publish().is_err());
        assert_eq!(fs::read(path.join("marker")).unwrap(), b"preserve");
        assert!(!workspace.exists());
    }

    #[test]
    fn json_round_trips_and_legacy_temporary_files_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("endpoint.json");
        let legacy = root.path().join(".tmp-unattributed");
        fs::write(&legacy, b"unrelated").unwrap();
        save_json(&path, &"http://127.0.0.1:8080").unwrap();
        recover(root.path()).unwrap();
        let saved: String = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(saved, "http://127.0.0.1:8080");
        assert_eq!(fs::read(legacy).unwrap(), b"unrelated");
    }

    #[test]
    fn overlapping_writers_keep_their_staging_and_publish_complete_values() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("runtime.json");
        let first = Publication::prepare(&path, br#"{"version":1}"#).unwrap();
        let second = Publication::prepare(&path, br#"{"version":2}"#).unwrap();
        recover(root.path()).unwrap();
        assert!(first.file.path().exists() && second.file.path().exists());
        first.publish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), br#"{"version":1}"#);
        recover(root.path()).unwrap();
        assert!(second.file.path().exists());
        second.publish().unwrap();
        assert_eq!(fs::read(&path).unwrap(), br#"{"version":2}"#);
    }

    #[test]
    fn encoding_failure_preserves_committed_settings() {
        struct InvalidValue;
        impl Serialize for InvalidValue {
            fn serialize<S: serde::Serializer>(
                &self,
                _: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("fixture encoding failure"))
            }
        }
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("runtime.json");
        fs::write(&path, br#"{"version":1}"#).unwrap();
        assert!(save_json(&path, &InvalidValue).is_err());
        assert_eq!(fs::read(&path).unwrap(), br#"{"version":1}"#);
    }

    struct TestChild(std::process::Child);
    impl Drop for TestChild {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn interrupted_writer_keeps_committed_settings_and_releases_staging_lease() {
        const CHILD_ROOT: &str = "CACHELLA_SETTINGS_RECOVERY_CHILD_ROOT";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let root = Path::new(&root);
            let staged = Publication::prepare(&root.join("runtime.json"), b"uncommitted").unwrap();
            fs::write(
                root.join("ready.tmp"),
                serde_json::to_vec(staged.file.path()).unwrap(),
            )
            .unwrap();
            fs::rename(root.join("ready.tmp"), root.join("ready.json")).unwrap();
            thread::sleep(Duration::from_secs(20));
            panic!("parent did not stop staged writer");
        }
        let root = tempfile::tempdir().unwrap();
        let committed = root.path().join("runtime.json");
        fs::write(&committed, br#"{"version":1}"#).unwrap();
        let mut child = TestChild(Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "settings::tests::interrupted_writer_keeps_committed_settings_and_releases_staging_lease", "--nocapture"])
            .env(CHILD_ROOT, root.path()).spawn().unwrap());
        let marker = root.path().join("ready.json");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.exists() && Instant::now() < deadline {
            if child.0.try_wait().unwrap().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(5));
        }
        if !marker.exists() {
            let _ = child.0.kill();
            let _ = child.0.wait();
            panic!("staged writer did not become ready");
        }
        let stage: std::path::PathBuf =
            serde_json::from_slice(&fs::read(&marker).unwrap()).unwrap();
        let live_recovery = recover(root.path());
        let live_stage_exists = stage.exists();
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        live_recovery.unwrap();
        assert!(live_stage_exists);
        recover(root.path()).unwrap();
        assert!(!stage.exists());
        assert_eq!(fs::read(committed).unwrap(), br#"{"version":1}"#);
    }
}
