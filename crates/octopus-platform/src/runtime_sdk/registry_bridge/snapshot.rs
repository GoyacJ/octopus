use std::collections::{BTreeMap, BTreeSet, HashMap};

use octopus_core::{
    AppError, ConfiguredModelRecord, CredentialBinding, ModelCatalogSnapshot,
    ModelRegistryDiagnostics,
};
use octopus_persistence::Database;
use serde_json::Value;

use crate::runtime_sdk::RuntimeSdkBridge;

use super::builtins::{
    builtin_model, builtin_provider, canonical_model_id, configured_model_status,
    hidden_builtin_model, token_usage_summary, CANONICAL_DEFAULTS,
};
use super::overrides::{
    build_default_selections, parse_budget_policy, parse_model_overrides, parse_provider_overrides,
};

pub(crate) fn load_configured_model_usage_map(
    bridge: &RuntimeSdkBridge,
) -> Result<HashMap<String, u64>, AppError> {
    let connection = Database::open(bridge.state.paths.db_path.clone())?.acquire()?;
    let mut statement = match connection.prepare(
        "SELECT configured_model_id, settled_tokens
         FROM configured_model_budget_projections",
    ) {
        Ok(statement) => statement,
        Err(error) if error.to_string().contains("no such table") => return Ok(HashMap::new()),
        Err(error) => return Err(AppError::database(error.to_string())),
    };
    let rows = statement
        .query_map([], |row| {
            let configured_model_id: String = row.get(0)?;
            let settled_tokens: i64 = row.get(1)?;
            Ok((configured_model_id, settled_tokens))
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    let mut usage = HashMap::new();
    for row in rows {
        let (configured_model_id, settled_tokens) =
            row.map_err(|error| AppError::database(error.to_string()))?;
        usage.insert(configured_model_id, settled_tokens.max(0) as u64);
    }
    Ok(usage)
}

pub(crate) fn build_catalog_snapshot(
    bridge: &RuntimeSdkBridge,
    effective_config: &Value,
) -> Result<ModelCatalogSnapshot, AppError> {
    let mut providers = BTreeMap::new();
    let mut models = BTreeMap::new();
    for provider_id in ["anthropic", "openai", "google", "minimax", "xai", "custom"] {
        providers.insert(provider_id.to_string(), builtin_provider(provider_id));
    }
    for (model_id, provider_id) in [
        ("claude-sonnet-4-5", "anthropic"),
        ("claude-opus-4-6", "anthropic"),
        ("claude-haiku-4-5-20251213", "anthropic"),
        ("gpt-4o", "openai"),
        ("grok-3", "xai"),
    ] {
        models.insert(model_id.to_string(), builtin_model(model_id, provider_id));
    }

    parse_provider_overrides(effective_config, &mut providers);
    parse_model_overrides(effective_config, &mut models);

    let usage = load_configured_model_usage_map(bridge)?;
    let mut diagnostics = ModelRegistryDiagnostics {
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    let configured_models_value = effective_config
        .get("configuredModels")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut configured_models = Vec::new();
    let mut credential_bindings = Vec::new();
    let mut configured_model_ids = BTreeSet::new();

    if configured_models_value.is_empty() {
        for (_purpose, provider_id, model_id, _) in CANONICAL_DEFAULTS {
            let record = ConfiguredModelRecord {
                configured_model_id: (*model_id).to_string(),
                name: builtin_model(model_id, provider_id).label,
                provider_id: (*provider_id).to_string(),
                model_id: (*model_id).to_string(),
                credential_ref: None,
                base_url: None,
                budget_policy: None,
                token_usage: token_usage_summary(None, 0),
                enabled: true,
                source: "builtin".into(),
                status: "unconfigured".into(),
                configured: false,
            };
            if configured_model_ids.insert(record.configured_model_id.clone()) {
                configured_models.push(record);
            }
        }
    } else {
        for (configured_model_id, entry) in configured_models_value {
            let Some(object) = entry.as_object() else {
                diagnostics.errors.push(format!(
                    "configured model `{configured_model_id}` must be a JSON object"
                ));
                continue;
            };
            let provider_id = object
                .get("providerId")
                .and_then(Value::as_str)
                .unwrap_or("custom");
            let model_id = object
                .get("modelId")
                .and_then(Value::as_str)
                .unwrap_or(configured_model_id.as_str());
            let canonical_id = canonical_model_id(model_id);
            let final_model_id = if models.contains_key(&canonical_id) {
                canonical_id
            } else {
                model_id.to_string()
            };
            let hidden_model = hidden_builtin_model(&final_model_id, provider_id);

            providers
                .entry(provider_id.to_string())
                .or_insert_with(|| builtin_provider(provider_id));
            match hidden_model {
                Some(ref model) => {
                    diagnostics.warnings.push(format!(
                        "configured model `{configured_model_id}` targets non-live builtin `{final_model_id}`; keeping it config-visible but runtime-unsupported"
                    ));
                    models
                        .entry(final_model_id.clone())
                        .or_insert_with(|| model.clone());
                }
                None => {
                    models
                        .entry(final_model_id.clone())
                        .or_insert_with(|| builtin_model(&final_model_id, provider_id));
                }
            }

            let credential_ref = object
                .get("credentialRef")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);
            if let Some(reference) = credential_ref.clone() {
                let provider_label = providers
                    .get(provider_id)
                    .map(|provider| provider.label.clone())
                    .unwrap_or_else(|| provider_id.to_string());
                credential_bindings.push(CredentialBinding {
                    credential_ref: reference.clone(),
                    provider_id: provider_id.to_string(),
                    label: format!("{provider_label} credential"),
                    base_url: object
                        .get("baseUrl")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    status: configured_model_status(Some(reference.as_str()), true, bridge),
                    configured: true,
                    source: object
                        .get("source")
                        .and_then(Value::as_str)
                        .unwrap_or("workspace")
                        .to_string(),
                });
            }

            configured_model_ids.insert(configured_model_id.clone());
            configured_models.push(ConfiguredModelRecord {
                configured_model_id: configured_model_id.clone(),
                name: object
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or(configured_model_id.as_str())
                    .to_string(),
                provider_id: provider_id.to_string(),
                model_id: final_model_id.clone(),
                credential_ref: credential_ref.clone(),
                base_url: object
                    .get("baseUrl")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                budget_policy: parse_budget_policy(object.get("budgetPolicy")),
                token_usage: token_usage_summary(
                    parse_budget_policy(object.get("budgetPolicy")).as_ref(),
                    usage.get(&configured_model_id).copied().unwrap_or(0),
                ),
                enabled: object
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                source: object
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .to_string(),
                status: if hidden_model.is_some() {
                    "unsupported".into()
                } else {
                    configured_model_status(
                        credential_ref.as_deref(),
                        object
                            .get("enabled")
                            .and_then(Value::as_bool)
                            .unwrap_or(true),
                        bridge,
                    )
                },
                configured: credential_ref.is_some(),
            });
        }
    }

    let default_selections = build_default_selections(effective_config, &configured_model_ids);

    Ok(ModelCatalogSnapshot {
        providers: providers.into_values().collect(),
        models: models.into_values().collect(),
        configured_models,
        credential_bindings,
        default_selections,
        diagnostics,
    })
}
