#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

use api::ToolDefinition;
use plugins::PluginTool;
use runtime::{
    permission_enforcer::PermissionEnforcer, McpDegradedReport, McpServerManager, PermissionMode,
    RuntimeConfig, ToolError, ToolExecutionOutcome,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::builtin_exec::execute_tool_with_enforcer;
use crate::tool_registry::{
    mvp_tool_specs, normalize_tool_search_query, search_tool_specs, RuntimeToolDefinition,
    SearchableToolSpec, ToolSearchOutput, ToolSpec,
};

use super::events::{
    CapabilityExecutionEvent, CapabilityExecutionRequest, CapabilityMediationDecision,
};
use super::executor::{CapabilityDispatchKind, CapabilityExecutor};
use super::planner::{
    concurrency_policy, plan_effective_capability_surface, CapabilityCompilationInput,
    CapabilityCompiler, CapabilityExecutionPlan, CapabilityPlanner, CapabilityPlannerInput,
};
use super::state::{CapabilitySurfaceProjection, SessionCapabilityStore};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySourceKind {
    Builtin,
    RuntimeTool,
    PluginTool,
    LocalSkill,
    BundledSkill,
    McpTool,
    McpPrompt,
    McpResource,
    PluginSkill,
}

impl CapabilitySourceKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::RuntimeTool => "runtime_tool",
            Self::PluginTool => "plugin_tool",
            Self::LocalSkill => "local_skill",
            Self::BundledSkill => "bundled_skill",
            Self::McpTool => "mcp_tool",
            Self::McpPrompt => "mcp_prompt",
            Self::McpResource => "mcp_resource",
            Self::PluginSkill => "plugin_skill",
        }
    }
}

impl fmt::Display for CapabilitySourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityExecutionKind {
    Tool,
    PromptSkill,
    Resource,
}

impl CapabilityExecutionKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tool => "tool",
            Self::PromptSkill => "prompt_skill",
            Self::Resource => "resource",
        }
    }
}

impl fmt::Display for CapabilityExecutionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityVisibility {
    DefaultVisible,
    Deferred,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Ready,
    Pending,
    AuthRequired,
    ApprovalRequired,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityConcurrencyPolicy {
    ParallelRead,
    Serialized,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPermissionProfile {
    pub required_permission: PermissionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityInvocationPolicy {
    pub selectable: bool,
    pub requires_approval: bool,
    pub requires_auth: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CapabilityTrustProfile {
    pub requires_trusted_workspace: bool,
    pub requires_explicit_user_trust: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CapabilityScopeConstraints {
    pub workspace_only: bool,
    pub requires_current_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityHandle {
    pub capability_id: String,
    pub source_kind: CapabilitySourceKind,
    pub execution_kind: CapabilityExecutionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executor_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilitySpec {
    pub capability_id: String,
    pub source_kind: CapabilitySourceKind,
    pub execution_kind: CapabilityExecutionKind,
    pub display_name: String,
    pub description: String,
    pub when_to_use: Option<String>,
    pub input_schema: Value,
    pub search_hint: Option<String>,
    pub visibility: CapabilityVisibility,
    pub state: CapabilityState,
    pub permission_profile: CapabilityPermissionProfile,
    pub trust_profile: CapabilityTrustProfile,
    pub scope_constraints: CapabilityScopeConstraints,
    pub invocation_policy: CapabilityInvocationPolicy,
    pub concurrency_policy: CapabilityConcurrencyPolicy,
    pub provider_key: Option<String>,
    pub executor_key: Option<String>,
}

impl CapabilitySpec {
    #[must_use]
    pub fn handle(&self) -> CapabilityHandle {
        CapabilityHandle {
            capability_id: self.capability_id.clone(),
            source_kind: self.source_kind,
            execution_kind: self.execution_kind,
            provider_key: self.provider_key.clone(),
            executor_key: self.executor_key.clone(),
        }
    }

    #[must_use]
    pub fn to_tool_definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.display_name.clone(),
            description: Some(self.description.clone()),
            input_schema: self.input_schema.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilitySurface {
    pub visible_tools: Vec<CapabilitySpec>,
    pub deferred_tools: Vec<CapabilitySpec>,
    pub discoverable_skills: Vec<CapabilitySpec>,
    pub available_resources: Vec<CapabilitySpec>,
    pub hidden_capabilities: Vec<CapabilitySpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpConnectionProjection {
    pub server_name: String,
    pub state: CapabilityState,
    pub status_detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct McpCapabilityDescriptor {
    pub capability_id: String,
    pub source_kind: CapabilitySourceKind,
    pub execution_kind: CapabilityExecutionKind,
    pub provider_key: Option<String>,
    pub executor_key: Option<String>,
    pub display_name: String,
    pub description: String,
    pub when_to_use: Option<String>,
    pub input_schema: Value,
    pub search_hint: Option<String>,
    pub visibility: CapabilityVisibility,
    pub state: CapabilityState,
    pub required_permission: PermissionMode,
    pub requires_auth: bool,
    pub requires_approval: bool,
    pub trust_profile: CapabilityTrustProfile,
    pub scope_constraints: CapabilityScopeConstraints,
}

impl McpCapabilityDescriptor {
    #[must_use]
    pub fn to_capability_spec(&self) -> CapabilitySpec {
        CapabilitySpec {
            capability_id: self.capability_id.clone(),
            source_kind: self.source_kind,
            execution_kind: self.execution_kind,
            provider_key: self.provider_key.clone(),
            executor_key: self.executor_key.clone(),
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            when_to_use: self.when_to_use.clone(),
            input_schema: self.input_schema.clone(),
            search_hint: self.search_hint.clone(),
            visibility: self.visibility,
            state: self.state,
            permission_profile: CapabilityPermissionProfile {
                required_permission: self.required_permission,
            },
            trust_profile: self.trust_profile.clone(),
            scope_constraints: self.scope_constraints.clone(),
            invocation_policy: CapabilityInvocationPolicy {
                selectable: true,
                requires_approval: self.requires_approval,
                requires_auth: self.requires_auth,
            },
            concurrency_policy: concurrency_policy(self.required_permission),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct McpCapabilityProvider {
    descriptors: Vec<McpCapabilityDescriptor>,
    connections: Vec<McpConnectionProjection>,
}

impl McpCapabilityProvider {
    #[must_use]
    pub fn new(
        descriptors: Vec<McpCapabilityDescriptor>,
        connections: Vec<McpConnectionProjection>,
    ) -> Self {
        Self {
            descriptors,
            connections,
        }
    }

    #[must_use]
    pub fn capabilities(&self) -> Vec<CapabilitySpec> {
        self.descriptors
            .iter()
            .map(McpCapabilityDescriptor::to_capability_spec)
            .collect()
    }

    #[must_use]
    pub fn descriptors(&self) -> &[McpCapabilityDescriptor] {
        &self.descriptors
    }

    #[must_use]
    pub fn connections(&self) -> &[McpConnectionProjection] {
        &self.connections
    }
}

#[derive(Debug)]
pub struct ManagedMcpRuntime {
    runtime: Option<tokio::runtime::Runtime>,
    manager: McpServerManager,
    pending_servers: Vec<String>,
    degraded_report: Option<McpDegradedReport>,
    capability_provider: McpCapabilityProvider,
    tool_capability_names: BTreeSet<String>,
    prompt_routes: BTreeMap<String, (String, String)>,
    resource_routes: BTreeMap<String, (String, String)>,
}

impl ManagedMcpRuntime {
    pub fn new(runtime_config: &RuntimeConfig) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let mut manager = McpServerManager::from_runtime_config(runtime_config);
        if manager.server_names().is_empty() && manager.unsupported_servers().is_empty() {
            return Ok(None);
        }

        let runtime = tokio::runtime::Runtime::new()?;
        let discovery = runtime.block_on(manager.discover_tools_best_effort());
        let pending_servers = discovery
            .failed_servers
            .iter()
            .map(|failure| failure.server_name.clone())
            .chain(
                discovery
                    .unsupported_servers
                    .iter()
                    .map(|server| server.server_name.clone()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let available_tools = discovery
            .tools
            .iter()
            .map(|tool| tool.qualified_name.clone())
            .collect::<Vec<_>>();
        let failed_server_names = pending_servers.iter().cloned().collect::<BTreeSet<_>>();
        let working_servers = manager
            .server_names()
            .into_iter()
            .filter(|server_name| !failed_server_names.contains(server_name))
            .collect::<Vec<_>>();
        let mut failed_servers =
            discovery
                .failed_servers
                .iter()
                .map(|failure| runtime::McpFailedServer {
                    server_name: failure.server_name.clone(),
                    phase: runtime::McpLifecyclePhase::ToolDiscovery,
                    error: runtime::McpErrorSurface::new(
                        runtime::McpLifecyclePhase::ToolDiscovery,
                        Some(failure.server_name.clone()),
                        failure.error.clone(),
                        std::collections::BTreeMap::new(),
                        true,
                    ),
                })
                .chain(discovery.unsupported_servers.iter().map(|server| {
                    runtime::McpFailedServer {
                        server_name: server.server_name.clone(),
                        phase: runtime::McpLifecyclePhase::ServerRegistration,
                        error: runtime::McpErrorSurface::new(
                            runtime::McpLifecyclePhase::ServerRegistration,
                            Some(server.server_name.clone()),
                            server.reason.clone(),
                            std::collections::BTreeMap::from([(
                                "transport".to_string(),
                                format!("{:?}", server.transport).to_ascii_lowercase(),
                            )]),
                            false,
                        ),
                    }
                }))
                .collect::<Vec<_>>();
        let mut listed_prompts = Vec::new();
        let mut listed_resources = Vec::new();
        let mut connection_projections = working_servers
            .iter()
            .map(|server_name| McpConnectionProjection {
                server_name: server_name.clone(),
                state: CapabilityState::Ready,
                status_detail: None,
            })
            .collect::<Vec<_>>();
        for server_name in &working_servers {
            match runtime.block_on(manager.discover_prompts_for_server(server_name)) {
                Ok(prompts) => {
                    listed_prompts.extend(prompts);
                }
                Err(error) => {
                    connection_projections
                        .retain(|projection| projection.server_name != *server_name);
                    connection_projections.push(McpConnectionProjection {
                        server_name: server_name.clone(),
                        state: CapabilityState::Degraded,
                        status_detail: Some(error.to_string()),
                    });
                    failed_servers.push(runtime::McpFailedServer {
                        server_name: server_name.clone(),
                        phase: runtime::McpLifecyclePhase::ToolDiscovery,
                        error: runtime::McpErrorSurface::new(
                            runtime::McpLifecyclePhase::ToolDiscovery,
                            Some(server_name.clone()),
                            error.to_string(),
                            std::collections::BTreeMap::new(),
                            true,
                        ),
                    });
                }
            }
            match runtime.block_on(manager.list_resources(server_name)) {
                Ok(result) => {
                    listed_resources.extend(
                        result
                            .resources
                            .into_iter()
                            .map(|resource| (server_name.clone(), resource)),
                    );
                }
                Err(error) => {
                    connection_projections
                        .retain(|projection| projection.server_name != *server_name);
                    connection_projections.push(McpConnectionProjection {
                        server_name: server_name.clone(),
                        state: CapabilityState::Degraded,
                        status_detail: Some(error.to_string()),
                    });
                    failed_servers.push(runtime::McpFailedServer {
                        server_name: server_name.clone(),
                        phase: runtime::McpLifecyclePhase::ResourceDiscovery,
                        error: runtime::McpErrorSurface::new(
                            runtime::McpLifecyclePhase::ResourceDiscovery,
                            Some(server_name.clone()),
                            error.to_string(),
                            std::collections::BTreeMap::new(),
                            true,
                        ),
                    });
                }
            }
        }
        connection_projections.extend(discovery.failed_servers.iter().map(|failure| {
            McpConnectionProjection {
                server_name: failure.server_name.clone(),
                state: CapabilityState::Degraded,
                status_detail: Some(failure.error.clone()),
            }
        }));
        connection_projections.extend(discovery.unsupported_servers.iter().map(|server| {
            McpConnectionProjection {
                server_name: server.server_name.clone(),
                state: CapabilityState::Unavailable,
                status_detail: Some(server.reason.clone()),
            }
        }));
        let degraded_report = (!failed_servers.is_empty()).then(|| {
            runtime::McpDegradedReport::new(
                working_servers.clone(),
                failed_servers,
                available_tools.clone(),
                available_tools,
            )
        });
        let mut capability_descriptors = discovery
            .tools
            .iter()
            .map(mcp_tool_capability_descriptor)
            .collect::<Vec<_>>();
        capability_descriptors.extend(listed_prompts.iter().map(mcp_prompt_capability_descriptor));
        capability_descriptors.extend(listed_resources.iter().map(|(server_name, resource)| {
            mcp_resource_capability_descriptor(server_name, resource)
        }));
        let capability_provider =
            McpCapabilityProvider::new(capability_descriptors, connection_projections);
        let tool_capability_names = discovery
            .tools
            .iter()
            .map(|tool| tool.qualified_name.clone())
            .collect::<BTreeSet<_>>();
        let prompt_routes = listed_prompts
            .iter()
            .map(|prompt| {
                (
                    prompt.qualified_name.clone(),
                    (prompt.server_name.clone(), prompt.raw_name.clone()),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let resource_routes = listed_resources
            .iter()
            .map(|(server_name, resource)| {
                let capability_id =
                    mcp_resource_capability_descriptor(server_name, resource).capability_id;
                (capability_id, (server_name.clone(), resource.uri.clone()))
            })
            .collect::<BTreeMap<_, _>>();

        Ok(Some(Self {
            runtime: Some(runtime),
            manager,
            pending_servers,
            degraded_report,
            capability_provider,
            tool_capability_names,
            prompt_routes,
            resource_routes,
        }))
    }

    pub fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.with_runtime_block_on(|runtime, manager| runtime.block_on(manager.shutdown()))?;
        Ok(())
    }

    #[must_use]
    pub fn pending_servers(&self) -> Option<Vec<String>> {
        (!self.pending_servers.is_empty()).then(|| self.pending_servers.clone())
    }

    #[must_use]
    pub fn degraded_report(&self) -> Option<McpDegradedReport> {
        self.degraded_report.clone()
    }

    #[must_use]
    pub fn provided_capabilities(&self) -> Vec<CapabilitySpec> {
        self.capability_provider.capabilities()
    }

    #[must_use]
    pub fn connection_projections(&self) -> &[McpConnectionProjection] {
        self.capability_provider.connections()
    }

    #[must_use]
    pub fn has_tool_capability(&self, tool_name: &str) -> bool {
        self.tool_capability_names.contains(tool_name)
    }

    pub fn execute_tool(&mut self, tool_name: &str, value: Value) -> Result<String, ToolError> {
        self.call_tool(tool_name, Some(value))
    }

    pub fn execute_prompt_skill(
        &mut self,
        capability: &CapabilitySpec,
        arguments: Option<Value>,
    ) -> Result<crate::SkillExecutionResult, String> {
        let executor_key = capability.executor_key.as_ref().ok_or_else(|| {
            format!(
                "prompt `{}` does not have a runtime executor",
                capability.display_name
            )
        })?;
        let (server_name, raw_name) =
            self.prompt_routes
                .get(executor_key)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "prompt `{}` is not registered with the MCP runtime",
                        capability.display_name
                    )
                })?;
        let prompt = self
            .with_runtime_block_on(|runtime, manager| {
                runtime.block_on(manager.get_prompt(&server_name, &raw_name, arguments))
            })
            .map_err(|error| error.to_string())?;
        let rendered_prompt = render_mcp_prompt_messages(&prompt.messages);
        Ok(crate::SkillExecutionResult {
            skill: capability.display_name.clone(),
            path: format!("mcp://{server_name}/{raw_name}"),
            description: prompt
                .description
                .or_else(|| Some(capability.description.clone())),
            context: crate::skill_runtime::SkillContextKind::Inline,
            messages_to_inject: vec![crate::skill_runtime::SkillInjectedMessage::system(
                rendered_prompt,
            )],
            tool_grants: Vec::new(),
            model_override: None,
            effort_override: None,
            state_updates: vec![
                crate::SkillStateUpdate::ContextPrepared {
                    context: crate::skill_runtime::SkillContextKind::Inline,
                },
                crate::SkillStateUpdate::MessageInjected {
                    role: "system".to_string(),
                },
            ],
        })
    }

    pub fn read_resource_capability(
        &mut self,
        capability: &CapabilitySpec,
    ) -> Result<String, String> {
        let executor_key = capability.executor_key.as_ref().ok_or_else(|| {
            format!(
                "resource `{}` does not have a runtime executor",
                capability.display_name
            )
        })?;
        let (server_name, uri) =
            self.resource_routes
                .get(executor_key)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "resource `{}` is not registered with the MCP runtime",
                        capability.display_name
                    )
                })?;
        let result = self
            .with_runtime_block_on(|runtime, manager| {
                runtime.block_on(manager.read_resource(&server_name, &uri))
            })
            .map_err(|error| error.to_string())?;
        serde_json::to_string_pretty(&result).map_err(|error| error.to_string())
    }

    fn server_names(&self) -> Vec<String> {
        self.manager.server_names()
    }

    fn call_tool(
        &mut self,
        qualified_tool_name: &str,
        arguments: Option<Value>,
    ) -> Result<String, ToolError> {
        let response = self
            .with_runtime_block_on(|runtime, manager| {
                runtime.block_on(manager.call_tool(qualified_tool_name, arguments))
            })
            .map_err(|error| ToolError::new(error.to_string()))?;
        if let Some(error) = response.error {
            return Err(ToolError::new(format!(
                "MCP tool `{qualified_tool_name}` returned JSON-RPC error: {} ({})",
                error.message, error.code
            )));
        }

        let result = response.result.ok_or_else(|| {
            ToolError::new(format!(
                "MCP tool `{qualified_tool_name}` returned no result payload"
            ))
        })?;
        serde_json::to_string_pretty(&result).map_err(|error| ToolError::new(error.to_string()))
    }

    fn runtime(&self) -> &tokio::runtime::Runtime {
        self.runtime
            .as_ref()
            .expect("managed MCP runtime is unavailable")
    }

    fn with_runtime_block_on<T, E>(
        &mut self,
        f: impl FnOnce(&tokio::runtime::Runtime, &mut McpServerManager) -> Result<T, E>,
    ) -> Result<T, E> {
        let runtime = self
            .runtime
            .take()
            .expect("managed MCP runtime is unavailable");
        let result = f(&runtime, &mut self.manager);
        self.runtime = Some(runtime);
        result
    }
}

impl Drop for ManagedMcpRuntime {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}

pub fn mcp_tool_capability_descriptor(tool: &runtime::ManagedMcpTool) -> McpCapabilityDescriptor {
    McpCapabilityDescriptor {
        capability_id: tool.qualified_name.clone(),
        source_kind: CapabilitySourceKind::McpTool,
        execution_kind: CapabilityExecutionKind::Tool,
        provider_key: Some(tool.server_name.clone()),
        executor_key: Some(tool.qualified_name.clone()),
        display_name: tool.qualified_name.clone(),
        description: tool
            .tool
            .description
            .clone()
            .unwrap_or_else(|| format!("Invoke MCP tool `{}`.", tool.qualified_name)),
        when_to_use: None,
        input_schema: tool.tool.input_schema.clone().unwrap_or_else(
            || serde_json::json!({ "type": "object", "additionalProperties": true }),
        ),
        search_hint: tool.tool.description.clone(),
        visibility: CapabilityVisibility::Deferred,
        state: CapabilityState::Ready,
        required_permission: permission_mode_for_mcp_tool(&tool.tool),
        requires_auth: false,
        requires_approval: permission_mode_for_mcp_tool(&tool.tool)
            == PermissionMode::DangerFullAccess,
        trust_profile: CapabilityTrustProfile::default(),
        scope_constraints: CapabilityScopeConstraints::default(),
    }
}

pub fn mcp_prompt_capability_descriptor(
    prompt: &runtime::ManagedMcpPrompt,
) -> McpCapabilityDescriptor {
    McpCapabilityDescriptor {
        capability_id: prompt.qualified_name.clone(),
        source_kind: CapabilitySourceKind::McpPrompt,
        execution_kind: CapabilityExecutionKind::PromptSkill,
        provider_key: Some(prompt.server_name.clone()),
        executor_key: Some(prompt.qualified_name.clone()),
        display_name: prompt.raw_name.clone(),
        description: prompt
            .prompt
            .description
            .clone()
            .unwrap_or_else(|| format!("Execute MCP prompt `{}`.", prompt.raw_name)),
        when_to_use: prompt.prompt.description.clone(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "skill": { "type": "string", "const": prompt.raw_name },
                "arguments": { "type": "object" }
            },
            "required": ["skill"],
            "additionalProperties": false
        }),
        search_hint: prompt.prompt.description.clone(),
        visibility: CapabilityVisibility::DefaultVisible,
        state: CapabilityState::Ready,
        required_permission: PermissionMode::ReadOnly,
        requires_auth: false,
        requires_approval: false,
        trust_profile: CapabilityTrustProfile::default(),
        scope_constraints: CapabilityScopeConstraints::default(),
    }
}

pub fn mcp_resource_capability_descriptor(
    server_name: &str,
    resource: &runtime::McpResource,
) -> McpCapabilityDescriptor {
    let display_name = format!(
        "mcp_resource__{}__{}",
        server_name,
        resource
            .uri
            .replace(|ch: char| !ch.is_ascii_alphanumeric(), "_")
    );
    let description = resource
        .description
        .clone()
        .or_else(|| resource.name.clone())
        .unwrap_or_else(|| {
            format!(
                "Read MCP resource `{}` from server `{server_name}`.",
                resource.uri
            )
        });
    McpCapabilityDescriptor {
        capability_id: display_name.clone(),
        source_kind: CapabilitySourceKind::McpResource,
        execution_kind: CapabilityExecutionKind::Resource,
        provider_key: Some(server_name.to_string()),
        executor_key: Some(display_name.clone()),
        display_name,
        description,
        when_to_use: None,
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "server": { "type": "string", "const": server_name },
                "uri": { "type": "string", "const": resource.uri }
            },
            "required": ["server", "uri"],
            "additionalProperties": false
        }),
        search_hint: resource
            .description
            .clone()
            .or_else(|| resource.name.clone()),
        visibility: CapabilityVisibility::DefaultVisible,
        state: CapabilityState::Ready,
        required_permission: PermissionMode::ReadOnly,
        requires_auth: false,
        requires_approval: false,
        trust_profile: CapabilityTrustProfile::default(),
        scope_constraints: CapabilityScopeConstraints::default(),
    }
}

pub fn permission_mode_for_mcp_tool(tool: &runtime::McpTool) -> PermissionMode {
    let read_only = mcp_annotation_flag(tool, "readOnlyHint");
    let destructive = mcp_annotation_flag(tool, "destructiveHint");
    let open_world = mcp_annotation_flag(tool, "openWorldHint");

    if read_only && !destructive && !open_world {
        PermissionMode::ReadOnly
    } else if destructive || open_world {
        PermissionMode::DangerFullAccess
    } else {
        PermissionMode::WorkspaceWrite
    }
}

fn mcp_annotation_flag(tool: &runtime::McpTool, key: &str) -> bool {
    tool.annotations
        .as_ref()
        .and_then(|annotations| annotations.get(key))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn render_mcp_prompt_messages(messages: &[runtime::McpPromptMessage]) -> String {
    let rendered = messages
        .iter()
        .map(|message| {
            let text = message
                .content
                .get("text")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| message.content.to_string());
            format!("{}:\n{}", message.role, text)
        })
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        "MCP prompt returned no messages.".to_string()
    } else {
        rendered.join("\n\n")
    }
}

#[derive(Debug, Clone)]
pub struct CapabilityProvider {
    builtin_specs: Vec<ToolSpec>,
    runtime_tools: Vec<RuntimeToolDefinition>,
    plugin_tools: Vec<PluginTool>,
    provided_capabilities: Vec<CapabilitySpec>,
    enforcer: Option<PermissionEnforcer>,
}

impl CapabilityProvider {
    #[must_use]
    pub fn builtin() -> Self {
        Self::new(mvp_tool_specs(), Vec::new(), Vec::new(), Vec::new(), None)
    }

    pub fn from_sources_checked(
        plugin_tools: Vec<PluginTool>,
        runtime_tools: Vec<RuntimeToolDefinition>,
        provided_capabilities: Vec<CapabilitySpec>,
        enforcer: Option<PermissionEnforcer>,
    ) -> Result<Self, String> {
        let builtin_names = mvp_tool_specs()
            .into_iter()
            .map(|spec| spec.name.to_string())
            .collect::<BTreeSet<_>>();
        let mut seen_plugin_names = BTreeSet::new();

        for tool in &plugin_tools {
            let name = tool.definition().name.clone();
            if builtin_names.contains(&name) {
                return Err(format!(
                    "plugin tool `{name}` conflicts with a built-in tool name"
                ));
            }
            if !seen_plugin_names.insert(name.clone()) {
                return Err(format!("duplicate plugin tool name `{name}`"));
            }
        }

        let mut seen_runtime_names = builtin_names
            .iter()
            .cloned()
            .chain(
                plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .chain(
                provided_capabilities
                    .iter()
                    .filter(|capability| capability.execution_kind == CapabilityExecutionKind::Tool)
                    .map(|capability| capability.display_name.clone()),
            )
            .collect::<BTreeSet<_>>();

        for tool in &runtime_tools {
            if !seen_runtime_names.insert(tool.name.clone()) {
                return Err(format!(
                    "runtime tool `{}` conflicts with an existing tool name",
                    tool.name
                ));
            }
        }

        let mut seen_provided_names = builtin_names
            .into_iter()
            .chain(
                plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .chain(runtime_tools.iter().map(|tool| tool.name.clone()))
            .collect::<BTreeSet<_>>();

        for capability in &provided_capabilities {
            if capability.execution_kind != CapabilityExecutionKind::Tool {
                continue;
            }

            if !seen_provided_names.insert(capability.display_name.clone()) {
                return Err(format!(
                    "provided capability `{}` conflicts with an existing tool name",
                    capability.display_name
                ));
            }
        }

        Ok(Self::new(
            mvp_tool_specs(),
            runtime_tools,
            plugin_tools,
            provided_capabilities,
            enforcer,
        ))
    }

    #[must_use]
    pub fn new(
        builtin_specs: Vec<ToolSpec>,
        runtime_tools: Vec<RuntimeToolDefinition>,
        plugin_tools: Vec<PluginTool>,
        provided_capabilities: Vec<CapabilitySpec>,
        enforcer: Option<PermissionEnforcer>,
    ) -> Self {
        Self {
            builtin_specs,
            runtime_tools,
            plugin_tools,
            provided_capabilities,
            enforcer,
        }
    }

    fn compilation_input(&self, current_dir: Option<&Path>) -> CapabilityCompilationInput {
        CapabilityCompilationInput {
            builtin_specs: self.builtin_specs.clone(),
            runtime_tools: self.runtime_tools.clone(),
            plugin_tools: self.plugin_tools.clone(),
            provided_capabilities: self.provided_capabilities.clone(),
            current_dir: current_dir.map(Path::to_path_buf),
            enforcer: self.enforcer.clone(),
        }
    }

    pub fn normalize_allowed_tools(
        &self,
        values: &[String],
    ) -> Result<Option<BTreeSet<String>>, String> {
        if values.is_empty() {
            return Ok(None);
        }

        let canonical_names = self
            .builtin_specs
            .iter()
            .map(|spec| spec.name.to_string())
            .chain(
                self.plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .chain(self.runtime_tools.iter().map(|tool| tool.name.clone()))
            .chain(
                self.provided_capabilities
                    .iter()
                    .filter(|capability| capability.execution_kind == CapabilityExecutionKind::Tool)
                    .map(|capability| capability.display_name.clone()),
            )
            .collect::<Vec<_>>();
        let mut name_map = canonical_names
            .iter()
            .map(|name| (normalize_allowed_tool_name(name), name.clone()))
            .collect::<BTreeMap<_, _>>();

        for (alias, canonical) in [
            ("read", "read_file"),
            ("write", "write_file"),
            ("edit", "edit_file"),
            ("glob", "glob_search"),
            ("grep", "grep_search"),
        ] {
            name_map.insert(alias.to_string(), canonical.to_string());
        }

        let mut allowed = BTreeSet::new();
        for value in values {
            for token in value
                .split(|ch: char| ch == ',' || ch.is_whitespace())
                .filter(|token| !token.is_empty())
            {
                let normalized = normalize_allowed_tool_name(token);
                let canonical = name_map.get(&normalized).ok_or_else(|| {
                    format!(
                        "unsupported tool in --allowedTools: {token} (expected one of: {})",
                        canonical_names.join(", ")
                    )
                })?;
                allowed.insert(canonical.clone());
            }
        }

        Ok(Some(allowed))
    }

    fn execute_local_tool(&self, tool_name: &str, input: &Value) -> Result<String, ToolError> {
        if self.builtin_specs.iter().any(|spec| spec.name == tool_name) {
            return execute_tool_with_enforcer(self.enforcer.as_ref(), tool_name, input)
                .map_err(ToolError::new);
        }

        self.plugin_tools
            .iter()
            .find(|tool| tool.definition().name == tool_name)
            .ok_or_else(|| ToolError::new(format!("unsupported tool: {tool_name}")))?
            .execute(input)
            .map_err(|error| ToolError::new(error.to_string()))
    }
}

fn normalize_allowed_tool_name(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

#[derive(Debug, Clone)]
pub struct CapabilityRuntime {
    provider: CapabilityProvider,
    compiler: CapabilityCompiler,
    planner: CapabilityPlanner,
    executor: CapabilityExecutor,
}

impl CapabilityRuntime {
    #[must_use]
    pub fn builtin() -> Self {
        Self::new(CapabilityProvider::builtin())
    }

    #[must_use]
    pub fn new(provider: CapabilityProvider) -> Self {
        Self {
            provider,
            compiler: CapabilityCompiler,
            planner: CapabilityPlanner,
            executor: CapabilityExecutor::default(),
        }
    }

    #[must_use]
    pub fn executor(&self) -> CapabilityExecutor {
        self.executor.clone()
    }

    pub fn register_prompt_skill_executor<F>(&self, key: impl Into<String>, executor: F)
    where
        F: Fn(
                &CapabilitySpec,
                Option<Value>,
                Option<&Path>,
            ) -> Result<crate::SkillExecutionResult, String>
            + Send
            + Sync
            + 'static,
    {
        self.executor.register_prompt_skill_executor(key, executor);
    }

    pub fn register_resource_executor<F>(&self, key: impl Into<String>, executor: F)
    where
        F: Fn(&CapabilitySpec, Value, Option<&Path>) -> Result<String, String>
            + Send
            + Sync
            + 'static,
    {
        self.executor.register_resource_executor(key, executor);
    }

    pub fn normalize_allowed_tools(
        &self,
        values: &[String],
    ) -> Result<Option<BTreeSet<String>>, String> {
        self.provider.normalize_allowed_tools(values)
    }

    pub fn set_execution_hook<F>(&self, hook: F)
    where
        F: Fn(CapabilityExecutionEvent) + Send + Sync + 'static,
    {
        self.executor.set_execution_hook(hook);
    }

    pub fn clear_execution_hook(&self) {
        self.executor.clear_execution_hook();
    }

    pub fn set_mediation_hook<F>(&self, hook: F)
    where
        F: Fn(&CapabilityExecutionRequest) -> CapabilityMediationDecision + Send + Sync + 'static,
    {
        self.executor.set_mediation_hook(hook);
    }

    pub fn clear_mediation_hook(&self) {
        self.executor.clear_mediation_hook();
    }

    pub fn set_enforcer(&mut self, enforcer: PermissionEnforcer) {
        self.provider.enforcer = Some(enforcer);
    }

    pub fn execute_local_tool(&self, tool_name: &str, input: &Value) -> Result<String, ToolError> {
        self.provider.execute_local_tool(tool_name, input)
    }

    pub fn execute_tool<F>(
        &self,
        tool_name: &str,
        input: Value,
        planner_input: CapabilityPlannerInput<'_>,
        store: &SessionCapabilityStore,
        pending_mcp_servers: Option<Vec<String>>,
        mcp_degraded: Option<McpDegradedReport>,
        mut dispatch: F,
    ) -> Result<String, ToolError>
    where
        F: FnMut(CapabilityDispatchKind, &str, Value) -> Result<String, ToolError>,
    {
        self.execute_tool_with_outcome(
            tool_name,
            input,
            planner_input,
            store,
            pending_mcp_servers,
            mcp_degraded,
            move |dispatch_kind, tool_name, input| {
                dispatch(dispatch_kind, tool_name, input)
                    .map(|output| ToolExecutionOutcome::Allow { output })
            },
        )
        .into_result(tool_name)
    }

    pub fn execute_tool_with_outcome<F>(
        &self,
        tool_name: &str,
        input: Value,
        planner_input: CapabilityPlannerInput<'_>,
        store: &SessionCapabilityStore,
        pending_mcp_servers: Option<Vec<String>>,
        mcp_degraded: Option<McpDegradedReport>,
        dispatch: F,
    ) -> ToolExecutionOutcome
    where
        F: FnMut(CapabilityDispatchKind, &str, Value) -> Result<ToolExecutionOutcome, ToolError>,
    {
        self.executor.execute_tool_with_outcome(
            self,
            tool_name,
            input,
            planner_input,
            store,
            pending_mcp_servers,
            mcp_degraded,
            dispatch,
        )
    }

    pub fn surface_projection(
        &self,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> Result<CapabilitySurfaceProjection, String> {
        let graph = self
            .compiler
            .compile(self.provider.compilation_input(planner_input.current_dir))?;
        Ok(self.finalize_surface(self.planner.surface_projection(graph, planner_input)))
    }

    pub fn execution_plan(
        &self,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> Result<CapabilityExecutionPlan, String> {
        let surface = self.surface_projection(planner_input)?;
        let session_state = planner_input.session_state.cloned().unwrap_or_default();
        Ok(CapabilityExecutionPlan {
            visible_tools: surface.visible_tools.clone(),
            deferred_tools: surface.deferred_tools.clone(),
            discoverable_skills: surface.discoverable_skills.clone(),
            available_resources: surface.available_resources.clone(),
            hidden_capabilities: surface.hidden_capabilities.clone(),
            activated_tools: session_state.activated_tools().iter().cloned().collect(),
            granted_tools: session_state.granted_tools().iter().cloned().collect(),
            pending_tools: session_state.pending_tools().iter().cloned().collect(),
            approved_tools: session_state.approved_tools().iter().cloned().collect(),
            auth_resolved_tools: session_state
                .auth_resolved_tools()
                .iter()
                .cloned()
                .collect(),
            provider_fallbacks: self.provider_fallbacks(&surface),
        })
    }

    pub fn planned_tool_definitions(
        &self,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> Result<Vec<ToolDefinition>, String> {
        Ok(self
            .surface_projection(planner_input)?
            .visible_tools
            .into_iter()
            .map(|capability| capability.to_tool_definition())
            .collect())
    }

    pub fn tool_definitions_for_allowlist(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
        current_dir: Option<&Path>,
    ) -> Result<Vec<ToolDefinition>, String> {
        Ok(self
            .surface_projection_for_allowlist(allowed_tools, current_dir)?
            .visible_tools
            .into_iter()
            .map(|capability| capability.to_tool_definition())
            .collect())
    }

    pub fn permission_specs(
        &self,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> Result<Vec<(String, PermissionMode)>, String> {
        Ok(self
            .surface_projection(planner_input)?
            .visible_tools
            .into_iter()
            .map(|capability| {
                (
                    capability.display_name,
                    capability.permission_profile.required_permission,
                )
            })
            .collect())
    }

    pub fn permission_specs_for_allowlist(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
        current_dir: Option<&Path>,
    ) -> Result<Vec<(String, PermissionMode)>, String> {
        Ok(self
            .surface_projection_for_allowlist(allowed_tools, current_dir)?
            .visible_tools
            .into_iter()
            .map(|capability| {
                (
                    capability.display_name,
                    capability.permission_profile.required_permission,
                )
            })
            .collect())
    }

    pub fn all_permission_specs(
        &self,
        current_dir: Option<&Path>,
    ) -> Result<Vec<(String, PermissionMode)>, String> {
        Ok(self
            .compiler
            .compile(self.provider.compilation_input(current_dir))?
            .capabilities
            .into_iter()
            .filter(|capability| {
                capability.execution_kind == CapabilityExecutionKind::Tool
                    && capability.state != CapabilityState::Unavailable
            })
            .map(|capability| {
                (
                    capability.display_name,
                    capability.permission_profile.required_permission,
                )
            })
            .collect())
    }

    pub fn surface_projection_for_allowlist(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
        current_dir: Option<&Path>,
    ) -> Result<CapabilitySurfaceProjection, String> {
        let graph = self
            .compiler
            .compile(self.provider.compilation_input(current_dir))?;
        Ok(self.finalize_surface(plan_effective_capability_surface(
            graph.capabilities,
            allowed_tools,
            graph.enforcer.as_ref(),
        )))
    }

    #[must_use]
    pub fn search(
        &self,
        query: &str,
        max_results: usize,
        planner_input: CapabilityPlannerInput<'_>,
        pending_mcp_servers: Option<Vec<String>>,
        mcp_degraded: Option<McpDegradedReport>,
    ) -> ToolSearchOutput {
        let query = query.trim().to_string();
        let normalized_query = normalize_tool_search_query(&query);
        let searchable = self
            .surface_projection(planner_input)
            .map(|surface| {
                surface
                    .deferred_tools
                    .into_iter()
                    .map(SearchableToolSpec::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let matches = search_tool_specs(&query, max_results.max(1), &searchable);
        let results = matches
            .iter()
            .filter_map(|name| searchable.iter().find(|spec| spec.name == *name))
            .map(SearchableToolSpec::to_search_result)
            .collect();

        ToolSearchOutput::new(
            matches,
            results,
            query,
            normalized_query,
            searchable.len(),
            pending_mcp_servers,
            mcp_degraded,
        )
    }

    #[must_use]
    pub fn skill_discovery(
        &self,
        query: &str,
        max_results: usize,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> crate::SkillDiscoveryOutput {
        let discoverable_skills = self
            .surface_projection(planner_input)
            .map(|surface| surface.discoverable_skills)
            .unwrap_or_default();
        crate::skill_runtime::discover_skills_from_capability_specs(
            query,
            max_results,
            &discoverable_skills,
        )
    }

    pub fn execute_skill(
        &self,
        skill_name: &str,
        arguments: Option<Value>,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> Result<crate::SkillExecutionResult, String> {
        self.execute_skill_detailed(skill_name, arguments, planner_input)
            .map_err(|failure| failure.message)
    }

    pub(crate) fn execute_skill_detailed(
        &self,
        skill_name: &str,
        arguments: Option<Value>,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> Result<crate::SkillExecutionResult, crate::skill_runtime::SkillExecutionFailure> {
        let requested = crate::skill_runtime::normalize_requested_skill_name(skill_name);
        let surface = self.surface_projection(planner_input).map_err(|message| {
            crate::skill_runtime::SkillExecutionFailure {
                message,
                state_updates: Vec::new(),
            }
        })?;
        let hidden_match = surface.hidden_capabilities.iter().any(|capability| {
            capability.execution_kind == CapabilityExecutionKind::PromptSkill
                && skill_matches_requested(capability, &requested)
        });
        let capability = surface
            .discoverable_skills
            .into_iter()
            .find(|capability| skill_matches_requested(capability, &requested))
            .ok_or_else(|| crate::skill_runtime::SkillExecutionFailure {
                message: if hidden_match {
                    format!("skill `{requested}` is not enabled in the current capability surface")
                } else {
                    crate::skill_runtime::explain_model_skill_unavailability(
                        skill_name,
                        planner_input.current_dir,
                    )
                },
                state_updates: Vec::new(),
            })?;
        crate::skill_runtime::execute_skill_capability_from_spec_detailed(
            &capability,
            arguments,
            planner_input.current_dir,
            Some(self.executor()),
        )
    }

    pub fn read_resource(
        &self,
        capability: &CapabilitySpec,
        input: Value,
        current_dir: Option<&Path>,
    ) -> Result<String, ToolError> {
        self.executor.read_resource(capability, input, current_dir)
    }

    fn finalize_surface(&self, surface: CapabilitySurface) -> CapabilitySurface {
        let mut discoverable_skills = Vec::new();
        let mut available_resources = Vec::new();
        let mut hidden_capabilities = surface.hidden_capabilities;

        for capability in surface.discoverable_skills {
            if self.executor.has_prompt_skill_executor(&capability) {
                discoverable_skills.push(capability);
            } else {
                hidden_capabilities.push(capability);
            }
        }

        for capability in surface.available_resources {
            if capability.state == CapabilityState::Ready
                && self.executor.has_resource_executor(&capability)
            {
                available_resources.push(capability);
            } else {
                hidden_capabilities.push(capability);
            }
        }

        CapabilitySurface {
            visible_tools: surface.visible_tools,
            deferred_tools: surface.deferred_tools,
            discoverable_skills,
            available_resources,
            hidden_capabilities,
        }
    }

    fn provider_fallbacks(&self, surface: &CapabilitySurface) -> Vec<String> {
        surface
            .available_resources
            .iter()
            .filter_map(|capability| {
                capability.provider_key.as_ref().map(|provider_key| {
                    format!(
                        "resource shim via provider `{provider_key}` for `{}`",
                        capability.display_name
                    )
                })
            })
            .collect()
    }
}

fn skill_matches_requested(capability: &CapabilitySpec, requested: &str) -> bool {
    capability.display_name.eq_ignore_ascii_case(requested)
        || capability.capability_id.eq_ignore_ascii_case(requested)
}
