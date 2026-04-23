use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use octopus_sdk_contracts::{ContentBlock, PluginSourceTag, ToolDecl};
use octopus_sdk_tools::{Tool, ToolContext, ToolError, ToolResult, ToolSpec};

use crate::{
    PluginApi, PluginComponent, PluginError, PluginManifest, PluginRuntime, PluginRuntimeCatalog,
    PluginToolRegistration,
};

const EXAMPLE_NOOP_TOOL_ID: &str = "example-noop-tool";
const EXAMPLE_NOOP_TOOL_MANIFEST: &str = include_str!("../bundled/example-noop-tool/plugin.json");

#[must_use]
pub fn bundled_plugin_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bundled")
}

#[must_use]
pub fn bundled_runtime_catalog() -> PluginRuntimeCatalog {
    let mut runtimes = PluginRuntimeCatalog::new();
    runtimes
        .register_bundled(EXAMPLE_NOOP_TOOL_ID, Arc::new(NoopPlugin))
        .expect("bundled runtime ids should stay unique");
    runtimes
}

#[must_use]
pub fn bundled_manifest(id: &str) -> Option<PluginManifest> {
    match id {
        EXAMPLE_NOOP_TOOL_ID => Some(example_noop_tool_manifest()),
        _ => None,
    }
}

fn example_noop_tool_manifest() -> PluginManifest {
    let manifest_path = bundled_plugin_root().join("example-noop-tool/plugin.json");
    let mut manifest: PluginManifest = serde_json::from_str(EXAMPLE_NOOP_TOOL_MANIFEST)
        .expect("bundled example manifest should parse");
    manifest.source = PluginSourceTag::Bundled;
    manifest
        .validate(&manifest_path)
        .expect("bundled example manifest should validate");
    manifest
}

fn example_noop_tool_decl() -> ToolDecl {
    example_noop_tool_manifest()
        .components
        .into_iter()
        .find_map(|component| match component {
            PluginComponent::Tool(decl) => Some(decl),
            _ => None,
        })
        .expect("bundled example manifest should include a tool decl")
}

struct NoopPlugin;

impl PluginRuntime for NoopPlugin {
    fn register(&self, api: &mut PluginApi<'_>) -> Result<(), PluginError> {
        api.register_tool(PluginToolRegistration {
            decl: example_noop_tool_decl(),
            tool: Arc::new(NoopTool::new()),
        })
    }
}

struct NoopTool {
    spec: ToolSpec,
}

impl NoopTool {
    fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "noop-tool".into(),
                description: "No-op bundled tool".into(),
                input_schema: serde_json::json!({ "type": "object" }),
                category: octopus_sdk_contracts::ToolCategory::Read,
            },
        }
    }
}

#[async_trait]
impl Tool for NoopTool {
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
        Ok(ToolResult {
            content: vec![ContentBlock::Text {
                text: "bundled noop".into(),
            }],
            ..ToolResult::default()
        })
    }
}
