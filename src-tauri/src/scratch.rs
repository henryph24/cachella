//! Crash-recoverable private workspaces for files passed to local helpers.
//!
//! The supplied temporary/application-data parent is the ambient trust boundary. Everything
//! below `cachella-scratch-v1` is opened one component at a time without
//! following links. Cleanup is therefore anchored to retained directory
//! handles. Windows uses native relative opens and delete-by-handle operations;
//! Unix uses cap-std's retained-directory cleanup.

#[cfg(not(windows))]
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
#[cfg(not(windows))]
use cap_std::fs::{Dir, OpenOptions};
#[cfg(any(windows, test))]
#[path = "scratch_windows_records.rs"]
mod scratch_windows_records;
#[cfg(windows)]
#[path = "scratch_windows.rs"]
mod windows;
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    thread,
    time::{Duration, Instant},
};
#[cfg(windows)]
use windows::Dir;

const NAMESPACE: &str = "cachella-scratch-v1";
const COORDINATOR: &str = ".coordinator";
const LEASE: &str = ".lease";
const SESSION_PREFIX: &str = "session-";
const WORKSPACE_PREFIX: &str = "workspace-";
const COORDINATOR_WAIT: Duration = Duration::from_secs(5);

/// A serializable reference that lets the process guardian keep this session
/// live while it owns a child. It contains no secret: the OS shared lock is
/// the authority and must be acquired before the guardian reports READY.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LeaseDescriptor {
    root: PathBuf,
    session: String,
}

/// A held shared OS lock for a guardian lifetime.
pub struct HeldLease {
    _file: std::fs::File,
}

impl Drop for HeldLease {
    fn drop(&mut self) {
        let _ = self._file.unlock();
    }
}

/// An individual private directory. Its contents are deleted when dropped.
pub struct Workspace {
    _session: Arc<Session>,
    directory: Option<Dir>,
    path: PathBuf,
}

impl Workspace {
    pub fn new() -> Result<Self, String> {
        session()?.workspace()
    }

    /// Same-filesystem staging for atomic settings publication. These files
    /// are never passed to subprocesses; only the writer retains the lease.
    pub(crate) fn publication_in(parent: &Path) -> Result<Self, String> {
        Arc::new(Session::new_in(parent)?).workspace()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        // `remove_open_dir_all` consumes the original descriptor. A
        // replacement at `self.path` or at the session-relative name cannot
        // redirect cleanup into its contents.
        if let Some(directory) = self.directory.take() {
            let _ = directory.remove_open_dir_all();
        }
    }
}

/// Initializes this process's scratch session. Desktop startup calls this so
/// orphan recovery happens before media, PDF, or TLS files are made.
pub fn initialize() -> Result<(), String> {
    let _ = session()?;
    Ok(())
}

/// Recovers settings staging in its trusted application-data parent. Active
/// writers retain their leases and are skipped under the shared coordinator.
pub(crate) fn recover_publications_in(parent: &Path) -> Result<(), String> {
    let root = open_or_create_root(parent)?;
    let coordinator = open_lock(&root, COORDINATOR, true)?;
    lock_exclusive(&coordinator)?;
    let result = recover_orphans(&root);
    let _ = coordinator.unlock();
    result
}

/// Returns the current process lease without creating a session.
pub fn current_lease() -> Option<LeaseDescriptor> {
    SESSION
        .get()
        .and_then(|value| value.as_ref().ok())
        .map(|session| session.descriptor())
}

/// Retains a session's shared lease for a guardian child. This never creates
/// directories; a stale descriptor simply fails. The returned value must stay
/// alive until the child has been reaped.
pub fn retain_lease(descriptor: &LeaseDescriptor) -> Result<HeldLease, String> {
    if !valid_component(&descriptor.session, SESSION_PREFIX) {
        return Err("Invalid scratch lease descriptor.".into());
    }
    let root = open_existing_root(&descriptor.root)?;
    let coordinator = open_lock(&root, COORDINATOR, false)?;
    lock_exclusive(&coordinator)?;
    let directory = root
        .open_dir_nofollow(&descriptor.session)
        .map_err(|_| "Scratch lease session is unavailable.".to_string())?;
    let file = open_lock(&directory, LEASE, false)?;
    file.try_lock_shared()
        .map_err(|_| "Scratch lease session is busy or unavailable.".to_string())?;
    let _ = coordinator.unlock();
    Ok(HeldLease { _file: file })
}

static SESSION: OnceLock<Result<Arc<Session>, String>> = OnceLock::new();

fn session() -> Result<Arc<Session>, String> {
    SESSION
        .get_or_init(|| Session::new_in(&std::env::temp_dir()).map(Arc::new))
        .clone()
}

struct Session {
    root_path: PathBuf,
    name: String,
    directory: Dir,
    _lease: std::fs::File,
}

impl Drop for Session {
    fn drop(&mut self) {
        // Release this process's lease explicitly. A guardian holds a
        // separate descriptor, and can therefore keep the session alive
        // after the desktop parent has exited.
        let _ = self._lease.unlock();
    }
}

impl Session {
    /// Kept private outside this module; tests use it with a dedicated parent.
    fn new_in(parent: &Path) -> Result<Self, String> {
        let root_path = parent.join(NAMESPACE);
        let root = open_or_create_root(parent)?;
        let coordinator = open_lock(&root, COORDINATOR, true)?;
        lock_exclusive(&coordinator)?;
        recover_orphans(&root)?;

        for _ in 0..8 {
            let name = format!("{SESSION_PREFIX}{}", uuid::Uuid::new_v4().simple());
            match root.create_dir(&name) {
                Ok(()) => {
                    let directory = root
                        .open_dir_nofollow(&name)
                        .map_err(|_| "Cannot open scratch session.".to_string())?;
                    set_private_dir(&directory)?;
                    let lease = open_lock(&directory, LEASE, true)?;
                    if lease.try_lock_shared().is_err() {
                        drop(lease);
                        cleanup_failed_allocation(directory)?;
                        return Err("Cannot retain scratch session lease.".into());
                    }
                    let _ = coordinator.unlock();
                    return Ok(Self {
                        root_path,
                        name,
                        directory,
                        _lease: lease,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err("Cannot create scratch session.".into()),
            }
        }
        Err("Cannot allocate scratch session.".into())
    }

    fn descriptor(&self) -> LeaseDescriptor {
        LeaseDescriptor {
            root: self.root_path.clone(),
            session: self.name.clone(),
        }
    }

    fn workspace(self: &Arc<Self>) -> Result<Workspace, String> {
        for _ in 0..8 {
            let name = format!("{WORKSPACE_PREFIX}{}", uuid::Uuid::new_v4().simple());
            match self.directory.create_dir(&name) {
                Ok(()) => {
                    let workspace = self
                        .directory
                        .open_dir_nofollow(&name)
                        .map_err(|_| "Cannot open scratch workspace.".to_string())?;
                    set_private_dir(&workspace)?;
                    return Ok(Workspace {
                        _session: Arc::clone(self),
                        directory: Some(workspace),
                        path: self.root_path.join(&self.name).join(&name),
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err("Cannot create scratch workspace.".into()),
            }
        }
        Err("Cannot allocate scratch workspace.".into())
    }
}

fn cleanup_failed_allocation(directory: Dir) -> Result<(), String> {
    directory
        .remove_open_dir_all()
        .map_err(|_| "Cannot clean up failed scratch session allocation.".to_string())
}

fn open_or_create_root(parent: &Path) -> Result<Dir, String> {
    let parent = Dir::open_ambient_dir(parent, cap_std::ambient_authority())
        .map_err(|_| "Cannot open system temporary directory.".to_string())?;
    match parent.create_dir(NAMESPACE) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err("Cannot create scratch directory.".into()),
    }
    let root = parent
        .open_dir_nofollow(NAMESPACE)
        .map_err(|_| "Scratch directory must be a directory.".to_string())?;
    set_private_dir(&root)?;
    Ok(root)
}

fn open_existing_root(root: &Path) -> Result<Dir, String> {
    let parent = root
        .parent()
        .ok_or_else(|| "Invalid scratch lease descriptor.".to_string())?;
    if root.file_name().and_then(|name| name.to_str()) != Some(NAMESPACE) {
        return Err("Invalid scratch lease descriptor.".into());
    }
    Dir::open_ambient_dir(parent, cap_std::ambient_authority())
        .map_err(|_| "Scratch lease root is unavailable.".to_string())?
        .open_dir_nofollow(NAMESPACE)
        .map_err(|_| "Scratch lease root is unavailable.".to_string())
}

fn open_lock(directory: &Dir, name: &str, create: bool) -> Result<std::fs::File, String> {
    #[cfg(windows)]
    let file = directory
        .open_lock(name, create)
        .map_err(|_| "Cannot open scratch lease.".to_string())?;
    #[cfg(not(windows))]
    let file = {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(create)
            .follow(FollowSymlinks::No);
        directory
            .open_with(name, &options)
            .map_err(|_| "Cannot open scratch lease.".to_string())?
            .into_std()
    };
    if !file
        .metadata()
        .map_err(|_| "Cannot inspect scratch lease.".to_string())?
        .is_file()
    {
        return Err("Scratch lease must be a regular file.".into());
    }
    set_private_file(&file)?;
    Ok(file)
}

fn lock_exclusive(file: &std::fs::File) -> Result<(), String> {
    lock_exclusive_for(file, COORDINATOR_WAIT)
}

fn lock_exclusive_for(file: &std::fs::File, wait: Duration) -> Result<(), String> {
    let deadline = Instant::now() + wait;
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                if Instant::now() >= deadline {
                    return Err("Timed out coordinating scratch sessions.".into());
                }
                thread::sleep(Duration::from_millis(5));
            }
            Err(_) => return Err("Cannot coordinate scratch sessions.".into()),
        }
    }
}

fn recover_orphans(root: &Dir) -> Result<(), String> {
    for entry in root
        .entries()
        .map_err(|_| "Cannot scan scratch sessions.".to_string())?
    {
        let entry = entry.map_err(|_| "Cannot inspect scratch session.".to_string())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !valid_component(name, SESSION_PREFIX) {
            continue;
        }
        let Ok(directory) = root.open_dir_nofollow(name) else {
            // A symlink or replacement is intentionally preserved.
            continue;
        };
        recover_session(directory)?;
    }
    Ok(())
}

fn recover_session(directory: Dir) -> Result<(), String> {
    match directory.symlink_metadata(LEASE) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => return Err("Scratch session lease must be a regular file.".into()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if directory
                .entries()
                .map_err(|_| "Cannot inspect incomplete scratch session.".to_string())?
                .next()
                .is_some()
            {
                return Err("Incomplete scratch session contains unexpected files.".into());
            }
            return directory
                .remove_open_dir_all()
                .map_err(|_| "Cannot remove incomplete scratch session.".to_string());
        }
        Err(_) => return Err("Cannot inspect scratch session lease.".into()),
    }
    let lease = open_lock(&directory, LEASE, false)?;
    match lease.try_lock() {
        Ok(()) => {
            // The root coordinator prevents a guardian attach between this
            // exclusive check and deletion. Dropping the lock first lets
            // Windows remove the file, while the coordinator retains the
            // same serialization boundary.
            drop(lease);
            directory
                .remove_open_dir_all()
                .map_err(|_| "Cannot remove recovered scratch session.".to_string())
        }
        Err(std::fs::TryLockError::WouldBlock) => Ok(()),
        Err(_) => Err("Cannot lock scratch session lease.".into()),
    }
}

fn valid_component(name: &str, prefix: &str) -> bool {
    let Some(id) = name.strip_prefix(prefix) else {
        return false;
    };
    id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(unix)]
fn set_private_dir(directory: &Dir) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    directory
        .set_permissions(
            ".",
            cap_std::fs::Permissions::from_std(std::fs::Permissions::from_mode(0o700)),
        )
        .map_err(|_| "Cannot secure scratch directory.".to_string())
}

#[cfg(not(unix))]
fn set_private_dir(_: &Dir) -> Result<(), String> {
    // Windows inherits the per-user temporary-directory ACL. It is not
    // validated here because Rust's portable permission API cannot inspect it.
    Ok(())
}

#[cfg(unix)]
fn set_private_file(file: &std::fs::File) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .map_err(|_| "Cannot secure scratch lease.".to_string())
}

#[cfg(not(unix))]
fn set_private_file(_: &std::fs::File) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn session(parent: &tempfile::TempDir) -> Session {
        Session::new_in(parent.path()).unwrap()
    }

    #[test]
    fn recovers_an_orphaned_owned_session() {
        let parent = tempfile::tempdir().unwrap();
        let first = session(&parent);
        let old = first.root_path.join(&first.name);
        drop(first);
        let _next = session(&parent);
        assert!(!old.exists());
    }

    #[test]
    fn recovery_removes_an_empty_session_interrupted_before_lease_creation() {
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join(NAMESPACE);
        fs::create_dir(&root).unwrap();
        let incomplete = root.join(format!("{SESSION_PREFIX}{}", "b".repeat(32)));
        fs::create_dir(&incomplete).unwrap();
        let _next = session(&parent);
        assert!(!incomplete.exists());
    }

    #[test]
    fn recovery_deletes_the_original_after_a_real_directory_replacement() {
        let parent = tempfile::tempdir().unwrap();
        let first = session(&parent);
        let root_path = first.root_path.clone();
        let session_path = root_path.join(&first.name);
        let name = first.name.clone();
        drop(first);
        let root = Dir::open_ambient_dir(&root_path, cap_std::ambient_authority()).unwrap();
        let original = root.open_dir_nofollow(&name).unwrap();
        let moved = session_path.with_extension("moved");
        fs::rename(&session_path, &moved).unwrap();
        fs::create_dir(&session_path).unwrap();
        let marker = session_path.join("marker");
        fs::write(&marker, b"replacement").unwrap();
        recover_session(original).unwrap();
        assert!(marker.exists());
        assert!(!moved.exists());
    }

    #[test]
    fn coordinator_wait_is_bounded() {
        let parent = tempfile::tempdir().unwrap();
        let root = open_or_create_root(parent.path()).unwrap();
        let held = open_lock(&root, COORDINATOR, true).unwrap();
        held.try_lock().unwrap();
        let contender = open_lock(&root, COORDINATOR, false).unwrap();
        let started = Instant::now();
        assert!(lock_exclusive_for(&contender, Duration::from_millis(10)).is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn active_parent_and_guardian_keep_session() {
        let parent = tempfile::tempdir().unwrap();
        let first = Arc::new(session(&parent));
        let descriptor = first.descriptor();
        let guardian = retain_lease(&descriptor).unwrap();
        let old = first.root_path.join(&first.name);
        drop(first);
        let _next = session(&parent);
        assert!(old.is_dir());
        drop(guardian);
        let _last = session(&parent);
        assert!(!old.exists());
    }

    #[test]
    fn workspace_drop_removes_its_directory() {
        let parent = tempfile::tempdir().unwrap();
        let session = Arc::new(session(&parent));
        let workspace = session.workspace().unwrap();
        let path = workspace.path().to_owned();
        fs::write(path.join("payload"), b"test").unwrap();
        drop(workspace);
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn recovery_preserves_unrelated_and_symlink_entries() {
        use std::os::unix::fs::symlink;
        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join(NAMESPACE);
        fs::create_dir(&root).unwrap();
        let unrelated = root.join("keep-me");
        fs::write(&unrelated, b"keep").unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = root.join(format!("{SESSION_PREFIX}{}", "a".repeat(32)));
        symlink(outside.path(), &link).unwrap();
        let _session = session(&parent);
        assert!(unrelated.exists());
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    }

    #[cfg(unix)]
    #[test]
    fn workspace_drop_does_not_follow_a_renamed_path() {
        use std::os::unix::fs::symlink;
        let parent = tempfile::tempdir().unwrap();
        let session = Arc::new(session(&parent));
        let workspace = session.workspace().unwrap();
        let path = workspace.path().to_owned();
        let moved = path.with_extension("moved");
        fs::rename(&path, &moved).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let marker = outside.path().join("marker");
        fs::write(&marker, b"outside").unwrap();
        symlink(outside.path(), &path).unwrap();
        drop(workspace);
        assert!(marker.exists());
        assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
        assert!(!moved.exists());
    }

    #[test]
    fn workspace_drop_preserves_a_real_directory_replacement() {
        let parent = tempfile::tempdir().unwrap();
        let session = Arc::new(session(&parent));
        let workspace = session.workspace().unwrap();
        let path = workspace.path().to_owned();
        let moved = path.with_extension("moved");
        fs::rename(&path, &moved).unwrap();
        fs::create_dir(&path).unwrap();
        let marker = path.join("marker");
        fs::write(&marker, b"replacement").unwrap();
        drop(workspace);
        assert!(marker.exists());
        assert!(!moved.exists());
    }

    #[test]
    fn failed_allocation_cleanup_preserves_a_real_directory_replacement() {
        let parent = tempfile::tempdir().unwrap();
        let root = open_or_create_root(parent.path()).unwrap();
        let name = format!("{SESSION_PREFIX}{}", "c".repeat(32));
        root.create_dir(&name).unwrap();
        let original = root.open_dir_nofollow(&name).unwrap();
        let session_path = parent.path().join(NAMESPACE).join(&name);
        let moved = session_path.with_extension("moved");
        fs::rename(&session_path, &moved).unwrap();
        fs::create_dir(&session_path).unwrap();
        let marker = session_path.join("marker");
        fs::write(&marker, b"replacement").unwrap();
        cleanup_failed_allocation(original).unwrap();
        assert!(marker.exists());
        assert!(!moved.exists());
    }
}
