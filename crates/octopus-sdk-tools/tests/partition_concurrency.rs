use octopus_sdk_contracts::{ToolCallId, ToolCallRequest};
use octopus_sdk_tools::{
    builtin::FileWriteTool, partition_tool_calls, ExecBatch, Tool, ToolRegistry,
    DEFAULT_TOOL_MAX_CONCURRENCY,
};
use serde_json::json;

fn call(id: &str, name: &str, input: serde_json::Value) -> ToolCallRequest {
    ToolCallRequest {
        id: ToolCallId(id.into()),
        name: name.into(),
        input,
    }
}

#[test]
fn partition_respects_max_concurrency() {
    let mut registry = ToolRegistry::new();
    registry
        .register(std::sync::Arc::new(
            octopus_sdk_tools::builtin::FileReadTool::new(),
        ))
        .expect("read tool should register");

    let calls = (0..=DEFAULT_TOOL_MAX_CONCURRENCY)
        .map(|index| {
            call(
                &format!("call-{index}"),
                "read_file",
                json!({ "path": "Cargo.toml" }),
            )
        })
        .collect::<Vec<_>>();

    let batches = partition_tool_calls(&calls, &registry);

    assert_eq!(batches.len(), 2);
    match (&batches[0], &batches[1]) {
        (ExecBatch::Concurrent(first), ExecBatch::Concurrent(second)) => {
            assert_eq!(first.len(), DEFAULT_TOOL_MAX_CONCURRENCY);
            assert_eq!(second.len(), 1);
        }
        other => panic!("expected two concurrent batches, got {other:?}"),
    }
}

#[test]
fn partition_serializes_writes() {
    let mut registry = ToolRegistry::new();
    registry
        .register(std::sync::Arc::new(
            octopus_sdk_tools::builtin::FileReadTool::new(),
        ))
        .expect("read tool should register");
    registry
        .register(std::sync::Arc::new(FileWriteTool::new()))
        .expect("write tool should register");

    let calls = vec![
        call("call-1", "read_file", json!({ "path": "Cargo.toml" })),
        call(
            "call-2",
            "write_file",
            json!({ "path": "a.txt", "content": "a" }),
        ),
        call(
            "call-3",
            "write_file",
            json!({ "path": "b.txt", "content": "b" }),
        ),
        call("call-4", "read_file", json!({ "path": "Cargo.toml" })),
    ];

    let batches = partition_tool_calls(&calls, &registry);

    assert_eq!(batches.len(), 4);
    assert!(matches!(batches[0], ExecBatch::Concurrent(_)));
    assert!(matches!(batches[1], ExecBatch::Serial(_)));
    assert!(matches!(batches[2], ExecBatch::Serial(_)));
    assert!(matches!(batches[3], ExecBatch::Concurrent(_)));
}

#[test]
fn partition_unknown_tool_serial() {
    let registry = ToolRegistry::new();
    let calls = vec![call("call-1", "missing_tool", json!({}))];

    let batches = partition_tool_calls(&calls, &registry);

    assert_eq!(batches, vec![ExecBatch::Serial(vec![&calls[0]])]);
}

#[test]
fn partition_respects_input_concurrency_hint() {
    let tool = FileWriteTool::new();
    assert!(!tool.is_concurrency_safe(&json!({ "path": "a.txt", "content": "a" })));
    assert!(!tool.is_concurrency_safe(&json!({ "path": "b.txt", "content": "b" })));
}
