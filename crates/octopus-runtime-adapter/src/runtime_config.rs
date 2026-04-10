use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeConfigScopeKind {
    Workspace,
    Project,
    User,
}

const KNOWN_RUNTIME_CONFIG_TOP_LEVEL_KEYS: &[&str] = &[
    "$schema",
    "aliases",
    "configuredModels",
    "credentialRefs",
    "defaultSelections",
    "enabledPlugins",
    "env",
    "hooks",
    "mcpServers",
    "model",
    "modelRegistry",
    "oauth",
    "permissionMode",
    "permissions",
    "plugins",
    "projectSettings",
    "provider",
    "providerFallbacks",
    "providerOverrides",
    "sandbox",
    "trustedRoots",
];

const DEPRECATED_RUNTIME_CONFIG_TOP_LEVEL_KEYS: &[(&str, &str)] = &[
    ("allowedTools", "permissions.allow"),
    ("ignorePatterns", "permissions.deny"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RuntimeConfigDocumentRecord {
    pub(super) scope: RuntimeConfigScopeKind,
    pub(super) owner_id: Option<String>,
    pub(super) source_key: String,
    pub(super) display_path: String,
    pub(super) storage_path: PathBuf,
    pub(super) exists: bool,
    pub(super) loaded: bool,
    pub(super) document: Option<std::collections::BTreeMap<String, JsonValue>>,
}

impl RuntimeAdapter {
    pub(super) fn hash_value(value: &serde_json::Value) -> Result<String, AppError> {
        let encoded = serde_json::to_vec(value)?;
        let digest = Sha256::digest(encoded);
        Ok(format!("{digest:x}"))
    }

    pub(super) fn runtime_json_to_serde(value: &JsonValue) -> serde_json::Value {
        match value {
            JsonValue::Null => serde_json::Value::Null,
            JsonValue::Bool(value) => serde_json::Value::Bool(*value),
            JsonValue::Number(value) => serde_json::Value::Number((*value).into()),
            JsonValue::String(value) => serde_json::Value::String(value.clone()),
            JsonValue::Array(values) => {
                serde_json::Value::Array(values.iter().map(Self::runtime_json_to_serde).collect())
            }
            JsonValue::Object(entries) => serde_json::Value::Object(
                entries
                    .iter()
                    .map(|(key, value)| (key.clone(), Self::runtime_json_to_serde(value)))
                    .collect(),
            ),
        }
    }

    pub(super) fn serde_to_runtime_json(value: &serde_json::Value) -> Result<JsonValue, AppError> {
        JsonValue::parse(&serde_json::to_string(value)?)
            .map_err(|error| AppError::invalid_input(error.to_string()))
    }

    pub(super) fn public_scope_label(scope: RuntimeConfigScopeKind) -> &'static str {
        match scope {
            RuntimeConfigScopeKind::Workspace => "workspace",
            RuntimeConfigScopeKind::Project => "project",
            RuntimeConfigScopeKind::User => "user",
        }
    }

    pub(super) fn parse_scope(scope: &str) -> Result<RuntimeConfigScopeKind, AppError> {
        match scope {
            "workspace" => Ok(RuntimeConfigScopeKind::Workspace),
            "project" => Ok(RuntimeConfigScopeKind::Project),
            "user" => Ok(RuntimeConfigScopeKind::User),
            other => Err(AppError::invalid_input(format!(
                "unsupported runtime config scope: {other}"
            ))),
        }
    }

    pub(super) fn internal_scope(scope: RuntimeConfigScopeKind) -> ConfigSource {
        match scope {
            RuntimeConfigScopeKind::User => ConfigSource::User,
            RuntimeConfigScopeKind::Workspace => ConfigSource::Project,
            RuntimeConfigScopeKind::Project => ConfigSource::Local,
        }
    }

    pub(super) fn scope_precedence(scope: RuntimeConfigScopeKind) -> u8 {
        match scope {
            RuntimeConfigScopeKind::User => 0,
            RuntimeConfigScopeKind::Workspace => 1,
            RuntimeConfigScopeKind::Project => 2,
        }
    }

    pub(super) fn workspace_config_path(&self) -> PathBuf {
        self.state.paths.runtime_config_dir.join("workspace.json")
    }

    pub(super) fn project_config_path(&self, project_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_project_config_dir
            .join(format!("{project_id}.json"))
    }

    pub(super) fn user_config_path(&self, user_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_user_config_dir
            .join(format!("{user_id}.json"))
    }

    pub(super) fn ensure_runtime_config_layout(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.state.paths.runtime_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_project_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_user_config_dir)?;
        Ok(())
    }

    pub(super) fn read_optional_runtime_document(
        path: &Path,
    ) -> Result<Option<BTreeMap<String, JsonValue>>, AppError> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Ok(None);
                }
                let parsed = JsonValue::parse(trimmed)
                    .map_err(|error| AppError::runtime(error.to_string()))?;
                parsed.as_object().cloned().map(Some).ok_or_else(|| {
                    AppError::runtime("runtime config document must be a JSON object")
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn write_runtime_document(
        &self,
        path: &Path,
        document: &BTreeMap<String, JsonValue>,
    ) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let rendered = serde_json::to_vec_pretty(&Self::runtime_json_to_serde(
            &JsonValue::Object(document.clone()),
        ))?;
        fs::write(path, rendered)?;
        Ok(())
    }

    pub(super) fn migrate_legacy_workspace_config_if_needed(&self) -> Result<(), AppError> {
        let workspace_path = self.workspace_config_path();
        if workspace_path.exists() {
            return Ok(());
        }

        self.ensure_runtime_config_layout()?;

        let mut merged = BTreeMap::new();
        let mut found = false;
        for legacy_path in [
            self.state
                .paths
                .config_dir
                .join(".claw")
                .join("settings.json"),
            self.state.paths.root.join(".claw.json"),
            self.state.paths.root.join(".claw").join("settings.json"),
        ] {
            if let Some(document) = Self::read_optional_runtime_document(&legacy_path)? {
                apply_config_patch(&mut merged, &document);
                found = true;
            }
        }

        if found {
            self.write_runtime_document(&workspace_path, &merged)?;
        }

        Ok(())
    }

    pub(super) fn config_document_record(
        &self,
        scope: RuntimeConfigScopeKind,
        owner_id: Option<&str>,
        storage_path: PathBuf,
        display_path: String,
        source_key: String,
    ) -> Result<RuntimeConfigDocumentRecord, AppError> {
        let document = Self::read_optional_runtime_document(&storage_path)?;
        Ok(RuntimeConfigDocumentRecord {
            scope,
            owner_id: owner_id.map(ToOwned::to_owned),
            source_key,
            display_path,
            exists: storage_path.exists(),
            loaded: document.is_some(),
            storage_path,
            document,
        })
    }

    pub(super) fn resolve_documents(
        &self,
        project_id: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<Vec<RuntimeConfigDocumentRecord>, AppError> {
        self.ensure_runtime_config_layout()?;
        self.migrate_legacy_workspace_config_if_needed()?;

        let mut documents = vec![self.config_document_record(
            RuntimeConfigScopeKind::Workspace,
            None,
            self.workspace_config_path(),
            "config/runtime/workspace.json".to_string(),
            "workspace".to_string(),
        )?];

        if let Some(project_id) = project_id.filter(|value| !value.is_empty()) {
            documents.push(self.config_document_record(
                RuntimeConfigScopeKind::Project,
                Some(project_id),
                self.project_config_path(project_id),
                format!("config/runtime/projects/{project_id}.json"),
                format!("project:{project_id}"),
            )?);
        }

        if let Some(user_id) = user_id.filter(|value| !value.is_empty()) {
            documents.push(self.config_document_record(
                RuntimeConfigScopeKind::User,
                Some(user_id),
                self.user_config_path(user_id),
                format!("config/runtime/users/{user_id}.json"),
                format!("user:{user_id}"),
            )?);
        }

        documents.sort_by_key(|document| Self::scope_precedence(document.scope));

        Ok(documents)
    }

    pub(super) fn to_internal_documents(
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Vec<ConfigDocument> {
        documents
            .iter()
            .map(|document| ConfigDocument {
                source: Self::internal_scope(document.scope),
                path: document.storage_path.clone(),
                exists: document.exists,
                loaded: document.loaded,
                document: document.document.clone(),
            })
            .collect()
    }

    fn is_sensitive_key(key: &str) -> bool {
        let normalized = key
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        [
            "apikey",
            "token",
            "secret",
            "password",
            "authorization",
            "authtoken",
            "clientsecret",
            "accesskey",
        ]
        .iter()
        .any(|needle| normalized.contains(needle))
    }

    fn is_reference_value(value: &str) -> bool {
        value.starts_with("env:")
            || value.starts_with("keychain:")
            || value.starts_with("op://")
            || value.starts_with("vault:")
            || value.starts_with("secret-ref:")
    }

    fn record_secret_reference(
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
        scope: &str,
        path: &str,
        reference: Option<String>,
        status: &str,
    ) {
        if secret_references.iter().any(|existing| {
            existing.scope == scope
                && existing.path == path
                && existing.reference == reference
                && existing.status == status
        }) {
            return;
        }

        secret_references.push(RuntimeSecretReferenceStatus {
            scope: scope.to_string(),
            path: path.to_string(),
            reference,
            status: status.to_string(),
        });
    }

    fn redact_secret_value(
        scope: &str,
        path: &str,
        raw: &str,
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
    ) -> serde_json::Value {
        if Self::is_reference_value(raw) {
            let status = if let Some(env_key) = raw.strip_prefix("env:") {
                if std::env::var_os(env_key).is_some() {
                    "reference-present"
                } else {
                    "reference-missing"
                }
            } else {
                "reference-present"
            };
            Self::record_secret_reference(
                secret_references,
                scope,
                path,
                Some(raw.to_string()),
                status,
            );
            return serde_json::Value::String(raw.to_string());
        }

        Self::record_secret_reference(secret_references, scope, path, None, "inline-redacted");
        serde_json::Value::String("***".to_string())
    }

    fn redact_config_value(
        scope: &str,
        path: &str,
        value: &serde_json::Value,
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
    ) -> serde_json::Value {
        match value {
            serde_json::Value::Object(object) => serde_json::Value::Object(
                object
                    .iter()
                    .map(|(key, value)| {
                        let next_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{path}.{key}")
                        };
                        let next_value = match value {
                            serde_json::Value::String(raw) if Self::is_sensitive_key(key) => {
                                Self::redact_secret_value(scope, &next_path, raw, secret_references)
                            }
                            _ => Self::redact_config_value(
                                scope,
                                &next_path,
                                value,
                                secret_references,
                            ),
                        };
                        (key.clone(), next_value)
                    })
                    .collect(),
            ),
            serde_json::Value::Array(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|value| Self::redact_config_value(scope, path, value, secret_references))
                    .collect(),
            ),
            _ => value.clone(),
        }
    }

    pub(super) fn validate_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let internal_documents = Self::to_internal_documents(documents);
        let warnings = Self::document_validation_warnings(documents);
        match self
            .state
            .config_loader
            .load_from_documents(&internal_documents)
        {
            Ok(_) => Ok(RuntimeConfigValidationResult {
                valid: true,
                errors: Vec::new(),
                warnings,
            }),
            Err(error) => Ok(RuntimeConfigValidationResult {
                valid: false,
                errors: vec![error.to_string()],
                warnings,
            }),
        }
    }

    fn document_validation_warnings(documents: &[RuntimeConfigDocumentRecord]) -> Vec<String> {
        let mut warnings = Vec::new();
        for document in documents {
            let Some(record) = document.document.as_ref() else {
                continue;
            };
            for key in record.keys() {
                if let Some((_, replacement)) = DEPRECATED_RUNTIME_CONFIG_TOP_LEVEL_KEYS
                    .iter()
                    .find(|(deprecated, _)| key == deprecated)
                {
                    warnings.push(format!(
                        "{}: deprecated runtime config key `{key}`; use `{replacement}` instead",
                        document.display_path
                    ));
                    continue;
                }
                if !KNOWN_RUNTIME_CONFIG_TOP_LEVEL_KEYS.contains(&key.as_str()) {
                    warnings.push(format!(
                        "{}: unknown runtime config key `{key}`",
                        document.display_path
                    ));
                }
            }
        }
        warnings
    }

    pub(super) fn load_effective_config_json(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<Value, AppError> {
        let mut merged = BTreeMap::new();
        for document in documents {
            if let Some(record) = &document.document {
                apply_config_patch(&mut merged, record);
            }
        }
        let mut effective_config = Self::runtime_json_to_serde(&JsonValue::Object(merged));
        let project_assignments = self.load_project_assignments_for_documents(documents)?;
        merge_project_assignments(&mut effective_config, project_assignments.as_ref());
        Ok(effective_config)
    }

    pub(super) fn effective_registry_from_json(
        &self,
        effective_config: &Value,
    ) -> Result<EffectiveModelRegistry, AppError> {
        EffectiveModelRegistry::from_effective_config(effective_config)
    }

    pub(super) fn effective_registry(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<EffectiveModelRegistry, AppError> {
        let effective_config = self.load_effective_config_json(documents)?;
        self.effective_registry_from_json(&effective_config)
    }

    pub(super) fn build_effective_config(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let mut secret_references = Vec::new();
        let effective_value = self.load_effective_config_json(documents)?;
        let effective_config =
            Self::redact_config_value("effective", "", &effective_value, &mut Vec::new());
        let effective_config_hash = Self::hash_value(&effective_value)?;

        let sources = documents
            .iter()
            .map(|document| {
                let document_value = document
                    .document
                    .as_ref()
                    .map(|value| Self::runtime_json_to_serde(&JsonValue::Object(value.clone())));
                let redacted_document = document_value.as_ref().map(|value| {
                    Self::redact_config_value(
                        Self::public_scope_label(document.scope),
                        "",
                        value,
                        &mut secret_references,
                    )
                });
                let content_hash = document_value.as_ref().map(Self::hash_value).transpose()?;

                Ok(RuntimeConfigSource {
                    scope: Self::public_scope_label(document.scope).to_string(),
                    owner_id: document.owner_id.clone(),
                    display_path: document.display_path.clone(),
                    source_key: document.source_key.clone(),
                    exists: document.exists,
                    loaded: document.loaded,
                    content_hash,
                    document: redacted_document,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(RuntimeEffectiveConfig {
            effective_config,
            effective_config_hash,
            sources,
            validation: RuntimeConfigValidationResult {
                valid: true,
                errors: Vec::new(),
                warnings: Vec::new(),
            },
            secret_references,
        })
    }

    pub(super) fn validate_registry_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let registry = self.effective_registry(documents)?;
        let mut validation = self.validate_documents(documents)?;
        validation
            .warnings
            .extend(registry.diagnostics().warnings.clone());
        validation
            .errors
            .extend(registry.diagnostics().errors.clone());
        validation.valid = validation.errors.is_empty();
        Ok(validation)
    }

    pub(super) fn patched_documents(
        &self,
        scope: RuntimeConfigScopeKind,
        project_id: Option<&str>,
        user_id: Option<&str>,
        patch: &serde_json::Value,
    ) -> Result<Vec<RuntimeConfigDocumentRecord>, AppError> {
        let patch = Self::serde_to_runtime_json(patch)?;
        let patch_object = patch
            .as_object()
            .ok_or_else(|| AppError::invalid_input("runtime config patch must be a JSON object"))?;

        let mut documents = self.resolve_documents(project_id, user_id)?;

        let target_document = documents
            .iter_mut()
            .find(|document| document.scope == scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        let mut next = target_document.document.clone().unwrap_or_default();
        apply_config_patch(&mut next, patch_object);
        target_document.exists = true;
        target_document.loaded = true;
        target_document.document = Some(next);

        Ok(documents)
    }

    pub(super) fn write_document(
        &self,
        document: &RuntimeConfigDocumentRecord,
    ) -> Result<(), AppError> {
        self.write_runtime_document(
            &document.storage_path,
            &document.document.clone().unwrap_or_default(),
        )
    }

    pub(super) fn current_config_snapshot(
        &self,
        project_id: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<RuntimeConfigSnapshotSummary, AppError> {
        let documents = self.resolve_documents(project_id, user_id)?;
        let effective = self.build_effective_config(&documents)?;
        let effective_config = self.load_effective_config_json(&documents)?;
        let source_refs = documents
            .iter()
            .filter(|document| document.loaded)
            .map(|document| document.source_key.clone())
            .collect::<Vec<_>>();
        let mut started_from_scope_set = Vec::new();
        for document in &documents {
            let scope = Self::public_scope_label(document.scope).to_string();
            if document.loaded
                && !started_from_scope_set
                    .iter()
                    .any(|existing| existing == &scope)
            {
                started_from_scope_set.push(scope);
            }
        }

        Ok(RuntimeConfigSnapshotSummary {
            id: format!("cfgsnap-{}", Uuid::new_v4()),
            effective_config_hash: effective.effective_config_hash,
            started_from_scope_set,
            source_refs,
            created_at: timestamp_now(),
            effective_config: Some(effective_config),
        })
    }

    pub(super) fn config_snapshot_value(&self, snapshot_id: &str) -> Result<Value, AppError> {
        self.state
            .config_snapshots
            .lock()
            .map_err(|_| AppError::runtime("runtime config snapshots mutex poisoned"))?
            .get(snapshot_id)
            .cloned()
            .ok_or_else(|| {
                AppError::runtime(format!(
                    "runtime config snapshot `{snapshot_id}` is unavailable"
                ))
            })
    }
}
