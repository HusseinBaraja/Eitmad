CREATE TABLE control.branches (
    tenant_id uuid NOT NULL,
    branch_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    created_at bigint NOT NULL,
    PRIMARY KEY (tenant_id, branch_id),
    FOREIGN KEY (tenant_id, organization_id)
        REFERENCES control.organizations(tenant_id, organization_id)
);

CREATE INDEX branches_by_organization
    ON control.branches (tenant_id, organization_id, branch_id);

ALTER TABLE control.branches ENABLE ROW LEVEL SECURITY;
ALTER TABLE control.branches FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON control.branches
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);
