CREATE TABLE sync.quotation_draft_revisions (
    tenant_id uuid NOT NULL,
    branch_id uuid NOT NULL,
    draft_id uuid NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    customer_id uuid NOT NULL,
    snapshot_json bytea NOT NULL,
    PRIMARY KEY (tenant_id, branch_id, draft_id, revision),
    FOREIGN KEY (tenant_id, branch_id) REFERENCES control.branches(tenant_id, branch_id)
);
ALTER TABLE sync.quotation_draft_revisions ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.quotation_draft_revisions FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.quotation_draft_revisions
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);
CREATE TRIGGER retain_quotation_draft_revision BEFORE UPDATE OR DELETE ON sync.quotation_draft_revisions
    FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
