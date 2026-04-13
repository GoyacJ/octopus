use std::collections::BTreeSet;
use std::path::Path;

use plugins::PluginTool;
use runtime::{permission_enforcer::PermissionEnforcer, PermissionMode};

use crate::tool_registry::{permission_mode_from_plugin, RuntimeToolDefinition, ToolSpec};

use super::provider::{
    CapabilityExecutionKind, CapabilityInvocationPolicy, CapabilityPermissionProfile,
    CapabilityScopeConstraints, CapabilitySourceKind, CapabilitySpec, CapabilityState,
    CapabilitySurface, CapabilityTrustProfile, CapabilityVisibility,
};
pub use super::state::CapabilitySurfaceProjection;
use super::state::SessionCapabilityState;

pub type EffectiveCapabilitySurface = CapabilitySurface;

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityExecutionPlan {
    pub visible_tools: Vec<CapabilitySpec>,
    pub deferred_tools: Vec<CapabilitySpec>,
    pub discoverable_skills: Vec<CapabilitySpec>,
    pub available_resources: Vec<CapabilitySpec>,
    pub hidden_capabilities: Vec<CapabilitySpec>,
    pub activated_tools: Vec<String>,
    pub granted_tools: Vec<String>,
    pub pending_tools: Vec<String>,
    pub approved_tools: Vec<String>,
    pub auth_resolved_tools: Vec<String>,
    pub provider_fallbacks: Vec<String>,
}

impl CapabilityExecutionPlan {
    #[must_use]
    pub fn surface(&self) -> CapabilitySurface {
        CapabilitySurface {
            visible_tools: self.visible_tools.clone(),
            deferred_tools: self.deferred_tools.clone(),
            discoverable_skills: self.discoverable_skills.clone(),
            available_resources: self.available_resources.clone(),
            hidden_capabilities: self.hidden_capabilities.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CapabilityCompilationInput {
    pub(crate) builtin_specs: Vec<ToolSpec>,
    pub(crate) runtime_tools: Vec<RuntimeToolDefinition>,
    pub(crate) plugin_tools: Vec<PluginTool>,
    pub(crate) provided_capabilities: Vec<CapabilitySpec>,
    pub(crate) current_dir: Option<std::path::PathBuf>,
    pub(crate) enforcer: Option<PermissionEnforcer>,
}

#[derive(Debug, Clone)]
pub(crate) struct CapabilityGraph {
    pub(crate) capabilities: Vec<CapabilitySpec>,
    pub(crate) enforcer: Option<PermissionEnforcer>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CapabilityCompiler;

impl CapabilityCompiler {
    pub(crate) fn compile(
        &self,
        input: CapabilityCompilationInput,
    ) -> Result<CapabilityGraph, String> {
        let has_invalid_plugin_permission = input
            .plugin_tools
            .iter()
            .any(|tool| permission_mode_from_plugin(tool.required_permission()).is_err());
        if has_invalid_plugin_permission {
            return Err(
                "plugin tool capability compile failed due to invalid permission".to_string(),
            );
        }

        let capabilities = compile_capability_specs(
            input.builtin_specs,
            &input.runtime_tools,
            &input.plugin_tools,
            &input.provided_capabilities,
            input.current_dir.as_deref(),
        );
        Ok(CapabilityGraph {
            capabilities,
            enforcer: input.enforcer,
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CapabilityPlanner;

impl CapabilityPlanner {
    #[must_use]
    pub(crate) fn surface_projection(
        &self,
        graph: CapabilityGraph,
        planner_input: CapabilityPlannerInput<'_>,
    ) -> CapabilitySurfaceProjection {
        plan_effective_capability_surface_with_planner(
            graph.capabilities,
            planner_input,
            graph.enforcer.as_ref(),
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CapabilityPlannerInput<'a> {
    pub profile_tools: Option<&'a BTreeSet<String>>,
    pub session_state: Option<&'a SessionCapabilityState>,
    pub current_dir: Option<&'a Path>,
}

impl<'a> CapabilityPlannerInput<'a> {
    #[must_use]
    pub fn new(
        profile_tools: Option<&'a BTreeSet<String>>,
        session_state: Option<&'a SessionCapabilityState>,
    ) -> Self {
        Self {
            profile_tools,
            session_state,
            current_dir: None,
        }
    }

    #[must_use]
    pub fn with_current_dir(mut self, current_dir: Option<&'a Path>) -> Self {
        self.current_dir = current_dir;
        self
    }
}

const DEFAULT_VISIBLE_BUILTIN_TOOL_NAMES: &[&str] = &[
    "bash",
    "read_file",
    "write_file",
    "edit_file",
    "glob_search",
    "grep_search",
    "ToolSearch",
    "TodoWrite",
    "SendUserMessage",
    "AskUserQuestion",
    "EnterPlanMode",
    "ExitPlanMode",
    "StructuredOutput",
];

fn builtin_visibility(name: &str) -> CapabilityVisibility {
    if DEFAULT_VISIBLE_BUILTIN_TOOL_NAMES.contains(&name) {
        CapabilityVisibility::DefaultVisible
    } else {
        CapabilityVisibility::Deferred
    }
}

pub(crate) fn concurrency_policy(
    required_permission: PermissionMode,
) -> super::provider::CapabilityConcurrencyPolicy {
    if required_permission == PermissionMode::ReadOnly {
        super::provider::CapabilityConcurrencyPolicy::ParallelRead
    } else {
        super::provider::CapabilityConcurrencyPolicy::Serialized
    }
}

fn invocation_policy(required_permission: PermissionMode) -> CapabilityInvocationPolicy {
    CapabilityInvocationPolicy {
        selectable: true,
        requires_approval: required_permission != PermissionMode::ReadOnly,
        requires_auth: false,
    }
}

fn compile_builtin_capability(spec: ToolSpec) -> CapabilitySpec {
    CapabilitySpec {
        capability_id: spec.name.to_string(),
        source_kind: CapabilitySourceKind::Builtin,
        execution_kind: CapabilityExecutionKind::Tool,
        provider_key: None,
        executor_key: None,
        display_name: spec.name.to_string(),
        description: spec.description.to_string(),
        when_to_use: None,
        input_schema: spec.input_schema,
        search_hint: Some(spec.description.to_string()),
        visibility: builtin_visibility(spec.name),
        state: CapabilityState::Ready,
        permission_profile: CapabilityPermissionProfile {
            required_permission: spec.required_permission,
        },
        trust_profile: CapabilityTrustProfile::default(),
        scope_constraints: CapabilityScopeConstraints::default(),
        invocation_policy: invocation_policy(spec.required_permission),
        concurrency_policy: concurrency_policy(spec.required_permission),
    }
}

fn compile_runtime_capability(tool: &RuntimeToolDefinition) -> CapabilitySpec {
    CapabilitySpec {
        capability_id: tool.name.clone(),
        source_kind: CapabilitySourceKind::RuntimeTool,
        execution_kind: CapabilityExecutionKind::Tool,
        provider_key: None,
        executor_key: Some(tool.name.clone()),
        display_name: tool.name.clone(),
        description: tool.description.clone().unwrap_or_default(),
        when_to_use: None,
        input_schema: tool.input_schema.clone(),
        search_hint: tool.description.clone(),
        visibility: CapabilityVisibility::Deferred,
        state: CapabilityState::Ready,
        permission_profile: CapabilityPermissionProfile {
            required_permission: tool.required_permission,
        },
        trust_profile: CapabilityTrustProfile::default(),
        scope_constraints: CapabilityScopeConstraints::default(),
        invocation_policy: invocation_policy(tool.required_permission),
        concurrency_policy: concurrency_policy(tool.required_permission),
    }
}

fn compile_plugin_capability(tool: &PluginTool) -> CapabilitySpec {
    let parsed_permission = permission_mode_from_plugin(tool.required_permission()).ok();
    let required_permission = parsed_permission.unwrap_or(PermissionMode::DangerFullAccess);
    let state = if parsed_permission.is_some() {
        CapabilityState::Ready
    } else {
        CapabilityState::Unavailable
    };

    CapabilitySpec {
        capability_id: tool.definition().name.clone(),
        source_kind: CapabilitySourceKind::PluginTool,
        execution_kind: CapabilityExecutionKind::Tool,
        provider_key: Some(tool.plugin_id().to_string()),
        executor_key: Some(tool.definition().name.clone()),
        display_name: tool.definition().name.clone(),
        description: tool.definition().description.clone().unwrap_or_default(),
        when_to_use: None,
        input_schema: tool.definition().input_schema.clone(),
        search_hint: tool.definition().description.clone(),
        visibility: CapabilityVisibility::Deferred,
        state,
        permission_profile: CapabilityPermissionProfile {
            required_permission,
        },
        trust_profile: CapabilityTrustProfile::default(),
        scope_constraints: CapabilityScopeConstraints::default(),
        invocation_policy: invocation_policy(required_permission),
        concurrency_policy: concurrency_policy(required_permission),
    }
}

#[must_use]
pub(crate) fn compile_capability_specs(
    builtin_specs: Vec<ToolSpec>,
    runtime_tools: &[RuntimeToolDefinition],
    plugin_tools: &[PluginTool],
    provided_capabilities: &[CapabilitySpec],
    current_dir: Option<&Path>,
) -> Vec<CapabilitySpec> {
    let builtin = builtin_specs.into_iter().map(compile_builtin_capability);
    let runtime = runtime_tools.iter().map(compile_runtime_capability);
    let plugin = plugin_tools.iter().map(compile_plugin_capability);
    let skills = crate::skill_runtime::compile_skill_capability_specs(current_dir).into_iter();
    let provided = provided_capabilities.iter().cloned();
    builtin
        .chain(runtime)
        .chain(plugin)
        .chain(skills)
        .chain(provided)
        .collect()
}

#[must_use]
pub(crate) fn plan_effective_capability_surface(
    capabilities: Vec<CapabilitySpec>,
    allowed_tools: Option<&BTreeSet<String>>,
    enforcer: Option<&PermissionEnforcer>,
) -> CapabilitySurface {
    let mut visible_tools = Vec::new();
    let mut deferred_tools = Vec::new();
    let mut discoverable_skills = Vec::new();
    let mut available_resources = Vec::new();
    let mut hidden_capabilities = Vec::new();

    for capability in capabilities {
        if capability.state == CapabilityState::Unavailable {
            hidden_capabilities.push(capability);
            continue;
        }

        match capability.execution_kind {
            CapabilityExecutionKind::Tool => {
                if let Some(enforcer) = enforcer {
                    if !enforcer.is_allowed(&capability.display_name, "{}") {
                        hidden_capabilities.push(capability);
                        continue;
                    }
                }

                if let Some(allowed_tools) = allowed_tools {
                    if allowed_tools.contains(capability.display_name.as_str()) {
                        visible_tools.push(capability);
                    } else {
                        hidden_capabilities.push(capability);
                    }
                    continue;
                }

                if capability.state != CapabilityState::Ready {
                    deferred_tools.push(capability);
                    continue;
                }

                match capability.visibility {
                    CapabilityVisibility::DefaultVisible => visible_tools.push(capability),
                    CapabilityVisibility::Deferred => deferred_tools.push(capability),
                    CapabilityVisibility::Hidden => hidden_capabilities.push(capability),
                }
            }
            CapabilityExecutionKind::PromptSkill => {
                if capability.state != CapabilityState::Ready
                    || !capability.invocation_policy.selectable
                    || !crate::skill_runtime::prompt_skill_has_runtime_executor(&capability)
                {
                    hidden_capabilities.push(capability);
                    continue;
                }

                match capability.visibility {
                    CapabilityVisibility::Hidden => hidden_capabilities.push(capability),
                    CapabilityVisibility::DefaultVisible | CapabilityVisibility::Deferred => {
                        discoverable_skills.push(capability);
                    }
                }
            }
            CapabilityExecutionKind::Resource => {
                if capability.state != CapabilityState::Ready {
                    hidden_capabilities.push(capability);
                    continue;
                }

                match capability.visibility {
                    CapabilityVisibility::Hidden => hidden_capabilities.push(capability),
                    CapabilityVisibility::DefaultVisible | CapabilityVisibility::Deferred => {
                        available_resources.push(capability);
                    }
                }
            }
        }
    }

    CapabilitySurface {
        visible_tools,
        deferred_tools,
        discoverable_skills,
        available_resources,
        hidden_capabilities,
    }
}

#[must_use]
pub(crate) fn plan_effective_capability_surface_with_planner(
    capabilities: Vec<CapabilitySpec>,
    planner_input: CapabilityPlannerInput<'_>,
    enforcer: Option<&PermissionEnforcer>,
) -> CapabilitySurface {
    let mut visible_tools = Vec::new();
    let mut deferred_tools = Vec::new();
    let mut discoverable_skills = Vec::new();
    let mut available_resources = Vec::new();
    let mut hidden_capabilities = Vec::new();

    for capability in capabilities {
        if capability.state == CapabilityState::Unavailable {
            hidden_capabilities.push(capability);
            continue;
        }

        match capability.execution_kind {
            CapabilityExecutionKind::Tool => {
                if let Some(enforcer) = enforcer {
                    if !enforcer.is_allowed(&capability.display_name, "{}") {
                        hidden_capabilities.push(capability);
                        continue;
                    }
                }

                if planner_input
                    .profile_tools
                    .is_some_and(|tools| !tools.contains(capability.display_name.as_str()))
                {
                    hidden_capabilities.push(capability);
                    continue;
                }

                if capability.state != CapabilityState::Ready {
                    deferred_tools.push(capability);
                    continue;
                }

                match capability.visibility {
                    CapabilityVisibility::DefaultVisible => visible_tools.push(capability),
                    CapabilityVisibility::Deferred => {
                        if planner_input.session_state.is_some_and(|state| {
                            state.is_tool_activated(&capability.display_name)
                                || state.is_tool_granted(&capability.display_name)
                        }) {
                            visible_tools.push(capability);
                        } else {
                            deferred_tools.push(capability);
                        }
                    }
                    CapabilityVisibility::Hidden => hidden_capabilities.push(capability),
                }
            }
            CapabilityExecutionKind::PromptSkill => {
                if capability.state != CapabilityState::Ready
                    || !capability.invocation_policy.selectable
                    || !crate::skill_runtime::prompt_skill_has_runtime_executor(&capability)
                {
                    hidden_capabilities.push(capability);
                    continue;
                }

                match capability.visibility {
                    CapabilityVisibility::Hidden => hidden_capabilities.push(capability),
                    CapabilityVisibility::DefaultVisible | CapabilityVisibility::Deferred => {
                        discoverable_skills.push(capability);
                    }
                }
            }
            CapabilityExecutionKind::Resource => {
                if capability.state != CapabilityState::Ready {
                    hidden_capabilities.push(capability);
                    continue;
                }

                match capability.visibility {
                    CapabilityVisibility::Hidden => hidden_capabilities.push(capability),
                    CapabilityVisibility::DefaultVisible | CapabilityVisibility::Deferred => {
                        available_resources.push(capability);
                    }
                }
            }
        }
    }

    CapabilitySurface {
        visible_tools,
        deferred_tools,
        discoverable_skills,
        available_resources,
        hidden_capabilities,
    }
}
