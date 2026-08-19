//! Rust-owned cross-platform secret persistence.
//!
//! Native credential stores are preferred. The encrypted file backend is used
//! only when the native store cannot complete a write/read/delete probe and a
//! trusted fallback key is supplied out of band.

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
};

use aes_gcm::{
    Aes256Gcm, KeyInit as _, Nonce,
    aead::{Aead as _, AeadCore as _, OsRng, Payload},
};
use eitmad_contracts::secrets::SecretId;
use fs2::FileExt as _;
use sha2::{Digest as _, Sha256};
use uuid::Uuid;
use zeroize::Zeroizing;

const FALLBACK_MAGIC: &[u8; 8] = b"EITSEC01";
const FALLBACK_NONCE_BYTES: usize = 12;
const MAX_SECRET_BYTES: usize = 64 * 1024;
const NATIVE_SERVICE_PREFIX: &str = "com.eitmad.secret";
const NATIVE_PROBE_SERVICE: &str = "com.eitmad.secret.probe";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretStorageError {
    EmptySecret,
    SecretTooLarge,
    NativeUnavailable,
    FallbackKeyRequired,
    Unavailable,
    Corrupt,
}

impl fmt::Display for SecretStorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::EmptySecret => "eitmad.error.secret-empty.v1",
            Self::SecretTooLarge => "eitmad.error.secret-too-large.v1",
            Self::NativeUnavailable => "eitmad.error.secret-native-unavailable.v1",
            Self::FallbackKeyRequired => "eitmad.error.secret-fallback-key-required.v1",
            Self::Unavailable => "eitmad.error.secret-storage-unavailable.v1",
            Self::Corrupt => "eitmad.error.secret-storage-corrupt.v1",
        };
        formatter.write_str(code)
    }
}

impl std::error::Error for SecretStorageError {}

pub struct SecretMaterial(Zeroizing<Vec<u8>>);

impl SecretMaterial {
    /// Creates bounded in-memory secret material.
    ///
    /// # Errors
    ///
    /// Rejects empty values and values larger than 64 KiB.
    pub fn new(value: impl Into<Vec<u8>>) -> Result<Self, SecretStorageError> {
        let value = Zeroizing::new(value.into());
        if value.is_empty() {
            return Err(SecretStorageError::EmptySecret);
        }
        if value.len() > MAX_SECRET_BYTES {
            return Err(SecretStorageError::SecretTooLarge);
        }
        Ok(Self(value))
    }

    /// Exposes the value only to the Rust capability that must use it.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for SecretMaterial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretMaterial([REDACTED])")
    }
}

pub struct FallbackEncryptionKey(Zeroizing<[u8; 32]>);

impl FallbackEncryptionKey {
    #[must_use]
    pub fn new(value: [u8; 32]) -> Self {
        Self(Zeroizing::new(value))
    }
}

impl fmt::Debug for FallbackEncryptionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FallbackEncryptionKey([REDACTED])")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretBackendKind {
    OsNative,
    EncryptedFallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteOutcome {
    Deleted,
    NotFound,
}

trait SecretBackend: Send + Sync {
    fn set(&self, id: &SecretId, material: &SecretMaterial) -> Result<(), SecretStorageError>;
    fn get(&self, id: &SecretId) -> Result<Option<SecretMaterial>, SecretStorageError>;
    fn delete(&self, id: &SecretId) -> Result<DeleteOutcome, SecretStorageError>;
}

#[derive(Clone)]
pub struct SecretStore {
    backend: Arc<dyn SecretBackend>,
    kind: SecretBackendKind,
}

impl SecretStore {
    /// Opens the OS-native store or, only when unavailable, an encrypted fallback.
    ///
    /// `fallback_key` must come from a trusted out-of-band source. It is never
    /// loaded from configuration or persisted beside encrypted values.
    ///
    /// # Errors
    ///
    /// Returns a sanitized availability error when no safe backend can open.
    pub fn open(
        fallback_directory: impl AsRef<Path>,
        fallback_key: Option<FallbackEncryptionKey>,
    ) -> Result<Self, SecretStorageError> {
        if NativeSecretBackend::probe().is_ok() {
            return Ok(Self {
                backend: Arc::new(NativeSecretBackend),
                kind: SecretBackendKind::OsNative,
            });
        }
        let key = fallback_key.ok_or(SecretStorageError::FallbackKeyRequired)?;
        let backend = EncryptedFallbackBackend::open(fallback_directory.as_ref(), key)?;
        Ok(Self {
            backend: Arc::new(backend),
            kind: SecretBackendKind::EncryptedFallback,
        })
    }

    #[must_use]
    pub const fn backend_kind(&self) -> SecretBackendKind {
        self.kind
    }

    /// Creates or replaces one typed secret.
    ///
    /// # Errors
    ///
    /// Returns only stable, sanitized validation or availability errors.
    pub fn set(&self, id: &SecretId, material: SecretMaterial) -> Result<(), SecretStorageError> {
        let result = self.backend.set(id, &material);
        drop(material);
        result
    }

    /// Retrieves one typed secret without serializing it.
    ///
    /// # Errors
    ///
    /// Returns only stable, sanitized availability or corruption errors.
    pub fn get(&self, id: &SecretId) -> Result<Option<SecretMaterial>, SecretStorageError> {
        self.backend.get(id)
    }

    /// Deletes one typed secret idempotently.
    ///
    /// # Errors
    ///
    /// Returns only stable, sanitized availability errors.
    pub fn delete(&self, id: &SecretId) -> Result<DeleteOutcome, SecretStorageError> {
        self.backend.delete(id)
    }
}

struct NativeSecretBackend;

impl NativeSecretBackend {
    fn probe() -> Result<(), SecretStorageError> {
        native_probe()
    }
}

impl SecretBackend for NativeSecretBackend {
    fn set(&self, id: &SecretId, material: &SecretMaterial) -> Result<(), SecretStorageError> {
        native_set(id, material.expose_secret())
    }

    fn get(&self, id: &SecretId) -> Result<Option<SecretMaterial>, SecretStorageError> {
        native_get(id)?.map(SecretMaterial::new).transpose()
    }

    fn delete(&self, id: &SecretId) -> Result<DeleteOutcome, SecretStorageError> {
        native_delete(id)
    }
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn native_probe() -> Result<(), SecretStorageError> {
    let account = format!("probe-{}", Uuid::new_v4());
    let entry = keyring::Entry::new(NATIVE_PROBE_SERVICE, &account)
        .map_err(|_| SecretStorageError::NativeUnavailable)?;
    let value = Uuid::new_v4();
    entry
        .set_secret(value.as_bytes())
        .map_err(|_| SecretStorageError::NativeUnavailable)?;
    let read = entry.get_secret().map_err(|_| {
        let _ = entry.delete_credential();
        SecretStorageError::NativeUnavailable
    })?;
    if read.as_slice() != value.as_bytes() {
        let _ = entry.delete_credential();
        return Err(SecretStorageError::NativeUnavailable);
    }
    entry
        .delete_credential()
        .map_err(|_| SecretStorageError::NativeUnavailable)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn native_probe() -> Result<(), SecretStorageError> {
    Err(SecretStorageError::NativeUnavailable)
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn native_entry(id: &SecretId) -> Result<keyring::Entry, SecretStorageError> {
    let service = format!("{NATIVE_SERVICE_PREFIX}.{}", id.kind());
    keyring::Entry::new(&service, &id.reference().value().to_string())
        .map_err(|_| SecretStorageError::Unavailable)
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn native_set(id: &SecretId, value: &[u8]) -> Result<(), SecretStorageError> {
    native_entry(id)?
        .set_secret(value)
        .map_err(|_| SecretStorageError::Unavailable)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn native_set(_id: &SecretId, _value: &[u8]) -> Result<(), SecretStorageError> {
    Err(SecretStorageError::NativeUnavailable)
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn native_get(id: &SecretId) -> Result<Option<Vec<u8>>, SecretStorageError> {
    match native_entry(id)?.get_secret() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err(SecretStorageError::Unavailable),
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn native_get(_id: &SecretId) -> Result<Option<Vec<u8>>, SecretStorageError> {
    Err(SecretStorageError::NativeUnavailable)
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn native_delete(id: &SecretId) -> Result<DeleteOutcome, SecretStorageError> {
    match native_entry(id)?.delete_credential() {
        Ok(()) => Ok(DeleteOutcome::Deleted),
        Err(keyring::Error::NoEntry) => Ok(DeleteOutcome::NotFound),
        Err(_) => Err(SecretStorageError::Unavailable),
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn native_delete(_id: &SecretId) -> Result<DeleteOutcome, SecretStorageError> {
    Err(SecretStorageError::NativeUnavailable)
}

struct EncryptedFallbackBackend {
    directory: PathBuf,
    key: FallbackEncryptionKey,
    process_lock: Mutex<()>,
}

impl EncryptedFallbackBackend {
    fn open(directory: &Path, key: FallbackEncryptionKey) -> Result<Self, SecretStorageError> {
        fs::create_dir_all(directory).map_err(|_| SecretStorageError::Unavailable)?;
        make_directory_private(directory)?;
        Ok(Self {
            directory: directory.to_owned(),
            key,
            process_lock: Mutex::new(()),
        })
    }

    fn path_for(&self, id: &SecretId) -> PathBuf {
        let digest = Sha256::digest(id.canonical_key().as_bytes());
        self.directory
            .join(format!("{}.secret", lowercase_hex(&digest)))
    }

    fn lock(&self) -> Result<(MutexGuard<'_, ()>, LockedFile), SecretStorageError> {
        let guard = self
            .process_lock
            .lock()
            .map_err(|_| SecretStorageError::Unavailable)?;
        let path = self.directory.join("secret-store.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|_| SecretStorageError::Unavailable)?;
        make_file_private(&path, &file)?;
        file.lock_exclusive()
            .map_err(|_| SecretStorageError::Unavailable)?;
        Ok((guard, LockedFile(file)))
    }

    fn encrypt(
        &self,
        id: &SecretId,
        material: &SecretMaterial,
    ) -> Result<Vec<u8>, SecretStorageError> {
        let cipher = Aes256Gcm::new_from_slice(self.key.0.as_ref())
            .map_err(|_| SecretStorageError::Unavailable)?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: material.expose_secret(),
                    aad: id.canonical_key().as_bytes(),
                },
            )
            .map_err(|_| SecretStorageError::Unavailable)?;
        let mut encoded = Vec::with_capacity(FALLBACK_MAGIC.len() + nonce.len() + ciphertext.len());
        encoded.extend_from_slice(FALLBACK_MAGIC);
        encoded.extend_from_slice(&nonce);
        encoded.extend_from_slice(&ciphertext);
        Ok(encoded)
    }

    fn decrypt(&self, id: &SecretId, encoded: &[u8]) -> Result<SecretMaterial, SecretStorageError> {
        if encoded.len() <= FALLBACK_MAGIC.len() + FALLBACK_NONCE_BYTES
            || &encoded[..FALLBACK_MAGIC.len()] != FALLBACK_MAGIC
        {
            return Err(SecretStorageError::Corrupt);
        }
        let nonce_start = FALLBACK_MAGIC.len();
        let ciphertext_start = nonce_start + FALLBACK_NONCE_BYTES;
        let nonce = Nonce::from_slice(&encoded[nonce_start..ciphertext_start]);
        let cipher = Aes256Gcm::new_from_slice(self.key.0.as_ref())
            .map_err(|_| SecretStorageError::Unavailable)?;
        let plaintext = cipher
            .decrypt(
                nonce,
                Payload {
                    msg: &encoded[ciphertext_start..],
                    aad: id.canonical_key().as_bytes(),
                },
            )
            .map_err(|_| SecretStorageError::Corrupt)?;
        SecretMaterial::new(plaintext).map_err(|_| SecretStorageError::Corrupt)
    }
}

impl SecretBackend for EncryptedFallbackBackend {
    fn set(&self, id: &SecretId, material: &SecretMaterial) -> Result<(), SecretStorageError> {
        let (_guard, _file_lock) = self.lock()?;
        let encoded = self.encrypt(id, material)?;
        let path = self.path_for(id);
        recover_backup(&path)?;
        replace_private_file(&path, &encoded)
    }

    fn get(&self, id: &SecretId) -> Result<Option<SecretMaterial>, SecretStorageError> {
        let (_guard, _file_lock) = self.lock()?;
        let path = self.path_for(id);
        recover_backup(&path)?;
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(SecretStorageError::Unavailable),
        };
        let maximum = MAX_SECRET_BYTES + FALLBACK_MAGIC.len() + FALLBACK_NONCE_BYTES + 16;
        let mut encoded = Vec::new();
        std::io::Read::by_ref(&mut file)
            .take(u64::try_from(maximum + 1).unwrap_or(u64::MAX))
            .read_to_end(&mut encoded)
            .map_err(|_| SecretStorageError::Unavailable)?;
        if encoded.len() > maximum {
            return Err(SecretStorageError::Corrupt);
        }
        self.decrypt(id, &encoded).map(Some)
    }

    fn delete(&self, id: &SecretId) -> Result<DeleteOutcome, SecretStorageError> {
        let (_guard, _file_lock) = self.lock()?;
        let path = self.path_for(id);
        recover_backup(&path)?;
        let outcome = match fs::remove_file(&path) {
            Ok(()) => DeleteOutcome::Deleted,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => DeleteOutcome::NotFound,
            Err(_) => return Err(SecretStorageError::Unavailable),
        };
        let _ = fs::remove_file(backup_path(&path));
        Ok(outcome)
    }
}

struct LockedFile(File);

impl Drop for LockedFile {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}

fn replace_private_file(path: &Path, content: &[u8]) -> Result<(), SecretStorageError> {
    let parent = path.parent().ok_or(SecretStorageError::Unavailable)?;
    let temporary = parent.join(format!(".secret-write-{}", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|_| SecretStorageError::Unavailable)?;
    if make_file_private(&temporary, &file).is_err()
        || file.write_all(content).is_err()
        || file.sync_all().is_err()
    {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(SecretStorageError::Unavailable);
    }
    drop(file);

    let backup = backup_path(path);
    if path.exists() && backup.exists() {
        fs::remove_file(&backup).map_err(|_| SecretStorageError::Unavailable)?;
    }
    let had_existing = match fs::rename(path, &backup) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            let _ = fs::remove_file(&temporary);
            return Err(SecretStorageError::Unavailable);
        }
    };
    if fs::rename(&temporary, path).is_err() {
        if had_existing {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temporary);
        return Err(SecretStorageError::Unavailable);
    }
    if had_existing {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("secret.backup")
}

fn recover_backup(path: &Path) -> Result<(), SecretStorageError> {
    if path.exists() {
        return Ok(());
    }
    let backup = backup_path(path);
    match fs::rename(backup, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err(SecretStorageError::Unavailable),
    }
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(unix)]
fn make_directory_private(path: &Path) -> Result<(), SecretStorageError> {
    use std::os::unix::fs::PermissionsExt as _;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| SecretStorageError::Unavailable)
}

#[cfg(windows)]
fn make_directory_private(path: &Path) -> Result<(), SecretStorageError> {
    make_windows_path_private(path, true)
}

#[cfg(all(not(unix), not(windows)))]
fn make_directory_private(path: &Path) -> Result<(), SecretStorageError> {
    fs::metadata(path)
        .map(|_| ())
        .map_err(|_| SecretStorageError::Unavailable)
}

#[cfg(unix)]
fn make_file_private(_path: &Path, file: &File) -> Result<(), SecretStorageError> {
    use std::os::unix::fs::PermissionsExt as _;
    file.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|_| SecretStorageError::Unavailable)
}

#[cfg(windows)]
fn make_file_private(path: &Path, _file: &File) -> Result<(), SecretStorageError> {
    make_windows_path_private(path, false)
}

#[cfg(all(not(unix), not(windows)))]
fn make_file_private(_path: &Path, file: &File) -> Result<(), SecretStorageError> {
    file.metadata()
        .map(|_| ())
        .map_err(|_| SecretStorageError::Unavailable)
}

#[cfg(windows)]
fn make_windows_path_private(path: &Path, directory: bool) -> Result<(), SecretStorageError> {
    use std::{ffi::OsStr, process::Command};

    let output = Command::new(windows_system_tool("whoami.exe"))
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|_| SecretStorageError::Unavailable)?;
    if !output.status.success() {
        return Err(SecretStorageError::Unavailable);
    }
    let decoded = String::from_utf8_lossy(&output.stdout);
    let sid = decoded
        .split(|character: char| {
            !(character == 'S' || character == '-' || character.is_ascii_digit())
        })
        .find(|value| value.starts_with("S-1-") && value.len() > 4)
        .ok_or(SecretStorageError::Unavailable)?;
    let grant = if directory {
        format!("*{sid}:(OI)(CI)F")
    } else {
        format!("*{sid}:F")
    };
    let output = Command::new(windows_system_tool("icacls.exe"))
        .args([
            path.as_os_str(),
            OsStr::new("/inheritance:r"),
            OsStr::new("/grant:r"),
            OsStr::new(&grant),
            OsStr::new("/q"),
        ])
        .output()
        .map_err(|_| SecretStorageError::Unavailable)?;
    output
        .status
        .success()
        .then_some(())
        .ok_or(SecretStorageError::Unavailable)
}

#[cfg(windows)]
fn windows_system_tool(name: &str) -> PathBuf {
    std::env::var_os("SystemRoot").map_or_else(
        || PathBuf::from(name),
        |root| PathBuf::from(root).join("System32").join(name),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use eitmad_contracts::{
        config::SecretReferenceId,
        secrets::{SecretId, SecretKind},
    };
    use tempfile::TempDir;

    use super::*;

    fn id(value: u128) -> SecretId {
        SecretId::new(
            SecretKind::parse("external-api-token").unwrap(),
            SecretReferenceId::new(Uuid::from_u128(value)),
        )
    }

    fn fallback(directory: &Path) -> SecretStore {
        SecretStore {
            backend: Arc::new(
                EncryptedFallbackBackend::open(directory, FallbackEncryptionKey::new([7; 32]))
                    .unwrap(),
            ),
            kind: SecretBackendKind::EncryptedFallback,
        }
    }

    #[test]
    fn encrypted_fallback_supports_secret_lifecycle_without_plaintext_files() {
        let directory = TempDir::new().unwrap();
        let store = fallback(directory.path());
        let identifier = id(1);
        let secret = b"token-never-written-in-plaintext";

        assert!(store.get(&identifier).unwrap().is_none());
        store
            .set(&identifier, SecretMaterial::new(secret.to_vec()).unwrap())
            .unwrap();
        assert_eq!(
            store.get(&identifier).unwrap().unwrap().expose_secret(),
            secret
        );
        let secret_path = match store.backend.as_ref().get(&identifier) {
            Ok(Some(_)) => {
                let digest = Sha256::digest(identifier.canonical_key().as_bytes());
                directory
                    .path()
                    .join(format!("{}.secret", lowercase_hex(&digest)))
            }
            _ => panic!("stored secret must exist"),
        };
        let persisted = fs::read(secret_path).unwrap();
        assert!(
            !persisted
                .windows(secret.len())
                .any(|window| window == secret)
        );

        store
            .set(
                &identifier,
                SecretMaterial::new(b"replacement".to_vec()).unwrap(),
            )
            .unwrap();
        assert_eq!(
            store.get(&identifier).unwrap().unwrap().expose_secret(),
            b"replacement"
        );
        assert_eq!(store.delete(&identifier).unwrap(), DeleteOutcome::Deleted);
        assert_eq!(store.delete(&identifier).unwrap(), DeleteOutcome::NotFound);
        assert!(store.get(&identifier).unwrap().is_none());
    }

    #[test]
    fn fallback_rejects_wrong_keys_and_identifier_swaps() {
        let directory = TempDir::new().unwrap();
        let identifier = id(1);
        fallback(directory.path())
            .set(
                &identifier,
                SecretMaterial::new(b"synthetic-token".to_vec()).unwrap(),
            )
            .unwrap();

        let wrong_key = SecretStore {
            backend: Arc::new(
                EncryptedFallbackBackend::open(
                    directory.path(),
                    FallbackEncryptionKey::new([8; 32]),
                )
                .unwrap(),
            ),
            kind: SecretBackendKind::EncryptedFallback,
        };
        assert!(matches!(
            wrong_key.get(&identifier),
            Err(SecretStorageError::Corrupt)
        ));

        let backend =
            EncryptedFallbackBackend::open(directory.path(), FallbackEncryptionKey::new([7; 32]))
                .unwrap();
        let source = backend.path_for(&identifier);
        let destination = backend.path_for(&id(2));
        fs::copy(source, destination).unwrap();
        assert!(matches!(
            backend.get(&id(2)),
            Err(SecretStorageError::Corrupt)
        ));
    }

    #[test]
    fn fallback_recovers_the_previous_value_after_interrupted_replacement() {
        let directory = TempDir::new().unwrap();
        let backend =
            EncryptedFallbackBackend::open(directory.path(), FallbackEncryptionKey::new([7; 32]))
                .unwrap();
        let identifier = id(9);
        backend
            .set(
                &identifier,
                &SecretMaterial::new(b"recoverable".to_vec()).unwrap(),
            )
            .unwrap();
        let path = backend.path_for(&identifier);
        fs::rename(&path, backup_path(&path)).unwrap();

        assert_eq!(
            backend.get(&identifier).unwrap().unwrap().expose_secret(),
            b"recoverable"
        );
        assert!(path.exists());
        assert!(!backup_path(&path).exists());
    }

    #[test]
    #[ignore = "writes one synthetic value to the OS credential store"]
    fn os_native_backend_supports_secret_lifecycle() {
        struct Cleanup(SecretId);

        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = native_delete(&self.0);
            }
        }

        NativeSecretBackend::probe().unwrap();
        let identifier = SecretId::new(
            SecretKind::parse("integration-test-token").unwrap(),
            SecretReferenceId::new(Uuid::new_v4()),
        );
        let _cleanup = Cleanup(identifier.clone());
        let backend = NativeSecretBackend;
        backend
            .set(
                &identifier,
                &SecretMaterial::new(b"synthetic-native-secret".to_vec()).unwrap(),
            )
            .unwrap();
        assert_eq!(
            backend.get(&identifier).unwrap().unwrap().expose_secret(),
            b"synthetic-native-secret"
        );
        assert_eq!(backend.delete(&identifier).unwrap(), DeleteOutcome::Deleted);
        assert!(backend.get(&identifier).unwrap().is_none());
    }

    #[derive(Default)]
    struct MemoryBackend {
        values: Mutex<BTreeMap<String, Vec<u8>>>,
    }

    impl SecretBackend for MemoryBackend {
        fn set(&self, id: &SecretId, material: &SecretMaterial) -> Result<(), SecretStorageError> {
            self.values
                .lock()
                .unwrap()
                .insert(id.canonical_key(), material.expose_secret().to_vec());
            Ok(())
        }

        fn get(&self, id: &SecretId) -> Result<Option<SecretMaterial>, SecretStorageError> {
            self.values
                .lock()
                .unwrap()
                .get(&id.canonical_key())
                .cloned()
                .map(SecretMaterial::new)
                .transpose()
        }

        fn delete(&self, id: &SecretId) -> Result<DeleteOutcome, SecretStorageError> {
            Ok(
                if self
                    .values
                    .lock()
                    .unwrap()
                    .remove(&id.canonical_key())
                    .is_some()
                {
                    DeleteOutcome::Deleted
                } else {
                    DeleteOutcome::NotFound
                },
            )
        }
    }

    #[test]
    fn typed_store_lifecycle_is_backend_independent() {
        let store = SecretStore {
            backend: Arc::new(MemoryBackend::default()),
            kind: SecretBackendKind::OsNative,
        };
        let identifier = id(3);
        store
            .set(
                &identifier,
                SecretMaterial::new(b"opaque".to_vec()).unwrap(),
            )
            .unwrap();
        assert_eq!(store.backend_kind(), SecretBackendKind::OsNative);
        assert_eq!(
            store.get(&identifier).unwrap().unwrap().expose_secret(),
            b"opaque"
        );
        assert_eq!(store.delete(&identifier).unwrap(), DeleteOutcome::Deleted);
    }

    #[test]
    fn secrets_and_fallback_keys_are_redacted_from_debug_and_errors() {
        let material = SecretMaterial::new(b"leak-sentinel".to_vec()).unwrap();
        let key = FallbackEncryptionKey::new([b'x'; 32]);
        let output = format!("{material:?} {key:?} {}", SecretStorageError::Corrupt);

        assert!(!output.contains("leak-sentinel"));
        assert!(!output.contains("xxxxxxxx"));
        assert!(output.contains("[REDACTED]"));
        assert!(output.contains("eitmad.error.secret-storage-corrupt.v1"));
    }

    #[test]
    fn invalid_and_oversized_secret_values_are_rejected() {
        assert!(matches!(
            SecretMaterial::new(Vec::new()),
            Err(SecretStorageError::EmptySecret)
        ));
        assert!(matches!(
            SecretMaterial::new(vec![0; MAX_SECRET_BYTES + 1]),
            Err(SecretStorageError::SecretTooLarge)
        ));
    }
}
