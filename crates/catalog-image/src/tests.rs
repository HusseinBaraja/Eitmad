use super::*;
use eitmad_contracts::{
    authorization::RelationshipSubject,
    identity::{
        AuthenticatedIdentity, PrincipalId, PrincipalKind, ScopeId, ScopeKind, SessionId, TenantId,
    },
    transport::{CorrelationId, IdempotencyKey, UnixMillis},
};
use tempfile::TempDir;

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
fn import_survives_source_removal_restart_and_exact_retry_without_path_diagnostics() {
    let dir = TempDir::new().unwrap();
    let (store, service, actor) = authority(&dir);
    let path = dir.path().join("private-synthetic-source.png");
    std::fs::write(&path, png(21, 13)).unwrap();
    let input = ImportCatalogImage {
        kind: CatalogImageKind::Product,
        source_path: path.to_str().unwrap().into(),
    };
    let saved = service.import(&mutation(&actor, 101), &input).unwrap();
    let original = store.catalog_image(&actor.scope, &saved).unwrap().unwrap();
    std::fs::remove_file(path).unwrap();
    drop(service);
    drop(store);
    let store = AuthorityStore::open(dir.path()).unwrap();
    let service = CatalogImageService::new(store.clone(), AuthorizationService::new(store));
    assert_eq!(
        service.import(&mutation(&actor, 101), &input),
        Ok(saved.clone())
    );
    let received = service
        .get(
            &actor,
            &GetCatalogImage {
                reference: saved,
                offset: 0,
            },
        )
        .unwrap();
    assert_eq!(STANDARD.decode(&received.base64).unwrap(), original);
    assert!(!format!("{input:?} {received:?}").contains("private-synthetic-source"));
    assert!(!format!("{received:?}").contains(&received.base64));
    let mut changed = input;
    changed.kind = CatalogImageKind::Furniture;
    assert_eq!(
        service.import(&mutation(&actor, 101), &changed),
        Err(ImageError::Invalid)
    );
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
            }
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
            }
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
            }
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
