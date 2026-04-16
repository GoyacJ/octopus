use std::sync::Arc;

use octopus_infra::build_infra_bundle;
use uuid::Uuid;

use super::{
    actor_context, approval_broker, approval_flow, config_service, execution_events,
    execution_service, execution_target, persistence, registry, runtime_config, session_service,
    MockRuntimeModelDriver, RuntimeAdapter,
};

#[test]
fn split_runtime_config_module_exposes_scope_helpers() {
    assert!(matches!(
        RuntimeAdapter::parse_scope("workspace").expect("workspace scope"),
        runtime_config::RuntimeConfigScopeKind::Workspace
    ));
    assert!(RuntimeAdapter::parse_scope("unsupported").is_err());
}

#[test]
fn split_persistence_module_exposes_runtime_paths_and_jsonl_append() {
    let temp =
        std::env::temp_dir().join(format!("octopus-runtime-adapter-split-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp).expect("tempdir");
    let infra = build_infra_bundle(&temp).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let events_path = adapter.runtime_events_path("rt-test");
    let subrun_path = adapter.runtime_subrun_state_path("run-test-subrun-1");

    assert_eq!(events_path, temp.join("runtime/events/rt-test.jsonl"));
    assert_eq!(
        subrun_path,
        temp.join("runtime/state/subruns/run-test-subrun-1.json")
    );

    let append_target = temp.join("runtime/events/append-test.jsonl");
    persistence::append_json_line(&append_target, &serde_json::json!({ "ok": true }))
        .expect("append json line");
    assert!(append_target.exists());

    std::fs::remove_dir_all(temp).expect("cleanup");
}

#[test]
fn split_actor_context_module_builds_prompt_sections() {
    let prompt = actor_context::build_actor_system_prompt([
        Some(" Alpha ".to_string()),
        None,
        Some("".to_string()),
        Some("Beta".to_string()),
    ]);

    assert_eq!(prompt.as_deref(), Some("Alpha\n\nBeta"));
}

#[test]
fn split_registry_submodules_expose_baseline_and_parse_helpers() {
    let providers = registry::baseline::baseline_providers();
    assert!(providers.contains_key("openai"));

    assert_eq!(
        registry::parse::titleize("workspace_models"),
        "Workspace Models"
    );
}

#[test]
fn split_session_service_module_exposes_session_kind_helper() {
    assert_eq!(
        session_service::default_session_kind(None),
        "project".to_string()
    );
    assert_eq!(
        session_service::default_session_kind(Some("review".into())),
        "review".to_string()
    );
}

#[test]
fn split_config_service_module_applies_validation_to_effective_config() {
    let effective = octopus_core::RuntimeEffectiveConfig {
        effective_config: serde_json::json!({}),
        effective_config_hash: "hash".into(),
        sources: Vec::new(),
        validation: octopus_core::RuntimeConfigValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        },
        secret_references: Vec::new(),
    };
    let validation = octopus_core::RuntimeConfigValidationResult {
        valid: false,
        errors: vec!["boom".into()],
        warnings: vec!["warn".into()],
    };

    let updated = config_service::apply_validation(effective, validation.clone());

    assert_eq!(updated.validation, validation);
}

#[test]
fn split_execution_service_module_normalizes_decision_and_permission_helpers() {
    assert_eq!(
        execution_service::approval_decision_status("approve").expect("approve"),
        "approved"
    );
    assert!(execution_service::requires_approval("workspace-write").expect("workspace-write"),);
    assert!(execution_service::approval_decision_status("nope").is_err());
}

#[test]
fn split_execution_target_module_exposes_permission_helpers() {
    assert!(execution_target::requires_approval("workspace-write").expect("workspace-write"));
    assert!(!execution_target::requires_approval("read-only").expect("read-only"));
}

#[test]
fn split_approval_flow_module_normalizes_decision_status() {
    assert_eq!(
        approval_flow::approval_decision_status("reject").expect("reject"),
        "rejected"
    );
    assert!(approval_flow::approval_decision_status("later").is_err());
}

#[test]
fn split_approval_broker_module_routes_all_mediation_through_one_entrypoint() {
    let base_request = approval_broker::MediationRequest {
        session_id: "rt-1".into(),
        conversation_id: "conv-1".into(),
        run_id: "run-1".into(),
        tool_name: "workspace-api".into(),
        summary: "Workspace API call requires mediation".into(),
        detail: "Review before the tool call can continue.".into(),
        mediation_kind: "approval".into(),
        approval_layer: "capability-call".into(),
        target_kind: "capability-call".into(),
        target_ref: "capability-call:run-1:tool-use-1".into(),
        capability_id: Some("cap-1".into()),
        dispatch_kind: "runtime_capability".into(),
        provider_key: None,
        concurrency_policy: "serialized".into(),
        input: serde_json::json!({ "path": "." }),
        required_permission: Some("workspace-write".into()),
        escalation_reason: Some("approval required".into()),
        requires_approval: true,
        requires_auth: false,
        created_at: 1,
        risk_level: "high".into(),
        checkpoint_ref: None,
        policy_action: None,
        pending_state: None,
    };

    let approval = approval_broker::mediate(&base_request);
    assert_eq!(approval.state, "requireApproval");
    assert!(approval.pending_mediation.is_some());
    assert!(approval.approval.is_some());

    let auth = approval_broker::mediate(&approval_broker::MediationRequest {
        mediation_kind: "auth".into(),
        approval_layer: "provider-auth".into(),
        target_kind: "provider-auth".into(),
        target_ref: "mcp-server".into(),
        provider_key: Some("mcp-server".into()),
        escalation_reason: Some("auth required".into()),
        requires_approval: false,
        requires_auth: true,
        policy_action: None,
        ..base_request.clone()
    });
    assert_eq!(auth.state, "requireAuth");
    assert!(auth.pending_mediation.is_some());
    assert!(auth.auth_challenge.is_some());

    let allowed = approval_broker::mediate(&approval_broker::MediationRequest {
        requires_approval: false,
        requires_auth: false,
        escalation_reason: None,
        policy_action: Some("allow".into()),
        ..base_request.clone()
    });
    assert_eq!(allowed.state, "allow");
    assert!(allowed.pending_mediation.is_none());

    let denied = approval_broker::mediate(&approval_broker::MediationRequest {
        requires_approval: false,
        requires_auth: false,
        policy_action: Some("deny".into()),
        escalation_reason: Some("policy denied".into()),
        ..base_request.clone()
    });
    assert_eq!(denied.state, "deny");
    assert!(denied.pending_mediation.is_none());
    assert_eq!(denied.execution_outcome.outcome, "deny");

    let deferred = approval_broker::mediate(&approval_broker::MediationRequest {
        mediation_kind: "memory".into(),
        approval_layer: "memory-review".into(),
        target_kind: "memory-write".into(),
        target_ref: "proposal-1".into(),
        tool_name: "Agent".into(),
        summary: "Memory proposal pending review".into(),
        detail: "Durable memory stays proposal-only until review.".into(),
        requires_approval: false,
        requires_auth: false,
        policy_action: Some("defer".into()),
        pending_state: Some("pending_review".into()),
        ..base_request
    });
    assert_eq!(deferred.state, "defer");
    assert_eq!(
        deferred
            .pending_mediation
            .as_ref()
            .map(|mediation| mediation.state.as_str()),
        Some("pending_review")
    );
    assert!(deferred.approval.is_none());
    assert!(deferred.auth_challenge.is_none());
}

#[test]
fn split_execution_events_module_derives_usage_cost_shape() {
    assert_eq!(
        execution_events::usage_cost_shape(Some(42)),
        ("tokens", 42_i64, "tokens")
    );
    assert_eq!(
        execution_events::usage_cost_shape(None),
        ("turns", 1_i64, "count")
    );
}
