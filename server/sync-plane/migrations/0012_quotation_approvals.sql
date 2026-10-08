CREATE TABLE sync.quotation_approval_history (
 tenant_id uuid NOT NULL, organization_id uuid NOT NULL, branch_id uuid NOT NULL,
 draft_id uuid NOT NULL, revision bigint NOT NULL CHECK(revision>0), record_json bytea NOT NULL,
 PRIMARY KEY(tenant_id,draft_id,revision),
 FOREIGN KEY(tenant_id,branch_id) REFERENCES control.branches(tenant_id,branch_id)
);
CREATE TABLE sync.quotation_approval_receipts (
 tenant_id uuid NOT NULL, idempotency_key uuid NOT NULL, request_hash bytea NOT NULL,
 response_json bytea NOT NULL, PRIMARY KEY(tenant_id,idempotency_key)
);
ALTER TABLE sync.quotation_approval_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.quotation_approval_history FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.quotation_approval_history
 USING(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid)
 WITH CHECK(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid);
ALTER TABLE sync.quotation_approval_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.quotation_approval_receipts FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.quotation_approval_receipts
 USING(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid)
 WITH CHECK(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid);
CREATE TRIGGER retain_approval_history BEFORE UPDATE OR DELETE ON sync.quotation_approval_history
 FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
CREATE TRIGGER retain_approval_receipt BEFORE UPDATE OR DELETE ON sync.quotation_approval_receipts
 FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
