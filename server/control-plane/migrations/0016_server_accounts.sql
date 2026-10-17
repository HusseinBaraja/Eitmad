ALTER TABLE control.accounts ADD COLUMN revision bigint NOT NULL DEFAULT 1 CHECK (revision > 0);

CREATE TABLE control.account_organizations (
    tenant_id uuid NOT NULL,
    account_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    PRIMARY KEY (tenant_id, account_id, organization_id),
    FOREIGN KEY (tenant_id, account_id) REFERENCES control.accounts(tenant_id, account_id),
    FOREIGN KEY (tenant_id, organization_id) REFERENCES control.organizations(tenant_id, organization_id)
);

-- Migrate each tenant under its own forced RLS context, including retained provisioning data.
DO $$ DECLARE tenant uuid; BEGIN
    FOR tenant IN SELECT tenant_id FROM control.tenants LOOP
        PERFORM set_config('eitmad.tenant_id', tenant::text, true);
        INSERT INTO control.account_organizations
        SELECT DISTINCT a.tenant_id, a.account_id, o.organization_id
        FROM control.accounts a JOIN control.relationship_tuples r
          ON r.tenant_id=a.tenant_id AND r.subject_principal_id=a.user_id AND r.subject_kind='user'
        JOIN control.organizations o ON o.tenant_id=r.tenant_id AND o.organization_id=r.object_id
        WHERE a.tenant_id=tenant AND r.object_kind='organization';
    END LOOP;
END $$;

CREATE TABLE control.account_results (
    tenant_id uuid NOT NULL REFERENCES control.tenants(tenant_id),
    actor_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    fingerprint bytea NOT NULL,
    receipt jsonb,
    error text,
    CHECK ((receipt IS NULL) <> (error IS NULL)),
    PRIMARY KEY (tenant_id, actor_id, idempotency_key)
);

CREATE TABLE control.activation_results (
    tenant_id uuid NOT NULL,
    invite_id uuid NOT NULL,
    fingerprint bytea NOT NULL,
    session_id uuid NOT NULL,
    initial_refresh_hash bytea NOT NULL REFERENCES control.refresh_tokens(token_hash),
    access_expires_at bigint NOT NULL,
    refresh_expires_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, invite_id),
    FOREIGN KEY (tenant_id, invite_id) REFERENCES control.invitations(tenant_id, invite_id),
    FOREIGN KEY (tenant_id, session_id) REFERENCES control.sessions(tenant_id, session_id)
);

DO $$ DECLARE table_name text; BEGIN
    FOREACH table_name IN ARRAY ARRAY['account_organizations','account_results','activation_results'] LOOP
        EXECUTE format('ALTER TABLE control.%I ENABLE ROW LEVEL SECURITY', table_name);
        EXECUTE format('ALTER TABLE control.%I FORCE ROW LEVEL SECURITY', table_name);
        EXECUTE format('CREATE POLICY tenant_isolation ON control.%I USING '
            '(tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid) '
            'WITH CHECK (tenant_id = nullif(current_setting(''eitmad.tenant_id'', true), '''')::uuid)', table_name);
    END LOOP;
END $$;
