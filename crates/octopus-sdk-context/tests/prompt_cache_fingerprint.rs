use std::{fmt::Write as _, sync::Arc};

use async_trait::async_trait;
use octopus_sdk_context::{PromptCtx, SystemPromptBuilder};
use octopus_sdk_contracts::{PermissionMode, SessionId};
use octopus_sdk_tools::{
    Tool, ToolCategory, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec, ToolSurface,
    ToolSurfaceState,
};
use serde_json::json;
use sha2::{Digest, Sha256};

struct DummyTool {
    spec: ToolSpec,
}

impl DummyTool {
    fn new(name: &str, description: &str, category: ToolCategory) -> Self {
        Self {
            spec: ToolSpec {
                name: name.into(),
                description: description.into(),
                input_schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }),
                category,
            },
        }
    }
}

#[async_trait]
impl Tool for DummyTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        _ctx: ToolContext,
        _input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        panic!("dummy tool should never execute in fingerprint tests");
    }
}

#[test]
fn prompt_and_tool_fingerprint_are_stable_and_sensitive_to_tool_set() {
    let builder = SystemPromptBuilder::new();
    let initial_registry = base_registry();
    let initial_surface = initial_registry.assemble_surface(&ToolSurfaceState::default());
    let base_ctx = sample_ctx(&initial_surface);

    let first = combined_fingerprint(&builder, &base_ctx);
    let second = combined_fingerprint(&builder, &base_ctx);
    let third = combined_fingerprint(&builder, &base_ctx);

    assert_eq!(first, second);
    assert_eq!(second, third);

    let mut expanded_registry = base_registry();
    expanded_registry
        .register(Arc::new(DummyTool::new(
            "search",
            "Search docs",
            ToolCategory::Read,
        )))
        .expect("search should register");
    let expanded_surface = expanded_registry.assemble_surface(&ToolSurfaceState::default());
    let expanded = combined_fingerprint(&builder, &sample_ctx(&expanded_surface));
    assert_ne!(first, expanded);

    let restored_registry = registry_with_original_tool_set_in_different_order();
    let restored_surface = restored_registry.assemble_surface(&ToolSurfaceState::default());
    let restored = combined_fingerprint(&builder, &sample_ctx(&restored_surface));
    assert_eq!(first, restored);
}

fn combined_fingerprint(builder: &SystemPromptBuilder, ctx: &PromptCtx<'_>) -> String {
    let mut prompt_hex = String::new();
    for byte in builder.fingerprint(ctx) {
        write!(&mut prompt_hex, "{byte:02x}").expect("writing to String should not fail");
    }
    let payload = format!("{prompt_hex}:{}", ctx.tools.fingerprint());
    format!("{:x}", Sha256::digest(payload.as_bytes()))
}

fn sample_ctx(tools: &ToolSurface) -> PromptCtx<'_> {
    PromptCtx {
        session: SessionId("session-fingerprint".into()),
        mode: PermissionMode::Default,
        project_root: "/tmp/octopus".into(),
        tools,
    }
}

fn base_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(DummyTool::new(
            "write_file",
            "Persist file content",
            ToolCategory::Write,
        )))
        .expect("write_file should register");
    registry
        .register(Arc::new(DummyTool::new(
            "glob",
            "Match files",
            ToolCategory::Read,
        )))
        .expect("glob should register");
    registry
        .register(Arc::new(DummyTool::new(
            "bash",
            "Run shell command",
            ToolCategory::Shell,
        )))
        .expect("bash should register");
    registry
}

fn registry_with_original_tool_set_in_different_order() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register(Arc::new(DummyTool::new(
            "bash",
            "Run shell command",
            ToolCategory::Shell,
        )))
        .expect("bash should register");
    registry
        .register(Arc::new(DummyTool::new(
            "glob",
            "Match files",
            ToolCategory::Read,
        )))
        .expect("glob should register");
    registry
        .register(Arc::new(DummyTool::new(
            "write_file",
            "Persist file content",
            ToolCategory::Write,
        )))
        .expect("write_file should register");
    registry
}
