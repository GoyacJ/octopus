use std::error::Error;

use octopus_sdk_contracts::{PermissionMode, SubagentError, SubagentSpec, TaskBudget};

#[test]
fn subagent_spec_serialization_is_stable() {
    let spec = SubagentSpec {
        id: "code-reviewer".into(),
        system_prompt: "Review the diff and report blockers.".into(),
        allowed_tools: vec!["fs_read".into(), "rg".into()],
        model_role: "main".into(),
        permission_mode: PermissionMode::Plan,
        task_budget: TaskBudget {
            total: 40_000,
            completion_threshold: 0.9,
        },
        max_turns: 12,
        depth: 1,
    };

    let first = serde_json::to_vec(&spec).expect("subagent spec should serialize");
    let second = serde_json::to_vec(&spec).expect("subagent spec should serialize twice");
    let third = serde_json::to_vec(&spec).expect("subagent spec should serialize three times");
    let roundtrip: SubagentSpec =
        serde_json::from_slice(&first).expect("subagent spec should roundtrip");

    assert_eq!(first, second);
    assert_eq!(second, third);
    assert_eq!(roundtrip, spec);
}

#[test]
fn subagent_error_implements_std_error() {
    let error = SubagentError::Provider {
        reason: "model unavailable".into(),
    };
    let as_error: &dyn Error = &error;

    assert!(as_error.to_string().contains("model unavailable"));
}
