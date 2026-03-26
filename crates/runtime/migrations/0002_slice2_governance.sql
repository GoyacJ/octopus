ALTER TABLE tasks ADD COLUMN capability_id TEXT NOT NULL DEFAULT 'capability-unknown';
ALTER TABLE tasks ADD COLUMN estimated_cost INTEGER NOT NULL DEFAULT 0;

ALTER TABLE runs ADD COLUMN approval_request_id TEXT;

CREATE TABLE IF NOT EXISTS capability_descriptors (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL,
    risk_level TEXT NOT NULL,
    requires_approval INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS capability_bindings (
    id TEXT PRIMARY KEY,
    capability_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    scope_ref TEXT NOT NULL,
    binding_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS capability_grants (
    id TEXT PRIMARY KEY,
    capability_id TEXT NOT NULL,
    subject_ref TEXT NOT NULL,
    grant_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS budget_policies (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    soft_cost_limit INTEGER NOT NULL,
    hard_cost_limit INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS approval_requests (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    approval_type TEXT NOT NULL,
    status TEXT NOT NULL,
    reason TEXT NOT NULL,
    dedupe_key TEXT NOT NULL UNIQUE,
    decided_by TEXT,
    decision_note TEXT,
    decided_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE TABLE IF NOT EXISTS inbox_items (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    approval_request_id TEXT NOT NULL,
    item_type TEXT NOT NULL,
    status TEXT NOT NULL,
    dedupe_key TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (approval_request_id) REFERENCES approval_requests(id)
);

CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    approval_request_id TEXT NOT NULL,
    status TEXT NOT NULL,
    dedupe_key TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (approval_request_id) REFERENCES approval_requests(id)
);

CREATE TABLE IF NOT EXISTS policy_decision_logs (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    decision TEXT NOT NULL,
    reason TEXT NOT NULL,
    estimated_cost INTEGER NOT NULL,
    approval_request_id TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_tasks_capability_id ON tasks (capability_id);
CREATE INDEX IF NOT EXISTS idx_runs_approval_request_id ON runs (approval_request_id);
CREATE INDEX IF NOT EXISTS idx_capability_bindings_scope ON capability_bindings (capability_id, workspace_id, project_id);
CREATE INDEX IF NOT EXISTS idx_capability_grants_subject ON capability_grants (capability_id, subject_ref);
CREATE INDEX IF NOT EXISTS idx_budget_policies_scope ON budget_policies (workspace_id, project_id);
CREATE INDEX IF NOT EXISTS idx_approval_requests_run_id ON approval_requests (run_id);
CREATE INDEX IF NOT EXISTS idx_approval_requests_dedupe_key ON approval_requests (dedupe_key);
CREATE INDEX IF NOT EXISTS idx_inbox_items_run_id ON inbox_items (run_id);
CREATE INDEX IF NOT EXISTS idx_inbox_items_workspace_id ON inbox_items (workspace_id);
CREATE INDEX IF NOT EXISTS idx_notifications_run_id ON notifications (run_id);
CREATE INDEX IF NOT EXISTS idx_notifications_workspace_id ON notifications (workspace_id);
CREATE INDEX IF NOT EXISTS idx_policy_decision_logs_run_id ON policy_decision_logs (run_id);
