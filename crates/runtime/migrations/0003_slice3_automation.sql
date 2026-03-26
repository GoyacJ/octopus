ALTER TABLE tasks ADD COLUMN source_kind TEXT NOT NULL DEFAULT 'manual';
ALTER TABLE tasks ADD COLUMN automation_id TEXT;

ALTER TABLE runs ADD COLUMN automation_id TEXT;
ALTER TABLE runs ADD COLUMN trigger_delivery_id TEXT;

CREATE TABLE IF NOT EXISTS automations (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    trigger_id TEXT NOT NULL,
    title TEXT NOT NULL,
    instruction TEXT NOT NULL,
    action_json TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    estimated_cost INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE IF NOT EXISTS triggers (
    id TEXT PRIMARY KEY,
    automation_id TEXT NOT NULL,
    trigger_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (automation_id) REFERENCES automations(id)
);

CREATE TABLE IF NOT EXISTS trigger_deliveries (
    id TEXT PRIMARY KEY,
    trigger_id TEXT NOT NULL,
    run_id TEXT,
    status TEXT NOT NULL,
    dedupe_key TEXT NOT NULL UNIQUE,
    payload_json TEXT NOT NULL,
    attempt_count INTEGER NOT NULL,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (trigger_id) REFERENCES triggers(id),
    FOREIGN KEY (run_id) REFERENCES runs(id)
);

CREATE INDEX IF NOT EXISTS idx_tasks_automation_id ON tasks (automation_id);
CREATE INDEX IF NOT EXISTS idx_runs_automation_id ON runs (automation_id);
CREATE INDEX IF NOT EXISTS idx_runs_trigger_delivery_id ON runs (trigger_delivery_id);
CREATE INDEX IF NOT EXISTS idx_automations_scope ON automations (workspace_id, project_id);
CREATE INDEX IF NOT EXISTS idx_triggers_automation_id ON triggers (automation_id);
CREATE INDEX IF NOT EXISTS idx_trigger_deliveries_trigger_id ON trigger_deliveries (trigger_id);
CREATE INDEX IF NOT EXISTS idx_trigger_deliveries_run_id ON trigger_deliveries (run_id);
