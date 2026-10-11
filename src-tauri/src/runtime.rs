use crate::store::Result;
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub executable: Option<PathBuf>,
    #[serde(default)]
    pub bundled_engine: bool,
    pub model: Option<PathBuf>,
    #[serde(default)]
    pub projector: Option<PathBuf>,
}

/// Resolve a bundled choice against this launch's resource directory.
pub fn resolve_bundled_config(mut config: RuntimeConfig, engine: Option<PathBuf>) -> RuntimeConfig {
    if config.bundled_engine || (config.executable.is_none() && engine.is_some()) {
        config.executable = engine;
        config.bundled_engine = true;
    }
    config
}

#[derive(Serialize)]
pub struct RuntimeStatus {
    pub config: RuntimeConfig,
    pub running: bool,
    pub ready: bool,
    pub managed: bool,
    pub auto_resume: bool,
    pub endpoint: Option<String>,
}

#[derive(Clone)]
pub struct Connection {
    pub endpoint: String,
    pub token: String,
    pub certificate: String,
}

#[derive(Default)]
pub struct Runtime {
    pub config: RuntimeConfig,
    child: Option<crate::process_guard::GuardedChild>,
    connection: Option<Connection>,
    managed: bool,
    ready: bool,
    tls_files: Option<crate::scratch::Workspace>,
}

pub fn validate_model(path: &Path) -> Result<PathBuf> {
    let path = path
        .canonicalize()
        .map_err(|_| "Cannot open the selected model.".to_string())?;
    let mut file = crate::regular_file::open_regular_file(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::InvalidInput {
            "Select a GGUF model file.".to_string()
        } else {
            "Cannot open the selected model.".to_string()
        }
    })?;
    let mut header = [0; 4];
    file.read_exact(&mut header)
        .map_err(|_| "The model file is incomplete.".to_string())?;
    if &header != b"GGUF" {
        return Err("The selected file is not a GGUF model.".into());
    }
    Ok(path)
}

impl Runtime {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            config,
            child: None,
            connection: None,
            managed: false,
            ready: false,
            tls_files: None,
        }
    }
    pub fn status(&mut self) -> Result<RuntimeStatus> {
        if let Some(child) = self.child.as_mut() {
            if child.try_wait().map_err(|e| e.to_string())?.is_some() {
                self.ready = false;
                self.child = None;
                self.connection = None;
                self.tls_files = None;
            }
        }
        Ok(RuntimeStatus {
            config: self.config.clone(),
            running: self.child.is_some(),
            ready: self.ready,
            managed: self.managed,
            auto_resume: false,
            endpoint: self.connection.as_ref().map(|c| c.endpoint.clone()),
        })
    }
    pub fn mark_ready(&mut self) -> Result<()> {
        if !self.status()?.running {
            return Err("Engine exited during startup.".into());
        }
        self.ready = true;
        Ok(())
    }
    pub fn connection(&mut self) -> Result<Option<Connection>> {
        let status = self.status()?;
        if status.managed && !status.ready {
            return Err("Start your selected local model before generating an answer.".into());
        }
        Ok(self.connection.clone())
    }
    pub fn use_external(&mut self) -> Result<()> {
        if self.status()?.running {
            return Err("Stop the managed model before switching connections.".into());
        }
        self.managed = false;
        Ok(())
    }
    pub fn configure(&mut self, config: RuntimeConfig) -> Result<()> {
        if self.status()?.running {
            return Err("Stop the model before changing its files.".into());
        }
        self.config = config;
        Ok(())
    }
    pub fn start(&mut self) -> Result<Connection> {
        self.start_mode(false)
    }
    pub fn start_embedding(&mut self) -> Result<Connection> {
        self.start_mode(true)
    }
    fn start_mode(&mut self, embedding: bool) -> Result<Connection> {
        if self.status()?.running {
            return Err("The managed model is already running.".into());
        }
        let executable = self
            .config
            .executable
            .as_ref()
            .ok_or("Select your llama-server executable first.")?;
        let executable = executable
            .canonicalize()
            .map_err(|_| "The selected executable is unavailable.".to_string())?;
        if !executable.is_file()
            || !matches!(
                executable.file_name().and_then(|n| n.to_str()),
                Some("llama-server" | "llama-server.exe")
            )
        {
            return Err(
                "Select the llama-server executable from your installed llama.cpp distribution."
                    .into(),
            );
        }
        let model = validate_model(
            self.config
                .model
                .as_ref()
                .ok_or("Select a GGUF model first.")?,
        )?;
        let projector = if embedding {
            Some(validate_model(
                self.config
                    .projector
                    .as_ref()
                    .ok_or("Select the EmbeddingGemma 2 projector first.")?,
            )?)
        } else {
            self.config
                .projector
                .as_deref()
                .map(validate_model)
                .transpose()?
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        let tls_files = crate::scratch::Workspace::new()?;
        let certified = rcgen::generate_simple_self_signed(vec!["127.0.0.1".into()])
            .map_err(|e| e.to_string())?;
        let certificate = certified.cert.pem();
        let cert_path = tls_files.path().join("certificate.pem");
        let key_path = tls_files.path().join("key.pem");
        std::fs::write(&cert_path, &certificate).map_err(|e| e.to_string())?;
        std::fs::write(&key_path, certified.signing_key.serialize_pem())
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| e.to_string())?;
        }
        let connection = Connection {
            endpoint: format!("https://127.0.0.1:{port}"),
            certificate,
            token: uuid::Uuid::new_v4().simple().to_string(),
        };
        let mut command = Command::new(executable);
        command.env_clear();
        // Inherit platform essentials only; LLAMA_* settings cannot enable tools,
        // external listeners, remote model downloads or prompt logging.
        for key in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "SYSTEMROOT",
            "WINDIR",
            "TEMP",
            "TMP",
            "TMPDIR",
        ] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        command
            .env("LLAMA_API_KEY", &connection.token)
            .arg("--ssl-cert-file")
            .arg(&cert_path)
            .arg("--ssl-key-file")
            .arg(&key_path)
            .arg("-m")
            .arg(model)
            .args([
                "--host",
                "127.0.0.1",
                "--port",
                &port.to_string(),
                "-c",
                if embedding { "2048" } else { "4096" },
                "-np",
                "1",
                "--no-webui",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        if let Some(projector) = projector {
            command.arg("--mmproj").arg(projector);
        }
        if embedding {
            command.args([
                "--embedding",
                "--pooling",
                "mean",
                "-b",
                "2048",
                "-ub",
                "2048",
            ]);
        }
        drop(listener);
        let child = crate::process_guard::spawn(&command, false).map_err(|_| {
            "Could not start llama-server. Check the executable and its required libraries."
                .to_string()
        })?;
        self.tls_files = Some(tls_files);
        self.child = Some(child);
        self.connection = Some(connection.clone());
        self.managed = true;
        self.ready = false;
        Ok(connection)
    }
    pub fn stop(&mut self) -> Result<()> {
        if let Some(child) = self.child.as_mut() {
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                child.kill().map_err(|e| e.to_string())?;
                child.wait().map_err(|e| e.to_string())?;
            }
        }
        self.ready = false;
        self.child = None;
        self.connection = None;
        self.tls_files = None;
        Ok(())
    }

    /// Cleanup owns a specific startup, even if a port is later reused.
    #[cfg(feature = "desktop")]
    pub(crate) fn stop_if_connection(&mut self, expected: &Connection) -> Result<()> {
        if self.connection.as_ref().is_some_and(|current| {
            current.endpoint == expected.endpoint && current.token == expected.token
        }) {
            self.stop()?;
        }
        Ok(())
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_defaults_preserve_manual_choices_and_follow_app_moves() {
        let included = PathBuf::from("new-app/resources/engine/llama-server");
        let fresh = resolve_bundled_config(RuntimeConfig::default(), Some(included.clone()));
        assert_eq!(fresh.executable, Some(included.clone()));
        assert!(fresh.bundled_engine);
        let manual = RuntimeConfig {
            executable: Some(PathBuf::from("custom/llama-server")),
            model: Some(PathBuf::from("private.gguf")),
            ..RuntimeConfig::default()
        };
        let resolved = resolve_bundled_config(manual.clone(), Some(included.clone()));
        assert!(resolved == manual);
        let mut moved = manual.clone();
        moved.bundled_engine = true;
        let resolved = resolve_bundled_config(moved.clone(), Some(included));
        assert!(resolved.bundled_engine);
        assert_eq!(resolved.model, manual.model);
        assert_ne!(resolved.executable, manual.executable);
        let unavailable = resolve_bundled_config(moved, None);
        assert!(unavailable.executable.is_none());
        assert!(unavailable.bundled_engine);
        let legacy: RuntimeConfig =
            serde_json::from_str(r#"{"executable":"custom/llama-server","model":null}"#).unwrap();
        assert!(!legacy.bundled_engine);
    }

    #[test]
    fn runtime_modes_validate_projectors_and_pass_mode_specific_flags() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir
            .path()
            .join(format!("llama-server{}", std::env::consts::EXE_SUFFIX));
        let capture = dir.path().join("args.json");
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
        std::fs::copy(fixture, &executable).unwrap();
        let model = dir.path().join("model.gguf");
        std::fs::write(&model, b"GGUFfixture").unwrap();
        let mut runtime = Runtime::new(RuntimeConfig {
            executable: Some(executable),
            bundled_engine: false,
            model: Some(model.clone()),
            projector: None,
        });
        assert!(runtime.start_embedding().is_err());
        runtime.config.projector = Some(model.clone());
        runtime.start_embedding().unwrap();
        assert!(runtime.child.as_mut().unwrap().wait().unwrap().success());
        let args = std::fs::read_to_string(&capture).unwrap();
        let args: Vec<String> = serde_json::from_str(&args).unwrap();
        assert!(args.iter().any(|arg| arg == "--embedding"));
        assert!(args.windows(2).any(|a| a == ["--pooling", "mean"]));
        assert!(args.windows(2).any(|a| a == ["-c", "2048"]));
        assert!(args.windows(2).any(|a| a == ["-ub", "2048"]));
        assert!(args
            .windows(2)
            .any(|a| a == ["--mmproj", model.canonicalize().unwrap().to_str().unwrap()]));
        std::fs::remove_file(&capture).unwrap();
        runtime.start().unwrap();
        assert!(runtime.child.as_mut().unwrap().wait().unwrap().success());
        let args = std::fs::read_to_string(&capture).unwrap();
        let args: Vec<String> = serde_json::from_str(&args).unwrap();
        assert!(!args
            .iter()
            .any(|arg| arg == "--embedding" || arg == "--pooling"));
        assert!(args.windows(2).any(|a| a == ["-c", "4096"]));
        assert!(args
            .windows(2)
            .any(|a| a == ["--mmproj", model.canonicalize().unwrap().to_str().unwrap()]));
        runtime.config.projector = Some(dir.path().join("missing.gguf"));
        assert!(runtime.start().is_err());
        assert!(!runtime.status().unwrap().running);
        runtime.config.projector = None;
        runtime.start().unwrap();
        assert!(runtime.child.as_mut().unwrap().wait().unwrap().success());
        let args: Vec<String> =
            serde_json::from_str(&std::fs::read_to_string(&capture).unwrap()).unwrap();
        assert!(!args
            .iter()
            .any(|arg| arg == "--embedding" || arg == "--mmproj"));
    }

    #[test]
    fn rejects_invalid_model_and_missing_configuration() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("fake.gguf");
        std::fs::write(&model, "ordinary text").unwrap();
        assert!(validate_model(&model).is_err());
        assert!(Runtime::default().start().is_err());
    }

    #[cfg(unix)]
    #[test]
    fn exited_child_does_not_fall_back_to_external_connection() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("llama-server");
        std::fs::write(&executable, "#!/bin/sh\nexit 17\n").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let model = dir.path().join("model.gguf");
        std::fs::write(&model, b"GGUFfixture").unwrap();
        let mut runtime = Runtime::new(RuntimeConfig {
            executable: Some(executable),
            bundled_engine: false,
            model: Some(model),
            projector: None,
        });
        runtime.start().unwrap();
        let tls_path = runtime.tls_files.as_ref().unwrap().path().to_owned();
        runtime.child.as_mut().unwrap().wait().unwrap();
        assert!(!runtime.status().unwrap().running);
        assert!(!tls_path.exists());
        assert!(runtime.connection().is_err());
        runtime.use_external().unwrap();
        assert!(runtime.connection().unwrap().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn owns_child_and_clears_credentials_on_stop() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("llama-server");
        // A fixture process only; model quality is tested separately with real inference.
        std::fs::write(&executable, "#!/bin/sh\nexec /bin/sleep 30\n").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let model = dir.path().join("model.gguf");
        std::fs::write(&model, b"GGUFfixture").unwrap();
        let mut runtime = Runtime {
            config: RuntimeConfig {
                executable: Some(executable),
                bundled_engine: false,
                model: Some(model),
                projector: None,
            },
            child: None,
            connection: None,
            managed: false,
            ready: false,
            tls_files: None,
        };
        let connection = runtime.start().unwrap();
        assert!(connection.endpoint.starts_with("https://127.0.0.1:"));
        assert!(connection.token.len() >= 32);
        assert!(runtime.connection().is_err());
        runtime.mark_ready().unwrap();
        assert!(runtime.connection().unwrap().is_some());
        let tls_path = runtime.tls_files.as_ref().unwrap().path().to_owned();
        assert!(
            tls_path.ancestors().any(|path| path
                .file_name()
                .is_some_and(|name| name == "cachella-scratch-v1")),
            "TLS files must use the recoverable scratch namespace"
        );
        let public_status = serde_json::to_string(&runtime.status().unwrap()).unwrap();
        assert!(!public_status.contains(&connection.token));
        assert!(!public_status.contains("BEGIN CERTIFICATE"));
        assert!(runtime.start().is_err());
        assert!(runtime
            .child
            .as_mut()
            .unwrap()
            .try_wait()
            .unwrap()
            .is_none());
        runtime.stop().unwrap();
        assert!(!tls_path.exists());
        assert!(runtime.child.is_none());
        assert!(runtime.connection.is_none());
        runtime.stop().unwrap();
    }
}
