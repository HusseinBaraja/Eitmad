CREATE TABLE sync.catalog_revisions (
    tenant_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('unit','material','part','product-category','furniture-category','product','furniture')),
    entry_id uuid NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    record_json bytea NOT NULL,
    PRIMARY KEY (tenant_id,organization_id,kind,entry_id,revision)
);
ALTER TABLE sync.catalog_revisions ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.catalog_revisions FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.catalog_revisions
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);
CREATE TRIGGER retain_catalog_revision BEFORE UPDATE OR DELETE ON sync.catalog_revisions
    FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
