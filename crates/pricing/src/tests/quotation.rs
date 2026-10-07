use super::*;
mod draft_sync;
use eitmad_contracts::{
    commands::CreateCustomer,
    customer::{CustomerName, CustomerPhone},
    quotation::{
        EvaluateQuotation, QuotationCustomerIntent, QuotationField as Field,
        QuotationIssue as Issue, QuotationLineIntent,
    },
    sales_catalog::CheckSalesConfiguration,
};

fn fixture_evaluation() -> (
    TempDir,
    AuthorityStore,
    PricingService,
    AuthorizationContext,
    EvaluateQuotation,
    eitmad_contracts::catalog_revision::CatalogEntry,
) {
    let dir = TempDir::new().unwrap();
    let (store, _, _, reader) = setup(&dir);
    let auth = AuthorizationService::new(store.clone());
    let mut owner = actor(99, 50);
    owner.scope = eitmad_contracts::identity::ScopeRef {
        kind: ScopeKind::parse("branch").unwrap(),
        id: ScopeId::new(Uuid::from_u128(500)),
    };
    let branch = AuthorizationContext {
        scope: owner.scope.clone(),
        ..reader.clone()
    };
    auth.bootstrap_owner(
        &mutation(owner.clone(), 500),
        &RelationshipSubject {
            principal_id: owner.identity.principal_id,
            principal_kind: PrincipalKind::User,
        },
    )
    .unwrap();
    auth.grant_relationship(
        &mutation(owner, 501),
        &GrantScopeRelationship {
            expected_policy_version: 1,
            subject: RelationshipSubject {
                principal_id: reader.identity.principal_id,
                principal_kind: PrincipalKind::User,
            },
            relation: RelationId::parse(RECEPTIONIST_RELATION).unwrap(),
        },
    )
    .unwrap();
    let customer = eitmad_customer::CustomerService::new(store.clone(), auth.clone())
        .create(
            &mutation(branch.clone(), 502),
            &CreateCustomer {
                name: CustomerName::parse("عميل تجريبي").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    let mut entry = published_product_entry();
    entry.price.selling_price_yer = 1010;
    project_sales(&store, &reader, vec![(1, 1, Some(entry.clone()))]);
    let input = EvaluateQuotation {
        customer: Some(QuotationCustomerIntent {
            id: customer.id,
            revision: customer.revision,
        }),
        lines: vec![QuotationLineIntent {
            id: Uuid::from_u128(600),
            configuration: CheckSalesConfiguration {
                selection: PriceSelection {
                    target: entry.price.target.clone(),
                    price_revision: entry.price.revision,
                    color_id: None,
                    handle_id: None,
                    quantity: 1,
                },
                dimensions: None,
            },
        }],
        discount_basis_points: 500,
    };
    (
        dir,
        store.clone(),
        PricingService::new(store, auth),
        branch,
        input,
        entry,
    )
}

#[test]
fn quotation_approved_example_rounds_once_and_never_grants_approval() {
    let (_dir, _, service, actor, mut input, entry) = fixture_evaluation();
    let evaluated = service.evaluate_quotation(&actor, &input).unwrap();
    assert!(evaluated.errors.is_empty());
    assert_eq!(evaluated.lines[0].name, entry.name);
    assert_eq!(evaluated.lines[0].price.snapshot, entry.price);
    assert_eq!(
        evaluated.totals,
        Some(DiscountTotal {
            subtotal_yer: 1010,
            discount_yer: 51,
            total_yer: 959,
            approval_required: false
        })
    );
    for (rate, amount, total, approval) in [
        (0, 0, 1010, false),
        (499, 50, 960, false),
        (501, 51, 959, true),
        (10000, 1010, 0, true),
    ] {
        input.discount_basis_points = rate;
        assert_eq!(
            service.evaluate_quotation(&actor, &input).unwrap().totals,
            Some(DiscountTotal {
                subtotal_yer: 1010,
                discount_yer: amount,
                total_yer: total,
                approval_required: approval
            })
        );
    }
    // Two 5-YER lines at 5% must produce one 1-YER discount, not two zero discounts.
    input.discount_basis_points = 500;
    let mut small = entry.clone();
    small.price.selling_price_yer = 5;
    small.price.revision = 2;
    input.lines[0].configuration.selection.price_revision = 2;
    let catalog_actor = service.authorize_quotation(&actor).unwrap();
    project_sales(&service.store, &catalog_actor, vec![(1, 2, Some(small))]);
    let mut second = input.lines[0].clone();
    second.id = Uuid::from_u128(601);
    input.lines.push(second);
    let totals = service
        .evaluate_quotation(&actor, &input)
        .unwrap()
        .totals
        .unwrap();
    assert_eq!(
        (totals.subtotal_yer, totals.discount_yer, totals.total_yer),
        (10, 1, 9)
    );
}

#[test]
fn quotation_reports_customer_line_and_discount_errors_without_partial_totals() {
    let (_dir, _, service, actor, mut input, _) = fixture_evaluation();
    input.customer.as_mut().unwrap().revision += 1;
    input.discount_basis_points = 10001;
    input.lines[0].configuration.selection.quantity = 0;
    let result = service.evaluate_quotation(&actor, &input).unwrap();
    assert!(result.totals.is_none());
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.field == Field::CustomerRevision && e.issue == Issue::Stale)
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.field == Field::DiscountBasisPoints)
    );
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.field == Field::Quantity && e.line_id == Some(input.lines[0].id))
    );
    input.customer = None;
    input.lines.clear();
    let result = service.evaluate_quotation(&actor, &input).unwrap();
    assert!(result.errors.iter().any(|e| e.field == Field::Customer));
    assert!(result.errors.iter().any(|e| e.field == Field::Lines));
}

#[test]
fn quotation_detects_changed_price_definition_withdrawal_and_customer() {
    let (_dir, store, service, actor, input, mut entry) = fixture_evaluation();
    let catalog_actor = service.authorize_quotation(&actor).unwrap();
    entry.price.revision += 1;
    entry.price.selling_price_yer += 1;
    project_sales(&store, &catalog_actor, vec![(1, 2, Some(entry.clone()))]);
    let stale = service.evaluate_quotation(&actor, &input).unwrap();
    assert!(stale.totals.is_none());
    assert_eq!(stale.errors[0].field, Field::PriceRevision);
    if let PriceTarget::Product(r) = &mut entry.price.target {
        r.revision += 1;
    }
    entry.price.revision += 1;
    project_sales(&store, &catalog_actor, vec![(1, 3, Some(entry))]);
    assert_eq!(
        service.evaluate_quotation(&actor, &input).unwrap().errors[0].field,
        Field::Target
    );
    project_sales(&store, &catalog_actor, vec![(1, 4, None)]);
    assert!(
        service
            .evaluate_quotation(&actor, &input)
            .unwrap()
            .totals
            .is_none()
    );
    let customer_service =
        eitmad_customer::CustomerService::new(store.clone(), AuthorizationService::new(store));
    customer_service
        .update(
            &mutation(actor.clone(), 503),
            &eitmad_contracts::commands::UpdateCustomer {
                customer_id: input.customer.as_ref().unwrap().id,
                expected_revision: 1,
                name: CustomerName::parse("عميل محدث").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap();
    assert!(
        service
            .evaluate_quotation(&actor, &input)
            .unwrap()
            .errors
            .iter()
            .any(|e| e.field == Field::CustomerRevision)
    );
}

#[test]
fn quotation_checks_identity_scope_duplicates_configuration_and_checked_totals() {
    let (_dir, store, service, actor, mut input, mut entry) = fixture_evaluation();
    let denied = AuthorizationContext {
        identity: AuthenticatedIdentity {
            principal_id: PrincipalId::new(Uuid::from_u128(999)),
            ..actor.identity.clone()
        },
        ..actor.clone()
    };
    assert_eq!(
        service.evaluate_quotation(&denied, &input),
        Err(PricingError::Denied)
    );
    let mut foreign = input.clone();
    if let PriceTarget::Product(r) = &mut foreign.lines[0].configuration.selection.target {
        r.scope.id = ScopeId::new(Uuid::from_u128(999));
    }
    assert_eq!(
        service.evaluate_quotation(&actor, &foreign),
        Err(PricingError::Denied)
    );
    let mut duplicate = input.clone();
    duplicate.lines.push(duplicate.lines[0].clone());
    assert_eq!(
        service
            .evaluate_quotation(&actor, &duplicate)
            .unwrap()
            .errors[0]
            .issue,
        Issue::Duplicate
    );
    input.lines[0].configuration.selection.color_id = Some(Uuid::new_v4());
    assert_eq!(
        service.evaluate_quotation(&actor, &input).unwrap().errors[0].field,
        Field::ColorId
    );
    input.lines[0].configuration.selection.color_id = None;
    let catalog_actor = service.authorize_quotation(&actor).unwrap();
    entry.price.selling_price_yer = i64::MAX;
    entry.price.revision = 2;
    input.lines[0].configuration.selection.price_revision = 2;
    project_sales(&store, &catalog_actor, vec![(1, 2, Some(entry))]);
    input.lines[0].configuration.selection.quantity = 2;
    assert_eq!(
        service.evaluate_quotation(&actor, &input).unwrap().errors[0].issue,
        Issue::Overflow
    );
    input.lines[0].configuration.selection.quantity = 1;
    let mut second = input.lines[0].clone();
    second.id = Uuid::from_u128(602);
    input.lines.push(second);
    let result = service.evaluate_quotation(&actor, &input).unwrap();
    assert!(result.totals.is_none());
    assert_eq!(result.errors[0].field, Field::Total);
}

#[test]
fn quotation_furniture_uses_public_adjustments_dimensions_and_changed_options() {
    let (_dir, store, service, actor, mut input, _) = fixture_evaluation();
    let mut entry = published_furniture_entry();
    let catalog_actor = service.authorize_quotation(&actor).unwrap();
    project_sales(&store, &catalog_actor, vec![(2, 1, Some(entry.clone()))]);
    input.lines[0].configuration = CheckSalesConfiguration {
        selection: PriceSelection {
            target: entry.price.target.clone(),
            price_revision: 1,
            color_id: Some(entry.colors[0].id),
            handle_id: Some(entry.handles[0].id),
            quantity: 2,
        },
        dimensions: entry.dimensions.clone(),
    };
    let evaluated = service.evaluate_quotation(&actor, &input).unwrap();
    assert!(evaluated.errors.is_empty());
    assert_eq!(evaluated.lines[0].price.unit_price_yer, 251_500);
    assert_eq!(evaluated.totals.as_ref().unwrap().subtotal_yer, 503_000);
    assert_eq!(
        evaluated.lines[0].color_name.as_ref(),
        Some(&entry.colors[0].name)
    );
    let payload = serde_json::to_string(&evaluated).unwrap();
    for forbidden in ["costYer", "marginYer", "parts", "notes"] {
        assert!(!payload.contains(forbidden));
    }
    input.lines[0]
        .configuration
        .dimensions
        .as_mut()
        .unwrap()
        .width_mm += 101;
    assert_eq!(
        service.evaluate_quotation(&actor, &input).unwrap().errors[0].field,
        Field::Dimensions
    );
    input.lines[0].configuration.dimensions = entry.dimensions.clone();
    entry.colors.clear();
    entry.price.colors.clear();
    entry.price.revision = 2;
    project_sales(&store, &catalog_actor, vec![(2, 2, Some(entry))]);
    input.lines[0].configuration.selection.price_revision = 2;
    let result = service.evaluate_quotation(&actor, &input).unwrap();
    assert!(result.totals.is_none());
    assert_eq!(result.errors[0].field, Field::ColorId);
}

#[test]
fn quotation_draft_restart_atomicity_retry_and_stale_inputs() {
    use crate::{QuotationDraftError, QuotationDraftService};
    use eitmad_contracts::quotation_draft::*;
    let (dir, store, _, actor, input, mut entry) = fixture_evaluation();
    let service =
        QuotationDraftService::new(store.clone(), AuthorizationService::new(store.clone()));
    let context = mutation(actor.clone(), 900);
    let command = CreateQuotationDraft {
        intent: input.clone(),
    };
    let created = service.create(&context, &command).unwrap();
    assert_eq!(
        created
            .snapshot
            .evaluation
            .totals
            .as_ref()
            .unwrap()
            .total_yer,
        959
    );
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    let service = QuotationDraftService::new(
        reopened.clone(),
        AuthorizationService::new(reopened.clone()),
    );
    assert_eq!(
        service
            .get(
                &actor,
                &GetQuotationDraft {
                    draft_id: created.snapshot.id
                }
            )
            .unwrap(),
        created
    );
    entry.price.revision = 2;
    entry.price.selling_price_yer = 2000;
    let catalog_actor = AuthorizationContext {
        scope: entry.price.target.scope().clone(),
        ..actor.clone()
    };
    project_sales(&store, &catalog_actor, vec![(1, 2, Some(entry.clone()))]);
    assert_eq!(service.create(&context, &command).unwrap(), created);
    let mut changed = command.clone();
    changed.intent.discount_basis_points = 0;
    assert_eq!(
        service.create(&context, &changed),
        Err(QuotationDraftError::IdempotencyMismatch)
    );
    let update = UpdateQuotationDraft {
        draft_id: created.snapshot.id,
        expected_revision: 1,
        intent: input.clone(),
    };
    assert!(
        matches!(service.update(&mutation(actor.clone(),901),&update),Err(QuotationDraftError::Validation(e)) if e.iter().any(|e| e.field==Field::PriceRevision && e.issue==Issue::Stale))
    );
    assert_eq!(
        service
            .get(
                &actor,
                &GetQuotationDraft {
                    draft_id: created.snapshot.id
                }
            )
            .unwrap(),
        created
    );
    let mut refreshed = update;
    refreshed.intent.lines[0]
        .configuration
        .selection
        .price_revision = 2;
    let updated = service
        .update(&mutation(actor.clone(), 902), &refreshed)
        .unwrap();
    assert_eq!(updated.snapshot.revision, 2);
    assert_eq!(
        service.update(&mutation(actor.clone(), 903), &refreshed),
        Err(QuotationDraftError::Conflict {
            expected: Some(1),
            actual: Some(2)
        })
    );
    verify_draft_atomic_rollback(&store, &service, &actor, &updated, refreshed);
}
fn verify_draft_atomic_rollback(
    store: &AuthorityStore,
    service: &crate::QuotationDraftService,
    actor: &AuthorizationContext,
    updated: &eitmad_contracts::quotation_draft::QuotationDraft,
    mut refreshed: eitmad_contracts::quotation_draft::UpdateQuotationDraft,
) {
    use crate::QuotationDraftError;
    use eitmad_contracts::quotation_draft::GetQuotationDraft;
    let tx = rusqlite::Connection::open(store.path()).unwrap();
    assert_eq!(
        tx.query_row("SELECT count(*) FROM quotation_draft_outbox", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    tx.execute_batch("CREATE TRIGGER fail_draft_publication BEFORE INSERT ON publication_outbox BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    refreshed.expected_revision = 2;
    let failed = mutation(actor.clone(), 904);
    assert_eq!(
        service.update(&failed, &refreshed),
        Err(QuotationDraftError::Unavailable)
    );
    assert_eq!(
        service
            .get(
                actor,
                &GetQuotationDraft {
                    draft_id: updated.snapshot.id
                }
            )
            .unwrap(),
        *updated
    );
    assert!(
        store
            .pending_publication(&actor.scope, failed.idempotency_key)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        tx.query_row("SELECT count(*) FROM quotation_draft_outbox", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        tx.query_row(
            "SELECT count(*) FROM idempotency_records WHERE idempotency_key=?1",
            [failed.idempotency_key.value().to_string()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}
#[test]
fn quotation_draft_visibility_denial_and_projection_preserve_both_edits() {
    use crate::{QuotationDraftError, QuotationDraftService};
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use eitmad_contracts::quotation_draft::*;
    let (_dir, store, _, actor, input, _) = fixture_evaluation();
    let auth = AuthorizationService::new(store.clone());
    let service = QuotationDraftService::new(store.clone(), auth.clone());
    let created = service
        .create(
            &mutation(actor.clone(), 910),
            &CreateQuotationDraft { intent: input },
        )
        .unwrap();
    verify_draft_permissions(&service, &auth, &actor, &created);
    let mut remote = service.sync_batch(&actor, 50).unwrap()[0].clone();
    remote.change_id = eitmad_contracts::sync::ChangeId::new(Uuid::new_v4());
    remote.idempotency_key = IdempotencyKey::new(Uuid::new_v4());
    let mut competing = created.snapshot.clone();
    competing.intent.discount_basis_points = 0;
    competing.evaluation.discount_basis_points = 0;
    competing.evaluation.totals = Some(
        crate::discount(&CalculateDiscount {
            line_totals_yer: vec![1010],
            discount_basis_points: 0,
        })
        .unwrap(),
    );
    remote.payload.as_mut().unwrap().base64 =
        STANDARD.encode(serde_json::to_vec(&competing).unwrap());
    assert!(
        service
            .project_confirmed(&actor, &remote, CorrelationId::new(Uuid::new_v4()))
            .unwrap()
    );
    let preserved = service
        .get(
            &actor,
            &GetQuotationDraft {
                draft_id: created.snapshot.id,
            },
        )
        .unwrap();
    assert_eq!(preserved.snapshot, created.snapshot);
    assert_eq!(preserved.sync_state, QuotationDraftSyncState::Conflicted);
    assert_eq!(
        preserved.conflict.as_ref().unwrap().remote,
        Some(remote.clone())
    );
    assert!(
        !service
            .project_confirmed(&actor, &remote, CorrelationId::new(Uuid::new_v4()))
            .unwrap()
    );
    assert!(service.sync_batch(&actor, 50).unwrap().is_empty());
    assert_eq!(
        service.update(
            &mutation(actor, 914),
            &UpdateQuotationDraft {
                draft_id: created.snapshot.id,
                expected_revision: 1,
                intent: created.snapshot.intent
            }
        ),
        Err(QuotationDraftError::UnresolvedConflict)
    );
}

fn verify_draft_permissions(
    service: &crate::QuotationDraftService,
    auth: &AuthorizationService,
    actor: &AuthorizationContext,
    created: &eitmad_contracts::quotation_draft::QuotationDraft,
) {
    use crate::QuotationDraftError;
    use eitmad_contracts::quotation_draft::*;
    let mut denied = actor.clone();
    denied.identity.principal_id = PrincipalId::new(Uuid::from_u128(7000));
    assert_eq!(
        service.get(
            &denied,
            &GetQuotationDraft {
                draft_id: created.snapshot.id
            }
        ),
        Err(QuotationDraftError::Denied)
    );
    assert_eq!(
        service.create(
            &mutation(denied, 911),
            &CreateQuotationDraft {
                intent: created.snapshot.intent.clone()
            }
        ),
        Err(QuotationDraftError::Denied)
    );
    let mut owner = actor.clone();
    owner.identity.principal_id = PrincipalId::new(Uuid::from_u128(99));
    auth.grant_relationship(
        &mutation(owner, 912),
        &GrantScopeRelationship {
            expected_policy_version: 2,
            subject: RelationshipSubject {
                principal_id: PrincipalId::new(Uuid::from_u128(8)),
                principal_kind: PrincipalKind::User,
            },
            relation: RelationId::parse(MANAGER_RELATION).unwrap(),
        },
    )
    .unwrap();
    let manager = AuthorizationContext {
        identity: eitmad_contracts::identity::AuthenticatedIdentity {
            principal_id: PrincipalId::new(Uuid::from_u128(8)),
            ..actor.identity.clone()
        },
        ..actor.clone()
    };
    assert_eq!(
        service
            .get(
                &manager,
                &GetQuotationDraft {
                    draft_id: created.snapshot.id
                }
            )
            .unwrap(),
        *created
    );
    assert_eq!(
        service.update(
            &mutation(manager, 913),
            &UpdateQuotationDraft {
                draft_id: created.snapshot.id,
                expected_revision: 1,
                intent: created.snapshot.intent.clone()
            }
        ),
        Err(QuotationDraftError::Denied)
    );
}

#[test]
fn quotation_draft_list_is_scoped_and_cursor_does_not_repeat_records() {
    use crate::{QuotationDraftError, QuotationDraftService};
    use eitmad_contracts::identity::ScopeRef;
    use eitmad_contracts::quotation_draft::*;
    let (_dir, store, _, actor, input, _) = fixture_evaluation();
    let service = QuotationDraftService::new(store.clone(), AuthorizationService::new(store));
    let command = CreateQuotationDraft { intent: input };
    let mut created = (920..923)
        .map(|key| {
            service
                .create(&mutation(actor.clone(), key), &command)
                .unwrap()
        })
        .collect::<Vec<_>>();
    created.sort_by_key(|draft| draft.snapshot.id.value());
    let first = service
        .list(
            &actor,
            &ListQuotationDrafts {
                after: None,
                limit: 2,
            },
        )
        .unwrap();
    assert_eq!(first.items, created[..2]);
    let next = service
        .list(
            &actor,
            &ListQuotationDrafts {
                after: first.next,
                limit: 2,
            },
        )
        .unwrap();
    assert_eq!(next.items, created[2..]);
    assert!(next.next.is_none());
    for limit in [0, 101] {
        assert_eq!(
            service.list(&actor, &ListQuotationDrafts { after: None, limit }),
            Err(QuotationDraftError::Invalid)
        );
    }
    let foreign = AuthorizationContext {
        scope: ScopeRef {
            id: ScopeId::new(Uuid::new_v4()),
            ..actor.scope.clone()
        },
        ..actor
    };
    assert_eq!(
        service.list(
            &foreign,
            &ListQuotationDrafts {
                after: None,
                limit: 2
            }
        ),
        Err(QuotationDraftError::Denied)
    );
}
