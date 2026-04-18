use super::*;
use crate::model_runtime::{
    parse_model_credential_reference, validate_runtime_credential_reference, CredentialReference,
};

pub(super) fn validate_configured_models(
    providers: &BTreeMap<String, ProviderRegistryRecord>,
    models: &BTreeMap<String, ModelRegistryRecord>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
    diagnostics: &mut ModelRegistryDiagnostics,
) {
    let mut names = HashMap::<String, String>::new();

    for configured_model in configured_models.values() {
        let trimmed_name = configured_model.name.trim();
        if trimmed_name.is_empty() {
            diagnostics.errors.push(format!(
                "configured model `{}` is missing a display name",
                configured_model.configured_model_id
            ));
        } else if let Some(existing) = names.insert(
            trimmed_name.to_lowercase(),
            configured_model.configured_model_id.clone(),
        ) {
            diagnostics.errors.push(format!(
                "configured model name `{trimmed_name}` is duplicated by `{existing}` and `{}`",
                configured_model.configured_model_id
            ));
        }

        let Some(provider) = providers.get(&configured_model.provider_id) else {
            diagnostics.errors.push(format!(
                "configured model `{}` references unknown provider `{}`",
                configured_model.configured_model_id, configured_model.provider_id
            ));
            continue;
        };
        let Some(model) = models.get(&configured_model.model_id) else {
            diagnostics.errors.push(format!(
                "configured model `{}` references unknown model `{}`",
                configured_model.configured_model_id, configured_model.model_id
            ));
            continue;
        };
        if model.provider_id != provider.provider_id {
            diagnostics.errors.push(format!(
                "configured model `{}` model `{}` does not belong to provider `{}`",
                configured_model.configured_model_id,
                configured_model.model_id,
                configured_model.provider_id
            ));
        }

        let provider_type = provider
            .metadata
            .get("providerType")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let provider_managed_by_page = provider
            .metadata
            .get(WORKSPACE_MODELS_PAGE_METADATA_KEY)
            .and_then(Value::as_str)
            == Some(WORKSPACE_MODELS_PAGE_METADATA_VALUE);
        if provider_type == CUSTOM_PROVIDER_TYPE && provider_managed_by_page {
            let provider_surface_base_url = provider
                .surfaces
                .iter()
                .find(|surface| surface.enabled && surface.surface == "conversation")
                .map(|surface| surface.base_url.as_str())
                .or_else(|| {
                    provider
                        .surfaces
                        .iter()
                        .find(|surface| surface.enabled)
                        .map(|surface| surface.base_url.as_str())
                });
            let configured_base_url = configured_model.base_url.as_deref();
            if configured_base_url == Some(CUSTOM_BASE_URL_PLACEHOLDER)
                || provider_surface_base_url == Some(CUSTOM_BASE_URL_PLACEHOLDER)
            {
                diagnostics.warnings.push(format!(
                    "configured model `{}` still uses the custom provider placeholder base URL `{}`",
                    configured_model.configured_model_id, CUSTOM_BASE_URL_PLACEHOLDER
                ));
            }
        }
    }
}

pub(super) fn build_configured_models(
    providers: &BTreeMap<String, ProviderRegistryRecord>,
    models: &BTreeMap<String, ModelRegistryRecord>,
    credential_bindings: &BTreeMap<String, CredentialBinding>,
    configured_models_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Result<BTreeMap<String, ConfiguredModelRecord>, AppError> {
    let Some(object) = configured_models_value.and_then(Value::as_object) else {
        return Ok(BTreeMap::new());
    };

    let mut configured_models = BTreeMap::new();
    for (key, value) in object {
        let configured_model_id = value
            .get("configuredModelId")
            .and_then(Value::as_str)
            .unwrap_or(key)
            .to_string();
        let model_id = value
            .get("modelId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let provider_id = value
            .get("providerId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| models.get(&model_id).map(|model| model.provider_id.clone()))
            .unwrap_or_default();
        let credential_ref = match value.get("credentialRef").and_then(Value::as_str) {
            Some(reference) => match parse_model_credential_reference(Some(reference))? {
                Some(CredentialReference::Env(env_key)) => Some(format!("env:{env_key}")),
                Some(CredentialReference::ManagedSecret(reference)) => Some(reference.to_string()),
                Some(CredentialReference::Inline(value)) => Some(value.to_string()),
                None => None,
            },
            None => None,
        };
        let configured = credential_ref
            .as_deref()
            .map(reference_present)
            .transpose()?
            .unwrap_or(false);
        let status = if credential_ref.is_some() {
            if configured {
                "configured"
            } else {
                "error"
            }
        } else if credential_bindings.contains_key(&provider_id) {
            "configured"
        } else {
            "unconfigured"
        };
        let name = value
            .get("name")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .or_else(|| models.get(&model_id).map(|model| model.label.clone()))
            .unwrap_or_else(|| configured_model_id.clone());
        let token_quota =
            parse_token_quota(value.get("tokenQuota"), &configured_model_id, diagnostics);

        let _ = providers;
        configured_models.insert(
            configured_model_id.clone(),
            ConfiguredModelRecord {
                configured_model_id,
                name,
                provider_id,
                model_id,
                credential_ref,
                base_url: value
                    .get("baseUrl")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                token_quota: token_quota.clone(),
                token_usage: token_usage_summary(token_quota.as_ref(), 0),
                enabled: value
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                source: value
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .to_string(),
                status: status.to_string(),
                configured,
            },
        );
    }

    Ok(configured_models)
}

pub(super) fn build_seeded_configured_models(
    models: &BTreeMap<String, ModelRegistryRecord>,
    credential_bindings: &BTreeMap<String, CredentialBinding>,
) -> BTreeMap<String, ConfiguredModelRecord> {
    let mut configured_models = BTreeMap::new();

    for model in models.values() {
        let binding = credential_bindings.get(&model.provider_id);
        configured_models.insert(
            model.model_id.clone(),
            ConfiguredModelRecord {
                configured_model_id: model.model_id.clone(),
                name: model.label.clone(),
                provider_id: model.provider_id.clone(),
                model_id: model.model_id.clone(),
                credential_ref: binding.map(|entry| entry.credential_ref.clone()),
                base_url: binding.and_then(|entry| entry.base_url.clone()),
                token_quota: None,
                token_usage: token_usage_summary(None, 0),
                enabled: model.enabled,
                source: "seeded".into(),
                status: binding
                    .map(|entry| entry.status.clone())
                    .unwrap_or_else(|| "missing".into()),
                configured: binding.map(|entry| entry.configured).unwrap_or(false),
            },
        );
    }

    configured_models
}

pub(super) fn parse_token_quota(
    token_quota_value: Option<&Value>,
    configured_model_id: &str,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<ConfiguredModelTokenQuota> {
    let Some(token_quota_value) = token_quota_value else {
        return None;
    };
    let Some(token_quota_object) = token_quota_value.as_object() else {
        diagnostics.errors.push(format!(
            "configured model `{configured_model_id}` tokenQuota must be an object"
        ));
        return None;
    };

    let total_tokens = match token_quota_object.get("totalTokens") {
        None | Some(Value::Null) => None,
        Some(Value::Number(number)) => {
            let Some(value) = number.as_u64() else {
                diagnostics.errors.push(format!(
                    "configured model `{configured_model_id}` tokenQuota.totalTokens must be a positive integer"
                ));
                return None;
            };
            if value == 0 {
                diagnostics.errors.push(format!(
                    "configured model `{configured_model_id}` tokenQuota.totalTokens must be greater than zero"
                ));
                return None;
            }
            Some(value)
        }
        Some(_) => {
            diagnostics.errors.push(format!(
                "configured model `{configured_model_id}` tokenQuota.totalTokens must be a positive integer"
            ));
            return None;
        }
    };

    Some(ConfiguredModelTokenQuota { total_tokens })
}

pub(super) fn build_credential_bindings(
    providers: &BTreeMap<String, ProviderRegistryRecord>,
    configured_refs: Option<&Value>,
) -> Result<BTreeMap<String, CredentialBinding>, AppError> {
    let configured_object = configured_refs.and_then(Value::as_object);
    let mut bindings = BTreeMap::new();

    for provider in providers.values() {
        let env_name = default_credential_env(&provider.provider_id);
        let configured_value =
            configured_object.and_then(|entries| entries.get(&provider.provider_id));

        let mut credential_ref = env_name.map(|value| format!("env:{value}"));
        let mut label = format!("{} Primary", provider.label);
        let mut base_url = None;
        let mut source = "baseline".to_string();
        let mut status = "unconfigured".to_string();

        if let Some(value) = configured_value {
            source = "runtime_config".into();
            if let Some(reference) = value.as_str() {
                credential_ref = validate_runtime_credential_reference(
                    reference,
                    &format!("provider credential binding `{}`", provider.provider_id),
                )?;
            } else if let Some(object) = value.as_object() {
                if let Some(reference) = object
                    .get("credentialRef")
                    .or_else(|| object.get("reference"))
                    .and_then(Value::as_str)
                {
                    credential_ref = validate_runtime_credential_reference(
                        reference,
                        &format!("provider credential binding `{}`", provider.provider_id),
                    )?;
                }
                if let Some(configured_label) = object.get("label").and_then(Value::as_str) {
                    label = configured_label.to_string();
                }
                if let Some(configured_base_url) = object.get("baseUrl").and_then(Value::as_str) {
                    base_url = Some(configured_base_url.to_string());
                }
                if let Some(configured_status) = object.get("status").and_then(Value::as_str) {
                    status = configured_status.to_string();
                }
            }
        }

        let configured = credential_ref
            .as_deref()
            .map(reference_present)
            .transpose()?
            .unwrap_or(false);

        if status == "unconfigured" {
            status = if configured {
                "configured".into()
            } else {
                "unconfigured".into()
            };
        }

        bindings.insert(
            provider.provider_id.clone(),
            CredentialBinding {
                credential_ref: credential_ref
                    .unwrap_or_else(|| format!("env:{}", env_name.unwrap_or("MISSING_API_KEY"))),
                provider_id: provider.provider_id.clone(),
                label,
                base_url,
                status,
                configured,
                source,
            },
        );
    }

    Ok(bindings)
}
