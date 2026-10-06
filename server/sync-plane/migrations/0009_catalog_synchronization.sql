ALTER TABLE sync.catalog_revisions DROP CONSTRAINT catalog_revisions_kind_check;
ALTER TABLE sync.catalog_revisions ADD CONSTRAINT catalog_revisions_kind_check
    CHECK (kind IN ('unit','material-category','material','part-category','part',
                   'product-category','furniture-category','product','furniture'));
ALTER TABLE sync.catalog_revisions ADD CONSTRAINT catalog_revision_organization
    FOREIGN KEY (tenant_id, organization_id)
    REFERENCES control.organizations (tenant_id, organization_id);
