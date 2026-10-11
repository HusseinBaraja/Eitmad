use eitmad_contracts::{
    identity::ScopeRef, server::AuthenticatedServerSession, sync::ChangeRecord,
};
use eitmad_server_audit::{ServerAuditEnvelope, ServerAuditOutcome, append};
use sqlx::PgPool;

use crate::{OperationError, database::tenant_transaction, domain::DomainSyncHandler};

/// Rolls back denied page work and commits its audit separately before returning the denial.
/// Audit failure withholds the denial and returns unavailable.
pub(crate) async fn project_page<'a>(
    pool: &PgPool,
    mut transaction: sqlx::Transaction<'a, sqlx::Postgres>,
    handler: &dyn DomainSyncHandler,
    session: &AuthenticatedServerSession,
    scope: &ScopeRef,
    records: Vec<ChangeRecord>,
    mut audit: ServerAuditEnvelope<'_>,
) -> Result<(sqlx::Transaction<'a, sqlx::Postgres>, Vec<ChangeRecord>), OperationError> {
    match handler
        .project_page(&mut transaction, session, scope, records)
        .await
    {
        Ok(projected) => Ok((transaction, projected)),
        Err(OperationError::Denied) => {
            transaction
                .rollback()
                .await
                .map_err(|_| OperationError::Unavailable)?;
            audit.outcome = ServerAuditOutcome::Denied;
            audit.redacted_error = Some("eitmad.error.authorization-denied.v1");
            record(pool, &audit)
                .await
                .map_err(|_| OperationError::Unavailable)?;
            Err(OperationError::Denied)
        }
        Err(error) => Err(error),
    }
}

pub(crate) async fn record(
    pool: &PgPool,
    envelope: &ServerAuditEnvelope<'_>,
) -> Result<(), sqlx::Error> {
    let mut transaction = tenant_transaction(pool, envelope.actor.tenant_id).await?;
    append(&mut transaction, envelope).await?;
    transaction.commit().await
}
