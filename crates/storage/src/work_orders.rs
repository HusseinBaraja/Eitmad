use crate::{
    AuthorityStore, DurablePublication, PricingTransaction, StorageError, insert_audit,
    insert_publication, migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    events::Event,
    identity::AuthorizationContext,
    order::OrderNotice,
    transport::IdempotencyKey,
    work_order::{ListWorkOrders, WorkOrderPage, WorkOrderRecord},
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(28, "work-orders.confirmed-cache.v1", "work-orders",
    "CREATE TABLE work_order_confirmed_history (
     tenant_id TEXT NOT NULL, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
     work_id TEXT NOT NULL, order_id TEXT NOT NULL, revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
     PRIMARY KEY(tenant_id,scope_kind,scope_id,work_id,revision));
     CREATE TRIGGER work_order_history_no_update BEFORE UPDATE ON work_order_confirmed_history BEGIN SELECT RAISE(ABORT,'immutable work order'); END;
     CREATE TRIGGER work_order_history_no_delete BEFORE DELETE ON work_order_confirmed_history BEGIN SELECT RAISE(ABORT,'immutable work order'); END;")];

impl PricingTransaction<'_> {
    /// # Errors
    /// Contradictory confirmations, audit failure, or publication failure roll back together.
    pub fn cache_work_order(
        &self,
        value: &WorkOrderRecord,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        let (kind, scope) = scope_parts(&audit.scope);
        let mut retained = value.clone();
        retained.can_start = false;
        retained.can_complete = false;
        let json = serde_json::to_vec(&retained).map_err(|_| StorageError)?;
        let tenant = audit.tenant_id.value().to_string();
        let revision = i64::try_from(value.revision).map_err(|_| StorageError)?;
        let previous: Option<Vec<u8>> = self.connection.query_row("SELECT record_json FROM work_order_confirmed_history WHERE tenant_id=?1 AND scope_kind=?2 AND scope_id=?3 AND work_id=?4 AND revision=?5", params![tenant,kind,scope,value.id.to_string(),revision], |r| r.get(0)).optional().map_err(|_| StorageError)?;
        if let Some(previous) = previous {
            return if previous == json {
                Ok(())
            } else {
                Err(StorageError)
            };
        }
        self.connection
            .execute(
                "INSERT INTO work_order_confirmed_history VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    tenant,
                    kind,
                    scope,
                    value.id.to_string(),
                    value.order_id.to_string(),
                    revision,
                    json
                ],
            )
            .map_err(|_| StorageError)?;
        insert_audit(self.connection, audit)?;
        insert_publication(
            self.connection,
            &audit.scope,
            IdempotencyKey::new(audit.audit_id),
            &DurablePublication {
                event: Event::OrderChanged(OrderNotice {
                    scope: audit.scope.clone(),
                    order_id: value.order_id,
                    revision: value.revision,
                }),
                policy_changed: false,
            },
        )
    }
}
impl AuthorityStore {
    /// # Errors
    /// Reads only the exact authenticated tenant and scope. Offline actions stay disabled.
    pub fn cached_work_orders(
        &self,
        actor: &AuthorizationContext,
        query: &ListWorkOrders,
    ) -> Result<WorkOrderPage, StorageError> {
        if !(1..=100).contains(&query.limit) {
            return Err(StorageError);
        }
        self.read_transaction(|tx| {
            let (kind,scope) = scope_parts(&actor.scope);
            let mut stmt = tx.prepare("SELECT h.record_json FROM work_order_confirmed_history h WHERE tenant_id=?1 AND scope_kind=?2 AND scope_id=?3 AND (?4 IS NULL OR work_id>?4) AND (?5 IS NULL OR order_id=?5) AND NOT EXISTS(SELECT 1 FROM work_order_confirmed_history n WHERE n.tenant_id=h.tenant_id AND n.scope_kind=h.scope_kind AND n.scope_id=h.scope_id AND n.work_id=h.work_id AND n.revision>h.revision) ORDER BY work_id LIMIT ?6").map_err(|_| StorageError)?;
            let bytes = stmt.query_map(params![actor.tenant_id.value().to_string(),kind,scope,query.after.map(|id|id.to_string()),query.order_id.map(|id|id.to_string()),i64::from(query.limit)+1],|r|r.get::<_,Vec<u8>>(0)).map_err(|_| StorageError)?.collect::<Result<Vec<_>,_>>().map_err(|_| StorageError)?;
            let more = bytes.len() > query.limit as usize;
            let items: Vec<WorkOrderRecord> = bytes.into_iter().take(query.limit as usize).map(|b|serde_json::from_slice(&b).map_err(|_|StorageError)).collect::<Result<_,_>>()?;
            let next = if more { items.last().map(|w| w.id) } else { None };
            Ok(WorkOrderPage { server_available: false, items, next, pending: vec![] })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{
        identity::*,
        order::WorkState,
        transport::{CorrelationId, UnixMillis},
    };
    use eitmad_observability_audit::AuditTarget;
    use uuid::Uuid;

    fn actor() -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::new_v4()),
            identity: AuthenticatedIdentity {
                principal_id: PrincipalId::new(Uuid::new_v4()),
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(Uuid::new_v4()),
            workspace_id: None,
            scope: ScopeRef {
                kind: ScopeKind::parse("branch").unwrap(),
                id: ScopeId::new(Uuid::new_v4()),
            },
        }
    }
    fn fixture(actor: &AuthorizationContext) -> WorkOrderRecord {
        WorkOrderRecord {
            id: Uuid::new_v4(),
            scope: actor.scope.clone(),
            organization_id: Uuid::new_v4(),
            order_id: Uuid::new_v4(),
            order_number: "OR-2026-00001".into(),
            revision: 1,
            number: "WO-2026-00001".into(),
            customer: "عميل تجريبي".into(),
            state: WorkState::Planned,
            due_at: None,
            assignment: None,
            furniture: vec![],
            note: None,
            can_start: true,
            can_complete: false,
        }
    }
    #[test]
    fn work_orders_confirmations_are_atomic_immutable_scoped_and_durable() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let actor = actor();
        let value = fixture(&actor);
        let query = ListWorkOrders {
            after: None,
            limit: 100,
            order_id: Some(value.order_id),
        };
        let audit = MutationAuditRecord::from_authorization(
            &actor,
            UnixMillis(1),
            CorrelationId::new(Uuid::new_v4()),
            "eitmad.work-order.cache.v1",
            AuditTarget {
                kind: "work-order".into(),
                identifiers: vec![value.id.to_string()],
            },
        );
        let connection = rusqlite::Connection::open(store.path()).unwrap();
        connection.execute_batch("CREATE TRIGGER fail_work_publication BEFORE INSERT ON publication_outbox BEGIN SELECT RAISE(ABORT,'synthetic failure'); END;").unwrap();
        assert!(
            store
                .transact_pricing(true, |tx| tx.cache_work_order(&value, &audit))
                .is_err()
        );
        assert!(
            store
                .cached_work_orders(&actor, &query)
                .unwrap()
                .items
                .is_empty()
        );
        connection
            .execute_batch("DROP TRIGGER fail_work_publication")
            .unwrap();
        store
            .transact_pricing(true, |tx| tx.cache_work_order(&value, &audit))
            .unwrap();
        store
            .transact_pricing(true, |tx| tx.cache_work_order(&value, &audit))
            .unwrap();
        let mut altered = value.clone();
        altered.note = Some("تغيير".into());
        assert!(
            store
                .transact_pricing(true, |tx| tx.cache_work_order(&altered, &audit))
                .is_err()
        );
        assert!(
            connection
                .execute("DELETE FROM work_order_confirmed_history", [])
                .is_err()
        );
        drop(connection);
        drop(store);
        let reopened = AuthorityStore::open(directory.path()).unwrap();
        let page = reopened.cached_work_orders(&actor, &query).unwrap();
        assert_eq!(page.items.len(), 1);
        assert!(!page.server_available);
        assert!(!page.items[0].can_start);
        assert_eq!(
            reopened
                .pending_publications(crate::MAX_PUBLICATION_RECOVERY_PAGE)
                .unwrap()
                .len(),
            1
        );
        for foreign in [
            AuthorizationContext {
                tenant_id: TenantId::new(Uuid::new_v4()),
                ..actor.clone()
            },
            AuthorizationContext {
                scope: ScopeRef {
                    id: ScopeId::new(Uuid::new_v4()),
                    ..actor.scope.clone()
                },
                ..actor.clone()
            },
        ] {
            assert!(
                reopened
                    .cached_work_orders(&foreign, &query)
                    .unwrap()
                    .items
                    .is_empty()
            );
        }
    }
}
