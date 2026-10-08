CREATE TABLE sync.orders (
 tenant_id uuid NOT NULL, organization_id uuid NOT NULL, branch_id uuid NOT NULL,
 order_id uuid NOT NULL, draft_id uuid NOT NULL, document_revision bigint NOT NULL CHECK(document_revision>0),
 number text NOT NULL, PRIMARY KEY(tenant_id,order_id),
 UNIQUE(tenant_id,draft_id), UNIQUE(tenant_id,organization_id,number),
 FOREIGN KEY(tenant_id,branch_id) REFERENCES control.branches(tenant_id,branch_id)
);
CREATE TABLE sync.order_history (
 tenant_id uuid NOT NULL, order_id uuid NOT NULL, revision bigint NOT NULL CHECK(revision>0),
 record_json bytea NOT NULL, PRIMARY KEY(tenant_id,order_id,revision),
 FOREIGN KEY(tenant_id,order_id) REFERENCES sync.orders(tenant_id,order_id)
);
CREATE TABLE sync.order_work_history (
 tenant_id uuid NOT NULL, order_id uuid NOT NULL, work_id uuid NOT NULL,
 revision bigint NOT NULL CHECK(revision>0), number text NOT NULL, record_json bytea NOT NULL,
 PRIMARY KEY(tenant_id,work_id,revision),
 FOREIGN KEY(tenant_id,order_id) REFERENCES sync.orders(tenant_id,order_id)
);
CREATE TABLE sync.order_deliveries (
 tenant_id uuid NOT NULL, order_id uuid NOT NULL, delivery_id uuid NOT NULL, record_json bytea NOT NULL,
 PRIMARY KEY(tenant_id,delivery_id), UNIQUE(tenant_id,order_id),
 FOREIGN KEY(tenant_id,order_id) REFERENCES sync.orders(tenant_id,order_id)
);
CREATE TABLE sync.order_receipts (
 tenant_id uuid NOT NULL, idempotency_key uuid NOT NULL, request_hash bytea NOT NULL,
 response_json bytea NOT NULL, PRIMARY KEY(tenant_id,idempotency_key)
);
CREATE TABLE sync.order_numbers (
 tenant_id uuid NOT NULL, organization_id uuid NOT NULL, record_type text NOT NULL CHECK(record_type IN ('OR','WO')),
 calendar_year integer NOT NULL, last_number bigint NOT NULL CHECK(last_number>0),
 PRIMARY KEY(tenant_id,organization_id,record_type,calendar_year)
);
DO $orders$
DECLARE name text;
BEGIN
 FOREACH name IN ARRAY ARRAY['orders','order_history','order_work_history','order_deliveries','order_receipts','order_numbers'] LOOP
  EXECUTE format('ALTER TABLE sync.%I ENABLE ROW LEVEL SECURITY',name);
  EXECUTE format('ALTER TABLE sync.%I FORCE ROW LEVEL SECURITY',name);
  EXECUTE format('CREATE POLICY tenant_isolation ON sync.%I USING(tenant_id=nullif(current_setting(''eitmad.tenant_id'',true),'''')::uuid) WITH CHECK(tenant_id=nullif(current_setting(''eitmad.tenant_id'',true),'''')::uuid)',name);
  IF name <> 'order_numbers' THEN
   EXECUTE format('CREATE TRIGGER retain_order_record BEFORE UPDATE OR DELETE ON sync.%I FOR EACH ROW EXECUTE FUNCTION sync.retain_catalog_image()',name);
  END IF;
 END LOOP;
END $orders$;
