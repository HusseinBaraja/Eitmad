CREATE TABLE sync.quotation_history (
 tenant_id uuid NOT NULL, organization_id uuid NOT NULL, branch_id uuid NOT NULL,
 draft_id uuid NOT NULL, revision bigint NOT NULL CHECK(revision>0),
 state text NOT NULL, valid_until bigint, record_json bytea NOT NULL,
 PRIMARY KEY(tenant_id,draft_id,revision),
 FOREIGN KEY(tenant_id,branch_id) REFERENCES control.branches(tenant_id,branch_id)
);
CREATE TABLE sync.quotation_receipts (
 tenant_id uuid NOT NULL, idempotency_key uuid NOT NULL, request_hash bytea NOT NULL,
 response_json bytea NOT NULL, PRIMARY KEY(tenant_id,idempotency_key)
);
CREATE INDEX quotation_due_expiry ON sync.quotation_history(tenant_id,valid_until,draft_id,revision)
 WHERE state='Issued';
-- Reservations commit independently: a later failed issue can leave a gap, never recycle a number.
CREATE TABLE sync.quotation_numbers (
 tenant_id uuid NOT NULL, organization_id uuid NOT NULL, calendar_year integer NOT NULL,
 last_number bigint NOT NULL CHECK(last_number>0),
 PRIMARY KEY(tenant_id,organization_id,calendar_year)
);
ALTER TABLE sync.quotation_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.quotation_history FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.quotation_history
 USING(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid)
 WITH CHECK(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid);
ALTER TABLE sync.quotation_receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.quotation_receipts FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.quotation_receipts
 USING(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid)
 WITH CHECK(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid);
ALTER TABLE sync.quotation_numbers ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.quotation_numbers FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.quotation_numbers
 USING(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid)
 WITH CHECK(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid);
CREATE TRIGGER retain_quotation_history BEFORE UPDATE OR DELETE ON sync.quotation_history
 FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
CREATE TRIGGER retain_quotation_receipt BEFORE UPDATE OR DELETE ON sync.quotation_receipts
 FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
