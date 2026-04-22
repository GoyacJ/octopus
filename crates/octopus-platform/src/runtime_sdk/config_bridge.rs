use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use octopus_core::{
    AppError, RuntimeConfigPatch, RuntimeConfigSource, RuntimeConfigValidationResult,
    RuntimeConfiguredModelCredentialInput, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, RuntimeSecretReferenceStatus,
};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::runtime::RuntimeConfigService;

use super::{build_catalog_snapshot, RuntimeSdkBridge};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeConfigScopeKind {
    Workspace,
    Project,
    User,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeConfigDocumentRecord {
    scope: RuntimeConfigScopeKind,
    owner_id: Option<String>,
    source_key: String,
    display_path: String,
    storage_path: PathBuf,
    exists: bool,
    loaded: bool,
    document: Option<Map<String, Value>>,
    secret_reference_statuses: Vec<RuntimeSecretReferenceStatus>,
}

#[derive(Debug, Clone)]
struct ManagedConfiguredModelCredentialWrite {
    credential_ref: String,
    api_key: String,
    previous_value: Option<String>,
}

fn apply_validation(
    mut effective: RuntimeEffectiveConfig,
    validation: RuntimeConfigValidationResult,
) -> RuntimeEffectiveConfig {
    effective.validation = validation;
    effective
}

impl RuntimeSdkBridge {
    fn hash_value(value: &Value) -> Result<String, AppError> {
        let encoded = serde_json::to_vec(value)?;
        let digest = Sha256::digest(encoded);
        Ok(format!("{digest:x}"))
    }

    fn parse_scope(scope: &str) -> Result<RuntimeConfigScopeKind, AppError> {
        match scope {
            "workspace" => Ok(RuntimeConfigScopeKind::Workspace),
            "project" => Ok(RuntimeConfigScopeKind::Project),
            "user" => Ok(RuntimeConfigScopeKind::User),
            other => Err(AppError::invalid_input(format!(
                "unsupported runtime config scope: {other}"
            ))),
        }
    }

    fn public_scope_label(scope: RuntimeConfigScopeKind) -> &'static str {
        match scope {
            RuntimeConfigScopeKind::Workspace => "workspace",
            RuntimeConfigScopeKind::Project => "project",
            RuntimeConfigScopeKind::User => "user",
        }
    }

    fn scope_precedence(scope: RuntimeConfigScopeKind) -> u8 {
        match scope {
            RuntimeConfigScopeKind::User => 0,
            RuntimeConfigScopeKind::Workspace => 1,
            RuntimeConfigScopeKind::Project => 2,
        }
    }

    fn workspace_config_path(&self) -> PathBuf {
        self.state.paths.runtime_config_dir.join("workspace.json")
    }

    fn project_config_path(&self, project_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_project_config_dir
            .join(format!("{project_id}.json"))
    }

    fn user_config_path(&self, user_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_user_config_dir
            .join(format!("{user_id}.json"))
    }

    fn ensure_runtime_config_layout(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.state.paths.runtime_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_project_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_user_config_dir)?;
        Ok(())
    }

    fn configured_model_secret_reference(&self, configured_model_id: &str) -> String {
        format!(
            "secret-ref:workspace:{}:configured-model:{}",
            self.state.workspace_id,
            BASE64_STANDARD.encode(configured_model_id)
        )
    }

    fn configured_model_secret_status(&self, reference: &str) -> &'static str {
        if let Some(env_key) = reference.strip_prefix("env:") {
            return if std::env::var_os(env_key).is_some() {
                "reference-present"
            } else {
                "reference-missing"
            };
        }
        if reference.starts_with("secret-ref:") {
            return match self.state.secret_vault.get_optional_bytes(reference) {
                Ok(Some(_)) => "reference-present",
                Ok(None) => "reference-missing",
                Err(_) => "reference-error",
            };
        }
        if Self::is_reference_value(reference) {
            return "reference-unsupported";
        }
        "reference-inline"
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
            "credentialref",
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
        &self,
        scope: &str,
        path: &str,
        raw: &str,
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
    ) -> Value {
        if Self::is_reference_value(raw) {
            let status = self.configured_model_secret_status(raw);
            Self::record_secret_reference(
                secret_references,
                scope,
                path,
                Some(raw.to_string()),
                status,
            );
            return Value::String(raw.to_string());
        }

        Self::record_secret_reference(secret_references, scope, path, None, "inline-redacted");
        Value::String("***".to_string())
    }

    fn redact_config_value(
        &self,
        scope: &str,
        path: &str,
        value: &Value,
        secret_references: &mut Vec<RuntimeSecretReferenceStatus>,
    ) -> Value {
        match value {
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, child)| {
                        let child_path = if path.is_empty() {
                            key.clone()
                        } else {
                            format!("{path}.{key}")
                        };
                        let child = match child {
                            Value::String(raw) if Self::is_sensitive_key(key) => {
                                self.redact_secret_value(scope, &child_path, raw, secret_references)
                            }
                            _ => self.redact_config_value(
                                scope,
                                &child_path,
                                child,
                                secret_references,
                            ),
                        };
                        (key.clone(), child)
                    })
                    .collect(),
            ),
            Value::Array(values) => Value::Array(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, child)| {
                        self.redact_config_value(
                            scope,
                            &format!("{path}[{index}]"),
                            child,
                            secret_references,
                        )
                    })
                    .collect(),
            ),
            _ => value.clone(),
        }
    }

    fn read_optional_runtime_document(path: &Path) -> Result<Option<Map<String, Value>>, AppError> {
        match fs::read_to_string(path) {
            Ok(raw) => {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    return Ok(None);
                }
                let parsed: Value = serde_json::from_str(trimmed)
                    .map_err(|error| AppError::runtime(error.to_string()))?;
                parsed.as_object().cloned().map(Some).ok_or_else(|| {
                    AppError::runtime("runtime config document must be a JSON object")
                })
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn write_runtime_document(
        &self,
        path: &Path,
        document: &Map<String, Value>,
    ) -> Result<(), AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            path,
            serde_json::to_vec_pretty(&Value::Object(document.clone()))?,
        )?;
        Ok(())
    }

    fn config_document_record(
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
            secret_reference_statuses: Vec::new(),
        })
    }

    fn resolve_documents(
        &self,
        project_id: Option<&str>,
        user_id: Option<&str>,
    ) -> Result<Vec<RuntimeConfigDocumentRecord>, AppError> {
        self.ensure_runtime_config_layout()?;

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

    fn merge_patch(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
        for (key, value) in patch {
            if value.is_null() {
                target.remove(key);
                continue;
            }
            match (target.get_mut(key), value) {
                (Some(Value::Object(target_map)), Value::Object(patch_map)) => {
                    Self::merge_patch(target_map, patch_map);
                }
                _ => {
                    target.insert(key.clone(), value.clone());
                }
            }
        }
    }

    fn load_effective_config_json(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<Value, AppError> {
        let mut merged = Map::new();
        for document in documents {
            if let Some(record) = &document.document {
                Self::merge_patch(&mut merged, record);
            }
        }
        Ok(Value::Object(merged))
    }

    fn build_effective_config(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let mut secret_references = Vec::new();
        let effective_value = self.load_effective_config_json(documents)?;
        let effective_config =
            self.redact_config_value("effective", "", &effective_value, &mut Vec::new());
        let effective_config_hash = Self::hash_value(&effective_value)?;

        let sources = documents
            .iter()
            .map(|document| {
                for status in &document.secret_reference_statuses {
                    Self::record_secret_reference(
                        &mut secret_references,
                        &status.scope,
                        &status.path,
                        status.reference.clone(),
                        &status.status,
                    );
                }
                let redacted_document = document.document.as_ref().map(|value| {
                    self.redact_config_value(
                        Self::public_scope_label(document.scope),
                        "",
                        &Value::Object(value.clone()),
                        &mut secret_references,
                    )
                });
                let content_hash = document
                    .document
                    .as_ref()
                    .map(|value| Self::hash_value(&Value::Object(value.clone())))
                    .transpose()?;

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

    fn validate_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

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

            if let Some(configured_models) = record.get("configuredModels") {
                let Some(configured_models) = configured_models.as_object() else {
                    errors.push(format!(
                        "{}: configuredModels must be a JSON object",
                        document.display_path
                    ));
                    continue;
                };
                for (configured_model_id, entry) in configured_models {
                    let Some(entry_object) = entry.as_object() else {
                        errors.push(format!(
                            "{}: configuredModels.{configured_model_id} must be a JSON object",
                            document.display_path
                        ));
                        continue;
                    };
                    for field in ["providerId", "modelId", "name"] {
                        if entry_object.get(field).and_then(Value::as_str).is_none() {
                            errors.push(format!(
                                "{}: configuredModels.{configured_model_id}.{field} is required",
                                document.display_path
                            ));
                        }
                    }
                }
            }
        }

        Ok(RuntimeConfigValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    fn validate_registry_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let effective_config = self.load_effective_config_json(documents)?;
        let mut validation = self.validate_documents(documents)?;
        let snapshot = build_catalog_snapshot(self, &effective_config)?;
        validation
            .warnings
            .extend(snapshot.diagnostics.warnings.clone());
        validation
            .errors
            .extend(snapshot.diagnostics.errors.clone());
        validation.valid = validation.errors.is_empty();
        Ok(validation)
    }

    fn patched_documents(
        &self,
        scope: RuntimeConfigScopeKind,
        project_id: Option<&str>,
        user_id: Option<&str>,
        patch: &Value,
    ) -> Result<Vec<RuntimeConfigDocumentRecord>, AppError> {
        let patch_object = patch
            .as_object()
            .ok_or_else(|| AppError::invalid_input("runtime config patch must be a JSON object"))?;

        let mut documents = self.resolve_documents(project_id, user_id)?;
        let target_document = documents
            .iter_mut()
            .find(|document| document.scope == scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        let mut next = target_document.document.clone().unwrap_or_default();
        Self::merge_patch(&mut next, patch_object);
        target_document.exists = true;
        target_document.loaded = true;
        target_document.document = Some(next);

        Ok(documents)
    }

    fn write_document(&self, document: &RuntimeConfigDocumentRecord) -> Result<(), AppError> {
        self.write_runtime_document(
            &document.storage_path,
            &document.document.clone().unwrap_or_default(),
        )
    }

    fn workspace_target_document_mut(
        documents: &mut [RuntimeConfigDocumentRecord],
    ) -> Result<&mut RuntimeConfigDocumentRecord, AppError> {
        documents
            .iter_mut()
            .find(|document| document.scope == RuntimeConfigScopeKind::Workspace)
            .ok_or_else(|| AppError::not_found("workspace runtime config document"))
    }

    fn workspace_target_document(
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<&RuntimeConfigDocumentRecord, AppError> {
        documents
            .iter()
            .find(|document| document.scope == RuntimeConfigScopeKind::Workspace)
            .ok_or_else(|| AppError::not_found("workspace runtime config document"))
    }

    fn ensure_workspace_managed_credentials_supported(
        target_scope: RuntimeConfigScopeKind,
        configured_model_credentials: &[RuntimeConfiguredModelCredentialInput],
    ) -> Result<(), AppError> {
        if target_scope == RuntimeConfigScopeKind::Workspace
            || configured_model_credentials.is_empty()
        {
            return Ok(());
        }
        Err(AppError::invalid_input(
            "configured model credentials are only supported for workspace runtime config",
        ))
    }

    fn apply_workspace_managed_credentials(
        &self,
        documents: &mut [RuntimeConfigDocumentRecord],
        configured_model_credentials: &[RuntimeConfiguredModelCredentialInput],
    ) -> Result<Vec<ManagedConfiguredModelCredentialWrite>, AppError> {
        if configured_model_credentials.is_empty() {
            return Ok(Vec::new());
        }

        let target = Self::workspace_target_document_mut(documents)?;
        let document = target.document.get_or_insert_with(Map::new);
        let Some(Value::Object(configured_models)) = document.get_mut("configuredModels") else {
            return Err(AppError::invalid_input(
                "configured model credentials require configuredModels entries in the workspace patch",
            ));
        };

        let mut seen = HashSet::new();
        let mut writes = Vec::with_capacity(configured_model_credentials.len());

        for input in configured_model_credentials {
            let configured_model_id = input.configured_model_id.trim();
            if configured_model_id.is_empty() {
                return Err(AppError::invalid_input(
                    "configured model credential input requires configuredModelId",
                ));
            }
            if !seen.insert(configured_model_id.to_string()) {
                return Err(AppError::invalid_input(format!(
                    "duplicate configured model credential input `{configured_model_id}`"
                )));
            }

            let api_key = input.api_key.trim();
            if api_key.is_empty() {
                return Err(AppError::invalid_input(
                    "configured model credential input requires apiKey",
                ));
            }

            let Some(Value::Object(configured_model)) =
                configured_models.get_mut(configured_model_id)
            else {
                return Err(AppError::invalid_input(format!(
                    "configured model credential target `{configured_model_id}` is missing from the workspace patch"
                )));
            };

            let credential_ref = self.configured_model_secret_reference(configured_model_id);
            configured_model.insert(
                "credentialRef".to_string(),
                Value::String(credential_ref.clone()),
            );

            writes.push(ManagedConfiguredModelCredentialWrite {
                previous_value: self.state.secret_vault.get_optional_utf8(&credential_ref)?,
                credential_ref,
                api_key: api_key.to_string(),
            });
        }

        Ok(writes)
    }

    fn rollback_managed_credential_writes(
        &self,
        writes: &[ManagedConfiguredModelCredentialWrite],
    ) -> Result<(), AppError> {
        for write in writes.iter().rev() {
            if let Some(previous_value) = write.previous_value.as_deref() {
                self.state
                    .secret_vault
                    .put_utf8(&write.credential_ref, previous_value)?;
            } else {
                self.state.secret_vault.delete(&write.credential_ref)?;
            }
        }
        Ok(())
    }

    fn persist_managed_credential_writes(
        &self,
        writes: &[ManagedConfiguredModelCredentialWrite],
    ) -> Result<(), AppError> {
        for write in writes {
            self.state
                .secret_vault
                .put_utf8(&write.credential_ref, &write.api_key)?;
            let stored_secret = self
                .state
                .secret_vault
                .get_optional_utf8(&write.credential_ref)?;
            match stored_secret.as_deref() {
                Some(value) if value == write.api_key => {}
                Some(_) => {
                    return Err(AppError::runtime(format!(
                        "managed credential `{}` could not be verified after saving to local encrypted secret store",
                        write.credential_ref
                    )));
                }
                None => {
                    return Err(AppError::runtime(format!(
                        "managed credential `{}` is missing from local encrypted secret store after saving",
                        write.credential_ref
                    )));
                }
            }
        }
        Ok(())
    }

    fn workspace_owned_managed_credential_refs(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<HashSet<String>, AppError> {
        let mut refs = HashSet::new();
        let target = match Self::workspace_target_document(documents) {
            Ok(target) => target,
            Err(_) => return Ok(refs),
        };
        let Some(document) = target.document.as_ref() else {
            return Ok(refs);
        };
        let Some(configured_models) = document.get("configuredModels").and_then(Value::as_object)
        else {
            return Ok(refs);
        };

        for (entry_key, entry) in configured_models {
            let Some(entry_object) = entry.as_object() else {
                continue;
            };
            let configured_model_id = entry_object
                .get("configuredModelId")
                .and_then(Value::as_str)
                .unwrap_or(entry_key);
            let Some(reference) = entry_object
                .get("credentialRef")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                continue;
            };

            if reference == self.configured_model_secret_reference(configured_model_id) {
                refs.insert(reference.to_string());
            }
        }

        Ok(refs)
    }

    fn cleanup_orphaned_workspace_managed_credentials(
        &self,
        previous_refs: &HashSet<String>,
        documents: &[RuntimeConfigDocumentRecord],
    ) -> Result<(), AppError> {
        let retained_refs = self.workspace_owned_managed_credential_refs(documents)?;
        for orphaned_ref in previous_refs.difference(&retained_refs) {
            self.state.secret_vault.delete(orphaned_ref)?;
        }
        Ok(())
    }

    fn restore_document_state(
        &self,
        previous: &RuntimeConfigDocumentRecord,
    ) -> Result<(), AppError> {
        if let Some(document) = previous.document.as_ref() {
            return self.write_runtime_document(&previous.storage_path, document);
        }

        match fs::remove_file(&previous.storage_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    async fn probe_configured_model_documents(
        &self,
        documents: &[RuntimeConfigDocumentRecord],
        configured_model_id: &str,
        _api_key: Option<&str>,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError> {
        let validation = self.validate_registry_documents(documents)?;
        if !validation.valid {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: false,
                reachable: false,
                configured_model_id: configured_model_id.to_string(),
                configured_model_name: None,
                request_id: None,
                consumed_tokens: None,
                errors: validation.errors,
                warnings: validation.warnings,
            });
        }

        let effective_config = self.load_effective_config_json(documents)?;
        let snapshot = build_catalog_snapshot(self, &effective_config)?;
        let configured_model = snapshot
            .configured_models
            .iter()
            .find(|record| record.configured_model_id == configured_model_id)
            .cloned();

        let Some(configured_model) = configured_model else {
            return Ok(RuntimeConfiguredModelProbeResult {
                valid: false,
                reachable: false,
                configured_model_id: configured_model_id.to_string(),
                configured_model_name: None,
                request_id: None,
                consumed_tokens: None,
                errors: vec![format!(
                    "configured model `{configured_model_id}` is not registered"
                )],
                warnings: validation.warnings,
            });
        };

        let reachable = configured_model.enabled
            && configured_model.status == "configured"
            && snapshot
                .models
                .iter()
                .any(|model| model.model_id == configured_model.model_id);

        let errors = if reachable {
            Vec::new()
        } else {
            vec![format!(
                "configured model `{configured_model_id}` is not ready for probe"
            )]
        };

        Ok(RuntimeConfiguredModelProbeResult {
            valid: true,
            reachable,
            configured_model_id: configured_model_id.to_string(),
            configured_model_name: Some(configured_model.name),
            request_id: reachable.then(|| "runtime-sdk-probe".to_string()),
            consumed_tokens: reachable.then_some(0),
            errors,
            warnings: validation.warnings,
        })
    }
}

#[async_trait]
impl RuntimeConfigService for RuntimeSdkBridge {
    async fn get_config(&self) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(None, None)?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(
            effective,
            self.validate_registry_documents(&documents)?,
        ))
    }

    async fn get_project_config(
        &self,
        project_id: &str,
        user_id: &str,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(Some(project_id), Some(user_id))?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(
            effective,
            self.validate_registry_documents(&documents)?,
        ))
    }

    async fn get_user_config(&self, user_id: &str) -> Result<RuntimeEffectiveConfig, AppError> {
        let documents = self.resolve_documents(None, Some(user_id))?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(
            effective,
            self.validate_registry_documents(&documents)?,
        ))
    }

    async fn validate_config(
        &self,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let mut documents = self.patched_documents(target_scope, None, None, &patch.patch)?;
        if target_scope == RuntimeConfigScopeKind::Workspace {
            self.apply_workspace_managed_credentials(
                &mut documents,
                &patch.configured_model_credentials,
            )?;
        }
        self.validate_registry_documents(&documents)
    }

    async fn validate_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents =
            self.patched_documents(target_scope, Some(project_id), Some(user_id), &patch.patch)?;
        self.validate_registry_documents(&documents)
    }

    async fn validate_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeConfigValidationResult, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents = self.patched_documents(target_scope, None, Some(user_id), &patch.patch)?;
        self.validate_registry_documents(&documents)
    }

    async fn probe_configured_model(
        &self,
        input: RuntimeConfiguredModelProbeInput,
    ) -> Result<RuntimeConfiguredModelProbeResult, AppError> {
        let target_scope = Self::parse_scope(&input.scope)?;
        let documents = self.patched_documents(target_scope, None, None, &input.patch)?;
        self.probe_configured_model_documents(
            &documents,
            &input.configured_model_id,
            input.api_key.as_deref(),
        )
        .await
    }

    async fn save_config(
        &self,
        scope: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        if patch.scope != scope {
            return Err(AppError::invalid_input(
                "runtime config patch scope must match route scope",
            ));
        }
        let target_scope = Self::parse_scope(scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let existing_documents = self.resolve_documents(None, None)?;
        let previous_target = existing_documents
            .iter()
            .find(|document| document.scope == target_scope)
            .cloned()
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        let previous_managed_refs =
            self.workspace_owned_managed_credential_refs(&existing_documents)?;

        let mut documents = self.patched_documents(target_scope, None, None, &patch.patch)?;
        let writes = if target_scope == RuntimeConfigScopeKind::Workspace {
            self.apply_workspace_managed_credentials(
                &mut documents,
                &patch.configured_model_credentials,
            )?
        } else {
            Vec::new()
        };
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }

        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        if let Err(error) = self.persist_managed_credential_writes(&writes) {
            self.rollback_managed_credential_writes(&writes)?;
            return Err(error);
        }
        if let Err(error) = self.write_document(target) {
            self.rollback_managed_credential_writes(&writes)?;
            self.restore_document_state(&previous_target)?;
            return Err(error);
        }
        if let Err(error) =
            self.cleanup_orphaned_workspace_managed_credentials(&previous_managed_refs, &documents)
        {
            self.restore_document_state(&previous_target)?;
            return Err(error);
        }

        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(effective, validation))
    }

    async fn save_project_config(
        &self,
        project_id: &str,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents =
            self.patched_documents(target_scope, Some(project_id), Some(user_id), &patch.patch)?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }
        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        self.write_document(target)?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(effective, validation))
    }

    async fn save_user_config(
        &self,
        user_id: &str,
        patch: RuntimeConfigPatch,
    ) -> Result<RuntimeEffectiveConfig, AppError> {
        let target_scope = Self::parse_scope(&patch.scope)?;
        Self::ensure_workspace_managed_credentials_supported(
            target_scope,
            &patch.configured_model_credentials,
        )?;
        let documents = self.patched_documents(target_scope, None, Some(user_id), &patch.patch)?;
        let validation = self.validate_registry_documents(&documents)?;
        if !validation.valid {
            return Err(AppError::invalid_input(validation.errors.join("; ")));
        }
        let target = documents
            .iter()
            .find(|document| document.scope == target_scope)
            .ok_or_else(|| AppError::not_found("runtime config document"))?;
        self.write_document(target)?;
        let effective = self.build_effective_config(&documents)?;
        Ok(apply_validation(effective, validation))
    }
}
