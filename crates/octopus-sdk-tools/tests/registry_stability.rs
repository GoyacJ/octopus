use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_tools::{
    builtin::register_builtins, RegistryError, Tool, ToolCategory, ToolContext, ToolError,
    ToolRegistry, ToolResult, ToolSpec,
};
use serde_json::json;

struct ExtraTool {
    spec: ToolSpec,
}

impl ExtraTool {
    fn new(name: &str) -> Self {
        Self {
            spec: ToolSpec {
                name: name.into(),
                description: format!("{name} description"),
                input_schema: json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } }
                }),
                category: ToolCategory::Meta,
            },
        }
    }
}

#[async_trait]
impl Tool for ExtraTool {
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
        Ok(ToolResult::default())
    }
}

#[test]
fn registry_schemas_sorted_is_byte_stable() {
    let mut registry = ToolRegistry::new();
    register_builtins(&mut registry).expect("builtins should register");

    let first =
        serde_json::to_string(&registry.schemas_sorted()).expect("schemas should serialize");
    let second =
        serde_json::to_string(&registry.schemas_sorted()).expect("schemas should serialize again");
    let third =
        serde_json::to_string(&registry.schemas_sorted()).expect("schemas should serialize thrice");

    assert_eq!(first, second);
    assert_eq!(second, third);
}

#[test]
fn registry_tools_fingerprint_is_deterministic() {
    let mut registry = ToolRegistry::new();
    register_builtins(&mut registry).expect("builtins should register");

    let first = registry.tools_fingerprint();
    let second = registry.tools_fingerprint();
    let third = registry.tools_fingerprint();

    assert_eq!(first, second);
    assert_eq!(second, third);
}

#[test]
fn registry_fingerprint_diff_after_new_tool() {
    let mut registry = ToolRegistry::new();
    register_builtins(&mut registry).expect("builtins should register");
    let before = registry.tools_fingerprint();

    registry
        .register(Arc::new(ExtraTool::new("extra_tool")))
        .expect("extra tool should register");

    assert_ne!(before, registry.tools_fingerprint());
}

#[test]
fn registry_fingerprint_stable_across_order() {
    let mut left = ToolRegistry::new();
    register_builtins(&mut left).expect("left builtins should register");

    let mut right = ToolRegistry::new();
    let mut builtins = ToolRegistry::new();
    register_builtins(&mut builtins).expect("source builtins should register");
    let mut ordered = builtins
        .iter()
        .map(|(_, tool)| Arc::clone(tool))
        .collect::<Vec<_>>();
    ordered.reverse();
    for tool in ordered {
        right
            .register(tool)
            .expect("tool should register in reverse order");
    }

    assert_eq!(left.tools_fingerprint(), right.tools_fingerprint());
}

#[test]
fn register_builtins_rejects_duplicates() {
    let mut registry = ToolRegistry::new();
    register_builtins(&mut registry).expect("first registration should succeed");
    let error = register_builtins(&mut registry).expect_err("second registration should fail");

    assert!(matches!(error, RegistryError::DuplicateName(_)));
}
