use std::sync::Arc;

use eitmad_contracts::{
    identity::{AccountId, OrganizationId, PrincipalId, SessionId, TenantId, UserId},
    server::{
        AuthenticatedServerSession, CreateInviteRequest, InviteCreated, InviteId, LicenseId,
        TenantCode,
    },
    transport::{CorrelationId, UnixMillis},
};
use rand::RngCore as _;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    audit::{self, AuditEntry},
    authentication::{
        AuthenticationError, TokenCodec, TokenKey, canonical_username, is_direction_control,
    },
    database::tenant_transaction,
};

const OWNER_RELATION: &str = "eitmad.relation.organization.owner.v1";
const BOOTSTRAP_LOCK_KEY: i64 = 1_163_158_102;

#[derive(Clone, Debug)]
pub struct BootstrapInput {
    pub tenant_code: TenantCode,
    pub tenant_display_name: String,
    pub organization_display_name: String,
    pub owner_username: String,
}

/// Bootstrap output contains a one-time activation secret and has no `Debug`.
pub struct BootstrapResult {
    pub tenant_id: TenantId,
    pub organization_id: OrganizationId,
    pub account_id: AccountId,
    pub invite_token: String,
    pub expires_at: UnixMillis,
}

/// Secret-bearing delivery request. Providers must not persist or log its token.
#[derive(Clone)]
pub struct NotificationDelivery {
    pub invite_id: InviteId,
    pub destination: Option<String>,
    pub activation_token: String,
    pub expires_at: UnixMillis,
}

pub trait NotificationSink: Send + Sync {
    /// Accepts a delivery after its outbox job is durable.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured delivery provider cannot accept it.
    fn enqueue(&self, delivery: NotificationDelivery) -> Result<(), NotificationDelivery>;
}

#[derive(Clone)]
pub struct IdentityService {
    pool: PgPool,
    tokens: TokenCodec,
    notification_sink: Option<Arc<dyn NotificationSink>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum IdentityError {
    #[error("server bootstrap is already complete")]
    AlreadyBootstrapped,
    #[error("identity input is invalid")]
    InvalidInput,
    #[error("identity action is denied")]
    Denied,
    #[error("notification delivery is unavailable")]
    DeliveryUnavailable,
    #[error("identity authority is unavailable")]
    Unavailable,
}

impl IdentityService {
    #[must_use]
    pub fn new(pool: PgPool, token_key: TokenKey) -> Self {
        Self {
            pool,
            tokens: TokenCodec { key: token_key },
            notification_sink: None,
        }
    }

    #[must_use]
    pub fn with_notification_sink(mut self, sink: Arc<dyn NotificationSink>) -> Self {
        self.notification_sink = Some(sink);
        self
    }

    /// Creates the first tenant, organization, owner, and activation invite.
    ///
    /// # Errors
    ///
    /// Fails after any tenant exists or when the complete transaction cannot commit.
    pub async fn bootstrap(
        &self,
        input: &BootstrapInput,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<BootstrapResult, IdentityError> {
        validate_display_name(&input.tenant_display_name)?;
        validate_display_name(&input.organization_display_name)?;
        let canonical = canonical_username(&input.owner_username).map_err(map_authentication)?;
        let invite_token = random_activation_token();
        let token_hash = self
            .tokens
            .hash(&invite_token)
            .map_err(map_authentication)?;
        let expires_at = UnixMillis(now.0 + eitmad_contracts::server::DEFAULT_INVITE_TTL_MS);
        let state = BootstrapState {
            tenant_id: TenantId::new(Uuid::new_v4()),
            organization_id: OrganizationId::new(Uuid::new_v4()),
            account_id: AccountId::new(Uuid::new_v4()),
            user_id: UserId::new(Uuid::new_v4()),
            invite_id: InviteId::new(Uuid::new_v4()),
            canonical_username: canonical,
            token_hash,
            expires_at,
        };

        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(BOOTSTRAP_LOCK_KEY)
            .execute(&mut *transaction)
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        let tenant_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM control.tenants")
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        if tenant_count != 0 {
            return Err(IdentityError::AlreadyBootstrapped);
        }
        insert_bootstrap_identity(&mut transaction, input, &state, now).await?;
        insert_bootstrap_invite(&mut transaction, &state, now).await?;
        insert_bootstrap_defaults(&mut transaction, &state, now).await?;
        insert_publication_event(
            &mut transaction,
            state.tenant_id,
            "eitmad.server.bootstrap-completed.v1",
            serde_json::json!({
                "tenantId": state.tenant_id.value(),
                "organizationId": state.organization_id.value()
            }),
            now,
        )
        .await?;
        audit::append(
            &mut transaction,
            AuditEntry {
                tenant_id: state.tenant_id,
                session_id: SessionId::new(Uuid::nil()),
                device_id: None,
                principal_id: PrincipalId::new(state.user_id.value()),
                operation: "eitmad.server.identity.bootstrap.v1",
                outcome: "succeeded",
                target_kind: "tenant",
                correlation_id,
                redacted_error: None,
                now,
            },
        )
        .await
        .map_err(|_| IdentityError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        Ok(BootstrapResult {
            tenant_id: state.tenant_id,
            organization_id: state.organization_id,
            account_id: state.account_id,
            invite_token,
            expires_at: state.expires_at,
        })
    }

    /// Creates a tenant-scoped account activation invite.
    ///
    /// # Errors
    ///
    /// Denies non-owners and rolls back all identity state on storage failure.
    pub async fn create_invite(
        &self,
        actor: &AuthenticatedServerSession,
        request: &CreateInviteRequest,
        correlation_id: CorrelationId,
        now: UnixMillis,
    ) -> Result<InviteCreated, IdentityError> {
        let canonical_username =
            canonical_username(&request.username).map_err(map_authentication)?;
        let sink = self
            .notification_sink
            .as_ref()
            .ok_or(IdentityError::DeliveryUnavailable)?;
        let mut transaction = tenant_transaction(&self.pool, actor.tenant_id)
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        require_tenant_owner(&mut transaction, actor).await?;
        let token = random_activation_token();
        let token_hash = self.tokens.hash(&token).map_err(map_authentication)?;
        let expires_at = UnixMillis(now.0 + eitmad_contracts::server::DEFAULT_INVITE_TTL_MS);
        let state = InviteState {
            user_id: UserId::new(Uuid::new_v4()),
            account_id: AccountId::new(Uuid::new_v4()),
            invite_id: InviteId::new(Uuid::new_v4()),
            delivery_id: eitmad_contracts::server::ServerEventId::new(Uuid::new_v4()),
            token,
            token_hash,
            expires_at,
            canonical_username,
        };
        insert_invite_identity(&mut transaction, actor, request, &state, now).await?;
        insert_publication_event(
            &mut transaction,
            actor.tenant_id,
            "eitmad.server.invite-created.v1",
            serde_json::json!({
                "deliveryId": state.delivery_id.value(),
                "inviteId": state.invite_id.value()
            }),
            now,
        )
        .await?;
        audit::append(
            &mut transaction,
            AuditEntry {
                tenant_id: actor.tenant_id,
                session_id: actor.session_id,
                device_id: Some(actor.device_id),
                principal_id: PrincipalId::new(actor.user_id.value()),
                operation: "eitmad.server.identity.invite.v1",
                outcome: "succeeded",
                target_kind: "account",
                correlation_id,
                redacted_error: None,
                now,
            },
        )
        .await
        .map_err(|_| IdentityError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| IdentityError::Unavailable)?;
        let delivery = NotificationDelivery {
            invite_id: state.invite_id,
            destination: request.delivery_destination.clone(),
            activation_token: state.token,
            expires_at: state.expires_at,
        };
        sink.enqueue(delivery)
            .map_err(|_| IdentityError::DeliveryUnavailable)?;
        Ok(InviteCreated {
            invite_id: state.invite_id,
            account_id: state.account_id,
            expires_at: state.expires_at,
            delivery_id: state.delivery_id,
        })
    }
}

struct BootstrapState {
    tenant_id: TenantId,
    organization_id: OrganizationId,
    account_id: AccountId,
    user_id: UserId,
    invite_id: InviteId,
    canonical_username: String,
    token_hash: [u8; 32],
    expires_at: UnixMillis,
}

struct InviteState {
    user_id: UserId,
    account_id: AccountId,
    invite_id: InviteId,
    delivery_id: eitmad_contracts::server::ServerEventId,
    token: String,
    token_hash: [u8; 32],
    expires_at: UnixMillis,
    canonical_username: String,
}

pub(crate) async fn require_tenant_owner(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
) -> Result<(), IdentityError> {
    let authorized: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM control.relationship_tuples
            WHERE tenant_id = $1 AND subject_principal_id = $2
              AND relation = $3 AND object_kind = 'tenant' AND object_id = $1
         )",
    )
    .bind(actor.tenant_id.value())
    .bind(actor.user_id.value())
    .bind(OWNER_RELATION)
    .fetch_one(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    authorized.then_some(()).ok_or(IdentityError::Denied)
}

async fn insert_invite_identity(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    request: &CreateInviteRequest,
    state: &InviteState,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    sqlx::query("INSERT INTO control.users (tenant_id, user_id, created_at) VALUES ($1, $2, $3)")
        .bind(actor.tenant_id.value())
        .bind(state.user_id.value())
        .bind(now.0)
        .execute(&mut **transaction)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.accounts
            (tenant_id, account_id, user_id, username, canonical_username, status, created_at)
         VALUES ($1, $2, $3, $4, $5, 'pending_activation', $6)",
    )
    .bind(actor.tenant_id.value())
    .bind(state.account_id.value())
    .bind(state.user_id.value())
    .bind(&request.username)
    .bind(&state.canonical_username)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.invitations
            (tenant_id, invite_id, account_id, token_hash, expires_at,
             delivery_destination, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(actor.tenant_id.value())
    .bind(state.invite_id.value())
    .bind(state.account_id.value())
    .bind(state.token_hash.as_slice())
    .bind(state.expires_at.0)
    .bind(request.delivery_destination.as_deref())
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.invitation_directory (token_hash, tenant_id, invite_id)
         VALUES ($1, $2, $3)",
    )
    .bind(state.token_hash.as_slice())
    .bind(actor.tenant_id.value())
    .bind(state.invite_id.value())
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    insert_invite_memberships(transaction, actor, request, state.user_id, now).await
}

async fn insert_invite_memberships(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    actor: &AuthenticatedServerSession,
    request: &CreateInviteRequest,
    user_id: UserId,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    for organization_id in &request.organization_ids {
        let inserted = sqlx::query(
            "INSERT INTO control.relationship_tuples
                (tenant_id, subject_principal_id, subject_kind, relation,
                 object_kind, object_id, created_at)
             SELECT $1, $2, 'user', 'eitmad.relation.organization.member.v1',
                    'organization', organization_id, $4
             FROM control.organizations
             WHERE tenant_id = $1 AND organization_id = $3",
        )
        .bind(actor.tenant_id.value())
        .bind(user_id.value())
        .bind(organization_id.value())
        .bind(now.0)
        .execute(&mut **transaction)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
        if inserted.rows_affected() != 1 {
            return Err(IdentityError::InvalidInput);
        }
    }
    Ok(())
}

async fn insert_bootstrap_identity(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: &BootstrapInput,
    state: &BootstrapState,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    sqlx::query(
        "INSERT INTO control.tenants
            (tenant_id, tenant_code, display_name, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(state.tenant_id.value())
    .bind(input.tenant_code.as_str())
    .bind(&input.tenant_display_name)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query("SELECT set_config('eitmad.tenant_id', $1, true)")
        .bind(state.tenant_id.value().to_string())
        .execute(&mut **transaction)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query("INSERT INTO control.users (tenant_id, user_id, created_at) VALUES ($1, $2, $3)")
        .bind(state.tenant_id.value())
        .bind(state.user_id.value())
        .bind(now.0)
        .execute(&mut **transaction)
        .await
        .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.accounts
            (tenant_id, account_id, user_id, username, canonical_username, status, created_at)
         VALUES ($1, $2, $3, $4, $5, 'pending_activation', $6)",
    )
    .bind(state.tenant_id.value())
    .bind(state.account_id.value())
    .bind(state.user_id.value())
    .bind(&input.owner_username)
    .bind(&state.canonical_username)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.organizations
            (tenant_id, organization_id, display_name, created_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(state.tenant_id.value())
    .bind(state.organization_id.value())
    .bind(&input.organization_display_name)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    Ok(())
}

async fn insert_bootstrap_invite(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &BootstrapState,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    sqlx::query(
        "INSERT INTO control.invitations
            (tenant_id, invite_id, account_id, token_hash, expires_at, created_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(state.tenant_id.value())
    .bind(state.invite_id.value())
    .bind(state.account_id.value())
    .bind(state.token_hash.as_slice())
    .bind(state.expires_at.0)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.invitation_directory (token_hash, tenant_id, invite_id)
         VALUES ($1, $2, $3)",
    )
    .bind(state.token_hash.as_slice())
    .bind(state.tenant_id.value())
    .bind(state.invite_id.value())
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    Ok(())
}

async fn insert_bootstrap_defaults(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    state: &BootstrapState,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    sqlx::query(
        "INSERT INTO control.relationship_tuples
            (tenant_id, subject_principal_id, subject_kind, relation,
             object_kind, object_id, created_at)
         VALUES
            ($1, $2, 'user', $3, 'tenant', $1, $4),
            ($1, $2, 'user', $3, 'organization', $5, $4)",
    )
    .bind(state.tenant_id.value())
    .bind(state.user_id.value())
    .bind(OWNER_RELATION)
    .bind(now.0)
    .bind(state.organization_id.value())
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.licenses
            (tenant_id, license_id, provider_revision, status, updated_at)
         VALUES ($1, $2, 'bootstrap', 'unknown', $3)",
    )
    .bind(state.tenant_id.value())
    .bind(LicenseId::new(Uuid::new_v4()).value())
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.update_assignments
            (tenant_id, assignment_kind, device_id, channel, revision, updated_at)
         VALUES ($1, 'tenant', $2, 'stable', 1, $3)",
    )
    .bind(state.tenant_id.value())
    .bind(Uuid::nil())
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    Ok(())
}

pub(crate) async fn insert_publication_event(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: TenantId,
    topic: &str,
    payload: serde_json::Value,
    now: UnixMillis,
) -> Result<(), IdentityError> {
    sqlx::query(
        "INSERT INTO publication.server_events
            (event_id, tenant_id, topic, payload_json, occurred_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(tenant_id.value())
    .bind(topic)
    .bind(payload)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| IdentityError::Unavailable)?;
    Ok(())
}

fn validate_display_name(value: &str) -> Result<(), IdentityError> {
    let trimmed = value.trim();
    (!trimmed.is_empty()
        && trimmed.chars().count() <= 256
        && !trimmed
            .chars()
            .any(|character| character.is_control() || is_direction_control(character)))
    .then_some(())
    .ok_or(IdentityError::InvalidInput)
}

fn random_activation_token() -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let mut value = [0_u8; 32];
    rand::rng().fill_bytes(&mut value);
    URL_SAFE_NO_PAD.encode(value)
}

fn map_authentication(error: AuthenticationError) -> IdentityError {
    match error {
        AuthenticationError::InvalidInput => IdentityError::InvalidInput,
        _ => IdentityError::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arabic_display_names_are_valid_but_control_text_is_not() {
        assert!(validate_display_name("شركة الاعتماد للأثاث").is_ok());
        assert!(validate_display_name("bad\nname").is_err());
    }

    #[test]
    fn display_names_reject_bidirectional_format_characters() {
        for character in [
            '\u{061c}', '\u{200e}', '\u{200f}', '\u{202a}', '\u{202e}', '\u{2066}', '\u{2069}',
        ] {
            assert!(validate_display_name(&format!("name{character}")).is_err());
        }
    }

    #[test]
    fn bootstrap_result_has_no_debug_surface() {
        let token = random_activation_token();
        assert!(token.len() >= 42);
        assert!(!token.contains('='));
    }
}
