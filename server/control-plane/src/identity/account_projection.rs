use eitmad_contracts::{
    accounts::DesktopAccountRole,
    identity::{AccountId, OrganizationId, ScopeId, TenantId, UserId},
    server_accounts::{
        ServerAccount, ServerAccountOrganization, ServerAccountStatus, ServerRoleAssignment,
    },
};
use sqlx::{Postgres, Row as _, Transaction};

use super::{OWNER_RELATION, accounts::ServerAccountError};

pub(super) const MANAGER: &str = "eitmad.relation.organization.manager.v1";
pub(super) const RECEPTIONIST: &str = "eitmad.relation.organization.receptionist.v1";

pub(super) const fn relation(role: DesktopAccountRole) -> &'static str {
    match role {
        DesktopAccountRole::Manager => MANAGER,
        DesktopAccountRole::Receptionist => RECEPTIONIST,
    }
}

pub(super) async fn read(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    account: AccountId,
) -> Result<ServerAccount, ServerAccountError> {
    let row = sqlx::query("SELECT user_id,username,status,revision FROM control.accounts WHERE tenant_id=$1 AND account_id=$2")
        .bind(tenant.value()).bind(account.value()).fetch_optional(&mut **tx).await?
        .ok_or(ServerAccountError::Denied)?;
    let user = UserId::new(row.get("user_id"));
    let status = match row.get::<String, _>("status").as_str() {
        "active" => ServerAccountStatus::Active,
        "pending_activation" => ServerAccountStatus::PendingActivation,
        "disabled" => ServerAccountStatus::Disabled,
        "locked" => ServerAccountStatus::Locked,
        _ => return Err(ServerAccountError::Unavailable),
    };
    let ids: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT organization_id FROM control.account_organizations WHERE tenant_id=$1 AND account_id=$2 ORDER BY organization_id")
        .bind(tenant.value()).bind(account.value()).fetch_all(&mut **tx).await?;
    let mut organizations = Vec::new();
    for id in ids {
        organizations.push(organization(tx, tenant, user, OrganizationId::new(id)).await?);
    }
    Ok(ServerAccount {
        tenant_id: tenant,
        account_id: account,
        user_id: user,
        username: row.get("username"),
        status,
        revision: u64::try_from(row.get::<i64, _>("revision"))
            .map_err(|_| ServerAccountError::Unavailable)?,
        organizations,
    })
}

async fn organization(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    user: UserId,
    organization: OrganizationId,
) -> Result<ServerAccountOrganization, ServerAccountError> {
    let rows = sqlx::query("SELECT relation FROM control.relationship_tuples WHERE tenant_id=$1 AND subject_principal_id=$2 AND subject_kind='user' AND object_kind='organization' AND object_id=$3 ORDER BY relation")
        .bind(tenant.value()).bind(user.value()).bind(organization.value()).fetch_all(&mut **tx).await?;
    let mut result = ServerAccountOrganization {
        organization_id: organization,
        owner: false,
        assignments: Vec::new(),
    };
    for row in rows {
        let role = match row.get::<String, _>("relation").as_str() {
            OWNER_RELATION => {
                result.owner = true;
                continue;
            }
            MANAGER => DesktopAccountRole::Manager,
            RECEPTIONIST => DesktopAccountRole::Receptionist,
            _ => continue,
        };
        let branches: Vec<uuid::Uuid> = sqlx::query_scalar("SELECT r.object_id FROM control.relationship_tuples r JOIN control.branches b ON b.tenant_id=r.tenant_id AND b.branch_id=r.object_id WHERE r.tenant_id=$1 AND r.subject_principal_id=$2 AND r.subject_kind='user' AND r.object_kind='branch' AND r.relation=$3 AND b.organization_id=$4 ORDER BY r.object_id")
            .bind(tenant.value()).bind(user.value()).bind(relation(role)).bind(organization.value()).fetch_all(&mut **tx).await?;
        result.assignments.push(ServerRoleAssignment {
            role,
            branch_ids: branches.into_iter().map(ScopeId::new).collect(),
        });
    }
    Ok(result)
}
