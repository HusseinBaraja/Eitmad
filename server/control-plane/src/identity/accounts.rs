use eitmad_contracts::{
    identity::{AccountId, OrganizationId, TenantId, UserId},
    server::{AuthenticatedServerSession, DEFAULT_INVITE_TTL_MS, InviteId},
    server_accounts::{
        ChangeServerAccount, ListServerAccounts, ServerAccountChange, ServerAccountPage,
        ServerAccountReceipt, ServerAccountResult, ServerRoleAssignment,
    },
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditEvent, ServerAuditOutcome, append};
use sqlx::{Postgres, Row as _, Transaction};
use uuid::Uuid;

use super::{
    IdentityService, OWNER_RELATION,
    account_projection::{self, MANAGER, RECEPTIONIST},
    canonical_username,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ServerAccountError {
    #[error("server account access is denied")]
    Denied,
    #[error("server account input is invalid")]
    Invalid,
    #[error("server account revision is stale")]
    RevisionConflict,
    #[error("server account retry intent differs")]
    IdempotencyMismatch,
    #[error("owner account deactivation is not supported")]
    OwnerProtected,
    #[error("server account authority is unavailable")]
    Unavailable,
}

impl From<sqlx::Error> for ServerAccountError {
    fn from(_: sqlx::Error) -> Self {
        Self::Unavailable
    }
}

struct Attempt<'a> {
    actor: &'a AuthenticatedServerSession,
    operation: &'static str,
    correlation: CorrelationId,
    key: Option<IdempotencyKey>,
    now: UnixMillis,
}

impl IdentityService {
    /// Lists one bounded tenant account page after checking the current Owner relationship.
    /// # Errors
    /// Fails closed on denied access, invalid pagination, or unavailable audit/storage.
    pub async fn list_accounts(
        &self,
        actor: &AuthenticatedServerSession,
        request: &ListServerAccounts,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<ServerAccountPage, ServerAccountError> {
        let attempt = Attempt {
            actor,
            operation: "eitmad.server.account.list.v1",
            correlation,
            key: None,
            now,
        };
        let mut tx = self.account_owner(&attempt).await?;
        if !(1..=100).contains(&request.limit) {
            return reject(tx, &attempt, ServerAccountError::Invalid, None).await;
        }
        let mut ids: Vec<Uuid> = sqlx::query_scalar("SELECT account_id FROM control.accounts WHERE tenant_id=$1 AND ($2::uuid IS NULL OR account_id>$2) ORDER BY account_id LIMIT $3")
            .bind(actor.tenant_id.value()).bind(request.after.map(AccountId::value)).bind(i64::from(request.limit)+1).fetch_all(&mut *tx).await?;
        let more = ids.len() > usize::from(request.limit);
        ids.truncate(usize::from(request.limit));
        let mut accounts = Vec::new();
        for id in ids {
            accounts.push(
                account_projection::read(&mut tx, actor.tenant_id, AccountId::new(id)).await?,
            );
        }
        let next_after = if more {
            accounts.last().map(|a| a.account_id)
        } else {
            None
        };
        audit(&mut tx, &attempt, ServerAuditOutcome::Succeeded, None, None).await?;
        tx.commit().await?;
        Ok(ServerAccountPage {
            accounts,
            next_after,
        })
    }

    /// Commits one revisioned account change with its relationships, revocation, audit, and retry receipt.
    /// # Errors
    /// Rejects non-Owners, foreign IDs, stale revisions, changed retry intent, and Owner deactivation.
    pub async fn change_account(
        &self,
        actor: &AuthenticatedServerSession,
        request: &ChangeServerAccount,
        correlation: CorrelationId,
        now: UnixMillis,
    ) -> Result<ServerAccountResult, ServerAccountError> {
        let operation = match &request.change {
            ServerAccountChange::Invite { .. } => "eitmad.server.account.invite.v1",
            ServerAccountChange::Assign { .. } => "eitmad.server.account.assign.v1",
            ServerAccountChange::Deactivate { .. } => "eitmad.server.account.deactivate.v1",
            ServerAccountChange::RevokeSessions { .. } => {
                "eitmad.server.account.sessions.revoke.v1"
            }
        };
        let attempt = Attempt {
            actor,
            operation,
            correlation,
            key: Some(request.idempotency_key),
            now,
        };
        let mut tx = self.account_owner(&attempt).await?;
        let encoded =
            serde_json::to_string(request).map_err(|_| ServerAccountError::Unavailable)?;
        let fingerprint = self
            .tokens
            .hash(&format!("account-intent.v1:{encoded}"))
            .map_err(|_| ServerAccountError::Unavailable)?;
        let previous=sqlx::query("SELECT fingerprint,receipt,error FROM control.account_results WHERE tenant_id=$1 AND actor_id=$2 AND idempotency_key=$3")
            .bind(actor.tenant_id.value()).bind(actor.user_id.value()).bind(request.idempotency_key.value()).fetch_optional(&mut *tx).await?;
        if let Some(previous) = previous {
            if previous.get::<Vec<u8>, _>("fingerprint") != fingerprint {
                return reject(tx, &attempt, ServerAccountError::IdempotencyMismatch, None).await;
            }
            if let Some(code) = previous.get::<Option<String>, _>("error") {
                let error = stored_error(&code)?;
                tx.commit().await?;
                return Err(error);
            }
            let receipt = serde_json::from_value(previous.get("receipt"))
                .map_err(|_| ServerAccountError::Unavailable)?;
            tx.commit().await?;
            return self.account_result(receipt);
        }
        // Savepoint keeps invalid outcomes auditable without committing partial state.
        sqlx::query("SAVEPOINT account_change")
            .execute(&mut *tx)
            .await?;
        let receipt = match self
            .apply_account_change(&mut tx, &attempt, &request.change)
            .await
        {
            Ok(receipt) => receipt,
            Err(error) => {
                sqlx::query("ROLLBACK TO SAVEPOINT account_change")
                    .execute(&mut *tx)
                    .await?;
                if error == ServerAccountError::Unavailable {
                    return Err(error);
                }
                sqlx::query("INSERT INTO control.account_results(tenant_id,actor_id,idempotency_key,fingerprint,error) VALUES($1,$2,$3,$4,$5)")
                    .bind(actor.tenant_id.value()).bind(actor.user_id.value()).bind(request.idempotency_key.value()).bind(fingerprint.as_slice()).bind(error_code(error)).execute(&mut *tx).await?;
                return reject(tx, &attempt, error, target(&request.change)).await;
            }
        };
        let json = serde_json::to_value(&receipt).map_err(|_| ServerAccountError::Unavailable)?;
        sqlx::query("INSERT INTO control.account_results(tenant_id,actor_id,idempotency_key,fingerprint,receipt) VALUES($1,$2,$3,$4,$5)")
            .bind(actor.tenant_id.value()).bind(actor.user_id.value()).bind(request.idempotency_key.value()).bind(fingerprint.as_slice()).bind(json).execute(&mut *tx).await?;
        audit(
            &mut tx,
            &attempt,
            ServerAuditOutcome::Succeeded,
            Some(receipt.account.account_id),
            None,
        )
        .await?;
        tx.commit().await?;
        self.account_result(receipt)
    }

    fn account_result(
        &self,
        receipt: ServerAccountReceipt,
    ) -> Result<ServerAccountResult, ServerAccountError> {
        let activation_token = receipt
            .invitation_id
            .map(|id| {
                self.tokens
                    .derive(&format!("account-invite.v1:{}", id.value()))
            })
            .transpose()
            .map_err(|_| ServerAccountError::Unavailable)?;
        Ok(ServerAccountResult {
            receipt,
            activation_token,
        })
    }

    async fn account_owner(
        &self,
        attempt: &Attempt<'_>,
    ) -> Result<Transaction<'_, Postgres>, ServerAccountError> {
        let a = attempt.actor;
        let mut tx = crate::database::tenant_transaction(&self.pool, a.tenant_id).await?;
        // Serialize lifecycle decisions per tenant. Activation/login additionally lock the account.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,16016))")
            .bind(a.tenant_id.value().to_string())
            .execute(&mut *tx)
            .await?;
        let allowed: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                SELECT 1 FROM control.accounts a
                JOIN control.sessions s ON s.tenant_id=a.tenant_id
                  AND s.account_id=a.account_id AND s.user_id=a.user_id
                JOIN control.token_families f ON f.tenant_id=s.tenant_id
                  AND f.session_id=s.session_id AND f.revoked_at IS NULL
                JOIN control.account_devices ad ON ad.tenant_id=s.tenant_id
                  AND ad.account_id=s.account_id AND ad.device_id=s.device_id
                JOIN control.devices d ON d.device_id=s.device_id
                JOIN control.relationship_tuples r ON r.tenant_id=a.tenant_id
                  AND r.subject_principal_id=a.user_id
                WHERE a.tenant_id=$1 AND a.account_id=$2 AND a.user_id=$3 AND a.status='active'
                  AND s.session_id=$4 AND s.device_id=$5 AND s.revoked_at IS NULL
                  AND s.expires_at>$6 AND s.idle_expires_at>$6
                  AND ad.revoked_at IS NULL AND d.revoked_at IS NULL
                  AND r.subject_kind='user' AND r.object_kind='tenant'
                  AND r.object_id=$1 AND r.relation=$7)",
        )
        .bind(a.tenant_id.value())
        .bind(a.account_id.value())
        .bind(a.user_id.value())
        .bind(a.session_id.value())
        .bind(a.device_id.value())
        .bind(attempt.now.0)
        .bind(OWNER_RELATION)
        .fetch_one(&mut *tx)
        .await?;
        if !allowed {
            return reject(tx, attempt, ServerAccountError::Denied, None).await;
        }
        Ok(tx)
    }

    async fn apply_account_change(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        attempt: &Attempt<'_>,
        change: &ServerAccountChange,
    ) -> Result<ServerAccountReceipt, ServerAccountError> {
        let tenant = attempt.actor.tenant_id;
        let (account, invitation) = if let ServerAccountChange::Invite {
            organization_id,
            username,
            assignments,
        } = change
        {
            validate_assignments(tx, tenant, *organization_id, assignments).await?;
            let username = canonical_username(username).map_err(|_| ServerAccountError::Invalid)?;
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.accounts WHERE tenant_id=$1 AND canonical_username=$2)").bind(tenant.value()).bind(&username).fetch_one(&mut **tx).await?;
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
            sqlx::query("INSERT INTO control.accounts(tenant_id,account_id,user_id,username,canonical_username,status,created_at) VALUES($1,$2,$3,$4,$4,'pending_activation',$5)").bind(tenant.value()).bind(account.value()).bind(user.value()).bind(username).bind(attempt.now.0).execute(&mut **tx).await?;
            replace_assignments(tx, attempt, account, user, *organization_id, assignments).await?;
            let invite = InviteId::new(Uuid::new_v4());
            let token = self
                .tokens
                .derive(&format!("account-invite.v1:{}", invite.value()))
                .map_err(|_| ServerAccountError::Unavailable)?;
            let hash = self
                .tokens
                .hash(&token)
                .map_err(|_| ServerAccountError::Unavailable)?;
            let expires = attempt
                .now
                .0
                .checked_add(DEFAULT_INVITE_TTL_MS)
                .ok_or(ServerAccountError::Invalid)?;
            sqlx::query("INSERT INTO control.invitations(tenant_id,invite_id,account_id,token_hash,expires_at,created_at) VALUES($1,$2,$3,$4,$5,$6)").bind(tenant.value()).bind(invite.value()).bind(account.value()).bind(hash.as_slice()).bind(expires).bind(attempt.now.0).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO control.invitation_directory(token_hash,tenant_id,invite_id) VALUES($1,$2,$3)").bind(hash.as_slice()).bind(tenant.value()).bind(invite.value()).execute(&mut **tx).await?;
            (account, Some((invite, UnixMillis(expires))))
        } else {
            let account = target(change).ok_or(ServerAccountError::Invalid)?;
            self.update_account(tx, attempt, change, account).await?;
            (account, None)
        };
        Ok(ServerAccountReceipt {
            account: account_projection::read(tx, tenant, account).await?,
            invitation_id: invitation.map(|i| i.0),
            invitation_expires_at: invitation.map(|i| i.1),
        })
    }

    async fn update_account(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        attempt: &Attempt<'_>,
        change: &ServerAccountChange,
        account: AccountId,
    ) -> Result<(), ServerAccountError> {
        let tenant = attempt.actor.tenant_id;
        let row=sqlx::query("SELECT user_id,revision,status FROM control.accounts WHERE tenant_id=$1 AND account_id=$2 FOR UPDATE")
            .bind(tenant.value()).bind(account.value()).fetch_optional(&mut **tx).await?.ok_or(ServerAccountError::Denied)?;
        let user = UserId::new(row.get("user_id"));
        let revision = u64::try_from(row.get::<i64, _>("revision"))
            .map_err(|_| ServerAccountError::Unavailable)?;
        let expected = match change {
            ServerAccountChange::Assign {
                expected_revision, ..
            }
            | ServerAccountChange::Deactivate {
                expected_revision, ..
            }
            | ServerAccountChange::RevokeSessions {
                expected_revision, ..
            } => *expected_revision,
            ServerAccountChange::Invite { .. } => return Err(ServerAccountError::Invalid),
        };
        if expected != revision {
            return Err(ServerAccountError::RevisionConflict);
        }
        if revision >= i64::MAX as u64 {
            return Err(ServerAccountError::Unavailable);
        }
        if row.get::<String, _>("status") == "disabled" {
            return Err(ServerAccountError::Invalid);
        }
        match change {
            ServerAccountChange::Assign {
                organization_id,
                assignments,
                ..
            } => {
                validate_assignments(tx, tenant, *organization_id, assignments).await?;
                replace_assignments(tx, attempt, account, user, *organization_id, assignments)
                    .await?;
            }
            ServerAccountChange::Deactivate { .. } => {
                let owner:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.relationship_tuples WHERE tenant_id=$1 AND subject_principal_id=$2 AND subject_kind='user' AND relation=$3)").bind(tenant.value()).bind(user.value()).bind(OWNER_RELATION).fetch_one(&mut **tx).await?;
                if owner {
                    return Err(ServerAccountError::OwnerProtected);
                }
                sqlx::query("UPDATE control.accounts SET status='disabled' WHERE tenant_id=$1 AND account_id=$2").bind(tenant.value()).bind(account.value()).execute(&mut **tx).await?;
                sqlx::query("DELETE FROM control.relationship_tuples WHERE tenant_id=$1 AND subject_principal_id=$2 AND subject_kind='user'").bind(tenant.value()).bind(user.value()).execute(&mut **tx).await?;
                sqlx::query("UPDATE control.invitations SET consumed_at=COALESCE(consumed_at,$3) WHERE tenant_id=$1 AND account_id=$2").bind(tenant.value()).bind(account.value()).bind(attempt.now.0).execute(&mut **tx).await?;
            }
            _ => {}
        }
        sqlx::query(
            "UPDATE control.accounts SET revision=revision+1 WHERE tenant_id=$1 AND account_id=$2",
        )
        .bind(tenant.value())
        .bind(account.value())
        .execute(&mut **tx)
        .await?;
        revoke(tx, tenant, account, attempt.now).await
    }
}

fn target(change: &ServerAccountChange) -> Option<AccountId> {
    match change {
        ServerAccountChange::Invite { .. } => None,
        ServerAccountChange::Assign { account_id, .. }
        | ServerAccountChange::Deactivate { account_id, .. }
        | ServerAccountChange::RevokeSessions { account_id, .. } => Some(*account_id),
    }
}

async fn validate_assignments(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    organization: OrganizationId,
    assignments: &[ServerRoleAssignment],
) -> Result<(), ServerAccountError> {
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.organizations WHERE tenant_id=$1 AND organization_id=$2)").bind(tenant.value()).bind(organization.value()).fetch_one(&mut **tx).await?;
    if !exists {
        return Err(ServerAccountError::Denied);
    }
    if assignments.len() > 2 {
        return Err(ServerAccountError::Invalid);
    }
    let mut roles = std::collections::HashSet::new();
    for a in assignments {
        if !roles.insert(account_projection::relation(a.role)) || a.branch_ids.len() > 100 {
            return Err(ServerAccountError::Invalid);
        }
        // Receptionists require an assigned branch. Managers have organization-wide access.
        if a.role == eitmad_contracts::accounts::DesktopAccountRole::Receptionist
            && a.branch_ids.is_empty()
        {
            return Err(ServerAccountError::Invalid);
        }
        let mut branches = std::collections::HashSet::new();
        for branch in &a.branch_ids {
            if !branches.insert(branch.value()) {
                return Err(ServerAccountError::Invalid);
            }
            let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM control.branches WHERE tenant_id=$1 AND organization_id=$2 AND branch_id=$3)").bind(tenant.value()).bind(organization.value()).bind(branch.value()).fetch_one(&mut **tx).await?;
            if !exists {
                return Err(ServerAccountError::Denied);
            }
        }
    }
    Ok(())
}

async fn replace_assignments(
    tx: &mut Transaction<'_, Postgres>,
    attempt: &Attempt<'_>,
    account: AccountId,
    user: UserId,
    organization: OrganizationId,
    assignments: &[ServerRoleAssignment],
) -> Result<(), ServerAccountError> {
    let tenant = attempt.actor.tenant_id;
    sqlx::query("INSERT INTO control.account_organizations(tenant_id,account_id,organization_id) VALUES($1,$2,$3) ON CONFLICT DO NOTHING").bind(tenant.value()).bind(account.value()).bind(organization.value()).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM control.relationship_tuples r WHERE tenant_id=$1 AND subject_principal_id=$2 AND subject_kind='user' AND relation=ANY($3) AND ((object_kind='organization' AND object_id=$4) OR (object_kind='branch' AND EXISTS(SELECT 1 FROM control.branches b WHERE b.tenant_id=r.tenant_id AND b.branch_id=r.object_id AND b.organization_id=$4)))")
        .bind(tenant.value()).bind(user.value()).bind(vec![MANAGER,RECEPTIONIST]).bind(organization.value()).execute(&mut **tx).await?;
    for a in assignments {
        let relation = account_projection::relation(a.role);
        sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user',$3,'organization',$4,$5)").bind(tenant.value()).bind(user.value()).bind(relation).bind(organization.value()).bind(attempt.now.0).execute(&mut **tx).await?;
        for branch in &a.branch_ids {
            sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user',$3,'branch',$4,$5)").bind(tenant.value()).bind(user.value()).bind(relation).bind(branch.value()).bind(attempt.now.0).execute(&mut **tx).await?;
        }
    }
    Ok(())
}

async fn revoke(
    tx: &mut Transaction<'_, Postgres>,
    tenant: TenantId,
    account: AccountId,
    now: UnixMillis,
) -> Result<(), ServerAccountError> {
    sqlx::query("UPDATE control.sessions SET revoked_at=$3 WHERE tenant_id=$1 AND account_id=$2 AND revoked_at IS NULL").bind(tenant.value()).bind(account.value()).bind(now.0).execute(&mut **tx).await?;
    sqlx::query("UPDATE control.token_families f SET revoked_at=$3 WHERE tenant_id=$1 AND revoked_at IS NULL AND EXISTS(SELECT 1 FROM control.sessions s WHERE s.tenant_id=f.tenant_id AND s.session_id=f.session_id AND s.account_id=$2)").bind(tenant.value()).bind(account.value()).bind(now.0).execute(&mut **tx).await?;
    Ok(())
}

async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    attempt: &Attempt<'_>,
    outcome: ServerAuditOutcome,
    target: Option<AccountId>,
    error: Option<&str>,
) -> Result<(), ServerAccountError> {
    append(
        tx,
        &ServerAuditEnvelope::for_tenant_session(
            attempt.actor,
            ServerAuditEvent {
                operation: attempt.operation,
                outcome,
                target_kind: "account",
                target_id: target.map(AccountId::value),
                correlation_id: attempt.correlation,
                causation_id: None,
                idempotency_key: attempt.key,
                redacted_error: error,
                occurred_at: attempt.now,
            },
        ),
    )
    .await
    .map_err(|_| ServerAccountError::Unavailable)
}

async fn reject<T>(
    mut tx: Transaction<'_, Postgres>,
    attempt: &Attempt<'_>,
    error: ServerAccountError,
    target: Option<AccountId>,
) -> Result<T, ServerAccountError> {
    let outcome = match error {
        ServerAccountError::Denied => ServerAuditOutcome::Denied,
        ServerAccountError::RevisionConflict | ServerAccountError::IdempotencyMismatch => {
            ServerAuditOutcome::Conflict
        }
        _ => ServerAuditOutcome::Invalid,
    };
    let code = error_code(error);
    // A foreign target must not be included in a tenant's audit stream.
    let target = if error == ServerAccountError::Denied {
        None
    } else {
        target
    };
    audit(&mut tx, attempt, outcome, target, Some(code)).await?;
    tx.commit().await?;
    Err(error)
}

fn error_code(error: ServerAccountError) -> &'static str {
    match error {
        ServerAccountError::Denied => "eitmad.error.authorization-denied.v1",
        ServerAccountError::RevisionConflict => "eitmad.error.server-account-revision-conflict.v1",
        ServerAccountError::IdempotencyMismatch => "eitmad.error.server-idempotency-mismatch.v1",
        ServerAccountError::OwnerProtected => "eitmad.error.server-owner-protected.v1",
        _ => "eitmad.error.contract-invalid.v1",
    }
}

fn stored_error(code: &str) -> Result<ServerAccountError, ServerAccountError> {
    match code {
        "eitmad.error.authorization-denied.v1" => Ok(ServerAccountError::Denied),
        "eitmad.error.contract-invalid.v1" => Ok(ServerAccountError::Invalid),
        "eitmad.error.server-account-revision-conflict.v1" => {
            Ok(ServerAccountError::RevisionConflict)
        }
        "eitmad.error.server-owner-protected.v1" => Ok(ServerAccountError::OwnerProtected),
        _ => Err(ServerAccountError::Unavailable),
    }
}
