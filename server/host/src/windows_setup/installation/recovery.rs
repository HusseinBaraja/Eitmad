//! Retained, credential-free inputs bind a retry to one installation attempt.
use super::*;
use eitmad_contracts::setup::WindowsSetupRecovery as Recovery;

fn public_request(request: &InstallWindows) -> Result<serde_json::Value, MainError> {
    let mut request = request.clone();
    if let InstallWindows::Server(server) = &mut request {
        server.preparation.workshop.owner_password =
            eitmad_contracts::accounts::AccountPassword::new("");
        if let Some(login) = &mut server.first_desktop {
            zeroize::Zeroize::zeroize(&mut login.password);
            login.device_proof = None;
        }
    }
    serde_json::to_value(request).map_err(|_| MainError::Setup)
}

pub(in crate::windows_setup) fn save(
    root: &Path,
    request: &InstallWindows,
) -> Result<(), MainError> {
    preparation::write_new(
        &root.join("recovery-input.json"),
        &serde_json::to_vec(&public_request(request)?).map_err(|_| MainError::Setup)?,
    )
}

pub(in crate::windows_setup) fn load(root: &Path) -> Result<InstallWindows, MainError> {
    let path = root.join("recovery-input.json");
    plain_path(&path, true)?;
    let file = File::open(path).map_err(|_| MainError::Setup)?;
    let request: InstallWindows =
        serde_json::from_reader(file.take(65537)).map_err(|_| MainError::Setup)?;
    // A journal is not a place to retain credentials or a replayable proof.
    if serde_json::to_value(&request).map_err(|_| MainError::Setup)? != public_request(&request)? {
        return Err(MainError::Setup);
    }
    Ok(request)
}

pub(super) fn binding(request: &InstallWindows) -> Result<serde_json::Value, MainError> {
    let mut value = public_request(request)?;
    let payload = &mut value["payload"];
    // Elevation rebinds these authenticated tools to a protected copy of the same release.
    if matches!(request, InstallWindows::Server(_)) {
        for field in ["platformScript", "postgresBin", "certificateExecutable"] {
            payload["preparation"][field] = serde_json::Value::Null;
        }
        payload["server"]["directory"] = serde_json::Value::Null;
        payload["client"]["desktop"]["directory"] = serde_json::Value::Null;
        payload["winswExecutable"] = serde_json::Value::Null;
    }
    Ok(value)
}

pub(in crate::windows_setup) fn action(
    root: &Path,
    active: bool,
    progress: &WindowsInstallProgress,
) -> Recovery {
    if active {
        return Recovery::Wait;
    }
    if progress.stage == Stage::Complete && progress.status == Status::Succeeded {
        return Recovery::Complete;
    }
    if progress.status == Status::Blocked {
        return Recovery::InspectManually;
    }
    match load(root) {
        Ok(InstallWindows::Client(_)) => Recovery::Retry,
        Ok(InstallWindows::Server(_)) => Recovery::OwnerPassword,
        Err(_) => Recovery::InspectManually,
    }
}

/// A retry can reuse public trust only when its bytes are unchanged.
pub(in crate::windows_setup) fn retain_exact(path: &Path, bytes: &[u8]) -> Result<(), MainError> {
    plain_path(path, false)?;
    if path.try_exists().map_err(|_| MainError::Setup)? {
        if fs::read(path).map_err(|_| MainError::Setup)? != bytes {
            return Err(MainError::Setup);
        }
        return Ok(());
    }
    preparation::write_new(path, bytes)
}

impl Progress {
    pub(in crate::windows_setup) fn open(
        root: &str,
        request: &InstallWindows,
    ) -> Result<Self, MainError> {
        if !Path::new(root).try_exists().map_err(|_| MainError::Setup)? {
            let progress = Self::create(root)?;
            save(Path::new(root), request)?;
            return Ok(progress);
        }
        plain_path(Path::new(root), true)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .open(Path::new(root).join("installation.lock"))
            .map_err(|_| MainError::Setup)?;
        if binding(&load(Path::new(root))?)? != binding(request)? {
            return Err(MainError::Setup);
        }
        // A completed parent attempt is not a request to install again. The
        // elevated worker can be revisited to recheck HTTPS and return the same enrollment.
        if root
            == match request {
                InstallWindows::Client(client) => &client.progress_directory,
                InstallWindows::Server(server) => &server.client.progress_directory,
            }
            && read_last(Path::new(root))?
                .is_some_and(|p| p.stage == Stage::Complete && p.status == Status::Succeeded)
        {
            return Err(MainError::Setup);
        }
        let sequence = record_paths(Path::new(root))?
            .last()
            .map(|path| {
                path.file_stem()
                    .and_then(|name| name.to_str())
                    .ok_or(MainError::Setup)?
                    .parse::<u32>()
                    .map_err(|_| MainError::Setup)
            })
            .transpose()?
            .unwrap_or(0);
        // Never overwrite a torn record. Wire progress starts at one for each new process.
        Ok(Self {
            root: root.into(),
            _lock: lock,
            sequence,
            wire_sequence: 0,
            stage: Stage::Preflight,
            blocked: false,
        })
    }
}
