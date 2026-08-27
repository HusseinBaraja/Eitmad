-- eitmad:phase:prepare
CREATE OR REPLACE FUNCTION audit.reject_server_record_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND current_setting('eitmad.audit_migration', true) = 'backfill-v4' THEN
        RETURN NEW;
    END IF;
    RAISE EXCEPTION 'server audit is append-only';
END;
$$;

ALTER TABLE audit.server_records
    ADD COLUMN IF NOT EXISTS actor_kind text,
    ADD COLUMN IF NOT EXISTS workspace_id uuid,
    ADD COLUMN IF NOT EXISTS scope_kind text,
    ADD COLUMN IF NOT EXISTS scope_id uuid,
    ADD COLUMN IF NOT EXISTS target_id uuid,
    ADD COLUMN IF NOT EXISTS causation_id uuid,
    ADD COLUMN IF NOT EXISTS idempotency_key uuid,
    ALTER COLUMN session_id DROP NOT NULL,
    ALTER COLUMN principal_id DROP NOT NULL;

-- eitmad:phase:validate
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'server_audit_actor_kind'
                   AND conrelid = 'audit.server_records'::regclass) THEN
        ALTER TABLE audit.server_records ADD CONSTRAINT server_audit_actor_kind
            CHECK (actor_kind IN ('user', 'service', 'device', 'system')) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'server_audit_actor_kind_present'
                   AND conrelid = 'audit.server_records'::regclass) THEN
        ALTER TABLE audit.server_records ADD CONSTRAINT server_audit_actor_kind_present
            CHECK (actor_kind IS NOT NULL) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'server_audit_scope_kind_present'
                   AND conrelid = 'audit.server_records'::regclass) THEN
        ALTER TABLE audit.server_records ADD CONSTRAINT server_audit_scope_kind_present
            CHECK (scope_kind IS NOT NULL) NOT VALID;
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'server_audit_scope_id_present'
                   AND conrelid = 'audit.server_records'::regclass) THEN
        ALTER TABLE audit.server_records ADD CONSTRAINT server_audit_scope_id_present
            CHECK (scope_id IS NOT NULL) NOT VALID;
    END IF;
END;
$$;

ALTER TABLE audit.server_records VALIDATE CONSTRAINT server_audit_actor_kind;
ALTER TABLE audit.server_records VALIDATE CONSTRAINT server_audit_actor_kind_present;
ALTER TABLE audit.server_records VALIDATE CONSTRAINT server_audit_scope_kind_present;
ALTER TABLE audit.server_records VALIDATE CONSTRAINT server_audit_scope_id_present;

-- eitmad:phase:finalize
ALTER TABLE audit.server_records
    ALTER COLUMN actor_kind SET NOT NULL,
    ALTER COLUMN scope_kind SET NOT NULL,
    ALTER COLUMN scope_id SET NOT NULL,
    DROP CONSTRAINT server_audit_actor_kind_present,
    DROP CONSTRAINT server_audit_scope_kind_present,
    DROP CONSTRAINT server_audit_scope_id_present;

CREATE OR REPLACE FUNCTION audit.reject_server_record_change() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'server audit is append-only';
END;
$$;
