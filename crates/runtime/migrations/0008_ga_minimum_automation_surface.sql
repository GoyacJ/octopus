ALTER TABLE automations ADD COLUMN status TEXT NOT NULL DEFAULT 'active';

UPDATE automations
SET status = 'active'
WHERE status IS NULL OR status = '';
