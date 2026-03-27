ALTER TABLE triggers ADD COLUMN schedule TEXT;
ALTER TABLE triggers ADD COLUMN timezone TEXT;
ALTER TABLE triggers ADD COLUMN next_fire_at TEXT;
ALTER TABLE triggers ADD COLUMN ingress_mode TEXT;
ALTER TABLE triggers ADD COLUMN secret_header_name TEXT;
ALTER TABLE triggers ADD COLUMN secret_hint TEXT;
ALTER TABLE triggers ADD COLUMN webhook_secret_hash TEXT;
ALTER TABLE triggers ADD COLUMN server_id TEXT;
ALTER TABLE triggers ADD COLUMN event_name TEXT;
ALTER TABLE triggers ADD COLUMN event_pattern TEXT;

CREATE INDEX IF NOT EXISTS idx_triggers_type_next_fire_at
    ON triggers(trigger_type, next_fire_at);
