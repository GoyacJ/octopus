use octopus_sdk_contracts::{
    AskOption, AskPrompt, AskQuestion, CompactionResult, CompactionStrategyTag, EventId, HookEvent,
    MemoryError, MemoryItem, MemoryKind, PermissionOutcome, ToolCategory,
};
use serde_json::json;

#[test]
fn hook_event_variant_count_is_stable() {
    assert_eq!(HookEvent::VARIANT_COUNT, 8);
}

#[test]
fn permission_outcome_require_auth_serializes_with_stable_tag() {
    let outcome = PermissionOutcome::RequireAuth {
        prompt: AskPrompt {
            kind: "require-auth".into(),
            questions: vec![AskQuestion {
                id: "oauth".into(),
                question: "Authenticate?".into(),
                header: "Auth".into(),
                multi_select: false,
                options: vec![AskOption {
                    id: "approve".into(),
                    label: "Continue".into(),
                    description: "Open the auth flow.".into(),
                    preview: None,
                    preview_format: None,
                }],
            }],
        },
    };

    let value = serde_json::to_value(outcome).expect("require auth should serialize");
    assert_eq!(value["require_auth"]["prompt"]["kind"], "require-auth");
}

#[test]
fn compaction_and_memory_contracts_round_trip() {
    let result = CompactionResult {
        summary: "condensed".into(),
        folded_turn_ids: vec![EventId("event-1".into()), EventId("event-2".into())],
        tool_results_cleared: 2,
        tokens_before: 1200,
        tokens_after: 300,
        strategy: CompactionStrategyTag::Summarize,
    };
    let value = serde_json::to_value(&result).expect("compaction result should serialize");
    let roundtrip: CompactionResult =
        serde_json::from_value(value).expect("compaction result should deserialize");
    assert_eq!(roundtrip, result);

    let memory = MemoryItem {
        id: "memory-1".into(),
        kind: MemoryKind::Decision,
        payload: json!({ "text": "use deterministic hooks" }),
        created_at_ms: 1_713_692_800_123,
    };
    let value = serde_json::to_value(&memory).expect("memory item should serialize");
    let roundtrip: MemoryItem =
        serde_json::from_value(value).expect("memory item should deserialize");
    assert_eq!(roundtrip, memory);
}

#[test]
fn tool_category_lives_in_contracts() {
    assert_eq!(ToolCategory::Shell.category_priority(), 3);
}

#[test]
fn memory_error_serialization_variant_preserves_source_message() {
    let error = MemoryError::from(
        serde_json::from_str::<MemoryItem>("not-json")
            .expect_err("invalid json should raise a serde_json::Error"),
    );

    assert!(error.to_string().contains("memory serialization error"));
}
