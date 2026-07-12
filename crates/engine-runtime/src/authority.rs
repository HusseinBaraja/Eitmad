use std::{
    fs::{self, File, OpenOptions},
    io::{self, Seek as _, SeekFrom, Write as _},
    path::Path,
};

use eitmad_contracts::runtime::EngineProcessIdentity;
use fs2::FileExt as _;

pub(crate) enum AuthorityLockError {
    AlreadyRunning,
    Unavailable,
}

pub(crate) struct AuthorityLock {
    file: File,
}

impl AuthorityLock {
    pub(crate) fn acquire(
        runtime_directory: &Path,
        identity: &EngineProcessIdentity,
    ) -> Result<Self, AuthorityLockError> {
        fs::create_dir_all(runtime_directory).map_err(|_| AuthorityLockError::Unavailable)?;
        let path = runtime_directory.join("engine.authority.lock");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|error| classify_lock_error(&error))?;

        if let Err(error) = file.try_lock_exclusive() {
            return Err(classify_lock_error(&error));
        }

        let encoded = serde_json::to_vec(identity).map_err(|_| AuthorityLockError::Unavailable)?;
        file.set_len(0)
            .and_then(|()| file.seek(SeekFrom::Start(0)).map(|_| ()))
            .and_then(|()| file.write_all(&encoded))
            .and_then(|()| file.sync_data())
            .map_err(|_| AuthorityLockError::Unavailable)?;

        Ok(Self { file })
    }

    pub(crate) fn release(self) -> Result<(), AuthorityLockError> {
        fs2::FileExt::unlock(&self.file).map_err(|_| AuthorityLockError::Unavailable)
    }
}

fn classify_lock_error(error: &io::Error) -> AuthorityLockError {
    let locked = error.kind() == io::ErrorKind::WouldBlock;
    #[cfg(target_os = "windows")]
    let locked = locked || matches!(error.raw_os_error(), Some(32 | 33));

    if locked {
        AuthorityLockError::AlreadyRunning
    } else {
        AuthorityLockError::Unavailable
    }
}
