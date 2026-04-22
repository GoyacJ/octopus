use std::{collections::BTreeMap, sync::Arc};

use async_trait::async_trait;
use futures::stream;
use octopus_sdk_contracts::{
    AskAnswer, AskError, AskPrompt, AskResolver, EventId, EventSink, PermissionGate,
    PermissionOutcome, SecretVault, SessionEvent, SessionId, ToolCallRequest, VaultError,
};
use octopus_sdk_mcp::{McpError, McpTool, McpToolResult, ToolDirectory};
use octopus_sdk_session::{EventRange, EventStream, SessionError, SessionSnapshot, SessionStore};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use tokio_util::sync::CancellationToken;

use crate::{RegistryError, SandboxHandle, Tool, ToolContext, ToolSpec};

#[derive(Clone)]
pub struct ToolRegistry {
    tools: BTreeMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result<(), RegistryError> {
        let name = tool.spec().name.clone();
        if self.tools.contains_key(name.as_str()) {
            return Err(RegistryError::DuplicateName(name));
        }

        self.tools.insert(name, tool);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Arc<dyn Tool>)> {
        self.tools.iter().map(|(name, tool)| (name.as_str(), tool))
    }

    #[must_use]
    pub fn as_directory(&self) -> Arc<dyn ToolDirectory> {
        Arc::new(self.clone())
    }

    #[must_use]
    pub fn schemas_sorted(&self) -> Vec<&ToolSpec> {
        let mut specs = self
            .tools
            .values()
            .map(|tool| tool.spec())
            .collect::<Vec<_>>();
        specs.sort_by(|left, right| {
            (left.category.category_priority(), left.name.as_str())
                .cmp(&(right.category.category_priority(), right.name.as_str()))
        });
        specs
    }

    #[must_use]
    pub fn tools_fingerprint(&self) -> String {
        let joined = self
            .schemas_sorted()
            .into_iter()
            .map(|spec| {
                format!(
                    "{}\0{}\0{}",
                    spec.name,
                    spec.category.category_priority(),
                    canonical_json_string(&spec.input_schema)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("{:x}", Sha256::digest(joined.as_bytes()))
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ToolDirectory for ToolRegistry {
    fn list_tools(&self) -> Vec<McpTool> {
        self.schemas_sorted()
            .into_iter()
            .map(|spec| spec.to_mcp().into())
            .collect()
    }

    async fn call_tool(
        &self,
        name: &str,
        input: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        let Some(tool) = self.get(name) else {
            return Err(McpError::ToolNotFound {
                name: name.to_string(),
            });
        };

        let ctx = shim_tool_context();
        let result = match tool.execute(ctx, input).await {
            Ok(result) => result,
            Err(error) => error.as_tool_result(),
        };

        Ok(McpToolResult {
            content: result.content,
            is_error: result.is_error,
        })
    }
}

fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonicalize_value(value)).expect("canonical json should serialize")
}

fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize_value).collect()),
        Value::Object(entries) => {
            let mut sorted = Map::new();
            for (key, value) in entries.iter().collect::<BTreeMap<_, _>>() {
                sorted.insert(key.clone(), canonicalize_value(value));
            }
            Value::Object(sorted)
        }
        _ => value.clone(),
    }
}

fn shim_tool_context() -> ToolContext {
    let working_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());

    ToolContext {
        session_id: SessionId("sdk-shim".into()),
        permissions: Arc::new(AllowAllPermissionGate),
        sandbox: SandboxHandle::new(working_dir.clone(), vec!["PATH".into()], "noop"),
        session_store: Arc::new(NoopSessionStore),
        secret_vault: Arc::new(NoopSecretVault),
        ask_resolver: Arc::new(NoopAskResolver),
        event_sink: Arc::new(NoopEventSink),
        working_dir,
        cancellation: CancellationToken::new(),
    }
}

struct AllowAllPermissionGate;
struct NoopAskResolver;
struct NoopEventSink;
struct NoopSecretVault;
struct NoopSessionStore;

#[async_trait]
impl PermissionGate for AllowAllPermissionGate {
    async fn check(&self, _call: &ToolCallRequest) -> PermissionOutcome {
        PermissionOutcome::Allow
    }
}

#[async_trait]
impl AskResolver for NoopAskResolver {
    async fn resolve(&self, prompt_id: &str, _prompt: &AskPrompt) -> Result<AskAnswer, AskError> {
        Err(AskError::NotResolvable).map_err(|_| {
            let _ = prompt_id;
            AskError::NotResolvable
        })
    }
}

impl EventSink for NoopEventSink {
    fn emit(&self, _event: SessionEvent) {}
}

#[async_trait]
impl SecretVault for NoopSecretVault {
    async fn get(&self, _ref_id: &str) -> Result<octopus_sdk_contracts::SecretValue, VaultError> {
        Err(VaultError::NotFound)
    }

    async fn put(
        &self,
        _ref_id: &str,
        _value: octopus_sdk_contracts::SecretValue,
    ) -> Result<(), VaultError> {
        Ok(())
    }
}

#[async_trait]
impl SessionStore for NoopSessionStore {
    async fn append(&self, _id: &SessionId, _event: SessionEvent) -> Result<EventId, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn append_session_started(
        &self,
        _id: &SessionId,
        _working_dir: std::path::PathBuf,
        _permission_mode: octopus_sdk_contracts::PermissionMode,
        _model: String,
        _config_snapshot_id: String,
        _effective_config_hash: String,
        _token_budget: u32,
        _plugins_snapshot: Option<octopus_sdk_contracts::PluginsSnapshot>,
    ) -> Result<EventId, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn new_child_session(
        &self,
        _parent_id: &SessionId,
        _spec: &octopus_sdk_contracts::SubagentSpec,
    ) -> Result<SessionId, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn stream(
        &self,
        _id: &SessionId,
        _range: EventRange,
    ) -> Result<EventStream, SessionError> {
        Ok(Box::pin(stream::empty()))
    }

    async fn snapshot(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn fork(&self, _id: &SessionId, _from: EventId) -> Result<SessionId, SessionError> {
        Err(SessionError::NotFound)
    }

    async fn wake(&self, _id: &SessionId) -> Result<SessionSnapshot, SessionError> {
        Err(SessionError::NotFound)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;

    use super::ToolRegistry;
    use crate::{RegistryError, Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

    struct DummyTool {
        spec: ToolSpec,
    }

    impl DummyTool {
        fn new(name: &str, category: ToolCategory, input_schema: serde_json::Value) -> Self {
            Self {
                spec: ToolSpec {
                    name: name.into(),
                    description: format!("{name} description"),
                    input_schema,
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
            Ok(ToolResult::default())
        }
    }

    #[test]
    fn registry_rejects_duplicate_names() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(DummyTool::new(
                "grep",
                ToolCategory::Read,
                json!({ "type": "object" }),
            )))
            .expect("first registration should succeed");

        let error = registry
            .register(Arc::new(DummyTool::new(
                "grep",
                ToolCategory::Read,
                json!({ "type": "object" }),
            )))
            .expect_err("duplicate registration should fail");

        assert_eq!(error, RegistryError::DuplicateName("grep".into()));
    }

    #[test]
    fn registry_sorts_by_category_then_name() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(DummyTool::new(
                "write_file",
                ToolCategory::Write,
                json!({ "type": "object" }),
            )))
            .expect("write tool should register");
        registry
            .register(Arc::new(DummyTool::new(
                "bash",
                ToolCategory::Shell,
                json!({ "type": "object" }),
            )))
            .expect("shell tool should register");
        registry
            .register(Arc::new(DummyTool::new(
                "grep",
                ToolCategory::Read,
                json!({ "type": "object" }),
            )))
            .expect("read tool should register");
        registry
            .register(Arc::new(DummyTool::new(
                "glob",
                ToolCategory::Read,
                json!({ "type": "object" }),
            )))
            .expect("read tool should register");

        let names = registry
            .schemas_sorted()
            .into_iter()
            .map(|spec| spec.name.as_str())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["glob", "grep", "write_file", "bash"]);
    }

    #[test]
    fn registry_stability_byte_equal() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(DummyTool::new(
                "search",
                ToolCategory::Read,
                json!({
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" },
                        "limit": { "type": "integer" }
                    }
                }),
            )))
            .expect("search should register");
        registry
            .register(Arc::new(DummyTool::new(
                "bash",
                ToolCategory::Shell,
                json!({
                    "properties": {
                        "yield_time_ms": { "type": "integer" },
                        "cmd": { "type": "string" }
                    },
                    "type": "object"
                }),
            )))
            .expect("bash should register");

        let first =
            serde_json::to_string(&registry.schemas_sorted()).expect("schemas should serialize");
        let second =
            serde_json::to_string(&registry.schemas_sorted()).expect("schemas should serialize");
        let third =
            serde_json::to_string(&registry.schemas_sorted()).expect("schemas should serialize");

        assert_eq!(first, second);
        assert_eq!(second, third);
    }

    #[test]
    fn registry_fingerprint_changes_on_new_tool() {
        let mut registry = ToolRegistry::new();
        registry
            .register(Arc::new(DummyTool::new(
                "grep",
                ToolCategory::Read,
                json!({ "type": "object" }),
            )))
            .expect("grep should register");

        let before = registry.tools_fingerprint();

        registry
            .register(Arc::new(DummyTool::new(
                "bash",
                ToolCategory::Shell,
                json!({ "type": "object" }),
            )))
            .expect("bash should register");

        let after = registry.tools_fingerprint();

        assert_ne!(before, after);
    }

    #[test]
    fn registry_fingerprint_stable_on_reorder() {
        let read_tool = Arc::new(DummyTool::new(
            "grep",
            ToolCategory::Read,
            json!({
                "properties": {
                    "path": { "type": "string" },
                    "pattern": { "type": "string" }
                },
                "type": "object"
            }),
        ));
        let shell_tool = Arc::new(DummyTool::new(
            "bash",
            ToolCategory::Shell,
            json!({
                "type": "object",
                "properties": {
                    "yield_time_ms": { "type": "integer" },
                    "cmd": { "type": "string" }
                }
            }),
        ));

        let mut left = ToolRegistry::new();
        left.register(read_tool.clone())
            .expect("grep should register");
        left.register(shell_tool.clone())
            .expect("bash should register");

        let mut right = ToolRegistry::new();
        right.register(shell_tool).expect("bash should register");
        right.register(read_tool).expect("grep should register");

        assert_eq!(left.tools_fingerprint(), right.tools_fingerprint());
    }
}
