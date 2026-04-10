use super::*;

pub(super) fn apply_provider_overrides(
    providers: &mut BTreeMap<String, ProviderRegistryRecord>,
    overrides: &Value,
) -> Result<(), AppError> {
    let Some(object) = overrides.as_object() else {
        return Ok(());
    };

    for (provider_id, value) in object {
        let mut record =
            providers
                .get(provider_id)
                .cloned()
                .unwrap_or_else(|| ProviderRegistryRecord {
                    provider_id: provider_id.clone(),
                    label: titleize(provider_id),
                    enabled: true,
                    surfaces: Vec::new(),
                    metadata: json!({}),
                });

        if let Some(label) = value.get("label").and_then(Value::as_str) {
            record.label = label.to_string();
        }
        if let Some(enabled) = value.get("enabled").and_then(Value::as_bool) {
            record.enabled = enabled;
        }
        if let Some(metadata) = value.get("metadata") {
            record.metadata = metadata.clone();
        }
        if let Some(surfaces) = value.get("surfaces").and_then(Value::as_array) {
            record.surfaces = surfaces
                .iter()
                .map(parse_surface_descriptor)
                .collect::<Result<Vec<_>, AppError>>()?;
        }

        providers.insert(provider_id.clone(), record);
    }

    Ok(())
}

pub(super) fn apply_model_overrides(
    models: &mut BTreeMap<String, ModelRegistryRecord>,
    overrides: &Value,
) -> Result<(), AppError> {
    let Some(object) = overrides.as_object() else {
        return Ok(());
    };

    for (model_id, value) in object {
        let mut record = models
            .get(model_id)
            .cloned()
            .unwrap_or_else(|| ModelRegistryRecord {
                model_id: model_id.clone(),
                provider_id: value
                    .get("providerId")
                    .and_then(Value::as_str)
                    .unwrap_or("custom")
                    .to_string(),
                label: value
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(model_id)
                    .to_string(),
                description: value
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                family: value
                    .get("family")
                    .and_then(Value::as_str)
                    .unwrap_or(model_id)
                    .to_string(),
                track: value
                    .get("track")
                    .and_then(Value::as_str)
                    .unwrap_or("custom")
                    .to_string(),
                enabled: true,
                recommended_for: value
                    .get("recommendedFor")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                availability: value
                    .get("availability")
                    .and_then(Value::as_str)
                    .unwrap_or("configured")
                    .to_string(),
                default_permission: value
                    .get("defaultPermission")
                    .and_then(Value::as_str)
                    .unwrap_or("auto")
                    .to_string(),
                surface_bindings: Vec::new(),
                capabilities: Vec::new(),
                context_window: value
                    .get("contextWindow")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                max_output_tokens: value
                    .get("maxOutputTokens")
                    .and_then(Value::as_u64)
                    .map(|value| value as u32),
                metadata: value.get("metadata").cloned().unwrap_or_else(|| json!({})),
            });

        if let Some(provider_id) = value.get("providerId").and_then(Value::as_str) {
            record.provider_id = provider_id.to_string();
        }
        if let Some(label) = value.get("label").and_then(Value::as_str) {
            record.label = label.to_string();
        }
        if let Some(description) = value.get("description").and_then(Value::as_str) {
            record.description = description.to_string();
        }
        if let Some(family) = value.get("family").and_then(Value::as_str) {
            record.family = family.to_string();
        }
        if let Some(track) = value.get("track").and_then(Value::as_str) {
            record.track = track.to_string();
        }
        if let Some(enabled) = value.get("enabled").and_then(Value::as_bool) {
            record.enabled = enabled;
        }
        if let Some(recommended_for) = value.get("recommendedFor").and_then(Value::as_str) {
            record.recommended_for = recommended_for.to_string();
        }
        if let Some(availability) = value.get("availability").and_then(Value::as_str) {
            record.availability = availability.to_string();
        }
        if let Some(default_permission) = value.get("defaultPermission").and_then(Value::as_str) {
            record.default_permission = default_permission.to_string();
        }
        if let Some(context_window) = value.get("contextWindow").and_then(Value::as_u64) {
            record.context_window = Some(context_window as u32);
        }
        if let Some(max_output_tokens) = value.get("maxOutputTokens").and_then(Value::as_u64) {
            record.max_output_tokens = Some(max_output_tokens as u32);
        }
        if let Some(metadata) = value.get("metadata") {
            record.metadata = metadata.clone();
        }
        if let Some(surface_bindings) = value.get("surfaceBindings").and_then(Value::as_array) {
            record.surface_bindings = surface_bindings
                .iter()
                .map(parse_surface_binding)
                .collect::<Result<Vec<_>, AppError>>()?;
        }
        if let Some(capabilities) = value.get("capabilities").and_then(Value::as_array) {
            record.capabilities = parse_capabilities(capabilities)?;
        }

        models.insert(model_id.clone(), record);
    }

    Ok(())
}

pub(super) fn apply_default_selections(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    overrides: &Value,
) {
    let Some(object) = overrides.as_object() else {
        return;
    };

    for (purpose, value) in object {
        let provider_id = value
            .get("providerId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let model_id = value
            .get("modelId")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let surface = value
            .get("surface")
            .and_then(Value::as_str)
            .unwrap_or("conversation");
        if provider_id.is_empty() || model_id.is_empty() {
            continue;
        }
        default_selections.insert(
            purpose.clone(),
            DefaultSelection {
                configured_model_id: value
                    .get("configuredModelId")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                provider_id: provider_id.to_string(),
                model_id: model_id.to_string(),
                surface: surface.to_string(),
            },
        );
    }
}

pub(super) fn normalize_default_selection_configured_model_ids(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
) {
    for selection in default_selections.values_mut() {
        if selection.configured_model_id.is_some() {
            continue;
        }
        if configured_models.contains_key(&selection.model_id) {
            selection.configured_model_id = Some(selection.model_id.clone());
            continue;
        }
        if let Some(configured_model) = configured_models.values().find(|configured_model| {
            configured_model.provider_id == selection.provider_id
                && configured_model.model_id == selection.model_id
        }) {
            selection.configured_model_id = Some(configured_model.configured_model_id.clone());
        }
    }
}

pub(super) fn apply_project_settings(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
    project_settings_value: Option<&Value>,
    mcp_servers_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<HashSet<String>> {
    let Some(project_settings) = project_settings_value.and_then(Value::as_object) else {
        return None;
    };
    let workspace_assignments =
        parse_workspace_assignments(project_settings.get("workspaceAssignments"), diagnostics);

    let allowed_configured_model_ids = project_settings.get("models").and_then(|value| {
        apply_project_model_settings(
            default_selections,
            configured_models,
            workspace_assignments.as_ref(),
            value,
            diagnostics,
        )
    });

    if let Some(tool_settings) = project_settings.get("tools") {
        validate_project_tool_settings(
            tool_settings,
            workspace_assignments.as_ref(),
            mcp_servers_value,
            diagnostics,
        );
    }

    if let Some(agent_settings) = project_settings.get("agents") {
        validate_project_agent_settings(
            agent_settings,
            workspace_assignments.as_ref(),
            diagnostics,
        );
    }

    allowed_configured_model_ids
}

pub(super) fn parse_workspace_assignments(
    assignments_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<ProjectWorkspaceAssignments> {
    let Some(assignments_value) = assignments_value else {
        return None;
    };
    match serde_json::from_value::<ProjectWorkspaceAssignments>(assignments_value.clone()) {
        Ok(assignments) => Some(assignments),
        Err(error) => {
            diagnostics.errors.push(format!(
                "projectSettings.workspaceAssignments is invalid: {error}"
            ));
            None
        }
    }
}

pub(super) fn apply_project_model_settings(
    default_selections: &mut BTreeMap<String, DefaultSelection>,
    configured_models: &BTreeMap<String, ConfiguredModelRecord>,
    workspace_assignments: Option<&ProjectWorkspaceAssignments>,
    models_value: &Value,
    diagnostics: &mut ModelRegistryDiagnostics,
) -> Option<HashSet<String>> {
    let Some(models_object) = models_value.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.models must be an object".into());
        return None;
    };

    let allowed_configured_model_ids = models_object
        .get("allowedConfiguredModelIds")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if allowed_configured_model_ids.is_empty() {
        diagnostics.errors.push(
            "projectSettings.models.allowedConfiguredModelIds must include at least one configured model"
                .into(),
        );
        return None;
    }

    let default_configured_model_id = models_object
        .get("defaultConfiguredModelId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if default_configured_model_id.is_empty() {
        diagnostics
            .errors
            .push("projectSettings.models.defaultConfiguredModelId is required".into());
        return None;
    }
    if !allowed_configured_model_ids
        .iter()
        .any(|configured_model_id| configured_model_id == default_configured_model_id)
    {
        diagnostics.errors.push(format!(
            "projectSettings.models.defaultConfiguredModelId `{default_configured_model_id}` must be included in allowedConfiguredModelIds"
        ));
        return None;
    }

    let assigned_configured_model_ids = workspace_assignments
        .and_then(|assignments| assignments.models.as_ref())
        .map(|models| {
            models
                .configured_model_ids
                .iter()
                .cloned()
                .collect::<HashSet<_>>()
        });

    for configured_model_id in &allowed_configured_model_ids {
        if !configured_models.contains_key(configured_model_id) {
            diagnostics.errors.push(format!(
                "projectSettings.models.allowedConfiguredModelIds references unknown configured model `{configured_model_id}`"
            ));
        }
        if assigned_configured_model_ids
            .as_ref()
            .is_some_and(|assigned| !assigned.contains(configured_model_id))
        {
            diagnostics.errors.push(format!(
                "projectSettings.models.allowedConfiguredModelIds contains unassigned configured model `{configured_model_id}`"
            ));
        }
    }

    let Some(default_configured_model) = configured_models.get(default_configured_model_id) else {
        diagnostics.errors.push(format!(
            "projectSettings.models.defaultConfiguredModelId references unknown configured model `{default_configured_model_id}`"
        ));
        return None;
    };

    let conversation_surface = default_selections
        .get("conversation")
        .map(|selection| selection.surface.clone())
        .unwrap_or_else(|| "conversation".into());
    default_selections.insert(
        "conversation".into(),
        DefaultSelection {
            configured_model_id: Some(default_configured_model.configured_model_id.clone()),
            provider_id: default_configured_model.provider_id.clone(),
            model_id: default_configured_model.model_id.clone(),
            surface: conversation_surface,
        },
    );

    Some(
        allowed_configured_model_ids
            .into_iter()
            .collect::<HashSet<_>>(),
    )
}

pub(super) fn validate_project_tool_settings(
    tools_value: &Value,
    workspace_assignments: Option<&ProjectWorkspaceAssignments>,
    mcp_servers_value: Option<&Value>,
    diagnostics: &mut ModelRegistryDiagnostics,
) {
    let Some(tools_object) = tools_value.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.tools must be an object".into());
        return;
    };
    let assigned_source_keys = workspace_assignments
        .and_then(|assignments| assignments.tools.as_ref())
        .map(|tools| tools.source_keys.iter().cloned().collect::<HashSet<_>>());

    if let Some(enabled_source_keys) = tools_object.get("enabledSourceKeys") {
        let Some(enabled_source_keys) = enabled_source_keys.as_array() else {
            diagnostics
                .errors
                .push("projectSettings.tools.enabledSourceKeys must be an array".into());
            return;
        };
        if enabled_source_keys.is_empty() {
            diagnostics.errors.push(
                "projectSettings.tools.enabledSourceKeys must include at least one sourceKey"
                    .into(),
            );
        }
        for source_key in enabled_source_keys.iter().filter_map(Value::as_str) {
            if assigned_source_keys
                .as_ref()
                .is_some_and(|assigned| !assigned.contains(source_key))
            {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.enabledSourceKeys contains unassigned sourceKey `{source_key}`"
                ));
            }
        }
    }

    let Some(overrides) = tools_object.get("overrides") else {
        return;
    };
    let Some(overrides_object) = overrides.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.tools.overrides must be an object".into());
        return;
    };

    let known_mcp_server_names = mcp_servers_value
        .and_then(Value::as_object)
        .map(|servers| servers.keys().cloned().collect::<HashSet<_>>())
        .unwrap_or_default();

    for (source_key, override_value) in overrides_object {
        if assigned_source_keys
            .as_ref()
            .is_some_and(|assigned| !assigned.contains(source_key))
        {
            diagnostics.errors.push(format!(
                "projectSettings.tools.overrides contains unassigned sourceKey `{source_key}`"
            ));
        }
        let Some(override_object) = override_value.as_object() else {
            diagnostics.errors.push(format!(
                "projectSettings.tools.overrides.{source_key} must be an object"
            ));
            continue;
        };

        let permission_mode = override_object
            .get("permissionMode")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(permission_mode, "allow" | "ask" | "readonly" | "deny") {
            diagnostics.errors.push(format!(
                "projectSettings.tools.overrides.{source_key}.permissionMode `{permission_mode}` is unsupported"
            ));
        }

        if let Some(builtin_key) = source_key.strip_prefix("builtin:") {
            if builtin_key.trim().is_empty() {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.overrides contains an invalid builtin sourceKey `{source_key}`"
                ));
            }
            continue;
        }

        if let Some(server_name) = source_key.strip_prefix("mcp:") {
            if server_name.trim().is_empty() || !known_mcp_server_names.contains(server_name) {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.overrides references unknown mcp sourceKey `{source_key}`"
                ));
            }
            continue;
        }

        if let Some(skill_path) = source_key.strip_prefix("skill:") {
            if skill_path.trim().is_empty() {
                diagnostics.errors.push(format!(
                    "projectSettings.tools.overrides contains an invalid skill sourceKey `{source_key}`"
                ));
            }
            continue;
        }

        diagnostics.errors.push(format!(
            "projectSettings.tools.overrides contains unsupported sourceKey `{source_key}`"
        ));
    }
}

pub(super) fn validate_project_agent_settings(
    agents_value: &Value,
    workspace_assignments: Option<&ProjectWorkspaceAssignments>,
    diagnostics: &mut ModelRegistryDiagnostics,
) {
    let Some(agents_object) = agents_value.as_object() else {
        diagnostics
            .errors
            .push("projectSettings.agents must be an object".into());
        return;
    };

    let assigned_agent_ids = workspace_assignments
        .and_then(|assignments| assignments.agents.as_ref())
        .map(|agents| agents.agent_ids.iter().cloned().collect::<HashSet<_>>());
    let assigned_team_ids = workspace_assignments
        .and_then(|assignments| assignments.agents.as_ref())
        .map(|agents| agents.team_ids.iter().cloned().collect::<HashSet<_>>());

    if let Some(enabled_agent_ids) = agents_object.get("enabledAgentIds") {
        let Some(enabled_agent_ids) = enabled_agent_ids.as_array() else {
            diagnostics
                .errors
                .push("projectSettings.agents.enabledAgentIds must be an array".into());
            return;
        };
        for agent_id in enabled_agent_ids.iter().filter_map(Value::as_str) {
            if assigned_agent_ids
                .as_ref()
                .is_some_and(|assigned| !assigned.contains(agent_id))
            {
                diagnostics.errors.push(format!(
                    "projectSettings.agents.enabledAgentIds contains unassigned agent `{agent_id}`"
                ));
            }
        }
    }

    if let Some(enabled_team_ids) = agents_object.get("enabledTeamIds") {
        let Some(enabled_team_ids) = enabled_team_ids.as_array() else {
            diagnostics
                .errors
                .push("projectSettings.agents.enabledTeamIds must be an array".into());
            return;
        };
        for team_id in enabled_team_ids.iter().filter_map(Value::as_str) {
            if assigned_team_ids
                .as_ref()
                .is_some_and(|assigned| !assigned.contains(team_id))
            {
                diagnostics.errors.push(format!(
                    "projectSettings.agents.enabledTeamIds contains unassigned team `{team_id}`"
                ));
            }
        }
    }
}
