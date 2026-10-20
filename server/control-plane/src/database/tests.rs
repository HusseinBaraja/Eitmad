use super::*;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a fresh disposable EITMAD_DIRECT_TEST_DATABASE_URL with enforced RLS"]
async fn account_names_backfill_when_migration_16_runs_in_same_transaction() {
    check_backfill(false).await;
}

#[tokio::test]
#[ignore = "requires a fresh disposable EITMAD_DIRECT_TEST_DATABASE_URL with enforced RLS"]
async fn account_names_backfill_when_migration_16_is_already_recorded() {
    check_backfill(true).await;
}

async fn check_backfill(accounts_already_migrated: bool) {
    let url = std::env::var("EITMAD_DIRECT_TEST_DATABASE_URL")
        .expect("set a fresh disposable PostgreSQL database URL");
    let database = ControlDatabase::connect(&url, 1).await.unwrap();
    let pool = database.pool();
    let elevated: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!elevated, "test role must enforce RLS");
    let fresh: bool = sqlx::query_scalar(
        "SELECT to_regnamespace('control') IS NULL
            AND to_regclass('public.eitmad_server_migrations') IS NULL",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(fresh, "test requires a fresh disposable database");
    seed_retained_accounts(&pool, accounts_already_migrated).await;
    let forced: bool = sqlx::query_scalar(
        "SELECT relrowsecurity AND relforcerowsecurity
         FROM pg_class WHERE oid='control.accounts'::regclass",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(forced, "accounts must use forced RLS");
    let visible: i64 = sqlx::query_scalar("SELECT count(*) FROM control.accounts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(visible, 0, "no tenant context must expose no accounts");

    database.migrate().await.unwrap();
    let recorded: i64 =
        sqlx::query_scalar("SELECT count(*) FROM public.eitmad_server_migrations WHERE version=17")
            .fetch_one(&pool)
            .await
            .unwrap();
    let before_retry = account_snapshot(&pool).await;
    let filled = before_retry
        .iter()
        .filter(|row| row["display_name"] == row["username"])
        .count();
    assert_eq!(recorded, 1);
    assert_eq!(before_retry.len(), 2, "both retained accounts must remain");
    assert_eq!(filled, 2, "each tenant must receive its account name");
    let ledger_before = ledger_snapshot(&pool).await;
    database.migrate().await.unwrap();
    assert_eq!(account_snapshot(&pool).await, before_retry);
    assert_eq!(ledger_snapshot(&pool).await, ledger_before);
    pool.close().await;
}

async fn seed_retained_accounts(pool: &PgPool, accounts_already_migrated: bool) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql(
        "CREATE TABLE public.eitmad_server_migrations (
            version integer PRIMARY KEY, migration_id text NOT NULL UNIQUE,
            checksum text NOT NULL, applied_at timestamptz NOT NULL DEFAULT now())",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    let mut migrations = vec![
        (1, "server.control-foundation.v1", FOUNDATION_SQL),
        (5, "server.customer-branches.v1", CUSTOMER_BRANCHES_SQL),
    ];
    if accounts_already_migrated {
        migrations.push((16, "server.accounts.v1", ACCOUNTS_SQL));
    }
    for (version, id, sql) in migrations {
        sqlx::raw_sql(sql).execute(&mut *tx).await.unwrap();
        sqlx::query(
            "INSERT INTO public.eitmad_server_migrations(version,migration_id,checksum)
             VALUES($1,$2,$3)",
        )
        .bind(version)
        .bind(id)
        .bind(format!("{:x}", Sha256::digest(sql.as_bytes())))
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    for (index, username) in ["retained-owner-a", "retained-owner-b"].iter().enumerate() {
        let tenant = Uuid::new_v4();
        let user = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO control.tenants(tenant_id,tenant_code,display_name,created_at)
             VALUES($1,$2,'Synthetic tenant',1)",
        )
        .bind(tenant)
        .bind(format!("backfill-{index}"))
        .execute(&mut *tx)
        .await
        .unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(tenant.to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query("INSERT INTO control.users(tenant_id,user_id,created_at) VALUES($1,$2,1)")
            .bind(tenant)
            .bind(user)
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO control.accounts
             (tenant_id,account_id,user_id,username,canonical_username,status,created_at)
             VALUES($1,$2,$3,$4,$4,'pending_activation',1)",
        )
        .bind(tenant)
        .bind(Uuid::new_v4())
        .bind(user)
        .bind(*username)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
}

async fn account_snapshot(pool: &PgPool) -> Vec<serde_json::Value> {
    let tenants: Vec<Uuid> =
        sqlx::query_scalar("SELECT tenant_id FROM control.tenants ORDER BY tenant_code")
            .fetch_all(pool)
            .await
            .unwrap();
    let mut rows = Vec::new();
    for tenant in tenants {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('eitmad.tenant_id',$1,true)")
            .bind(tenant.to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        let account: serde_json::Value = sqlx::query_scalar(
            "SELECT to_jsonb(a) || jsonb_build_object('xmin',xmin::text)
             FROM control.accounts a WHERE tenant_id=$1",
        )
        .bind(tenant)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        rows.push(account);
        tx.commit().await.unwrap();
    }
    rows
}

async fn ledger_snapshot(pool: &PgPool) -> Vec<serde_json::Value> {
    sqlx::query_scalar(
        "SELECT to_jsonb(m) || jsonb_build_object('xmin',xmin::text)
         FROM public.eitmad_server_migrations m ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .unwrap()
}
