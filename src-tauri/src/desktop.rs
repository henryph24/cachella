use crate::{
    embedding, inference, media, models,
    runtime::{Runtime, RuntimeConfig, RuntimeStatus},
    store::{Document, Hit, Result, Store},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{Manager, State};
use tokio::sync::Semaphore;

struct AppState {
    database: PathBuf,
    settings: PathBuf,
    endpoint: Mutex<String>,
    generation: Arc<Semaphore>,
    operations: crate::operation_jobs::OperationJobs,
    runtime: Mutex<Runtime>,
    runtime_settings: PathBuf,
    embedding_runtime: Mutex<Runtime>,
    embedding_settings: PathBuf,
    embedding_profile: Mutex<Option<String>>,
    memory_policy: Mutex<MemoryPolicy>,
    memory_settings: PathBuf,
    resume_generation: AtomicBool,
    resume_embedding: AtomicBool,
    media_selection: Mutex<MediaSelection>,
    media_settings: PathBuf,
    media_resource_dir: PathBuf,
    models: models::ModelInstaller,
    bundled_engine: Option<PathBuf>,
    closing: AtomicBool,
    quit_waiter: AtomicBool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum MemoryPolicy {
    #[default]
    Both,
    Single,
}

fn load_memory_policy(path: &std::path::Path) -> MemoryPolicy {
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

struct StartupCleanup<'a> {
    state: &'a AppState,
    embedding: bool,
    connection: crate::runtime::Connection,
    committed: bool,
}

impl Drop for StartupCleanup<'_> {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        let mutex = if self.embedding {
            &self.state.embedding_runtime
        } else {
            &self.state.runtime
        };
        // Cleanup must still own its child if another task poisoned the mutex.
        let mut runtime = mutex
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if runtime.stop_if_connection(&self.connection).is_err() {
            eprintln!("Could not finish cleanup of an interrupted model startup.");
        }
    }
}

impl AppState {
    fn policy(&self) -> Result<MemoryPolicy> {
        self.memory_policy
            .lock()
            .map(|value| *value)
            .map_err(|_| "Memory policy is unavailable.".into())
    }

    fn resume_flag(&self, embedding: bool) -> &AtomicBool {
        if embedding {
            &self.resume_embedding
        } else {
            &self.resume_generation
        }
    }

    fn runtime_status(&self, embedding: bool) -> Result<RuntimeStatus> {
        let auto_resume = self.policy()? == MemoryPolicy::Single
            && self.resume_flag(embedding).load(Ordering::Acquire);
        let mut status = self.runtime_for(embedding)?.status()?;
        status.auto_resume = auto_resume;
        Ok(status)
    }

    fn set_policy(&self, policy: MemoryPolicy) -> Result<MemoryPolicy> {
        let _permit = self
            .generation
            .try_acquire()
            .map_err(|_| "Wait for the current model operation to finish.".to_string())?;
        if self.closing.load(Ordering::Acquire) {
            return Err("Cachella is closing.".into());
        }
        let mut current = self
            .memory_policy
            .lock()
            .map_err(|_| "Memory policy is unavailable.")?;
        if self.runtime()?.status()?.running || self.runtime_for(true)?.status()?.running {
            return Err("Stop both models before changing the memory policy.".into());
        }
        crate::settings::save_json(&self.memory_settings, &policy)?;
        *current = policy;
        self.resume_generation.store(false, Ordering::Release);
        self.resume_embedding.store(false, Ordering::Release);
        Ok(policy)
    }

    // Call only while the shared operation permit is held. Polling never starts models.
    async fn ensure_resident(&self, embedding: bool) -> Result<()> {
        if self.policy()? == MemoryPolicy::Single
            && self.resume_flag(embedding).load(Ordering::Acquire)
            && !self.runtime_for(embedding)?.status()?.ready
        {
            start_runtime_admitted(embedding, self).await?;
        }
        Ok(())
    }

    fn stop_managed(&self, embedding: bool) -> Result<RuntimeStatus> {
        let _permit = self
            .generation
            .try_acquire()
            .map_err(|_| "Wait for the current answer or model startup to finish.".to_string())?;
        self.runtime_for(embedding)?.stop()?;
        self.resume_flag(embedding).store(false, Ordering::Release);
        self.runtime_status(embedding)
    }

    fn runtime(&self) -> Result<std::sync::MutexGuard<'_, Runtime>> {
        self.runtime
            .lock()
            .map_err(|_| "Runtime state is unavailable.".into())
    }
    fn runtime_for(&self, embedding: bool) -> Result<std::sync::MutexGuard<'_, Runtime>> {
        if embedding {
            self.embedding_runtime
                .lock()
                .map_err(|_| "Embedding runtime is unavailable.".into())
        } else {
            self.runtime()
        }
    }
    fn embedding_connection(&self) -> Result<(crate::runtime::Connection, String)> {
        let connection = {
            let mut runtime = self.runtime_for(true)?;
            if !runtime.status()?.ready {
                return Err("Start the embedding model in Local runtime first.".into());
            }
            runtime
                .connection()?
                .ok_or("Start the embedding model in Local runtime first.")?
        };
        let profile = self
            .embedding_profile
            .lock()
            .map_err(|_| "Embedding profile is unavailable.")?
            .clone()
            .ok_or("Start the embedding model before indexing or semantic search.")?;
        Ok((connection, profile))
    }
    async fn retrieve(&self, query: &str, semantic: bool) -> Result<Vec<Hit>> {
        if !semantic {
            return self.store()?.search(query);
        }
        self.ensure_resident(true).await?;
        let (connection, profile) = self.embedding_connection()?;
        let (indexed, total) = self.store()?.embedding_counts(&profile)?;
        if indexed != total {
            return Err(format!("Semantic index has {indexed} of {total} passages. Resume indexing in Local runtime first."));
        }
        if total == 0 {
            return Ok(vec![]);
        }
        let vector = embedding::query(&connection, query).await?;
        self.store()?.semantic_search(&profile, &vector)
    }
    fn connection(&self) -> Result<(String, Option<String>, Option<String>)> {
        if let Some(connection) = self.runtime()?.connection()? {
            Ok((
                connection.endpoint,
                Some(connection.token),
                Some(connection.certificate),
            ))
        } else {
            Ok((self.endpoint()?, None, None))
        }
    }
    fn store(&self) -> Result<Store> {
        Store::open(&self.database)
    }
    fn delete(&self, id: &str) -> Result<()> {
        let _permit = self.generation.try_acquire().map_err(|_| {
            "An answer is using library evidence. Wait for it to finish before deleting."
                .to_string()
        })?;
        self.store()?.delete(id)
    }
    fn endpoint(&self) -> Result<String> {
        self.endpoint
            .lock()
            .map(|v| v.clone())
            .map_err(|_| "Settings are unavailable.".into())
    }

    fn selected_media_executable(&self) -> Result<Option<PathBuf>> {
        let selection = self
            .media_selection
            .lock()
            .map_err(|_| "Media settings are unavailable.")?
            .clone();
        Ok(match selection.source {
            MediaSource::Included => media::included_tool(&self.media_resource_dir),
            MediaSource::Manual => selection
                .executable
                .as_deref()
                .and_then(|path| media::validate_tool(path).ok()),
        })
    }

    fn select_media(&self, selection: MediaSelection) -> Result<MediaTools> {
        let _permit = self
            .generation
            .try_acquire()
            .map_err(|_| "Wait for the current model operation to finish.".to_string())?;
        if self.closing.load(Ordering::Acquire) {
            return Err("Cachella is closing.".into());
        }
        let mut current = self
            .media_selection
            .lock()
            .map_err(|_| "Media settings are unavailable.")?;
        crate::settings::save_json(&self.media_settings, &selection.persisted())?;
        *current = selection;
        drop(current);
        self.media_tools()
    }

    fn media_tools(&self) -> Result<MediaTools> {
        let selection = self
            .media_selection
            .lock()
            .map_err(|_| "Media settings are unavailable.")?
            .clone();
        let included = media::included_tool(&self.media_resource_dir);
        let executable = match selection.source {
            MediaSource::Included => included.clone(),
            MediaSource::Manual => selection.executable.clone(),
        };
        let available = match selection.source {
            MediaSource::Included => included.is_some(),
            MediaSource::Manual => executable
                .as_deref()
                .is_some_and(|path| media::validate_tool(path).is_ok()),
        };
        Ok(MediaTools {
            executable,
            source: selection.source,
            available,
            included_available: included.is_some(),
        })
    }

    fn select_runtime_path(
        &self,
        embedding: bool,
        kind: &str,
        path: PathBuf,
        bundled: bool,
    ) -> Result<RuntimeStatus> {
        let _permit = self
            .generation
            .try_acquire()
            .map_err(|_| "Wait for the current model operation to finish.".to_string())?;
        let mut runtime = self.runtime_for(embedding)?;
        if self.closing.load(Ordering::Acquire) {
            return Err("Cachella is closing.".into());
        }
        if runtime.status()?.running {
            return Err("Stop the model before changing its files.".into());
        }
        let mut config = runtime.config.clone();
        match kind {
            "model" => {
                config.model = Some(crate::runtime::validate_model(&path)?);
                if !embedding {
                    config.projector = None;
                }
            }
            "projector" if embedding => {
                config.projector = Some(crate::runtime::validate_model(&path)?)
            }
            "executable" => {
                config.executable = Some(path);
                config.bundled_engine = bundled;
            }
            _ => return Err("Unknown runtime file type.".into()),
        }
        save_runtime_settings(
            if embedding {
                &self.embedding_settings
            } else {
                &self.runtime_settings
            },
            &config,
        )?;
        runtime.configure(config)?;
        self.resume_flag(embedding).store(false, Ordering::Release);
        if embedding {
            *self
                .embedding_profile
                .lock()
                .map_err(|_| "Embedding profile is unavailable.")? = None;
        }
        runtime.status()
    }

    fn select_bundled_engine(&self, embedding: bool) -> Result<RuntimeStatus> {
        let path = self.bundled_engine.clone().filter(|p| p.is_file())
            .ok_or("This build has no included inference engine. Choose a trusted llama-server installation.")?;
        self.select_runtime_path(embedding, "executable", path, true)
    }

    fn start_owned(&self, embedding: bool) -> Result<crate::runtime::Connection> {
        // All production callers hold the shared operation permit. Check the
        // target first so a duplicate Start cannot evict an unrelated model.
        if self.runtime_for(embedding)?.status()?.running {
            return Err("The managed model is already running.".into());
        }
        if self.policy()? == MemoryPolicy::Single {
            self.runtime_for(!embedding)?.stop()?;
        }
        let mut runtime = self.runtime_for(embedding)?;
        // Exit uses the same lock: a late startup cannot create a child after
        // the shutdown callback has stopped that runtime.
        if self.closing.load(Ordering::Acquire) {
            return Err("Cachella is closing.".into());
        }
        if embedding {
            runtime.start_embedding()
        } else {
            runtime.start()
        }
    }

    fn select_installed_models(
        &self,
        installed: models::InstalledModels,
        expected: &RuntimeConfig,
    ) -> Result<RuntimeStatus> {
        if self.closing.load(Ordering::Acquire) {
            return Err(
                "Cachella is closing. Verified model files remain available for the next start."
                    .into(),
            );
        }
        let _permit = self.generation.try_acquire().map_err(|_| {
            "Model files are verified. Wait for the current operation, then retry installation to select them.".to_string()
        })?;
        let mut runtime = self.runtime_for(installed.embedding)?;
        if runtime.status()?.running || &runtime.config != expected {
            return Err("Model files are verified. The selected runtime changed during installation. Stop it and retry installation to select the downloaded files.".into());
        }
        let mut config = runtime.config.clone();
        config.model = Some(installed.model);
        config.projector = installed.projector;
        save_runtime_settings(
            if installed.embedding {
                &self.embedding_settings
            } else {
                &self.runtime_settings
            },
            &config,
        )?;
        runtime.configure(config)?;
        self.resume_flag(installed.embedding)
            .store(false, Ordering::Release);
        if installed.embedding {
            *self
                .embedding_profile
                .lock()
                .map_err(|_| "Embedding profile is unavailable.")? = None;
        }
        runtime.status()
    }
}

fn save_runtime_settings(path: &std::path::Path, config: &RuntimeConfig) -> Result<()> {
    crate::settings::save_json(path, config)
}

#[derive(Serialize)]
struct Workspace {
    documents: Vec<Document>,
    endpoint: String,
    data_path: String,
}

#[derive(Serialize)]
struct ImportReport {
    imported: Vec<Document>,
    errors: Vec<String>,
}

#[tauri::command]
async fn workspace(state: State<'_, AppState>) -> Result<Workspace> {
    Ok(Workspace {
        documents: state.store()?.documents()?,
        endpoint: state.endpoint()?,
        data_path: state
            .database
            .parent()
            .unwrap_or(&state.database)
            .display()
            .to_string(),
    })
}

#[tauri::command]
async fn import_documents(state: State<'_, AppState>) -> Result<ImportReport> {
    let selected = rfd::AsyncFileDialog::new()
        .set_title("Import documents into Cachella")
        .add_filter(
            "Documents and media",
            &[
                "txt", "md", "pdf", "png", "jpg", "jpeg", "wav", "mp3", "flac", "ogg", "mp4",
                "webm",
            ],
        )
        .pick_files()
        .await;
    let mut report = ImportReport {
        imported: vec![],
        errors: vec![],
    };
    if let Some(files) = selected {
        if files.len() > 50 {
            report.errors.push(format!("Selected {} files. Only the first 50 were processed; import the remaining files separately.",files.len()));
        }
        let _permit = state.generation.try_acquire().map_err(|_| {
            "Wait for the current model operation to finish before importing.".to_string()
        })?;
        let mut prepared_count = 0;
        for file in files.into_iter().take(50) {
            let path = file.path();
            let title = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Document");
            let kind = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let result = if matches!(kind.as_str(), "txt" | "md") {
                (|| -> Result<Document> {
                    let text = crate::store::read_text_file_bounded(path)?;
                    state.store()?.import_text(title, &kind, &text)
                })()
            } else {
                prepared_count += 1;
                if prepared_count > 5 {
                    Err(
                        "Import up to five PDF or media files per batch. Import this file separately."
                            .into(),
                    )
                } else if kind == "pdf" {
                    match crate::pdf::prepare(path).await {
                        Ok(prepared) => state.store()?.import_pdf(title, &prepared),
                        Err(error) => Err(error),
                    }
                } else {
                    let executable = state.selected_media_executable()?;
                    if let Some(executable) = executable {
                        match media::prepare(&executable, path).await {
                            Ok(prepared) => state.store()?.import_media(title, &prepared),
                            Err(error) => Err(error),
                        }
                    } else {
                        Err("Select an available media decoder before importing media.".into())
                    }
                }
            };
            match result {
                Ok(doc) => report.imported.push(doc),
                Err(error) => report.errors.push(format!("{title}: {error}")),
            }
        }
    }
    Ok(report)
}

#[tauri::command]
async fn add_sample_documents(state: State<'_, AppState>) -> Result<Vec<Document>> {
    let _permit = state
        .generation
        .try_acquire()
        .map_err(|_| "Wait for the current model operation to finish.".to_string())?;
    let mut db = state.store()?;
    for (title,text) in [
        ("Launch decision.md", "# Launch decision\nSynthetic demonstration document.\nThe Atlas launch moved from Tuesday to Friday. The security review found an unresolved permission bug in shared document previews. Priya owns the fix. The launch can proceed when the permission regression tests pass. The product team approved the revised Friday launch."),
        ("Infrastructure budget.md", "# Infrastructure budget\nSynthetic demonstration document.\nCachella's monthly cloud budget is less than ten US dollars. The desktop application runs a quantized model locally. There is no recurring hosted inference charge for local generation. Model performance and electricity costs depend on the employee's device. Fly.io may host an optional supporting service after separate approval."),
        ("Workspace principles.md", "# Workspace principles\nSynthetic demonstration document.\nDocuments and search indexes stay on the employee's computer. File import requires an explicit user action. An answer cites supporting library sources. A local prompt cache can reuse matching prefixes. LMCache is reserved for a future compatible enterprise inference server. An unanswered question should show missing evidence clearly.")
    ] { db.import_text(title,"md",text)?; }
    db.documents()
}

#[tauri::command]
async fn search_library(
    query: String,
    semantic: Option<bool>,
    request_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<Hit>> {
    state
        .operations
        .run(
            &request_id,
            state.generation.clone(),
            state.retrieve(&query, semantic.unwrap_or(false)),
        )
        .await
}

#[tauri::command]
async fn read_document(id: String, state: State<'_, AppState>) -> Result<String> {
    state.store()?.document_text(&id)
}

#[tauri::command]
async fn read_pdf_pages(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<crate::pdf::PdfPage>> {
    state.store()?.pdf_pages(&id)
}

#[tauri::command]
async fn delete_document(id: String, state: State<'_, AppState>) -> Result<()> {
    state.delete(&id)
}

#[tauri::command]
async fn save_endpoint(value: String, state: State<'_, AppState>) -> Result<String> {
    persist_endpoint(&state, &value)
}

fn persist_endpoint(state: &AppState, value: &str) -> Result<String> {
    let value = inference::endpoint(value)?;
    let _permit = state
        .generation
        .try_acquire()
        .map_err(|_| "Wait for the current model operation to finish.".to_string())?;
    let mut runtime = state.runtime()?;
    if state.closing.load(Ordering::Acquire) {
        return Err("Cachella is closing.".into());
    }
    if runtime.status()?.running {
        return Err("Stop the managed model before switching connections.".into());
    }
    let mut current = state
        .endpoint
        .lock()
        .map_err(|_| "Settings are unavailable.".to_string())?;
    crate::settings::save_json(&state.settings, &value)?;
    runtime.use_external()?;
    state.resume_generation.store(false, Ordering::Release);
    *current = value.clone();
    Ok(value)
}

#[tauri::command]
async fn engine_status(state: State<'_, AppState>) -> Result<inference::EngineStatus> {
    let (endpoint, token, certificate) = state.connection()?;
    inference::status_authenticated(&endpoint, token.as_deref(), certificate.as_deref()).await
}

#[tauri::command]
fn begin_operation(state: State<'_, AppState>) -> Result<String> {
    state.operations.begin()
}

#[tauri::command]
fn cancel_operation(request_id: String, state: State<'_, AppState>) -> Result<bool> {
    state.operations.cancel(&request_id)
}

#[tauri::command]
async fn ask_library(
    question: String,
    cache: bool,
    semantic: Option<bool>,
    request_id: String,
    state: State<'_, AppState>,
) -> Result<inference::AnswerResult> {
    state
        .operations
        .run(&request_id, state.generation.clone(), async {
            let hits = state.retrieve(&question, semantic.unwrap_or(false)).await?;
            state.ensure_resident(false).await?;
            let (endpoint, token, certificate) = state.connection()?;
            let media = if certificate.is_some() && state.runtime()?.config.projector.is_some() {
                Some(inference::load_answer_media(&state.store()?, &hits)?)
            } else {
                None
            };
            let result = inference::answer_with_media_authenticated(
                &endpoint,
                &question,
                hits,
                cache,
                token.as_deref(),
                certificate.as_deref(),
                media.as_ref(),
            )
            .await?;
            let db = state.store()?;
            for source in &result.sources {
                if db.document_text(&source.document_id).is_err() {
                    return Err(
                "The library changed during generation. Search again to refresh the evidence."
                    .into(),
            );
                }
            }
            Ok(result)
        })
        .await
}

#[tauri::command]
async fn managed_runtime(
    embedding: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RuntimeStatus> {
    state.runtime_status(embedding.unwrap_or(false))
}

#[tauri::command]
async fn choose_runtime_file(
    kind: String,
    embedding: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RuntimeStatus> {
    let embedding = embedding.unwrap_or(false);
    if kind != "model" && kind != "executable" && !(embedding && kind == "projector") {
        return Err("Unknown runtime file type.".into());
    }
    let mut dialog = rfd::AsyncFileDialog::new();
    dialog = if kind == "model" || kind == "projector" {
        dialog
            .set_title("Choose a GGUF model")
            .add_filter("GGUF model", &["gguf"])
    } else {
        dialog.set_title("Choose llama-server from your trusted llama.cpp installation")
    };
    if let Some(file) = dialog.pick_file().await {
        return state.select_runtime_path(embedding, &kind, file.path().to_path_buf(), false);
    }
    state.runtime_status(embedding)
}

#[tauri::command]
fn bundled_engine(state: State<'_, AppState>) -> Option<PathBuf> {
    state.bundled_engine.clone().filter(|p| p.is_file())
}

#[tauri::command]
fn use_bundled_engine(
    embedding: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RuntimeStatus> {
    state.select_bundled_engine(embedding.unwrap_or(false))
}

#[tauri::command]
fn model_catalog() -> Vec<models::ModelBundle> {
    models::catalog()
}

#[tauri::command]
fn model_install_status(state: State<'_, AppState>) -> Result<models::InstallProgress> {
    state.models.status()
}

#[tauri::command]
fn cancel_model_install(state: State<'_, AppState>) -> Result<()> {
    state.models.cancel()
}

#[tauri::command]
async fn install_model(bundle_id: String, state: State<'_, AppState>) -> Result<RuntimeStatus> {
    if state.closing.load(Ordering::Acquire) {
        return Err("Cachella is closing.".into());
    }
    let bundle = models::catalog()
        .into_iter()
        .find(|b| b.id == bundle_id)
        .ok_or("Unknown model bundle.")?;
    let expected = {
        let _permit = state
            .generation
            .try_acquire()
            .map_err(|_| "Wait for the current model operation before installing.".to_string())?;
        let mut runtime = state.runtime_for(bundle.embedding)?;
        if runtime.status()?.running {
            return Err("Stop this model before installing its replacement.".into());
        }
        runtime.config.clone()
    };
    // Downloading does not hold the inference permit. Selection rechecks state
    // after verification so a long transfer cannot overwrite newer user choices.
    let installed = state.models.install(&bundle_id).await?;
    state.select_installed_models(installed, &expected)
}

#[tauri::command]
async fn start_runtime(
    embedding: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RuntimeStatus> {
    start_runtime_inner(embedding.unwrap_or(false), &state).await
}

async fn start_runtime_inner(embedding: bool, state: &AppState) -> Result<RuntimeStatus> {
    if state.closing.load(Ordering::Acquire) {
        return Err("Cachella is closing.".into());
    }
    let _permit = state
        .generation
        .try_acquire()
        .map_err(|_| "Wait for the current model operation to finish.".to_string())?;
    start_runtime_admitted(embedding, state).await?;
    state.resume_flag(embedding).store(true, Ordering::Release);
    state.runtime_status(embedding)
}

// The caller holds the operation permit across retrieval, switching and generation.
async fn start_runtime_admitted(embedding: bool, state: &AppState) -> Result<RuntimeStatus> {
    if state.closing.load(Ordering::Acquire) {
        return Err("Cachella is closing.".into());
    }
    let profile = if embedding {
        let config = state.runtime_for(true)?.config.clone();
        Some(embedding::profile_cancellable(config).await?)
    } else {
        None
    };
    let connection = state.start_owned(embedding)?;
    let mut cleanup = StartupCleanup {
        state,
        embedding,
        connection: connection.clone(),
        committed: false,
    };
    let ready = tokio::time::timeout(std::time::Duration::from_secs(60), async {
        loop {
            if state.closing.load(Ordering::Acquire) { return Err("Cachella is closing.".to_string()); }
            if !state.runtime_for(embedding)?.status()?.running { return Err("llama-server exited during startup. Check model compatibility and available memory.".to_string()); }
            if let Ok(status) = inference::status_authenticated(&connection.endpoint, Some(&connection.token), Some(&connection.certificate)).await {
                if status.connected {
                    inference::verify_authentication(&connection.endpoint, Some(&connection.certificate)).await?;
                    if !state.runtime_for(embedding)?.status()?.running { return Err("Engine exited during authentication.".to_string()); }
                    if embedding { embedding::query(&connection, "Runtime compatibility check").await?; }
                    return Ok(());
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }).await;
    match ready {
        Ok(Ok(())) => {
            let mut runtime = state.runtime_for(embedding)?;
            if state.closing.load(Ordering::Acquire) {
                return Err("Cachella is closing.".into());
            }
            runtime.mark_ready()?;
            if embedding {
                *state
                    .embedding_profile
                    .lock()
                    .map_err(|_| "Embedding profile is unavailable.")? = profile;
            }
            let status = runtime.status()?;
            cleanup.committed = true;
            Ok(status)
        }
        failure => {
            state
                .runtime_for(embedding)?
                .stop_if_connection(&connection)?;
            cleanup.committed = true;
            Err(match failure {
                Ok(Err(message)) => message,
                _ => "Model startup exceeded 60 seconds. Try a smaller compatible model.".into(),
            })
        }
    }
}

#[tauri::command]
async fn stop_runtime(
    embedding: Option<bool>,
    state: State<'_, AppState>,
) -> Result<RuntimeStatus> {
    state.stop_managed(embedding.unwrap_or(false))
}

#[tauri::command]
fn memory_policy(state: State<'_, AppState>) -> Result<MemoryPolicy> {
    state.policy()
}

#[tauri::command]
fn set_memory_policy(policy: MemoryPolicy, state: State<'_, AppState>) -> Result<MemoryPolicy> {
    state.set_policy(policy)
}

#[tauri::command]
async fn embedding_progress(state: State<'_, AppState>) -> Result<embedding::IndexProgress> {
    // A suspended model's retained profile is sufficient for read-only counts.
    // Every resumed model is rehashed before using vectors for retrieval/indexing.
    let profile = state
        .embedding_profile
        .lock()
        .map_err(|_| "Embedding profile is unavailable.")?
        .clone()
        .ok_or("Start the embedding model to check the index.")?;
    embedding::index_progress(&state.database, &profile)
}

#[tauri::command]
async fn index_library(
    request_id: String,
    state: State<'_, AppState>,
) -> Result<embedding::IndexProgress> {
    state
        .operations
        .run(&request_id, state.generation.clone(), async {
            state.ensure_resident(true).await?;
            let (connection, profile) = state.embedding_connection()?;
            embedding::index_batch(&state.database, &connection, &profile).await
        })
        .await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum MediaSource {
    Included,
    Manual,
}

#[derive(Clone)]
struct MediaSelection {
    source: MediaSource,
    executable: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "lowercase")]
enum PersistedMediaSelection {
    Included,
    Manual { executable: PathBuf },
}

impl MediaSelection {
    fn included() -> Self {
        Self {
            source: MediaSource::Included,
            executable: None,
        }
    }

    fn manual(executable: PathBuf) -> Self {
        Self {
            source: MediaSource::Manual,
            executable: Some(executable),
        }
    }

    fn persisted(&self) -> PersistedMediaSelection {
        match self.source {
            MediaSource::Included => PersistedMediaSelection::Included,
            MediaSource::Manual => PersistedMediaSelection::Manual {
                executable: self
                    .executable
                    .clone()
                    .expect("manual media selection always has an executable"),
            },
        }
    }
}

fn load_media_selection(path: &std::path::Path) -> MediaSelection {
    let Ok(bytes) = std::fs::read(path) else {
        return MediaSelection::included();
    };
    if let Ok(selection) = serde_json::from_slice::<PersistedMediaSelection>(&bytes) {
        return match selection {
            PersistedMediaSelection::Included => MediaSelection::included(),
            PersistedMediaSelection::Manual { executable } => MediaSelection::manual(executable),
        };
    }
    serde_json::from_slice::<PathBuf>(&bytes)
        .map(MediaSelection::manual)
        .unwrap_or_else(|_| MediaSelection::included())
}

#[derive(Serialize)]
struct MediaTools {
    executable: Option<PathBuf>,
    source: MediaSource,
    available: bool,
    included_available: bool,
}

#[tauri::command]
async fn media_tools(state: State<'_, AppState>) -> Result<MediaTools> {
    state.media_tools()
}

#[tauri::command]
async fn choose_media_tool(state: State<'_, AppState>) -> Result<MediaTools> {
    if let Some(file) = rfd::AsyncFileDialog::new()
        .set_title("Choose ffmpeg from your trusted installation (with ffprobe alongside)")
        .pick_file()
        .await
    {
        let path = media::validate_tool(file.path())?;
        return state.select_media(MediaSelection::manual(path));
    }
    state.media_tools()
}

#[tauri::command]
fn use_included_media_tool(state: State<'_, AppState>) -> Result<MediaTools> {
    if media::included_tool(&state.media_resource_dir).is_none() {
        return Err("This build has no included FFmpeg decoder.".into());
    }
    state.select_media(MediaSelection::included())
}

pub fn run() {
    tauri::Builder::default()
        .register_uri_scheme_protocol("cachella-media", |context, request| {
            let result = if let Some(state) = context.app_handle().try_state::<AppState>() {
                match state.store() {
                    Ok(store) => crate::asset::serve(
                        &store,
                        request.uri().path(),
                        request.headers().get("range").and_then(|v| v.to_str().ok()),
                        request.method().as_str(),
                    ),
                    Err(_) => crate::asset::AssetResponse {
                        status: 503,
                        headers: vec![],
                        body: vec![],
                    },
                }
            } else {
                crate::asset::AssetResponse {
                    status: 503,
                    headers: vec![],
                    body: vec![],
                }
            };
            let mut response = tauri::http::Response::builder().status(result.status);
            for (key, value) in result.headers {
                response = response.header(key, value);
            }
            response
                .body(result.body)
                .expect("validated local media response")
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            crate::app_data::prepare(&dir)?;
            crate::scratch::initialize().map_err(std::io::Error::other)?;
            crate::settings::recover(&dir).map_err(std::io::Error::other)?;
            let database = dir.join("workspace.sqlite");
            Store::open(&database).map_err(std::io::Error::other)?;
            let settings = dir.join("endpoint.json");
            let saved = std::fs::read(&settings)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<String>(&bytes).ok())
                .and_then(|v| inference::endpoint(&v).ok())
                .unwrap_or_else(|| "http://127.0.0.1:8080".into());
            let media_resource_dir = app.path().resource_dir()?;
            let bundled_engine = Some(media_resource_dir.join("engine").join(if cfg!(windows) {
                "llama-server.exe"
            } else {
                "llama-server"
            }))
            .filter(|path| path.is_file());
            let runtime_settings = dir.join("runtime.json");
            let config = std::fs::read(&runtime_settings)
                .ok()
                .and_then(|data| serde_json::from_slice::<RuntimeConfig>(&data).ok())
                .unwrap_or_default();
            let config = crate::runtime::resolve_bundled_config(config, bundled_engine.clone());
            let embedding_settings = dir.join("embedding-runtime.json");
            let embedding_config = std::fs::read(&embedding_settings)
                .ok()
                .and_then(|data| serde_json::from_slice::<RuntimeConfig>(&data).ok())
                .unwrap_or_default();
            let embedding_config =
                crate::runtime::resolve_bundled_config(embedding_config, bundled_engine.clone());
            let media_settings = dir.join("media-tools.json");
            let media_selection = load_media_selection(&media_settings);
            let models = models::ModelInstaller::new(dir.join("models"));
            let recovered = models
                .recover_interrupted()
                .map_err(std::io::Error::other)?;
            if recovered.removed_files > 0 {
                eprintln!(
                    "Recovered {} interrupted model downloads ({} bytes).",
                    recovered.removed_files, recovered.removed_bytes
                );
            }
            app.manage(AppState {
                closing: AtomicBool::new(false),
                quit_waiter: AtomicBool::new(false),
                models,
                bundled_engine,
                media_selection: Mutex::new(media_selection),
                media_settings,
                media_resource_dir,
                embedding_runtime: Mutex::new(Runtime::new(embedding_config)),
                embedding_settings,
                embedding_profile: Mutex::new(None),
                memory_policy: Mutex::new(load_memory_policy(&dir.join("memory-policy.json"))),
                memory_settings: dir.join("memory-policy.json"),
                resume_generation: AtomicBool::new(false),
                resume_embedding: AtomicBool::new(false),
                runtime: Mutex::new(Runtime::new(config)),
                runtime_settings,
                database,
                settings,
                endpoint: Mutex::new(saved),
                generation: Arc::new(Semaphore::new(1)),
                operations: crate::operation_jobs::OperationJobs::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            workspace,
            import_documents,
            add_sample_documents,
            search_library,
            read_document,
            read_pdf_pages,
            delete_document,
            save_endpoint,
            engine_status,
            begin_operation,
            cancel_operation,
            ask_library,
            managed_runtime,
            choose_runtime_file,
            start_runtime,
            stop_runtime,
            embedding_progress,
            memory_policy,
            set_memory_policy,
            index_library,
            media_tools,
            choose_media_tool,
            use_included_media_tool,
            bundled_engine,
            use_bundled_engine,
            model_catalog,
            model_install_status,
            install_model,
            cancel_model_install
        ])
        .build(tauri::generate_context!())
        .expect("Could not launch Cachella")
        .run(|app, event| {
            if matches!(
                event,
                tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
            ) {
                let state = app.state::<AppState>();
                state.closing.store(true, Ordering::Release);
                state.operations.shutdown();
                let downloading = state.models.begin_shutdown().unwrap_or(true);
                media::stop_all();
                if let Ok(mut runtime) = state.runtime() {
                    let _ = runtime.stop();
                }
                if let Ok(mut runtime) = state.runtime_for(true) {
                    let _ = runtime.stop();
                }
                if let tauri::RunEvent::ExitRequested { api, .. } = event {
                    if downloading {
                        api.prevent_exit();
                        if !state.quit_waiter.swap(true, Ordering::AcqRel) {
                            let installer = state.models.clone();
                            let handle = app.clone();
                            tauri::async_runtime::spawn(async move {
                                installer.wait_idle().await;
                                handle.exit(0);
                            });
                        }
                    }
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(dir: &std::path::Path) -> AppState {
        AppState {
            closing: AtomicBool::new(false),
            quit_waiter: AtomicBool::new(false),
            models: models::ModelInstaller::new(dir.join("models")),
            bundled_engine: None,
            media_selection: Mutex::new(MediaSelection::included()),
            media_settings: dir.join("media-tools.json"),
            media_resource_dir: dir.join("resources"),
            embedding_runtime: Mutex::new(Runtime::default()),
            embedding_settings: dir.join("embedding-runtime.json"),
            embedding_profile: Mutex::new(None),
            memory_policy: Mutex::new(MemoryPolicy::Both),
            memory_settings: dir.join("memory-policy.json"),
            resume_generation: AtomicBool::new(false),
            resume_embedding: AtomicBool::new(false),
            runtime: Mutex::new(Runtime::default()),
            runtime_settings: dir.join("runtime.json"),
            database: dir.join("workspace.sqlite"),
            settings: dir.join("endpoint.json"),
            endpoint: Mutex::new("http://127.0.0.1:8080".into()),
            generation: Arc::new(Semaphore::new(1)),
            operations: crate::operation_jobs::OperationJobs::default(),
        }
    }

    fn decoder_fixture(resource_dir: &std::path::Path) -> PathBuf {
        let decoder = resource_dir.join("decoder");
        std::fs::create_dir_all(&decoder).unwrap();
        let ffmpeg = decoder.join(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        });
        let ffprobe = decoder.join(if cfg!(windows) {
            "ffprobe.exe"
        } else {
            "ffprobe"
        });
        std::fs::write(&ffmpeg, b"fixture").unwrap();
        std::fs::write(&ffprobe, b"fixture").unwrap();
        ffmpeg
    }

    fn holding_runtime_fixture(dir: &std::path::Path) -> RuntimeConfig {
        std::fs::create_dir_all(dir).unwrap();
        let executable = dir.join(format!("llama-server{}", std::env::consts::EXE_SUFFIX));
        let fixture = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join(format!(
                "process_guard_fixture{}",
                std::env::consts::EXE_SUFFIX
            ));
        std::fs::copy(fixture, &executable).unwrap();
        let model = dir.join("hold.gguf");
        std::fs::write(&model, b"GGUFfixture").unwrap();
        RuntimeConfig {
            executable: Some(executable),
            bundled_engine: false,
            model: Some(model.clone()),
            projector: Some(model),
        }
    }

    #[test]
    fn stopped_embedding_request_names_the_embedding_model() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        state
            .runtime_for(true)
            .unwrap()
            .configure(holding_runtime_fixture(&dir.path().join("embedding")))
            .unwrap();
        state.start_owned(true).unwrap();
        state.stop_managed(true).unwrap();
        assert!(matches!(state.embedding_connection(), Err(message)
            if message == "Start the embedding model in Local runtime first."));
    }

    #[test]
    fn memory_policy_persists_only_after_idle_successful_publication() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = test_state(dir.path());
        assert_eq!(
            load_memory_policy(&state.memory_settings),
            MemoryPolicy::Both
        );
        let permit = state.generation.try_acquire().unwrap();
        assert!(state.set_policy(MemoryPolicy::Single).is_err());
        drop(permit);
        let saved_path = state.memory_settings.clone();
        state.memory_settings = dir.path().join("missing").join("memory.json");
        state.resume_generation.store(true, Ordering::Release);
        assert!(state.set_policy(MemoryPolicy::Single).is_err());
        assert_eq!(state.policy().unwrap(), MemoryPolicy::Both);
        assert!(state.resume_generation.load(Ordering::Acquire));
        state.memory_settings = saved_path;
        assert_eq!(
            state.set_policy(MemoryPolicy::Single).unwrap(),
            MemoryPolicy::Single
        );
        assert_eq!(
            load_memory_policy(&state.memory_settings),
            MemoryPolicy::Single
        );
        assert!(!state.resume_generation.load(Ordering::Acquire));
        state
            .runtime()
            .unwrap()
            .configure(holding_runtime_fixture(&dir.path().join("model")))
            .unwrap();
        state.start_owned(false).unwrap();
        assert!(state.set_policy(MemoryPolicy::Both).is_err());
        assert_eq!(
            load_memory_policy(&state.memory_settings),
            MemoryPolicy::Single
        );
        state.stop_managed(false).unwrap();
        state.closing.store(true, Ordering::Release);
        assert!(state.set_policy(MemoryPolicy::Both).is_err());
    }

    #[tokio::test]
    async fn explicit_stop_and_external_selection_disable_automatic_restart() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        state.set_policy(MemoryPolicy::Single).unwrap();
        for embedding in [false, true] {
            state.resume_flag(embedding).store(true, Ordering::Release);
            assert!(state.runtime_status(embedding).unwrap().auto_resume);
            let stopped = state.stop_managed(embedding).unwrap();
            assert!(!stopped.auto_resume);
            // No files configured: an unwanted automatic start would fail here.
            let _permit = state.generation.try_acquire().unwrap();
            state.ensure_resident(embedding).await.unwrap();
        }
        state.resume_generation.store(true, Ordering::Release);
        persist_endpoint(&state, "http://127.0.0.1:8080").unwrap();
        assert!(!state.runtime_status(false).unwrap().auto_resume);
        assert!(state.connection().unwrap().1.is_none());
    }

    #[tokio::test]
    async fn cancelled_automatic_switch_stops_new_child_and_allows_retry() {
        for embedding in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let state = Arc::new(test_state(dir.path()));
            state.set_policy(MemoryPolicy::Single).unwrap();
            let target = dir.path().join("target");
            state
                .runtime_for(embedding)
                .unwrap()
                .configure(holding_runtime_fixture(&target))
                .unwrap();
            state
                .runtime_for(!embedding)
                .unwrap()
                .configure(holding_runtime_fixture(&dir.path().join("other")))
                .unwrap();
            state.start_owned(!embedding).unwrap();
            state.resume_flag(embedding).store(true, Ordering::Release);
            let id = state.operations.begin().unwrap();
            let task_state = state.clone();
            let task_id = id.clone();
            let task = tokio::spawn(async move {
                task_state
                    .operations
                    .run(
                        &task_id,
                        task_state.generation.clone(),
                        task_state.ensure_resident(embedding),
                    )
                    .await
            });
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while !target.join("runtime-ready").exists() {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            assert!(!state.runtime_status(!embedding).unwrap().running);
            assert!(state.operations.cancel(&id).unwrap());
            assert_eq!(
                task.await.unwrap().unwrap_err(),
                crate::operation_jobs::CANCELLED
            );
            assert!(!state.runtime_status(embedding).unwrap().running);
            assert!(state.runtime_status(embedding).unwrap().auto_resume);
            assert!(state.generation.try_acquire().is_ok());
            let retry = state.operations.begin().unwrap();
            state
                .operations
                .run(&retry, state.generation.clone(), async { Ok(()) })
                .await
                .unwrap();
        }
    }

    #[tokio::test]
    async fn failed_switch_keeps_resume_intent_without_two_resident_models() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        state.set_policy(MemoryPolicy::Single).unwrap();
        state
            .runtime_for(true)
            .unwrap()
            .configure(holding_runtime_fixture(&dir.path().join("embedding")))
            .unwrap();
        state.start_owned(true).unwrap();
        state.resume_generation.store(true, Ordering::Release);
        let id = state.operations.begin().unwrap();
        let error = state
            .operations
            .run(&id, state.generation.clone(), state.ensure_resident(false))
            .await
            .unwrap_err();
        assert!(error.contains("Select your llama-server"));
        assert!(!state.runtime_status(false).unwrap().running);
        assert!(!state.runtime_status(true).unwrap().running);
        assert!(state.runtime_status(false).unwrap().auto_resume);
        assert!(state.generation.try_acquire().is_ok());
    }

    #[test]
    fn single_residency_stops_previous_owned_engine_before_next_start() {
        for embedding in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let state = test_state(dir.path());
            *state.memory_policy.lock().unwrap() = MemoryPolicy::Single;
            for role in [false, true] {
                state
                    .runtime_for(role)
                    .unwrap()
                    .configure(holding_runtime_fixture(&dir.path().join(if role {
                        "embedding"
                    } else {
                        "generation"
                    })))
                    .unwrap();
            }
            state.start_owned(!embedding).unwrap();
            state.start_owned(embedding).unwrap();
            assert!(
                !state
                    .runtime_for(!embedding)
                    .unwrap()
                    .status()
                    .unwrap()
                    .running,
                "one-model mode kept the previous engine resident"
            );
            assert!(
                state
                    .runtime_for(embedding)
                    .unwrap()
                    .status()
                    .unwrap()
                    .running
            );
        }
    }

    #[tokio::test]
    async fn cancelled_startup_stops_only_its_owned_runtime_and_releases_permit() {
        for embedding in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let state = Arc::new(test_state(dir.path()));
            let target_dir = dir.path().join("target");
            state
                .runtime_for(embedding)
                .unwrap()
                .configure(holding_runtime_fixture(&target_dir))
                .unwrap();
            state
                .runtime_for(!embedding)
                .unwrap()
                .configure(holding_runtime_fixture(&dir.path().join("other")))
                .unwrap();
            let other = state.start_owned(!embedding).unwrap();
            let task_state = state.clone();
            let task =
                tokio::spawn(async move { start_runtime_inner(embedding, &task_state).await });
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while !target_dir.join("runtime-ready").exists() {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            let endpoint = state
                .runtime_for(embedding)
                .unwrap()
                .status()
                .unwrap()
                .endpoint
                .unwrap();
            task.abort();
            assert!(matches!(task.await, Err(error) if error.is_cancelled()));
            assert!(
                !state
                    .runtime_for(embedding)
                    .unwrap()
                    .status()
                    .unwrap()
                    .running,
                "cancelled startup left its model running"
            );
            let port = reqwest::Url::parse(&endpoint).unwrap().port().unwrap();
            assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_err());
            let untouched = state.runtime_for(!embedding).unwrap().status().unwrap();
            assert!(untouched.running);
            assert_eq!(untouched.endpoint.as_deref(), Some(other.endpoint.as_str()));
            assert!(state.generation.try_acquire().is_ok());
            state.runtime_for(!embedding).unwrap().stop().unwrap();
        }
    }

    #[tokio::test]
    async fn shutdown_during_startup_releases_permit_without_publishing_readiness() {
        for embedding in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let state = Arc::new(test_state(dir.path()));
            let target = dir.path().join("target");
            state
                .runtime_for(embedding)
                .unwrap()
                .configure(holding_runtime_fixture(&target))
                .unwrap();
            let task_state = state.clone();
            let task =
                tokio::spawn(async move { start_runtime_inner(embedding, &task_state).await });
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                while !target.join("runtime-ready").exists() {
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            let endpoint = state
                .runtime_for(embedding)
                .unwrap()
                .status()
                .unwrap()
                .endpoint
                .unwrap();
            // Match the exit callback's admission barrier and owned-runtime shutdown.
            state.closing.store(true, Ordering::Release);
            state.operations.shutdown();
            state.runtime().unwrap().stop().unwrap();
            state.runtime_for(true).unwrap().stop().unwrap();
            let result = tokio::time::timeout(std::time::Duration::from_secs(10), task)
                .await
                .unwrap()
                .unwrap();
            assert!(matches!(result, Err(message) if message == "Cachella is closing."));
            let status = state.runtime_for(embedding).unwrap().status().unwrap();
            assert!(!status.running && !status.ready);
            assert!(state.embedding_profile.lock().unwrap().is_none());
            let port = reqwest::Url::parse(&endpoint).unwrap().port().unwrap();
            assert!(std::net::TcpStream::connect(("127.0.0.1", port)).is_err());
            assert!(state.generation.try_acquire().is_ok());
        }
    }

    #[test]
    fn startup_cleanup_preserves_committed_or_different_connection() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        state
            .runtime()
            .unwrap()
            .configure(holding_runtime_fixture(&dir.path().join("runtime")))
            .unwrap();
        let connection = state.start_owned(false).unwrap();
        let mut stale = connection.clone();
        stale.token = "a different startup at the same endpoint".into();
        drop(StartupCleanup {
            state: &state,
            embedding: false,
            connection: stale,
            committed: false,
        });
        assert!(state.runtime().unwrap().status().unwrap().running);
        state.runtime().unwrap().mark_ready().unwrap();
        drop(StartupCleanup {
            state: &state,
            embedding: false,
            connection: connection.clone(),
            committed: true,
        });
        assert!(state.runtime().unwrap().status().unwrap().ready);
        drop(StartupCleanup {
            state: &state,
            embedding: false,
            connection,
            committed: false,
        });
        assert!(!state.runtime().unwrap().status().unwrap().running);
    }

    #[test]
    fn startup_cleanup_recovers_poisoned_mutex_to_stop_its_child() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(test_state(dir.path()));
        state
            .runtime()
            .unwrap()
            .configure(holding_runtime_fixture(&dir.path().join("runtime")))
            .unwrap();
        let connection = state.start_owned(false).unwrap();
        let poisoned = state.clone();
        assert!(std::thread::spawn(move || {
            let _lock = poisoned.runtime.lock().unwrap();
            panic!("deliberate fixture mutex poison");
        })
        .join()
        .is_err());
        drop(StartupCleanup {
            state: &state,
            embedding: false,
            connection,
            committed: false,
        });
        let mut runtime = state
            .runtime
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert!(!runtime.status().unwrap().running);
    }

    #[test]
    fn media_selection_reads_legacy_manual_path_and_resolves_included_after_move() {
        let first = tempfile::tempdir().unwrap();
        let legacy = first.path().join("manual-ffmpeg");
        std::fs::write(&legacy, b"legacy").unwrap();
        let settings = first.path().join("media-tools.json");
        crate::settings::save_json(&settings, &legacy).unwrap();
        let loaded = load_media_selection(&settings);
        assert_eq!(loaded.source, MediaSource::Manual);
        assert_eq!(loaded.executable, Some(legacy));

        crate::settings::save_json(&settings, &PersistedMediaSelection::Included).unwrap();
        assert_eq!(
            load_media_selection(&settings).source,
            MediaSource::Included
        );
        let second = tempfile::tempdir().unwrap();
        let first_tool = decoder_fixture(first.path());
        let second_tool = decoder_fixture(second.path());
        assert_ne!(first_tool, second_tool);
        assert_eq!(
            media::included_tool(second.path()),
            Some(std::fs::canonicalize(second_tool).unwrap())
        );
    }

    #[test]
    fn failed_media_save_keeps_the_prior_selection() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = test_state(dir.path());
        let prior = dir.path().join("manual-ffmpeg");
        *state.media_selection.lock().unwrap() = MediaSelection::manual(prior.clone());
        state.media_settings = dir.path().join("settings-directory");
        std::fs::create_dir(&state.media_settings).unwrap();
        assert!(state.select_media(MediaSelection::included()).is_err());
        let current = state.media_selection.lock().unwrap().clone();
        assert_eq!(current.source, MediaSource::Manual);
        assert_eq!(current.executable, Some(prior));
    }

    #[test]
    fn included_media_status_and_selection_follow_current_resource_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = test_state(dir.path());
        let decoder = decoder_fixture(dir.path());
        state.media_resource_dir = dir.path().to_path_buf();
        let tools = state.media_tools().unwrap();
        assert_eq!(tools.source, MediaSource::Included);
        assert!(tools.available && tools.included_available);
        assert_eq!(
            tools.executable,
            Some(std::fs::canonicalize(decoder).unwrap())
        );
        state.select_media(MediaSelection::included()).unwrap();
        let persisted: PersistedMediaSelection =
            serde_json::from_slice(&std::fs::read(&state.media_settings).unwrap()).unwrap();
        assert!(matches!(persisted, PersistedMediaSelection::Included));
    }

    #[test]
    fn manual_generation_model_change_clears_old_projector_after_persistence() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let model = dir.path().join("new.gguf");
        std::fs::write(&model, b"GGUFfixture").unwrap();
        let original = RuntimeConfig {
            model: Some(dir.path().join("old.gguf")),
            projector: Some(dir.path().join("old-projector.gguf")),
            ..RuntimeConfig::default()
        };
        state
            .runtime()
            .unwrap()
            .configure(original.clone())
            .unwrap();
        std::fs::create_dir(&state.runtime_settings).unwrap();
        assert!(state
            .select_runtime_path(false, "model", model.clone(), false)
            .is_err());
        assert!(state.runtime().unwrap().config == original);
        std::fs::remove_dir(&state.runtime_settings).unwrap();
        let selected = state
            .select_runtime_path(false, "model", model, false)
            .unwrap();
        assert!(selected.config.projector.is_none());
        let saved: RuntimeConfig =
            serde_json::from_slice(&std::fs::read(&state.runtime_settings).unwrap()).unwrap();
        assert!(saved == selected.config);
    }

    #[test]
    fn failed_endpoint_save_keeps_managed_mode_and_committed_selection() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let current = std::env::current_exe().unwrap();
        let fixture = current
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join(format!(
                "process_guard_fixture{}",
                std::env::consts::EXE_SUFFIX
            ));
        let executable = dir
            .path()
            .join(format!("llama-server{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(fixture, &executable).unwrap();
        let model = dir.path().join("model.gguf");
        std::fs::write(&model, b"GGUFfixture").unwrap();
        {
            let mut runtime = state.runtime().unwrap();
            runtime
                .configure(RuntimeConfig {
                    executable: Some(executable),
                    model: Some(model),
                    ..RuntimeConfig::default()
                })
                .unwrap();
            runtime.start().unwrap();
            runtime.stop().unwrap();
            assert!(runtime.status().unwrap().managed);
        }
        std::fs::create_dir(&state.settings).unwrap();
        assert!(persist_endpoint(&state, "http://127.0.0.1:9090").is_err());
        assert_eq!(state.endpoint().unwrap(), "http://127.0.0.1:8080");
        assert!(state.runtime().unwrap().status().unwrap().managed);
        std::fs::remove_dir(&state.settings).unwrap();
        assert_eq!(
            persist_endpoint(&state, "http://127.0.0.1:9090").unwrap(),
            "http://127.0.0.1:9090"
        );
        assert!(!state.runtime().unwrap().status().unwrap().managed);
        assert_eq!(state.endpoint().unwrap(), "http://127.0.0.1:9090");
        let saved: String =
            serde_json::from_slice(&std::fs::read(&state.settings).unwrap()).unwrap();
        assert_eq!(saved, state.endpoint().unwrap());
    }

    #[test]
    fn bundled_selection_preserves_models_and_guards_settings() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = test_state(dir.path());
        assert!(state.select_bundled_engine(false).is_err());
        let engine = dir.path().join("llama-server");
        std::fs::write(&engine, b"fixture only, never executed").unwrap();
        state.bundled_engine = Some(engine.clone());
        let model = dir.path().join("model.gguf");
        let projector = dir.path().join("projector.gguf");
        let config = RuntimeConfig {
            executable: Some(dir.path().join("manual")),
            bundled_engine: false,
            model: Some(model.clone()),
            projector: Some(projector.clone()),
        };
        state
            .runtime_for(true)
            .unwrap()
            .configure(config.clone())
            .unwrap();
        *state.embedding_profile.lock().unwrap() = Some("old-profile".into());
        let permit = state.generation.try_acquire().unwrap();
        assert!(state.select_bundled_engine(true).is_err());
        drop(permit);
        let selected = state.select_bundled_engine(true).unwrap();
        assert_eq!(selected.config.executable, Some(engine));
        assert!(selected.config.bundled_engine);
        assert_eq!(selected.config.model, Some(model));
        assert_eq!(selected.config.projector, Some(projector));
        assert!(!selected.running);
        assert!(state.embedding_profile.lock().unwrap().is_none());
        let saved: RuntimeConfig =
            serde_json::from_slice(&std::fs::read(&state.embedding_settings).unwrap()).unwrap();
        assert!(saved == selected.config);
        assert!(state.runtime().unwrap().config == RuntimeConfig::default());
        let manual = dir.path().join("manual-engine");
        let selected = state
            .select_runtime_path(true, "executable", manual.clone(), false)
            .unwrap();
        assert!(!selected.config.bundled_engine);
        assert_eq!(selected.config.executable, Some(manual));
        state.closing.store(true, Ordering::Release);
        assert!(state.select_bundled_engine(true).is_err());
        assert!(state.runtime_for(true).unwrap().config == selected.config);
        state.closing.store(false, Ordering::Release);
        std::fs::remove_file(&state.embedding_settings).unwrap();
        std::fs::create_dir(&state.embedding_settings).unwrap();
        assert!(state.select_bundled_engine(true).is_err());
        assert!(state.runtime_for(true).unwrap().config == selected.config);
    }

    #[test]
    fn deletion_is_blocked_while_generation_uses_evidence() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let document = state
            .store()
            .unwrap()
            .import_text("Evidence", "txt", "Launch Friday")
            .unwrap();
        let permit = state.generation.try_acquire().unwrap();
        assert!(state.delete(&document.id).is_err());
        assert!(state.store().unwrap().document_text(&document.id).is_ok());
        drop(permit);
        state.delete(&document.id).unwrap();
        assert!(state.store().unwrap().document_text(&document.id).is_err());
    }

    #[test]
    fn installed_model_selection_preserves_engine_and_newer_user_choices() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        let expected = RuntimeConfig {
            executable: Some(dir.path().join("llama-server")),
            bundled_engine: false,
            model: Some(dir.path().join("user-model.gguf")),
            projector: None,
        };
        state
            .runtime()
            .unwrap()
            .configure(expected.clone())
            .unwrap();
        let installed = models::InstalledModels {
            model: dir.path().join("verified-q4.gguf"),
            projector: None,
            embedding: false,
        };
        let permit = state.generation.try_acquire().unwrap();
        assert!(state
            .select_installed_models(installed.clone(), &expected)
            .is_err());
        assert!(state.runtime().unwrap().config == expected);
        drop(permit);
        let selected = state
            .select_installed_models(installed.clone(), &expected)
            .unwrap();
        assert_eq!(selected.config.executable, expected.executable);
        assert_eq!(selected.config.model, Some(installed.model));
        assert!(!selected.running && !selected.ready);
        let persisted: RuntimeConfig =
            serde_json::from_slice(&std::fs::read(&state.runtime_settings).unwrap()).unwrap();
        assert!(persisted == selected.config);
        // A download begun before this selection cannot overwrite the newer choice.
        assert!(state
            .select_installed_models(
                models::InstalledModels {
                    model: dir.path().join("older-download.gguf"),
                    projector: None,
                    embedding: false,
                },
                &expected
            )
            .is_err());
        assert!(state.runtime().unwrap().config == persisted);
    }

    #[test]
    fn installed_embedding_selection_clears_profile_and_failed_save_preserves_config() {
        let dir = tempfile::tempdir().unwrap();
        let mut state = test_state(dir.path());
        *state.embedding_profile.lock().unwrap() = Some("old-model-profile".into());
        let expected = RuntimeConfig::default();
        let installed = models::InstalledModels {
            model: dir.path().join("embedding.gguf"),
            projector: Some(dir.path().join("projector.gguf")),
            embedding: true,
        };
        state.embedding_settings = dir.path().join("missing").join("settings.json");
        assert!(state
            .select_installed_models(installed.clone(), &expected)
            .is_err());
        assert!(state.runtime_for(true).unwrap().config == expected);
        assert_eq!(
            state.embedding_profile.lock().unwrap().as_deref(),
            Some("old-model-profile")
        );
        state.embedding_settings = dir.path().join("embedding-runtime.json");
        let selected = state.select_installed_models(installed, &expected).unwrap();
        assert!(selected.config.model.is_some() && selected.config.projector.is_some());
        assert!(state.embedding_profile.lock().unwrap().is_none());
        assert!(state.runtime().unwrap().config == RuntimeConfig::default());
    }

    #[test]
    fn shutdown_rejects_late_startup_and_download_selection() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path());
        state.closing.store(true, Ordering::Release);
        for embedding in [false, true] {
            assert!(
                matches!(state.start_owned(embedding), Err(message) if message == "Cachella is closing.")
            );
            assert!(state
                .select_installed_models(
                    models::InstalledModels {
                        model: dir.path().join("download.gguf"),
                        projector: None,
                        embedding,
                    },
                    &RuntimeConfig::default()
                )
                .is_err());
            assert!(state.runtime_for(embedding).unwrap().config == RuntimeConfig::default());
        }
        assert!(!state.runtime_settings.exists() && !state.embedding_settings.exists());
    }
}
