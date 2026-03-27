CREATE TABLE IF NOT EXISTS knowledge_spaces (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT,
    owner_ref TEXT NOT NULL,
    display_name TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_knowledge_spaces_workspace_project
    ON knowledge_spaces(workspace_id, project_id);

CREATE TABLE IF NOT EXISTS knowledge_candidates (
    id TEXT PRIMARY KEY,
    knowledge_space_id TEXT NOT NULL,
    source_run_id TEXT NOT NULL,
    source_task_id TEXT NOT NULL,
    source_artifact_id TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    status TEXT NOT NULL,
    content TEXT NOT NULL,
    dedupe_key TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (knowledge_space_id) REFERENCES knowledge_spaces(id),
    FOREIGN KEY (source_run_id) REFERENCES runs(id),
    FOREIGN KEY (source_task_id) REFERENCES tasks(id),
    FOREIGN KEY (source_artifact_id) REFERENCES artifacts(id)
);

CREATE TABLE IF NOT EXISTS knowledge_assets (
    id TEXT PRIMARY KEY,
    knowledge_space_id TEXT NOT NULL,
    source_candidate_id TEXT NOT NULL UNIQUE,
    capability_id TEXT NOT NULL,
    status TEXT NOT NULL,
    content TEXT NOT NULL,
    trust_level TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (knowledge_space_id) REFERENCES knowledge_spaces(id),
    FOREIGN KEY (source_candidate_id) REFERENCES knowledge_candidates(id)
);

CREATE TABLE IF NOT EXISTS knowledge_capture_retries (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL UNIQUE,
    task_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL,
    capability_id TEXT NOT NULL,
    last_error TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    resolved_at TEXT,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id),
    FOREIGN KEY (artifact_id) REFERENCES artifacts(id)
);

CREATE TABLE IF NOT EXISTS knowledge_lineage_records (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    task_id TEXT NOT NULL,
    source_ref TEXT NOT NULL,
    target_ref TEXT NOT NULL,
    relation_type TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (run_id) REFERENCES runs(id),
    FOREIGN KEY (task_id) REFERENCES tasks(id),
    UNIQUE(run_id, source_ref, target_ref, relation_type)
);
