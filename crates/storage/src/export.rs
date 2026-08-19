use std::{
    fs::{self, OpenOptions},
    io::Write as _,
    path::Path,
};

use eitmad_contracts::{identity::TenantId, transport::UnixMillis};
use rusqlite::{OptionalExtension as _, params};
use serde::Serialize;
use uuid::Uuid;

use crate::{AuthorityStore, StorageError, make_file_private};

pub const LOCAL_DATA_EXPORT_FORMAT: &str = "eitmad.local-data-export.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalDataExportPolicy {
    pub tenant_scoped: bool,
    pub includes_identity_directory: bool,
    pub includes_configuration: bool,
    pub includes_sessions: bool,
    pub includes_devices: bool,
    pub includes_audit: bool,
    pub includes_secrets: bool,
}

impl LocalDataExportPolicy {
    #[must_use]
    pub const fn portable() -> Self {
        Self {
            tenant_scoped: true,
            includes_identity_directory: true,
            includes_configuration: true,
            includes_sessions: false,
            includes_devices: false,
            includes_audit: false,
            includes_secrets: false,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TenantExport {
    format: &'static str,
    storage_version: u32,
    exported_at: i64,
    tenant_id: String,
    organizations: Vec<String>,
    workspaces: Vec<WorkspaceExport>,
    accounts: Vec<AccountExport>,
    configuration: Vec<ConfigurationExport>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceExport {
    workspace_id: String,
    organization_id: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountExport {
    account_id: String,
    user_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigurationExport {
    scope_kind: String,
    scope_id: String,
    schema_version: u32,
    revision: u64,
    values: Vec<ConfigurationValueExport>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigurationValueExport {
    key: String,
    value: serde_json::Value,
}

impl AuthorityStore {
    /// Writes one portable, tenant-scoped export without sessions, audit, or secrets.
    ///
    /// The destination must not exist. The export is written privately and renamed
    /// atomically only after serialization and flush succeed.
    pub fn export_tenant_data(
        &self,
        tenant_id: TenantId,
        destination: impl AsRef<Path>,
        exported_at: UnixMillis,
    ) -> Result<(), StorageError> {
        let destination = destination.as_ref();
        if destination.exists() {
            return Err(StorageError);
        }
        let export = self.read_transaction(|connection| {
            build_tenant_export(connection, tenant_id, exported_at)
        })?;
        let encoded = serde_json::to_vec_pretty(&export).map_err(|_| StorageError)?;
        let parent = destination.parent().ok_or(StorageError)?;
        fs::create_dir_all(parent).map_err(|_| StorageError)?;
        let temporary = parent.join(format!(".eitmad-export-{}.json", Uuid::new_v4()));
        let result = write_private_new(&temporary, &encoded)
            .and_then(|()| fs::rename(&temporary, destination).map_err(|_| StorageError));
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
}

fn build_tenant_export(
    connection: &rusqlite::Connection,
    tenant_id: TenantId,
    exported_at: UnixMillis,
) -> Result<TenantExport, StorageError> {
    let tenant = tenant_id.value().to_string();
    let exists = connection
        .query_row(
            "SELECT COUNT(*) FROM identity_tenants WHERE tenant_id = ?1",
            [&tenant],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|_| StorageError)?;
    if exists != 1 {
        return Err(StorageError);
    }
    let organizations = collect_column(
        connection,
        "SELECT organization_id FROM identity_organizations
         WHERE tenant_id = ?1 ORDER BY organization_id",
        &tenant,
    )?;
    let workspaces = collect_workspaces(connection, &tenant)?;
    let accounts = collect_accounts(connection, &tenant)?;
    let mut scopes = organizations
        .iter()
        .map(|id| ("organization", id.as_str()))
        .collect::<Vec<_>>();
    scopes.extend(
        workspaces
            .iter()
            .map(|workspace| ("workspace", workspace.workspace_id.as_str())),
    );
    let configuration = scopes
        .into_iter()
        .map(|(kind, id)| collect_configuration(connection, kind, id))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();
    Ok(TenantExport {
        format: LOCAL_DATA_EXPORT_FORMAT,
        storage_version: crate::CURRENT_STORAGE_VERSION,
        exported_at: exported_at.0,
        tenant_id: tenant,
        organizations,
        workspaces,
        accounts,
        configuration,
    })
}

fn collect_column(
    connection: &rusqlite::Connection,
    sql: &str,
    parameter: &str,
) -> Result<Vec<String>, StorageError> {
    let mut statement = connection.prepare(sql).map_err(|_| StorageError)?;
    statement
        .query_map([parameter], |row| row.get(0))
        .map_err(|_| StorageError)?
        .map(|row| row.map_err(|_| StorageError))
        .collect()
}

fn collect_workspaces(
    connection: &rusqlite::Connection,
    tenant: &str,
) -> Result<Vec<WorkspaceExport>, StorageError> {
    let mut statement = connection
        .prepare(
            "SELECT workspace_id, organization_id FROM identity_workspaces
             WHERE tenant_id = ?1 ORDER BY workspace_id",
        )
        .map_err(|_| StorageError)?;
    statement
        .query_map([tenant], |row| {
            Ok(WorkspaceExport {
                workspace_id: row.get(0)?,
                organization_id: row.get(1)?,
            })
        })
        .map_err(|_| StorageError)?
        .map(|row| row.map_err(|_| StorageError))
        .collect()
}

fn collect_accounts(
    connection: &rusqlite::Connection,
    tenant: &str,
) -> Result<Vec<AccountExport>, StorageError> {
    let mut statement = connection
        .prepare(
            "SELECT account_id, user_id FROM identity_accounts
             WHERE tenant_id = ?1 ORDER BY account_id",
        )
        .map_err(|_| StorageError)?;
    statement
        .query_map([tenant], |row| {
            Ok(AccountExport {
                account_id: row.get(0)?,
                user_id: row.get(1)?,
            })
        })
        .map_err(|_| StorageError)?
        .map(|row| row.map_err(|_| StorageError))
        .collect()
}

fn collect_configuration(
    connection: &rusqlite::Connection,
    scope_kind: &str,
    scope_id: &str,
) -> Result<Option<ConfigurationExport>, StorageError> {
    let metadata = connection
        .query_row(
            "SELECT schema_version, revision FROM configuration_scopes
             WHERE scope_kind = ?1 AND scope_id = ?2",
            params![scope_kind, scope_id],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| StorageError)?;
    let Some((schema_version, revision)) = metadata else {
        return Ok(None);
    };
    let revision = u64::try_from(revision).map_err(|_| StorageError)?;
    let mut statement = connection
        .prepare(
            "SELECT config_key, value_json FROM configuration_values
             WHERE scope_kind = ?1 AND scope_id = ?2 ORDER BY config_key",
        )
        .map_err(|_| StorageError)?;
    let values = statement
        .query_map(params![scope_kind, scope_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| StorageError)?
        .map(|row| {
            let (key, encoded) = row.map_err(|_| StorageError)?;
            Ok(ConfigurationValueExport {
                key,
                value: serde_json::from_str(&encoded).map_err(|_| StorageError)?,
            })
        })
        .collect::<Result<Vec<_>, StorageError>>()?;
    Ok(Some(ConfigurationExport {
        scope_kind: scope_kind.to_owned(),
        scope_id: scope_id.to_owned(),
        schema_version,
        revision,
        values,
    }))
}

fn write_private_new(path: &Path, contents: &[u8]) -> Result<(), StorageError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| StorageError)?;
    if make_file_private(path).is_err() {
        let _ = fs::remove_file(path);
        return Err(StorageError);
    }
    file.write_all(contents).map_err(|_| StorageError)?;
    file.sync_all().map_err(|_| StorageError)
}

#[cfg(test)]
mod tests {
    use eitmad_contracts::identity::{AccountId, OrganizationId, UserId, WorkspaceId};
    use tempfile::TempDir;

    use super::*;
    use crate::IdentityTopology;

    fn topology(tenant: u128) -> IdentityTopology {
        IdentityTopology {
            tenant_id: TenantId::new(Uuid::from_u128(tenant)),
            user_id: UserId::new(Uuid::from_u128(tenant + 10)),
            account_id: AccountId::new(Uuid::from_u128(tenant + 20)),
            organization_id: Some(OrganizationId::new(Uuid::from_u128(tenant + 30))),
            workspace_id: Some(WorkspaceId::new(Uuid::from_u128(tenant + 40))),
            created_at: UnixMillis(1),
        }
    }

    #[test]
    fn portable_export_is_atomic_scoped_and_excludes_sensitive_state() {
        let directory = TempDir::new().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let first = topology(1);
        let second = topology(2);
        store.persist_identity_topology(&first).unwrap();
        store.persist_identity_topology(&second).unwrap();

        let destination = directory.path().join("tenant.json");
        store
            .export_tenant_data(first.tenant_id, &destination, UnixMillis(50))
            .unwrap();
        let contents = fs::read_to_string(&destination).unwrap();
        assert!(contents.contains(&first.tenant_id.value().to_string()));
        assert!(contents.contains(&first.account_id.value().to_string()));
        assert!(!contents.contains(&second.tenant_id.value().to_string()));
        assert!(!contents.contains("session"));
        assert!(!contents.contains("audit"));
        assert!(!contents.contains("device"));
        assert!(
            store
                .export_tenant_data(first.tenant_id, &destination, UnixMillis(51))
                .is_err()
        );
        assert_eq!(LocalDataExportPolicy::portable().includes_secrets, false);
    }
}
