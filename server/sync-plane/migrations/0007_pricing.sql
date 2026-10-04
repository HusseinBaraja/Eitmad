CREATE TABLE sync.price_revisions (
    tenant_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('product','furniture')),
    entry_id uuid NOT NULL,
    variant_id uuid NOT NULL,
    revision bigint NOT NULL CHECK (revision > 0),
    record_json bytea NOT NULL,
    PRIMARY KEY (tenant_id,organization_id,kind,entry_id,variant_id,revision)
);
CREATE TABLE sync.price_receipts (
    tenant_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    idempotency_key uuid NOT NULL,
    request_hash bytea NOT NULL,
    record_json bytea NOT NULL,
    PRIMARY KEY (tenant_id,organization_id,idempotency_key)
);
ALTER TABLE sync.price_revisions ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.price_revisions FORCE ROW LEVEL SECURITY;
ALTER TABLE sync.price_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.price_receipts FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.price_revisions
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);
CREATE POLICY tenant_isolation ON sync.price_receipts
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);
CREATE TRIGGER retain_price_revision BEFORE UPDATE OR DELETE ON sync.price_revisions
    FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
CREATE TRIGGER retain_price_receipt BEFORE UPDATE OR DELETE ON sync.price_receipts
    FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
