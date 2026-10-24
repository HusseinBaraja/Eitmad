//! Durable Owner admission, bounded PAKE attempts, and atomic device approval.
use crate::database::tenant_transaction;
use eitmad_contracts::{
    identity::TenantId,
    pairing::{
        CompletePairing, OpenPairing, OpenedPairing, PairingChallenge, PairingIdentity,
        PairingOffer, PairingReceipt, StartPairing,
    },
    server::AuthenticatedServerSession,
    transport::{CorrelationId, UnixMillis},
};
use eitmad_pairing::PairingRegistration;
use eitmad_server_audit::{
    ServerAuditActor, ServerAuditEnvelope, ServerAuditOutcome, append, tenant_scope,
};
use sha2::{Digest as _, Sha256};
use sqlx::{PgPool, Postgres, Row as _, Transaction};
use uuid::Uuid;
use zeroize::Zeroizing;

mod first_desktop;

const TTL_MS: i64 = 180_000;
const MAX_ATTEMPTS: i32 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PairingError {
    #[error("pairing is denied")]
    Denied,
    #[error("pairing is unavailable")]
    Unavailable,
}
impl From<sqlx::Error> for PairingError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

#[derive(Clone)]
pub struct PairingService {
    pool: PgPool,
}

impl PairingService {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Opens one three-minute, five-attempt, single-device code on an authenticated Owner channel.
    /// # Errors
    /// Denies non-Owners or foreign scopes; fails closed if mandatory audit cannot commit.
    pub async fn open(
        &self,
        actor: &AuthenticatedServerSession,
        identity: &PairingIdentity,
        request: &OpenPairing,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<OpenedPairing, PairingError> {
        let mut tx = tenant_transaction(&self.pool, actor.tenant_id).await?;
        // Serialize opens for this installation, including other workshop contexts.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,18))")
            .bind(identity.server_id.value().to_string())
            .execute(&mut *tx)
            .await?;
        let scope: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.branches WHERE tenant_id=$1 AND organization_id=$2 AND branch_id=$3)")
            .bind(actor.tenant_id.value()).bind(request.organization_id.value()).bind(request.branch_id.value())
            .fetch_one(&mut *tx).await?;
        let owner = live_owner(&mut tx, actor, now).await?;
        if request.version != 1 || !scope || !owner {
            audit(
                &mut tx,
                ServerAuditActor::from_session(actor),
                None,
                "eitmad.server.pairing.open.v1",
                false,
                correlation,
                now,
            )
            .await?;
            tx.commit().await?;
            return Err(PairingError::Denied);
        }
        let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.pairing_directory WHERE server_id=$1 AND expires_at>$2)")
            .bind(identity.server_id.value()).bind(now.0).fetch_one(&mut *tx).await?;
        if active {
            audit(
                &mut tx,
                ServerAuditActor::from_session(actor),
                None,
                "eitmad.server.pairing.open.v1",
                false,
                correlation,
                now,
            )
            .await?;
            tx.commit().await?;
            return Err(PairingError::Denied);
        }
        let tenant_code: String =
            sqlx::query_scalar("SELECT tenant_code FROM control.tenants WHERE tenant_id=$1")
                .bind(actor.tenant_id.value())
                .fetch_one(&mut *tx)
                .await?;
        let offer = PairingOffer {
            version: 1,
            session_id: Uuid::new_v4(),
            identity: identity.clone(),
            tenant_id: actor.tenant_id,
            tenant_code: eitmad_contracts::server::TenantCode::parse(tenant_code)
                .map_err(|_| PairingError::Unavailable)?,
            organization_id: request.organization_id,
            branch_id: request.branch_id,
            expires_at: UnixMillis(now.0.saturating_add(TTL_MS)),
        };
        let code = eitmad_pairing::generate_code();
        // OPAQUE registration includes Argon2. Keep it off the asynchronous executor.
        let password = code.clone();
        let session = offer.session_id;
        let registration = tokio::task::spawn_blocking(move || {
            eitmad_pairing::register(&password, session.as_bytes())
        })
        .await
        .map_err(|_| PairingError::Unavailable)?
        .map_err(|_| PairingError::Unavailable)?;
        sqlx::query("INSERT INTO control.pairing_sessions(tenant_id,session_id,server_id,offer,owner_account_id,owner_user_id,setup,password_file,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(actor.tenant_id.value()).bind(offer.session_id).bind(identity.server_id.value())
            .bind(serde_json::to_value(&offer).map_err(|_| PairingError::Unavailable)?)
            .bind(actor.account_id.value()).bind(actor.user_id.value()).bind(registration.setup.as_slice())
            .bind(registration.password_file.as_slice()).bind(offer.expires_at.0).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO control.pairing_directory(session_id,tenant_id,server_id,expires_at) VALUES($1,$2,$3,$4)")
            .bind(offer.session_id).bind(actor.tenant_id.value()).bind(identity.server_id.value())
            .bind(offer.expires_at.0).execute(&mut *tx).await?;
        audit(
            &mut tx,
            ServerAuditActor::from_session(actor),
            Some(offer.session_id),
            "eitmad.server.pairing.open.v1",
            true,
            correlation,
            now,
        )
        .await?;
        tx.commit().await?;
        Ok(OpenedPairing {
            offer,
            code: code.to_string(),
        })
    }

    /// Returns the current untrusted public offer, without any code or verifier.
    /// # Errors
    /// Returns denied if no live offer exists.
    pub async fn offer(
        &self,
        identity: &PairingIdentity,
        now: UnixMillis,
    ) -> Result<PairingOffer, PairingError> {
        let row = sqlx::query("SELECT tenant_id,session_id FROM control.pairing_directory WHERE server_id=$1 AND expires_at>$2 ORDER BY expires_at DESC LIMIT 1")
            .bind(identity.server_id.value()).bind(now.0).fetch_optional(&self.pool).await?.ok_or(PairingError::Denied)?;
        let mut tx = tenant_transaction(&self.pool, TenantId::new(row.get("tenant_id"))).await?;
        let value: serde_json::Value = sqlx::query_scalar("SELECT offer FROM control.pairing_sessions WHERE session_id=$1 AND closed_at IS NULL AND attempts<5 AND expires_at>$2")
            .bind(row.get::<Uuid,_>("session_id")).bind(now.0).fetch_optional(&mut *tx).await?.ok_or(PairingError::Denied)?;
        let offer: PairingOffer =
            serde_json::from_value(value).map_err(|_| PairingError::Unavailable)?;
        if &offer.identity != identity {
            return Err(PairingError::Denied);
        }
        tx.commit().await?;
        Ok(offer)
    }

    /// Charges one attempt before disclosing the OPAQUE response. Abandoned attempts remain charged.
    /// # Errors
    /// Rejects expired, exhausted, replayed, or changed offers with mandatory redacted audit.
    pub async fn start(
        &self,
        identity: &PairingIdentity,
        request: &StartPairing,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<PairingChallenge, PairingError> {
        let offer = &request.binding.offer;
        let mut tx = tenant_transaction(&self.pool, offer.tenant_id).await?;
        let row = sqlx::query("SELECT offer,setup,password_file,attempts,expires_at,closed_at FROM control.pairing_sessions WHERE tenant_id=$1 AND session_id=$2 FOR UPDATE")
            .bind(offer.tenant_id.value()).bind(offer.session_id).fetch_optional(&mut *tx).await?;
        let result = if let Some(row) = row {
            let expected: PairingOffer =
                serde_json::from_value(row.get("offer")).map_err(|_| PairingError::Unavailable)?;
            let repeated: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.pairing_attempts WHERE tenant_id=$1 AND session_id=$2 AND attempt_id=$3)")
                .bind(offer.tenant_id.value()).bind(offer.session_id).bind(request.binding.attempt_id).fetch_one(&mut *tx).await?;
            if expected != *offer
                || &offer.identity != identity
                || repeated
                || row.get::<Option<i64>, _>("closed_at").is_some()
                || now.0 >= row.get::<i64, _>("expires_at")
                || row.get::<i32, _>("attempts") >= MAX_ATTEMPTS
            {
                Err(PairingError::Denied)
            } else {
                sqlx::query("UPDATE control.pairing_sessions SET attempts=attempts+1 WHERE tenant_id=$1 AND session_id=$2")
                    .bind(offer.tenant_id.value()).bind(offer.session_id).execute(&mut *tx).await?;
                let registration = PairingRegistration {
                    setup: Zeroizing::new(row.get("setup")),
                    password_file: Zeroizing::new(row.get("password_file")),
                };
                match eitmad_pairing::challenge(&registration, request) {
                    Ok((challenge, state)) => {
                        sqlx::query("INSERT INTO control.pairing_attempts(tenant_id,session_id,attempt_id,binding,state) VALUES($1,$2,$3,$4,$5)")
                            .bind(offer.tenant_id.value()).bind(offer.session_id).bind(request.binding.attempt_id)
                            .bind(serde_json::to_value(&request.binding).map_err(|_| PairingError::Unavailable)?)
                            .bind(state.as_slice()).execute(&mut *tx).await?;
                        Ok(challenge)
                    }
                    Err(_) => Err(PairingError::Denied),
                }
            }
        } else {
            Err(PairingError::Denied)
        };
        audit(
            &mut tx,
            ServerAuditActor::system(offer.tenant_id),
            Some(offer.session_id),
            "eitmad.server.pairing.start.v1",
            result.is_ok(),
            correlation,
            now,
        )
        .await?;
        tx.commit().await?;
        result
    }

    /// Approves exactly one device and stores a retry receipt in the same mandatory audit transaction.
    /// # Errors
    /// Rejects invalid proofs, expiry, revoked Owners, changed keys, and non-identical retries.
    pub async fn complete(
        &self,
        identity: &PairingIdentity,
        request: &CompletePairing,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<PairingReceipt, PairingError> {
        let offer = &request.binding.offer;
        let mut tx = tenant_transaction(&self.pool, offer.tenant_id).await?;
        sqlx::query("SAVEPOINT pairing_completion")
            .execute(&mut *tx)
            .await?;
        let result = finish(&mut tx, identity, request, now).await;
        // Storage errors roll back all work, including attempt consumption.
        if result.as_ref().err() == Some(&PairingError::Unavailable) {
            return result;
        }
        if result.is_err() {
            sqlx::query("ROLLBACK TO SAVEPOINT pairing_completion")
                .execute(&mut *tx)
                .await?;
            sqlx::query("UPDATE control.pairing_attempts SET state=NULL,finished_at=$4 WHERE tenant_id=$1 AND session_id=$2 AND attempt_id=$3 AND receipt IS NULL")
                .bind(offer.tenant_id.value()).bind(offer.session_id).bind(request.binding.attempt_id).bind(now.0).execute(&mut *tx).await?;
        }
        audit(
            &mut tx,
            ServerAuditActor::system(offer.tenant_id),
            Some(offer.session_id),
            "eitmad.server.pairing.complete.v1",
            result.is_ok(),
            correlation,
            now,
        )
        .await?;
        tx.commit().await?;
        result
    }
}

async fn finish(
    tx: &mut Transaction<'_, Postgres>,
    identity: &PairingIdentity,
    request: &CompletePairing,
    now: UnixMillis,
) -> Result<PairingReceipt, PairingError> {
    let offer = &request.binding.offer;
    let session = sqlx::query("SELECT offer,owner_account_id,owner_user_id,expires_at,closed_at FROM control.pairing_sessions WHERE tenant_id=$1 AND session_id=$2 FOR UPDATE")
        .bind(offer.tenant_id.value()).bind(offer.session_id).fetch_optional(&mut **tx).await?.ok_or(PairingError::Denied)?;
    let attempt = sqlx::query("SELECT binding,state,completion_hash,receipt,finished_at FROM control.pairing_attempts WHERE tenant_id=$1 AND session_id=$2 AND attempt_id=$3 FOR UPDATE")
        .bind(offer.tenant_id.value()).bind(offer.session_id).bind(request.binding.attempt_id)
        .fetch_optional(&mut **tx).await?.ok_or(PairingError::Denied)?;
    let expected: eitmad_contracts::pairing::PairingBinding =
        serde_json::from_value(attempt.get("binding")).map_err(|_| PairingError::Unavailable)?;
    if expected != request.binding
        || &offer.identity != identity
        || eitmad_pairing::verify_device(request).is_err()
    {
        return Err(PairingError::Denied);
    }
    let hash = Sha256::digest(serde_json::to_vec(request).map_err(|_| PairingError::Unavailable)?)
        .to_vec();
    if let Some(receipt) = attempt.get::<Option<serde_json::Value>, _>("receipt") {
        if attempt.get::<Option<Vec<u8>>, _>("completion_hash") != Some(hash) {
            return Err(PairingError::Denied);
        }
        // Exact authenticated retry recovers the original result, never repeats approval.
        return serde_json::from_value(receipt).map_err(|_| PairingError::Unavailable);
    }
    if session.get::<Option<i64>, _>("closed_at").is_some()
        || attempt.get::<Option<i64>, _>("finished_at").is_some()
        || now.0 >= session.get::<i64, _>("expires_at")
    {
        return Err(PairingError::Denied);
    }
    sqlx::query("UPDATE control.pairing_attempts SET state=NULL,finished_at=$4 WHERE tenant_id=$1 AND session_id=$2 AND attempt_id=$3")
        .bind(offer.tenant_id.value()).bind(offer.session_id).bind(request.binding.attempt_id).bind(now.0).execute(&mut **tx).await?;
    let owner = sqlx::query("SELECT a.account_id FROM control.accounts a JOIN control.relationship_tuples r ON r.tenant_id=a.tenant_id AND r.subject_principal_id=a.user_id WHERE a.tenant_id=$1 AND a.account_id=$2 AND a.user_id=$3 AND a.status='active' AND (a.locked_until IS NULL OR a.locked_until<=$4) AND r.relation='eitmad.relation.organization.owner.v1' AND r.object_kind='tenant' AND r.object_id=$1 FOR SHARE OF a,r")
        .bind(offer.tenant_id.value()).bind(session.get::<Uuid,_>("owner_account_id")).bind(session.get::<Uuid,_>("owner_user_id")).bind(now.0)
        .fetch_optional(&mut **tx).await?.is_some();
    if !owner {
        return Err(PairingError::Denied);
    }
    let state = Zeroizing::new(attempt.get::<Vec<u8>, _>("state"));
    let receipt = eitmad_pairing::complete(&state, request).map_err(|_| PairingError::Denied)?;
    let public = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &request.binding.device_public_key.base64,
    )
    .map_err(|_| PairingError::Denied)?;
    crate::authentication::ensure_device(
        tx,
        request.binding.device_id,
        &public,
        "paired desktop",
        now,
    )
    .await
    .map_err(|error| match error {
        crate::AuthenticationError::Unavailable => PairingError::Unavailable,
        _ => PairingError::Denied,
    })?;
    // Never restore revoked approval or rebind a retained device to a different workshop.
    let inserted = sqlx::query("INSERT INTO control.paired_devices(tenant_id,device_id,organization_id,branch_id,approved_at) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
        .bind(offer.tenant_id.value()).bind(request.binding.device_id.value()).bind(offer.organization_id.value())
        .bind(offer.branch_id.value()).bind(now.0).execute(&mut **tx).await?;
    if inserted.rows_affected() != 1 {
        return Err(PairingError::Denied);
    }
    sqlx::query("UPDATE control.pairing_sessions SET closed_at=$3,setup=NULL,password_file=NULL WHERE tenant_id=$1 AND session_id=$2")
        .bind(offer.tenant_id.value()).bind(offer.session_id).bind(now.0).execute(&mut **tx).await?;
    sqlx::query("UPDATE control.pairing_directory SET expires_at=$2 WHERE session_id=$1")
        .bind(offer.session_id)
        .bind(now.0)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "UPDATE control.pairing_attempts SET state=NULL WHERE tenant_id=$1 AND session_id=$2",
    )
    .bind(offer.tenant_id.value())
    .bind(offer.session_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE control.pairing_attempts SET completion_hash=$4,receipt=$5 WHERE tenant_id=$1 AND session_id=$2 AND attempt_id=$3")
        .bind(offer.tenant_id.value()).bind(offer.session_id).bind(request.binding.attempt_id).bind(hash)
        .bind(serde_json::to_value(&receipt).map_err(|_| PairingError::Unavailable)?).execute(&mut **tx).await?;
    Ok(receipt)
}

async fn live_owner(
    tx: &mut Transaction<'_, Postgres>,
    actor: &AuthenticatedServerSession,
    now: UnixMillis,
) -> Result<bool, PairingError> {
    Ok(sqlx::query("SELECT s.session_id FROM control.sessions s JOIN control.accounts a ON a.tenant_id=s.tenant_id AND a.account_id=s.account_id JOIN control.account_devices ad ON ad.tenant_id=s.tenant_id AND ad.account_id=s.account_id AND ad.device_id=s.device_id JOIN control.devices d ON d.device_id=s.device_id JOIN control.relationship_tuples r ON r.tenant_id=s.tenant_id AND r.subject_principal_id=s.user_id WHERE s.tenant_id=$1 AND s.session_id=$2 AND s.account_id=$3 AND s.user_id=$4 AND s.device_id=$5 AND s.revoked_at IS NULL AND s.expires_at>$6 AND s.idle_expires_at>$6 AND a.status='active' AND (a.locked_until IS NULL OR a.locked_until<=$6) AND ad.revoked_at IS NULL AND d.revoked_at IS NULL AND NOT EXISTS(SELECT 1 FROM control.paired_devices p WHERE p.tenant_id=s.tenant_id AND p.device_id=s.device_id AND p.revoked_at IS NOT NULL) AND r.relation='eitmad.relation.organization.owner.v1' AND r.object_kind='tenant' AND r.object_id=$1 FOR SHARE OF s,a,ad,d,r")
        .bind(actor.tenant_id.value()).bind(actor.session_id.value()).bind(actor.account_id.value())
        .bind(actor.user_id.value()).bind(actor.device_id.value()).bind(now.0).fetch_optional(&mut **tx).await?.is_some())
}

pub(crate) async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: ServerAuditActor,
    target: Option<Uuid>,
    operation: &str,
    success: bool,
    correlation: CorrelationId,
    now: UnixMillis,
) -> Result<(), PairingError> {
    append(
        tx,
        &ServerAuditEnvelope {
            actor,
            scope: tenant_scope(actor.tenant_id),
            operation,
            outcome: if success {
                ServerAuditOutcome::Succeeded
            } else {
                ServerAuditOutcome::Denied
            },
            target_kind: "pairing",
            target_id: target,
            correlation_id: correlation,
            causation_id: None,
            idempotency_key: None,
            redacted_error: if success {
                None
            } else {
                Some("eitmad.error.pairing-denied.v1")
            },
            occurred_at: now,
        },
    )
    .await?;
    Ok(())
}
