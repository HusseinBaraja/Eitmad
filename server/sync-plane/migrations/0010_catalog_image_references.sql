ALTER TABLE sync.records ADD COLUMN public_image_id uuid;
ALTER TABLE sync.records ADD COLUMN public_image_sha256 text;
CREATE INDEX catalog_public_image_reference ON sync.records
    (tenant_id, scope_id, public_image_id, public_image_sha256)
    WHERE scope_kind = 'organization'
      AND schema_id = 'eitmad.schema.catalog-public.v1' AND NOT tombstone;
