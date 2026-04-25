use std::{fmt::Write as _, sync::Arc};

use async_trait::async_trait;
use octopus_sdk_context::{PromptCtx, SystemPromptBuilder, SystemPromptSection};
use octopus_sdk_contracts::{PermissionMode, SessionId};
use octopus_sdk_tools::{
    Tool, ToolCategory, ToolContext, ToolError, ToolRegistry, ToolResult, ToolSpec, ToolSurface,
    ToolSurfaceState,
};
use serde_json::json;

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
        panic!("dummy tool should never execute in prompt tests");
    }
}

#[test]
fn prompt_build_is_byte_stable() {
    let registry = sample_registry();
    let surface = registry.assemble_surface(&ToolSurfaceState::default());
    let ctx = sample_ctx(&surface);
    let builder = SystemPromptBuilder::new().with_section(SystemPromptSection {
        id: "custom",
        order: 35,
        body: "<custom>\nStay deterministic.\n</custom>".into(),
    });

    let first = builder.build(&ctx);
    let second = builder.build(&ctx);
    let third = builder.build(&ctx);

    assert_eq!(first, second);
    assert_eq!(second, third);
    assert_eq!(
        hex(builder.fingerprint(&ctx)),
        hex(builder.fingerprint(&ctx))
    );
}

#[test]
fn test_tools_guidance_stability() {
    let registry = sample_registry();
    let surface = registry.assemble_surface(&ToolSurfaceState::default());
    let ctx = sample_ctx(&surface);
    let builder = SystemPromptBuilder::new();

    let first = builder.build(&ctx);
    let second = builder.build(&ctx);
    let third = builder.build(&ctx);

    assert_eq!(first[1], second[1]);
    assert_eq!(second[1], third[1]);
    assert!(first[1].contains("- glob: Match files"));
    assert!(first[1].contains("- write_file: Persist file content"));
    assert!(
        first[1]
            .find("- glob: Match files")
            .expect("glob line should exist")
            < first[1]
                .find("- write_file: Persist file content")
                .expect("write_file line should exist")
    );
}

fn sample_registry() -> ToolRegistry {
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

fn sample_ctx(tools: &ToolSurface) -> PromptCtx<'_> {
    PromptCtx {
        session: SessionId("session-ctx".into()),
        mode: PermissionMode::Plan,
        project_root: "/tmp/octopus".into(),
        tools,
    }
}

fn hex(bytes: [u8; 32]) -> String {
    let mut output = String::new();
    for byte in bytes {
        write!(&mut output, "{byte:02x}").expect("writing to String should not fail");
    }
    output
}
