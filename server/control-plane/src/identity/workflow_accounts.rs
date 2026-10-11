//! Owner-authorized account invitations and operational role assignment.
use super::{
    IdentityError, IdentityService, OWNER_RELATION, canonical_username, random_activation_token,
};
use eitmad_contracts::{
    accounts::DesktopAccountRole,
    identity::{AccountId, OrganizationId, ScopeId, UserId},
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_server_audit::{
    ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append, tenant_scope,
};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// One-time activation credential. Deliberately has no Debug or serialization.
pub struct AccountInvitation {
    pub account_id: AccountId,
    pub activation_token: String,
}

pub struct WorkflowRole {
    pub organization: OrganizationId,
    pub branch: ScopeId,
    pub role: DesktopAccountRole,
}

impl IdentityService {
    /// Invites one account under an organization and assigns its workflow role atomically.
    ///
    /// # Errors
    /// Denies an expired or non-owner session, an unknown branch, duplicate username,
    /// or a transaction whose required audit cannot commit.
    pub async fn invite_workflow_account(
        &self,
        actor: &AuthenticatedServerSession,
        assignment: &WorkflowRole,
        username: &str,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<AccountInvitation, IdentityError> {
        let WorkflowRole {
            organization,
            branch,
            role,
        } = *assignment;
        let username = canonical_username(username).map_err(|_| IdentityError::InvalidInput)?;
        let mut tx = self
            .workflow_owner(actor, organization, branch, correlation, now)
            .await?;
        let user = Uuid::new_v4();
        let account = AccountId::new(Uuid::new_v4());
        let invite = Uuid::new_v4();
        let token = random_activation_token();
        let hash = self
            .tokens
            .hash(&token)
            .map_err(|_| IdentityError::Unavailable)?;
        sqlx::query("INSERT INTO control.users(tenant_id,user_id,created_at) VALUES($1,$2,$3)")
            .bind(actor.tenant_id.value())
            .bind(user)
            .bind(now.0)
            .execute(&mut *tx)
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        sqlx::query("INSERT INTO control.accounts(tenant_id,account_id,user_id,username,canonical_username,status,created_at)
            VALUES($1,$2,$3,$4,$4,'pending_activation',$5)")
            .bind(actor.tenant_id.value()).bind(account.value()).bind(user).bind(username).bind(now.0)
            .execute(&mut *tx).await.map_err(|_| IdentityError::InvalidInput)?;
        sqlx::query("INSERT INTO control.invitations(tenant_id,invite_id,account_id,token_hash,expires_at,created_at)
            VALUES($1,$2,$3,$4,$5,$6)")
            .bind(actor.tenant_id.value()).bind(invite).bind(account.value()).bind(hash.as_slice())
            .bind(now.0 + eitmad_contracts::server::DEFAULT_INVITE_TTL_MS).bind(now.0)
            .execute(&mut *tx).await.map_err(|_| IdentityError::Unavailable)?;
        sqlx::query("INSERT INTO control.invitation_directory(token_hash,tenant_id,invite_id) VALUES($1,$2,$3)")
            .bind(hash.as_slice()).bind(actor.tenant_id.value()).bind(invite)
            .execute(&mut *tx).await.map_err(|_| IdentityError::Unavailable)?;
        insert_role(
            &mut tx,
            actor,
            organization,
            branch,
            UserId::new(user),
            role,
            now,
        )
        .await?;
        audit(
            &mut tx,
            actor,
            correlation,
            now,
            ServerAuditOutcome::Succeeded,
            Some(account.value()),
        )
        .await?;
        tx.commit().await.map_err(|_| IdentityError::Unavailable)?;
        Ok(AccountInvitation {
            account_id: account,
            activation_token: token,
        })
    }

    /// Assigns an operational role to an existing active account. Owner privilege is separate.
    ///
    /// # Errors
    /// Denies an expired or non-owner session, a foreign account or branch, or failed audit.
    pub async fn grant_workflow_role(
        &self,
        actor: &AuthenticatedServerSession,
        assignment: &WorkflowRole,
        user: UserId,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<(), IdentityError> {
        let WorkflowRole {
            organization,
            branch,
            role,
        } = *assignment;
        let mut tx = self
            .workflow_owner(actor, organization, branch, correlation, now)
            .await?;
        let account: Option<Uuid> = sqlx::query_scalar(
            "SELECT account_id FROM control.accounts
            WHERE tenant_id=$1 AND user_id=$2 AND status='active'",
        )
        .bind(actor.tenant_id.value())
        .bind(user.value())
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
        let account = account.ok_or(IdentityError::InvalidInput)?;
        insert_role(&mut tx, actor, organization, branch, user, role, now).await?;
        audit(
            &mut tx,
            actor,
            correlation,
            now,
            ServerAuditOutcome::Succeeded,
            Some(account),
        )
        .await?;
        tx.commit().await.map_err(|_| IdentityError::Unavailable)
    }

    async fn workflow_owner(
        &self,
        actor: &AuthenticatedServerSession,
        organization: OrganizationId,
        branch: ScopeId,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<Transaction<'_, Postgres>, IdentityError> {
        let mut tx = crate::database::tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        let allowed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM control.relationship_tuples r
            JOIN control.branches b ON b.tenant_id=r.tenant_id AND b.organization_id=r.object_id
            JOIN control.sessions s ON s.tenant_id=r.tenant_id AND s.user_id=r.subject_principal_id
            WHERE r.tenant_id=$1 AND r.subject_principal_id=$2 AND r.subject_kind='user'
            AND r.relation=$3 AND r.object_kind='organization' AND r.object_id=$4 AND b.branch_id=$5
            AND s.session_id=$6 AND s.account_id=$7 AND s.device_id=$8 AND s.revoked_at IS NULL
            AND s.expires_at>$9 AND s.idle_expires_at>$9)",
        )
        .bind(actor.tenant_id.value())
        .bind(actor.user_id.value())
        .bind(OWNER_RELATION)
        .bind(organization.value())
        .bind(branch.value())
        .bind(actor.session_id.value())
        .bind(actor.account_id.value())
        .bind(actor.device_id.value())
        .bind(now.0)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
        if !allowed {
            audit(
                &mut tx,
                actor,
                correlation,
                now,
                ServerAuditOutcome::Denied,
                None,
            )
            .await?;
            tx.commit().await.map_err(|_| IdentityError::Unavailable)?;
            return Err(IdentityError::Denied);
        }
        Ok(tx)
    }
}

async fn insert_role(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    organization: OrganizationId,
    branch: ScopeId,
    user: UserId,
    role: DesktopAccountRole,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    let relation = match role {
        DesktopAccountRole::Manager => "eitmad.relation.organization.manager.v1",
        DesktopAccountRole::Receptionist => "eitmad.relation.organization.receptionist.v1",
    };
    sqlx::query(
        "INSERT INTO control.relationship_tuples
        (tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at)
        VALUES($1,$2,'user',$3,'organization',$4,$6),($1,$2,'user',$3,'branch',$5,$6)
        ON CONFLICT DO NOTHING",
    )
    .bind(actor.tenant_id.value())
    .bind(user.value())
    .bind(relation)
    .bind(organization.value())
    .bind(branch.value())
    .bind(now.0)
    .execute(&mut **tx)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    Ok(())
}

async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    correlation: CorrelationId,
    now: UnixMillis,
    outcome: ServerAuditOutcome,
    target: Option<Uuid>,
) -> Result<(), IdentityError> {
    append(
        tx,
        &ServerAuditEnvelope::from_session(
            actor,
            tenant_scope(actor.tenant_id),
            ServerAuditEvent {
                operation: "eitmad.server.identity.workflow-account.provision.v1",
                outcome,
                target_kind: "account",
                target_id: target,
                correlation_id: correlation,
                causation_id: None,
                idempotency_key: None,
                redacted_error: None,
                occurred_at: now,
            },
        ),
    )
    .await
    .map_err(|_| IdentityError::Unavailable)
}
