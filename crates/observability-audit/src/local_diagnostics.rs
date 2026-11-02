//! Bounded local technical evidence. No payload or free-text error API is provided.
use std::{
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use eitmad_contracts::local_diagnostics::{
    LocalDiagnosticComponent, LocalDiagnosticEvent, LocalDiagnosticPolicy,
};
use sha2::{Digest as _, Sha256};
use uuid::Uuid;

static RECORDER: OnceLock<Recorder> = OnceLock::new();
static POLICY: OnceLock<LocalDiagnosticPolicy> = OnceLock::new();
enum WriteRequest {
    Event(Box<LocalDiagnosticEvent>),
    Flush(mpsc::Sender<()>),
}
struct Recorder {
    output: SyncSender<WriteRequest>,
    dropped: AtomicU64,
}

/// Start once per executable. The first call fixes the process-wide component, directory,
/// and build and starts a bounded writer. Collection failure never prevents product startup.
/// Each call records startup and replaces the panic hook with metadata-only capture and a
/// best-effort flush. The previous hook is not called: its output can expose panic payloads.
pub fn initialize(component: LocalDiagnosticComponent, directory: PathBuf, build: &str) {
    RECORDER.get_or_init(|| {
        let build = build.to_owned();
        let policy = LocalDiagnosticPolicy::default();
        let (output, input) = mpsc::sync_channel(policy.queue_capacity);
        let _ = std::thread::Builder::new()
            .name("local-diagnostics".into())
            .spawn(move || {
                let mut sink = RollingLog::new(directory, component);
                let hash = executable_hash().unwrap_or_default();
                let session = Uuid::new_v4();
                let mut sequence = 0;
                for request in input {
                    match request {
                        WriteRequest::Event(mut event) => {
                            sequence += 1;
                            event.session = session;
                            event.sequence = sequence;
                            event.component = component;
                            event.build.clone_from(&build);
                            event.binary_sha256.clone_from(&hash);
                            if sink.append(&event).is_err() {
                                if let Some(recorder) = RECORDER.get() {
                                    recorder
                                        .dropped
                                        .fetch_add(event.dropped + 1, Ordering::Relaxed);
                                }
                            }
                        }
                        WriteRequest::Flush(done) => {
                            let _ = done.send(());
                        }
                    }
                }
            });
        Recorder {
            output,
            dropped: AtomicU64::new(0),
        }
    });
    record(event("Process", "Started"));
    std::panic::set_hook(Box::new(|info| {
        let mut failure = event("Crash", "Failed");
        if let Some(location) = info.location() {
            let name = location
                .file()
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or("unknown");
            failure
                .locations
                .push(format!("{name}:{}:{}", location.line(), location.column()));
        }
        // A panic payload can contain domain data or credentials. Never inspect it.
        record(failure);
        flush();
    }));
}

#[must_use]
pub fn user_directory() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| PathBuf::from(root).join("Eitmad/Logs"))
}

#[must_use]
pub fn event(phase: &str, result: &str) -> LocalDiagnosticEvent {
    LocalDiagnosticEvent {
        version: 1,
        utc_millis: u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        )
        .unwrap_or(u64::MAX),
        session: Uuid::nil(),
        sequence: 0,
        process_id: std::process::id(),
        component: LocalDiagnosticComponent::Engine,
        build: String::new(),
        binary_sha256: String::new(),
        os_version: std::env::consts::OS.into(),
        phase: phase.into(),
        result: result.into(),
        correlation: None,
        operation: None,
        error_code: None,
        elapsed_ms: None,
        native_code: None,
        http_status: None,
        locations: Vec::new(),
        dropped: 0,
    }
}

/// Enqueue approved technical metadata without waiting; do nothing before initialization.
/// A full/disconnected queue counts drops for the next event. The worker stamps session,
/// sequence, component, build and binary identity and counts invalid or unwritable records.
pub fn record(mut event: LocalDiagnosticEvent) {
    if let Some(recorder) = RECORDER.get() {
        event.dropped = recorder.dropped.swap(0, Ordering::Relaxed);
        let dropped = event.dropped;
        if recorder
            .output
            .try_send(WriteRequest::Event(Box::new(event)))
            .is_err()
        {
            recorder.dropped.fetch_add(dropped + 1, Ordering::Relaxed);
        }
    }
}

/// Best-effort wait for earlier queued writes, for at most two seconds. Do nothing before
/// initialization or when the flush marker cannot enter the queue; this does not force disk durability.
pub fn flush() {
    if let Some(recorder) = RECORDER.get() {
        let (done, waiting) = mpsc::channel();
        if recorder.output.try_send(WriteRequest::Flush(done)).is_ok() {
            let _ = waiting.recv_timeout(Duration::from_secs(2));
        }
    }
}

fn executable_hash() -> std::io::Result<String> {
    let mut file = File::open(std::env::current_exe()?)?;
    let mut hash = Sha256::new();
    let mut buffer = vec![0; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub struct RollingLog {
    directory: PathBuf,
    component: LocalDiagnosticComponent,
    path: Option<PathBuf>,
}
impl RollingLog {
    #[must_use]
    pub const fn new(directory: PathBuf, component: LocalDiagnosticComponent) -> Self {
        Self {
            directory,
            component,
            path: None,
        }
    }

    /// Append a single bounded event, retaining at most the policy's files per component.
    ///
    /// # Errors
    /// Returns an I/O error for unsafe paths, invalid records, or unavailable storage.
    pub fn append(&mut self, event: &LocalDiagnosticEvent) -> std::io::Result<()> {
        let policy = POLICY.get_or_init(LocalDiagnosticPolicy::default);
        let mut bytes = serde_json::to_vec(event)?;
        if !valid(event, policy) || bytes.len() >= policy.record_bytes {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        bytes.push(b'\n');
        reject_links(&self.directory)?;
        fs::create_dir_all(&self.directory)?;
        if self.path.as_ref().is_none_or(|path| {
            fs::metadata(path).map_or(true, |m| {
                m.len() + bytes.len() as u64 > policy.file_bytes as u64
            })
        }) {
            let prefix = format!("{:?}-", self.component).to_lowercase();
            let path = self
                .directory
                .join(format!("{prefix}{}.jsonl", Uuid::new_v4()));
            File::create_new(&path)?;
            self.path = Some(path);
            let mut files = fs::read_dir(&self.directory)?
                .filter_map(Result::ok)
                .filter(|entry| {
                    entry.file_name().to_str().is_some_and(|name| {
                        name.starts_with(&prefix)
                            && Path::new(name)
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
                    })
                })
                .filter(|entry| {
                    entry
                        .file_type()
                        .is_ok_and(|kind| kind.is_file() && !kind.is_symlink())
                })
                .collect::<Vec<_>>();
            files.sort_by_key(|entry| entry.metadata().and_then(|meta| meta.modified()).ok());
            let remove = files.len().saturating_sub(policy.files_per_component);
            for file in files
                .into_iter()
                .filter(|file| Some(&file.path()) != self.path.as_ref())
                .take(remove)
            {
                fs::remove_file(file.path())?;
            }
        }
        let path = self.path.as_ref().ok_or(std::io::ErrorKind::NotFound)?;
        reject_links(path)?;
        OpenOptions::new()
            .append(true)
            .open(path)?
            .write_all(&bytes)
    }
}

fn valid(event: &LocalDiagnosticEvent, policy: &LocalDiagnosticPolicy) -> bool {
    event.version == 1
        && policy.phases.contains(&event.phase.as_str())
        && policy.results.contains(&event.result.as_str())
        && event
            .error_code
            .as_ref()
            .is_none_or(|code| policy.error_codes.contains(&code.as_str()))
        && event
            .operation
            .as_ref()
            .is_none_or(|name| policy.operations.contains(&name.as_str()))
        && event.locations.len() <= policy.max_locations
        && event.locations.iter().all(|s| symbol(s))
        && (event.build.is_empty() || symbol(&event.build))
        && (event.os_version.is_empty() || symbol(&event.os_version))
        && (event.binary_sha256.is_empty()
            || event.binary_sha256.len() == 64
                && event.binary_sha256.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn symbol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 240
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:+<>`/{}-".contains(&b))
}

fn reject_links(path: &Path) -> std::io::Result<()> {
    for ancestor in path.ancestors() {
        if let Ok(meta) = fs::symlink_metadata(ancestor) {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt as _;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err(std::io::ErrorKind::PermissionDenied.into());
                }
            }
            if meta.is_symlink() {
                return Err(std::io::ErrorKind::PermissionDenied.into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounded_files_retain_parseable_metadata_and_reject_unknown_fields() {
        let root = tempfile::tempdir().unwrap();
        let mut sink = RollingLog::new(root.path().into(), LocalDiagnosticComponent::Engine);
        let mut sample = event("Crash", "Failed");
        sample.locations = vec!["connection.rs:123:4".into(); 12];
        let bytes = serde_json::to_vec(&sample).unwrap().len() + 1;
        let policy = LocalDiagnosticPolicy::default();
        for _ in 0..(policy.file_bytes / bytes * (policy.files_per_component + 1)) {
            sink.append(&sample).unwrap();
        }
        let files = fs::read_dir(root.path())
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        assert_eq!(files.len(), policy.files_per_component);
        for file in files {
            assert!(file.metadata().unwrap().len() <= policy.file_bytes as u64);
            for line in fs::read_to_string(file.path()).unwrap().lines() {
                serde_json::from_str::<LocalDiagnosticEvent>(line).unwrap();
            }
        }
        sample.error_code = Some("private-customer-sentinel".into());
        assert!(sink.append(&sample).is_err());
        sample.error_code = None;
        sample.locations = vec!["password=private-sentinel".into()];
        assert!(sink.append(&sample).is_err());
        assert!(
            serde_json::from_str::<LocalDiagnosticEvent>("{\"payload\":\"private-sentinel\"}")
                .is_err()
        );
    }
}
