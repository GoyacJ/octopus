use std::{path::Path, sync::Arc};

use async_trait::async_trait;
use octopus_sdk_contracts::{ContentBlock, PluginSourceTag, ToolDecl};
use octopus_sdk_tools::{Tool, ToolContext, ToolError, ToolResult, ToolSpec};

use crate::{
    Plugin, PluginApi, PluginComponent, PluginError, PluginManifest, PluginToolRegistration,
};

const EXAMPLE_NOOP_TOOL_MANIFEST: &str = include_str!("../bundled/example-noop-tool/plugin.json");

#[must_use]
pub fn example_bundled_plugins() -> Vec<Box<dyn Plugin>> {
    vec![Box::new(NoopPlugin::new())]
}

struct NoopPlugin {
    manifest: PluginManifest,
}

impl NoopPlugin {
    fn new() -> Self {
        let manifest_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("bundled/example-noop-tool/plugin.json");
        let manifest = serde_json::from_str::<PluginManifest>(EXAMPLE_NOOP_TOOL_MANIFEST)
            .expect("bundled example manifest should parse");
        manifest
            .validate(&manifest_path)
            .expect("bundled example manifest should validate");

        Self { manifest }
    }

    fn tool_decl(&self) -> ToolDecl {
        self.manifest
            .components
            .iter()
            .find_map(|component| match component {
                PluginComponent::Tool(decl) => Some(decl.clone()),
                _ => None,
            })
            .expect("bundled example manifest should include a tool decl")
    }
}

impl Plugin for NoopPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn source(&self) -> PluginSourceTag {
        PluginSourceTag::Bundled
    }

    fn register(&self, api: &mut PluginApi<'_>) -> Result<(), PluginError> {
        api.register_tool(PluginToolRegistration {
            decl: self.tool_decl(),
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
