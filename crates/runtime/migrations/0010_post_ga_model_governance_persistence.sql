CREATE TABLE IF NOT EXISTS model_providers (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    provider_family TEXT NOT NULL,
    status TEXT NOT NULL,
    default_base_url TEXT,
    protocol_families_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS model_catalog_items (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    model_key TEXT NOT NULL,
    provider_model_id TEXT NOT NULL,
    release_channel TEXT NOT NULL,
    modality_tags_json TEXT NOT NULL,
    feature_tags_json TEXT NOT NULL,
    context_window INTEGER NOT NULL,
    max_output_tokens INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (provider_id) REFERENCES model_providers(id)
);

CREATE TABLE IF NOT EXISTS model_profiles (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    scope_ref TEXT NOT NULL,
    primary_model_key TEXT NOT NULL,
    fallback_model_keys_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tenant_model_policies (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL UNIQUE,
    allowed_model_keys_json TEXT NOT NULL,
    denied_model_keys_json TEXT NOT NULL,
    allowed_provider_ids_json TEXT NOT NULL,
    denied_release_channels_json TEXT NOT NULL,
    require_approval_for_preview INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS model_selection_decisions (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL UNIQUE,
    model_profile_id TEXT,
    requested_intent TEXT NOT NULL,
    decision_outcome TEXT NOT NULL,
    selected_model_key TEXT,
    selected_provider_id TEXT,
    required_feature_tags_json TEXT NOT NULL,
    missing_feature_tags_json TEXT NOT NULL,
    requires_approval INTEGER NOT NULL,
    decision_reason TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (run_id) REFERENCES runs(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_model_catalog_items_model_key
    ON model_catalog_items(model_key);
CREATE INDEX IF NOT EXISTS idx_model_catalog_items_provider_id
    ON model_catalog_items(provider_id);
CREATE INDEX IF NOT EXISTS idx_model_profiles_scope_ref
    ON model_profiles(scope_ref);
CREATE INDEX IF NOT EXISTS idx_model_selection_decisions_run_id
    ON model_selection_decisions(run_id);
