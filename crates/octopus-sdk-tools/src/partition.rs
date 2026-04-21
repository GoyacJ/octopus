use octopus_sdk_contracts::ToolCallRequest;

use crate::{ToolRegistry, DEFAULT_TOOL_MAX_CONCURRENCY};

#[derive(Debug, Clone, PartialEq)]
pub enum ExecBatch<'a> {
    Concurrent(Vec<&'a ToolCallRequest>),
    Serial(Vec<&'a ToolCallRequest>),
}

#[must_use]
pub fn partition_tool_calls<'a>(
    calls: &'a [ToolCallRequest],
    registry: &ToolRegistry,
) -> Vec<ExecBatch<'a>> {
    let mut batches = Vec::new();
    let mut concurrent = Vec::new();

    for call in calls {
        match registry.get(&call.name) {
            Some(tool) if tool.is_concurrency_safe(&call.input) => {
                concurrent.push(call);
                if concurrent.len() == DEFAULT_TOOL_MAX_CONCURRENCY {
                    batches.push(ExecBatch::Concurrent(std::mem::take(&mut concurrent)));
                }
            }
            Some(_) | None => {
                flush_concurrent(&mut batches, &mut concurrent);
                batches.push(ExecBatch::Serial(vec![call]));
            }
        }
    }

    flush_concurrent(&mut batches, &mut concurrent);
    batches
}

fn flush_concurrent<'a>(
    batches: &mut Vec<ExecBatch<'a>>,
    concurrent: &mut Vec<&'a ToolCallRequest>,
) {
    if !concurrent.is_empty() {
        batches.push(ExecBatch::Concurrent(std::mem::take(concurrent)));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use octopus_sdk_contracts::{ToolCallId, ToolCallRequest};
    use serde_json::json;

    use super::{partition_tool_calls, ExecBatch};
    use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec};

    struct TestTool {
        spec: ToolSpec,
        concurrency_safe: bool,
    }

    impl TestTool {
        fn new(name: &str, category: ToolCategory, concurrency_safe: bool) -> Self {
            Self {
                spec: ToolSpec {
                    name: name.into(),
                    description: format!("{name} description"),
                    input_schema: json!({ "type": "object" }),
                    category,
                },
                concurrency_safe,
            }
        }
    }

    #[async_trait]
    impl Tool for TestTool {
        fn spec(&self) -> &ToolSpec {
            &self.spec
        }

        fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
            self.concurrency_safe
        }

        async fn execute(
            &self,
            _ctx: ToolContext,
            _input: serde_json::Value,
        ) -> Result<ToolResult, ToolError> {
            Ok(ToolResult::default())
        }
    }

    fn call(id: &str, name: &str) -> ToolCallRequest {
        ToolCallRequest {
            id: ToolCallId(id.into()),
            name: name.into(),
            input: json!({ "path": format!("/tmp/{id}") }),
        }
    }

    #[test]
    fn partition_groups_read_only_calls_into_one_batch() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(TestTool::new("read", ToolCategory::Read, true)))
            .expect("read tool should register");

        let calls = (0..8)
            .map(|index| call(&format!("call-{index}"), "read"))
            .collect::<Vec<_>>();

        let batches = partition_tool_calls(&calls, &registry);

        assert_eq!(batches.len(), 1);
        match &batches[0] {
            ExecBatch::Concurrent(batch) => assert_eq!(batch.len(), 8),
            ExecBatch::Serial(_) => panic!("expected concurrent batch"),
        }
    }

    #[test]
    fn partition_respects_max_concurrency_limit() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(TestTool::new("read", ToolCategory::Read, true)))
            .expect("read tool should register");

        let calls = (0..11)
            .map(|index| call(&format!("call-{index}"), "read"))
            .collect::<Vec<_>>();

        let batches = partition_tool_calls(&calls, &registry);

        assert_eq!(batches.len(), 2);
        match (&batches[0], &batches[1]) {
            (ExecBatch::Concurrent(first), ExecBatch::Concurrent(second)) => {
                assert_eq!(first.len(), 10);
                assert_eq!(second.len(), 1);
            }
            _ => panic!("expected concurrent batches"),
        }
    }

    #[test]
    fn partition_serializes_writes_between_reads() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(TestTool::new("read", ToolCategory::Read, true)))
            .expect("read tool should register");
        registry
            .register(Arc::new(TestTool::new("write", ToolCategory::Write, false)))
            .expect("write tool should register");

        let calls = vec![
            call("call-1", "read"),
            call("call-2", "write"),
            call("call-3", "read"),
        ];

        let batches = partition_tool_calls(&calls, &registry);

        assert_eq!(batches.len(), 3);
        match (&batches[0], &batches[1], &batches[2]) {
            (
                ExecBatch::Concurrent(first),
                ExecBatch::Serial(second),
                ExecBatch::Concurrent(third),
            ) => {
                assert_eq!(first.len(), 1);
                assert_eq!(second.len(), 1);
                assert_eq!(third.len(), 1);
            }
            _ => panic!("expected read/write/read partitioning"),
        }
    }

    #[test]
    fn partition_unknown_tool_is_serial() {
        let registry = ToolRegistry::new();
        let calls = vec![call("call-1", "missing")];

        let batches = partition_tool_calls(&calls, &registry);

        assert_eq!(batches, vec![ExecBatch::Serial(vec![&calls[0]])]);
    }
}
