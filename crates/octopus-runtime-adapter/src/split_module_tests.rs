use std::sync::Arc;

use octopus_infra::build_infra_bundle;
use uuid::Uuid;

use super::{
    actor_context, approval_flow, config_service, execution_events, execution_service,
    execution_target, persistence, registry, runtime_config, session_service,
    MockRuntimeModelExecutor, RuntimeAdapter,
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
fn split_persistence_module_exposes_runtime_debug_paths() {
    let temp =
        std::env::temp_dir().join(format!("octopus-runtime-adapter-split-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp).expect("tempdir");
    let infra = build_infra_bundle(&temp).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        Arc::new(MockRuntimeModelExecutor),
    );

    let session_path = adapter.runtime_debug_session_path("rt-test");
    let events_path = adapter.runtime_debug_events_path("rt-test");

    assert_eq!(session_path, temp.join("runtime/sessions/rt-test.json"));
    assert_eq!(
        events_path,
        temp.join("runtime/sessions/rt-test-events.json")
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
