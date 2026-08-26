DROP TRIGGER server_audit_no_update ON audit.server_records;
DROP TRIGGER server_audit_no_delete ON audit.server_records;

ALTER TABLE audit.server_records
    ADD COLUMN actor_kind text,
    ADD COLUMN workspace_id uuid,
    ADD COLUMN scope_kind text,
    ADD COLUMN scope_id uuid,
    ADD COLUMN target_id uuid,
    ADD COLUMN causation_id uuid,
    ADD COLUMN idempotency_key uuid;

UPDATE audit.server_records
SET actor_kind = 'user', scope_kind = 'tenant', scope_id = tenant_id
WHERE actor_kind IS NULL OR scope_kind IS NULL OR scope_id IS NULL;

ALTER TABLE audit.server_records
    ALTER COLUMN session_id DROP NOT NULL,
    ALTER COLUMN principal_id DROP NOT NULL,
    ALTER COLUMN actor_kind SET NOT NULL,
    ALTER COLUMN scope_kind SET NOT NULL,
    ALTER COLUMN scope_id SET NOT NULL;

ALTER TABLE audit.server_records
    ADD CONSTRAINT server_audit_actor_kind
    CHECK (actor_kind IN ('user', 'service', 'device', 'system'));

CREATE TRIGGER server_audit_no_update BEFORE UPDATE ON audit.server_records
FOR EACH ROW EXECUTE FUNCTION audit.reject_server_record_change();
CREATE TRIGGER server_audit_no_delete BEFORE DELETE ON audit.server_records
FOR EACH ROW EXECUTE FUNCTION audit.reject_server_record_change();
