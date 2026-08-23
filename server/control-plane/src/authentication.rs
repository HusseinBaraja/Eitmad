use std::time::{SystemTime, UNIX_EPOCH};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use eitmad_contracts::{
    identity::{AccountId, DeviceId, PrincipalId, SessionId, TenantId, UserId},
    server::{
        ActivateAccountRequest, AuthenticatedServerSession, AuthenticationResult, DeviceProof,
        DevicePublicKey, IssuedTokens, LoginRequest, RefreshRequest, SessionPolicy, TokenFamilyId,
    },
    transport::UnixMillis,
};
use hmac::{Hmac, Mac as _};
use rand::RngCore as _;
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Row as _};
use uuid::Uuid;

use crate::{
    audit::{self, AuditEntry},
    database::tenant_transaction,
};

const MAX_DEVICE_PROOF_AGE_MS: i64 = 5 * 60 * 1_000;

#[derive(Clone)]
pub struct TokenKey([u8; 32]);

impl TokenKey {
    #[must_use]
    pub const fn new(value: [u8; 32]) -> Self {
        Self(value)
    }

    /// Decodes a 32-byte URL-safe base64 key.
    ///
    /// # Errors
    ///
    /// Returns [`AuthenticationError::InvalidConfiguration`] for an invalid key.
    pub fn from_base64(value: &str) -> Result<Self, AuthenticationError> {
        let value = URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|_| AuthenticationError::InvalidConfiguration)?;
        let value: [u8; 32] = value
            .try_into()
            .map_err(|_| AuthenticationError::InvalidConfiguration)?;
        Ok(Self(value))
    }
}

impl std::fmt::Debug for TokenKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("TokenKey([REDACTED])")
    }
}

#[derive(Clone)]
pub struct AuthenticationService {
    pool: PgPool,
    tokens: TokenCodec,
    policy: SessionPolicy,
}

#[derive(Clone)]
pub(crate) struct TokenCodec {
    pub(crate) key: TokenKey,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AuthenticationError {
    #[error("authentication failed")]
    Failed,
    #[error("authentication token expired")]
    TokenExpired,
    #[error("authentication token was reused")]
    TokenReuse,
    #[error("device proof is invalid")]
    InvalidDeviceProof,
    #[error("authentication input is invalid")]
    InvalidInput,
    #[error("authentication configuration is invalid")]
    InvalidConfiguration,
    #[error("authentication authority is unavailable")]
    Unavailable,
}

impl AuthenticationService {
    #[must_use]
    pub fn new(pool: PgPool, token_key: TokenKey) -> Self {
        Self {
            pool,
            tokens: TokenCodec { key: token_key },
            policy: SessionPolicy::default(),
        }
    }

    /// Activates one pending account and registers its first device.
    ///
    /// # Errors
    ///
    /// Returns one redacted failure for invalid, expired, or consumed invites.
    pub async fn activate(
        &self,
        request: &ActivateAccountRequest,
        now: UnixMillis,
    ) -> Result<AuthenticationResult, AuthenticationError> {
        let token_hash = self.tokens.hash(&request.invite_token)?;
        let directory = sqlx::query(
            "SELECT tenant_id, invite_id FROM control.invitation_directory WHERE token_hash = $1",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Failed)?;
        let tenant_id = TenantId::new(directory.get("tenant_id"));
        let invite_id: Uuid = directory.get("invite_id");
        let mut transaction = tenant_transaction(&self.pool, tenant_id)
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        let invite = sqlx::query(
            "SELECT i.account_id, i.expires_at, i.consumed_at, a.user_id, a.status
             FROM control.invitations i
             JOIN control.accounts a
               ON a.tenant_id = i.tenant_id AND a.account_id = i.account_id
             WHERE i.tenant_id = $1 AND i.invite_id = $2 AND i.token_hash = $3
             FOR UPDATE OF i, a",
        )
        .bind(tenant_id.value())
        .bind(invite_id)
        .bind(token_hash.as_slice())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Failed)?;
        if invite.get::<i64, _>("expires_at") <= now.0
            || invite.get::<Option<i64>, _>("consumed_at").is_some()
            || invite.get::<String, _>("status") != "pending_activation"
        {
            return Err(AuthenticationError::Failed);
        }
        let password_hash = hash_password(&request.password)?;
        let public_key = decode_device_key(&request.device_public_key)?;
        let account_id = AccountId::new(invite.get("account_id"));
        let user_id = UserId::new(invite.get("user_id"));
        sqlx::query(
            "UPDATE control.accounts
             SET password_hash = $3, status = 'active', activated_at = $4
             WHERE tenant_id = $1 AND account_id = $2",
        )
        .bind(tenant_id.value())
        .bind(account_id.value())
        .bind(password_hash)
        .bind(now.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
        sqlx::query(
            "UPDATE control.invitations SET consumed_at = $3
             WHERE tenant_id = $1 AND invite_id = $2",
        )
        .bind(tenant_id.value())
        .bind(invite_id)
        .bind(now.0)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
        register_device(
            &mut transaction,
            tenant_id,
            account_id,
            request.device_id,
            &public_key,
            &request.device_label,
            now,
        )
        .await?;
        let result = self
            .issue_session(
                &mut transaction,
                tenant_id,
                account_id,
                user_id,
                request.device_id,
                now,
            )
            .await?;
        append_session_audit(
            &mut transaction,
            &result.session,
            "eitmad.server.authentication.activate.v1",
            now,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        Ok(result)
    }

    /// Validates an access token and its device-bound proof.
    ///
    /// # Errors
    ///
    /// Rejects expired, revoked, replayed, or wrongly bound credentials.
    pub async fn authenticate_access(
        &self,
        access_token: &str,
        proof: &DeviceProof,
        now: UnixMillis,
    ) -> Result<AuthenticatedServerSession, AuthenticationError> {
        let token_hash = self.tokens.hash(access_token)?;
        let directory = sqlx::query(
            "SELECT tenant_id FROM control.token_directory
             WHERE token_hash = $1 AND token_kind = 'access'",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Failed)?;
        let tenant_id = TenantId::new(directory.get("tenant_id"));
        let mut transaction = tenant_transaction(&self.pool, tenant_id)
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        let row = sqlx::query(
            "SELECT s.session_id, s.account_id, s.user_id, s.device_id,
                    s.issued_at, s.expires_at, s.idle_expires_at, s.revoked_at,
                    a.expires_at AS access_expires_at, f.revoked_at AS family_revoked_at,
                    d.public_key
             FROM control.access_tokens a
             JOIN control.sessions s
               ON s.tenant_id = a.tenant_id AND s.session_id = a.session_id
             JOIN control.token_families f
               ON f.tenant_id = a.tenant_id AND f.token_family_id = a.token_family_id
             JOIN control.devices d ON d.device_id = s.device_id
             WHERE a.tenant_id = $1 AND a.token_hash = $2
             FOR UPDATE OF s",
        )
        .bind(tenant_id.value())
        .bind(token_hash.as_slice())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Failed)?;
        let device_id = DeviceId::new(row.get("device_id"));
        if proof.device_id != device_id {
            return Err(AuthenticationError::InvalidDeviceProof);
        }
        let expired = row.get::<i64, _>("access_expires_at") <= now.0
            || row.get::<i64, _>("expires_at") <= now.0
            || row.get::<i64, _>("idle_expires_at") <= now.0
            || row.get::<Option<i64>, _>("revoked_at").is_some()
            || row.get::<Option<i64>, _>("family_revoked_at").is_some();
        if expired {
            return Err(AuthenticationError::TokenExpired);
        }
        verify_device_proof(proof, &row.get::<Vec<u8>, _>("public_key"), now)?;
        consume_nonce(&mut transaction, proof, now).await?;
        let session_id = SessionId::new(row.get("session_id"));
        sqlx::query(
            "UPDATE control.sessions
             SET last_seen_at = $3, idle_expires_at = $4
             WHERE tenant_id = $1 AND session_id = $2",
        )
        .bind(tenant_id.value())
        .bind(session_id.value())
        .bind(now.0)
        .bind(now.0 + self.policy.idle_ttl_ms)
        .execute(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        Ok(AuthenticatedServerSession {
            session_id,
            account_id: AccountId::new(row.get("account_id")),
            user_id: UserId::new(row.get("user_id")),
            device_id,
            tenant_id,
            issued_at: UnixMillis(row.get("issued_at")),
            expires_at: UnixMillis(row.get("expires_at")),
        })
    }

    /// Authenticates an active account and enrolls or verifies its device.
    ///
    /// # Errors
    ///
    /// Returns a redacted authentication error. It does not reveal whether the
    /// tenant, username, password, or device caused a denial.
    pub async fn login(
        &self,
        request: &LoginRequest,
        now: UnixMillis,
    ) -> Result<AuthenticationResult, AuthenticationError> {
        validate_password_input(&request.password)?;
        let canonical_username = canonical_username(&request.username)?;
        let tenant = sqlx::query("SELECT tenant_id FROM control.tenants WHERE tenant_code = $1")
            .bind(request.tenant_code.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| AuthenticationError::Unavailable)?
            .ok_or(AuthenticationError::Failed)?;
        let tenant_id = TenantId::new(tenant.get("tenant_id"));
        let mut transaction = tenant_transaction(&self.pool, tenant_id)
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        let account = sqlx::query(
            "SELECT account_id, user_id, status, password_hash, locked_until
             FROM control.accounts
             WHERE tenant_id = $1 AND canonical_username = $2
             FOR UPDATE",
        )
        .bind(tenant_id.value())
        .bind(canonical_username)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Failed)?;
        let status: String = account.get("status");
        let locked_until: Option<i64> = account.get("locked_until");
        if status != "active" || locked_until.is_some_and(|value| value > now.0) {
            return Err(AuthenticationError::Failed);
        }
        let password_hash: Option<String> = account.get("password_hash");
        verify_password(
            &request.password,
            password_hash
                .as_deref()
                .ok_or(AuthenticationError::Failed)?,
        )?;
        let account_id = AccountId::new(account.get("account_id"));
        let user_id = UserId::new(account.get("user_id"));

        let registered_key = sqlx::query(
            "SELECT d.public_key
             FROM control.account_devices ad
             JOIN control.devices d ON d.device_id = ad.device_id
             WHERE ad.tenant_id = $1 AND ad.account_id = $2 AND ad.device_id = $3
               AND ad.revoked_at IS NULL AND d.revoked_at IS NULL",
        )
        .bind(tenant_id.value())
        .bind(account_id.value())
        .bind(request.device_id.value())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;

        if let Some(row) = registered_key {
            let key: Vec<u8> = row.get("public_key");
            let proof = request
                .device_proof
                .as_ref()
                .ok_or(AuthenticationError::InvalidDeviceProof)?;
            verify_device_proof(proof, &key, now)?;
            consume_nonce(&mut transaction, proof, now).await?;
        } else {
            let public_key = decode_device_key(&request.device_public_key)?;
            register_device(
                &mut transaction,
                tenant_id,
                account_id,
                request.device_id,
                &public_key,
                &request.device_label,
                now,
            )
            .await?;
        }

        let result = self
            .issue_session(
                &mut transaction,
                tenant_id,
                account_id,
                user_id,
                request.device_id,
                now,
            )
            .await?;
        append_session_audit(
            &mut transaction,
            &result.session,
            "eitmad.server.authentication.login.v1",
            now,
        )
        .await?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        Ok(result)
    }

    /// Rotates a refresh token and rejects reuse of a consumed token.
    ///
    /// # Errors
    ///
    /// Returns a redacted expiry, reuse, proof, or availability error.
    pub async fn refresh(
        &self,
        request: &RefreshRequest,
        now: UnixMillis,
    ) -> Result<AuthenticationResult, AuthenticationError> {
        let old_hash = self.tokens.hash(&request.refresh_token)?;
        let directory = sqlx::query(
            "SELECT tenant_id FROM control.token_directory
             WHERE token_hash = $1 AND token_kind = 'refresh'",
        )
        .bind(old_hash.as_slice())
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Failed)?;
        let tenant_id = TenantId::new(directory.get("tenant_id"));
        let mut transaction = tenant_transaction(&self.pool, tenant_id)
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        let row = sqlx::query(
            "SELECT r.token_family_id, r.session_id, r.device_id, r.expires_at,
                    r.consumed_at, f.revoked_at, s.account_id, s.user_id,
                    s.expires_at AS session_expires_at, s.idle_expires_at,
                    s.revoked_at AS session_revoked_at, d.public_key
             FROM control.refresh_tokens r
             JOIN control.token_families f
               ON f.tenant_id = r.tenant_id AND f.token_family_id = r.token_family_id
             JOIN control.sessions s
               ON s.tenant_id = r.tenant_id AND s.session_id = r.session_id
             JOIN control.devices d ON d.device_id = r.device_id
             WHERE r.tenant_id = $1 AND r.token_hash = $2
             FOR UPDATE OF r, f, s",
        )
        .bind(tenant_id.value())
        .bind(old_hash.as_slice())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?
        .ok_or(AuthenticationError::Failed)?;
        let family_id: Uuid = row.get("token_family_id");
        if row.get::<Option<i64>, _>("consumed_at").is_some() {
            sqlx::query(
                "UPDATE control.token_families SET revoked_at = $3
                 WHERE tenant_id = $1 AND token_family_id = $2",
            )
            .bind(tenant_id.value())
            .bind(family_id)
            .bind(now.0)
            .execute(&mut *transaction)
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
            transaction
                .commit()
                .await
                .map_err(|_| AuthenticationError::Unavailable)?;
            return Err(AuthenticationError::TokenReuse);
        }
        let expired = row.get::<i64, _>("expires_at") <= now.0
            || row.get::<i64, _>("session_expires_at") <= now.0
            || row.get::<i64, _>("idle_expires_at") <= now.0
            || row.get::<Option<i64>, _>("revoked_at").is_some()
            || row.get::<Option<i64>, _>("session_revoked_at").is_some();
        if expired {
            return Err(AuthenticationError::TokenExpired);
        }
        let device_id = DeviceId::new(row.get("device_id"));
        if request.device_proof.device_id != device_id {
            return Err(AuthenticationError::InvalidDeviceProof);
        }
        verify_device_proof(
            &request.device_proof,
            &row.get::<Vec<u8>, _>("public_key"),
            now,
        )?;
        consume_nonce(&mut transaction, &request.device_proof, now).await?;
        let result = self
            .rotate_refresh(
                &mut transaction,
                RefreshRotation {
                    row: &row,
                    tenant_id,
                    old_hash: &old_hash,
                    family_id: TokenFamilyId::new(family_id),
                    device_id,
                    now,
                },
            )
            .await?;
        transaction
            .commit()
            .await
            .map_err(|_| AuthenticationError::Unavailable)?;
        Ok(result)
    }

    async fn rotate_refresh(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        rotation: RefreshRotation<'_>,
    ) -> Result<AuthenticationResult, AuthenticationError> {
        let session_id = SessionId::new(rotation.row.get("session_id"));
        let tokens = TokenCodec::generate_pair(rotation.now, &self.policy);
        let access_hash = self.tokens.hash(&tokens.access_token)?;
        let refresh_hash = self.tokens.hash(&tokens.refresh_token)?;
        sqlx::query(
            "UPDATE control.refresh_tokens
             SET consumed_at = $3, replaced_by_hash = $4
             WHERE tenant_id = $1 AND token_hash = $2",
        )
        .bind(rotation.tenant_id.value())
        .bind(rotation.old_hash.as_slice())
        .bind(rotation.now.0)
        .bind(refresh_hash.as_slice())
        .execute(&mut **transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
        insert_tokens(
            transaction,
            TokenInsert {
                tenant_id: rotation.tenant_id,
                family_id: rotation.family_id,
                session_id,
                device_id: rotation.device_id,
                tokens: &tokens,
                access_hash: &access_hash,
                refresh_hash: &refresh_hash,
            },
        )
        .await?;
        sqlx::query(
            "UPDATE control.sessions
             SET last_seen_at = $3, idle_expires_at = $4
             WHERE tenant_id = $1 AND session_id = $2",
        )
        .bind(rotation.tenant_id.value())
        .bind(session_id.value())
        .bind(rotation.now.0)
        .bind(rotation.now.0 + self.policy.idle_ttl_ms)
        .execute(&mut **transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
        let session = AuthenticatedServerSession {
            session_id,
            account_id: AccountId::new(rotation.row.get("account_id")),
            user_id: UserId::new(rotation.row.get("user_id")),
            device_id: rotation.device_id,
            tenant_id: rotation.tenant_id,
            issued_at: rotation.now,
            expires_at: UnixMillis(rotation.row.get("session_expires_at")),
        };
        append_session_audit(
            transaction,
            &session,
            "eitmad.server.authentication.refresh.v1",
            rotation.now,
        )
        .await?;
        Ok(AuthenticationResult { session, tokens })
    }

    async fn issue_session(
        &self,
        transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: TenantId,
        account_id: AccountId,
        user_id: UserId,
        device_id: DeviceId,
        now: UnixMillis,
    ) -> Result<AuthenticationResult, AuthenticationError> {
        let session_id = SessionId::new(Uuid::new_v4());
        let token_family_id = TokenFamilyId::new(Uuid::new_v4());
        let tokens = TokenCodec::generate_pair(now, &self.policy);
        let access_hash = self.tokens.hash(&tokens.access_token)?;
        let refresh_hash = self.tokens.hash(&tokens.refresh_token)?;
        let session_expires_at = now.0 + self.policy.refresh_token_ttl_ms;
        sqlx::query(
            "INSERT INTO control.sessions
                (tenant_id, session_id, account_id, user_id, device_id, issued_at,
                 expires_at, idle_expires_at, last_seen_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $6)",
        )
        .bind(tenant_id.value())
        .bind(session_id.value())
        .bind(account_id.value())
        .bind(user_id.value())
        .bind(device_id.value())
        .bind(now.0)
        .bind(session_expires_at)
        .bind(now.0 + self.policy.idle_ttl_ms)
        .execute(&mut **transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
        sqlx::query(
            "INSERT INTO control.token_families
                (tenant_id, token_family_id, session_id)
             VALUES ($1, $2, $3)",
        )
        .bind(tenant_id.value())
        .bind(token_family_id.value())
        .bind(session_id.value())
        .execute(&mut **transaction)
        .await
        .map_err(|_| AuthenticationError::Unavailable)?;
        insert_tokens(
            transaction,
            TokenInsert {
                tenant_id,
                family_id: token_family_id,
                session_id,
                device_id,
                tokens: &tokens,
                access_hash: &access_hash,
                refresh_hash: &refresh_hash,
            },
        )
        .await?;
        Ok(AuthenticationResult {
            session: AuthenticatedServerSession {
                session_id,
                account_id,
                user_id,
                device_id,
                tenant_id,
                issued_at: now,
                expires_at: UnixMillis(session_expires_at),
            },
            tokens,
        })
    }
}

async fn append_session_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: &AuthenticatedServerSession,
    operation: &str,
    now: UnixMillis,
) -> Result<(), AuthenticationError> {
    audit::append(
        transaction,
        AuditEntry {
            tenant_id: session.tenant_id,
            session_id: session.session_id,
            device_id: Some(session.device_id),
            principal_id: PrincipalId::new(session.user_id.value()),
            operation,
            outcome: "succeeded",
            target_kind: "session",
            now,
        },
    )
    .await
    .map_err(|_| AuthenticationError::Unavailable)
}

impl TokenCodec {
    pub(crate) fn hash(&self, token: &str) -> Result<[u8; 32], AuthenticationError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key.0)
            .map_err(|_| AuthenticationError::InvalidConfiguration)?;
        mac.update(token.as_bytes());
        Ok(mac.finalize().into_bytes().into())
    }

    fn generate_pair(now: UnixMillis, policy: &SessionPolicy) -> IssuedTokens {
        IssuedTokens {
            access_token: random_token(),
            refresh_token: random_token(),
            access_expires_at: UnixMillis(now.0 + policy.access_token_ttl_ms),
            refresh_expires_at: UnixMillis(now.0 + policy.refresh_token_ttl_ms),
        }
    }
}

async fn register_device(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant_id: TenantId,
    account_id: AccountId,
    device_id: DeviceId,
    public_key: &[u8],
    label: &str,
    now: UnixMillis,
) -> Result<(), AuthenticationError> {
    let device = sqlx::query(
        "INSERT INTO control.devices
            (device_id, algorithm, public_key, label, created_at)
         VALUES ($1, 'ed25519', $2, $3, $4)
         ON CONFLICT (device_id) DO UPDATE SET device_id = EXCLUDED.device_id
         WHERE control.devices.algorithm = 'ed25519'
           AND control.devices.public_key = EXCLUDED.public_key
           AND control.devices.revoked_at IS NULL
         RETURNING device_id",
    )
    .bind(device_id.value())
    .bind(public_key)
    .bind(label)
    .bind(now.0)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| AuthenticationError::Unavailable)?;
    if device.is_none() {
        return Err(AuthenticationError::InvalidDeviceProof);
    }
    let association = sqlx::query(
        "INSERT INTO control.account_devices
            (tenant_id, account_id, device_id, registered_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (tenant_id, account_id, device_id)
         DO UPDATE SET registered_at = control.account_devices.registered_at
         WHERE control.account_devices.revoked_at IS NULL
         RETURNING device_id",
    )
    .bind(tenant_id.value())
    .bind(account_id.value())
    .bind(device_id.value())
    .bind(now.0)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(|_| AuthenticationError::Unavailable)?;
    association
        .is_some()
        .then_some(())
        .ok_or(AuthenticationError::InvalidDeviceProof)
}

struct RefreshRotation<'a> {
    row: &'a sqlx::PgRow,
    tenant_id: TenantId,
    old_hash: &'a [u8; 32],
    family_id: TokenFamilyId,
    device_id: DeviceId,
    now: UnixMillis,
}

struct TokenInsert<'a> {
    tenant_id: TenantId,
    family_id: TokenFamilyId,
    session_id: SessionId,
    device_id: DeviceId,
    tokens: &'a IssuedTokens,
    access_hash: &'a [u8; 32],
    refresh_hash: &'a [u8; 32],
}

async fn insert_tokens(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    insert: TokenInsert<'_>,
) -> Result<(), AuthenticationError> {
    sqlx::query(
        "INSERT INTO control.access_tokens
            (tenant_id, token_hash, token_family_id, session_id, expires_at)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(insert.tenant_id.value())
    .bind(insert.access_hash.as_slice())
    .bind(insert.family_id.value())
    .bind(insert.session_id.value())
    .bind(insert.tokens.access_expires_at.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| AuthenticationError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.refresh_tokens
            (tenant_id, token_hash, token_family_id, session_id, device_id, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(insert.tenant_id.value())
    .bind(insert.refresh_hash.as_slice())
    .bind(insert.family_id.value())
    .bind(insert.session_id.value())
    .bind(insert.device_id.value())
    .bind(insert.tokens.refresh_expires_at.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| AuthenticationError::Unavailable)?;
    sqlx::query(
        "INSERT INTO control.token_directory (token_hash, tenant_id, token_kind)
         VALUES ($1, $2, 'access'), ($3, $2, 'refresh')",
    )
    .bind(insert.access_hash.as_slice())
    .bind(insert.tenant_id.value())
    .bind(insert.refresh_hash.as_slice())
    .execute(&mut **transaction)
    .await
    .map_err(|_| AuthenticationError::Unavailable)?;
    Ok(())
}

async fn consume_nonce(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    proof: &DeviceProof,
    now: UnixMillis,
) -> Result<(), AuthenticationError> {
    let nonce_hash: [u8; 32] = Sha256::digest(proof.nonce.as_bytes()).into();
    let inserted = sqlx::query(
        "INSERT INTO control.device_nonces
            (device_id, nonce_hash, expires_at, consumed_at)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (device_id, nonce_hash) DO NOTHING",
    )
    .bind(proof.device_id.value())
    .bind(nonce_hash.as_slice())
    .bind(now.0 + MAX_DEVICE_PROOF_AGE_MS)
    .bind(now.0)
    .execute(&mut **transaction)
    .await
    .map_err(|_| AuthenticationError::Unavailable)?;
    (inserted.rows_affected() == 1)
        .then_some(())
        .ok_or(AuthenticationError::InvalidDeviceProof)
}

pub(crate) fn canonical_username(value: &str) -> Result<String, AuthenticationError> {
    use unicode_normalization::UnicodeNormalization as _;

    let trimmed = value.trim();
    let invalid = trimmed.is_empty()
        || trimmed.chars().count() > 128
        || trimmed.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '\u{061c}'
                        | '\u{200e}'
                        | '\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{2069}'
                )
        });
    if invalid {
        return Err(AuthenticationError::InvalidInput);
    }
    Ok(trimmed
        .nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>())
}

pub(crate) fn hash_password(password: &str) -> Result<String, AuthenticationError> {
    validate_password_input(password)?;
    let mut salt = [0_u8; 16];
    rand::rng().fill_bytes(&mut salt);
    let salt = SaltString::encode_b64(&salt).map_err(|_| AuthenticationError::Unavailable)?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|value| value.to_string())
        .map_err(|_| AuthenticationError::Unavailable)
}

fn verify_password(password: &str, encoded: &str) -> Result<(), AuthenticationError> {
    let hash = PasswordHash::new(encoded).map_err(|_| AuthenticationError::Failed)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .map_err(|_| AuthenticationError::Failed)
}

fn validate_password_input(password: &str) -> Result<(), AuthenticationError> {
    let length = password.chars().count();
    ((12..=256).contains(&length) && password.len() <= 1_024)
        .then_some(())
        .ok_or(AuthenticationError::InvalidInput)
}

fn decode_device_key(key: &DevicePublicKey) -> Result<Vec<u8>, AuthenticationError> {
    if key.algorithm != "ed25519" {
        return Err(AuthenticationError::InvalidDeviceProof);
    }
    let decoded = URL_SAFE_NO_PAD
        .decode(&key.base64)
        .map_err(|_| AuthenticationError::InvalidDeviceProof)?;
    let _: [u8; 32] = decoded
        .clone()
        .try_into()
        .map_err(|_| AuthenticationError::InvalidDeviceProof)?;
    Ok(decoded)
}

fn verify_device_proof(
    proof: &DeviceProof,
    public_key: &[u8],
    now: UnixMillis,
) -> Result<(), AuthenticationError> {
    let skew = now
        .0
        .checked_sub(proof.issued_at.0)
        .and_then(i64::checked_abs);
    if proof.nonce.len() < 16
        || proof.nonce.len() > 256
        || !skew.is_some_and(|value| value <= MAX_DEVICE_PROOF_AGE_MS)
    {
        return Err(AuthenticationError::InvalidDeviceProof);
    }
    let public_key: [u8; 32] = public_key
        .try_into()
        .map_err(|_| AuthenticationError::InvalidDeviceProof)?;
    let signature: [u8; 64] = URL_SAFE_NO_PAD
        .decode(&proof.signature_base64)
        .map_err(|_| AuthenticationError::InvalidDeviceProof)?
        .try_into()
        .map_err(|_| AuthenticationError::InvalidDeviceProof)?;
    let message = format!(
        "{}\n{}\n{}",
        proof.device_id.value(),
        proof.issued_at.0,
        proof.nonce
    );
    VerifyingKey::from_bytes(&public_key)
        .map_err(|_| AuthenticationError::InvalidDeviceProof)?
        .verify(message.as_bytes(), &Signature::from_bytes(&signature))
        .map_err(|_| AuthenticationError::InvalidDeviceProof)
}

fn random_token() -> String {
    let mut value = [0_u8; 32];
    rand::rng().fill_bytes(&mut value);
    URL_SAFE_NO_PAD.encode(value)
}

#[must_use]
pub fn unix_millis_now() -> UnixMillis {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    UnixMillis(i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer as _, SigningKey};

    use super::*;

    #[test]
    fn usernames_preserve_arabic_and_reject_direction_controls() {
        assert_eq!(canonical_username("  حسين Admin  ").unwrap(), "حسين admin");
        assert!(canonical_username("admin\u{202e}txt").is_err());
    }

    #[test]
    fn password_hashes_verify_without_exposing_the_password() {
        let password = "synthetic-آمن-123";
        let encoded = hash_password(password).unwrap();
        assert!(!encoded.contains(password));
        assert!(verify_password(password, &encoded).is_ok());
        assert!(verify_password("synthetic-wrong-123", &encoded).is_err());
    }

    #[test]
    fn token_hashes_are_keyed_and_token_key_debug_is_redacted() {
        let left = TokenCodec {
            key: TokenKey::new([1; 32]),
        };
        let right = TokenCodec {
            key: TokenKey::new([2; 32]),
        };
        assert_ne!(left.hash("token").unwrap(), right.hash("token").unwrap());
        assert_eq!(format!("{:?}", left.key), "TokenKey([REDACTED])");
    }

    #[test]
    fn valid_device_proof_is_bound_to_device_time_and_nonce() {
        let signing = SigningKey::from_bytes(&[7; 32]);
        let device_id = DeviceId::new(Uuid::from_u128(8));
        let issued_at = UnixMillis(1_000_000);
        let nonce = "synthetic-proof-nonce";
        let message = format!("{}\n{}\n{}", device_id.value(), issued_at.0, nonce);
        let signature = signing.sign(message.as_bytes());
        let proof = DeviceProof {
            device_id,
            nonce: nonce.to_owned(),
            issued_at,
            signature_base64: URL_SAFE_NO_PAD.encode(signature.to_bytes()),
        };
        assert!(verify_device_proof(&proof, signing.verifying_key().as_bytes(), issued_at).is_ok());
        assert!(
            verify_device_proof(
                &proof,
                signing.verifying_key().as_bytes(),
                UnixMillis(2_000_000)
            )
            .is_err()
        );
    }

    #[test]
    fn extreme_proof_timestamps_reject_without_overflow() {
        let signing = SigningKey::from_bytes(&[7; 32]);
        let device_id = DeviceId::new(Uuid::from_u128(8));
        let nonce = "synthetic-proof-nonce";
        for issued_at in [UnixMillis(i64::MIN), UnixMillis(i64::MAX)] {
            let message = format!("{}\n{}\n{}", device_id.value(), issued_at.0, nonce);
            let signature = signing.sign(message.as_bytes());
            let proof = DeviceProof {
                device_id,
                nonce: nonce.to_owned(),
                issued_at,
                signature_base64: URL_SAFE_NO_PAD.encode(signature.to_bytes()),
            };
            assert_eq!(
                verify_device_proof(&proof, signing.verifying_key().as_bytes(), unix_millis_now()),
                Err(AuthenticationError::InvalidDeviceProof)
            );
        }
    }
}
