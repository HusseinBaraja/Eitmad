//! Two real desktop installations; the receiving runtime never creates outbox work.
use super::*;
use std::process::{Child, Command, Stdio};

struct RuntimeProcess(Child);

impl RuntimeProcess {
    fn start(installation: &Installation) -> Self {
        let binary = env::var_os("EITMAD_CUSTOMER_SYNC_ENGINE")
            .expect("set EITMAD_CUSTOMER_SYNC_ENGINE to the current engine executable");
        Self(
            Command::new(binary)
                .args(["run", "--mode", "supervised", "--runtime-directory"])
                .arg(installation.directory.path())
                .arg("--supervisor-pid")
                .arg(std::process::id().to_string())
                .stdin(Stdio::piped())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        )
    }

    async fn stop(mut self) {
        drop(self.0.stdin.take());
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "Runtime failed during shutdown");
                return;
            }
            assert!(Instant::now() < deadline, "Runtime shutdown timed out");
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Drop for RuntimeProcess {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

fn branch_actor(installation: &Installation) -> AuthorizationContext {
    let mut actor = local_sign_in(installation);
    actor.scope = installation
        .store
        .desktop_customer_branch(
            actor.tenant_id,
            eitmad_contracts::identity::UserId::new(actor.identity.principal_id.value()),
        )
        .unwrap()
        .unwrap();
    actor
}

fn customers(installation: &Installation) -> CustomerService {
    CustomerService::new(
        installation.store.clone(),
        AuthorizationService::new(installation.store.clone()),
    )
}

fn mutation(actor: &AuthorizationContext) -> MutationContext {
    MutationContext {
        authorization: actor.clone(),
        correlation_id: CorrelationId::new(Uuid::new_v4()),
        causation_id: None,
        idempotency_key: IdempotencyKey::new(Uuid::new_v4()),
        occurred_at: eitmad_authorization::now(),
    }
}

fn assert_empty_outbox(installation: &Installation) {
    let count: i64 = rusqlite::Connection::open(installation.store.path())
        .unwrap()
        .query_row("SELECT count(*) FROM customer_sync_outbox", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 0, "Receiving installation must never create uploads");
}

async fn received(
    runtime: &mut RuntimeProcess,
    installation: &Installation,
    actor: &AuthorizationContext,
    expected: &Customer,
) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        assert!(
            runtime.0.try_wait().unwrap().is_none(),
            "Runtime exited before incoming delivery"
        );
        assert_empty_outbox(installation);
        if let Ok(actual) = customers(installation).get(
            actor,
            &GetCustomer {
                customer_id: expected.id,
                branch_id: None,
            },
        ) {
            if actual.name == expected.name && actual.revision == expected.revision {
                assert_eq!(actual.sync_state, CustomerSyncState::Confirmed);
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "Ordinary runtime did not deliver incoming customer history"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires disposable PostgreSQL, TLS certificates, and the current engine executable"]
async fn clean_installation_receives_customers_in_the_ordinary_runtime() {
    let _ = rustls::crypto::ring::default_provider().install_default();
    let server = provision_server(
        &env::var("EITMAD_DIRECT_TEST_DATABASE_URL").unwrap(),
        &required_path("EITMAD_DIRECT_TEST_CERTIFICATE"),
        &required_path("EITMAD_DIRECT_TEST_PRIVATE_KEY"),
    )
    .await;
    let first = Installation::new();
    let second = Installation::new();
    let inputs = inputs_for_shell(&server);
    assert!(first.sign_in(request(Some(inputs.clone()))).await.unwrap());
    assert!(second.sign_in(request(Some(inputs))).await.unwrap());
    let author = branch_actor(&first);
    let reader = branch_actor(&second);
    assert_eq!(author.scope, reader.scope);
    assert_ne!(first.device(), second.device());
    assert_empty_outbox(&second);
    let source = RuntimeProcess::start(&first);
    let mut target = RuntimeProcess::start(&second);
    let service = customers(&first);
    let created = service
        .create(
            &mutation(&author),
            &CreateCustomer {
                name: CustomerName::parse("عميل مزامنة اختباري").unwrap(),
                phone: CustomerPhone::parse("777123456").unwrap(),
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    received(&mut target, &second, &reader, &created).await;
    let updated = service
        .update(
            &mutation(&author),
            &UpdateCustomer {
                customer_id: created.id,
                expected_revision: 1,
                name: CustomerName::parse("عميل محدث عبر الخادم").unwrap(),
                phone: created.phone,
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    received(&mut target, &second, &reader, &updated).await;
    target.stop().await;
    let caught_up = service
        .update(
            &mutation(&author),
            &UpdateCustomer {
                customer_id: updated.id,
                expected_revision: 2,
                name: CustomerName::parse("تحديث أثناء التوقف").unwrap(),
                phone: updated.phone,
                address: None,
                notes: None,
            },
        )
        .unwrap()
        .customer;
    let mut restarted = RuntimeProcess::start(&second);
    received(&mut restarted, &second, &reader, &caught_up).await;
    assert_empty_outbox(&second);
    restarted.stop().await;
    source.stop().await;
    server
        .handle
        .graceful_shutdown(Some(Duration::from_secs(1)));
}
