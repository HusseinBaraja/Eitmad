ALTER TABLE control.accounts ADD COLUMN display_name text NOT NULL DEFAULT '';

-- Backfill retained accounts under each tenant's forced RLS context.
DO $$ DECLARE tenant uuid; BEGIN
    FOR tenant IN SELECT tenant_id FROM control.tenants LOOP
        PERFORM set_config('eitmad.tenant_id', tenant::text, true);
        UPDATE control.accounts SET display_name=username WHERE tenant_id=tenant AND display_name='';
    END LOOP;
END $$;
ALTER TABLE control.tenants ADD COLUMN account_revision bigint NOT NULL DEFAULT 1;
