use super::*;
use eitmad_contracts::accounts::{CreateDesktopAccount, UpdateDesktopAccount};
use eitmad_contracts::identity::ScopeId;

fn display_name(value: &str) -> Result<&str, ServerAccountError> {
    let value = value.trim();
    if !(1..=128).contains(&value.chars().count())
        || value.chars().any(|c| c.is_control() || matches!(c,
            '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'))
    {
        return Err(ServerAccountError::Invalid);
    }
    Ok(value)
}

pub(super) async fn create(
    tx: &mut Transaction<'_, Postgres>,
    attempt: &Attempt<'_>,
    organization: OrganizationId,
    branch: ScopeId,
    input: &CreateDesktopAccount,
) -> Result<ServerAccountReceipt, ServerAccountError> {
    let assignments = [ServerRoleAssignment {
        role: input.role,
        branch_ids: vec![branch],
    }];
    validate_assignments(tx, attempt.actor.tenant_id, organization, &assignments).await?;
    let name = display_name(&input.display_name)?;
    let username = canonical_username(&input.username).map_err(|_| ServerAccountError::Invalid)?;
    let password = input.password.expose_secret().to_owned();
    let hash = tokio::task::spawn_blocking(move || crate::authentication::hash_password(&password))
        .await
        .map_err(|_| ServerAccountError::Unavailable)?
        .map_err(|_| ServerAccountError::Invalid)?;
    let tenant = attempt.actor.tenant_id;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.accounts WHERE tenant_id=$1 AND canonical_username=$2)")
        .bind(tenant.value()).bind(&username).fetch_one(&mut **tx).await?;
    if exists {
        return Err(ServerAccountError::Invalid);
    }
    let user = UserId::new(Uuid::new_v4());
    let account = AccountId::new(Uuid::new_v4());
    sqlx::query("INSERT INTO control.users(tenant_id,user_id,created_at) VALUES($1,$2,$3)")
        .bind(tenant.value())
        .bind(user.value())
        .bind(attempt.now.0)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO control.accounts(tenant_id,account_id,user_id,username,canonical_username,status,created_at,password_hash,display_name) VALUES($1,$2,$3,$4,$4,'active',$5,$6,$7)")
        .bind(tenant.value()).bind(account.value()).bind(user.value()).bind(username)
        .bind(attempt.now.0).bind(hash).bind(name).execute(&mut **tx).await?;
    replace_assignments(tx, attempt, account, user, organization, &assignments).await?;
    Ok(ServerAccountReceipt {
        account: account_projection::read(tx, tenant, account).await?,
        invitation_id: None,
        invitation_expires_at: None,
    })
}

pub(super) async fn update(
    tx: &mut Transaction<'_, Postgres>,
    attempt: &Attempt<'_>,
    organization: OrganizationId,
    branch: ScopeId,
    input: &UpdateDesktopAccount,
    user: UserId,
) -> Result<(), ServerAccountError> {
    let name = display_name(&input.display_name)?;
    // A thin desktop cannot replace unseen assignments in other organizations or branches.
    let current = account_projection::read(tx, attempt.actor.tenant_id, input.account_id).await?;
    let membership = current
        .organizations
        .iter()
        .find(|o| o.organization_id == organization)
        .ok_or(ServerAccountError::Denied)?;
    if membership.assignments.len() != 1 {
        return Err(ServerAccountError::Invalid);
    }
    let assignments = [ServerRoleAssignment {
        role: input.role,
        branch_ids: if input.role == eitmad_contracts::accounts::DesktopAccountRole::Receptionist {
            let branches = &membership.assignments[0].branch_ids;
            if branches.is_empty() {
                vec![branch]
            } else {
                branches.clone()
            }
        } else {
            membership.assignments[0].branch_ids.clone()
        },
    }];
    validate_assignments(tx, attempt.actor.tenant_id, organization, &assignments).await?;
    replace_assignments(
        tx,
        attempt,
        input.account_id,
        user,
        organization,
        &assignments,
    )
    .await?;
    sqlx::query("UPDATE control.accounts SET display_name=$3 WHERE tenant_id=$1 AND account_id=$2")
        .bind(attempt.actor.tenant_id.value())
        .bind(input.account_id.value())
        .bind(name)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
