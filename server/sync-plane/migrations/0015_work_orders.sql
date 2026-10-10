CREATE TABLE sync.work_order_history (
 tenant_id uuid NOT NULL, organization_id uuid NOT NULL, branch_id uuid NOT NULL,
 order_id uuid NOT NULL, work_id uuid NOT NULL, revision bigint NOT NULL CHECK(revision>0),
 record_json bytea NOT NULL, PRIMARY KEY(tenant_id,work_id,revision),
 FOREIGN KEY(tenant_id,order_id) REFERENCES sync.orders(tenant_id,order_id),
 FOREIGN KEY(tenant_id,branch_id) REFERENCES control.branches(tenant_id,branch_id)
);
ALTER TABLE sync.work_order_history ENABLE ROW LEVEL SECURITY;
ALTER TABLE sync.work_order_history FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON sync.work_order_history
 USING(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid)
 WITH CHECK(tenant_id=nullif(current_setting('eitmad.tenant_id',true),'')::uuid);
CREATE TRIGGER retain_work_order BEFORE UPDATE OR DELETE ON sync.work_order_history
 FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image();
