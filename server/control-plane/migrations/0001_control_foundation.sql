CREATE SCHEMA IF NOT EXISTS control;
CREATE SCHEMA IF NOT EXISTS audit;
CREATE SCHEMA IF NOT EXISTS publication;

CREATE TABLE control.tenants (
    tenant_id uuid PRIMARY KEY,
    tenant_code text NOT NULL UNIQUE,
    display_name text NOT NULL,
    created_at bigint NOT NULL,
    CHECK (tenant_code = lower(tenant_code)),
    CHECK (length(tenant_code) BETWEEN 3 AND 32)
);

CREATE TABLE control.users (
    tenant_id uuid NOT NULL REFERENCES control.tenants(tenant_id),
    user_id uuid NOT NULL,
    created_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, user_id)
);

CREATE TABLE control.accounts (
    tenant_id uuid NOT NULL,
    account_id uuid NOT NULL,
    user_id uuid NOT NULL,
    username text NOT NULL,
    canonical_username text NOT NULL,
    status text NOT NULL,
    password_hash text,
    created_at bigint NOT NULL,
    activated_at bigint,
    locked_until bigint,
    PRIMARY KEY (tenant_id, account_id),
    UNIQUE (tenant_id, canonical_username),
    FOREIGN KEY (tenant_id, user_id) REFERENCES control.users(tenant_id, user_id),
    CHECK (status IN ('pending_activation', 'active', 'disabled', 'locked'))
);

CREATE TABLE control.organizations (
    tenant_id uuid NOT NULL REFERENCES control.tenants(tenant_id),
    organization_id uuid NOT NULL,
    display_name text NOT NULL,
    created_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, organization_id)
);

CREATE TABLE control.devices (
    device_id uuid PRIMARY KEY,
    algorithm text NOT NULL,
    public_key bytea NOT NULL,
    label text NOT NULL,
    created_at bigint NOT NULL,
    revoked_at bigint,
    CHECK (algorithm = 'ed25519')
);

CREATE TABLE control.account_devices (
    tenant_id uuid NOT NULL,
    account_id uuid NOT NULL,
    device_id uuid NOT NULL REFERENCES control.devices(device_id),
    registered_at bigint NOT NULL,
    revoked_at bigint,
    PRIMARY KEY (tenant_id, account_id, device_id),
    FOREIGN KEY (tenant_id, account_id) REFERENCES control.accounts(tenant_id, account_id)
);

CREATE TABLE control.invitations (
    tenant_id uuid NOT NULL,
    invite_id uuid NOT NULL,
    account_id uuid NOT NULL,
    token_hash bytea NOT NULL UNIQUE,
    expires_at bigint NOT NULL,
    consumed_at bigint,
    delivery_destination text,
    created_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, invite_id),
    FOREIGN KEY (tenant_id, account_id) REFERENCES control.accounts(tenant_id, account_id)
);

CREATE TABLE control.invitation_directory (
    token_hash bytea PRIMARY KEY,
    tenant_id uuid NOT NULL,
    invite_id uuid NOT NULL
);

CREATE TABLE control.sessions (
    tenant_id uuid NOT NULL,
    session_id uuid NOT NULL,
    account_id uuid NOT NULL,
    user_id uuid NOT NULL,
    device_id uuid NOT NULL,
    issued_at bigint NOT NULL,
    expires_at bigint NOT NULL,
    idle_expires_at bigint NOT NULL,
    last_seen_at bigint NOT NULL,
    revoked_at bigint,
    PRIMARY KEY (tenant_id, session_id),
    FOREIGN KEY (tenant_id, account_id) REFERENCES control.accounts(tenant_id, account_id),
    FOREIGN KEY (tenant_id, user_id) REFERENCES control.users(tenant_id, user_id),
    FOREIGN KEY (tenant_id, account_id, device_id)
        REFERENCES control.account_devices(tenant_id, account_id, device_id)
);

CREATE TABLE control.token_families (
    tenant_id uuid NOT NULL,
    token_family_id uuid NOT NULL,
    session_id uuid NOT NULL,
    revoked_at bigint,
    PRIMARY KEY (tenant_id, token_family_id),
    FOREIGN KEY (tenant_id, session_id) REFERENCES control.sessions(tenant_id, session_id)
);

CREATE TABLE control.access_tokens (
    tenant_id uuid NOT NULL,
    token_hash bytea PRIMARY KEY,
    token_family_id uuid NOT NULL,
    session_id uuid NOT NULL,
    expires_at bigint NOT NULL,
    FOREIGN KEY (tenant_id, token_family_id)
        REFERENCES control.token_families(tenant_id, token_family_id),
    FOREIGN KEY (tenant_id, session_id) REFERENCES control.sessions(tenant_id, session_id)
);

CREATE TABLE control.refresh_tokens (
    tenant_id uuid NOT NULL,
    token_hash bytea PRIMARY KEY,
    token_family_id uuid NOT NULL,
    session_id uuid NOT NULL,
    device_id uuid NOT NULL,
    expires_at bigint NOT NULL,
    consumed_at bigint,
    replaced_by_hash bytea,
    FOREIGN KEY (tenant_id, token_family_id)
        REFERENCES control.token_families(tenant_id, token_family_id),
    FOREIGN KEY (tenant_id, session_id) REFERENCES control.sessions(tenant_id, session_id)
);

CREATE TABLE control.token_directory (
    token_hash bytea PRIMARY KEY,
    tenant_id uuid NOT NULL,
    token_kind text NOT NULL,
    CHECK (token_kind IN ('access', 'refresh'))
);

CREATE TABLE control.device_nonces (
    device_id uuid NOT NULL REFERENCES control.devices(device_id),
    nonce_hash bytea NOT NULL,
    expires_at bigint NOT NULL,
    consumed_at bigint,
    PRIMARY KEY (device_id, nonce_hash)
);

CREATE TABLE control.relationship_tuples (
    tenant_id uuid NOT NULL,
    subject_principal_id uuid NOT NULL,
    subject_kind text NOT NULL,
    relation text NOT NULL,
    object_kind text NOT NULL,
    object_id uuid NOT NULL,
    workspace_id uuid,
    created_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, subject_principal_id, relation, object_kind, object_id)
);

CREATE TABLE control.licenses (
    tenant_id uuid PRIMARY KEY REFERENCES control.tenants(tenant_id),
    license_id uuid NOT NULL,
    provider_revision text NOT NULL,
    status text NOT NULL,
    valid_until bigint,
    grace_until bigint,
    updated_at bigint NOT NULL,
    CHECK (status IN ('active', 'grace', 'expired', 'suspended', 'unknown'))
);

CREATE TABLE control.license_entitlements (
    tenant_id uuid NOT NULL REFERENCES control.licenses(tenant_id),
    entitlement text NOT NULL,
    PRIMARY KEY (tenant_id, entitlement)
);

CREATE TABLE control.update_assignments (
    tenant_id uuid NOT NULL REFERENCES control.tenants(tenant_id),
    assignment_kind text NOT NULL,
    device_id uuid,
    channel text NOT NULL,
    revision bigint NOT NULL,
    updated_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, assignment_kind, device_id),
    CHECK (
        (assignment_kind = 'tenant' AND device_id = '00000000-0000-0000-0000-000000000000')
        OR (assignment_kind = 'device' AND device_id <> '00000000-0000-0000-0000-000000000000')
    )
);

CREATE TABLE audit.server_records (
    audit_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    session_id uuid NOT NULL,
    device_id uuid,
    principal_id uuid NOT NULL,
    operation text NOT NULL,
    outcome text NOT NULL,
    target_kind text NOT NULL,
    correlation_id uuid NOT NULL,
    redacted_error text,
    occurred_at bigint NOT NULL
);

CREATE TABLE publication.server_events (
    event_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    topic text NOT NULL,
    payload_json jsonb NOT NULL,
    occurred_at bigint NOT NULL,
    published_at bigint,
    lease_owner uuid,
    lease_until bigint
);

CREATE INDEX server_events_pending ON publication.server_events (occurred_at)
    WHERE published_at IS NULL;

DO $$
DECLARE
    table_name text;
BEGIN
    FOREACH table_name IN ARRAY ARRAY[
        'users', 'accounts', 'organizations', 'account_devices', 'invitations',
        'sessions', 'token_families', 'access_tokens', 'refresh_tokens',
        'relationship_tuples', 'licenses', 'license_entitlements',
        'update_assignments'
    ] LOOP
        EXECUTE format('ALTER TABLE control.%I ENABLE ROW LEVEL SECURITY', table_name);
        EXECUTE format('ALTER TABLE control.%I FORCE ROW LEVEL SECURITY', table_name);
        EXECUTE format(
            'CREATE POLICY tenant_isolation ON control.%I USING '
            '(tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid) '
            'WITH CHECK (tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid)',
            table_name
        );
    END LOOP;
END;
$$;

ALTER TABLE audit.server_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit.server_records FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON audit.server_records
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);

ALTER TABLE publication.server_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE publication.server_events FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON publication.server_events
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);

CREATE FUNCTION audit.reject_server_record_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'server audit is append-only';
END;
$$;

CREATE TRIGGER server_audit_no_update BEFORE UPDATE ON audit.server_records
FOR EACH ROW EXECUTE FUNCTION audit.reject_server_record_change();
CREATE TRIGGER server_audit_no_delete BEFORE DELETE ON audit.server_records
FOR EACH ROW EXECUTE FUNCTION audit.reject_server_record_change();
