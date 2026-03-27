ALTER TABLE capability_descriptors ADD COLUMN kind TEXT NOT NULL DEFAULT 'core';
ALTER TABLE capability_descriptors ADD COLUMN source TEXT NOT NULL DEFAULT 'builtin';
ALTER TABLE capability_descriptors ADD COLUMN platform TEXT NOT NULL DEFAULT 'runtime_local';
ALTER TABLE capability_descriptors ADD COLUMN input_schema_uri TEXT;
ALTER TABLE capability_descriptors ADD COLUMN output_schema_uri TEXT;
ALTER TABLE capability_descriptors ADD COLUMN fallback_capability_id TEXT;
ALTER TABLE capability_descriptors ADD COLUMN trust_level TEXT NOT NULL DEFAULT 'trusted';

ALTER TABLE artifacts ADD COLUMN provenance_source TEXT NOT NULL DEFAULT 'builtin';
ALTER TABLE artifacts ADD COLUMN source_descriptor_id TEXT NOT NULL DEFAULT 'capability-unknown';
ALTER TABLE artifacts ADD COLUMN source_invocation_id TEXT;
ALTER TABLE artifacts ADD COLUMN trust_level TEXT NOT NULL DEFAULT 'trusted';
ALTER TABLE artifacts ADD COLUMN knowledge_gate_status TEXT NOT NULL DEFAULT 'eligible';

ALTER TABLE knowledge_candidates ADD COLUMN provenance_source TEXT NOT NULL DEFAULT 'builtin';
ALTER TABLE knowledge_candidates ADD COLUMN source_trust_level TEXT NOT NULL DEFAULT 'trusted';

CREATE TABLE IF NOT EXISTS mcp_servers (
    id TEXT PRIMARY KEY,
    capability_id TEXT NOT NULL,
    namespace TEXT NOT NULL,
    platform TEXT NOT NULL,
    trust_level TEXT NOT NULL,
    health_status TEXT NOT NULL,
    lease_ttl_seconds INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_checked_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_mcp_servers_capability_id
    ON mcp_servers(capability_id);

CREATE TABLE IF NOT EXISTS environment_leases (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    environment_type TEXT NOT NULL,
    sandbox_tier TEXT NOT NULL,
    status TEXT NOT NULL,
    heartbeat_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    resume_token TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id)
);

CREATE INDEX IF NOT EXISTS idx_environment_leases_run_id
    ON environment_leases(run_id);

CREATE INDEX IF NOT EXISTS idx_environment_leases_status_expires_at
    ON environment_leases(status, expires_at);

CREATE TABLE IF NOT EXISTS mcp_invocations (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    server_id TEXT NOT NULL,
    lease_id TEXT NOT NULL,
    namespace TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    request_json TEXT NOT NULL,
    response_json TEXT,
    status TEXT NOT NULL,
    error_message TEXT,
    retryable INTEGER NOT NULL,
    trust_level TEXT NOT NULL,
    gate_status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (server_id) REFERENCES mcp_servers(id),
    FOREIGN KEY (lease_id) REFERENCES environment_leases(id)
);

CREATE INDEX IF NOT EXISTS idx_mcp_invocations_run_id
    ON mcp_invocations(run_id);
