ALTER TABLE mcp_servers ADD COLUMN transport_kind TEXT NOT NULL DEFAULT 'simulated';
ALTER TABLE mcp_servers ADD COLUMN endpoint TEXT;
ALTER TABLE mcp_servers ADD COLUMN request_timeout_ms INTEGER NOT NULL DEFAULT 1000;
ALTER TABLE mcp_servers ADD COLUMN credential_ref TEXT;

CREATE TABLE IF NOT EXISTS mcp_credentials (
    id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    credential_kind TEXT NOT NULL,
    header_name TEXT NOT NULL,
    auth_scheme TEXT,
    secret_value TEXT NOT NULL,
    secret_present INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mcp_credentials_namespace
    ON mcp_credentials(namespace);
