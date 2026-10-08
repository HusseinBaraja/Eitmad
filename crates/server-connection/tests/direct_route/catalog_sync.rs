use super::*;
use base64::engine::general_purpose::STANDARD;
use eitmad_contracts::{
    catalog_revision::CatalogRevision,
    furniture::*,
    identity::UserId,
    material::*,
    part::*,
    pricing::*,
    product::*,
    sales_catalog::{CheckSalesConfiguration, GetSalesCatalogItem, ListSalesCatalog},
};
use eitmad_pricing::{CatalogReplication, PriceConfirmation, PricingError, PricingService};
fn mutation(actor: &AuthorizationContext) -> MutationContext {
    MutationContext {
        authorization: actor.clone(),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        occurred_at: eitmad_authorization::now(),
    }
}
fn replication(
    directory: &Path,
    store: AuthorityStore,
    authentication: AuthenticationResult,
    seed: [u8; 32],
    scope: &ScopeRef,
    endpoint: &str,
    trust: &Path,
) -> Arc<eitmad_server_connection::DirectCatalogSyncClient> {
    let secrets = SecretStore::open(
        directory.join("sync-secrets"),
        Some(FallbackEncryptionKey::new([7; 32])),
    )
    .unwrap();
    let credential = SecretId::new(
        SecretKind::parse("catalog-sync-test").unwrap(),
        SecretReferenceId::new(Uuid::new_v4()),
    );
    store_session(&secrets, &credential, authentication, seed).unwrap();
    let config = DirectServerConfig::new(
        endpoint,
        scope.clone(),
        SchemaId::parse("eitmad.schema.catalog-public.v1").unwrap(),
        1,
        trust,
    )
    .unwrap();
    Arc::new(
        eitmad_server_connection::DirectCatalogSyncClient::from_config(
            config, secrets, credential, store,
        ),
    )
}
/// Runs blocking WAN synchronization outside the asynchronous test executor.
async fn cycle(
    client: Arc<eitmad_server_connection::DirectCatalogSyncClient>,
    actor: AuthorizationContext,
) -> Result<usize, PricingError> {
    tokio::task::spawn_blocking(move || {
        client.synchronize(&actor, UnixMillis(eitmad_authorization::now().0 + 30_000))
    })
    .await
    .unwrap()
}
async fn receptionist(database: &str, server: &ProvisionedServer) -> AuthenticationResult {
    let pool = SyncDatabase::connect(database, 2).await.unwrap().pool();
    let user = UserId::new(Uuid::new_v4());
    let account = eitmad_contracts::identity::AccountId::new(Uuid::new_v4());
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
        .bind(server.authentication.session.tenant_id.value().to_string())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.users VALUES($1,$2,1)")
        .bind(server.authentication.session.tenant_id.value())
        .bind(user.value())
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO control.accounts(tenant_id,account_id,user_id,username,canonical_username,status,password_hash,created_at,activated_at) SELECT tenant_id,$2,$3,'reception','reception','active',password_hash,1,1 FROM control.accounts WHERE tenant_id=$1 AND account_id=$4")
        .bind(server.authentication.session.tenant_id.value()).bind(account.value()).bind(user.value()).bind(server.authentication.session.account_id.value()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO control.relationship_tuples(tenant_id,subject_principal_id,subject_kind,relation,object_kind,object_id,created_at) VALUES($1,$2,'user','eitmad.relation.organization.receptionist.v1','organization',$3,1)")
        .bind(server.authentication.session.tenant_id.value()).bind(user.value()).bind(server.scope.id.value()).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let signing = SigningKey::from_bytes(&[19; 32]);
    ControlPlane::new(pool, TokenKey::new([9; 32]))
        .authentication
        .login(
            &LoginRequest {
                tenant_code: TenantCode::parse("direct-test").unwrap(),
                username: "reception".into(),
                password: "synthetic-test-password-123".into(),
                device_id: DeviceId::new(Uuid::new_v4()),
                device_label: "استقبال الاختبار".into(),
                device_public_key: DevicePublicKey {
                    algorithm: "ed25519".into(),
                    base64: URL_SAFE_NO_PAD.encode(signing.verifying_key().as_bytes()),
                },
                device_proof: None,
            },
            CorrelationId::new(Uuid::new_v4()),
            eitmad_authorization::now(),
        )
        .await
        .unwrap()
}
fn receptionist_authority(
    directory: &Path,
    session: &AuthenticatedServerSession,
) -> (AuthorityStore, AuthorizationContext) {
    let (store, owner) = catalog_local_authority(directory, session);
    let auth = AuthorizationService::new(store.clone());
    auth.revoke_relationship(
        &mutation(&owner),
        &eitmad_contracts::commands::RevokeScopeRelationship {
            expected_policy_version: 2,
            relationship_id: store
                .list_relationships(&owner.scope, None, 10)
                .unwrap()
                .relationships
                .into_iter()
                .find(|r| r.relation.as_str() == MANAGER_RELATION)
                .unwrap()
                .relationship_id,
        },
    )
    .unwrap();
    auth.grant_relationship(
        &mutation(&owner),
        &GrantScopeRelationship {
            expected_policy_version: 3,
            subject: RelationshipSubject {
                principal_id: owner.identity.principal_id,
                principal_kind: PrincipalKind::User,
            },
            relation: RelationId::parse(eitmad_authorization::RECEPTIONIST_RELATION).unwrap(),
        },
    )
    .unwrap();
    (store, owner)
}
fn part_fixture(store: &AuthorityStore, actor: &AuthorizationContext) -> (Part, Material) {
    let auth = AuthorizationService::new(store.clone());
    let materials = eitmad_material::MaterialService::new(store.clone(), auth.clone());
    let category = materials
        .save_category(
            &mutation(actor),
            &SaveMaterialCategory {
                id: None,
                expected_revision: None,
                name: "أخشاب".into(),
                archived: false,
            },
        )
        .unwrap();
    let unit = materials
        .save_unit(
            &mutation(actor),
            &SaveMaterialUnit {
                id: None,
                expected_revision: None,
                name: "قطعة".into(),
                symbol: "قطعة".into(),
                dimension: UnitDimension::Count,
                numerator: 1,
                denominator: 1,
                archived: false,
            },
        )
        .unwrap();
    let material = materials
        .save_material(
            &mutation(actor),
            &SaveMaterial {
                id: None,
                expected_revision: None,
                name: "لوح اختبار".into(),
                category_id: category.id,
                unit_id: unit.id,
                current_cost_yer: 500,
                archived: false,
            },
        )
        .unwrap();
    let parts = eitmad_part::PartService::new(store.clone(), auth.clone());
    let category = parts
        .save_category(
            &mutation(actor),
            &SavePartCategory {
                id: None,
                expected_revision: None,
                name: "ألواح".into(),
                archived: false,
            },
        )
        .unwrap();
    let part = parts
        .save(
            &mutation(actor),
            &SavePart {
                id: None,
                expected_revision: None,
                name: "جانب".into(),
                category_id: category.id,
                description: "جزء اختبار".into(),
                usages: vec![PartUsage {
                    material_id: material.id,
                    material_revision: material.revision,
                    unit_id: unit.id,
                    unit_revision: unit.revision,
                    quantity: MaterialQuantity::parse("2".into()).unwrap(),
                }],
                archived: false,
            },
        )
        .unwrap();
    (part, material)
}
fn furniture_fixture(
    store: &AuthorityStore,
    actor: &AuthorizationContext,
    part: Part,
    image: eitmad_contracts::catalog_image::CatalogImageRef,
) -> (Furniture, SaveFurniture) {
    let auth = AuthorizationService::new(store.clone());
    let furnitures = eitmad_furniture::FurnitureService::new(store.clone(), auth.clone());
    let category = furnitures
        .save_category(
            &mutation(actor),
            &SaveFurnitureCategory {
                id: None,
                expected_revision: None,
                name: "خزائن".into(),
                archived: false,
            },
        )
        .unwrap();
    let input = SaveFurniture {
        image: Some(Box::new(image)),
        id: None,
        expected_revision: None,
        name: "خزانة اختبار".into(),
        category_id: category.id,
        description: "وصف عام".into(),
        notes: "restricted-furniture-note".into(),
        parts: vec![FurniturePart {
            reference: part.composition,
            quantity: 3,
        }],
        variants: vec![FurnitureVariant {
            id: FurnitureVariantId::new(Uuid::new_v4()),
            name: "قياسي".into(),
            dimensions: FurnitureDimensions {
                width_mm: 1000,
                height_mm: 1800,
                depth_mm: 500,
            },
            customization: None,
            selling_price_yer: 5000,
            archived: false,
            color_ids: vec![],
            handle_ids: vec![],
        }],
        colors: vec![],
        handles: vec![],
        state: FurnitureState::Active,
        confirm_below_cost: false,
    };
    let furniture = furnitures.save(&mutation(actor), &input).unwrap();
    (furniture, input)
}
fn product_fixture(store: &AuthorityStore, actor: &AuthorizationContext) -> Product {
    let auth = AuthorizationService::new(store.clone());
    let products = eitmad_product::ProductService::new(store.clone(), auth);
    let category = products
        .save_category(
            &mutation(actor),
            &SaveProductCategory {
                id: None,
                expected_revision: None,
                name: "مراتب".into(),
                archived: false,
            },
        )
        .unwrap();
    products
        .save(
            &mutation(actor),
            &SaveProduct {
                image: None,
                id: None,
                expected_revision: None,
                name: "مرتبة اختبار".into(),
                category_id: category.id,
                description: "وصف منتج".into(),
                notes: "restricted-product-note".into(),
                variants: vec![SaveProductVariant {
                    id: ProductVariantId::new(Uuid::new_v4()),
                    name: "مفرد".into(),
                    purchase_cost_yer: 90_000,
                    archived: false,
                }],
                archived: false,
            },
        )
        .unwrap()
}
fn page(store: &AuthorityStore, actor: &AuthorizationContext) -> PricePage {
    PricingService::new(store.clone(), AuthorizationService::new(store.clone()))
        .list(
            actor,
            &ListPrices {
                term: String::new(),
                after: None,
                limit: 100,
            },
        )
        .unwrap()
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL and trusted development certificates"]
async fn catalog_reaches_separate_receptionist_and_recovers_without_private_fields() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let mut scenario = CatalogScenario::new().await;
    scenario.transfer_dependencies().await;
    scenario.publish_prices().await;
    scenario.verify_public().await;
    scenario.verify_image_reference_migration().await;
    scenario.verify_isolation().await;
    scenario.verify_product_projection().await;
    scenario.verify_rejection_progress().await;
    scenario.interrupt_snapshot().await;
    scenario.restart_receiver().await;
    scenario.change_material_cost().await;
    scenario.verify_history_and_archive().await;
}
struct CatalogScenario {
    _manager_directory: tempfile::TempDir,
    database: String,
    certificate: PathBuf,
    key: PathBuf,
    trust: PathBuf,
    server: ProvisionedServer,
    endpoint: String,
    manager_store: AuthorityStore,
    manager: AuthorizationContext,
    image: eitmad_contracts::catalog_image::CatalogImageRef,
    furniture: Furniture,
    furniture_input: SaveFurniture,
    material: Material,
    reception_auth: AuthenticationResult,
    reception_directory: tempfile::TempDir,
    reception_store: AuthorityStore,
    reception: AuthorizationContext,
    reception_client: Arc<eitmad_server_connection::DirectCatalogSyncClient>,
    read_media: Arc<eitmad_catalog_image::CatalogImageService>,
    query: eitmad_contracts::catalog_image::GetCatalogImage,
    price_client: Arc<eitmad_server_connection::DirectPriceClient>,
    manager_client: Arc<eitmad_server_connection::DirectCatalogSyncClient>,
    pricing: PricingService,
    furniture_target: PriceTarget,
    product_target: PriceTarget,
    fetched: Option<eitmad_contracts::catalog_image::CatalogImageChunk>,
}
impl CatalogScenario {
    async fn new() -> Self {
        let database = env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap();
        let certificate = required_path("EITMAD_DIRECT_TEST_CERTIFICATE");
        let key = required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY");
        let trust = required_path("EITMAD_DIRECT_TEST_TRUSTED_CERTIFICATE");
        let server = provision_server(&database, &certificate, &key).await;
        let endpoint = format!("https://localhost:{}/", server.address.port());
        let manager_directory = tempfile::tempdir().unwrap();
        let (manager_store, manager) =
            catalog_local_authority(manager_directory.path(), &server.authentication.session);
        let image = upload_image(
            manager_directory.path(),
            &manager_store,
            &manager,
            &server,
            &endpoint,
            &trust,
        )
        .await;
        let (part, material) = part_fixture(&manager_store, &manager);
        let (furniture, furniture_input) =
            furniture_fixture(&manager_store, &manager, part, image.clone());
        let product = product_fixture(&manager_store, &manager);
        let reception_auth = receptionist(&database, &server).await;
        let reception_directory = tempfile::tempdir().unwrap();
        let (reception_store, reception) =
            receptionist_authority(reception_directory.path(), &reception_auth.session);
        let reception_client = replication(
            reception_directory.path(),
            reception_store.clone(),
            reception_auth.clone(),
            [19; 32],
            &server.scope,
            &endpoint,
            &trust,
        );
        let read_media = reader_media(
            reception_directory.path(),
            &reception_store,
            &reception_auth,
            &server,
            &endpoint,
            &trust,
        );
        let query = eitmad_contracts::catalog_image::GetCatalogImage {
            reference: image.clone(),
            offset: 0,
        };
        let price_client = Arc::new(pricing_test_client(
            manager_directory.path(),
            &server,
            &endpoint,
            &trust,
        ));
        let manager_store = AuthorityStore::open(manager_directory.path()).unwrap();
        let manager_client = replication(
            manager_directory.path(),
            manager_store.clone(),
            server.authentication.clone(),
            [11; 32],
            &server.scope,
            &endpoint,
            &trust,
        );
        let pricing = PricingService::new(
            manager_store.clone(),
            AuthorizationService::new(manager_store.clone()),
        )
        .with_confirmation(price_client.clone());
        let (furniture_target, product_target) =
            price_targets(&manager.scope, &furniture, &product);
        Self {
            _manager_directory: manager_directory,
            database,
            certificate,
            key,
            trust,
            server,
            endpoint,
            manager_store,
            manager,
            image,
            furniture,
            furniture_input,
            material,
            reception_auth,
            reception_directory,
            reception_store,
            reception,
            reception_client,
            read_media,
            query,
            price_client,
            manager_client,
            pricing,
            furniture_target,
            product_target,
            fetched: None,
        }
    }
    async fn transfer_dependencies(&mut self) {
        cycle(self.reception_client.clone(), self.reception.clone())
            .await
            .unwrap();
        assert!(
            page(&self.reception_store, &self.reception)
                .items
                .is_empty()
        );
        let service = self.read_media.clone();
        let reader = self.reception.clone();
        let q = self.query.clone();
        assert_eq!(
            tokio::task::spawn_blocking(move || service.get(
                &reader,
                &q,
                UnixMillis(eitmad_authorization::now().0 + 30_000)
            ))
            .await
            .unwrap(),
            Err(eitmad_catalog_image::ImageError::Denied)
        );
        let mut dependencies = self
            .manager_store
            .pending_catalog_revisions(&self.manager.scope, None)
            .unwrap();
        dependencies.retain(|r| !matches!(r, CatalogRevision::Furniture(_)));
        let a = self.manager.clone();
        let p = self.price_client.clone();
        tokio::task::spawn_blocking(move || {
            p.synchronize_catalog(
                &a,
                &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions {
                    scope: a.scope.clone(),
                    records: dependencies,
                },
                UnixMillis(i64::MAX),
            )
        })
        .await
        .unwrap()
        .unwrap();
        cycle(self.reception_client.clone(), self.reception.clone())
            .await
            .unwrap();
        assert!(
            page(&self.reception_store, &self.reception)
                .items
                .is_empty()
        );
        cycle(self.manager_client.clone(), self.manager.clone())
            .await
            .unwrap();
        assert!(
            self.manager_store
                .pending_catalog_revisions(&self.manager.scope, None)
                .unwrap()
                .is_empty()
        );
        cycle(self.reception_client.clone(), self.reception.clone())
            .await
            .unwrap();
        assert!(
            page(&self.reception_store, &self.reception)
                .items
                .is_empty()
        );
    }
    async fn publish_prices(&mut self) {
        for (target, selling_price_yer) in [
            (self.furniture_target.clone(), 5000),
            (self.product_target.clone(), 100_000),
        ] {
            let service = self.pricing.clone();
            let a = self.manager.clone();
            tokio::task::spawn_blocking(move || {
                service.publish(
                    &mutation(&a),
                    &PublishPrice {
                        target,
                        expected_revision: None,
                        selling_price_yer,
                        confirm_below_cost: false,
                    },
                    UnixMillis(eitmad_authorization::now().0 + 30_000),
                )
            })
            .await
            .unwrap()
            .unwrap();
        }
        cycle(self.reception_client.clone(), self.reception.clone())
            .await
            .unwrap();
    }
    async fn verify_public(&mut self) {
        let service = self.read_media.clone();
        let reader = self.reception.clone();
        let q = self.query.clone();
        let fetched = tokio::task::spawn_blocking(move || {
            service.get(
                &reader,
                &q,
                UnixMillis(eitmad_authorization::now().0 + 30_000),
            )
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(fetched.reference, self.image);
        let expected = self
            .manager_store
            .catalog_image(&self.manager.scope, &self.image)
            .unwrap()
            .unwrap();
        assert!(
            STANDARD.decode(&fetched.base64).unwrap()
                == expected[..(fetched.total_bytes as usize)
                    .min(eitmad_contracts::catalog_image::IMAGE_CHUNK_BYTES)]
        );
        let catalog = PricingService::new(
            self.reception_store.clone(),
            AuthorizationService::new(self.reception_store.clone()),
        );
        let published = catalog
            .sales_catalog(
                &self.reception,
                &ListSalesCatalog {
                    term: String::new(),
                    category: None,
                    after: None,
                    limit: 30,
                },
            )
            .unwrap();
        assert_eq!(published.items.len(), 2);
        for entry in &published.items {
            let details = catalog
                .sales_catalog_item(
                    &self.reception,
                    &GetSalesCatalogItem {
                        target: entry.price.target.clone(),
                    },
                )
                .unwrap();
            assert!(details.variants.contains(entry));
            let checked = catalog
                .sales_configuration(
                    &self.reception,
                    &CheckSalesConfiguration {
                        selection: PriceSelection {
                            target: entry.price.target.clone(),
                            price_revision: entry.price.revision,
                            color_id: entry.colors.first().map(|o| o.id),
                            handle_id: entry.handles.first().map(|o| o.id),
                            quantity: 2,
                        },
                        dimensions: entry.dimensions.clone(),
                    },
                )
                .unwrap();
            assert_eq!(checked.price.total_yer, checked.price.unit_price_yer * 2);
        }
        let serialized = serde_json::to_string(&published).unwrap();
        for field in [
            "cost",
            "margin",
            "parts",
            "restricted-product-note",
            "restricted-furniture-note",
            "90000",
        ] {
            assert!(!serialized.contains(field), "restricted field {field}");
        }
        assert_eq!(
            self.reception_store
                .catalog_sales(&self.reception.scope)
                .unwrap()
                .len(),
            2
        );
        self.fetched = Some(fetched);
    }
    async fn verify_isolation(&mut self) {
        let pool = SyncDatabase::connect(&self.database, 2)
            .await
            .unwrap()
            .pool();
        let sync = eitmad_sync_plane::SyncCoordinator::new(
            &SyncDatabase::from_pool(pool.clone()),
            DomainRegistry::new(eitmad_sync_plane::CatalogSyncHandler::handlers(&pool)).unwrap(),
        );
        let privileges: bool = sqlx::query_scalar(
            "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(
            !privileges,
            "real RLS evidence requires an ordinary test role"
        );
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(Uuid::new_v4().to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM sync.catalog_revisions")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert_eq!(count, 0);
        drop(tx);
        for schema in [
            "eitmad.schema.material.v1",
            "eitmad.schema.part.v1",
            "eitmad.schema.furniture.v1",
        ] {
            assert_eq!(
                sync.pull(eitmad_sync_plane::PullPageRequest {
                    session: &self.reception_auth.session,
                    scope: &self.server.scope,
                    schema_id: &SchemaId::parse(schema).unwrap(),
                    schema_version: 1,
                    after: None,
                    maximum_records: 50,
                    correlation_id: CorrelationId::new(Uuid::new_v4()),
                    now: eitmad_authorization::now()
                })
                .await,
                Err(eitmad_sync_plane::OperationError::Denied)
            );
        }
    }
    async fn verify_image_reference_migration(&self) {
        let database = SyncDatabase::connect(&self.database, 2).await.unwrap();
        let pool = database.pool();
        let id = eitmad_pricing::catalog_record_id(&format!(
            "product:{}:{}",
            self.product_target.identity().1,
            self.product_target.identity().2
        ));
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(self.reception_auth.session.tenant_id.value().to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        let original: serde_json::Value = sqlx::query_scalar("SELECT change_json FROM sync.records WHERE tenant_id=$1 AND scope_id=$2 AND schema_id=$3 AND record_id=$4")
            .bind(self.reception_auth.session.tenant_id.value()).bind(self.server.scope.id.value()).bind("eitmad.schema.catalog-public.v1").bind(id).fetch_one(&mut *tx).await.unwrap();
        sqlx::query("UPDATE sync.records SET change_json=jsonb_set(change_json,'{payload,base64}','\"/w==\"') WHERE tenant_id=$1 AND scope_id=$2 AND schema_id=$3 AND record_id=$4")
            .bind(self.reception_auth.session.tenant_id.value()).bind(self.server.scope.id.value()).bind("eitmad.schema.catalog-public.v1").bind(id).execute(&mut *tx).await.unwrap();
        sqlx::raw_sql("DROP INDEX sync.catalog_public_image_reference; ALTER TABLE sync.records DROP COLUMN public_image_id, DROP COLUMN public_image_sha256; DELETE FROM public.eitmad_server_migrations WHERE version=10;").execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
        database.migrate().await.unwrap();
        let service = eitmad_sync_plane::CatalogImageServer::new(pool.clone());
        let mut input = eitmad_contracts::catalog_image::DownloadCatalogImage {
            scope: self.server.scope.clone(),
            image: self.query.clone(),
        };
        assert!(
            service
                .download(&self.reception_auth.session, &input)
                .await
                .is_ok()
        );
        input.image.reference.sha256 = "0".repeat(64);
        assert_eq!(
            service.download(&self.reception_auth.session, &input).await,
            Err(eitmad_catalog_image::ImageError::Denied)
        );
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(self.reception_auth.session.tenant_id.value().to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("UPDATE sync.records SET change_json=$5 WHERE tenant_id=$1 AND scope_id=$2 AND schema_id=$3 AND record_id=$4")
            .bind(self.reception_auth.session.tenant_id.value()).bind(self.server.scope.id.value()).bind("eitmad.schema.catalog-public.v1").bind(id).bind(original).execute(&mut *tx).await.unwrap();
        tx.commit().await.unwrap();
    }
    /// Checks Product history and snapshot redaction with only one available pool connection.
    async fn verify_product_projection(&mut self) {
        // A page and snapshot must complete while their only pool connection is held.
        let pool = SyncDatabase::connect(&self.database, 1)
            .await
            .unwrap()
            .pool();
        let sync = SyncCoordinator::new(
            &SyncDatabase::from_pool(pool.clone()),
            DomainRegistry::new(eitmad_sync_plane::CatalogSyncHandler::handlers(&pool)).unwrap(),
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        // Restriction applies to raw history and chunked snapshots, before network serialization.
        let schema = SchemaId::parse("eitmad.schema.product.v1").unwrap();

        let product_batch = tokio::time::timeout_at(
            deadline,
            sync.pull(eitmad_sync_plane::PullPageRequest {
                session: &self.reception_auth.session,
                scope: &self.server.scope,
                schema_id: &schema,
                schema_version: 1,
                after: None,
                maximum_records: 50,
                correlation_id: CorrelationId::new(Uuid::new_v4()),
                now: eitmad_authorization::now(),
            }),
        )
        .await
        .expect("product pull must not acquire a second pool connection")
        .unwrap();
        for record in &product_batch.records {
            let payload = record.payload.as_ref().unwrap();
            let decoded = STANDARD.decode(&payload.base64).unwrap();
            let json = String::from_utf8(decoded).unwrap();
            assert!(
                !json.contains("90000")
                    && !json.contains("restricted-product-note")
                    && !json.contains("purchaseCostYer")
                    && !json.contains("\"notes\"")
            );
        }
        let snapshot = tokio::time::timeout_at(
            deadline,
            sync.create_snapshot(
                eitmad_sync_plane::SnapshotRequest {
                    session: &self.reception_auth.session,
                    scope: &self.server.scope,
                    schema_id: &schema,
                    schema_version: 1,
                },
                CorrelationId::new(Uuid::new_v4()),
                eitmad_authorization::now(),
                60_000,
            ),
        )
        .await
        .expect("product snapshot must not acquire a second pool connection")
        .unwrap();
        assert!(
            snapshot.chunks[0].records.len() == product_batch.records.len()
                && snapshot.chunks[0]
                    .records
                    .iter()
                    .all(|record| product_batch.records.contains(record))
        );
    }
    async fn prepare_rejected_category(&self) -> ProductCategory {
        let products = eitmad_product::ProductService::new(
            self.manager_store.clone(),
            AuthorizationService::new(self.manager_store.clone()),
        );
        let category = products
            .save_category(
                &mutation(&self.manager),
                &SaveProductCategory {
                    id: None,
                    expected_revision: None,
                    name: "فئة تحتاج مراجعة".into(),
                    archived: false,
                },
            )
            .unwrap();
        let mut remote = category.clone();
        remote.name = "فئة مستقلة على الخادم".into();
        let price_client = self.price_client.clone();
        let actor = self.manager.clone();
        let local_scope = actor.scope.clone();
        remote.scope = local_scope.clone();
        tokio::task::spawn_blocking(move || {
            price_client.synchronize_catalog(
                &actor,
                &eitmad_contracts::catalog_revision::SynchronizeCatalogRevisions {
                    scope: local_scope,
                    records: vec![CatalogRevision::ProductCategory(Box::new(remote))],
                },
                UnixMillis(eitmad_authorization::now().0 + 30_000),
            )
        })
        .await
        .unwrap()
        .unwrap();
        for index in 0..51 {
            products
                .save(
                    &mutation(&self.manager),
                    &SaveProduct {
                        id: None,
                        expected_revision: None,
                        name: format!("منتج ينتظر الفئة {index}"),
                        category_id: category.id,
                        image: None,
                        description: String::new(),
                        notes: String::new(),
                        variants: vec![SaveProductVariant {
                            id: ProductVariantId::new(Uuid::new_v4()),
                            name: "قياسي".into(),
                            purchase_cost_yer: 1000,
                            archived: false,
                        }],
                        archived: false,
                    },
                )
                .unwrap();
        }
        category
    }

    async fn verify_rejection_progress(&mut self) {
        let category = self.prepare_rejected_category().await;
        let products = eitmad_product::ProductService::new(
            self.manager_store.clone(),
            AuthorizationService::new(self.manager_store.clone()),
        );
        let furniture = eitmad_furniture::FurnitureService::new(
            self.manager_store.clone(),
            AuthorizationService::new(self.manager_store.clone()),
        );
        let independent = furniture
            .save_category(
                &mutation(&self.manager),
                &SaveFurnitureCategory {
                    id: None,
                    expected_revision: None,
                    name: "فئة غير مرتبطة".into(),
                    archived: false,
                },
            )
            .unwrap();
        assert_eq!(
            cycle(self.manager_client.clone(), self.manager.clone())
                .await
                .unwrap(),
            1
        );
        let issues = page(&self.manager_store, &self.manager).catalog_sync_issues;
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].id, category.id.value());
        assert!(
            page(&self.reception_store, &self.reception)
                .catalog_sync_issues
                .is_empty()
        );
        assert!(
            self.manager_store
                .catalog_dependencies_ready(
                    &self.manager.scope,
                    &[("furniture-category", independent.id.value(), 1)]
                )
                .unwrap()
        );
        assert!(
            self.manager_store
                .catalog_checkpoint(
                    &self.manager,
                    &SchemaId::parse("eitmad.schema.catalog-public.v1").unwrap()
                )
                .unwrap()
                .is_some()
        );
        // A subsequent cycle and a reopened client must not resend the terminal root.
        assert_eq!(
            cycle(self.manager_client.clone(), self.manager.clone())
                .await
                .unwrap(),
            0
        );
        products
            .save_category(
                &mutation(&self.manager),
                &SaveProductCategory {
                    id: Some(category.id),
                    expected_revision: Some(1),
                    name: "فئة مصححة".into(),
                    archived: false,
                },
            )
            .unwrap();
        assert_eq!(
            cycle(self.manager_client.clone(), self.manager.clone())
                .await
                .unwrap(),
            50
        );
        assert_eq!(
            cycle(self.manager_client.clone(), self.manager.clone())
                .await
                .unwrap(),
            2
        );
        assert!(
            page(&self.manager_store, &self.manager)
                .catalog_sync_issues
                .is_empty()
        );
        assert!(
            self.manager_store
                .pending_catalog_revisions(&self.manager.scope, None)
                .unwrap()
                .is_empty()
        );
    }
    async fn interrupt_snapshot(&mut self) {
        // A local audit failure after receiving an updated server page cannot move its cursor or price.
        let service = self.pricing.clone();
        let a = self.manager.clone();
        let target = self.furniture_target.clone();
        tokio::task::spawn_blocking(move || {
            service.publish(
                &mutation(&a),
                &PublishPrice {
                    target,
                    expected_revision: Some(1),
                    selling_price_yer: 6500,
                    confirm_below_cost: false,
                },
                UnixMillis(eitmad_authorization::now().0 + 30_000),
            )
        })
        .await
        .unwrap()
        .unwrap();
        // Force the existing stale-checkpoint route to deliver a checksum-verified snapshot.
        let stale = eitmad_contracts::sync::Checkpoint::new(Uuid::new_v4());
        rusqlite::Connection::open(self.reception_store.path()).unwrap().execute("UPDATE catalog_sync_checkpoints SET checkpoint=?1 WHERE schema_id='eitmad.schema.catalog-public.v1'",[stale.value().to_string()]).unwrap();
        let cursor = self
            .reception_store
            .catalog_checkpoint(
                &self.reception,
                &SchemaId::parse("eitmad.schema.catalog-public.v1").unwrap(),
            )
            .unwrap();
        let db = rusqlite::Connection::open(self.reception_store.path()).unwrap();
        db.execute_batch("CREATE TRIGGER block_catalog_projection BEFORE INSERT ON mutation_audit WHEN NEW.operation='eitmad.catalog.sync.project.v1' BEGIN SELECT RAISE(ABORT,'synthetic interrupted projection'); END;").unwrap();
        assert_eq!(
            cycle(self.reception_client.clone(), self.reception.clone()).await,
            Err(PricingError::Unconfirmed)
        );
        assert_eq!(
            self.reception_store
                .catalog_checkpoint(
                    &self.reception,
                    &SchemaId::parse("eitmad.schema.catalog-public.v1").unwrap()
                )
                .unwrap(),
            cursor
        );
        assert_eq!(
            page(&self.reception_store, &self.reception)
                .items
                .iter()
                .find(|i| i.target == self.furniture_target)
                .unwrap()
                .published
                .as_ref()
                .unwrap()
                .revision,
            1
        );
        db.execute_batch("DROP TRIGGER block_catalog_projection")
            .unwrap();
        drop(db);
        self.server
            .handle
            .graceful_shutdown(Some(Duration::from_secs(1)));
        tokio::time::sleep(Duration::from_millis(1200)).await;
        assert!(
            cycle(self.reception_client.clone(), self.reception.clone())
                .await
                .is_err()
        );
        let service = self.read_media.clone();
        let reader = self.reception.clone();
        let q = self.query.clone();
        assert_eq!(
            tokio::task::spawn_blocking(move || service.get(
                &reader,
                &q,
                UnixMillis(eitmad_authorization::now().0 + 30_000)
            ))
            .await
            .unwrap()
            .unwrap(),
            self.fetched.clone().unwrap()
        );
    }
    async fn restart_receiver(&mut self) {
        let database = SyncDatabase::connect(&self.database, 4).await.unwrap();
        let control = ControlPlane::new(
            ControlDatabase::connect(&self.database, 4)
                .await
                .unwrap()
                .pool(),
            TokenKey::new([9; 32]),
        );
        self.server.state = ServerState::new(
            control,
            SyncCoordinator::new(
                &database,
                DomainRegistry::new(eitmad_sync_plane::CatalogSyncHandler::handlers(
                    &database.pool(),
                ))
                .unwrap(),
            ),
        );
        self.server.handle = start_server(
            self.server.address,
            self.server.state.clone(),
            &self.certificate,
            &self.key,
        )
        .await;
        self.reception_store = AuthorityStore::open(self.reception_directory.path()).unwrap();
        self.reception_client = replication(
            self.reception_directory.path(),
            self.reception_store.clone(),
            self.reception_auth.clone(),
            [19; 32],
            &self.server.scope,
            &self.endpoint,
            &self.trust,
        );
        cycle(self.reception_client.clone(), self.reception.clone())
            .await
            .unwrap();
        assert_eq!(
            page(&self.reception_store, &self.reception)
                .items
                .iter()
                .find(|i| i.target == self.furniture_target)
                .unwrap()
                .published
                .as_ref()
                .unwrap()
                .revision,
            2
        );
    }
    async fn change_material_cost(&self) {
        eitmad_material::MaterialService::new(
            self.manager_store.clone(),
            AuthorizationService::new(self.manager_store.clone()),
        )
        .save_material(
            &mutation(&self.manager),
            &SaveMaterial {
                id: Some(self.material.id),
                expected_revision: Some(1),
                name: self.material.name.clone(),
                category_id: self.material.category_id,
                unit_id: self.material.unit_id,
                current_cost_yer: 2000,
                archived: true,
            },
        )
        .unwrap();
        cycle(self.manager_client.clone(), self.manager.clone())
            .await
            .unwrap();
    }
    async fn verify_history_and_archive(&mut self) {
        // A new Manager device resolves exact Parts and historical cost inputs after restart.
        let other_directory = tempfile::tempdir().unwrap();
        let (other_store, other) = catalog_local_authority(
            other_directory.path(),
            &self.server.second_authentication.session,
        );
        let other_client = replication(
            other_directory.path(),
            other_store.clone(),
            self.server.second_authentication.clone(),
            [12; 32],
            &self.server.scope,
            &self.endpoint,
            &self.trust,
        );
        let db = rusqlite::Connection::open(other_store.path()).unwrap();
        for schema in [
            "eitmad.schema.material.v1",
            "eitmad.schema.part.v1",
            "eitmad.schema.product.v1",
            "eitmad.schema.furniture.v1",
        ] {
            db.execute(
                "INSERT INTO catalog_sync_checkpoints VALUES('organization',?1,?2,?3,?4)",
                rusqlite::params![
                    other.scope.id.value().to_string(),
                    other.identity.principal_id.value().to_string(),
                    schema,
                    Uuid::new_v4().to_string()
                ],
            )
            .unwrap();
        }
        drop(db);
        cycle(other_client, other.clone()).await.unwrap();
        let references = other_store.material_references(&other.scope).unwrap();
        let current = other_store
            .list_materials(&other.scope, "", None, 100)
            .unwrap()
            .items
            .into_iter()
            .find(|m| m.id == self.material.id)
            .unwrap();
        assert_eq!(current.current_cost_yer, 2000);
        assert!(current.archived);
        assert_eq!(references.units.len(), 1);

        assert_eq!(
            other_store
                .transact_furnitures::<_, eitmad_storage::StorageError>(
                    |tx| tx.furniture(&other.scope, self.furniture.id.value())
                )
                .unwrap()
                .unwrap()
                .parts,
            self.furniture.parts
        );
        assert_eq!(
            other_store
                .transact_furnitures::<_, eitmad_storage::StorageError>(
                    |tx| tx.composition(&self.furniture.parts[0].reference)
                )
                .unwrap()
                .unwrap()
                .cost
                .total_cost_yer,
            1000
        );
        assert_eq!(
            other_store
                .material_references(&other.scope)
                .unwrap()
                .units
                .len(),
            1
        );
        assert_eq!(self.material.current_cost_yer, 500);
        self.furniture_input.id = Some(self.furniture.id);
        self.furniture_input.expected_revision = Some(1);
        self.furniture_input.state = FurnitureState::Archived;
        eitmad_furniture::FurnitureService::new(
            self.manager_store.clone(),
            AuthorizationService::new(self.manager_store.clone()),
        )
        .save(&mutation(&self.manager), &self.furniture_input)
        .unwrap();
        cycle(self.manager_client.clone(), self.manager.clone())
            .await
            .unwrap();
        cycle(self.reception_client.clone(), self.reception.clone())
            .await
            .unwrap();
        assert_eq!(page(&self.reception_store, &self.reception).items.len(), 1);
        self.server
            .handle
            .graceful_shutdown(Some(Duration::from_secs(1)));
    }
}
async fn upload_image(
    directory: &Path,
    store: &AuthorityStore,
    actor: &AuthorizationContext,
    server: &ProvisionedServer,
    endpoint: &str,
    trust: &Path,
) -> eitmad_contracts::catalog_image::CatalogImageRef {
    let image_transfer = Arc::new(catalog_image_client(
        directory,
        server.authentication.clone(),
        [11; 32],
        &server.scope,
        endpoint,
        trust,
    ));
    let media = eitmad_catalog_image::CatalogImageService::new(
        store.clone(),
        AuthorizationService::new(store.clone()),
    )
    .with_transfer(image_transfer);
    let source = directory.join("synthetic-furniture.png");
    image::RgbImage::from_pixel(2, 2, image::Rgb([80, 60, 40]))
        .save(&source)
        .unwrap();
    let image = media
        .import(
            &mutation(actor),
            &eitmad_contracts::catalog_image::ImportCatalogImage {
                kind: eitmad_contracts::catalog_image::CatalogImageKind::Furniture,
                source_path: source.to_str().unwrap().into(),
            },
            UnixMillis(eitmad_authorization::now().0 + 30_000),
        )
        .unwrap();
    assert_eq!(
        tokio::task::spawn_blocking(move || media.retry_uploads())
            .await
            .unwrap(),
        Ok(1)
    );
    image
}
fn reader_media(
    directory: &Path,
    store: &AuthorityStore,
    authentication: &AuthenticationResult,
    server: &ProvisionedServer,
    endpoint: &str,
    trust: &Path,
) -> Arc<eitmad_catalog_image::CatalogImageService> {
    Arc::new(
        eitmad_catalog_image::CatalogImageService::new(
            store.clone(),
            AuthorizationService::new(store.clone()),
        )
        .with_transfer(Arc::new(catalog_image_client(
            directory,
            authentication.clone(),
            [19; 32],
            &server.scope,
            endpoint,
            trust,
        ))),
    )
}
fn price_targets(
    scope: &ScopeRef,
    furniture: &Furniture,
    product: &Product,
) -> (PriceTarget, PriceTarget) {
    let furniture_target = PriceTarget::Furniture(FurnitureReference {
        scope: scope.clone(),
        furniture_id: furniture.id,
        variant_id: furniture.variants[0].id,
        revision: 1,
        schema_version: 1,
    });
    let product_target = PriceTarget::Product(ProductReference {
        scope: scope.clone(),
        product_id: product.id,
        variant_id: product.variants[0].id,
        revision: 1,
        schema_version: 1,
    });
    (furniture_target, product_target)
}

#[path = "quotation_drafts.rs"]
mod quotation_drafts;
