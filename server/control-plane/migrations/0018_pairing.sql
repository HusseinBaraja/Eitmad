CREATE TABLE control.pairing_sessions (
    tenant_id uuid NOT NULL REFERENCES control.tenants(tenant_id),
    session_id uuid NOT NULL,
    server_id uuid NOT NULL,
    offer jsonb NOT NULL,
    owner_account_id uuid NOT NULL,
    owner_user_id uuid NOT NULL,
    setup bytea,
    password_file bytea,
    attempts integer NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 5),
    expires_at bigint NOT NULL,
    closed_at bigint,
    PRIMARY KEY (tenant_id, session_id),
    FOREIGN KEY (tenant_id, owner_account_id) REFERENCES control.accounts(tenant_id, account_id)
);

-- Public routing handles contain no code, verifier, or OPAQUE state.
CREATE TABLE control.pairing_directory (
    session_id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL,
    server_id uuid NOT NULL,
    expires_at bigint NOT NULL,
    FOREIGN KEY (tenant_id, session_id) REFERENCES control.pairing_sessions(tenant_id, session_id)
);

CREATE TABLE control.pairing_attempts (
    tenant_id uuid NOT NULL,
    session_id uuid NOT NULL,
    attempt_id uuid NOT NULL,
    binding jsonb NOT NULL,
    state bytea,
    completion_hash bytea,
    receipt jsonb,
    finished_at bigint,
    PRIMARY KEY (tenant_id, session_id, attempt_id),
    FOREIGN KEY (tenant_id, session_id) REFERENCES control.pairing_sessions(tenant_id, session_id)
);

CREATE TABLE control.paired_devices (
    tenant_id uuid NOT NULL REFERENCES control.tenants(tenant_id),
    device_id uuid NOT NULL REFERENCES control.devices(device_id),
    organization_id uuid NOT NULL,
    branch_id uuid NOT NULL,
    approved_at bigint NOT NULL,
    revoked_at bigint,
    PRIMARY KEY (tenant_id, device_id),
    FOREIGN KEY (tenant_id, organization_id) REFERENCES control.organizations(tenant_id, organization_id),
    FOREIGN KEY (tenant_id, branch_id) REFERENCES control.branches(tenant_id, branch_id)
);

-- A durable claim prevents re-opening first-desktop enrollment after revocation.
CREATE TABLE control.first_desktop_claims (
    tenant_id uuid PRIMARY KEY REFERENCES control.tenants(tenant_id),
    claimed_at bigint NOT NULL
);
DO $$
DECLARE tenant uuid;
BEGIN
    FOR tenant IN SELECT tenant_id FROM control.tenants LOOP
        PERFORM set_config('eitmad.tenant_id', tenant::text, true);
        INSERT INTO control.first_desktop_claims(tenant_id, claimed_at)
        SELECT tenant_id, min(registered_at) FROM control.account_devices
        WHERE tenant_id=tenant GROUP BY tenant_id;
    END LOOP;
    PERFORM set_config('eitmad.tenant_id', '', true);
END $$;

DO $$
DECLARE t text;
BEGIN
    FOREACH t IN ARRAY ARRAY['pairing_sessions','pairing_attempts','paired_devices','first_desktop_claims'] LOOP
        EXECUTE format('ALTER TABLE control.%I ENABLE ROW LEVEL SECURITY', t);
        EXECUTE format('ALTER TABLE control.%I FORCE ROW LEVEL SECURITY', t);
        EXECUTE format('CREATE POLICY tenant_scope ON control.%I USING (tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid) WITH CHECK (tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid)', t);
    END LOOP;
END $$;
