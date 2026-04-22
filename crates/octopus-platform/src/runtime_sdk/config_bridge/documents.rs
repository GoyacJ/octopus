use std::{
    fs,
    path::{Path, PathBuf},
};

use octopus_core::{
    AppError, RuntimeConfigSource, RuntimeConfigValidationResult, RuntimeEffectiveConfig,
    RuntimeSecretReferenceStatus,
};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::runtime_sdk::RuntimeSdkBridge;

use super::{RuntimeConfigDocumentRecord, RuntimeConfigScopeKind};

impl RuntimeSdkBridge {
    pub(crate) fn hash_value(value: &Value) -> Result<String, AppError> {
        let encoded = serde_json::to_vec(value)?;
        let digest = Sha256::digest(encoded);
        Ok(format!("{digest:x}"))
    }

    pub(crate) fn parse_scope(scope: &str) -> Result<RuntimeConfigScopeKind, AppError> {
        match scope {
            "workspace" => Ok(RuntimeConfigScopeKind::Workspace),
            "project" => Ok(RuntimeConfigScopeKind::Project),
            "user" => Ok(RuntimeConfigScopeKind::User),
            other => Err(AppError::invalid_input(format!(
                "unsupported runtime config scope: {other}"
            ))),
        }
    }

    pub(crate) fn public_scope_label(scope: RuntimeConfigScopeKind) -> &'static str {
        match scope {
            RuntimeConfigScopeKind::Workspace => "workspace",
            RuntimeConfigScopeKind::Project => "project",
            RuntimeConfigScopeKind::User => "user",
        }
    }

    pub(crate) fn scope_precedence(scope: RuntimeConfigScopeKind) -> u8 {
        match scope {
            RuntimeConfigScopeKind::User => 0,
            RuntimeConfigScopeKind::Workspace => 1,
            RuntimeConfigScopeKind::Project => 2,
        }
    }

    pub(crate) fn workspace_config_path(&self) -> PathBuf {
        self.state.paths.runtime_config_dir.join("workspace.json")
    }

    pub(crate) fn project_config_path(&self, project_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_project_config_dir
            .join(format!("{project_id}.json"))
    }

    pub(crate) fn user_config_path(&self, user_id: &str) -> PathBuf {
        self.state
            .paths
            .runtime_user_config_dir
            .join(format!("{user_id}.json"))
    }

    pub(crate) fn ensure_runtime_config_layout(&self) -> Result<(), AppError> {
        fs::create_dir_all(&self.state.paths.runtime_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_project_config_dir)?;
        fs::create_dir_all(&self.state.paths.runtime_user_config_dir)?;
        Ok(())
    }

    pub(crate) fn redact_secret_value(
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

    pub(crate) fn redact_config_value(
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

    pub(crate) fn read_optional_runtime_document(
        path: &Path,
    ) -> Result<Option<Map<String, Value>>, AppError> {
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

    pub(crate) fn write_runtime_document(
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

    pub(crate) fn config_document_record(
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

    pub(crate) fn resolve_documents(
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

    pub(crate) fn merge_patch(target: &mut Map<String, Value>, patch: &Map<String, Value>) {
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

    pub(crate) fn load_effective_config_json(
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

    pub(crate) fn build_effective_config(
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

    pub(crate) fn write_document(
        &self,
        document: &RuntimeConfigDocumentRecord,
    ) -> Result<(), AppError> {
        self.write_runtime_document(
            &document.storage_path,
            &document.document.clone().unwrap_or_default(),
        )
    }
}
