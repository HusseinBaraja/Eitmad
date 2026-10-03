CREATE TABLE sync.catalog_images (
    tenant_id uuid NOT NULL,
    organization_id uuid NOT NULL,
    id uuid NOT NULL,
    kind text NOT NULL CHECK (kind IN ('product','furniture')),
    sha256 text NOT NULL,
    content bytea NOT NULL CHECK (octet_length(content) BETWEEN 1 AND 8388608),
    PRIMARY KEY (tenant_id, organization_id, id)
);
ALTER TABLE sync.catalog_images ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.catalog_images FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.catalog_images
    USING (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid)
    WITH CHECK (tenant_id = nullif(current_setting('eitmad.tenant_id', true), '')::uuid);
CREATE FUNCTION sync.retain_catalog_image() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'catalog images are immutable and retained'; END;
$$;
CREATE TRIGGER retain_catalog_image BEFORE UPDATE OR DELETE ON sync.catalog_images
    FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
