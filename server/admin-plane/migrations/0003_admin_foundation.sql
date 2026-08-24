CREATE SCHEMA IF NOT EXISTS operations;

CREATE TABLE operations.backup_status (
    tenant_id uuid PRIMARY KEY REFERENCES control.tenants(tenant_id),
    state text NOT NULL,
    last_success_at bigint,
    last_verified_at bigint,
    next_scheduled_at bigint,
    recovery_point_age_ms bigint,
    failure_code text,
    updated_at bigint NOT NULL,
    CHECK (state IN ('current', 'stale', 'running', 'failed', 'not_configured')),
    CHECK (recovery_point_age_ms IS NULL OR recovery_point_age_ms >= 0)
);

CREATE TABLE operations.support_workflows (
    tenant_id uuid NOT NULL REFERENCES control.tenants(tenant_id),
    workflow_id uuid NOT NULL,
    action_json jsonb NOT NULL,
    reason_code text NOT NULL,
    state text NOT NULL,
    requested_at bigint NOT NULL,
    completed_at bigint,
    failure_code text,
    PRIMARY KEY (tenant_id, workflow_id),
    CHECK (state IN ('pending', 'running', 'succeeded', 'failed'))
);

DO $$
DECLARE
    table_name text;
BEGIN
    FOREACH table_name IN ARRAY ARRAY['backup_status', 'support_workflows'] LOOP
        EXECUTE format('ALTER TABLE operations.%I ENABLE ROW LEVEL SECURITY', table_name);
        EXECUTE format('ALTER TABLE operations.%I FORCE ROW LEVEL SECURITY', table_name);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON operations.%I USING '
            '(tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid) '
            'WITH CHECK (tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid)',
            table_name
        );
    END LOOP;
END;
$$;
