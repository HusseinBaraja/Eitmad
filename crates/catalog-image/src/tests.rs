use super::*;
use eitmad_contracts::{
    authorization::RelationshipSubject,
    identity::{
        AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId,
    },
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
use tempfile::TempDir;

#[test]
fn rejected_and_unauthorized_upload_pages_do_not_starve_valid_work_after_restart() {
    struct Transfer;
    impl CatalogImageTransfer for Transfer {
        fn upload(
            &self,
            _: &AuthorizationContext,
            image: &CatalogImageRef,
            _: &[u8],
        ) -> Result<(), ImageError> {
            if image.kind == CatalogImageKind::Furniture {
                Err(ImageError::Invalid)
            } else {
                Ok(())
            }
        }
        fn download(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: UnixMillis,
        ) -> Result<Vec<u8>, ImageError> {
            Err(ImageError::NotFound)
        }
    }
    let dir = TempDir::new().unwrap();
    let (store, service, actor) = authority(&dir);
    let mut denied = actor.clone();
    denied.scope.id = ScopeId::new(uuid::Uuid::from_u128(3));
    for index in 1..=17 {
        let content = normalize(&png(index, 2)).unwrap();
        let image = reference(CatalogImageKind::Product, &content);
        let audit = MutationAuditRecord::from_authorization(
            &denied,
            UnixMillis(1000),
            CorrelationId::new(uuid::Uuid::new_v4()),
            "eitmad.catalog-image.import.v1",
            AuditTarget {
                kind: "catalog-image".into(),
                identifiers: vec![image.id.to_string()],
            },
        );
        store
            .insert_catalog_image(&denied.scope, &image, &content, &audit, None, Some(&denied))
            .unwrap();
    }
    let path = dir.path().join("synthetic.png");
    for (key, kind) in [
        (401, CatalogImageKind::Furniture),
        (402, CatalogImageKind::Product),
    ] {
        std::fs::write(&path, png(7, 9)).unwrap();
        service
            .import(
                &mutation(&actor, key),
                &ImportCatalogImage {
                    kind,
                    source_path: path.to_str().unwrap().into(),
                },
                UnixMillis(i64::MAX),
            )
            .unwrap();
    }
    // Ensure the server rejection is attempted before the valid row.
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    connection.execute("UPDATE catalog_image_uploads SET next_attempt_at=1 WHERE scope_id=?1 AND CAST(reference_json AS TEXT) LIKE '%product%'", [actor.scope.id.value().to_string()]).unwrap();
    let service = service.with_transfer(Arc::new(Transfer));
    connection.execute_batch("CREATE TRIGGER block_deferral_audit BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.catalog-image.upload-deferred.v1' BEGIN SELECT RAISE(ABORT,'blocked'); END;").unwrap();
    assert_eq!(service.retry_uploads(), Err(ImageError::Unavailable));
    let deferred: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM catalog_image_uploads WHERE next_attempt_at>1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(deferred, 0);
    connection
        .execute_batch("DROP TRIGGER block_deferral_audit;")
        .unwrap();
    assert_eq!(service.retry_uploads(), Ok(0));
    drop(service);
    drop(store);
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    let service = CatalogImageService::new(
        reopened.clone(),
        AuthorizationService::new(reopened.clone()),
    )
    .with_transfer(Arc::new(Transfer));
    assert_eq!(service.retry_uploads(), Ok(1));
    assert!(reopened.pending_catalog_images().unwrap().is_empty());
    let deferred: i64 = connection
        .query_row("SELECT COUNT(*) FROM catalog_image_uploads", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(deferred, 18);
}

#[test]
fn expired_import_does_not_commit_and_expired_download_does_not_cache() {
    struct Expired;
    impl CatalogImageTransfer for Expired {
        fn upload(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: &[u8],
        ) -> Result<(), ImageError> {
            Ok(())
        }
        fn download(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: UnixMillis,
        ) -> Result<Vec<u8>, ImageError> {
            std::thread::sleep(std::time::Duration::from_millis(40));
            Ok(png(2, 2))
        }
    }
    let dir = TempDir::new().unwrap();
    let (store, service, actor) = authority(&dir);
    let path = dir.path().join("synthetic.png");
    std::fs::write(&path, png(2, 2)).unwrap();
    assert_eq!(
        service.import(
            &mutation(&actor, 403),
            &ImportCatalogImage {
                kind: CatalogImageKind::Product,
                source_path: path.to_str().unwrap().into()
            },
            UnixMillis(0)
        ),
        Err(ImageError::Unavailable)
    );
    assert!(
        store
            .catalog_image_retry(&actor.scope, mutation(&actor, 403).idempotency_key)
            .unwrap()
            .is_none()
    );
    let image = reference(CatalogImageKind::Product, &png(2, 2));
    let service = service.with_transfer(Arc::new(Expired));
    assert_eq!(
        service.get(
            &actor,
            &GetCatalogImage {
                reference: image.clone(),
                offset: 0
            },
            UnixMillis(eitmad_authorization::now().0 + 30)
        ),
        Err(ImageError::Unavailable)
    );
    assert!(store.catalog_image(&actor.scope, &image).unwrap().is_none());
}

fn actor() -> AuthorizationContext {
    AuthorizationContext {
        session_id: SessionId::new(uuid::Uuid::from_u128(1)),
        identity: AuthenticatedIdentity {
            principal_id: PrincipalId::new(uuid::Uuid::from_u128(2)),
            principal_kind: PrincipalKind::User,
            device_id: None,
            service_id: None,
        },
        tenant_id: TenantId::new(uuid::Uuid::from_u128(4)),
        workspace_id: None,
        scope: eitmad_contracts::identity::ScopeRef {
            kind: ScopeKind::parse("organization").unwrap(),
            id: ScopeId::new(uuid::Uuid::from_u128(4)),
        },
    }
}
fn mutation(context: &AuthorizationContext, key: u128) -> MutationContext {
    MutationContext {
        authorization: context.clone(),
        correlation_id: CorrelationId::new(uuid::Uuid::from_u128(key)),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(uuid::Uuid::from_u128(key)),
        occurred_at: UnixMillis(1000),
    }
}
fn authority(dir: &TempDir) -> (AuthorityStore, CatalogImageService, AuthorizationContext) {
    let store = AuthorityStore::open(dir.path()).unwrap();
    let auth = AuthorizationService::new(store.clone());
    let actor = actor();
    auth.bootstrap_owner(
        &mutation(&actor, 100),
        &RelationshipSubject {
            principal_id: actor.identity.principal_id,
            principal_kind: PrincipalKind::User,
        },
    )
    .unwrap();
    auth.grant_relationship(
        &mutation(&actor, 1001),
        &eitmad_contracts::commands::GrantScopeRelationship {
            expected_policy_version: 1,
            subject: RelationshipSubject {
                principal_id: actor.identity.principal_id,
                principal_kind: PrincipalKind::User,
            },
            relation: eitmad_contracts::authorization::RelationId::parse(
                eitmad_authorization::MANAGER_RELATION,
            )
            .unwrap(),
        },
    )
    .unwrap();
    (store.clone(), CatalogImageService::new(store, auth), actor)
}
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(width, height)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

#[test]
fn import_survives_source_removal_upgrade_restart_and_exact_retry_without_path_diagnostics() {
    let dir = TempDir::new().unwrap();
    let (store, service, actor) = authority(&dir);
    let path = dir.path().join("private-synthetic-source.png");
    std::fs::write(&path, png(21, 13)).unwrap();
    let input = ImportCatalogImage {
        kind: CatalogImageKind::Product,
        source_path: path.to_str().unwrap().into(),
    };
    let saved = service
        .import(
            &mutation(&actor, 101),
            &input,
            eitmad_contracts::transport::UnixMillis(i64::MAX),
        )
        .unwrap();
    let original = store.catalog_image(&actor.scope, &saved).unwrap().unwrap();
    std::fs::remove_file(path).unwrap();
    // Reproduce the pre-deferral schema with an already-retained asset and retry evidence.
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    connection
        .execute_batch(
            "DROP TABLE work_order_confirmed_history;
        DELETE FROM schema_migrations WHERE version=28;
        DROP TABLE order_confirmed_history;
        DROP TABLE order_pending;
        DELETE FROM schema_migrations WHERE version=27;
        DROP TABLE quotation_confirmed_history;
        DELETE FROM schema_migrations WHERE version=26;
        DROP TABLE quotation_draft_outbox;
        DROP TABLE quotation_drafts;
        DELETE FROM schema_migrations WHERE version=24;
        ALTER TABLE sync_scopes RENAME TO sync_scopes_v25;
        CREATE TABLE sync_scopes (
            scope_kind TEXT NOT NULL, scope_id TEXT NOT NULL,
            application_mode TEXT NOT NULL CHECK (application_mode IN ('local-first','server-authoritative')),
            state_version INTEGER NOT NULL, revision INTEGER NOT NULL, state_json BLOB NOT NULL,
            PRIMARY KEY (scope_kind, scope_id)
        );
        INSERT INTO sync_scopes(scope_kind,scope_id,application_mode,state_version,revision,state_json)
            SELECT scope_kind,scope_id,application_mode,state_version,revision,state_json
            FROM sync_scopes_v25 GROUP BY scope_kind,scope_id;
        DROP TABLE sync_scopes_v25;
        DELETE FROM schema_migrations WHERE version=25;
        DROP TABLE catalog_sync_exceptions;
        DELETE FROM schema_migrations WHERE version=23;
        DROP TABLE catalog_sync_clients;
        DROP TABLE catalog_sales_records;
        DROP TABLE catalog_sync_checkpoints;
        DROP TABLE catalog_sync_revisions;
        DELETE FROM schema_migrations WHERE version=22;
        DROP TABLE pricing_intents;
        DROP TABLE pricing_revisions;
        DELETE FROM schema_migrations WHERE version=21;
        DROP INDEX catalog_image_upload_due;
        ALTER TABLE catalog_image_uploads DROP COLUMN next_attempt_at;
        DELETE FROM schema_migrations WHERE version=20;",
        )
        .unwrap();
    drop(connection);
    drop(service);
    drop(store);
    let store = AuthorityStore::open(dir.path()).unwrap();
    let service = CatalogImageService::new(store.clone(), AuthorizationService::new(store));
    assert_eq!(
        service.import(
            &mutation(&actor, 101),
            &input,
            eitmad_contracts::transport::UnixMillis(i64::MAX)
        ),
        Ok(saved.clone())
    );
    let received = service
        .get(
            &actor,
            &GetCatalogImage {
                reference: saved,
                offset: 0,
            },
            eitmad_contracts::transport::UnixMillis(i64::MAX),
        )
        .unwrap();
    assert_eq!(STANDARD.decode(&received.base64).unwrap(), original);
    assert!(!format!("{input:?} {received:?}").contains("private-synthetic-source"));
    assert!(!format!("{received:?}").contains(&received.base64));
    let mut changed = input;
    changed.kind = CatalogImageKind::Furniture;
    assert_eq!(
        service.import(
            &mutation(&actor, 101),
            &changed,
            eitmad_contracts::transport::UnixMillis(i64::MAX)
        ),
        Err(ImageError::Invalid)
    );
}

#[test]
fn receptionist_cannot_read_known_private_product_or_furniture_images() {
    let dir = TempDir::new().unwrap();
    let (store, service, manager) = authority(&dir);
    let mut receptionist = manager.clone();
    receptionist.identity.principal_id = PrincipalId::new(uuid::Uuid::from_u128(99));
    AuthorizationService::new(store)
        .grant_relationship(
            &mutation(&manager, 1002),
            &eitmad_contracts::commands::GrantScopeRelationship {
                expected_policy_version: 2,
                subject: RelationshipSubject {
                    principal_id: receptionist.identity.principal_id,
                    principal_kind: PrincipalKind::User,
                },
                relation: eitmad_contracts::authorization::RelationId::parse(
                    eitmad_authorization::RECEPTIONIST_RELATION,
                )
                .unwrap(),
            },
        )
        .unwrap();
    let path = dir.path().join("private.png");
    std::fs::write(&path, png(3, 2)).unwrap();
    for (key, kind) in [
        (110, CatalogImageKind::Product),
        (111, CatalogImageKind::Furniture),
    ] {
        let saved = service
            .import(
                &mutation(&manager, key),
                &ImportCatalogImage {
                    kind,
                    source_path: path.to_str().unwrap().into(),
                },
                UnixMillis(i64::MAX),
            )
            .unwrap();
        assert_eq!(
            service.get(
                &receptionist,
                &GetCatalogImage {
                    reference: saved,
                    offset: 0
                },
                UnixMillis(i64::MAX),
            ),
            Err(ImageError::Denied)
        );
    }
}

#[test]
fn invalid_images_bounds_forged_references_and_unauthorized_reads_fail() {
    assert_eq!(normalize(b"not an image"), Err(ImageError::Invalid));
    assert_eq!(
        normalize(&vec![0; MAX_IMAGE_BYTES + 1]),
        Err(ImageError::Invalid)
    );
    assert_eq!(normalize(&png(8193, 1)), Err(ImageError::Invalid));
    assert_eq!(normalize(&png(8192, 2049)), Err(ImageError::Invalid));
    assert_eq!(normalize(&png(10, 10)[..24]), Err(ImageError::Invalid));
    let bytes = normalize(&png(9, 8)).unwrap();
    let mut forged = reference(CatalogImageKind::Furniture, &bytes);
    forged.sha256 = "00".repeat(32);
    assert_eq!(validate_asset(&forged, &bytes), Err(ImageError::Invalid));
    let dir = TempDir::new().unwrap();
    let (_, service, mut actor) = authority(&dir);
    actor.identity.principal_id = PrincipalId::new(uuid::Uuid::from_u128(99));
    assert_eq!(
        service.get(
            &actor,
            &GetCatalogImage {
                reference: reference(CatalogImageKind::Product, &bytes),
                offset: 0
            },
            eitmad_contracts::transport::UnixMillis(i64::MAX)
        ),
        Err(ImageError::Denied)
    );
    assert_eq!(
        chunk(&forged, &bytes, u32::try_from(bytes.len()).unwrap()),
        Err(ImageError::Invalid)
    );
}

#[test]
fn offline_import_retains_upload_work_across_restart_until_authenticated_acknowledgement() {
    struct Transfer {
        available: bool,
        expected: Vec<u8>,
    }
    impl CatalogImageTransfer for Transfer {
        fn upload(
            &self,
            _: &AuthorizationContext,
            image: &CatalogImageRef,
            content: &[u8],
        ) -> Result<(), ImageError> {
            assert_eq!(content, self.expected);
            validate_asset(image, content)?;
            if self.available {
                Ok(())
            } else {
                Err(ImageError::Unavailable)
            }
        }
        fn download(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: UnixMillis,
        ) -> Result<Vec<u8>, ImageError> {
            Err(ImageError::Unavailable)
        }
    }
    let dir = TempDir::new().unwrap();
    let (store, service, actor) = authority(&dir);
    let path = dir.path().join("temporary-source.png");
    std::fs::write(&path, png(23, 17)).unwrap();
    let image = service
        .import(
            &mutation(&actor, 201),
            &ImportCatalogImage {
                kind: CatalogImageKind::Product,
                source_path: path.to_str().unwrap().into(),
            },
            eitmad_contracts::transport::UnixMillis(i64::MAX),
        )
        .unwrap();
    std::fs::remove_file(path).unwrap();
    let expected = store.catalog_image(&actor.scope, &image).unwrap().unwrap();
    let service = service.with_transfer(Arc::new(Transfer {
        available: false,
        expected: expected.clone(),
    }));
    assert_eq!(service.retry_uploads(), Ok(0));
    assert!(
        service
            .attach(&actor, CatalogImageKind::Product, Some(&image))
            .is_ok()
    );
    drop(service);
    drop(store);
    let reopened = AuthorityStore::open(dir.path()).unwrap();
    assert_eq!(reopened.pending_catalog_images().unwrap().len(), 1);
    let service = CatalogImageService::new(
        reopened.clone(),
        AuthorizationService::new(reopened.clone()),
    )
    .with_transfer(Arc::new(Transfer {
        available: true,
        expected,
    }));
    assert_eq!(service.retry_uploads(), Ok(1));
    assert!(reopened.pending_catalog_images().unwrap().is_empty());
    assert!(
        reopened
            .catalog_image(&actor.scope, &image)
            .unwrap()
            .is_some()
    );
}

#[test]
fn receiving_authority_rejects_tampered_remote_assets_before_caching() {
    struct Tampered;
    impl CatalogImageTransfer for Tampered {
        fn upload(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: &[u8],
        ) -> Result<(), ImageError> {
            Ok(())
        }
        fn download(
            &self,
            _: &AuthorizationContext,
            _: &CatalogImageRef,
            _: UnixMillis,
        ) -> Result<Vec<u8>, ImageError> {
            Ok(png(1, 1))
        }
    }
    let dir = TempDir::new().unwrap();
    let (store, service, actor) = authority(&dir);
    let reference = reference(CatalogImageKind::Product, &normalize(&png(2, 2)).unwrap());
    let service = service.with_transfer(Arc::new(Tampered));
    assert_eq!(
        service.get(
            &actor,
            &GetCatalogImage {
                reference: reference.clone(),
                offset: 0
            },
            eitmad_contracts::transport::UnixMillis(i64::MAX)
        ),
        Err(ImageError::Invalid)
    );
    assert!(
        store
            .catalog_image(&actor.scope, &reference)
            .unwrap()
            .is_none()
    );
}

#[test]
fn mandatory_audit_failure_rolls_back_asset_retry_and_pending_upload() {
    let dir = TempDir::new().unwrap();
    let (store, service, actor) = authority(&dir);
    let connection = rusqlite::Connection::open(store.path()).unwrap();
    connection.execute_batch("CREATE TRIGGER block_image_audit BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.catalog-image.import.v1' BEGIN SELECT RAISE(ABORT,'blocked'); END;").unwrap();
    let bytes = png(13, 11);
    let expected = reference(CatalogImageKind::Product, &normalize(&bytes).unwrap());
    let path = dir.path().join("synthetic-source.png");
    std::fs::write(&path, bytes).unwrap();
    let mutation = mutation(&actor, 301);
    assert_eq!(
        service.import(
            &mutation,
            &ImportCatalogImage {
                kind: CatalogImageKind::Product,
                source_path: path.to_str().unwrap().into()
            },
            eitmad_contracts::transport::UnixMillis(i64::MAX)
        ),
        Err(ImageError::Unavailable)
    );
    assert!(
        store
            .catalog_image(&actor.scope, &expected)
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .catalog_image_retry(&actor.scope, mutation.idempotency_key)
            .unwrap()
            .is_none()
    );
    assert!(store.pending_catalog_images().unwrap().is_empty());
}
