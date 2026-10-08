//! Immutable scoped confirmed orders and atomic audit/publication cache.
use crate::{
    AuthorityStore, DurablePublication, PricingTransaction, StorageError, insert_audit,
    insert_publication, migrations::Migration, scope_parts,
};
use eitmad_contracts::{
    events::Event,
    order::{OrderNotice, OrderPage, OrderRecord},
    transport::IdempotencyKey,
};
use eitmad_observability_audit::MutationAuditRecord;
use rusqlite::{OptionalExtension as _, params};

pub(crate) const MIGRATIONS: &[Migration] = &[Migration::new(27,"orders.confirmed-cache.v1","orders",
    "CREATE TABLE order_confirmed_history (
     tenant_id TEXT NOT NULL, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, order_id TEXT NOT NULL,
     revision INTEGER NOT NULL CHECK(revision>0), record_json BLOB NOT NULL,
     PRIMARY KEY(tenant_id,scope_kind,scope_id,order_id,revision));
     CREATE TABLE order_pending (tenant_id TEXT NOT NULL, principal_id TEXT NOT NULL, scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL, request_key TEXT NOT NULL, request_json BLOB NOT NULL, rejected_code TEXT, PRIMARY KEY(tenant_id,principal_id,scope_kind,scope_id,request_key));
     CREATE TRIGGER order_history_no_update BEFORE UPDATE ON order_confirmed_history BEGIN SELECT RAISE(ABORT,'immutable order'); END;
     CREATE TRIGGER order_history_no_delete BEFORE DELETE ON order_confirmed_history BEGIN SELECT RAISE(ABORT,'immutable order'); END;")];
impl PricingTransaction<'_> {
    /// # Errors
    /// Rejects contradictory confirmed history or a failed mandatory audit/publication write.
    pub fn cache_order(
        &self,
        value: &OrderRecord,
        audit: &MutationAuditRecord,
    ) -> Result<(), StorageError> {
        let (kind, id) = scope_parts(&audit.scope);
        let mut retained = value.clone();
        retained.permitted_actions.clear();
        // Offline order reads retain public readiness summaries only.
        for work in &mut retained.work {
            work.assignment = None;
        }
        let json = serde_json::to_vec(&retained).map_err(|_| StorageError)?;
        let revision = i64::try_from(value.revision).map_err(|_| StorageError)?;
        let tenant = audit.tenant_id.value().to_string();
        let existing:Option<Vec<u8>>=self.connection.query_row("SELECT record_json FROM order_confirmed_history WHERE tenant_id=?1 AND scope_kind=?2 AND scope_id=?3 AND order_id=?4 AND revision=?5",params![tenant,kind,id,value.id.to_string(),revision],|r|r.get(0)).optional().map_err(|_|StorageError)?;
        if let Some(existing) = existing {
            return if existing == json {
                Ok(())
            } else {
                Err(StorageError)
            };
        }
        self.connection
            .execute(
                "INSERT INTO order_confirmed_history VALUES(?1,?2,?3,?4,?5,?6)",
                params![tenant, kind, id, value.id.to_string(), revision, json],
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
                    order_id: value.id,
                    revision: value.revision,
                }),
                policy_changed: false,
            },
        )
    }
}
impl AuthorityStore {
    /// # Errors
    /// Reads only previously confirmed history from the exact authenticated tenant and scope.
    pub fn cached_orders(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
        after: Option<uuid::Uuid>,
        limit: u32,
        order_id: Option<uuid::Uuid>,
    ) -> Result<OrderPage, StorageError> {
        if !(1..=100).contains(&limit) {
            return Err(StorageError);
        }
        self.read_transaction(|tx|{
            let (kind,id)=scope_parts(&actor.scope);
            let mut statement=tx.prepare("SELECT h.record_json FROM order_confirmed_history h WHERE tenant_id=?1 AND scope_kind=?2 AND scope_id=?3 AND (?4 IS NULL OR order_id>?4) AND (?5 IS NULL OR order_id=?5) AND NOT EXISTS(SELECT 1 FROM order_confirmed_history n WHERE n.tenant_id=h.tenant_id AND n.scope_kind=h.scope_kind AND n.scope_id=h.scope_id AND n.order_id=h.order_id AND n.revision>h.revision) ORDER BY order_id LIMIT ?6").map_err(|_|StorageError)?;
            let bytes=statement.query_map(params![actor.tenant_id.value().to_string(),kind,id,after.map(|v|v.to_string()),order_id.map(|v|v.to_string()),i64::from(limit)+1],|r|r.get::<_,Vec<u8>>(0)).map_err(|_|StorageError)?.collect::<Result<Vec<_>,_>>().map_err(|_|StorageError)?;
            let more=bytes.len()>limit as usize;
            let items:Vec<OrderRecord>=bytes.into_iter().take(limit as usize).map(|b|serde_json::from_slice(&b).map_err(|_|StorageError)).collect::<Result<_,_>>()?;
            let next=if more{items.last().map(|v|v.id)}else{None};
            Ok(OrderPage{server_available:false,items,next,pending:vec![]})
        })
    }
}

impl AuthorityStore {
    /// # Errors
    /// Retains the exact request before transport; changed retry intent fails closed.
    pub fn stage_order(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
        request: &eitmad_contracts::order::ConfirmOrder,
    ) -> Result<(), StorageError> {
        self.transact_pricing(true,|tx|{
            let (kind,id)=scope_parts(&actor.scope); let json=serde_json::to_vec(request).map_err(|_|StorageError)?;
            let existing:Option<Vec<u8>>=tx.connection.query_row("SELECT request_json FROM order_pending WHERE tenant_id=?1 AND principal_id=?2 AND scope_kind=?3 AND scope_id=?4 AND request_key=?5",params![actor.tenant_id.value().to_string(),actor.identity.principal_id.value().to_string(),kind,id,request.idempotency_key.value().to_string()],|r|r.get(0)).optional().map_err(|_|StorageError)?;
            if let Some(existing)=existing {return if existing==json{Ok(())}else{Err(StorageError)};}
            tx.connection.execute("INSERT INTO order_pending VALUES(?1,?2,?3,?4,?5,?6,NULL)",params![actor.tenant_id.value().to_string(),actor.identity.principal_id.value().to_string(),kind,id,request.idempotency_key.value().to_string(),json]).map_err(|_|StorageError)?;Ok(())
        })
    }
    /// # Errors
    /// Persists rejected status or removes a resolved intent from the authenticated actor only.
    pub fn finish_order(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
        key: IdempotencyKey,
        rejected: Option<&str>,
    ) -> Result<(), StorageError> {
        self.transact_pricing(true,|tx|{
            let (kind,id)=scope_parts(&actor.scope);
            if let Some(code)=rejected {
                tx.connection.execute("UPDATE order_pending SET rejected_code=?6 WHERE tenant_id=?1 AND principal_id=?2 AND scope_kind=?3 AND scope_id=?4 AND request_key=?5",params![actor.tenant_id.value().to_string(),actor.identity.principal_id.value().to_string(),kind,id,key.value().to_string(),code]).map_err(|_|StorageError)?;
            }else{
                tx.connection.execute("DELETE FROM order_pending WHERE tenant_id=?1 AND principal_id=?2 AND scope_kind=?3 AND scope_id=?4 AND request_key=?5",params![actor.tenant_id.value().to_string(),actor.identity.principal_id.value().to_string(),kind,id,key.value().to_string()]).map_err(|_|StorageError)?;
            }Ok(())
        })
    }
    /// Returns at most 100 requests for the authenticated tenant, principal and scope.
    /// Unresolved requests come before rejected requests so rejected history cannot
    /// hide retryable intent. Each group is ordered by request key.
    /// See `docs/developer/subsystems/orders.md` for the pending-operation recovery flow.
    ///
    /// # Errors
    /// Returns an error if the scoped read or stored request decoding fails.
    pub fn pending_orders(
        &self,
        actor: &eitmad_contracts::identity::AuthorizationContext,
    ) -> Result<Vec<eitmad_contracts::order::OrderPending>, StorageError> {
        self.read_transaction(|tx|{
            let (kind,id)=scope_parts(&actor.scope);
            let mut stmt=tx.prepare("SELECT request_json,rejected_code FROM order_pending WHERE tenant_id=?1 AND principal_id=?2 AND scope_kind=?3 AND scope_id=?4 ORDER BY rejected_code IS NOT NULL,request_key LIMIT 100").map_err(|_|StorageError)?;
            stmt.query_map(params![actor.tenant_id.value().to_string(),actor.identity.principal_id.value().to_string(),kind,id],|r|Ok((r.get::<_,Vec<u8>>(0)?,r.get::<_,Option<String>>(1)?))).map_err(|_|StorageError)?.map(|r|{let (bytes,rejected_code)=r.map_err(|_|StorageError)?;Ok(eitmad_contracts::order::OrderPending{request:serde_json::from_slice(&bytes).map_err(|_|StorageError)?,rejected_code})}).collect()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eitmad_contracts::{
        identity::*,
        order::*,
        quotation::{EvaluateQuotation, QuotationEvaluation},
        quotation_draft::{QuotationDraftId, QuotationDraftSnapshot},
        quotation_lifecycle::{QuotationRecord, QuotationState},
        transport::{CorrelationId, UnixMillis},
    };
    use eitmad_observability_audit::AuditTarget;
    use uuid::Uuid;
    fn order(work: Vec<OrderWork>) -> OrderRecord {
        let scope = ScopeRef {
            kind: ScopeKind::parse("branch").unwrap(),
            id: ScopeId::new(Uuid::new_v4()),
        };
        let actor = PrincipalId::new(Uuid::new_v4());
        OrderRecord {
            id: Uuid::new_v4(),
            scope: scope.clone(),
            organization_id: Uuid::new_v4(),
            revision: 1,
            number: "OR-2026-00001".into(),
            state: OrderState::Ready,
            work,
            source: QuotationRecord {
                scope: scope.clone(),
                organization_id: Uuid::new_v4(),
                revision: 2,
                document_revision: 1,
                state: QuotationState::Accepted,
                quotation: QuotationDraftSnapshot {
                    cancelled: false,
                    id: QuotationDraftId::new(Uuid::new_v4()),
                    revision: 1,
                    intent: EvaluateQuotation {
                        customer: None,
                        lines: vec![],
                        discount_basis_points: 0,
                    },
                    evaluation: QuotationEvaluation {
                        scope,
                        customer: None,
                        lines: vec![],
                        currency: "YER".into(),
                        discount_basis_points: 0,
                        totals: None,
                        errors: vec![],
                        server_available: true,
                    },
                },
                number: Some("QT-2026-00001".into()),
                validity_days: 30,
                issued_at: Some(UnixMillis(1)),
                valid_until: Some(UnixMillis(1000)),
                approval_request_id: None,
                approval_fingerprint: None,
                changed_at: UnixMillis(1),
                changed_by: actor,
                cancellation_reason: None,
                acceptance: None,
                permitted_actions: vec![],
            },
            delivery: None,
            fulfillment_note: None,
            cancellation_reason: None,
            created_at: UnixMillis(1),
            changed_at: UnixMillis(1),
            changed_by: actor,
            permitted_actions: vec![],
        }
    }

    fn actor(value: &OrderRecord) -> AuthorizationContext {
        AuthorizationContext {
            session_id: SessionId::new(Uuid::new_v4()),
            identity: AuthenticatedIdentity {
                principal_id: value.changed_by,
                principal_kind: PrincipalKind::User,
                device_id: None,
                service_id: None,
            },
            tenant_id: TenantId::new(value.organization_id),
            workspace_id: None,
            scope: value.scope.clone(),
        }
    }
    #[test]
    fn pending_orders_survive_restart_and_remain_bound_to_exact_actor_and_request() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let value = order(vec![]);
        let actor = actor(&value);
        let request = ConfirmOrder {
            scope: actor.scope.clone(),
            idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
            action: OrderAction::Cancel(CancelOrder {
                order_id: value.id,
                expected_revision: 1,
                reason: "اختبار".into(),
            }),
        };
        store.stage_order(&actor, &request).unwrap();
        store.stage_order(&actor, &request).unwrap();
        let mut changed = request.clone();
        changed.action = OrderAction::Cancel(CancelOrder {
            order_id: value.id,
            expected_revision: 2,
            reason: "تغيير".into(),
        });
        assert!(store.stage_order(&actor, &changed).is_err());
        let mut foreign = actor.clone();
        foreign.tenant_id = TenantId::new(Uuid::new_v4());
        assert!(store.pending_orders(&foreign).unwrap().is_empty());
        foreign = actor.clone();
        foreign.identity.principal_id = PrincipalId::new(Uuid::new_v4());
        assert!(store.pending_orders(&foreign).unwrap().is_empty());
        foreign = actor.clone();
        foreign.scope.id = ScopeId::new(Uuid::new_v4());
        assert!(store.pending_orders(&foreign).unwrap().is_empty());
        drop(store);
        let reopened = AuthorityStore::open(directory.path()).unwrap();
        assert_eq!(reopened.pending_orders(&actor).unwrap()[0].request, request);
        reopened
            .finish_order(
                &actor,
                request.idempotency_key,
                Some("eitmad.error.order-conflict.v1"),
            )
            .unwrap();
        drop(reopened);
        let reopened = AuthorityStore::open(directory.path()).unwrap();
        assert!(
            reopened.pending_orders(&actor).unwrap()[0]
                .rejected_code
                .is_some()
        );
        reopened
            .finish_order(&actor, request.idempotency_key, None)
            .unwrap();
        assert!(reopened.pending_orders(&actor).unwrap().is_empty());
    }
    #[test]
    fn pending_recovery_is_not_hidden_by_a_full_rejected_history_page() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let value = order(vec![]);
        let actor = actor(&value);
        let mut request = ConfirmOrder {
            scope: actor.scope.clone(),
            idempotency_key: IdempotencyKey::new(Uuid::from_u128(1)),
            action: OrderAction::Convert(ConvertQuotation {
                draft_id: value.source.quotation.id,
                expected_revision: 1,
            }),
        };
        for key in 1..=100 {
            request.idempotency_key = IdempotencyKey::new(Uuid::from_u128(key));
            store.stage_order(&actor, &request).unwrap();
            store
                .finish_order(
                    &actor,
                    request.idempotency_key,
                    Some("eitmad.error.order-conflict.v1"),
                )
                .unwrap();
        }
        request.idempotency_key = IdempotencyKey::new(Uuid::from_u128(101));
        store.stage_order(&actor, &request).unwrap();
        let page = store.pending_orders(&actor).unwrap();
        assert_eq!(page.len(), 100);
        assert_eq!(page[0].request, request);
        assert!(page[0].rejected_code.is_none());
    }
    #[test]
    fn confirmed_orders_atomically_retain_audit_publication_and_scoped_history() {
        let directory = tempfile::tempdir().unwrap();
        let store = AuthorityStore::open(directory.path()).unwrap();
        let mut value = order(vec![]);
        let actor = actor(&value);
        value.permitted_actions = vec![OrderPermittedAction::Deliver];
        let audit = MutationAuditRecord::from_authorization(
            &actor,
            UnixMillis(1),
            CorrelationId::new(Uuid::new_v4()),
            "eitmad.order.cache.v1",
            AuditTarget {
                kind: "order".into(),
                identifiers: vec![value.id.to_string()],
            },
        );
        let connection = rusqlite::Connection::open(store.path()).unwrap();
        connection.execute_batch("CREATE TRIGGER fail_order_publication BEFORE INSERT ON publication_outbox BEGIN SELECT RAISE(ABORT,'test publication failure'); END;").unwrap();
        assert!(
            store
                .transact_pricing(true, |tx| tx.cache_order(&value, &audit))
                .is_err()
        );
        assert!(
            store
                .cached_orders(&actor, None, 100, None)
                .unwrap()
                .items
                .is_empty()
        );
        connection
            .execute_batch("DROP TRIGGER fail_order_publication;")
            .unwrap();
        store
            .transact_pricing(true, |tx| tx.cache_order(&value, &audit))
            .unwrap();
        store
            .transact_pricing(true, |tx| tx.cache_order(&value, &audit))
            .unwrap();
        assert_eq!(
            store
                .pending_publications(crate::MAX_PUBLICATION_RECOVERY_PAGE)
                .unwrap()
                .len(),
            1
        );
        let mut contradiction = value.clone();
        contradiction.fulfillment_note = Some("تغيير".into());
        assert!(
            store
                .transact_pricing(true, |tx| tx.cache_order(&contradiction, &audit))
                .is_err()
        );
        drop(connection);
        drop(store);
        let reopened = AuthorityStore::open(directory.path()).unwrap();
        let page = reopened
            .cached_orders(&actor, None, 100, Some(value.id))
            .unwrap();
        assert!(!page.server_available);
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].permitted_actions.is_empty());
        assert_eq!(page.items[0].source, value.source);
        let mut foreign = actor.clone();
        foreign.tenant_id = TenantId::new(Uuid::new_v4());
        assert!(
            reopened
                .cached_orders(&foreign, None, 100, None)
                .unwrap()
                .items
                .is_empty()
        );
        assert!(reopened.cached_orders(&actor, None, 101, None).is_err());
        assert_eq!(
            reopened
                .pending_publications(crate::MAX_PUBLICATION_RECOVERY_PAGE)
                .unwrap()
                .len(),
            1
        );
    }
}
