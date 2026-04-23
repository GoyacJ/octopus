use std::collections::{BTreeMap, BTreeSet};

use octopus_core::{
    ConfiguredModelBudgetPolicy, DefaultSelection, ModelRegistryRecord, ProviderRegistryRecord,
};
use serde_json::{Map, Value};

use super::builtins::{
    builtin_provider, canonical_model_id, hidden_builtin_model, infer_surface_bindings,
    provider_surface, CANONICAL_DEFAULTS,
};

pub(crate) fn parse_provider_overrides(
    effective_config: &Value,
    providers: &mut BTreeMap<String, ProviderRegistryRecord>,
) {
    for path in [
        effective_config.get("providerOverrides"),
        effective_config
            .get("modelRegistry")
            .and_then(|registry| registry.get("providers")),
    ] {
        let Some(entries) = path.and_then(Value::as_object) else {
            continue;
        };
        for (provider_id, entry) in entries {
            let Some(object) = entry.as_object() else {
                continue;
            };
            let label = object
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or(provider_id);
            let surfaces = object
                .get("surfaces")
                .and_then(Value::as_array)
                .map(|surfaces| {
                    surfaces
                        .iter()
                        .filter_map(|surface| {
                            let object = surface.as_object()?;
                            Some(provider_surface(
                                object
                                    .get("surface")
                                    .and_then(Value::as_str)
                                    .unwrap_or("conversation"),
                                object
                                    .get("protocolFamily")
                                    .and_then(Value::as_str)
                                    .unwrap_or("openai_chat"),
                                object
                                    .get("baseUrl")
                                    .and_then(Value::as_str)
                                    .unwrap_or("https://api.example.com/v1"),
                            ))
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|surfaces| !surfaces.is_empty())
                .unwrap_or_else(|| builtin_provider(provider_id).surfaces);

            providers.insert(
                provider_id.clone(),
                ProviderRegistryRecord {
                    provider_id: provider_id.clone(),
                    label: label.into(),
                    enabled: object
                        .get("enabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(true),
                    surfaces,
                    metadata: Value::Object(Map::new()),
                },
            );
        }
    }
}

pub(crate) fn parse_model_overrides(
    effective_config: &Value,
    models: &mut BTreeMap<String, ModelRegistryRecord>,
) {
    let Some(entries) = effective_config
        .get("modelRegistry")
        .and_then(|registry| registry.get("models"))
        .and_then(Value::as_object)
    else {
        return;
    };

    for (model_id, entry) in entries {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let provider_id = object
            .get("providerId")
            .and_then(Value::as_str)
            .unwrap_or("custom");
        let label = object
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or(model_id);
        let description = object
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("Workspace-defined model.");
        models.insert(
            model_id.clone(),
            ModelRegistryRecord {
                label: label.into(),
                description: description.into(),
                surface_bindings: infer_surface_bindings(
                    provider_id,
                    model_id,
                    object.get("surfaceBindings"),
                ),
                provider_id: provider_id.into(),
                model_id: model_id.clone(),
                family: object
                    .get("family")
                    .and_then(Value::as_str)
                    .unwrap_or(provider_id)
                    .into(),
                track: object
                    .get("track")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .into(),
                enabled: object
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
                recommended_for: object
                    .get("recommendedFor")
                    .and_then(Value::as_str)
                    .unwrap_or("general")
                    .into(),
                availability: object
                    .get("availability")
                    .and_then(Value::as_str)
                    .unwrap_or("workspace")
                    .into(),
                default_permission: object
                    .get("defaultPermission")
                    .and_then(Value::as_str)
                    .unwrap_or("default")
                    .into(),
                capabilities: Vec::new(),
                context_window: object
                    .get("contextWindow")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                max_output_tokens: object
                    .get("maxOutputTokens")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                metadata: Value::Object(Map::new()),
            },
        );
    }
}

pub(crate) fn parse_budget_policy(value: Option<&Value>) -> Option<ConfiguredModelBudgetPolicy> {
    serde_json::from_value(value?.clone()).ok()
}

pub(crate) fn build_default_selections(
    effective_config: &Value,
    configured_model_ids: &BTreeSet<String>,
) -> BTreeMap<String, DefaultSelection> {
    let mut selections = BTreeMap::new();
    for (purpose, provider_id, model_id, surface) in CANONICAL_DEFAULTS {
        let configured_model_id = configured_model_ids
            .get(&canonical_model_id(model_id))
            .cloned()
            .or_else(|| configured_model_ids.get(*model_id).cloned())
            .or_else(|| Some((*model_id).to_string()));
        selections.insert(
            (*purpose).to_string(),
            DefaultSelection {
                configured_model_id,
                provider_id: (*provider_id).to_string(),
                model_id: (*model_id).to_string(),
                surface: (*surface).to_string(),
            },
        );
    }

    if let Some(overrides) = effective_config
        .get("defaultSelections")
        .and_then(Value::as_object)
    {
        for (purpose, entry) in overrides {
            let Some(object) = entry.as_object() else {
                continue;
            };
            let provider_id = object
                .get("providerId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let model_id = object
                .get("modelId")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if hidden_builtin_model(model_id, provider_id).is_some() {
                continue;
            }
            selections.insert(
                purpose.clone(),
                DefaultSelection {
                    configured_model_id: object
                        .get("configuredModelId")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    provider_id: provider_id.to_string(),
                    model_id: model_id.to_string(),
                    surface: object
                        .get("surface")
                        .and_then(Value::as_str)
                        .unwrap_or("conversation")
                        .to_string(),
                },
            );
        }
    }

    selections
}
