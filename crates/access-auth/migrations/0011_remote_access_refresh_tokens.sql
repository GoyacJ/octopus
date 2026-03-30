CREATE TABLE IF NOT EXISTS auth_refresh_tokens (
    id TEXT PRIMARY KEY,
    family_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    issued_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    rotated_at TEXT,
    replaced_by_token_id TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(session_id) REFERENCES auth_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY(user_id) REFERENCES remote_users(id) ON DELETE CASCADE,
    FOREIGN KEY(workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY(replaced_by_token_id) REFERENCES auth_refresh_tokens(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_family
ON auth_refresh_tokens(family_id);

CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_session
ON auth_refresh_tokens(session_id);

CREATE INDEX IF NOT EXISTS idx_auth_refresh_tokens_expires
ON auth_refresh_tokens(expires_at);
