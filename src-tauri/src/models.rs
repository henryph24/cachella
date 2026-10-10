//! Pinned, verified GGUF installation into an application-owned directory.
use crate::store::Result;
use cap_fs_ext::{DirExt, FollowSymlinks, MetadataExt, OpenOptionsFollowExt};
use cap_std::fs::{Dir, OpenOptions};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const CANCELLED: &str = "Model installation cancelled.";
static INSTALL_BUSY: AtomicBool = AtomicBool::new(false);
const QWEN_REVISION: &str = "d7f544eead698dbd1f15126ef60b45a1e1933222";
const GEMMA_REVISION: &str = "bfcd298762cc34d0357ece5ebdd31791a3a374d8";
const GEMMA4_E2B_REVISION: &str = "b4243c156154b6dca9324415f8c7ccc098b4aed1";
const MODEL_REVISIONS: [(&str, &str); 3] = [
    ("qwen3-1.7b-q4", QWEN_REVISION),
    ("embeddinggemma2-q8", GEMMA_REVISION),
    ("gemma4-e2b-q4", GEMMA4_E2B_REVISION),
];

#[derive(Debug, Default, Serialize)]
pub struct RecoveryReport {
    pub removed_files: u64,
    pub removed_bytes: u64,
    pub busy_directories: u64,
}

/// The lock file stays at a stable path for the lifetime of the installation.
/// Removing it would let a second process lock a different inode.
struct RevisionLease {
    _lock: std::fs::File,
    directory: Dir,
}

impl RevisionLease {
    #[cfg(test)]
    fn acquire(directory: &Path) -> Result<Option<Self>> {
        Self::acquire_in(open_root(directory, false)?.ok_or("Missing model directory.")?)
    }

    fn acquire_in(directory: Dir) -> Result<Option<Self>> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(true)
            .follow(FollowSymlinks::No);
        let file = directory
            .open_with(".installation.lock", &options)
            .map_err(|_| "Cannot open model installation lock.".to_string())?
            .into_std();
        if !file
            .metadata()
            .map_err(|_| "Cannot inspect model installation lock.")?
            .is_file()
        {
            return Err("Model installation lock must be a regular file.".into());
        }
        match file.try_lock() {
            Ok(()) => Ok(Some(Self {
                _lock: file,
                directory,
            })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(_) => Err("Cannot lock model installation directory.".into()),
        }
    }

    fn recover(&self) -> Result<RecoveryReport> {
        let mut report = RecoveryReport::default();
        for entry in self
            .directory
            .entries()
            .map_err(|_| "Cannot scan interrupted model downloads.".to_string())?
        {
            let entry =
                entry.map_err(|_| "Cannot inspect interrupted model download.".to_string())?;
            let name = entry.file_name();
            let Some(random) = name
                .to_str()
                .and_then(|n| n.strip_prefix(".download-"))
                .and_then(|n| n.strip_suffix(".part"))
            else {
                continue;
            };
            let legacy = random.len() == 6 && random.bytes().all(|c| c.is_ascii_alphanumeric());
            let current = random.len() == 32 && random.bytes().all(|c| c.is_ascii_hexdigit());
            if !legacy && !current {
                continue;
            }
            // All metadata and deletion stay anchored to the locked directory,
            // including when its former pathname is replaced with a symlink.
            let metadata = self
                .directory
                .symlink_metadata(&name)
                .map_err(|_| "Cannot inspect interrupted model download.".to_string())?;
            if !metadata.file_type().is_file() {
                continue;
            }
            self.directory
                .remove_file(&name)
                .map_err(|_| "Cannot remove interrupted model download.".to_string())?;
            report.removed_files += 1;
            report.removed_bytes += metadata.len();
        }
        Ok(report)
    }
}

impl Drop for RevisionLease {
    fn drop(&mut self) {
        // Explicit unlock also releases the lease when another thread briefly
        // inherited the descriptor during fork before its close-on-exec step.
        let _ = self._lock.unlock();
    }
}

// The application-data parent is the ambient trust boundary. Open each owned
// descendant as a single no-follow component and retain the resulting handle.
fn open_root(path: &Path, create: bool) -> Result<Option<Dir>> {
    let absolute = std::path::absolute(path).map_err(|_| "Invalid model directory.")?;
    let parent = absolute.parent().ok_or("Invalid model directory.")?;
    let name = absolute.file_name().ok_or("Invalid model directory.")?;
    let parent = match Dir::open_ambient_dir(parent, cap_std::ambient_authority()) {
        Ok(parent) => parent,
        Err(error) if !create && error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Cannot open model storage parent directory.".into()),
    };
    open_child(&parent, name, create)
}

fn open_child(parent: &Dir, name: impl AsRef<Path>, create: bool) -> Result<Option<Dir>> {
    let name = name.as_ref();
    if create {
        match parent.create_dir(name) {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err("Cannot create model installation directory.".into()),
        }
    }
    match parent.open_dir_nofollow(name) {
        Ok(dir) => Ok(Some(dir)),
        Err(error) if !create && error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Model installation directory must be an owned directory.".into()),
    }
}

fn ensure_same_directory(owned: &Dir, current: &Dir) -> Result<()> {
    let owned = owned
        .dir_metadata()
        .map_err(|_| "Cannot inspect model directory.")?;
    let current = current
        .dir_metadata()
        .map_err(|_| "Cannot inspect model directory.")?;
    if (owned.dev(), owned.ino()) != (current.dev(), current.ino()) {
        return Err("Model installation directory changed. Retry the installation.".into());
    }
    Ok(())
}

struct DownloadPartial<'a> {
    directory: &'a Dir,
    name: String,
}

impl<'a> DownloadPartial<'a> {
    fn create(directory: &'a Dir) -> Result<(Self, std::fs::File)> {
        let name = format!(".download-{}.part", uuid::Uuid::new_v4().simple());
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let file = directory
            .open_with(&name, &options)
            .map_err(|_| "Cannot create model download file.".to_string())?
            .into_std();
        Ok((Self { directory, name }, file))
    }
}

impl Drop for DownloadPartial<'_> {
    fn drop(&mut self) {
        let _ = self.directory.remove_file(&self.name);
    }
}

#[derive(Clone, Copy)]
struct Asset {
    name: &'static str,
    url: &'static str,
    bytes: u64,
    sha: &'static str,
}

const QWEN: Asset = Asset {
    name: "Qwen3-1.7B-Q4_K_M.gguf",
    url: "https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/resolve/d7f544eead698dbd1f15126ef60b45a1e1933222/Qwen3-1.7B-Q4_K_M.gguf",
    bytes: 1_107_409_472,
    sha: "b139949c5bd74937ad8ed8c8cf3d9ffb1e99c866c823204dc42c0d91fa181897",
};
const GEMMA: Asset = Asset {
    name: "embeddinggemma-2-Q8_0.gguf",
    url: "https://huggingface.co/ggml-org/embeddinggemma-2-GGUF/resolve/bfcd298762cc34d0357ece5ebdd31791a3a374d8/embeddinggemma-2-Q8_0.gguf",
    bytes: 309_855_456,
    sha: "2188ac1deca4b77dffefd603c2776a9d76d9d74ec01841392982ebb840b09135",
};
const PROJECTOR: Asset = Asset {
    name: "mmproj-embeddinggemma-2-Q8_0.gguf",
    url: "https://huggingface.co/ggml-org/embeddinggemma-2-GGUF/resolve/bfcd298762cc34d0357ece5ebdd31791a3a374d8/mmproj-embeddinggemma-2-Q8_0.gguf",
    bytes: 554_821_024,
    sha: "c4a8a52691ecef40618438928bdf9e68379b854e24166f292592353db0aab64f",
};
const GEMMA4_E2B: Asset = Asset {
    name: "gemma-4-E2B-it-Q4_0.gguf",
    url: "https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/resolve/b4243c156154b6dca9324415f8c7ccc098b4aed1/gemma-4-E2B-it-Q4_0.gguf",
    bytes: 2_841_481_184,
    sha: "8e30dff3ac4c8434c49a7036fa15564bdbb6044e42bf04550bf1a096ad7e6a52",
};
const GEMMA4_E2B_PROJECTOR: Asset = Asset {
    name: "mmproj-gemma-4-E2B-it-Q8_0.gguf",
    url: "https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/resolve/b4243c156154b6dca9324415f8c7ccc098b4aed1/mmproj-gemma-4-E2B-it-Q8_0.gguf",
    bytes: 557_368_064,
    sha: "9406f99c16d68cda4f1f0552192dcc99021ea1fc6d2fd50b1dc3ccf30d04b292",
};

#[derive(Clone, Serialize)]
pub struct ModelBundle {
    pub id: String,
    pub title: String,
    pub description: String,
    pub license: String,
    pub publisher: String,
    pub source_url: String,
    pub total_bytes: u64,
    pub embedding: bool,
}

#[derive(Clone, Serialize, Debug)]
pub struct InstallProgress {
    pub bundle_id: Option<String>,
    pub phase: String,
    pub downloaded_bytes: u64,
    pub total_bytes: u64,
    pub current_file: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Serialize, Debug)]
pub struct InstalledModels {
    pub model: PathBuf,
    pub projector: Option<PathBuf>,
    pub embedding: bool,
}

pub fn catalog() -> Vec<ModelBundle> {
    vec![
        ModelBundle {
            id: "qwen3-1.7b-q4".into(),
            title: "Qwen3 1.7B".into(),
            description: "Q4_K_M GGUF for local answers. 1.11 GB download.".into(),
            license: "Apache-2.0".into(),
            publisher: "Unsloth (third-party GGUF conversion)".into(),
            source_url: "https://huggingface.co/unsloth/Qwen3-1.7B-GGUF/tree/d7f544eead698dbd1f15126ef60b45a1e1933222".into(),
            total_bytes: QWEN.bytes,
            embedding: false,
        },
        ModelBundle {
            id: "gemma4-e2b-q4".into(),
            title: "Gemma 4 E2B".into(),
            description: "Q4_0 GGUF and matching vision/audio projector for local media answers. 3.40 GB download.".into(),
            license: "Apache-2.0".into(),
            publisher: "ggml-org (third-party GGUF conversion)".into(),
            source_url: "https://huggingface.co/ggml-org/gemma-4-E2B-it-GGUF/tree/b4243c156154b6dca9324415f8c7ccc098b4aed1".into(),
            total_bytes: GEMMA4_E2B.bytes + GEMMA4_E2B_PROJECTOR.bytes,
            embedding: false,
        },
        ModelBundle {
            id: "embeddinggemma2-q8".into(),
            title: "EmbeddingGemma 2".into(),
            description: "Q8_0 GGUF and projector for multimodal search. 865 MB download.".into(),
            license: "Apache-2.0".into(),
            publisher: "ggml-org (third-party GGUF conversion)".into(),
            source_url: "https://huggingface.co/ggml-org/embeddinggemma-2-GGUF/tree/bfcd298762cc34d0357ece5ebdd31791a3a374d8".into(),
            total_bytes: GEMMA.bytes + PROJECTOR.bytes,
            embedding: true,
        },
    ]
}

fn trusted_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.fragment().is_none()
        && url.host_str().is_some_and(|host| {
            host == "huggingface.co"
                || host.ends_with(".hf.co")
                || host.ends_with(".huggingface.co")
        })
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .https_only(true)
        .no_proxy()
        .referer(false)
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(2 * 60 * 60))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 5 || !trusted_url(attempt.url().as_str()) {
                attempt.error("Untrusted model download redirect.")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|_| "Cannot initialize the model download connection.".into())
}

struct State {
    progress: Mutex<InstallProgress>,
    cancelled: AtomicBool,
    active: AtomicBool,
    closing: AtomicBool,
    idle_signal: tokio::sync::Notify,
    cancel_signal: tokio::sync::Notify,
}

impl State {
    async fn wait_cancelled(&self) {
        loop {
            if self.cancelled.load(Ordering::Acquire) {
                return;
            }
            self.cancel_signal.notified().await;
        }
    }
}

#[derive(Clone)]
pub struct ModelInstaller {
    root: PathBuf,
    state: Arc<State>,
}

impl ModelInstaller {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            state: Arc::new(State {
                progress: Mutex::new(InstallProgress {
                    bundle_id: None,
                    phase: "idle".into(),
                    downloaded_bytes: 0,
                    total_bytes: 0,
                    current_file: None,
                    error: None,
                }),
                cancelled: AtomicBool::new(false),
                active: AtomicBool::new(false),
                closing: AtomicBool::new(false),
                idle_signal: tokio::sync::Notify::new(),
                cancel_signal: tokio::sync::Notify::new(),
            }),
        }
    }

    /// Reclaim only catalog-owned partials whose revision has no active installer.
    /// Run before exposing desktop commands. A busy revision is left untouched.
    pub fn recover_interrupted(&self) -> Result<RecoveryReport> {
        let mut report = RecoveryReport::default();
        let Some(root) = open_root(&self.root, false)? else {
            return Ok(report);
        };
        for (bundle, revision) in MODEL_REVISIONS {
            let Some(bundle_dir) = open_child(&root, bundle, false)? else {
                continue;
            };
            let Some(directory) = open_child(&bundle_dir, revision, false)? else {
                continue;
            };
            let Some(lease) = RevisionLease::acquire_in(directory)? else {
                report.busy_directories += 1;
                continue;
            };
            let recovered = lease.recover()?;
            report.removed_files += recovered.removed_files;
            report.removed_bytes += recovered.removed_bytes;
        }
        Ok(report)
    }

    pub fn status(&self) -> Result<InstallProgress> {
        self.state
            .progress
            .lock()
            .map(|p| p.clone())
            .map_err(|_| "Cannot read model installation progress.".into())
    }

    pub fn cancel(&self) -> Result<()> {
        let _progress = self
            .state
            .progress
            .lock()
            .map_err(|_| "Cannot cancel model installation.".to_string())?;
        if self.state.active.load(Ordering::Acquire) {
            self.state.cancelled.store(true, Ordering::Release);
            self.state.cancel_signal.notify_one();
        }
        Ok(())
    }

    /// Permanently close this installer and cancel any active install.
    /// The caller must await `wait_idle` before terminating the process.
    pub fn begin_shutdown(&self) -> Result<bool> {
        let _progress = self
            .state
            .progress
            .lock()
            .map_err(|_| "Cannot begin model installer shutdown.".to_string())?;
        self.state.closing.store(true, Ordering::Release);
        let active = self.state.active.load(Ordering::Acquire);
        if active {
            self.state.cancelled.store(true, Ordering::Release);
            self.state.cancel_signal.notify_one();
        }
        Ok(active)
    }

    /// Wait until the active install future has released its owned partial files.
    pub async fn wait_idle(&self) {
        loop {
            let idle = self.state.idle_signal.notified();
            tokio::pin!(idle);
            // Register before observing active to avoid missing a completion wakeup.
            idle.as_mut().enable();
            if !self.state.active.load(Ordering::Acquire) {
                return;
            }
            idle.await;
        }
    }

    pub async fn install(&self, bundle_id: &str) -> Result<InstalledModels> {
        let bundle = catalog()
            .into_iter()
            .find(|b| b.id == bundle_id)
            .ok_or_else(|| "Unknown model bundle.".to_string())?;
        let (revision, assets): (&str, &[Asset]) = match bundle_id {
            "qwen3-1.7b-q4" => (QWEN_REVISION, &[QWEN]),
            "gemma4-e2b-q4" => (GEMMA4_E2B_REVISION, &[GEMMA4_E2B, GEMMA4_E2B_PROJECTOR]),
            "embeddinggemma2-q8" => (GEMMA_REVISION, &[GEMMA, PROJECTOR]),
            _ => return Err("Unknown model bundle.".into()),
        };
        self.install_assets(&bundle, revision, assets, &Transport::Live)
            .await
    }

    // All assets and path components passed here are compile-time catalog values.
    async fn install_assets(
        &self,
        bundle: &ModelBundle,
        revision: &str,
        assets: &[Asset],
        transport: &Transport,
    ) -> Result<InstalledModels> {
        let mut guard = BusyGuard::acquire(self.state.clone(), bundle)?;
        let result = self.run_install(bundle, revision, assets, transport).await;
        let mut progress = self
            .state
            .progress
            .lock()
            .map_err(|_| "Cannot update model installation progress.".to_string())?;
        match &result {
            Ok(_) => {
                progress.phase = "ready".into();
                progress.current_file = None;
            }
            Err(error) => {
                progress.phase = if error == CANCELLED {
                    "cancelled"
                } else {
                    "error"
                }
                .into();
                progress.error = if error == CANCELLED {
                    None
                } else {
                    Some(error.clone())
                };
            }
        }
        guard.finished = true;
        drop(progress);
        result
    }

    fn progress(&self, phase: &str, downloaded: u64, current_file: &str) -> Result<()> {
        let mut progress = self
            .state
            .progress
            .lock()
            .map_err(|_| "Cannot update model installation progress.".to_string())?;
        progress.phase = phase.into();
        progress.downloaded_bytes = downloaded;
        progress.current_file = Some(current_file.into());
        Ok(())
    }

    async fn run_install(
        &self,
        bundle: &ModelBundle,
        revision: &str,
        assets: &[Asset],
        transport: &Transport,
    ) -> Result<InstalledModels> {
        let directory = self.root.join(&bundle.id).join(revision);
        let root = open_root(&self.root, true)?.ok_or("Missing model directory.")?;
        let bundle_dir = open_child(&root, &bundle.id, true)?.ok_or("Missing model bundle.")?;
        let revision_dir =
            open_child(&bundle_dir, revision, true)?.ok_or("Missing model revision.")?;
        let lease = RevisionLease::acquire_in(revision_dir)?.ok_or(
            "Another Cachella instance is installing this model. Retry after it finishes.",
        )?;
        lease.recover()?;
        let client = match transport {
            Transport::Live => client()?,
            #[cfg(test)]
            Transport::Fixture(client) => client.clone(),
        };
        let mut paths = Vec::new();
        let mut completed = 0;
        for asset in assets {
            check_cancelled(&self.state.cancelled)?;
            let destination = directory.join(asset.name);
            match lease.directory.symlink_metadata(asset.name) {
                Ok(_) => {
                    self.progress("verifying", completed, asset.name)?;
                    verify_at(
                        &lease.directory,
                        asset.name,
                        asset.bytes,
                        asset.sha,
                        &self.state.cancelled,
                    )
                    .await?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    self.download(&client, transport, &lease.directory, asset, completed)
                        .await?;
                }
                Err(_) => return Err("Cannot inspect the installed model file.".into()),
            }
            completed += asset.bytes;
            self.progress("verifying", completed, asset.name)?;
            paths.push(destination);
        }
        check_cancelled(&self.state.cancelled)?;
        let current_root = open_root(&self.root, false)?.ok_or("Model directory moved.")?;
        let current_bundle =
            open_child(&current_root, &bundle.id, false)?.ok_or("Model directory moved.")?;
        let current_revision =
            open_child(&current_bundle, revision, false)?.ok_or("Model directory moved.")?;
        ensure_same_directory(&lease.directory, &current_revision)?;
        Ok(InstalledModels {
            model: paths[0].clone(),
            projector: paths.get(1).cloned(),
            embedding: bundle.embedding,
        })
    }

    async fn download(
        &self,
        client: &reqwest::Client,
        transport: &Transport,
        directory: &Dir,
        asset: &Asset,
        completed: u64,
    ) -> Result<()> {
        if matches!(transport, Transport::Live) && !trusted_url(asset.url) {
            return Err("Untrusted model download URL.".into());
        }
        self.progress("downloading", completed, asset.name)?;
        let request = client
            .get(asset.url)
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .send();
        let mut response = tokio::select! {
            _ = self.state.wait_cancelled() => return Err(CANCELLED.into()),
            response = request => response.map_err(|_| "Model download connection failed. Retry to restart the download.".to_string())?,
        };
        check_cancelled(&self.state.cancelled)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(
                "Model download server rejected the request. Retry to restart the download.".into(),
            );
        }
        if response
            .content_length()
            .is_some_and(|length| length != asset.bytes)
        {
            return Err("Model download has an unexpected size.".into());
        }
        // Drop cleanup uses the same anchored directory even after a rename.
        let (partial, file) = DownloadPartial::create(directory)?;
        let mut file = tokio::fs::File::from_std(file);
        let mut downloaded: u64 = 0;
        loop {
            let chunk = tokio::select! {
                _ = self.state.wait_cancelled() => return Err(CANCELLED.into()),
                chunk = response.chunk() => chunk.map_err(|_| "Model download interrupted. Retry to restart the download.".to_string())?,
            };
            check_cancelled(&self.state.cancelled)?;
            let Some(chunk) = chunk else {
                break;
            };
            downloaded = downloaded
                .checked_add(chunk.len() as u64)
                .filter(|total| *total <= asset.bytes)
                .ok_or_else(|| "Model download exceeds the expected size.".to_string())?;
            file.write_all(&chunk).await.map_err(|_| {
                "Cannot save the model download. Check available disk space.".to_string()
            })?;
            self.progress("downloading", completed + downloaded, asset.name)?;
        }
        if downloaded != asset.bytes {
            return Err("Model download is incomplete. Retry to restart the download.".into());
        }
        file.flush()
            .await
            .map_err(|_| "Cannot flush the model download.".to_string())?;
        file.sync_all()
            .await
            .map_err(|_| "Cannot sync the model download.".to_string())?;
        drop(file);
        self.progress("verifying", completed + downloaded, asset.name)?;
        verify_at(
            directory,
            &partial.name,
            asset.bytes,
            asset.sha,
            &self.state.cancelled,
        )
        .await?;
        check_cancelled(&self.state.cancelled)?;
        // Hard-link publication is atomic and refuses an existing destination.
        match directory.hard_link(&partial.name, directory, asset.name) {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                verify_at(
                    directory,
                    asset.name,
                    asset.bytes,
                    asset.sha,
                    &self.state.cancelled,
                )
                .await
            }
            Err(_) => Err("Cannot publish the verified model file.".into()),
        }
    }
}

#[derive(Clone)]
enum Transport {
    Live,
    #[cfg(test)]
    Fixture(reqwest::Client),
}

struct BusyGuard {
    state: Arc<State>,
    finished: bool,
}

impl BusyGuard {
    fn acquire(state: Arc<State>, bundle: &ModelBundle) -> Result<Self> {
        let next_progress = InstallProgress {
            bundle_id: Some(bundle.id.clone()),
            phase: "downloading".into(),
            downloaded_bytes: 0,
            total_bytes: bundle.total_bytes,
            current_file: None,
            error: None,
        };
        // Shutdown and job startup share the mutex: once closing is set, a job
        // cannot become active even if its async command was queued previously.
        let mut progress = state
            .progress
            .lock()
            .map_err(|_| "Cannot initialize model installation progress.".to_string())?;
        if state.closing.load(Ordering::Acquire) {
            return Err("Model installer is shutting down.".into());
        }
        INSTALL_BUSY
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "A model installation is already running.".to_string())?;
        state.cancelled.store(false, Ordering::Release);
        state.active.store(true, Ordering::Release);
        *progress = next_progress;
        drop(progress);
        // No fallible work follows acquiring the global gate before its guard exists.
        Ok(Self {
            state,
            finished: false,
        })
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        let mut progress = self.state.progress.lock();
        if let Ok(progress) = progress.as_mut() {
            if !self.finished {
                progress.phase = "cancelled".into();
                progress.error = None;
            }
        }
        // run_install has returned (or its later-created future was dropped),
        // so its NamedTempFile has been cleaned before idle becomes observable.
        // Keep the mutex through both stores so a new job cannot race this cleanup.
        INSTALL_BUSY.store(false, Ordering::Release);
        self.state.active.store(false, Ordering::Release);
        drop(progress);
        self.state.idle_signal.notify_waiters();
    }
}

fn check_cancelled(cancelled: &AtomicBool) -> Result<()> {
    if cancelled.load(Ordering::Acquire) {
        Err(CANCELLED.into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
async fn verify_file(path: &Path, bytes: u64, sha: &str, cancelled: &AtomicBool) -> Result<()> {
    let parent = Dir::open_ambient_dir(
        path.parent().ok_or("Invalid model file path.")?,
        cap_std::ambient_authority(),
    )
    .map_err(|_| "Cannot open model directory.")?;
    verify_at(
        &parent,
        path.file_name().ok_or("Invalid model file path.")?,
        bytes,
        sha,
        cancelled,
    )
    .await
}

async fn verify_at(
    directory: &Dir,
    name: impl AsRef<Path>,
    bytes: u64,
    sha: &str,
    cancelled: &AtomicBool,
) -> Result<()> {
    check_cancelled(cancelled)?;
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = directory
        .open_with(name, &options)
        .map_err(|_| "Cannot open the model file for verification.".to_string())?
        .into_std();
    let metadata = file
        .metadata()
        .map_err(|_| "Cannot inspect model file.".to_string())?;
    if !metadata.is_file() || metadata.len() != bytes {
        return Err("Model file has an unexpected size or type.".into());
    }
    let mut file = tokio::fs::File::from_std(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 256 * 1024];
    let mut header = Vec::with_capacity(4);
    let mut total = 0_u64;
    loop {
        check_cancelled(cancelled)?;
        let count = file
            .read(&mut buffer)
            .await
            .map_err(|_| "Cannot read the model file for verification.".to_string())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > bytes {
            return Err("Model file exceeds the expected size.".into());
        }
        let needed = 4 - header.len();
        header.extend_from_slice(&buffer[..count.min(needed)]);
        hasher.update(&buffer[..count]);
    }
    check_cancelled(cancelled)?;
    if total != bytes || header != b"GGUF" {
        return Err("Model file is incomplete or has an invalid GGUF header.".into());
    }
    if format!("{:x}", hasher.finalize()) != sha {
        return Err(
            "Model file checksum verification failed. Retry to restart the download.".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::AtomicBool;

    fn recovery_directory(root: &Path) -> PathBuf {
        let directory = root.join(MODEL_REVISIONS[0].0).join(MODEL_REVISIONS[0].1);
        std::fs::create_dir_all(&directory).unwrap();
        directory
    }

    #[test]
    fn recovery_preserves_active_owner_and_reclaims_after_release() {
        let root = tempfile::tempdir().unwrap();
        let directory = recovery_directory(root.path());
        let lease = RevisionLease::acquire(&directory).unwrap().unwrap();
        let partial = directory.join(".download-Ab12Cd.part");
        std::fs::write(&partial, b"partial").unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let busy = installer.recover_interrupted().unwrap();
        assert_eq!(busy.busy_directories, 1);
        assert_eq!(busy.removed_files, 0);
        assert!(partial.exists());
        assert!(RevisionLease::acquire(&directory).unwrap().is_none());
        drop(lease);
        let recovered = installer.recover_interrupted().unwrap();
        assert_eq!(recovered.removed_files, 1);
        assert_eq!(recovered.removed_bytes, 7);
        assert!(!partial.exists());
        assert_eq!(installer.recover_interrupted().unwrap().removed_files, 0);
    }

    #[test]
    fn recovery_only_removes_regular_partials_in_known_revisions() {
        let root = tempfile::tempdir().unwrap();
        let directory = recovery_directory(root.path());
        for name in [
            "model.gguf",
            "notes.part",
            ".download-too-long.part",
            ".download-!!!!!!.part",
        ] {
            std::fs::write(directory.join(name), b"keep").unwrap();
        }
        std::fs::create_dir(directory.join(".download-Dir123.part")).unwrap();
        let unknown = root.path().join("unrecognized");
        std::fs::create_dir(&unknown).unwrap();
        std::fs::write(unknown.join(".download-Ab12Cd.part"), b"keep").unwrap();
        std::fs::write(directory.join(".download-Ab12Cd.part"), b"remove").unwrap();
        std::fs::write(
            directory.join(".download-0123456789abcdef0123456789abcdef.part"),
            b"remove",
        )
        .unwrap();
        let result = ModelInstaller::new(root.path().into())
            .recover_interrupted()
            .unwrap();
        assert_eq!(result.removed_files, 2);
        assert_eq!(result.removed_bytes, 12);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 6); // 5 retained paths + lock
        assert!(unknown.join(".download-Ab12Cd.part").exists());
        let missing = root.path().join("missing");
        assert_eq!(
            ModelInstaller::new(missing.clone())
                .recover_interrupted()
                .unwrap()
                .removed_files,
            0
        );
        assert!(!missing.exists());
    }

    #[cfg(unix)]
    #[test]
    fn recovery_stays_in_locked_directory_after_path_replacement() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let directory = recovery_directory(root.path());
        let lease = RevisionLease::acquire(&directory).unwrap().unwrap();
        let name = ".download-Ab12Cd.part";
        std::fs::write(directory.join(name), b"owned").unwrap();
        std::fs::write(outside.path().join(name), b"outside").unwrap();
        let moved = directory.with_extension("moved");
        std::fs::rename(&directory, &moved).unwrap();
        symlink(outside.path(), &directory).unwrap();
        lease.recover().unwrap();
        assert_eq!(
            std::fs::read(outside.path().join(name)).unwrap(),
            b"outside"
        );
        assert!(!moved.join(name).exists());
        assert!(!outside.path().join(".installation.lock").exists());
        let (partial, mut file) = DownloadPartial::create(&lease.directory).unwrap();
        std::io::Write::write_all(&mut file, b"anchored").unwrap();
        let partial_name = partial.name.clone();
        assert!(moved.join(&partial_name).exists());
        assert!(!outside.path().join(&partial_name).exists());
        drop(file);
        drop(partial);
        assert!(!moved.join(&partial_name).exists());
        let outside_dir =
            Dir::open_ambient_dir(outside.path(), cap_std::ambient_authority()).unwrap();
        assert!(ensure_same_directory(&lease.directory, &outside_dir).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn recovery_rejects_linked_directories_and_lock_and_preserves_linked_partials() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("target");
        std::fs::write(&target, b"outside").unwrap();
        let directory = recovery_directory(root.path());
        symlink(&target, directory.join(".download-Ab12Cd.part")).unwrap();
        let installer = ModelInstaller::new(root.path().into());
        assert_eq!(installer.recover_interrupted().unwrap().removed_files, 0);
        assert!(
            std::fs::symlink_metadata(directory.join(".download-Ab12Cd.part"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        std::fs::remove_file(directory.join(".installation.lock")).unwrap();
        symlink(&target, directory.join(".installation.lock")).unwrap();
        assert!(installer.recover_interrupted().is_err());
        let linked_root = root.path().join("linked-root");
        symlink(outside.path(), &linked_root).unwrap();
        assert!(ModelInstaller::new(linked_root)
            .recover_interrupted()
            .is_err());
        let linked_bundle_root = root.path().join("linked-bundle");
        std::fs::create_dir(&linked_bundle_root).unwrap();
        symlink(
            outside.path(),
            linked_bundle_root.join(MODEL_REVISIONS[0].0),
        )
        .unwrap();
        assert!(ModelInstaller::new(linked_bundle_root)
            .recover_interrupted()
            .is_err());
        assert_eq!(std::fs::read(target).unwrap(), b"outside");
    }

    #[test]
    fn recovery_after_forced_process_exit_releases_os_lease() {
        const CHILD_ROOT: &str = "CACHELLA_RECOVERY_CHILD_ROOT";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let directory = recovery_directory(Path::new(&root));
            let _lease = RevisionLease::acquire(&directory).unwrap().unwrap();
            let partial = tempfile::Builder::new()
                .prefix(".download-")
                .suffix(".part")
                .tempfile_in(&directory)
                .unwrap();
            std::fs::write(partial.path(), b"abandoned transfer").unwrap();
            std::fs::write(Path::new(&root).join("ready"), b"ready").unwrap();
            std::thread::sleep(Duration::from_secs(20));
            panic!("parent did not terminate fixture");
        }
        let root = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "models::tests::recovery_after_forced_process_exit_releases_os_lease",
                "--nocapture",
            ])
            .env(CHILD_ROOT, root.path())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !root.path().join("ready").exists() && std::time::Instant::now() < deadline {
            if child.try_wait().unwrap().is_some() {
                panic!("child exited before readiness");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        if !root.path().join("ready").exists() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child readiness timed out");
        }
        let installer = ModelInstaller::new(root.path().into());
        let busy = installer.recover_interrupted();
        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(busy.unwrap().busy_directories, 1);
        let result = installer.recover_interrupted().unwrap();
        assert_eq!(result.removed_files, 1);
        assert_eq!(result.removed_bytes, 18);
    }

    #[test]
    fn catalog_is_pinned_and_has_exact_totals() {
        let entries = catalog();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].id, "qwen3-1.7b-q4");
        assert_eq!(entries[0].total_bytes, 1_107_409_472);
        assert_eq!(entries[1].id, "gemma4-e2b-q4");
        assert_eq!(entries[1].total_bytes, 3_398_849_248);
        assert!(!entries[1].embedding);
        assert_eq!(entries[2].id, "embeddinggemma2-q8");
        assert_eq!(entries[2].total_bytes, 864_676_480);
        assert!(entries[2].embedding);
        assert!(entries.iter().all(|b| b.license == "Apache-2.0"));
    }

    #[test]
    fn rejects_untrusted_urls() {
        for url in [
            "http://huggingface.co/a",
            "https://huggingface.co.evil.test/a",
            "https://evil.test/a",
            "https://user@huggingface.co/a",
            "https://huggingface.co:444/a",
            "https://127.0.0.1/a",
            "https://huggingface.co/a#fragment",
        ] {
            assert!(!trusted_url(url), "{url}");
        }
        for url in [
            "https://huggingface.co/a",
            "https://us.aws.cdn.hf.co/a",
            "https://cdn.huggingface.co/a",
        ] {
            assert!(trusted_url(url), "{url}");
        }
    }

    #[tokio::test]
    async fn verifies_header_length_and_checksum() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("model.gguf");
        let bytes = b"GGUFfixture";
        let sha = format!("{:x}", Sha256::digest(bytes));
        std::fs::write(&path, bytes).unwrap();
        let cancel = AtomicBool::new(false);
        assert!(verify_file(&path, bytes.len() as u64, &sha, &cancel)
            .await
            .is_ok());
        assert!(verify_file(&path, bytes.len() as u64 + 1, &sha, &cancel)
            .await
            .is_err());
        assert!(
            verify_file(&path, bytes.len() as u64, &"0".repeat(64), &cancel)
                .await
                .is_err()
        );
        std::fs::write(&path, b"FAILfixture").unwrap();
        let sha = format!("{:x}", Sha256::digest(b"FAILfixture"));
        assert!(verify_file(&path, bytes.len() as u64, &sha, &cancel)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn verification_observes_cancellation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("model.gguf");
        std::fs::write(&path, b"GGUFfixture").unwrap();
        let result = verify_file(
            &path,
            11,
            &format!("{:x}", Sha256::digest(b"GGUFfixture")),
            &AtomicBool::new(true),
        )
        .await;
        assert_eq!(result.unwrap_err(), "Model installation cancelled.");
    }

    static INSTALL_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    struct FixtureServer(tokio::task::JoinHandle<()>);
    impl Drop for FixtureServer {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    async fn fixture(
        body: &'static [u8],
        length: Option<u64>,
        delay: Duration,
    ) -> (
        Asset,
        Transport,
        FixtureServer,
        tokio::sync::oneshot::Receiver<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/fixture", listener.local_addr().unwrap());
        let (sent, headers) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            let mut buffer = [0; 1024];
            loop {
                let size = socket.read(&mut buffer).await.unwrap();
                if size == 0 {
                    return;
                }
                request.extend_from_slice(&buffer[..size]);
                if request.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            let length = length
                .map(|bytes| format!("Content-Length: {bytes}\r\n"))
                .unwrap_or_default();
            socket
                .write_all(
                    format!("HTTP/1.1 200 OK\r\n{length}Connection: close\r\n\r\n").as_bytes(),
                )
                .await
                .unwrap();
            let _ = sent.send(());
            tokio::time::sleep(delay).await;
            let _ = socket.write_all(body).await;
        });
        let client = reqwest::Client::builder()
            .no_proxy()
            .timeout(Duration::from_secs(2))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        (
            Asset {
                name: "fixture.gguf",
                url: Box::leak(url.into_boxed_str()),
                bytes: 11,
                sha: Box::leak(format!("{:x}", Sha256::digest(b"GGUFfixture")).into_boxed_str()),
            },
            Transport::Fixture(client),
            FixtureServer(task),
            headers,
        )
    }

    fn fixture_bundle() -> ModelBundle {
        let mut bundle = catalog().remove(0);
        bundle.id = "fixture".into();
        bundle.total_bytes = 11;
        bundle
    }

    fn assert_no_partials(root: &Path) {
        let directory = root.join("fixture").join("revision");
        for entry in std::fs::read_dir(directory).unwrap() {
            assert!(!entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .ends_with(".part"));
        }
    }

    #[tokio::test]
    async fn installs_verified_fixture_and_reuses_without_network() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, server, _) = fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        let installed = installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&installed.model).unwrap(), b"GGUFfixture");
        assert_eq!(installer.status().unwrap().phase, "ready");
        assert_eq!(installer.status().unwrap().downloaded_bytes, 11);
        drop(server);
        assert!(installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .is_ok());
        assert_no_partials(root.path());
    }

    #[tokio::test]
    async fn install_rejects_an_external_revision_lease_without_touching_its_partial() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("fixture/revision");
        std::fs::create_dir_all(&directory).unwrap();
        let lease = RevisionLease::acquire(&directory).unwrap().unwrap();
        let partial = directory.join(".download-Ab12Cd.part");
        std::fs::write(&partial, b"active transfer").unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, _server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        assert_eq!(
            installer
                .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                .await
                .unwrap_err(),
            "Another Cachella instance is installing this model. Retry after it finishes."
        );
        assert_eq!(std::fs::read(&partial).unwrap(), b"active transfer");
        assert_eq!(installer.status().unwrap().phase, "error");
        drop(lease);
        installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .unwrap();
        assert!(!partial.exists());
    }

    #[tokio::test]
    async fn retry_removes_abandoned_partials_and_preserves_other_files() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("fixture/revision");
        std::fs::create_dir_all(&directory).unwrap();
        let abandoned = directory.join(".download-Ab12Cd.part");
        std::fs::write(&abandoned, b"interrupted").unwrap();
        std::fs::write(directory.join("personal.part"), b"keep").unwrap();
        std::fs::write(directory.join("other.gguf"), b"keep model").unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, _server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .unwrap();
        assert!(
            !abandoned.exists(),
            "retry left the abandoned partial on disk"
        );
        assert_eq!(
            std::fs::read(directory.join("personal.part")).unwrap(),
            b"keep"
        );
        assert_eq!(
            std::fs::read(directory.join("other.gguf")).unwrap(),
            b"keep model"
        );
        assert_eq!(
            std::fs::read(directory.join("fixture.gguf")).unwrap(),
            b"GGUFfixture"
        );
    }

    #[tokio::test]
    async fn rejects_corrupt_existing_file_without_clobbering_it() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("fixture/revision");
        std::fs::create_dir_all(&directory).unwrap();
        let destination = directory.join("fixture.gguf");
        std::fs::write(&destination, b"GGUFchanged").unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, _server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        assert!(installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .is_err());
        assert_eq!(std::fs::read(destination).unwrap(), b"GGUFchanged");
        assert_eq!(installer.status().unwrap().phase, "error");
    }

    #[tokio::test]
    async fn rejects_oversized_and_corrupt_downloads_with_partial_cleanup() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        for body in [
            &b"GGUFfixtureX"[..],
            &b"GGUFchanged"[..],
            &b"FAILfixture"[..],
        ] {
            let root = tempfile::tempdir().unwrap();
            let installer = ModelInstaller::new(root.path().into());
            let (asset, transport, _server, _) = fixture(body, None, Duration::ZERO).await;
            assert!(installer
                .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                .await
                .is_err());
            assert_eq!(installer.status().unwrap().phase, "error");
            assert!(!root.path().join("fixture/revision/fixture.gguf").exists());
            assert_no_partials(root.path());
        }
    }

    #[tokio::test]
    async fn cancellation_releases_global_gate_and_retry_restarts() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, _server, headers) =
            fixture(b"GGUFfixture", Some(11), Duration::from_secs(30)).await;
        let owner = installer.clone();
        let task = tokio::spawn(async move {
            owner
                .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                .await
        });
        headers.await.unwrap();
        let other = ModelInstaller::new(root.path().join("other"));
        let (next_asset, next_transport, _next_server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        assert_eq!(
            other
                .install_assets(
                    &fixture_bundle(),
                    "revision",
                    &[next_asset],
                    &next_transport
                )
                .await
                .unwrap_err(),
            "A model installation is already running."
        );
        installer.cancel().unwrap();
        let result = tokio::time::timeout(Duration::from_millis(500), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.unwrap_err(), CANCELLED);
        assert_eq!(installer.status().unwrap().phase, "cancelled");
        assert_no_partials(root.path());
        assert!(installer
            .install_assets(
                &fixture_bundle(),
                "revision",
                &[next_asset],
                &next_transport
            )
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn dropped_install_future_releases_gate_and_removes_partial() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, _server, headers) =
            fixture(b"GGUFfixture", Some(11), Duration::from_secs(30)).await;
        let owner = installer.clone();
        let task = tokio::spawn(async move {
            owner
                .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                .await
        });
        headers.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(installer.status().unwrap().phase, "cancelled");
        assert_no_partials(root.path());
        let (asset, transport, _server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        assert!(installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn unknown_bundle_does_not_create_files_or_change_progress() {
        let root = tempfile::tempdir().unwrap();
        let installer = ModelInstaller::new(root.path().join("models"));
        assert!(installer
            .install("https://evil.test/model.gguf")
            .await
            .is_err());
        assert_eq!(installer.status().unwrap().phase, "idle");
        assert!(!root.path().join("models").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn refuses_symlinked_install_directories() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("fixture")).unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, _server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        assert!(installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .is_err());
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn publication_never_clobbers_a_concurrently_created_destination() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let installer = ModelInstaller::new(root.path().into());
        let (asset, transport, _server, headers) =
            fixture(b"GGUFfixture", Some(11), Duration::from_millis(30)).await;
        let owner = installer.clone();
        let task = tokio::spawn(async move {
            owner
                .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                .await
        });
        headers.await.unwrap();
        let destination = root.path().join("fixture/revision/fixture.gguf");
        std::fs::write(&destination, b"GGUFchanged").unwrap();
        assert!(task.await.unwrap().is_err());
        assert_eq!(std::fs::read(destination).unwrap(), b"GGUFchanged");
        assert_no_partials(root.path());
    }

    #[tokio::test]
    async fn truncated_or_wrong_declared_size_never_publishes() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        for length in [None, Some(12)] {
            let root = tempfile::tempdir().unwrap();
            let installer = ModelInstaller::new(root.path().into());
            let (asset, transport, _server, _) =
                fixture(b"GGUFshort", length, Duration::ZERO).await;
            assert!(installer
                .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                .await
                .is_err());
            assert_eq!(installer.status().unwrap().phase, "error");
            assert!(!root.path().join("fixture/revision/fixture.gguf").exists());
            assert_no_partials(root.path());
        }
    }

    #[tokio::test]
    async fn stale_cancel_notification_does_not_cancel_a_new_install() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let installer = ModelInstaller::new(root.path().into());
        installer.state.cancelled.store(true, Ordering::Release);
        installer.state.cancel_signal.notify_one();
        let (asset, transport, _server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        assert!(installer
            .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
            .await
            .is_ok());
        assert_eq!(installer.status().unwrap().phase, "ready");
    }

    #[tokio::test]
    async fn shutdown_permanently_rejects_new_installs() {
        let _serial = INSTALL_TEST_LOCK.lock().await;
        let root = tempfile::tempdir().unwrap();
        let installer = ModelInstaller::new(root.path().into());
        assert!(!installer.begin_shutdown().unwrap());
        installer.wait_idle().await;
        let (asset, transport, _server, _) =
            fixture(b"GGUFfixture", Some(11), Duration::ZERO).await;
        assert_eq!(
            installer
                .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                .await
                .unwrap_err(),
            "Model installer is shutting down."
        );
        assert_eq!(installer.status().unwrap().phase, "idle");
    }

    #[test]
    fn shutdown_child_exits_after_stalled_transfer_cleanup() {
        const CHILD_ROOT: &str = "CACHELLA_MODEL_SHUTDOWN_TEST_ROOT";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let installer = ModelInstaller::new(PathBuf::from(&root));
                let (asset, transport, _server, headers) =
                    fixture(b"GGUFfixture", Some(11), Duration::from_secs(30)).await;
                let owner = installer.clone();
                let _task = tokio::spawn(async move {
                    owner
                        .install_assets(&fixture_bundle(), "revision", &[asset], &transport)
                        .await
                });
                headers.await.unwrap();
                let directory = PathBuf::from(&root).join("fixture/revision");
                tokio::time::timeout(Duration::from_secs(1), async {
                    loop {
                        if std::fs::read_dir(&directory).unwrap().any(|entry| {
                            entry
                                .unwrap()
                                .file_name()
                                .to_string_lossy()
                                .ends_with(".part")
                        }) {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                })
                .await
                .unwrap();
                assert!(installer.begin_shutdown().unwrap());
                tokio::time::timeout(Duration::from_secs(1), installer.wait_idle())
                    .await
                    .unwrap();
                assert_eq!(
                    installer.install("qwen3-1.7b-q4").await.unwrap_err(),
                    "Model installer is shutting down."
                );
                // Tauri exits the process directly, bypassing ordinary Rust destructors.
                std::process::exit(0);
            });
            unreachable!();
        }
        let root = tempfile::tempdir().unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "models::tests::shutdown_child_exits_after_stalled_transfer_cleanup",
                "--nocapture",
            ])
            .env(CHILD_ROOT, root.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_no_partials(root.path());
    }
}
