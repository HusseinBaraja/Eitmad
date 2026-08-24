CREATE SCHEMA IF NOT EXISTS sync;

CREATE TABLE sync.scopes (
    tenant_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    mode text NOT NULL,
    server_generation bigint NOT NULL DEFAULT 1,
    head_sequence bigint NOT NULL DEFAULT 0,
    head_checkpoint uuid,
    PRIMARY KEY (tenant_id, scope_kind, scope_id, schema_id),
    CHECK (mode IN ('local_first', 'server_authoritative'))
);

CREATE TABLE sync.records (
    tenant_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    record_id uuid NOT NULL,
    revision bigint NOT NULL,
    tombstone boolean NOT NULL,
    change_json jsonb NOT NULL,
    changed_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, scope_kind, scope_id, schema_id, record_id),
    FOREIGN KEY (tenant_id, scope_kind, scope_id, schema_id)
        REFERENCES sync.scopes(tenant_id, scope_kind, scope_id, schema_id)
);

CREATE TABLE sync.operations (
    tenant_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    sequence bigint NOT NULL,
    checkpoint uuid NOT NULL,
    change_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    request_fingerprint bytea NOT NULL,
    change_json jsonb NOT NULL,
    created_at bigint NOT NULL,
    retention_until bigint NOT NULL,
    PRIMARY KEY (tenant_id, scope_kind, scope_id, schema_id, sequence),
    UNIQUE (tenant_id, scope_kind, scope_id, schema_id, checkpoint),
    UNIQUE (tenant_id, scope_kind, scope_id, schema_id, idempotency_key)
);

CREATE TABLE sync.idempotency_results (
    tenant_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    idempotency_key uuid NOT NULL,
    request_fingerprint bytea NOT NULL,
    result_json jsonb NOT NULL,
    created_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, scope_kind, scope_id, schema_id, idempotency_key)
);

CREATE TABLE sync.conflicts (
    tenant_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    conflict_id uuid NOT NULL,
    record_id uuid NOT NULL,
    conflict_json jsonb NOT NULL,
    status text NOT NULL,
    detected_at bigint NOT NULL,
    resolved_at bigint,
    PRIMARY KEY (tenant_id, conflict_id),
    CHECK (status IN ('open', 'resolved'))
);

CREATE TABLE sync.snapshots (
    tenant_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    snapshot_id uuid NOT NULL,
    checkpoint uuid NOT NULL,
    server_generation bigint NOT NULL,
    manifest_json jsonb NOT NULL,
    chunks_json jsonb NOT NULL,
    checksum text NOT NULL,
    created_at bigint NOT NULL,
    valid_until bigint NOT NULL,
    PRIMARY KEY (tenant_id, snapshot_id)
);

CREATE TABLE sync.device_checkpoints (
    tenant_id uuid NOT NULL,
    account_id uuid NOT NULL,
    device_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    checkpoint uuid NOT NULL,
    sequence bigint NOT NULL,
    acknowledged_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, account_id, device_id, scope_kind, scope_id, schema_id)
);

CREATE TABLE sync.subscription_events (
    cursor bigserial PRIMARY KEY,
    event_id uuid NOT NULL UNIQUE,
    tenant_id uuid NOT NULL,
    scope_kind text NOT NULL,
    scope_id uuid NOT NULL,
    schema_id text NOT NULL,
    event_json jsonb NOT NULL,
    occurred_at bigint NOT NULL
);

CREATE INDEX subscription_events_resume
    ON sync.subscription_events (tenant_id, scope_kind, scope_id, schema_id, cursor);
CREATE INDEX sync_operations_retention
    ON sync.operations (retention_until, sequence);
CREATE INDEX sync_conflicts_open
    ON sync.conflicts (tenant_id, scope_kind, scope_id, schema_id)
    WHERE status = 'open';

DO $$
DECLARE
    table_name text;
BEGIN
    FOREACH table_name IN ARRAY ARRAY[
        'scopes', 'records', 'operations', 'idempotency_results', 'conflicts',
        'snapshots', 'device_checkpoints', 'subscription_events'
    ] LOOP
        EXECUTE format('ALTER TABLE sync.%I ENABLE ROW LEVEL SECURITY', table_name);
        EXECUTE format('ALTER TABLE sync.%I FORCE ROW LEVEL SECURITY', table_name);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON sync.%I USING '
            '(tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid) '
            'WITH CHECK (tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid)',
            table_name
        );
    END LOOP;
END;
$$;
