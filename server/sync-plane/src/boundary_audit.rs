use eitmad_server_audit::{ServerAuditEnvelope, append};
use sqlx::PgPool;

use crate::database::tenant_transaction;

pub(crate) async fn record(
    pool: &PgPool,
    envelope: &ServerAuditEnvelope<'_>,
) -> Result<(), sqlx::Error> {
    let mut transaction = tenant_transaction(pool, envelope.actor.tenant_id).await?;
    append(&mut transaction, envelope).await?;
    transaction.commit().await
}
