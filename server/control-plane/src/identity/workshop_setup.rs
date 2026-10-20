use super::{
    BootstrapInput, BootstrapState, IdentityError, IdentityService, insert_bootstrap_defaults,
    insert_bootstrap_identity, insert_publication_event, map_authentication, validate_display_name,
};
use eitmad_contracts::{
    identity::{AccountId, OrganizationId, ScopeId, TenantId, UserId},
    server::{InviteId, TenantCode},
    setup::{InitializeWorkshop, InitializedWorkshop},
    transport::{CorrelationId, UnixMillis},
};
use eitmad_server_audit::{
    ServerAuditActor, ServerAuditEnvelope, ServerAuditOutcome, append, tenant_scope,
};
use uuid::Uuid;

/// Checks first-owner input before a native installer creates fresh host state.
///
/// # Errors
/// Rejects unsupported versions and invalid workshop names or account credentials.
pub fn validate_workshop_initialization(request: &InitializeWorkshop) -> Result<(), IdentityError> {
    if request.version != 1 {
        return Err(IdentityError::InvalidInput);
    }
    validate_display_name(&request.workshop_name)?;
    crate::authentication::canonical_username(&request.owner_username)
        .map_err(map_authentication)?;
    crate::authentication::validate_password_input(request.owner_password.expose_secret())
        .map_err(map_authentication)
}

impl IdentityService {
    /// Creates the first active Owner/Manager and branch in one audited transaction.
    /// This local deployment operation is never exposed as a network route.
    ///
    /// # Errors
    /// Rejects invalid input, any existing tenant, or failure of a mandatory write or audit.
    pub async fn initialize_workshop(
        &self,
        request: &InitializeWorkshop,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<InitializedWorkshop, IdentityError> {
        validate_workshop_initialization(request)?;
        let canonical_username = crate::authentication::canonical_username(&request.owner_username)
            .map_err(map_authentication)?;
        let password = request.owner_password.clone();
        let hash = tokio::task::spawn_blocking(move || {
            crate::authentication::hash_password(password.expose_secret())
        })
        .await
        .map_err(|_| IdentityError::Unavailable)?
        .map_err(map_authentication)?;
        let tenant_id = TenantId::new(Uuid::new_v4());
        let tenant_code = TenantCode::parse(format!(
            "w-{}",
            &tenant_id.value().simple().to_string()[..30]
        ))
        .map_err(|_| IdentityError::InvalidInput)?;
        let state = BootstrapState {
            tenant_id,
            organization_id: OrganizationId::new(Uuid::new_v4()),
            account_id: AccountId::new(Uuid::new_v4()),
            user_id: UserId::new(Uuid::new_v4()),
            invite_id: InviteId::new(Uuid::new_v4()),
            canonical_username,
            token_hash: [0; 32],
            expires_at: now,
        };
        let branch = ScopeId::new(Uuid::new_v4());
        let input = BootstrapInput {
            tenant_code: tenant_code.clone(),
            tenant_display_name: request.workshop_name.trim().into(),
            organization_display_name: request.workshop_name.trim().into(),
            owner_username: request.owner_username.clone(),
        };
        let mut tx = self.begin_initialization().await?;
        insert_bootstrap_identity(&mut tx, &input, &state, now).await?;
        insert_bootstrap_defaults(&mut tx, &state, now).await?;
        finish_owner(&mut tx, &state, branch, &hash, now).await?;
        insert_publication_event(&mut tx, tenant_id, "eitmad.server.bootstrap-completed.v1",
            serde_json::json!({"tenantId": tenant_id.value(), "organizationId": state.organization_id.value()}), now).await?;
        append(
            &mut tx,
            &ServerAuditEnvelope {
                actor: ServerAuditActor::system(tenant_id),
                scope: tenant_scope(tenant_id),
                operation: "eitmad.server.identity.workshop-initialized.v1",
                outcome: ServerAuditOutcome::Succeeded,
                target_kind: "tenant",
                target_id: Some(tenant_id.value()),
                correlation_id,
                causation_id: None,
                idempotency_key: None,
                redacted_error: None,
                occurred_at: now,
            },
        )
        .await
        .map_err(|_| IdentityError::Unavailable)?;
        tx.commit().await.map_err(|_| IdentityError::Unavailable)?;
        Ok(InitializedWorkshop {
            version: 1,
            tenant_code,
            tenant_id,
            organization_id: state.organization_id,
            branch_id: branch,
            owner_account_id: state.account_id,
        })
    }
}

async fn finish_owner(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &BootstrapState,
    branch: ScopeId,
    hash: &str,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    sqlx::query("UPDATE control.accounts SET status='active',password_hash=$3,activated_at=$4 WHERE tenant_id=$1 AND account_id=$2")
        .bind(state.tenant_id.value()).bind(state.account_id.value()).bind(hash).bind(now.0)
        .execute(&mut **tx).await.map_err(|_| IdentityError::Unavailable)?;
    sqlx::query("INSERT INTO control.branches(tenant_id,branch_id,organization_id,created_at) VALUES($1,$2,$3,$4)")
        .bind(state.tenant_id.value()).bind(branch.value()).bind(state.organization_id.value()).bind(now.0)
        .execute(&mut **tx).await.map_err(|_| IdentityError::Unavailable)?;
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.manager.v1','organization',$3,$4)")
        .bind(state.tenant_id.value()).bind(state.user_id.value()).bind(state.organization_id.value()).bind(now.0)
        .execute(&mut **tx).await.map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "UPDATE control.tenants SET account_revision=account_revision+1 WHERE tenant_id=$1",
    )
    .bind(state.tenant_id.value())
    .execute(&mut **tx)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    Ok(())
}
