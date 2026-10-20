use eitmad_contracts::discovery::ServerInstallationId;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read as _, Write as _},
    path::Path,
};
use uuid::Uuid;

/// Creates an installation identity once. Existing invalid or interrupted files are never replaced.
/// # Errors
/// Returns a filesystem or invalid-data error. The caller must preserve the file for recovery.
pub fn load_or_create_identity(path: &Path) -> io::Result<ServerInstallationId> {
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => {
            let id = ServerInstallationId::new(Uuid::new_v4());
            file.write_all(id.value().to_string().as_bytes())?;
            file.sync_all()?;
            Ok(id)
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            let mut bytes = String::new();
            File::open(path)?.take(37).read_to_string(&mut bytes)?;
            let id =
                Uuid::parse_str(&bytes).map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;
            if bytes.len() != 36 || id.is_nil() {
                return Err(io::ErrorKind::InvalidData.into());
            }
            Ok(ServerInstallationId::new(id))
        }
        Err(error) => Err(error),
    }
}
