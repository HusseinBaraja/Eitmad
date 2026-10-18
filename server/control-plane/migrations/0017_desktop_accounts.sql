ALTER TABLE control.accounts ADD COLUMN display_name text NOT NULL DEFAULT '';
UPDATE control.accounts SET display_name=username WHERE display_name='';
ALTER TABLE control.tenants ADD COLUMN account_revision bigint NOT NULL DEFAULT 1;
