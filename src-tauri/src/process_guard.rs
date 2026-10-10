//! A small owning parent for tools which must not outlive Cachella.
//!
//! `spawn` starts this executable in `--cachella-process-guard` mode.  The
//! original process keeps the guardian's stdin open as a lease.  If Cachella
//! exits abruptly, the operating system closes that pipe and the guardian
//! kills and reaps the tool it owns.  The protocol deliberately contains no
//! shell syntax, secrets in argv, or inherited environment beyond the values
//! explicitly attached to the supplied [`Command`].

use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

const GUARD_FLAG: &str = "--cachella-process-guard";
const MAX_SPEC_BYTES: usize = 64 * 1024;
const READY_TO_START: u8 = b'R';
const STARTED: u8 = b'S';
const FAILED: u8 = b'E';

#[derive(Debug, Serialize, Deserialize)]
struct SpawnSpec {
    program: WireString,
    args: Vec<WireString>,
    environment: Vec<(WireString, WireString)>,
    current_dir: Option<WireString>,
    capture: bool,
    #[serde(default)]
    scratch_lease: Option<crate::scratch::LeaseDescriptor>,
}

/// JSON-safe, lossless representation of an operating-system string.
/// UTF-8 stays readable in the private frame; platform-native strings retain
/// their original bytes or UTF-16 units when a user selects an unusual path.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "encoding", content = "value")]
enum WireString {
    Utf8(String),
    #[cfg(unix)]
    UnixBytes(Vec<u8>),
    #[cfg(windows)]
    WindowsWide(Vec<u16>),
}

impl WireString {
    fn from_os(value: &OsStr) -> Self {
        if let Some(value) = value.to_str() {
            return Self::Utf8(value.to_owned());
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            return Self::UnixBytes(value.as_bytes().to_vec());
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            return Self::WindowsWide(value.encode_wide().collect());
        }
        #[allow(unreachable_code)]
        Self::Utf8(value.to_string_lossy().into_owned())
    }

    fn into_os(self) -> OsString {
        match self {
            Self::Utf8(value) => OsString::from(value),
            #[cfg(unix)]
            Self::UnixBytes(value) => {
                use std::os::unix::ffi::OsStringExt;
                OsString::from_vec(value)
            }
            #[cfg(windows)]
            Self::WindowsWide(value) => {
                use std::os::windows::ffi::OsStringExt;
                OsString::from_wide(&value)
            }
        }
    }
}

/// The outer guardian process plus its stdin lease.
///
/// The process returned by [`spawn`] is the guardian.  Its exit status is the
/// tool status on normal completion, and a non-zero status after lease loss.
pub struct GuardedChild {
    guardian: Child,
    lease: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

impl GuardedChild {
    pub fn id(&self) -> u32 {
        self.guardian.id()
    }

    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.guardian.try_wait()
    }

    /// Requests teardown by closing the private lease.  The guardian owns the
    /// real child, so this never sends a signal directly to the tool.
    pub fn kill(&mut self) -> io::Result<()> {
        self.lease.take();
        Ok(())
    }

    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        self.guardian.wait()
    }
}

impl Drop for GuardedChild {
    fn drop(&mut self) {
        self.lease.take();
        if self.guardian.try_wait().ok().flatten().is_none() {
            let _ = self.guardian.wait();
        }
    }
}

/// Spawns `command` behind a guardian.
///
/// The command's program, arguments, current directory, and values set via
/// `Command::env` are copied into a bounded JSON message.  The guardian calls
/// `env_clear` before applying those values.  Callers must therefore use
/// `env_clear` and explicitly provide every environment value their tool
/// needs; `Command::env_remove` cannot be represented and is rejected.
///
/// `capture` overrides the supplied command stdio.  With `true`, stdout and
/// stderr are available on the returned value.  With `false`, both streams
/// are connected to null devices.  Drain captured stdout and stderr
/// concurrently before waiting when either stream can be large, matching the
/// standard child-process pipe contract.
pub fn spawn(command: &Command, capture: bool) -> io::Result<GuardedChild> {
    let spec = serialize_command(command, capture)?;
    let encoded = serde_json::to_vec(&spec)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))?;
    if encoded.len() > MAX_SPEC_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Guarded command specification exceeds 64 KiB.",
        ));
    }

    let executable = guardian_executable()?;
    let mut guardian = Command::new(executable);
    guardian
        .arg(GUARD_FLAG)
        .stdin(Stdio::piped())
        // stdout carries a single startup byte before any forwarded stdout.
        .stdout(Stdio::piped())
        .stderr(if capture {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    #[cfg(test)]
    copy_test_guard_settings(command, &mut guardian);
    hide_window(&mut guardian);
    let mut guardian = guardian.spawn()?;
    let lease = guardian.stdin.take().ok_or_else(|| {
        io::Error::new(io::ErrorKind::BrokenPipe, "Guardian stdin was unavailable.")
    })?;

    let bootstrap = guardian.stdout.take().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::BrokenPipe,
            "Guardian stdout was unavailable.",
        )
    })?;
    // The guardian cannot spawn until it sees an explicit start byte.  That
    // makes a bounded frame-write failure safe to terminate directly: there is
    // no owned child yet.
    let mut lease =
        write_frame_with_timeout(lease, &encoded, bootstrap_timeout()).inspect_err(|_| {
            let _ = guardian.kill();
            let _ = guardian.wait();
        })?;
    let bootstrap = read_bootstrap_byte(bootstrap, bootstrap_timeout()).inspect_err(|_| {
        let _ = guardian.kill();
        let _ = guardian.wait();
    })?;
    if bootstrap.0 != READY_TO_START {
        let _ = guardian.kill();
        let _ = guardian.wait();
        return Err(io::Error::other(
            "Guardian rejected its startup specification.",
        ));
    }

    // From this point the guardian may own a tool.  Every failure must release
    // the lease and let the guardian kill and reap that tool itself.
    if let Err(error) = lease.write_all(&[STARTED]).and_then(|_| lease.flush()) {
        drop(lease);
        let _ = guardian.wait();
        return Err(error);
    }
    let bootstrap = match read_bootstrap_byte(bootstrap.1, postcommit_timeout(command)) {
        Ok(bootstrap) => bootstrap,
        Err(error) => {
            drop(lease);
            let _ = guardian.wait();
            return Err(error);
        }
    };
    if bootstrap.0 != STARTED {
        drop(lease);
        let _ = guardian.wait();
        return Err(io::Error::other("Guardian could not start its child."));
    }

    let stderr = if capture {
        guardian.stderr.take()
    } else {
        None
    };
    Ok(GuardedChild {
        guardian,
        lease: Some(lease),
        stdout: capture.then_some(bootstrap.1),
        stderr,
    })
}

fn write_frame_with_timeout(
    lease: ChildStdin,
    encoded: &[u8],
    timeout: Duration,
) -> io::Result<ChildStdin> {
    let bytes = encoded.to_vec();
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut lease = lease;
        let result = write_spec(&mut lease, &bytes);
        let _ = tx.send((result, lease));
    });
    match rx.recv_timeout(timeout) {
        Ok((result, lease)) => result.map(|_| lease),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Guardian did not accept its startup frame within the configured timeout.",
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "Guardian ended while receiving its startup frame.",
        )),
    }
}

fn read_bootstrap_byte(bootstrap: ChildStdout, timeout: Duration) -> io::Result<(u8, ChildStdout)> {
    let (tx, rx) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let mut bootstrap = bootstrap;
        let mut byte = [0_u8; 1];
        let result = bootstrap.read_exact(&mut byte).map(|_| byte[0]);
        let _ = tx.send((result, bootstrap));
    });
    match rx.recv_timeout(timeout) {
        Ok((Ok(byte), bootstrap)) => Ok((byte, bootstrap)),
        Ok((Err(error), _)) => Err(io::Error::new(
            error.kind(),
            "Guardian ended before acknowledging child startup.",
        )),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "Guardian did not acknowledge child startup within the configured timeout.",
        )),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "Guardian acknowledgement reader ended unexpectedly.",
        )),
    }
}

/// Runs guardian mode when this executable was invoked by [`spawn`].
///
/// Call this before constructing the desktop application, then exit with the
/// returned code when it is `Some`.
pub fn entry() -> Option<i32> {
    std::env::args_os()
        .any(|argument| argument == OsStr::new(GUARD_FLAG))
        .then(|| guardian_main().unwrap_or(1))
}

fn serialize_command(command: &Command, capture: bool) -> io::Result<SpawnSpec> {
    let program = WireString::from_os(command.get_program());
    let args = command
        .get_args()
        .map(WireString::from_os)
        .collect::<Vec<_>>();
    let mut environment = Vec::new();
    for (key, value) in command.get_envs() {
        let value = value.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Guarded commands cannot use env_remove.",
            )
        })?;
        environment.push((WireString::from_os(key), WireString::from_os(value)));
    }
    let current_dir = command
        .get_current_dir()
        .map(|path| WireString::from_os(path.as_os_str()));
    Ok(SpawnSpec {
        program,
        args,
        environment,
        current_dir,
        capture,
        scratch_lease: crate::scratch::current_lease(),
    })
}

fn guardian_executable() -> io::Result<PathBuf> {
    #[cfg(test)]
    {
        let test_binary = std::env::current_exe()?;
        let profile = test_binary
            .parent()
            .and_then(std::path::Path::parent)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Could not find the test profile directory.",
                )
            })?;
        let fixture = profile.join("examples").join(format!(
            "process_guard_fixture{}",
            std::env::consts::EXE_SUFFIX
        ));
        if fixture.is_file() {
            return Ok(fixture);
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Process-guard test fixture is missing. Run `cargo build --example process_guard_fixture --features desktop` before cargo test.",
        ))
    }
    #[cfg(not(test))]
    {
        std::env::current_exe()
    }
}

fn bootstrap_timeout() -> Duration {
    Duration::from_secs(5)
}

fn postcommit_timeout(command: &Command) -> Duration {
    let _ = command;
    #[cfg(test)]
    for (key, value) in command.get_envs() {
        if key == OsStr::new("CACHELLA_TEST_GUARD_ACK_TIMEOUT_MS") {
            if let Some(value) = value.and_then(OsStr::to_str) {
                if let Ok(milliseconds) = value.parse::<u64>() {
                    return Duration::from_millis(milliseconds.clamp(1, 5_000));
                }
            }
        }
    }
    Duration::from_secs(5)
}

#[cfg(test)]
fn copy_test_guard_settings(source: &Command, destination: &mut Command) {
    for (key, value) in source.get_envs() {
        if matches!(
            key.to_str(),
            Some(
                "CACHELLA_TEST_GUARD_DELAY_ACK_MS"
                    | "CACHELLA_TEST_GUARD_FAIL_AFTER_SPAWN"
                    | "CACHELLA_TEST_GUARD_READY_MARKER"
            )
        ) {
            if let Some(value) = value {
                destination.env(key, value);
            }
        }
    }
}

fn write_spec(lease: &mut ChildStdin, bytes: &[u8]) -> io::Result<()> {
    let length = u32::try_from(bytes.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Guard spec is too large."))?;
    lease.write_all(&length.to_le_bytes())?;
    lease.write_all(bytes)?;
    lease.flush()
}

fn guardian_main() -> io::Result<i32> {
    let mut lease = io::stdin();
    let spec = read_spec(&mut lease)?;
    // Keep the workspace session leased until every owned child has been
    // reaped. Parent death may race the next application's recovery scan.
    // Declare this before OwnedChild so error unwinding also reaps first.
    let _scratch_lease = spec
        .scratch_lease
        .as_ref()
        .map(crate::scratch::retain_lease)
        .transpose()
        .map_err(io::Error::other)?;
    let mut command = Command::new(spec.program.into_os());
    command.env_clear();
    command.args(spec.args.into_iter().map(WireString::into_os));
    command.envs(
        spec.environment
            .into_iter()
            .map(|(key, value)| (key.into_os(), value.into_os())),
    );
    if let Some(current_dir) = spec.current_dir {
        command.current_dir(PathBuf::from(current_dir.into_os()));
    }
    command.stdin(Stdio::null());
    if spec.capture {
        command.stdout(Stdio::piped());
        // Guardian stderr is the caller's capture pipe.
        command.stderr(Stdio::inherit());
    } else {
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());
    }
    hide_window(&mut command);

    // A direct kill is safe until the parent sends this final commit byte.
    // It prevents a timed-out frame write or READY acknowledgement from ever
    // leaving a tool behind.
    io::stdout().write_all(&[READY_TO_START])?;
    io::stdout().flush()?;
    let mut start = [0_u8; 1];
    lease.read_exact(&mut start)?;
    if start[0] != STARTED {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Guardian received an invalid start byte.",
        ));
    }

    let child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            io::stdout().write_all(&[FAILED])?;
            io::stdout().flush()?;
            return Ok(1);
        }
    };
    let mut child = OwnedChild::new(child);
    test_delay_after_spawn();
    if test_fail_after_spawn() {
        return Err(io::Error::other("Injected guardian post-spawn failure."));
    }
    io::stdout().write_all(&[STARTED])?;
    io::stdout().flush()?;

    let relay = if spec.capture {
        let mut output = child
            .child_mut()
            .stdout
            .take()
            .expect("piped stdout must exist");
        let (relay_done_tx, relay_done_rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let _ = io::copy(&mut output, &mut io::stdout().lock());
            let _ = relay_done_tx.send(());
        });
        Some(relay_done_rx)
    } else {
        None
    };

    let (lease_closed_tx, lease_closed_rx) = mpsc::channel();
    thread::spawn(move || {
        let mut byte = [0_u8; 1024];
        while lease.read(&mut byte).unwrap_or(0) != 0 {}
        let _ = lease_closed_tx.send(());
    });

    let (status, natural_exit) = loop {
        if let Some(status) = child.child_mut().try_wait()? {
            child.disarm();
            break (status, true);
        }
        match lease_closed_rx.recv_timeout(Duration::from_millis(20)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                break (child.kill_and_wait()?, false);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    };
    // A lease loss must exit promptly even if a caller abandoned the captured
    // output pipe. Natural completion forwards every remaining byte for either
    // success or failure until the caller closes its lease.
    if natural_exit {
        if let Some(relay_done) = relay {
            loop {
                match relay_done.recv_timeout(Duration::from_millis(20)) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        if lease_closed_rx.try_recv().is_ok() {
                            break;
                        }
                    }
                }
            }
        }
    }
    Ok(status.code().unwrap_or(1))
}

struct OwnedChild {
    child: Option<Child>,
}

impl OwnedChild {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    fn child_mut(&mut self) -> &mut Child {
        self.child.as_mut().expect("owned child must exist")
    }

    fn disarm(&mut self) {
        self.child.take();
    }

    fn kill_and_wait(&mut self) -> io::Result<ExitStatus> {
        let child = self.child_mut();
        let _ = child.kill();
        let status = child.wait()?;
        self.disarm();
        Ok(status)
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

fn test_delay_after_spawn() {
    if fixture_faults_enabled() && wait_for_test_child_ready() {
        if let Some(milliseconds) = std::env::var_os("CACHELLA_TEST_GUARD_DELAY_ACK_MS")
            .and_then(|value| value.to_str().and_then(|value| value.parse::<u64>().ok()))
        {
            thread::sleep(Duration::from_millis(milliseconds.min(5_000)));
        }
    }
}

fn test_fail_after_spawn() -> bool {
    fixture_faults_enabled()
        && wait_for_test_child_ready()
        && std::env::var_os("CACHELLA_TEST_GUARD_FAIL_AFTER_SPAWN").is_some()
}

fn fixture_faults_enabled() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|name| name == OsStr::new("process_guard_fixture"))
        })
        .unwrap_or(false)
}

fn wait_for_test_child_ready() -> bool {
    let Some(marker) = std::env::var_os("CACHELLA_TEST_GUARD_READY_MARKER") else {
        return false;
    };
    let marker = PathBuf::from(marker);
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !marker.exists() && std::time::Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    marker.exists()
}

fn read_spec(reader: &mut impl Read) -> io::Result<SpawnSpec> {
    let mut length = [0_u8; 4];
    reader.read_exact(&mut length)?;
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_SPEC_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Guardian command specification exceeds 64 KiB.",
        ));
    }
    let mut bytes = vec![0_u8; length];
    reader.read_exact(&mut bytes)?;
    serde_json::from_slice(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

fn hide_window(command: &mut Command) {
    let _ = command;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{TcpListener, TcpStream};
    use std::time::Instant;

    fn fixture(args: &[&str]) -> Command {
        let mut command = Command::new(guardian_executable().unwrap());
        command.env_clear().args(args);
        command
    }

    fn unused_port() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        port
    }

    fn wait_for_marker(marker: &std::path::Path) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while !marker.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(marker.exists(), "fixture holder did not create its marker");
    }

    fn wait_for_port_to_close(port: u16) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline {
            if TcpStream::connect(("127.0.0.1", port)).is_err() {
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        panic!("guarded fixture still listens on port {port}");
    }

    #[test]
    fn serializes_only_explicit_environment_values() {
        let mut command = Command::new("tool");
        command
            .env_clear()
            .env("PATH", "/tmp/bin")
            .arg("--input")
            .arg("document.txt")
            .current_dir("/tmp");
        let spec = serialize_command(&command, true).unwrap();
        assert_eq!(spec.program, WireString::Utf8("tool".into()));
        assert_eq!(
            spec.args,
            [
                WireString::Utf8("--input".into()),
                WireString::Utf8("document.txt".into())
            ]
        );
        assert_eq!(
            spec.environment,
            [(
                WireString::Utf8("PATH".into()),
                WireString::Utf8("/tmp/bin".into())
            )]
        );
        assert_eq!(spec.current_dir, Some(WireString::Utf8("/tmp".into())));
        assert!(spec.capture);
    }

    #[test]
    fn rejects_environment_removal() {
        let mut command = Command::new("tool");
        command.env_remove("PATH");
        assert_eq!(
            serialize_command(&command, false).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn rejects_oversized_protocol_frame() {
        let mut input = Vec::new();
        input.extend_from_slice(&((MAX_SPEC_BYTES as u32) + 1).to_le_bytes());
        assert_eq!(
            read_spec(&mut input.as_slice()).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[cfg(unix)]
    #[test]
    fn preserves_non_utf8_os_arguments() {
        use std::os::unix::ffi::OsStringExt;

        let original = OsString::from_vec(vec![b'a', 0xFF, b'b']);
        let wire = WireString::from_os(&original);
        assert_eq!(wire.into_os(), original);
    }

    #[test]
    fn forwards_output_and_normal_exit_status() {
        let mut child = spawn(&fixture(&["emit"]), true).unwrap();
        let mut stdout = String::new();
        let mut stderr = String::new();
        child
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut stdout)
            .unwrap();
        child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(7));
        assert_eq!(stdout, "fixture stdout");
        assert_eq!(stderr, "fixture stderr");
    }

    #[test]
    fn closing_lease_kills_and_reaps_the_inner_child() {
        let temporary = tempfile::tempdir().unwrap();
        let marker = temporary.path().join("holder.pid");
        let port = unused_port();
        let mut child = spawn(
            &fixture(&["hold", marker.to_str().unwrap(), &port.to_string()]),
            false,
        )
        .unwrap();
        wait_for_marker(&marker);
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        wait_for_port_to_close(port);
        TcpListener::bind(("127.0.0.1", port)).unwrap();
    }

    #[test]
    fn abrupt_parent_exit_closes_lease_and_reaps_the_inner_child() {
        let temporary = tempfile::tempdir().unwrap();
        let marker = temporary.path().join("abrupt-holder.pid");
        let port = unused_port();
        let status = fixture(&["lease-parent", marker.to_str().unwrap(), &port.to_string()])
            .status()
            .unwrap();
        assert!(status.success());
        wait_for_marker(&marker);
        wait_for_port_to_close(port);
        TcpListener::bind(("127.0.0.1", port)).unwrap();
    }

    #[test]
    fn delayed_post_spawn_ack_releases_lease_and_reaps_child() {
        let temporary = tempfile::tempdir().unwrap();
        let marker = temporary.path().join("delayed-holder.pid");
        let port = unused_port();
        let mut command = fixture(&["hold", marker.to_str().unwrap(), &port.to_string()]);
        command
            .env("CACHELLA_TEST_GUARD_DELAY_ACK_MS", "200")
            .env("CACHELLA_TEST_GUARD_ACK_TIMEOUT_MS", "20")
            .env("CACHELLA_TEST_GUARD_READY_MARKER", marker.as_os_str());
        let error = match spawn(&command, false) {
            Ok(_) => panic!("delayed guardian acknowledgement unexpectedly succeeded"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        wait_for_marker(&marker);
        wait_for_port_to_close(port);
        TcpListener::bind(("127.0.0.1", port)).unwrap();
    }

    #[test]
    fn post_spawn_guard_failure_reaps_child_before_startup_error_returns() {
        let temporary = tempfile::tempdir().unwrap();
        let marker = temporary.path().join("failed-holder.pid");
        let port = unused_port();
        let mut command = fixture(&["hold", marker.to_str().unwrap(), &port.to_string()]);
        command
            .env("CACHELLA_TEST_GUARD_READY_MARKER", marker.as_os_str())
            .env("CACHELLA_TEST_GUARD_FAIL_AFTER_SPAWN", "1");
        assert!(spawn(&command, false).is_err());
        wait_for_marker(&marker);
        wait_for_port_to_close(port);
        TcpListener::bind(("127.0.0.1", port)).unwrap();
    }

    #[test]
    fn lease_closure_does_not_wait_for_unread_forwarded_output() {
        let mut child = spawn(&fixture(&["emit-large"]), true).unwrap();
        thread::sleep(Duration::from_millis(50));
        child.kill().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let _ = child.wait().unwrap();
        assert!(
            Instant::now() < deadline,
            "guardian waited for blocked relay output"
        );
    }

    #[test]
    fn forwards_large_normal_output_when_drained_concurrently() {
        let mut child = spawn(&fixture(&["emit-large"]), true).unwrap();
        let stdout = child.stdout.take().unwrap();
        let reader = thread::spawn(move || {
            let mut output = Vec::new();
            let mut stdout = stdout;
            stdout.read_to_end(&mut output).unwrap();
            output
        });
        assert!(child.wait().unwrap().success());
        assert_eq!(reader.join().unwrap().len(), 2 * 1024 * 1024);
    }
}
