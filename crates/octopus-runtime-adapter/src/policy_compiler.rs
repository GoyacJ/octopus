use super::*;

use octopus_core::{
    default_agent_delegation_policy, default_agent_memory_policy, default_approval_preference,
    default_team_delegation_policy, default_team_memory_policy, ApprovalPreference,
    DelegationPolicy, MemoryPolicy, RuntimeTargetPolicyDecision, RuntimeTargetPolicyDecisions,
    SessionRecord,
};
use serde::de::DeserializeOwned;

fn normalize_permission_mode_input(value: &str, field: &str) -> Result<Option<String>, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    octopus_core::normalize_runtime_permission_mode_label(trimmed)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            AppError::invalid_input(format!(
                "unsupported permission mode for {field}: {trimmed}"
            ))
        })
        .map(Some)
}

fn configured_permission_mode(
    effective_config: &serde_json::Value,
    pointer: &str,
) -> Option<String> {
    effective_config
        .pointer(pointer)
        .and_then(serde_json::Value::as_str)
        .and_then(octopus_core::normalize_runtime_permission_mode_label)
        .map(ToOwned::to_owned)
}

fn clamp_permission_mode(requested: &str, ceiling: &str) -> String {
    octopus_core::clamp_runtime_permission_mode(requested, ceiling)
}

fn synthetic_runtime_session(
    adapter: &RuntimeAdapter,
    session_id: &str,
    user_id: &str,
) -> SessionRecord {
    SessionRecord {
        id: session_id.to_string(),
        workspace_id: adapter.state.workspace_id.clone(),
        user_id: user_id.to_string(),
        client_app_id: "runtime-adapter".into(),
        token: String::new(),
        status: "active".into(),
        created_at: timestamp_now(),
        expires_at: None,
    }
}

fn decision_key(target_kind: &str, target_ref: &str) -> String {
    format!("{target_kind}:{target_ref}")
}

fn merge_policy_json(base: serde_json::Value, patch: serde_json::Value) -> serde_json::Value {
    match (base, patch) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(patch_map)) => {
            for (key, patch_value) in patch_map {
                let merged = merge_policy_json(
                    base_map.remove(&key).unwrap_or(serde_json::Value::Null),
                    patch_value,
                );
                base_map.insert(key, merged);
            }
            serde_json::Value::Object(base_map)
        }
        (base, serde_json::Value::Null) => base,
        (_, patch) => patch,
    }
}

fn parse_policy_value<T, F>(value: serde_json::Value, default: F) -> T
where
    T: DeserializeOwned + serde::Serialize,
    F: FnOnce() -> T,
{
    let default_value = default();
    let merged = serde_json::to_value(&default_value)
        .ok()
        .map(|default_json| merge_policy_json(default_json, value))
        .unwrap_or(serde_json::Value::Null);
    serde_json::from_value(merged).unwrap_or(default_value)
}

fn actor_policy_defaults(
    manifest: &actor_manifest::CompiledActorManifest,
) -> (MemoryPolicy, DelegationPolicy) {
    match manifest {
        actor_manifest::CompiledActorManifest::Agent(_) => (
            default_agent_memory_policy(),
            default_agent_delegation_policy(),
        ),
        actor_manifest::CompiledActorManifest::Team(_) => (
            default_team_memory_policy(),
            default_team_delegation_policy(),
        ),
    }
}

fn deferred_decision(
    target_kind: &str,
    action: &str,
    reason: impl Into<String>,
    capability_id: Option<String>,
    provider_key: Option<String>,
    requires_approval: bool,
    requires_auth: bool,
) -> RuntimeTargetPolicyDecision {
    RuntimeTargetPolicyDecision {
        target_kind: target_kind.into(),
        action: action.into(),
        hidden: false,
        visible: false,
        deferred: true,
        requires_approval,
        requires_auth,
        reason: Some(reason.into()),
        capability_id,
        provider_key,
        required_permission: None,
    }
}

fn deny_decision(
    target_kind: &str,
    reason: impl Into<String>,
    capability_id: Option<String>,
    provider_key: Option<String>,
    required_permission: Option<String>,
) -> RuntimeTargetPolicyDecision {
    RuntimeTargetPolicyDecision {
        target_kind: target_kind.into(),
        action: "deny".into(),
        hidden: true,
        visible: false,
        deferred: false,
        requires_approval: false,
        requires_auth: false,
        reason: Some(reason.into()),
        capability_id,
        provider_key,
        required_permission,
    }
}

fn allow_decision(
    target_kind: &str,
    capability_id: Option<String>,
    reason: Option<String>,
    required_permission: Option<String>,
) -> RuntimeTargetPolicyDecision {
    RuntimeTargetPolicyDecision {
        target_kind: target_kind.into(),
        action: "allow".into(),
        hidden: false,
        visible: true,
        deferred: false,
        requires_approval: false,
        requires_auth: false,
        reason,
        capability_id,
        provider_key: None,
        required_permission,
    }
}

async fn authorize_bucket(
    adapter: &RuntimeAdapter,
    session_id: &str,
    user_id: &str,
    project_id: Option<&str>,
    capability: &str,
    target_kind: &str,
) -> Result<RuntimeTargetPolicyDecision, AppError> {
    if user_id.trim().is_empty() {
        return Ok(RuntimeTargetPolicyDecision {
            target_kind: target_kind.into(),
            action: "allow".into(),
            visible: true,
            capability_id: Some(capability.into()),
            ..RuntimeTargetPolicyDecision::default()
        });
    }

    let session = synthetic_runtime_session(adapter, session_id, user_id);
    let decision = adapter
        .state
        .authorization
        .authorize(&session, capability, project_id)
        .await?;
    Ok(RuntimeTargetPolicyDecision {
        target_kind: target_kind.into(),
        action: if decision.allowed { "allow" } else { "deny" }.into(),
        hidden: !decision.allowed,
        visible: decision.allowed,
        deferred: false,
        requires_approval: false,
        requires_auth: false,
        reason: decision.reason,
        capability_id: Some(capability.into()),
        provider_key: None,
        required_permission: None,
    })
}

fn configured_model_enabled(
    effective_config: &serde_json::Value,
    configured_model_id: &str,
) -> bool {
    effective_config
        .get("configuredModels")
        .and_then(serde_json::Value::as_object)
        .and_then(|models| models.get(configured_model_id))
        .and_then(serde_json::Value::as_object)
        .map(|model| {
            model
                .get("enabled")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true)
        })
        .unwrap_or(false)
}

fn mcp_server_configured(effective_config: &serde_json::Value, server_name: &str) -> bool {
    effective_config
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|servers| servers.contains_key(server_name))
}

fn compile_target_decisions(
    manifest: &actor_manifest::CompiledActorManifest,
    approval_preference: &ApprovalPreference,
    memory_policy: &MemoryPolicy,
    delegation_policy: &DelegationPolicy,
    execution_permission_mode: &str,
    configured_model_id: Option<&str>,
    effective_config: &serde_json::Value,
) -> RuntimeTargetPolicyDecisions {
    let actor_ref = manifest.actor_ref().to_string();
    let mut decisions = RuntimeTargetPolicyDecisions::new();

    if let Some(configured_model_id) = configured_model_id {
        let model_execution_decision =
            if !configured_model_enabled(effective_config, configured_model_id) {
                deny_decision(
                    "model-execution",
                    "configured model is disabled or unavailable in the frozen runtime config",
                    Some(format!("model-execution:{configured_model_id}")),
                    None,
                    Some(execution_permission_mode.into()),
                )
            } else if approval_preference.tool_execution == "require-approval" {
                RuntimeTargetPolicyDecision {
                    target_kind: "model-execution".into(),
                    action: "requireApproval".into(),
                    hidden: false,
                    visible: false,
                    deferred: true,
                    requires_approval: true,
                    requires_auth: false,
                    reason: Some("tool execution requires mediation review".into()),
                    capability_id: Some(format!("model-execution:{configured_model_id}")),
                    provider_key: None,
                    required_permission: Some(execution_permission_mode.into()),
                }
            } else {
                allow_decision(
                    "model-execution",
                    Some(format!("model-execution:{configured_model_id}")),
                    Some("model execution is enabled by the frozen session policy".into()),
                    Some(execution_permission_mode.into()),
                )
            };
        decisions.insert(
            decision_key("model-execution", configured_model_id),
            model_execution_decision,
        );
    }

    let memory_requires_approval = memory_policy.write_requires_approval
        || approval_preference.memory_write == "require-approval";
    let memory_decision = if memory_requires_approval {
        deferred_decision(
            "memory-write",
            "requireApproval",
            "memory writes are held for mediation review",
            Some(actor_ref.clone()),
            None,
            true,
            false,
        )
    } else {
        allow_decision(
            "memory-write",
            Some(actor_ref.clone()),
            Some("memory writes are enabled by the frozen session policy".into()),
            None,
        )
    };
    decisions.insert(decision_key("memory-write", &actor_ref), memory_decision);

    let team_spawn_decision =
        if delegation_policy.mode == "disabled" || delegation_policy.max_worker_count == 0 {
            RuntimeTargetPolicyDecision {
                target_kind: "team-spawn".into(),
                action: "deny".into(),
                hidden: true,
                visible: false,
                deferred: false,
                requires_approval: false,
                requires_auth: false,
                reason: Some("delegation is disabled by the frozen session policy".into()),
                capability_id: Some(actor_ref.clone()),
                provider_key: None,
                required_permission: None,
            }
        } else if approval_preference.team_spawn == "require-approval" {
            deferred_decision(
                "team-spawn",
                "requireApproval",
                "team worker spawning requires mediation review",
                Some(actor_ref.clone()),
                None,
                true,
                false,
            )
        } else {
            allow_decision(
                "team-spawn",
                Some(actor_ref.clone()),
                Some("team worker spawning is enabled by the frozen session policy".into()),
                None,
            )
        };
    decisions.insert(decision_key("team-spawn", &actor_ref), team_spawn_decision);

    let workflow_decision = if approval_preference.workflow_escalation == "require-approval" {
        deferred_decision(
            "workflow-continuation",
            "requireApproval",
            "workflow continuation requires mediation review",
            Some(actor_ref.clone()),
            None,
            true,
            false,
        )
    } else {
        allow_decision(
            "workflow-continuation",
            Some(actor_ref.clone()),
            Some("workflow continuation is enabled by the frozen session policy".into()),
            None,
        )
    };
    decisions.insert(
        decision_key("workflow-continuation", &actor_ref),
        workflow_decision,
    );

    for provider_key in manifest.mcp_server_names() {
        let provider_auth_decision = if mcp_server_configured(effective_config, provider_key) {
            deferred_decision(
                "provider-auth",
                "requireAuth",
                "provider or MCP auth must resolve before mediated execution can continue",
                Some(format!("provider-auth:{provider_key}")),
                Some(provider_key.clone()),
                approval_preference.mcp_auth == "require-approval",
                true,
            )
        } else {
            deny_decision(
                "provider-auth",
                "provider or MCP server is not configured in the frozen runtime config",
                Some(format!("provider-auth:{provider_key}")),
                Some(provider_key.clone()),
                None,
            )
        };
        decisions.insert(
            decision_key("provider-auth", provider_key),
            provider_auth_decision,
        );
    }

    decisions
}

pub(crate) fn compile_manifest_target_decisions(
    manifest: &actor_manifest::CompiledActorManifest,
    execution_permission_mode: &str,
    selected_configured_model_id: Option<&str>,
    effective_config: &serde_json::Value,
) -> RuntimeTargetPolicyDecisions {
    let normalized_execution_permission_mode =
        octopus_core::normalize_runtime_permission_mode_label(execution_permission_mode)
            .unwrap_or(execution_permission_mode);
    let (default_memory_policy, default_delegation_policy) = actor_policy_defaults(manifest);
    let memory_policy =
        parse_policy_value(manifest.memory_policy_value(), || default_memory_policy);
    let delegation_policy = parse_policy_value(manifest.delegation_policy_value(), || {
        default_delegation_policy
    });
    let approval_preference = parse_policy_value(
        manifest.approval_preference_value(),
        default_approval_preference,
    );
    let configured_model_id = selected_configured_model_id.or_else(|| manifest.default_model_ref());

    compile_target_decisions(
        manifest,
        &approval_preference,
        &memory_policy,
        &delegation_policy,
        normalized_execution_permission_mode,
        configured_model_id,
        effective_config,
    )
}

pub(super) async fn compile_session_policy(
    adapter: &RuntimeAdapter,
    session_id: &str,
    manifest: &actor_manifest::CompiledActorManifest,
    snapshot: &RuntimeConfigSnapshotSummary,
    selected_configured_model_id: Option<&str>,
    execution_permission_mode: &str,
    user_id: &str,
    project_id: Option<&str>,
    owner_permission_ceiling: Option<&str>,
) -> Result<session_policy::CompiledSessionPolicy, AppError> {
    let effective_config = adapter.config_snapshot_value(&snapshot.id)?;
    let configured_default_mode =
        configured_permission_mode(&effective_config, "/permissions/defaultMode");
    let configured_max_mode = configured_permission_mode(&effective_config, "/permissions/maxMode");
    let requested_permission_mode = normalize_permission_mode_input(
        execution_permission_mode,
        "session execution permission mode",
    )?
    .or(configured_default_mode)
    .unwrap_or_else(|| RUNTIME_PERMISSION_WORKSPACE_WRITE.to_string());
    let manifest_permission_ceiling =
        octopus_core::normalize_runtime_permission_mode_label(manifest.permission_ceiling())
            .unwrap_or(RUNTIME_PERMISSION_WORKSPACE_WRITE);
    let effective_permission_ceiling = configured_max_mode
        .as_deref()
        .map(|configured_max_mode| {
            clamp_permission_mode(manifest_permission_ceiling, configured_max_mode)
        })
        .unwrap_or_else(|| manifest_permission_ceiling.to_string());
    let effective_permission_ceiling = owner_permission_ceiling
        .map(|owner_permission_ceiling| {
            clamp_permission_mode(&effective_permission_ceiling, owner_permission_ceiling)
        })
        .unwrap_or(effective_permission_ceiling);
    let normalized_execution_permission_mode =
        clamp_permission_mode(&requested_permission_mode, &effective_permission_ceiling);

    let builtin = authorize_bucket(
        adapter,
        session_id,
        user_id,
        project_id,
        "tool.builtin.invoke",
        "capability-builtin",
    )
    .await?;
    let skill = authorize_bucket(
        adapter,
        session_id,
        user_id,
        project_id,
        "tool.skill.invoke",
        "capability-skill",
    )
    .await?;
    let mcp = authorize_bucket(
        adapter,
        session_id,
        user_id,
        project_id,
        "tool.mcp.invoke",
        "capability-mcp",
    )
    .await?;

    let configured_model_id = selected_configured_model_id
        .map(ToOwned::to_owned)
        .or_else(|| manifest.default_model_ref().map(ToOwned::to_owned));
    let target_decisions = compile_manifest_target_decisions(
        manifest,
        &normalized_execution_permission_mode,
        configured_model_id.as_deref(),
        &effective_config,
    );

    Ok(session_policy::CompiledSessionPolicy {
        user_id: user_id.to_string(),
        selected_actor_ref: manifest.actor_ref().to_string(),
        selected_configured_model_id: selected_configured_model_id
            .map(ToOwned::to_owned)
            .or_else(|| manifest.default_model_ref().map(ToOwned::to_owned)),
        execution_permission_mode: normalized_execution_permission_mode,
        config_snapshot_id: snapshot.id.clone(),
        effective_config_hash: snapshot.effective_config_hash.clone(),
        started_from_scope_set: snapshot.started_from_scope_set.clone(),
        manifest_revision: manifest.manifest_revision().to_string(),
        capability_policy: manifest.capability_policy_value(),
        memory_policy: manifest.memory_policy_value(),
        delegation_policy: manifest.delegation_policy_value(),
        approval_preference: manifest.approval_preference_value(),
        capability_decisions: RuntimeCapabilityPolicyDecisions {
            builtin,
            skill,
            mcp,
        },
        target_decisions,
        manifest_snapshot_ref: format!("{session_id}-manifest"),
        session_policy_snapshot_ref: format!("{session_id}-policy"),
    })
}

pub(super) fn policy_decision_for_capability<'a>(
    session_policy: &'a session_policy::CompiledSessionPolicy,
    capability: &tools::CapabilitySpec,
) -> &'a RuntimeTargetPolicyDecision {
    match capability.source_kind {
        tools::CapabilitySourceKind::Builtin
        | tools::CapabilitySourceKind::RuntimeTool
        | tools::CapabilitySourceKind::PluginTool => &session_policy.capability_decisions.builtin,
        tools::CapabilitySourceKind::LocalSkill
        | tools::CapabilitySourceKind::BundledSkill
        | tools::CapabilitySourceKind::PluginSkill => &session_policy.capability_decisions.skill,
        tools::CapabilitySourceKind::McpTool
        | tools::CapabilitySourceKind::McpPrompt
        | tools::CapabilitySourceKind::McpResource => &session_policy.capability_decisions.mcp,
    }
}
