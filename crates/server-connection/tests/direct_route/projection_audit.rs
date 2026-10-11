use super::*;
use eitmad_contracts::identity::UserId;
use eitmad_sync_plane::{OperationError, PullPageRequest, SnapshotError, SnapshotRequest};
use sqlx::Row as _;

struct RevokedProjection;

#[async_trait]
impl DomainSyncHandler for RevokedProjection {
    fn descriptor(&self) -> DomainDescriptor {
        TestDomain.descriptor()
    }

    async fn authorize(
        &self,
        _session: &AuthenticatedServerSession,
        _scope: &ScopeRef,
        _intent: SyncIntent,
    ) -> bool {
        true
    }

    fn validate_local(&self, _draft: &LocalOperationDraft) -> Result<(), DomainValidationError> {
        Ok(())
    }

    async fn project_page(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        _session: &AuthenticatedServerSession,
        _scope: &ScopeRef,
        _changes: Vec<ChangeRecord>,
    ) -> Result<Vec<ChangeRecord>, OperationError> {
        Err(OperationError::Denied)
    }
}

#[tokio::test]
#[ignore = "requires disposable PostgreSQL"]
async fn revoked_page_projection_commits_denial_audit_for_pull_and_snapshot() {
    let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
    let (_, sync) = migrate_test_databases(&database).await;
    let session = AuthenticatedServerSession {
        session_id: SessionId::new(Uuid::new_v4()),
        account_id: AccountId::new(Uuid::new_v4()),
        user_id: UserId::new(Uuid::new_v4()),
        device_id: DeviceId::new(Uuid::new_v4()),
        tenant_id: TenantId::new(Uuid::new_v4()),
        issued_at: UnixMillis(1),
        expires_at: UnixMillis(i64::MAX),
    };
    let scope = ScopeRef {
        kind: ScopeKind::parse("organization").unwrap(),
        id: ScopeId::new(session.tenant_id.value()),
    };
    sqlx::query("INSERT INTO control.tenants(tenant_id,tenant_code,display_name,created_at) VALUES($1,$2,'Synthetic audit test',1)")
        .bind(session.tenant_id.value()).bind(session.tenant_id.value().simple().to_string())
        .execute(&sync.pool()).await.unwrap();
    let schema = schema_id();
    let registry =
        DomainRegistry::new([Arc::new(RevokedProjection) as Arc<dyn DomainSyncHandler>]).unwrap();
    let coordinator = SyncCoordinator::new(&sync, registry);
    coordinator
        .apply_local_operation(
            &session,
            &LocalOperationDraft {
                change_id: ChangeId::new(Uuid::new_v4()),
                scope: scope.clone(),
                schema_id: schema.clone(),
                schema_version: 1,
                record_id: RecordId::new(Uuid::new_v4()),
                operation: ChangeOperation::Upsert,
                base_revision: None,
                idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
                payload: None,
            },
            CorrelationId::new(Uuid::new_v4()),
            UnixMillis(2),
        )
        .await
        .unwrap();
    let pull = CorrelationId::new(Uuid::new_v4());
    assert_eq!(
        coordinator
            .pull(PullPageRequest {
                session: &session,
                scope: &scope,
                schema_id: &schema,
                schema_version: 1,
                after: None,
                maximum_records: 10,
                correlation_id: pull,
                now: UnixMillis(3),
            })
            .await,
        Err(OperationError::Denied)
    );
    let snapshot = CorrelationId::new(Uuid::new_v4());
    assert!(matches!(
        coordinator
            .create_snapshot(
                SnapshotRequest {
                    session: &session,
                    scope: &scope,
                    schema_id: &schema,
                    schema_version: 1,
                },
                snapshot,
                UnixMillis(4),
                60_000
            )
            .await,
        Err(SnapshotError::Denied)
    ));
    assert_denials(&sync.pool(), &session, &scope, [pull, snapshot]).await;
}

async fn assert_denials(
    pool: &sqlx::PgPool,
    session: &AuthenticatedServerSession,
    scope: &ScopeRef,
    correlations: [CorrelationId; 2],
) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    for (correlation, operation) in correlations.into_iter().zip([
        "eitmad.server.sync.pull.v1",
        "eitmad.server.sync.create-snapshot.v1",
    ]) {
        let rows = sqlx::query("SELECT operation,outcome,redacted_error,scope_id,session_id FROM audit.server_records WHERE tenant_id=$1 AND correlation_id=$2")
            .bind(session.tenant_id.value()).bind(correlation.value()).fetch_all(&mut *tx).await.unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.get::<String, _>("operation"), operation);
        assert_eq!(row.get::<String, _>("outcome"), "denied");
        assert_eq!(
            row.get::<String, _>("redacted_error"),
            "eitmad.error.authorization-denied.v1"
        );
        assert_eq!(row.get::<Uuid, _>("scope_id"), scope.id.value());
        assert_eq!(row.get::<Uuid, _>("session_id"), session.session_id.value());
    }
    let stored: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sync.snapshots WHERE tenant_id=$1")
        .bind(session.tenant_id.value())
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(stored, 0);
    tx.commit().await.unwrap();
}
