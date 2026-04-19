#[test]
fn seam_registry_module_exposes_builtin_specs() {
    assert!(!crate::tool_registry::mvp_tool_specs().is_empty());
}

#[test]
fn builtin_capability_catalog_classifies_builtin_families_and_visibility() {
    let catalog = crate::builtin_catalog::builtin_capability_catalog();

    let bash = catalog
        .find_tool("bash")
        .expect("bash should be cataloged as a builtin");
    assert_eq!(
        bash.category,
        crate::builtin_catalog::BuiltinCapabilityCategory::WorkerPrimitive
    );
    assert_eq!(bash.visibility, crate::CapabilityVisibility::DefaultVisible);
    assert_eq!(
        bash.handler_key,
        crate::builtin_catalog::BuiltinHandlerKey::Bash
    );

    let tool_search = catalog
        .find_tool("ToolSearch")
        .expect("ToolSearch should be cataloged as a builtin");
    assert_eq!(
        tool_search.category,
        crate::builtin_catalog::BuiltinCapabilityCategory::ControlPlane
    );
    assert_eq!(
        tool_search.visibility,
        crate::CapabilityVisibility::DefaultVisible
    );
    assert_eq!(
        tool_search.handler_key,
        crate::builtin_catalog::BuiltinHandlerKey::ToolSearch
    );

    let web_search = catalog
        .find_tool("WebSearch")
        .expect("WebSearch should be cataloged as a builtin");
    assert_eq!(
        web_search.category,
        crate::builtin_catalog::BuiltinCapabilityCategory::WebContext
    );
    assert_eq!(web_search.visibility, crate::CapabilityVisibility::Deferred);
}

#[test]
fn builtin_capability_catalog_resolves_aliases_to_handler_ownership() {
    let catalog = crate::builtin_catalog::builtin_capability_catalog();

    let canonical = catalog
        .resolve("SendUserMessage")
        .expect("canonical built-in should resolve");
    let alias = catalog
        .resolve("Brief")
        .expect("legacy alias should resolve");

    assert_eq!(canonical.name, "SendUserMessage");
    assert_eq!(alias.name, "SendUserMessage");
    assert_eq!(
        canonical.handler_key,
        crate::builtin_catalog::BuiltinHandlerKey::Brief
    );
    assert_eq!(alias.handler_key, canonical.handler_key);
    assert!(catalog.resolve("TaskCreate").is_none());
}

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use super::{
    agent_permission_policy, allowed_tools_for_subagent, classify_lane_failure, derive_agent_state,
    execute_tool, final_assistant_text, maybe_commit_provenance, mvp_tool_specs,
    permission_mode_from_plugin, persist_agent_terminal_state, push_output_block,
    spawn_subagent_with_job, AgentInput, AgentJob, CapabilityPlannerInput, CapabilityProvider,
    CapabilityRuntime, LaneEventName, LaneFailureClass, SubagentToolExecutor,
};
use api::OutputContentBlock;
use plugins::{PluginTool, PluginToolDefinition, PluginToolPermission};
use runtime::{
    permission_enforcer::PermissionEnforcer, ApiRequest, AssistantEvent, ConfigLoader,
    ConversationRuntime, PermissionMode, PermissionPolicy, RuntimeError, Session, ToolExecutor,
};
use serde_json::json;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvVarRestore {
    key: &'static str,
    value: Option<OsString>,
}

impl Drop for EnvVarRestore {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            std::env::set_var(self.key, value);
        } else {
            std::env::remove_var(self.key);
        }
    }
}

fn override_env_var(key: &'static str, value: impl Into<OsString>) -> EnvVarRestore {
    let previous = std::env::var_os(key);
    let value = value.into();
    std::env::set_var(key, &value);
    EnvVarRestore {
        key,
        value: previous,
    }
}

fn temp_path(name: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("clawd-tools-{unique}-{name}"))
}

fn legacy_tool_name(parts: &[&str]) -> String {
    parts.concat()
}

fn run_git(cwd: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|error| panic!("git {} failed: {error}", args.join(" ")));
    assert!(
        status.success(),
        "git {} exited with {status}",
        args.join(" ")
    );
}

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).expect("create repo");
    run_git(path, &["init", "--quiet", "-b", "main"]);
    run_git(path, &["config", "user.email", "tests@example.com"]);
    run_git(path, &["config", "user.name", "Tools Tests"]);
    std::fs::write(path.join("README.md"), "initial\n").expect("write readme");
    run_git(path, &["add", "README.md"]);
    run_git(path, &["commit", "-m", "initial commit", "--quiet"]);
}

fn commit_file(path: &Path, file: &str, contents: &str, message: &str) {
    std::fs::write(path.join(file), contents).expect("write file");
    run_git(path, &["add", file]);
    run_git(path, &["commit", "-m", message, "--quiet"]);
}

fn permission_policy_for_mode(mode: PermissionMode) -> PermissionPolicy {
    mvp_tool_specs()
        .into_iter()
        .fold(PermissionPolicy::new(mode), |policy, spec| {
            policy.with_tool_requirement(spec.name, spec.required_permission)
        })
}

fn capability_provider_from_sources(
    plugin_tools: Vec<PluginTool>,
    runtime_tools: Vec<super::RuntimeToolDefinition>,
    provided_capabilities: Vec<super::CapabilitySpec>,
    enforcer: Option<PermissionEnforcer>,
) -> CapabilityProvider {
    CapabilityProvider::from_sources_checked(
        plugin_tools,
        runtime_tools,
        provided_capabilities,
        enforcer,
    )
    .expect("capability sources should validate")
}

fn capability_runtime_from_sources(
    plugin_tools: Vec<PluginTool>,
    runtime_tools: Vec<super::RuntimeToolDefinition>,
    provided_capabilities: Vec<super::CapabilitySpec>,
    enforcer: Option<PermissionEnforcer>,
) -> CapabilityRuntime {
    CapabilityRuntime::new(capability_provider_from_sources(
        plugin_tools,
        runtime_tools,
        provided_capabilities,
        enforcer,
    ))
}

fn capability_runtime_with_provided_capabilities(
    provided_capabilities: Vec<super::CapabilitySpec>,
) -> CapabilityRuntime {
    capability_runtime_from_sources(Vec::new(), Vec::new(), provided_capabilities, None)
}

fn provider_prompt_skill_capability(
    capability_id: &str,
    source_kind: super::CapabilitySourceKind,
    display_name: &str,
) -> super::CapabilitySpec {
    super::CapabilitySpec {
        capability_id: capability_id.to_string(),
        source_kind,
        execution_kind: super::CapabilityExecutionKind::PromptSkill,
        display_name: display_name.to_string(),
        description: "Provider-backed workspace guidance skill.".to_string(),
        when_to_use: Some("Use when the task needs workspace-specific guidance.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "skill": { "type": "string" },
                "arguments": {}
            },
            "required": ["skill"],
            "additionalProperties": false
        }),
        search_hint: Some("workspace guidance".to_string()),
        visibility: super::CapabilityVisibility::DefaultVisible,
        state: super::CapabilityState::Ready,
        permission_profile: crate::capability_runtime::CapabilityPermissionProfile {
            required_permission: PermissionMode::ReadOnly,
        },
        trust_profile: crate::capability_runtime::CapabilityTrustProfile::default(),
        scope_constraints: crate::capability_runtime::CapabilityScopeConstraints::default(),
        invocation_policy: crate::capability_runtime::CapabilityInvocationPolicy {
            selectable: true,
            requires_approval: false,
            requires_auth: false,
        },
        concurrency_policy: super::CapabilityConcurrencyPolicy::Serialized,
        provider_key: Some(source_kind.to_string()),
        executor_key: Some(capability_id.to_string()),
    }
}

fn capability_runtime_with_runtime_tools(
    runtime_tools: Vec<super::RuntimeToolDefinition>,
) -> CapabilityRuntime {
    capability_runtime_from_sources(Vec::new(), runtime_tools, Vec::new(), None)
}

fn capability_runtime_with_plugin_tools(plugin_tools: Vec<PluginTool>) -> CapabilityRuntime {
    capability_runtime_from_sources(plugin_tools, Vec::new(), Vec::new(), None)
}

fn execute_local_tool_with_runtime(
    runtime: &CapabilityRuntime,
    name: &str,
    input: &serde_json::Value,
) -> Result<String, String> {
    runtime
        .execute_local_tool(name, input)
        .map_err(|error| error.to_string())
}

fn discover_skills_with_runtime(
    runtime: &CapabilityRuntime,
    query: &str,
    max_results: usize,
) -> serde_json::Value {
    let current_dir = std::env::current_dir().ok();
    let discovery = runtime.skill_discovery(
        query,
        max_results,
        CapabilityPlannerInput::default().with_current_dir(current_dir.as_deref()),
    );
    serde_json::to_value(discovery).expect("skill discovery output should be json")
}

fn execute_prompt_skill_with_runtime(
    runtime: &CapabilityRuntime,
    skill: &str,
    arguments: Option<serde_json::Value>,
) -> Result<String, String> {
    let current_dir = std::env::current_dir().ok();
    match runtime.execute_skill(
        skill,
        arguments,
        CapabilityPlannerInput::default().with_current_dir(current_dir.as_deref()),
    ) {
        Ok(result) => serde_json::to_string_pretty(&result).map_err(|error| error.to_string()),
        Err(error) => Err(error),
    }
}

#[test]
fn exposes_mvp_tools() {
    let names = mvp_tool_specs()
        .into_iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>();
    assert!(names.contains(&"bash"));
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"WebFetch"));
    assert!(names.contains(&"WebSearch"));
    assert!(names.contains(&"TodoWrite"));
    assert!(!names.contains(&"Agent"));
    assert!(names.contains(&"ToolSearch"));
    assert!(names.contains(&"NotebookEdit"));
    assert!(names.contains(&"Sleep"));
    assert!(names.contains(&"SendUserMessage"));
    assert!(names.contains(&"Config"));
    assert!(names.contains(&"EnterPlanMode"));
    assert!(names.contains(&"ExitPlanMode"));
    assert!(names.contains(&"StructuredOutput"));
    assert!(names.contains(&"REPL"));
    assert!(names.contains(&"PowerShell"));
    assert!(!names.contains(&legacy_tool_name(&["Task", "Create"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Run", "Task", "Packet"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Task", "Get"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Task", "List"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Task", "Stop"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Task", "Update"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Task", "Output"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Create"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Get"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Observe"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Resolve", "Trust"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Await", "Ready"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Send", "Prompt"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Restart"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Terminate"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Worker", "Observe", "Completion"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Team", "Create"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Team", "Delete"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Cron", "Create"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Cron", "Delete"]).as_str()));
    assert!(!names.contains(&legacy_tool_name(&["Cron", "List"]).as_str()));
}

#[test]
fn rejects_unknown_tool_names() {
    let error = execute_tool("nope", &json!({})).expect_err("tool should be rejected");
    assert!(error.contains("unsupported tool"));
}

#[test]
fn rejects_legacy_orchestration_tool_names() {
    for tool_name in [
        legacy_tool_name(&["Task", "Create"]),
        legacy_tool_name(&["Run", "Task", "Packet"]),
        legacy_tool_name(&["Task", "Get"]),
        legacy_tool_name(&["Task", "List"]),
        legacy_tool_name(&["Task", "Stop"]),
        legacy_tool_name(&["Task", "Update"]),
        legacy_tool_name(&["Task", "Output"]),
        legacy_tool_name(&["Worker", "Create"]),
        legacy_tool_name(&["Worker", "Get"]),
        legacy_tool_name(&["Worker", "Observe"]),
        legacy_tool_name(&["Worker", "Resolve", "Trust"]),
        legacy_tool_name(&["Worker", "Await", "Ready"]),
        legacy_tool_name(&["Worker", "Send", "Prompt"]),
        legacy_tool_name(&["Worker", "Restart"]),
        legacy_tool_name(&["Worker", "Terminate"]),
        legacy_tool_name(&["Worker", "Observe", "Completion"]),
        legacy_tool_name(&["Team", "Create"]),
        legacy_tool_name(&["Team", "Delete"]),
        legacy_tool_name(&["Cron", "Create"]),
        legacy_tool_name(&["Cron", "Delete"]),
        legacy_tool_name(&["Cron", "List"]),
    ] {
        let error =
            execute_tool(&tool_name, &json!({})).expect_err("legacy tool should be rejected");
        assert!(
            error.contains("unsupported tool"),
            "{tool_name} should be unsupported"
        );
    }
}

#[test]
fn capability_runtime_denies_blocked_tool_before_dispatch() {
    // given
    let policy = permission_policy_for_mode(PermissionMode::ReadOnly);
    let runtime = capability_runtime_from_sources(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Some(PermissionEnforcer::new(policy)),
    );

    // when
    let error = execute_local_tool_with_runtime(
        &runtime,
        "write_file",
        &json!({
            "path": "blocked.txt",
            "content": "blocked"
        }),
    )
    .expect_err("write tool should be denied before dispatch");

    // then
    assert!(error.contains("requires workspace-write permission"));
}

#[test]
fn subagent_tool_executor_denies_blocked_tool_before_dispatch() {
    // given
    let policy = permission_policy_for_mode(PermissionMode::ReadOnly);
    let mut executor = SubagentToolExecutor::new(BTreeSet::from([String::from("write_file")]))
        .with_enforcer(PermissionEnforcer::new(policy));

    // when
    let error = executor
        .execute(
            "write_file",
            &json!({
                "path": "blocked.txt",
                "content": "blocked"
            })
            .to_string(),
        )
        .expect_err("subagent write tool should be denied before dispatch");

    // then
    assert!(error
        .to_string()
        .contains("is not enabled in the current capability surface"));
}

#[test]
fn subagent_tool_search_select_updates_shared_session_capability_state() {
    let capability_provider = CapabilityProvider::builtin();
    let capability_runtime = CapabilityRuntime::new(capability_provider.clone());
    let profile = super::CapabilityProfile::from_tools(
        ["ToolSearch", "WebSearch"]
            .into_iter()
            .map(str::to_string)
            .collect(),
    );
    let shared_state = std::sync::Arc::new(std::sync::Mutex::new(
        super::SessionCapabilityState::default(),
    ));
    let mut executor = SubagentToolExecutor::from_capability_provider(
        profile.clone(),
        capability_provider,
        shared_state.clone(),
    );

    let output = executor
        .execute(
            "ToolSearch",
            r#"{"query":"select:WebSearch","max_results":5}"#,
        )
        .expect("ToolSearch select should succeed");
    let output_json: serde_json::Value =
        serde_json::from_str(&output).expect("search output should be valid json");
    assert_eq!(output_json["matches"][0], "WebSearch");

    let locked = shared_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(locked.is_tool_activated("WebSearch"));

    let surface = capability_runtime
        .surface_projection(super::CapabilityPlannerInput::new(
            Some(profile.allowed_tools()),
            Some(&locked),
        ))
        .expect("activated tool should be visible");
    assert!(surface
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));
}

#[test]
fn tool_search_select_records_selection_detail_and_exposure_state() {
    let runtime = CapabilityRuntime::builtin();
    let store = super::SessionCapabilityStore::default();
    let profile = BTreeSet::from([String::from("ToolSearch"), String::from("WebSearch")]);
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured_events = Arc::clone(&events);

    runtime.set_execution_hook(move |event| {
        captured_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    });

    let output = runtime
        .execute_tool(
            "ToolSearch",
            json!({
                "query": "select:WebSearch",
                "max_results": 5
            }),
            super::CapabilityPlannerInput::new(Some(&profile), Some(&store.snapshot())),
            &store,
            None,
            None,
            |_dispatch_kind, _tool_name, _input| {
                panic!("builtin ToolSearch should not dispatch through runtime capability")
            },
        )
        .expect("ToolSearch select should succeed");
    let output_json: serde_json::Value = serde_json::from_str(&output).expect("valid json");
    assert_eq!(output_json["matches"][0], "WebSearch");

    let state = store.snapshot();
    assert!(state.is_tool_discovered("WebSearch"));
    assert!(state.is_tool_activated("WebSearch"));
    assert!(state.is_tool_exposed("WebSearch"));

    let events = events
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(events.iter().any(|event| {
        event.phase == super::CapabilityExecutionPhase::Completed
            && event.tool_name == "ToolSearch"
            && event.detail.as_deref().is_some_and(|detail| {
                detail.contains("select:WebSearch") && detail.contains("WebSearch")
            })
    }));
}

#[test]
fn session_capability_state_tracks_discovery_activation_and_exposure_separately() {
    let mut state = super::SessionCapabilityState::default();

    state.discover_tool("WebSearch");
    assert!(state.is_tool_discovered("WebSearch"));
    assert!(!state.is_tool_activated("WebSearch"));
    assert!(!state.is_tool_exposed("WebSearch"));

    state.activate_discovered_tool("WebSearch");
    assert!(state.is_tool_activated("WebSearch"));
    assert!(!state.is_tool_exposed("WebSearch"));

    state.expose_tool("WebSearch");
    assert!(state.is_tool_exposed("WebSearch"));
    assert!(state
        .exposure_snapshot()
        .activated_tools()
        .contains("WebSearch"));
}

#[test]
fn deferred_tool_direct_call_returns_retry_hint_without_state_mutation() {
    let runtime = CapabilityRuntime::builtin();
    let store = super::SessionCapabilityStore::default();
    let profile = BTreeSet::from([String::from("ToolSearch"), String::from("WebSearch")]);

    let error = runtime
        .execute_tool(
            "WebSearch",
            json!({
                "query": "latest octopus runtime"
            }),
            super::CapabilityPlannerInput::new(Some(&profile), Some(&store.snapshot())),
            &store,
            None,
            None,
            |_dispatch_kind, _tool_name, _input| {
                panic!("deferred builtin should be blocked before dispatch")
            },
        )
        .expect_err("unexposed deferred tool should return a retry hint");

    let message = error.to_string();
    assert!(message.contains("ToolSearch"));
    assert!(message.contains("select:WebSearch"));
    assert!(message.contains("not exposed"));

    let state = store.snapshot();
    assert!(!state.is_tool_discovered("WebSearch"));
    assert!(!state.is_tool_activated("WebSearch"));
    assert!(!state.is_tool_exposed("WebSearch"));
}

#[test]
fn workspace_and_subagent_skill_paths_match_runtime_surface_rules() {
    let capability = super::CapabilitySpec {
        capability_id: "plugin-skill.workspace-guide-parity".to_string(),
        source_kind: super::CapabilitySourceKind::PluginSkill,
        execution_kind: super::CapabilityExecutionKind::PromptSkill,
        provider_key: Some("plugin_skill".to_string()),
        executor_key: Some("plugin-skill.workspace-guide-parity".to_string()),
        display_name: "workspace-guide-parity".to_string(),
        description: "Provider-backed workspace guidance skill.".to_string(),
        when_to_use: Some("Use when the task needs workspace-specific guidance.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "skill": { "type": "string" },
                "arguments": {}
            },
            "required": ["skill"],
            "additionalProperties": false
        }),
        search_hint: Some("workspace guidance".to_string()),
        visibility: super::CapabilityVisibility::DefaultVisible,
        state: super::CapabilityState::Ready,
        permission_profile: crate::capability_runtime::CapabilityPermissionProfile {
            required_permission: PermissionMode::ReadOnly,
        },
        trust_profile: crate::capability_runtime::CapabilityTrustProfile::default(),
        scope_constraints: crate::capability_runtime::CapabilityScopeConstraints::default(),
        invocation_policy: crate::capability_runtime::CapabilityInvocationPolicy {
            selectable: true,
            requires_approval: false,
            requires_auth: false,
        },
        concurrency_policy: super::CapabilityConcurrencyPolicy::Serialized,
    };
    let capability_provider =
        capability_provider_from_sources(Vec::new(), Vec::new(), vec![capability], None);
    let capability_runtime = CapabilityRuntime::new(capability_provider.clone());

    let workspace_discovery = capability_runtime.skill_discovery(
        "workspace guidance",
        10,
        super::CapabilityPlannerInput::default(),
    );
    let workspace_discovery: serde_json::Value =
        serde_json::to_value(workspace_discovery).expect("workspace discovery should serialize");
    assert!(!workspace_discovery["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .any(|value| value == "workspace-guide-parity"));

    let workspace_error = capability_runtime
        .execute_skill(
            "workspace-guide-parity",
            Some(json!({ "topic": "workspace" })),
            super::CapabilityPlannerInput::default(),
        )
        .expect_err("workspace/runtime skill path should be surface gated");
    assert!(workspace_error.contains("is not enabled in the current capability surface"));

    let profile =
        super::CapabilityProfile::from_tools(BTreeSet::from([String::from("ToolSearch")]));
    let shared_state = std::sync::Arc::new(std::sync::Mutex::new(
        super::SessionCapabilityState::default(),
    ));
    let mut executor =
        SubagentToolExecutor::from_capability_provider(profile, capability_provider, shared_state);

    let subagent_discovery = executor
        .execute(
            "ToolSearch",
            r#"{"query":"workspace guidance","max_results":10}"#,
        )
        .expect("subagent tool search should succeed");
    let subagent_discovery: serde_json::Value =
        serde_json::from_str(&subagent_discovery).expect("subagent discovery should be json");
    assert!(!subagent_discovery["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .any(|value| value == "workspace-guide-parity"));

    let subagent_error = capability_runtime
        .execute_skill(
            "workspace-guide-parity",
            Some(json!({"topic":"workspace"})),
            super::CapabilityPlannerInput::default(),
        )
        .expect_err("subagent skill path should be surface gated");
    assert!(subagent_error.contains("is not enabled in the current capability surface"));
}

#[test]
fn permission_mode_from_plugin_rejects_invalid_inputs() {
    let unknown_permission =
        permission_mode_from_plugin("admin").expect_err("unknown plugin permission should fail");
    assert!(unknown_permission.contains("unsupported plugin permission: admin"));

    let empty_permission =
        permission_mode_from_plugin("").expect_err("empty plugin permission should fail");
    assert!(empty_permission.contains("unsupported plugin permission: "));
}

#[test]
fn builtin_capability_surface_classifies_default_visible_and_deferred_tools() {
    let surface = CapabilityRuntime::builtin()
        .surface_projection_for_allowlist(None, None)
        .expect("builtin capabilities should plan");

    let visible = surface
        .visible_tools
        .iter()
        .map(|capability| capability.display_name.as_str())
        .collect::<Vec<_>>();
    let deferred = surface
        .deferred_tools
        .iter()
        .map(|capability| capability.display_name.as_str())
        .collect::<Vec<_>>();
    let hidden = surface
        .hidden_capabilities
        .iter()
        .map(|capability| capability.display_name.as_str())
        .collect::<Vec<_>>();

    assert!(visible.contains(&"read_file"));
    assert!(visible.contains(&"ToolSearch"));
    assert!(!deferred.contains(&"read_file"));
    assert!(deferred.contains(&"WebSearch"));
    assert!(!deferred.contains(&"Skill"));
    assert!(!hidden.contains(&"Skill"));
}

#[test]
fn denied_tools_are_filtered_before_exposure() {
    let policy = permission_policy_for_mode(runtime::PermissionMode::ReadOnly);
    let runtime = capability_runtime_from_sources(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Some(PermissionEnforcer::new(policy)),
    );
    let allowed = runtime
        .normalize_allowed_tools(&["read_file,write_file".to_string()])
        .expect("allow-list should normalize")
        .expect("allow-list should be populated");

    let definitions = runtime
        .tool_definitions_for_allowlist(Some(&allowed), None)
        .expect("definitions should plan");
    let names = definitions
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();

    assert_eq!(names, vec!["read_file".to_string()]);
}

#[test]
fn runtime_tools_compile_into_deferred_runtime_capabilities() {
    let runtime = capability_runtime_from_sources(
        Vec::new(),
        vec![super::RuntimeToolDefinition {
            name: "mcp__demo__echo".to_string(),
            description: Some("Echo text from the demo MCP server".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "additionalProperties": false
            }),
            required_permission: runtime::PermissionMode::ReadOnly,
        }],
        Vec::new(),
        None,
    );
    let surface = runtime
        .surface_projection_for_allowlist(None, None)
        .expect("runtime capabilities should plan");

    let capability = surface
        .deferred_tools
        .iter()
        .find(|capability| capability.display_name == "mcp__demo__echo")
        .expect("runtime capability should be present");

    assert_eq!(
        capability.source_kind,
        super::capability_runtime::CapabilitySourceKind::RuntimeTool
    );
    assert_eq!(
        capability.execution_kind,
        super::capability_runtime::CapabilityExecutionKind::Tool
    );
    assert_eq!(
        capability.visibility,
        super::capability_runtime::CapabilityVisibility::Deferred
    );
    assert_eq!(
        capability.permission_profile.required_permission,
        runtime::PermissionMode::ReadOnly
    );
}

#[test]
fn workspace_write_runtime_tools_stay_deferred_without_approval_until_execution() {
    let runtime = capability_runtime_from_sources(
        Vec::new(),
        vec![super::RuntimeToolDefinition {
            name: "workspace_write_tool".to_string(),
            description: Some("Writes inside the workspace.".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": { "path": { "type": "string" } },
                "additionalProperties": false
            }),
            required_permission: runtime::PermissionMode::WorkspaceWrite,
        }],
        Vec::new(),
        None,
    );
    let surface = runtime
        .surface_projection_for_allowlist(None, None)
        .expect("runtime capabilities should plan");

    let capability = surface
        .deferred_tools
        .iter()
        .find(|entry| entry.display_name == "workspace_write_tool")
        .expect("workspace write capability should be present");

    assert_eq!(
        capability.permission_profile.required_permission,
        runtime::PermissionMode::WorkspaceWrite
    );
    assert!(!capability.invocation_policy.requires_approval);
}

#[test]
fn runtime_tools_extend_provider_definitions_permissions_and_search() {
    let runtime = capability_runtime_from_sources(
        Vec::new(),
        vec![super::RuntimeToolDefinition {
            name: "mcp__demo__echo".to_string(),
            description: Some("Echo text from the demo MCP server".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "additionalProperties": false
            }),
            required_permission: runtime::PermissionMode::ReadOnly,
        }],
        Vec::new(),
        None,
    );

    let allowed = runtime
        .normalize_allowed_tools(&["mcp__demo__echo".to_string()])
        .expect("runtime tool should be allow-listable")
        .expect("allow-list should be populated");
    assert!(allowed.contains("mcp__demo__echo"));

    let definitions = runtime
        .tool_definitions_for_allowlist(Some(&allowed), None)
        .expect("definitions should plan");
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].name, "mcp__demo__echo");

    let permissions = runtime
        .permission_specs_for_allowlist(Some(&allowed), None)
        .expect("runtime tool permissions should resolve");
    assert_eq!(
        permissions,
        vec![(
            "mcp__demo__echo".to_string(),
            runtime::PermissionMode::ReadOnly
        )]
    );

    let search = runtime.search(
        "demo echo",
        5,
        super::CapabilityPlannerInput::default(),
        Some(vec!["pending-server".to_string()]),
        Some(runtime::McpDegradedReport::new(
            vec!["demo".to_string()],
            vec![runtime::McpFailedServer {
                server_name: "pending-server".to_string(),
                phase: runtime::McpLifecyclePhase::ToolDiscovery,
                error: runtime::McpErrorSurface::new(
                    runtime::McpLifecyclePhase::ToolDiscovery,
                    Some("pending-server".to_string()),
                    "tool discovery failed",
                    BTreeMap::new(),
                    true,
                ),
            }],
            vec!["mcp__demo__echo".to_string()],
            vec!["mcp__demo__echo".to_string()],
        )),
    );
    let output = serde_json::to_value(search).expect("search output should serialize");
    assert_eq!(output["matches"][0], "mcp__demo__echo");
    assert_eq!(output["pending_mcp_servers"][0], "pending-server");
    assert_eq!(
        output["mcp_degraded"]["failed_servers"][0]["phase"],
        "tool_discovery"
    );
}

#[test]
fn tool_search_returns_only_deferred_tool_capabilities_with_metadata() {
    let runtime = capability_runtime_from_sources(
        Vec::new(),
        vec![super::RuntimeToolDefinition {
            name: "mcp__demo__echo".to_string(),
            description: Some("Echo text from the demo MCP server".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "additionalProperties": false
            }),
            required_permission: runtime::PermissionMode::ReadOnly,
        }],
        Vec::new(),
        None,
    );

    let search = runtime.search(
        "select:read_file,mcp__demo__echo,WebSearch",
        5,
        super::CapabilityPlannerInput::default(),
        None,
        None,
    );
    let output = serde_json::to_value(search).expect("search output should serialize");
    let matches = output["matches"]
        .as_array()
        .expect("matches should be present");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0], "mcp__demo__echo");
    assert_eq!(matches[1], "WebSearch");

    let results = output["results"]
        .as_array()
        .expect("results should be present");
    let runtime_match = results
        .iter()
        .find(|entry| entry["name"] == "mcp__demo__echo")
        .expect("runtime match should be present");
    assert_eq!(runtime_match["source_kind"], "runtime_tool");
    assert_eq!(runtime_match["permission"], "read-only");
    assert_eq!(runtime_match["state"], "ready");
    assert_eq!(runtime_match["deferred"], true);

    let builtin_match = results
        .iter()
        .find(|entry| entry["name"] == "WebSearch")
        .expect("builtin match should be present");
    assert_eq!(builtin_match["source_kind"], "builtin");
    assert_eq!(builtin_match["deferred"], true);
}

#[test]
fn mcp_capability_helpers_build_tool_and_resource_specs() {
    let tool = runtime::ManagedMcpTool {
        server_name: "alpha".to_string(),
        qualified_name: "mcp__alpha__echo".to_string(),
        raw_name: "echo".to_string(),
        tool: runtime::McpTool {
            name: "echo".to_string(),
            description: Some("Echo input".to_string()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            })),
            annotations: Some(json!({
                "readOnlyHint": true
            })),
            meta: None,
        },
    };
    let tool_descriptor = super::capability_runtime::mcp_tool_capability_descriptor(&tool);
    assert_eq!(tool_descriptor.display_name, "mcp__alpha__echo");
    assert_eq!(
        tool_descriptor.source_kind,
        super::CapabilitySourceKind::McpTool
    );
    assert_eq!(
        tool_descriptor.execution_kind,
        super::CapabilityExecutionKind::Tool
    );
    assert_eq!(
        tool_descriptor.required_permission,
        PermissionMode::ReadOnly
    );
    assert!(!tool_descriptor.requires_auth);
    assert!(!tool_descriptor.requires_approval);

    let destructive_tool = runtime::McpTool {
        name: "apply".to_string(),
        description: None,
        input_schema: None,
        annotations: Some(json!({
            "destructiveHint": true
        })),
        meta: None,
    };
    assert_eq!(
        super::capability_runtime::permission_mode_for_mcp_tool(&destructive_tool),
        PermissionMode::DangerFullAccess
    );

    let workspace_write_tool = runtime::ManagedMcpTool {
        server_name: "alpha".to_string(),
        qualified_name: "mcp__alpha__apply".to_string(),
        raw_name: "apply".to_string(),
        tool: runtime::McpTool {
            name: "apply".to_string(),
            description: Some("Apply workspace edits".to_string()),
            input_schema: Some(json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            })),
            annotations: None,
            meta: None,
        },
    };
    let workspace_write_descriptor =
        super::capability_runtime::mcp_tool_capability_descriptor(&workspace_write_tool);
    assert_eq!(
        workspace_write_descriptor.required_permission,
        PermissionMode::WorkspaceWrite
    );
    assert!(!workspace_write_descriptor.requires_approval);

    let resource = runtime::McpResource {
        uri: "file://guide.txt".to_string(),
        name: Some("Guide".to_string()),
        description: Some("Workspace guide".to_string()),
        mime_type: Some("text/plain".to_string()),
        annotations: None,
        meta: None,
    };
    let resource_descriptor =
        super::capability_runtime::mcp_resource_capability_descriptor("alpha", &resource);
    assert_eq!(
        resource_descriptor.source_kind,
        super::CapabilitySourceKind::McpResource
    );
    assert_eq!(
        resource_descriptor.execution_kind,
        super::CapabilityExecutionKind::Resource
    );
    assert_eq!(
        resource_descriptor.visibility,
        super::CapabilityVisibility::DefaultVisible
    );
    assert_eq!(
        resource_descriptor.required_permission,
        PermissionMode::ReadOnly
    );
}

#[test]
fn managed_mcp_runtime_builds_capabilities_and_surfaces_connection_state() {
    let (config_home, workspace, mut mcp_runtime) = setup_managed_mcp_runtime_fixture(true);

    let provided_capabilities = mcp_runtime.provided_capabilities();
    assert!(provided_capabilities.iter().any(|capability| {
        capability.display_name == "mcp__alpha__echo"
            && capability.source_kind == super::CapabilitySourceKind::McpTool
            && capability.execution_kind == super::CapabilityExecutionKind::Tool
    }));
    assert!(provided_capabilities.iter().any(|capability| {
        capability.source_kind == super::CapabilitySourceKind::McpResource
            && capability.execution_kind == super::CapabilityExecutionKind::Resource
    }));

    let connections = mcp_runtime.connection_projections();
    assert!(connections.iter().any(|connection| {
        connection.server_name == "alpha" && connection.state == super::CapabilityState::Ready
    }));
    assert!(connections.iter().any(|connection| {
        connection.server_name == "broken" && connection.state == super::CapabilityState::Degraded
    }));

    assert_eq!(
        mcp_runtime.pending_servers(),
        Some(vec!["broken".to_string()])
    );
    let degraded = mcp_runtime
        .degraded_report()
        .expect("degraded report should surface failed server");
    assert_eq!(degraded.failed_servers[0].server_name, "broken");

    mcp_runtime.shutdown().expect("mcp shutdown should succeed");
    cleanup_mcp_runtime_fixture(&config_home, &workspace);
}

#[test]
fn managed_mcp_runtime_dispatches_direct_calls_without_wrapper_passthroughs() {
    let (config_home, workspace, mut mcp_runtime) = setup_managed_mcp_runtime_fixture(false);

    let direct = mcp_runtime
        .execute_tool("mcp__alpha__echo", json!({"text":"hello"}))
        .expect("direct discovered mcp tool should execute");
    let direct_json: serde_json::Value =
        serde_json::from_str(&direct).expect("direct output should be json");
    assert_eq!(direct_json["structuredContent"]["echoed"], "hello");

    let wrapper_error = mcp_runtime
        .execute_tool(
            "MCPTool",
            json!({
                "qualifiedName": "mcp__alpha__echo",
                "arguments": { "text": "wrapped" }
            }),
        )
        .expect_err("wrapper mcp tool should no longer be dispatchable");
    assert!(wrapper_error.to_string().contains("MCPTool"));

    mcp_runtime.shutdown().expect("mcp shutdown should succeed");
    cleanup_mcp_runtime_fixture(&config_home, &workspace);
}

#[test]
fn managed_mcp_runtime_discovers_and_executes_prompt_capabilities() {
    let (config_home, workspace, mut mcp_runtime) = setup_managed_mcp_runtime_fixture(false);

    let prompt_capability = mcp_runtime
        .provided_capabilities()
        .into_iter()
        .find(|capability| {
            capability.source_kind == super::CapabilitySourceKind::McpPrompt
                && capability.execution_kind == super::CapabilityExecutionKind::PromptSkill
        })
        .expect("managed runtime should expose MCP prompt capabilities");

    let executed = mcp_runtime
        .execute_prompt_skill(&prompt_capability, Some(json!({"topic": "workspace"})))
        .expect("managed runtime should execute MCP prompts as prompt skills");

    assert_eq!(executed.skill, prompt_capability.display_name);
    assert!(executed
        .injected_system_sections()
        .iter()
        .any(|message| message.contains("MCP workspace guidance")));

    mcp_runtime.shutdown().expect("mcp shutdown should succeed");
    cleanup_mcp_runtime_fixture(&config_home, &workspace);
}

#[test]
fn session_exposure_snapshot_controls_deferred_visibility_and_search_state() {
    let runtime = CapabilityRuntime::builtin();
    let profile = BTreeSet::from([String::from("ToolSearch"), String::from("WebSearch")]);
    let mut state = super::SessionCapabilityState::default();

    let before = runtime
        .surface_projection(super::CapabilityPlannerInput::new(
            Some(&profile),
            Some(&state),
        ))
        .expect("planner surface should resolve");
    assert!(before
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "ToolSearch"));
    assert!(!before
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));
    assert!(before
        .deferred_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));

    let search = runtime.search(
        "select:WebSearch",
        5,
        super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
        None,
        None,
    );
    let search_json = serde_json::to_value(search).expect("search output should serialize");
    assert_eq!(search_json["matches"][0], "WebSearch");
    assert_eq!(search_json["results"][0]["discovered"], false);
    assert_eq!(search_json["results"][0]["activated"], false);
    assert_eq!(search_json["results"][0]["exposed"], false);

    state.discover_tool("WebSearch");
    state.activate_discovered_tool("WebSearch");

    let activated_only = runtime
        .surface_projection(super::CapabilityPlannerInput::new(
            Some(&profile),
            Some(&state),
        ))
        .expect("planner surface should resolve after activation");
    assert!(!activated_only
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));
    assert!(activated_only
        .deferred_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));

    let activated_search = runtime.search(
        "web",
        5,
        super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
        None,
        None,
    );
    let activated_search_json =
        serde_json::to_value(activated_search).expect("search output should serialize");
    assert_eq!(activated_search_json["results"][0]["discovered"], true);
    assert_eq!(activated_search_json["results"][0]["activated"], true);
    assert_eq!(activated_search_json["results"][0]["exposed"], false);

    state.expose_tool("WebSearch");

    let after = runtime
        .surface_projection(super::CapabilityPlannerInput::new(
            Some(&profile),
            Some(&state),
        ))
        .expect("planner surface should resolve after exposure");
    assert!(after
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));
    assert!(!after
        .deferred_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));

    let after_search = runtime.search(
        "web",
        5,
        super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
        None,
        None,
    );
    let after_search_json =
        serde_json::to_value(after_search).expect("search output should serialize");
    assert!(after_search_json["matches"]
        .as_array()
        .expect("matches should be present")
        .iter()
        .any(|value| value == "WebSearch"));
    let exposed_entry = after_search_json["results"]
        .as_array()
        .expect("results should be present")
        .iter()
        .find(|entry| entry["name"] == "WebSearch")
        .expect("WebSearch should remain searchable with exposure metadata");
    assert_eq!(exposed_entry["discovered"], true);
    assert_eq!(exposed_entry["activated"], true);
    assert_eq!(exposed_entry["exposed"], true);
}

#[test]
fn web_fetch_returns_prompt_aware_summary() {
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.starts_with("GET /page "));
        HttpResponse::html(
            200,
            "OK",
            "<html><head><title>Ignored</title></head><body><h1>Test Page</h1><p>Hello <b>world</b> from local server.</p></body></html>",
        )
    }));

    let result = execute_tool(
        "WebFetch",
        &json!({
            "url": format!("http://{}/page", server.addr()),
            "prompt": "Summarize this page"
        }),
    )
    .expect("WebFetch should succeed");

    let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(output["code"], 200);
    let summary = output["result"].as_str().expect("result string");
    assert!(summary.contains("Fetched"));
    assert!(summary.contains("Test Page"));
    assert!(summary.contains("Hello world from local server"));

    let titled = execute_tool(
        "WebFetch",
        &json!({
            "url": format!("http://{}/page", server.addr()),
            "prompt": "What is the page title?"
        }),
    )
    .expect("WebFetch title query should succeed");
    let titled_output: serde_json::Value = serde_json::from_str(&titled).expect("valid json");
    let titled_summary = titled_output["result"].as_str().expect("result string");
    assert!(titled_summary.contains("Title: Ignored"));
}

#[test]
fn web_fetch_supports_plain_text_and_rejects_invalid_url() {
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.starts_with("GET /plain "));
        HttpResponse::text(200, "OK", "plain text response")
    }));

    let result = execute_tool(
        "WebFetch",
        &json!({
            "url": format!("http://{}/plain", server.addr()),
            "prompt": "Show me the content"
        }),
    )
    .expect("WebFetch should succeed for text content");

    let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(output["url"], format!("http://{}/plain", server.addr()));
    assert!(output["result"]
        .as_str()
        .expect("result")
        .contains("plain text response"));

    let error = execute_tool(
        "WebFetch",
        &json!({
            "url": "not a url",
            "prompt": "Summarize"
        }),
    )
    .expect_err("invalid URL should fail");
    assert!(error.contains("relative URL without a base") || error.contains("invalid"));
}

#[test]
fn web_search_extracts_and_filters_results() {
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.contains("GET /search?q=rust+web+search "));
        HttpResponse::html(
            200,
            "OK",
            r#"
                <html><body>
                  <a class="result__a" href="https://docs.rs/reqwest">Reqwest docs</a>
                  <a class="result__a" href="https://example.com/blocked">Blocked result</a>
                </body></html>
                "#,
        )
    }));

    std::env::set_var(
        "CLAWD_WEB_SEARCH_BASE_URL",
        format!("http://{}/search", server.addr()),
    );
    let result = execute_tool(
        "WebSearch",
        &json!({
            "query": "rust web search",
            "allowed_domains": ["https://DOCS.rs/"],
            "blocked_domains": ["HTTPS://EXAMPLE.COM"]
        }),
    )
    .expect("WebSearch should succeed");
    std::env::remove_var("CLAWD_WEB_SEARCH_BASE_URL");

    let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(output["query"], "rust web search");
    let results = output["results"].as_array().expect("results array");
    let search_result = results
        .iter()
        .find(|item| item.get("content").is_some())
        .expect("search result block present");
    let content = search_result["content"].as_array().expect("content array");
    assert_eq!(content.len(), 1);
    assert_eq!(content[0]["title"], "Reqwest docs");
    assert_eq!(content[0]["url"], "https://docs.rs/reqwest");
}

#[test]
fn web_search_handles_generic_links_and_invalid_base_url() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let server = TestServer::spawn(Arc::new(|request_line: &str| {
        assert!(request_line.contains("GET /fallback?q=generic+links "));
        HttpResponse::html(
            200,
            "OK",
            r#"
                <html><body>
                  <a href="https://example.com/one">Example One</a>
                  <a href="https://example.com/one">Duplicate Example One</a>
                  <a href="https://docs.rs/tokio">Tokio Docs</a>
                </body></html>
                "#,
        )
    }));

    std::env::set_var(
        "CLAWD_WEB_SEARCH_BASE_URL",
        format!("http://{}/fallback", server.addr()),
    );
    let result = execute_tool(
        "WebSearch",
        &json!({
            "query": "generic links"
        }),
    )
    .expect("WebSearch fallback parsing should succeed");
    std::env::remove_var("CLAWD_WEB_SEARCH_BASE_URL");

    let output: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let results = output["results"].as_array().expect("results array");
    let search_result = results
        .iter()
        .find(|item| item.get("content").is_some())
        .expect("search result block present");
    let content = search_result["content"].as_array().expect("content array");
    assert_eq!(content.len(), 2);
    assert_eq!(content[0]["url"], "https://example.com/one");
    assert_eq!(content[1]["url"], "https://docs.rs/tokio");

    std::env::set_var("CLAWD_WEB_SEARCH_BASE_URL", "://bad-base-url");
    let error = execute_tool("WebSearch", &json!({ "query": "generic links" }))
        .expect_err("invalid base URL should fail");
    std::env::remove_var("CLAWD_WEB_SEARCH_BASE_URL");
    assert!(error.contains("relative URL without a base") || error.contains("empty host"));
}

#[test]
fn pending_tools_preserve_multiple_streaming_tool_calls_by_index() {
    let mut events = Vec::new();
    let mut pending_tools = BTreeMap::new();

    push_output_block(
        OutputContentBlock::ToolUse {
            id: "tool-1".to_string(),
            name: "read_file".to_string(),
            input: json!({}),
        },
        1,
        &mut events,
        &mut pending_tools,
        true,
    );
    push_output_block(
        OutputContentBlock::ToolUse {
            id: "tool-2".to_string(),
            name: "grep_search".to_string(),
            input: json!({}),
        },
        2,
        &mut events,
        &mut pending_tools,
        true,
    );

    pending_tools
        .get_mut(&1)
        .expect("first tool pending")
        .2
        .push_str("{\"path\":\"src/main.rs\"}");
    pending_tools
        .get_mut(&2)
        .expect("second tool pending")
        .2
        .push_str("{\"pattern\":\"TODO\"}");

    assert_eq!(
        pending_tools.remove(&1),
        Some((
            "tool-1".to_string(),
            "read_file".to_string(),
            "{\"path\":\"src/main.rs\"}".to_string(),
        ))
    );
    assert_eq!(
        pending_tools.remove(&2),
        Some((
            "tool-2".to_string(),
            "grep_search".to_string(),
            "{\"pattern\":\"TODO\"}".to_string(),
        ))
    );
}

#[test]
fn todo_write_persists_and_returns_previous_state() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = temp_path("todos.json");
    std::env::set_var("CLAWD_TODO_STORE", &path);

    let first = execute_tool(
        "TodoWrite",
        &json!({
            "todos": [
                {"content": "Add tool", "activeForm": "Adding tool", "status": "in_progress"},
                {"content": "Run tests", "activeForm": "Running tests", "status": "pending"}
            ]
        }),
    )
    .expect("TodoWrite should succeed");
    let first_output: serde_json::Value = serde_json::from_str(&first).expect("valid json");
    assert_eq!(first_output["oldTodos"].as_array().expect("array").len(), 0);

    let second = execute_tool(
        "TodoWrite",
        &json!({
            "todos": [
                {"content": "Add tool", "activeForm": "Adding tool", "status": "completed"},
                {"content": "Run tests", "activeForm": "Running tests", "status": "completed"},
                {"content": "Verify", "activeForm": "Verifying", "status": "completed"}
            ]
        }),
    )
    .expect("TodoWrite should succeed");
    std::env::remove_var("CLAWD_TODO_STORE");
    let _ = std::fs::remove_file(path);

    let second_output: serde_json::Value = serde_json::from_str(&second).expect("valid json");
    assert_eq!(
        second_output["oldTodos"].as_array().expect("array").len(),
        2
    );
    assert_eq!(
        second_output["newTodos"].as_array().expect("array").len(),
        3
    );
    assert!(second_output["verificationNudgeNeeded"].is_null());
}

#[test]
fn todo_write_rejects_invalid_payloads_and_sets_verification_nudge() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = temp_path("todos-errors.json");
    std::env::set_var("CLAWD_TODO_STORE", &path);

    let empty =
        execute_tool("TodoWrite", &json!({ "todos": [] })).expect_err("empty todos should fail");
    assert!(empty.contains("todos must not be empty"));

    // Multiple in_progress items are now allowed for parallel workflows
    let _multi_active = execute_tool(
        "TodoWrite",
        &json!({
            "todos": [
                {"content": "One", "activeForm": "Doing one", "status": "in_progress"},
                {"content": "Two", "activeForm": "Doing two", "status": "in_progress"}
            ]
        }),
    )
    .expect("multiple in-progress todos should succeed");

    let blank_content = execute_tool(
        "TodoWrite",
        &json!({
            "todos": [
                {"content": "   ", "activeForm": "Doing it", "status": "pending"}
            ]
        }),
    )
    .expect_err("blank content should fail");
    assert!(blank_content.contains("todo content must not be empty"));

    let nudge = execute_tool(
        "TodoWrite",
        &json!({
            "todos": [
                {"content": "Write tests", "activeForm": "Writing tests", "status": "completed"},
                {"content": "Fix errors", "activeForm": "Fixing errors", "status": "completed"},
                {"content": "Ship branch", "activeForm": "Shipping branch", "status": "completed"}
            ]
        }),
    )
    .expect("completed todos should succeed");
    std::env::remove_var("CLAWD_TODO_STORE");
    let _ = fs::remove_file(path);

    let output: serde_json::Value = serde_json::from_str(&nudge).expect("valid json");
    assert_eq!(output["verificationNudgeNeeded"], true);
}

#[test]
fn legacy_skill_shim_is_removed_from_builtin_dispatch() {
    let error = execute_tool(
        "Skill",
        &json!({
            "skill": "help"
        }),
    )
    .expect_err("legacy Skill shim should no longer be dispatchable");
    assert!(error.contains("unsupported tool: Skill"));
}

#[test]
fn legacy_mcp_wrappers_are_removed_from_builtin_dispatch() {
    for (tool_name, input) in [
        ("MCP", json!({"server": "alpha", "tool": "echo"})),
        ("ListMcpResources", json!({"server": "alpha"})),
        (
            "ReadMcpResource",
            json!({"server": "alpha", "uri": "file://guide.txt"}),
        ),
    ] {
        let error = execute_tool(tool_name, &input)
            .expect_err("legacy MCP wrappers should no longer be dispatchable");
        assert!(
            error.contains(&format!("unsupported tool: {tool_name}")),
            "unexpected error for {tool_name}: {error}"
        );
    }
}

#[test]
fn tool_search_supports_keyword_and_select_queries() {
    let keyword = execute_tool(
        "ToolSearch",
        &json!({"query": "web current", "max_results": 3}),
    )
    .expect("ToolSearch should succeed");
    let keyword_output: serde_json::Value = serde_json::from_str(&keyword).expect("valid json");
    let matches = keyword_output["matches"].as_array().expect("matches");
    assert!(matches.iter().any(|value| value == "WebSearch"));

    let selected = execute_tool("ToolSearch", &json!({"query": "select:WebFetch,WebSearch"}))
        .expect("ToolSearch should succeed");
    let selected_output: serde_json::Value = serde_json::from_str(&selected).expect("valid json");
    assert_eq!(selected_output["matches"][0], "WebFetch");
    assert_eq!(selected_output["matches"][1], "WebSearch");

    let aliased = execute_tool("ToolSearch", &json!({"query": "WebSearchTool"}))
        .expect("ToolSearch should support tool aliases");
    let aliased_output: serde_json::Value = serde_json::from_str(&aliased).expect("valid json");
    assert_eq!(aliased_output["matches"][0], "WebSearch");
    assert_eq!(aliased_output["normalized_query"], "websearch");

    let selected_with_alias = execute_tool(
        "ToolSearch",
        &json!({"query": "select:WebSearchTool,WebFetchTool"}),
    )
    .expect("ToolSearch alias select should succeed");
    let selected_with_alias_output: serde_json::Value =
        serde_json::from_str(&selected_with_alias).expect("valid json");
    assert_eq!(selected_with_alias_output["matches"][0], "WebSearch");
    assert_eq!(selected_with_alias_output["matches"][1], "WebFetch");
}

#[test]
fn skill_discovery_lists_only_model_invocable_skills() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("skill-discovery-home");
    let bundled_root = temp_path("skill-discovery-bundled-root");
    let executable_skill_dir = home.join(".agents").join("skills").join("help");
    let doc_skill_dir = home.join(".agents").join("skills").join("reference");
    fs::create_dir_all(&executable_skill_dir).expect("executable skill dir should exist");
    fs::create_dir_all(&doc_skill_dir).expect("doc skill dir should exist");
    fs::create_dir_all(&bundled_root).expect("bundled root should exist");
    fs::write(
        executable_skill_dir.join("SKILL.md"),
        r"---
name: help
description: Help the model decide when to use the workspace guidance skill.
when_to_use: Use when the task asks for workspace orientation.
allowed-tools:
  - WebSearch
model-invocable: true
user-invocable: true
context: inline
---
# help

Guide the model through the workspace.
",
    )
    .expect("executable skill file should exist");
    fs::write(
        doc_skill_dir.join("SKILL.md"),
        r"---
name: reference
description: Reference-only skill that should not be model invocable.
model-invocable: false
---
# reference

Reference notes only.
",
    )
    .expect("doc skill file should exist");

    let _home_restore = override_env_var("HOME", home.as_os_str());
    let _codex_restore = override_env_var("CODEX_HOME", home.join(".codex").into_os_string());
    let _claw_restore = override_env_var("CLAW_CONFIG_HOME", home.join(".claw").into_os_string());
    let _bundled_roots_restore =
        override_env_var("OCTOPUS_BUNDLED_SKILLS_ROOTS", bundled_root.as_os_str());

    let capability_runtime = CapabilityRuntime::builtin();
    let discovered = serde_json::to_string_pretty(&capability_runtime.skill_discovery(
        "workspace guidance",
        5,
        super::CapabilityPlannerInput::default(),
    ))
    .expect("skill discovery should succeed");

    let output: serde_json::Value = serde_json::from_str(&discovered).expect("valid json");
    assert_eq!(output["matches"][0], "help");
    let results = output["results"].as_array().expect("results");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0]["name"], "help");
    assert_eq!(results[0]["source_kind"], "local_skill");
    assert_eq!(results[0]["execution_kind"], "prompt_skill");
    assert_eq!(results[0]["tool_grants"][0], "WebSearch");
    fs::remove_dir_all(home).expect("temp home should clean up");
    fs::remove_dir_all(bundled_root).expect("temp bundled root should clean up");
}

#[test]
fn skill_discovery_surfaces_bundled_skills_with_distinct_source_kinds() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("skill-bundled-home");
    let bundled_root = temp_path("skill-bundled-root");
    let local_skill_dir = home.join(".agents").join("skills").join("local-help");
    let bundled_skill_dir = bundled_root.join("bundled-help");
    fs::create_dir_all(&local_skill_dir).expect("local skill dir should exist");
    fs::create_dir_all(&bundled_skill_dir).expect("bundled skill dir should exist");
    fs::write(
        local_skill_dir.join("SKILL.md"),
        r"---
name: local-help
description: Local workspace guidance skill.
model-invocable: true
user-invocable: true
---
# local-help

Local workspace guidance.
",
    )
    .expect("local skill file should exist");
    fs::write(
        bundled_skill_dir.join("SKILL.md"),
        r"---
name: bundled-help
description: Bundled workspace guidance skill.
model-invocable: true
user-invocable: true
---
# bundled-help

Bundled workspace guidance.
",
    )
    .expect("bundled skill file should exist");

    let original_home = std::env::var("HOME").ok();
    let original_bundled_roots = std::env::var("OCTOPUS_BUNDLED_SKILLS_ROOTS").ok();
    std::env::set_var("HOME", &home);
    std::env::set_var("OCTOPUS_BUNDLED_SKILLS_ROOTS", &bundled_root);

    let discovery = CapabilityRuntime::builtin().skill_discovery(
        "workspace guidance",
        10,
        super::CapabilityPlannerInput::default(),
    );
    let output: serde_json::Value =
        serde_json::to_value(discovery).expect("skill discovery output should be json");
    let results = output["results"].as_array().expect("results");
    let sources = results
        .iter()
        .filter_map(|entry| {
            Some((
                entry.get("name")?.as_str()?.to_string(),
                entry.get("source_kind")?.as_str()?.to_string(),
            ))
        })
        .collect::<BTreeMap<_, _>>();

    assert_eq!(
        sources.get("local-help").map(String::as_str),
        Some("local_skill")
    );
    assert_eq!(
        sources.get("bundled-help").map(String::as_str),
        Some("bundled_skill")
    );

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    if let Some(bundled_roots) = original_bundled_roots {
        std::env::set_var("OCTOPUS_BUNDLED_SKILLS_ROOTS", bundled_roots);
    } else {
        std::env::remove_var("OCTOPUS_BUNDLED_SKILLS_ROOTS");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
    fs::remove_dir_all(bundled_root).expect("temp bundled root should clean up");
}

#[test]
fn skill_discovery_trust_gates_local_skills_but_keeps_bundled_skills_visible() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("skill-trust-home");
    let bundled_root = temp_path("skill-trust-bundled");
    let cwd = temp_path("skill-trust-cwd");
    let local_skill_dir = home.join(".agents").join("skills").join("local-help");
    let bundled_skill_dir = bundled_root.join("bundled-help");
    fs::create_dir_all(&local_skill_dir).expect("local skill dir should exist");
    fs::create_dir_all(&bundled_skill_dir).expect("bundled skill dir should exist");
    fs::create_dir_all(cwd.join(".claw")).expect("config dir should exist");
    fs::write(
        cwd.join(".claw").join("settings.json"),
        r#"{"trustedRoots":["/definitely/not-this-workspace"]}"#,
    )
    .expect("workspace settings should exist");
    fs::write(
        local_skill_dir.join("SKILL.md"),
        r"---
name: local-help
description: Local workspace guidance skill.
model-invocable: true
user-invocable: true
---
# local-help

Local workspace guidance.
",
    )
    .expect("local skill file should exist");
    fs::write(
        bundled_skill_dir.join("SKILL.md"),
        r"---
name: bundled-help
description: Bundled workspace guidance skill.
model-invocable: true
user-invocable: true
---
# bundled-help

Bundled workspace guidance.
",
    )
    .expect("bundled skill file should exist");

    let original_home = std::env::var("HOME").ok();
    let original_bundled_roots = std::env::var("OCTOPUS_BUNDLED_SKILLS_ROOTS").ok();
    let original_cwd = std::env::current_dir().expect("current dir");
    std::env::set_var("HOME", &home);
    std::env::set_var("OCTOPUS_BUNDLED_SKILLS_ROOTS", &bundled_root);
    std::env::set_current_dir(&cwd).expect("cwd should switch");

    let discovery = CapabilityRuntime::builtin().skill_discovery(
        "workspace guidance",
        10,
        super::CapabilityPlannerInput::default().with_current_dir(Some(cwd.as_path())),
    );
    let output: serde_json::Value =
        serde_json::to_value(discovery).expect("skill discovery output should be json");
    let matches = output["matches"].as_array().expect("matches");

    assert!(!matches.iter().any(|value| value == "local-help"));
    assert!(matches.iter().any(|value| value == "bundled-help"));

    std::env::set_current_dir(original_cwd).expect("cwd should restore");
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    if let Some(bundled_roots) = original_bundled_roots {
        std::env::set_var("OCTOPUS_BUNDLED_SKILLS_ROOTS", bundled_roots);
    } else {
        std::env::remove_var("OCTOPUS_BUNDLED_SKILLS_ROOTS");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
    fs::remove_dir_all(bundled_root).expect("temp bundled root should clean up");
    fs::remove_dir_all(cwd).expect("temp cwd should clean up");
}

#[test]
fn provider_prompt_skills_without_runtime_executors_stay_hidden_from_skill_discovery() {
    let mut capability = provider_prompt_skill_capability(
        "plugin-skill.workspace-guide",
        super::CapabilitySourceKind::PluginSkill,
        "workspace-guide",
    );
    capability.executor_key = None;
    let runtime = capability_runtime_with_provided_capabilities(vec![capability]);

    let surface = runtime
        .surface_projection(super::CapabilityPlannerInput::default())
        .expect("planner should project a capability surface");
    assert!(!surface
        .discoverable_skills
        .iter()
        .any(|skill| skill.display_name == "workspace-guide"));
    assert!(surface
        .hidden_capabilities
        .iter()
        .any(|skill| skill.display_name == "workspace-guide"));

    let discovery = runtime.skill_discovery(
        "workspace guidance",
        10,
        super::CapabilityPlannerInput::default(),
    );
    let output: serde_json::Value =
        serde_json::to_value(discovery).expect("skill discovery output should be json");
    let matches = output["matches"].as_array().expect("matches");
    let results = output["results"].as_array().expect("results");

    assert!(!matches.iter().any(|value| value == "workspace-guide"));
    assert!(!results
        .iter()
        .any(|entry| entry["name"] == "workspace-guide"));
}

#[test]
fn provider_prompt_skills_with_registered_runtime_executors_are_discoverable_and_executable() {
    let capability = provider_prompt_skill_capability(
        "plugin-skill.workspace-guide",
        super::CapabilitySourceKind::PluginSkill,
        "workspace-guide",
    );
    let runtime = capability_runtime_with_provided_capabilities(vec![capability.clone()]);
    runtime.register_prompt_skill_executor(
        capability
            .executor_key
            .clone()
            .expect("provider prompt skill should have executor key"),
        |_capability, arguments, _current_dir| {
            Ok(super::SkillExecutionResult {
                skill: "workspace-guide".to_string(),
                path: "plugin://workspace-guide".to_string(),
                description: Some("Provider-backed workspace guidance skill.".to_string()),
                context: super::skill_runtime::SkillContextKind::Inline,
                messages_to_inject: vec![super::skill_runtime::SkillInjectedMessage::system(
                    format!(
                        "Provider workspace guidance\n{}",
                        serde_json::to_string(&arguments.unwrap_or_default()).expect("json")
                    ),
                )],
                tool_grants: vec!["WebSearch".to_string()],
                model_override: Some("claude-opus-4-6".to_string()),
                effort_override: Some("high".to_string()),
                state_updates: vec![super::SkillStateUpdate::ContextPrepared {
                    context: super::skill_runtime::SkillContextKind::Inline,
                }],
            })
        },
    );
    let store = super::SessionCapabilityStore::default();

    let surface = runtime
        .surface_projection(super::CapabilityPlannerInput::default())
        .expect("planner should project a capability surface");
    assert!(surface
        .discoverable_skills
        .iter()
        .any(|skill| skill.display_name == "workspace-guide"));

    let discovery = runtime.skill_discovery(
        "workspace guidance",
        10,
        super::CapabilityPlannerInput::default(),
    );
    let output: serde_json::Value =
        serde_json::to_value(discovery).expect("skill discovery output should be json");
    assert!(output["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .any(|value| value == "workspace-guide"));

    let executed = runtime
        .execute_skill_detailed(
            "workspace-guide",
            Some(json!({ "topic": "workspace" })),
            super::CapabilityPlannerInput::default(),
        )
        .expect("provider-backed prompt skill should execute");
    store.apply_skill_execution_result(&executed);
    let executed: serde_json::Value =
        serde_json::to_value(&executed).expect("skill execution output should be json");
    assert_eq!(executed["skill"], "workspace-guide");
    assert_eq!(executed["tool_grants"][0], "WebSearch");
    assert_eq!(executed["model_override"], "claude-opus-4-6");
    assert_eq!(executed["effort_override"], "high");

    let snapshot = store.snapshot();
    assert!(snapshot.is_tool_granted("WebSearch"));
    assert_eq!(snapshot.model_override(), Some("claude-opus-4-6"));
    assert_eq!(snapshot.effort_override(), Some("high"));
    assert!(snapshot
        .injected_skill_messages()
        .iter()
        .any(|message| message.contains("Provider workspace guidance")));
}

#[test]
fn capability_runtime_execute_tool_surface_gates_provider_prompt_skill_without_runtime_executor() {
    let mut capability = provider_prompt_skill_capability(
        "plugin-skill.workspace-guide",
        super::CapabilitySourceKind::PluginSkill,
        "workspace-guide",
    );
    capability.executor_key = None;
    let runtime = capability_runtime_with_provided_capabilities(vec![capability]);
    let store = super::SessionCapabilityStore::default();

    let error = runtime
        .execute_skill(
            "workspace-guide",
            Some(json!({ "topic": "workspace" })),
            super::CapabilityPlannerInput::default(),
        )
        .expect_err("provider-backed prompt skills without executors should be surface gated");

    assert!(error.contains("is not enabled in the current capability surface"));
    let snapshot = store.snapshot();
    assert!(snapshot.skill_state_updates().is_empty());
    assert!(snapshot.injected_skill_messages().is_empty());
    assert_eq!(snapshot.model_override(), None);
    assert_eq!(snapshot.effort_override(), None);
}

#[test]
fn capability_runtime_execute_tool_reports_hidden_provider_prompt_skill_as_surface_gated() {
    let mut capability = provider_prompt_skill_capability(
        "plugin-skill.workspace-guide-hidden",
        super::CapabilitySourceKind::PluginSkill,
        "workspace-guide-hidden",
    );
    capability.visibility = super::CapabilityVisibility::Hidden;
    capability.executor_key = None;
    let runtime = capability_runtime_with_provided_capabilities(vec![capability]);
    let error = runtime
        .execute_skill(
            "workspace-guide-hidden",
            Some(json!({ "topic": "workspace" })),
            super::CapabilityPlannerInput::default(),
        )
        .expect_err("hidden provider prompt skills should be surface gated");

    assert!(error.contains("is not enabled in the current capability surface"));
}

#[test]
fn skill_discovery_hides_non_selectable_provider_prompt_skills() {
    let mut capability = provider_prompt_skill_capability(
        "plugin-skill.workspace-guide-disabled",
        super::CapabilitySourceKind::PluginSkill,
        "workspace-guide-disabled",
    );
    capability.invocation_policy.selectable = false;
    let runtime = capability_runtime_with_provided_capabilities(vec![capability]);

    let surface = runtime
        .surface_projection(super::CapabilityPlannerInput::default())
        .expect("planner should project a capability surface");
    assert!(!surface
        .discoverable_skills
        .iter()
        .any(|skill| skill.display_name == "workspace-guide-disabled"));
    assert!(surface
        .hidden_capabilities
        .iter()
        .any(|skill| skill.display_name == "workspace-guide-disabled"));

    let discovery = runtime.skill_discovery(
        "workspace guidance",
        10,
        super::CapabilityPlannerInput::default(),
    );
    let output: serde_json::Value =
        serde_json::to_value(discovery).expect("skill discovery output should be json");
    let matches = output["matches"].as_array().expect("matches");
    assert!(!matches
        .iter()
        .any(|value| value == "workspace-guide-disabled"));
}

#[test]
fn skill_discovery_hides_provider_prompt_skills_without_runtime_executors() {
    let mut capability = provider_prompt_skill_capability(
        "plugin-skill.workspace-guide-compat",
        super::CapabilitySourceKind::PluginSkill,
        "workspace-guide-compat",
    );
    capability.executor_key = None;
    let runtime = capability_runtime_with_provided_capabilities(vec![capability]);

    let output = discover_skills_with_runtime(&runtime, "workspace guidance", 10);

    assert!(!output["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .any(|value| value == "workspace-guide-compat"));
}

#[test]
fn prompt_skill_execution_surface_gates_provider_prompt_skills_without_runtime_executors() {
    let mut capability = provider_prompt_skill_capability(
        "plugin-skill.workspace-guide-compat",
        super::CapabilitySourceKind::PluginSkill,
        "workspace-guide-compat",
    );
    capability.executor_key = None;
    let runtime = capability_runtime_with_provided_capabilities(vec![capability]);

    let error = execute_prompt_skill_with_runtime(
        &runtime,
        "workspace-guide-compat",
        Some(json!({ "topic": "workspace" })),
    )
    .expect_err("prompt skill execution should report surface gating");

    assert!(error.contains("is not enabled in the current capability surface"));
}

#[test]
fn prompt_skill_runtime_does_not_bypass_workspace_gating() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("compat-skill-gating-home");
    let cwd = temp_path("compat-skill-gating-cwd");
    let skill_dir = home
        .join(".agents")
        .join("skills")
        .join("workspace-guide-compat-local");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::create_dir_all(cwd.join(".claw")).expect("workspace config dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: workspace-guide-compat-local
description: Workspace-scoped compat skill.
model-invocable: true
user-invocable: true
paths:
  - package.json
context: inline
---
# workspace-guide-compat-local

Scoped workspace guidance.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    let original_cwd = std::env::current_dir().expect("current dir");
    std::env::set_var("HOME", &home);
    std::env::set_current_dir(&cwd).expect("cwd should switch");

    let runtime = CapabilityRuntime::builtin();

    let path_mismatch_discovery = discover_skills_with_runtime(&runtime, "workspace guidance", 10);
    assert!(!path_mismatch_discovery["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .any(|value| value == "workspace-guide-compat-local"));

    let path_mismatch_error = execute_prompt_skill_with_runtime(
        &runtime,
        "workspace-guide-compat-local",
        Some(json!({ "topic": "workspace" })),
    )
    .expect_err("prompt skill execution should stay surface-gated when paths do not match");
    assert!(path_mismatch_error.contains("is not visible for the current workspace"));

    fs::write(
        cwd.join("package.json"),
        r#"{"name":"compat-skill-gating"}"#,
    )
    .expect("package.json should exist");
    fs::write(
        cwd.join(".claw").join("settings.json"),
        r#"{"trustedRoots":["/definitely/not-this-workspace"]}"#,
    )
    .expect("workspace settings should exist");

    let trust_mismatch_discovery = discover_skills_with_runtime(&runtime, "workspace guidance", 10);
    assert!(!trust_mismatch_discovery["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .any(|value| value == "workspace-guide-compat-local"));

    let trust_mismatch_error = execute_prompt_skill_with_runtime(
        &runtime,
        "workspace-guide-compat-local",
        Some(json!({ "topic": "workspace" })),
    )
    .expect_err("prompt skill execution should stay surface-gated when workspace is untrusted");
    assert!(trust_mismatch_error.contains("is not trusted for the current workspace"));

    std::env::set_current_dir(original_cwd).expect("cwd should restore");
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
    fs::remove_dir_all(cwd).expect("temp cwd should clean up");
}

#[test]
fn skill_tool_rejects_model_non_invocable_skills() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("skill-model-invocable-home");
    let skill_dir = home.join(".agents").join("skills").join("reference");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: reference
description: Reference-only user skill.
model-invocable: false
user-invocable: true
context: inline
---
# reference

Reference notes only.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &home);

    let error = CapabilityRuntime::builtin()
        .execute_skill("reference", None, super::CapabilityPlannerInput::default())
        .expect_err("model-only skill execution should reject non-model-invocable skills");
    assert!(error.contains("not model invocable"));

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
}

fn noop_skill_fork_spawn(_job: super::AgentJob) -> Result<(), String> {
    Ok(())
}

fn fail_skill_fork_spawn(_job: super::AgentJob) -> Result<(), String> {
    Err(String::from("thread creation failed"))
}

#[test]
fn skill_tool_fork_context_spawns_structured_agent_state() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("skill-fork-home");
    let agent_store = temp_path("skill-fork-agent-store");
    let skill_dir = home.join(".agents").join("skills").join("planner");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: planner
description: Fork planning guidance into a dedicated subagent.
allowed-tools:
  - WebSearch
model-invocable: true
user-invocable: true
agent: plan
model: claude-sonnet-4-5
effort: high
context: fork
---
# planner

Build a plan for the provided task.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    let original_agent_store = std::env::var("CLAWD_AGENT_STORE").ok();
    std::env::set_var("HOME", &home);
    std::env::set_var("CLAWD_AGENT_STORE", &agent_store);
    super::skill_runtime::set_skill_fork_spawn_override(Some(noop_skill_fork_spawn));

    let result = CapabilityRuntime::builtin()
        .execute_skill(
            "planner",
            Some(json!({"topic":"auth"})),
            super::CapabilityPlannerInput::default(),
        )
        .expect("fork skill should execute");

    assert_eq!(result.context, super::skill_runtime::SkillContextKind::Fork);
    assert!(result.messages_to_inject.is_empty());
    assert_eq!(result.tool_grants, vec![String::from("WebSearch")]);
    assert_eq!(result.model_override.as_deref(), Some("claude-sonnet-4-5"));
    assert_eq!(result.effort_override.as_deref(), Some("high"));

    let fork_spawn = result
        .state_updates
        .iter()
        .find_map(|update| match update {
            super::SkillStateUpdate::ForkSpawned {
                agent_id,
                subagent_type,
                output_file,
                manifest_file,
            } => Some((
                agent_id.clone(),
                subagent_type.clone(),
                output_file.clone(),
                manifest_file.clone(),
            )),
            _ => None,
        })
        .expect("fork skill should emit fork_spawned state");
    assert!(!fork_spawn.0.is_empty());
    assert_eq!(fork_spawn.1.as_deref(), Some("Plan"));
    assert!(Path::new(&fork_spawn.2).exists());
    assert!(Path::new(&fork_spawn.3).exists());

    super::skill_runtime::set_skill_fork_spawn_override(None);
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    if let Some(agent_store) = original_agent_store {
        std::env::set_var("CLAWD_AGENT_STORE", agent_store);
    } else {
        std::env::remove_var("CLAWD_AGENT_STORE");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
    fs::remove_dir_all(agent_store).expect("temp agent store should clean up");
}

#[test]
fn capability_surface_projects_prompt_skills_separately_from_tool_surface() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("skill-surface-home");
    let skill_dir = home.join(".agents").join("skills").join("help");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: help
description: Help the model decide when to use the workspace guidance skill.
when_to_use: Use when the task asks for workspace orientation.
allowed-tools:
  - WebSearch
model-invocable: true
user-invocable: true
context: inline
---
# help

Guide the model through the workspace.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &home);

    let surface = CapabilityRuntime::builtin()
        .surface_projection_for_allowlist(None, None)
        .expect("builtin capabilities should plan");

    assert!(surface
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "read_file"));
    assert!(surface
        .discoverable_skills
        .iter()
        .any(|capability| capability.display_name == "help"));
    assert!(surface.available_resources.is_empty());
    assert!(!surface
        .hidden_capabilities
        .iter()
        .any(|capability| capability.display_name == "help"));

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
}

#[test]
fn capability_runtime_facade_projects_surface_and_search_from_provider() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("capability-runtime-facade-home");
    let skill_dir = home.join(".agents").join("skills").join("help");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: help
description: Help the model decide when to use the workspace guidance skill.
when_to_use: Use when the task asks for workspace orientation.
allowed-tools:
  - WebSearch
model-invocable: true
user-invocable: true
context: inline
---
# help

Guide the model through the workspace.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &home);

    let runtime = CapabilityRuntime::builtin();
    let profile = BTreeSet::from([String::from("ToolSearch"), String::from("WebSearch")]);

    let surface = runtime
        .surface_projection(super::CapabilityPlannerInput::new(Some(&profile), None))
        .expect("runtime facade should project a capability surface");
    assert!(surface
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "ToolSearch"));
    assert!(surface
        .discoverable_skills
        .iter()
        .any(|capability| capability.display_name == "help"));
    assert!(surface
        .deferred_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));

    let search = runtime.search(
        "select:WebSearch",
        5,
        super::CapabilityPlannerInput::new(Some(&profile), None),
        None,
        None,
    );
    let search_json = serde_json::to_value(search).expect("search output should serialize");
    assert_eq!(search_json["matches"][0], "WebSearch");
    assert_eq!(search_json["results"][0]["source_kind"], "builtin");

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
}

#[test]
fn capability_runtime_execute_tool_applies_tool_search_activation() {
    let runtime = CapabilityRuntime::builtin();
    let store = super::SessionCapabilityStore::default();

    let output = runtime
        .execute_tool(
            "ToolSearch",
            json!({
                "query": "select:WebSearch",
                "max_results": 5
            }),
            super::CapabilityPlannerInput::default(),
            &store,
            None,
            None,
            |_kind, _tool_name, _input| {
                panic!("ToolSearch should execute inside capability runtime")
            },
        )
        .expect("ToolSearch should execute through runtime facade");
    let output_json: serde_json::Value =
        serde_json::from_str(&output).expect("tool search output should be json");

    assert_eq!(output_json["matches"][0], "WebSearch");
    assert!(store.snapshot().is_tool_activated("WebSearch"));
}

#[test]
fn capability_runtime_execute_tool_applies_skill_state_updates() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("runtime-skill-home");
    let skill_dir = home.join(".agents").join("skills").join("help");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: help
description: Help the model decide when to use the workspace guidance skill.
when_to_use: Use when the task asks for workspace orientation.
allowed-tools:
  - WebSearch
model-invocable: true
user-invocable: true
model: claude-sonnet-4-5
effort: high
context: inline
---
# help

Guide the model through the workspace.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &home);

    let runtime = CapabilityRuntime::builtin();
    let store = super::SessionCapabilityStore::default();

    let output = runtime
        .execute_skill_detailed(
            "help",
            Some(json!({
                "topic": "workspace"
            })),
            super::CapabilityPlannerInput::default(),
        )
        .expect("prompt skill should execute through runtime facade");
    store.apply_skill_execution_result(&output);
    let output_json: serde_json::Value =
        serde_json::to_value(&output).expect("skill tool output should be json");

    assert_eq!(output_json["skill"], "help");
    assert_eq!(output_json["tool_grants"][0], "WebSearch");
    let snapshot = store.snapshot();
    assert!(snapshot.is_tool_granted("WebSearch"));
    assert_eq!(snapshot.model_override(), Some("claude-sonnet-4-5"));
    assert_eq!(snapshot.effort_override(), Some("high"));
    assert!(snapshot
        .injected_skill_messages()
        .iter()
        .any(|message| message.contains("Guide the model through the workspace")));

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
}

#[test]
fn capability_runtime_execute_tool_persists_failed_fork_skill_state_updates() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("runtime-skill-fork-failure-home");
    let agent_store = temp_path("runtime-skill-fork-failure-agent-store");
    let skill_dir = home.join(".agents").join("skills").join("planner");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: planner
description: Fork planning guidance into a dedicated subagent.
allowed-tools:
  - WebSearch
model-invocable: true
user-invocable: true
agent: plan
model: claude-sonnet-4-5
effort: high
context: fork
---
# planner

Build a plan for the provided task.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    let original_agent_store = std::env::var("CLAWD_AGENT_STORE").ok();
    std::env::set_var("HOME", &home);
    std::env::set_var("CLAWD_AGENT_STORE", &agent_store);
    super::skill_runtime::set_skill_fork_spawn_override(Some(fail_skill_fork_spawn));

    let runtime = CapabilityRuntime::builtin();
    let store = super::SessionCapabilityStore::default();

    let error = runtime
        .execute_skill_detailed(
            "planner",
            Some(json!({
                "topic": "workspace auth"
            })),
            super::CapabilityPlannerInput::default(),
        )
        .expect_err("fork spawn failures should surface");
    store.apply_skill_state_updates(&error.state_updates);
    assert!(error.message.contains("failed to spawn sub-agent"));

    let raw_state = store.with_state(Clone::clone);
    assert!(!raw_state.is_tool_granted("WebSearch"));
    assert_eq!(raw_state.model_override(), None);
    assert_eq!(raw_state.effort_override(), None);

    let fork_spawn = raw_state
        .skill_state_updates()
        .iter()
        .find_map(|update| match update {
            super::SkillStateUpdate::ForkSpawned {
                agent_id,
                subagent_type,
                output_file,
                manifest_file,
            } => Some((
                agent_id.clone(),
                subagent_type.clone(),
                output_file.clone(),
                manifest_file.clone(),
            )),
            _ => None,
        })
        .expect("failed fork skill should still record fork_spawned");
    assert!(!fork_spawn.0.is_empty());
    assert_eq!(fork_spawn.1.as_deref(), Some("Plan"));
    assert!(Path::new(&fork_spawn.2).exists());
    assert!(Path::new(&fork_spawn.3).exists());
    assert!(raw_state
        .skill_state_updates()
        .contains(&super::SkillStateUpdate::ForkFailed {
            agent_id: fork_spawn.0,
            output_file: fork_spawn.2,
            manifest_file: fork_spawn.3,
            error: Some("failed to spawn sub-agent: thread creation failed".to_string()),
        }));

    super::skill_runtime::set_skill_fork_spawn_override(None);
    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    if let Some(agent_store) = original_agent_store {
        std::env::set_var("CLAWD_AGENT_STORE", agent_store);
    } else {
        std::env::remove_var("CLAWD_AGENT_STORE");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
    fs::remove_dir_all(agent_store).expect("temp agent store should clean up");
}

#[test]
fn capability_runtime_execute_tool_routes_runtime_capabilities_through_dispatch_kind() {
    let runtime = capability_runtime_with_runtime_tools(vec![super::RuntimeToolDefinition {
        name: "RuntimeEcho".to_string(),
        description: Some("Echo runtime payload.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            },
            "required": ["value"],
            "additionalProperties": false
        }),
        required_permission: runtime::PermissionMode::ReadOnly,
    }]);
    let store = super::SessionCapabilityStore::default();
    store.activate(super::CapabilityActivation::tool("RuntimeEcho"));
    let profile = BTreeSet::from([String::from("RuntimeEcho")]);
    let state = store.snapshot();
    let dispatched = Arc::new(Mutex::new(None::<(String, String, serde_json::Value)>));
    let captured = Arc::clone(&dispatched);

    let output = runtime
        .execute_tool(
            "RuntimeEcho",
            json!({ "value": "ok" }),
            super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
            &store,
            None,
            None,
            move |kind: super::CapabilityDispatchKind,
                  tool_name: &str,
                  input: serde_json::Value| {
                *captured
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) =
                    Some((format!("{kind:?}"), tool_name.to_string(), input.clone()));
                Ok("runtime dispatch".to_string())
            },
        )
        .expect("runtime capability should dispatch through facade");

    assert_eq!(output, "runtime dispatch");
    let dispatched = dispatched
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("runtime dispatch should be captured");
    assert_eq!(dispatched.0, "RuntimeCapability");
    assert_eq!(dispatched.1, "RuntimeEcho");
    assert_eq!(dispatched.2, json!({ "value": "ok" }));
}

#[test]
fn capability_runtime_execute_tool_serializes_non_read_only_dispatches() {
    let runtime = capability_runtime_with_runtime_tools(vec![super::RuntimeToolDefinition {
        name: "RuntimeWrite".to_string(),
        description: Some("Serialized runtime write.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            },
            "required": ["value"],
            "additionalProperties": false
        }),
        required_permission: runtime::PermissionMode::WorkspaceWrite,
    }]);
    let store = super::SessionCapabilityStore::default();
    store.activate(super::CapabilityActivation::tool("RuntimeWrite"));
    let profile = Arc::new(BTreeSet::from([String::from("RuntimeWrite")]));
    let state = Arc::new(store.snapshot());
    let start_barrier = Arc::new(Barrier::new(3));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for value in ["first", "second"] {
        let runtime = runtime.clone();
        let store = store.clone();
        let profile = Arc::clone(&profile);
        let state = Arc::clone(&state);
        let start_barrier = Arc::clone(&start_barrier);
        let active = Arc::clone(&active);
        let max_active = Arc::clone(&max_active);
        handles.push(thread::spawn(move || {
            start_barrier.wait();
            runtime
                .execute_tool(
                    "RuntimeWrite",
                    json!({ "value": value }),
                    super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
                    &store,
                    None,
                    None,
                    move |_dispatch_kind, _tool_name, _input| {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(current, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(120));
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok(value.to_string())
                    },
                )
                .expect("serialized runtime call should succeed")
        }));
    }

    let started = Instant::now();
    start_barrier.wait();
    let outputs = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread should finish"))
        .collect::<Vec<_>>();

    assert_eq!(outputs.len(), 2);
    assert_eq!(max_active.load(Ordering::SeqCst), 1);
    assert!(
        started.elapsed() >= Duration::from_millis(200),
        "serialized dispatches should not overlap"
    );
}

#[test]
fn capability_runtime_execute_tool_allows_parallel_read_dispatches() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("parallel-read-home");
    fs::create_dir_all(home.join(".agents").join("skills")).expect("temp skills dir should exist");
    let _home_restore = override_env_var("HOME", home.as_os_str());
    let _codex_restore = override_env_var("CODEX_HOME", home.join(".codex").into_os_string());
    let _claw_restore = override_env_var("CLAW_CONFIG_HOME", home.join(".claw").into_os_string());
    let runtime = capability_runtime_with_runtime_tools(vec![super::RuntimeToolDefinition {
        name: "RuntimeRead".to_string(),
        description: Some("Parallel runtime read.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            },
            "required": ["value"],
            "additionalProperties": false
        }),
        required_permission: runtime::PermissionMode::ReadOnly,
    }]);
    let store = super::SessionCapabilityStore::default();
    store.activate(super::CapabilityActivation::tool("RuntimeRead"));
    let profile = Arc::new(BTreeSet::from([String::from("RuntimeRead")]));
    let state = Arc::new(store.snapshot());
    let start_barrier = Arc::new(Barrier::new(3));
    let active = Arc::new(AtomicUsize::new(0));
    let max_active = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for value in ["alpha", "beta"] {
        let runtime = runtime.clone();
        let store = store.clone();
        let profile = Arc::clone(&profile);
        let state = Arc::clone(&state);
        let start_barrier = Arc::clone(&start_barrier);
        let active = Arc::clone(&active);
        let max_active = Arc::clone(&max_active);
        handles.push(thread::spawn(move || {
            start_barrier.wait();
            runtime
                .execute_tool(
                    "RuntimeRead",
                    json!({ "value": value }),
                    super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
                    &store,
                    None,
                    None,
                    move |_dispatch_kind, _tool_name, _input| {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(current, Ordering::SeqCst);
                        thread::sleep(Duration::from_millis(120));
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok(value.to_string())
                    },
                )
                .expect("parallel runtime call should succeed")
        }));
    }

    let started = Instant::now();
    start_barrier.wait();
    let outputs = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread should finish"))
        .collect::<Vec<_>>();

    assert_eq!(outputs.len(), 2);
    assert!(
        max_active.load(Ordering::SeqCst) >= 2,
        "read-only dispatches should be allowed to overlap"
    );
    // The authoritative signal here is overlap inside the dispatch closure.
    // Surface compilation refreshes capability inputs, including skill scans,
    // on each call, so an absolute wall-clock threshold would be dominated by
    // environment-dependent setup cost rather than the dispatch concurrency gate.
    let _elapsed = started.elapsed();
    fs::remove_dir_all(home).expect("temp home should clean up");
}

#[test]
fn capability_runtime_execute_tool_mediation_hook_blocks_and_traces_dispatch() {
    let runtime = capability_runtime_with_runtime_tools(vec![super::RuntimeToolDefinition {
        name: "RuntimeApproval".to_string(),
        description: Some("Approval-gated runtime write.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            },
            "required": ["value"],
            "additionalProperties": false
        }),
        required_permission: runtime::PermissionMode::WorkspaceWrite,
    }]);
    let store = super::SessionCapabilityStore::default();
    store.activate(super::CapabilityActivation::tool("RuntimeApproval"));
    let profile = BTreeSet::from([String::from("RuntimeApproval")]);
    let state = store.snapshot();
    let events = Arc::new(Mutex::new(Vec::new()));
    let captured_events = Arc::clone(&events);

    runtime.set_execution_hook(move |event| {
        captured_events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(event);
    });
    runtime.set_mediation_hook(|request| {
        assert_eq!(request.tool_name, "RuntimeApproval");
        assert_eq!(
            request.required_permission,
            runtime::PermissionMode::WorkspaceWrite
        );
        super::CapabilityMediationDecision::RequireApproval(Some(
            "approval required for runtime write".to_string(),
        ))
    });

    let error = runtime
        .execute_tool(
            "RuntimeApproval",
            json!({ "value": "blocked" }),
            super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
            &store,
            None,
            None,
            |_dispatch_kind, _tool_name, _input| {
                panic!("approval-gated dispatch should not execute")
            },
        )
        .expect_err("mediation hook should block runtime dispatch");

    assert!(error.to_string().contains("requires approval"));
    assert!(store.snapshot().is_tool_pending("RuntimeApproval"));
    let events = events
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(events.iter().any(|event| {
        event.phase == super::CapabilityExecutionPhase::BlockedApproval
            && event.tool_name == "RuntimeApproval"
    }));
}

#[test]
fn capability_runtime_execute_tool_with_outcome_preserves_structured_mediation_state() {
    let runtime = capability_runtime_with_runtime_tools(vec![super::RuntimeToolDefinition {
        name: "RuntimeAuth".to_string(),
        description: Some("Auth-gated runtime write.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "value": { "type": "string" }
            },
            "required": ["value"],
            "additionalProperties": false
        }),
        required_permission: runtime::PermissionMode::WorkspaceWrite,
    }]);
    let store = super::SessionCapabilityStore::default();
    store.activate(super::CapabilityActivation::tool("RuntimeAuth"));
    let profile = BTreeSet::from([String::from("RuntimeAuth")]);
    let state = store.snapshot();

    runtime.set_mediation_hook(|request| {
        assert_eq!(request.tool_name, "RuntimeAuth");
        super::CapabilityMediationDecision::RequireAuth(Some(
            "sign in before calling runtime auth tool".to_string(),
        ))
    });

    let outcome = runtime.execute_tool_with_outcome(
        "RuntimeAuth",
        json!({ "value": "blocked" }),
        super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
        &store,
        None,
        None,
        |_dispatch_kind, _tool_name, _input| panic!("auth-gated dispatch should not execute"),
    );

    assert_eq!(
        outcome,
        runtime::ToolExecutionOutcome::RequireAuth {
            reason: Some("sign in before calling runtime auth tool".to_string())
        }
    );
    assert!(store.snapshot().is_tool_pending("RuntimeAuth"));
}

#[test]
fn capability_runtime_execute_tool_executes_builtin_without_external_dispatch() {
    let runtime = CapabilityRuntime::builtin();
    let store = super::SessionCapabilityStore::default();

    let output = runtime
        .execute_tool(
            "StructuredOutput",
            json!({ "ok": true, "items": [1, 2, 3] }),
            super::CapabilityPlannerInput::default(),
            &store,
            None,
            None,
            |_kind, _tool_name, _input| {
                panic!("builtin tools should execute inside capability runtime")
            },
        )
        .expect("builtin tool should execute through runtime facade");
    let output_json: serde_json::Value =
        serde_json::from_str(&output).expect("builtin output should be json");

    assert_eq!(
        output_json["data"],
        "Structured output provided successfully"
    );
    assert_eq!(output_json["structured_output"]["ok"], true);
    assert_eq!(output_json["structured_output"]["items"], json!([1, 2, 3]));
}

#[test]
fn capability_runtime_execute_tool_executes_plugin_without_external_dispatch() {
    let runtime = capability_runtime_with_plugin_tools(vec![plugins::PluginTool::new(
        "plugin-demo@external",
        "plugin-demo",
        plugins::PluginToolDefinition {
            name: "plugin_echo".to_string(),
            description: Some("Echo plugin payload".to_string()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"],
                "additionalProperties": false
            }),
        },
        "sh".to_string(),
        vec!["-c".to_string(), "cat".to_string()],
        plugins::PluginToolPermission::WorkspaceWrite,
        None,
    )]);
    let store = super::SessionCapabilityStore::default();
    store.activate(super::CapabilityActivation::tool("plugin_echo"));
    let profile = BTreeSet::from([String::from("plugin_echo")]);
    let state = store.snapshot();

    let output = runtime
        .execute_tool(
            "plugin_echo",
            json!({ "message": "runtime-owned plugin dispatch" }),
            super::CapabilityPlannerInput::new(Some(&profile), Some(&state)),
            &store,
            None,
            None,
            |_kind, _tool_name, _input| {
                panic!("plugin tools should execute inside capability runtime")
            },
        )
        .expect("plugin tool should execute through runtime facade");
    let output_json: serde_json::Value =
        serde_json::from_str(&output).expect("plugin output should be json");

    assert_eq!(output_json["message"], "runtime-owned plugin dispatch");
}

#[test]
fn session_capability_store_persists_and_restores_shared_runtime_state() {
    let store = super::SessionCapabilityStore::default();
    store.activate(super::CapabilityActivation::tool("WebSearch"));
    store.apply_skill_execution_result(&super::SkillExecutionResult {
        skill: "help".to_string(),
        path: "/tmp/help/SKILL.md".to_string(),
        description: Some("workspace help".to_string()),
        context: super::skill_runtime::SkillContextKind::Inline,
        messages_to_inject: vec![super::skill_runtime::SkillInjectedMessage::system(
            "Injected guidance".to_string(),
        )],
        tool_grants: vec!["WebSearch".to_string()],
        model_override: Some("claude-sonnet-4-6".to_string()),
        effort_override: Some("medium".to_string()),
        state_updates: vec![
            super::SkillStateUpdate::ContextPrepared {
                context: super::skill_runtime::SkillContextKind::Inline,
            },
            super::SkillStateUpdate::MessageInjected {
                role: "system".to_string(),
            },
            super::SkillStateUpdate::ToolGranted {
                tool: "WebSearch".to_string(),
            },
            super::SkillStateUpdate::ModelOverride {
                model: "claude-sonnet-4-6".to_string(),
            },
            super::SkillStateUpdate::EffortOverride {
                effort: "medium".to_string(),
            },
            super::SkillStateUpdate::ForkSpawned {
                agent_id: "agent-123".to_string(),
                subagent_type: Some("Plan".to_string()),
                output_file: "/tmp/agent-123/output.json".to_string(),
                manifest_file: "/tmp/agent-123/manifest.json".to_string(),
            },
        ],
    });

    let mut session = runtime::Session::new();
    store
        .persist_into_session(&mut session)
        .expect("store state should persist");

    let restored = super::SessionCapabilityStore::restore_from_session(&session)
        .expect("store state should restore");
    let snapshot = restored.snapshot();
    assert!(snapshot.is_tool_activated("WebSearch"));
    assert!(snapshot.is_tool_discovered("WebSearch"));
    assert!(snapshot.is_tool_exposed("WebSearch"));
    assert!(snapshot.is_tool_granted("WebSearch"));
    assert_eq!(snapshot.injected_skill_messages(), &["Injected guidance"]);
    assert_eq!(snapshot.model_override(), Some("claude-sonnet-4-6"));
    assert_eq!(snapshot.effort_override(), Some("medium"));
    assert!(snapshot
        .skill_state_updates()
        .contains(&super::SkillStateUpdate::ForkSpawned {
            agent_id: "agent-123".to_string(),
            subagent_type: Some("Plan".to_string()),
            output_file: "/tmp/agent-123/output.json".to_string(),
            manifest_file: "/tmp/agent-123/manifest.json".to_string(),
        }));
}

#[test]
fn session_capability_store_restore_marks_fork_lifecycle_from_agent_manifest() {
    let output_dir = temp_path("session-skill-fork-restore");
    fs::create_dir_all(&output_dir).expect("output dir should exist");
    let output_file = output_dir.join("agent-restore.md");
    let manifest_file = output_dir.join("agent-restore.json");
    fs::write(&output_file, "# Agent Task\n").expect("output file should exist");

    let manifest = super::AgentOutput {
        agent_id: "agent-restore".to_string(),
        name: "planner".to_string(),
        description: "Execute planning fork skill".to_string(),
        subagent_type: Some("Plan".to_string()),
        model: Some("claude-sonnet-4-5".to_string()),
        status: "completed".to_string(),
        output_file: output_file.display().to_string(),
        manifest_file: manifest_file.display().to_string(),
        created_at: "2026-04-12T00:00:00Z".to_string(),
        started_at: Some("2026-04-12T00:00:01Z".to_string()),
        completed_at: Some("2026-04-12T00:00:05Z".to_string()),
        lane_events: Vec::new(),
        current_blocker: None,
        derived_state: "finished_cleanable".to_string(),
        error: None,
    };
    fs::write(
        &manifest_file,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should exist");

    let store = super::SessionCapabilityStore::default();
    store.apply_skill_execution_result(&super::SkillExecutionResult {
        skill: "planner".to_string(),
        path: "/tmp/planner/SKILL.md".to_string(),
        description: Some("Fork planning guidance into a dedicated subagent.".to_string()),
        context: super::skill_runtime::SkillContextKind::Fork,
        messages_to_inject: Vec::new(),
        tool_grants: vec!["WebSearch".to_string()],
        model_override: Some("claude-sonnet-4-5".to_string()),
        effort_override: Some("high".to_string()),
        state_updates: vec![
            super::SkillStateUpdate::ContextPrepared {
                context: super::skill_runtime::SkillContextKind::Fork,
            },
            super::SkillStateUpdate::ForkSpawned {
                agent_id: "agent-restore".to_string(),
                subagent_type: Some("Plan".to_string()),
                output_file: output_file.display().to_string(),
                manifest_file: manifest_file.display().to_string(),
            },
        ],
    });

    let mut session = Session::new();
    store
        .persist_into_session(&mut session)
        .expect("store state should persist");

    let restored = super::SessionCapabilityStore::restore_from_session(&session)
        .expect("store state should restore");
    let snapshot = restored.snapshot();

    assert!(snapshot
        .skill_state_updates()
        .contains(&super::SkillStateUpdate::ForkRestored {
            agent_id: "agent-restore".to_string(),
            status: "completed".to_string(),
            derived_state: "finished_cleanable".to_string(),
            output_file: output_file.display().to_string(),
            manifest_file: manifest_file.display().to_string(),
        }));
    assert!(snapshot
        .skill_state_updates()
        .contains(&super::SkillStateUpdate::ForkCompleted {
            agent_id: "agent-restore".to_string(),
            output_file: output_file.display().to_string(),
            manifest_file: manifest_file.display().to_string(),
            completed_at: Some("2026-04-12T00:00:05Z".to_string()),
        }));

    fs::remove_dir_all(output_dir).expect("temp output dir should clean up");
}

#[test]
fn session_capability_store_restore_is_idempotent_for_fork_manifest() {
    let output_dir = temp_path("session-skill-fork-restore-idempotent");
    fs::create_dir_all(&output_dir).expect("output dir should exist");
    let output_file = output_dir.join("agent-restore.md");
    let manifest_file = output_dir.join("agent-restore.json");
    fs::write(&output_file, "# Agent Task\n").expect("output file should exist");

    let manifest = super::AgentOutput {
        agent_id: "agent-restore".to_string(),
        name: "planner".to_string(),
        description: "Execute planning fork skill".to_string(),
        subagent_type: Some("Plan".to_string()),
        model: Some("claude-sonnet-4-5".to_string()),
        status: "completed".to_string(),
        output_file: output_file.display().to_string(),
        manifest_file: manifest_file.display().to_string(),
        created_at: "2026-04-12T00:00:00Z".to_string(),
        started_at: Some("2026-04-12T00:00:01Z".to_string()),
        completed_at: Some("2026-04-12T00:00:05Z".to_string()),
        lane_events: Vec::new(),
        current_blocker: None,
        derived_state: "finished_cleanable".to_string(),
        error: None,
    };
    fs::write(
        &manifest_file,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should exist");

    let store = super::SessionCapabilityStore::default();
    store.apply_skill_execution_result(&super::SkillExecutionResult {
        skill: "planner".to_string(),
        path: "/tmp/planner/SKILL.md".to_string(),
        description: Some("Fork planning guidance into a dedicated subagent.".to_string()),
        context: super::skill_runtime::SkillContextKind::Fork,
        messages_to_inject: Vec::new(),
        tool_grants: vec!["WebSearch".to_string()],
        model_override: Some("claude-sonnet-4-5".to_string()),
        effort_override: Some("high".to_string()),
        state_updates: vec![
            super::SkillStateUpdate::ContextPrepared {
                context: super::skill_runtime::SkillContextKind::Fork,
            },
            super::SkillStateUpdate::ForkSpawned {
                agent_id: "agent-restore".to_string(),
                subagent_type: Some("Plan".to_string()),
                output_file: output_file.display().to_string(),
                manifest_file: manifest_file.display().to_string(),
            },
        ],
    });

    let mut first_session = Session::new();
    store
        .persist_into_session(&mut first_session)
        .expect("store state should persist");

    let first_restore = super::SessionCapabilityStore::restore_from_session(&first_session)
        .expect("first restore should succeed");
    let first_snapshot = first_restore.snapshot();
    assert_eq!(
        first_snapshot
            .skill_state_updates()
            .iter()
            .filter(|update| matches!(
                update,
                super::SkillStateUpdate::ForkRestored { agent_id, .. } if agent_id == "agent-restore"
            ))
            .count(),
        1
    );
    assert_eq!(
        first_snapshot
            .skill_state_updates()
            .iter()
            .filter(|update| matches!(
                update,
                super::SkillStateUpdate::ForkCompleted { agent_id, .. } if agent_id == "agent-restore"
            ))
            .count(),
        1
    );

    let mut second_session = Session::new();
    first_restore
        .persist_into_session(&mut second_session)
        .expect("restored store state should persist");

    let second_restore = super::SessionCapabilityStore::restore_from_session(&second_session)
        .expect("second restore should succeed");
    let second_snapshot = second_restore.snapshot();
    assert_eq!(
        second_snapshot
            .skill_state_updates()
            .iter()
            .filter(|update| matches!(
                update,
                super::SkillStateUpdate::ForkRestored { agent_id, .. } if agent_id == "agent-restore"
            ))
            .count(),
        1
    );
    assert_eq!(
        second_snapshot
            .skill_state_updates()
            .iter()
            .filter(|update| matches!(
                update,
                super::SkillStateUpdate::ForkCompleted { agent_id, .. } if agent_id == "agent-restore"
            ))
            .count(),
        1
    );

    fs::remove_dir_all(output_dir).expect("temp output dir should clean up");
}

#[test]
fn session_capability_store_snapshot_marks_failed_fork_lifecycle_from_agent_manifest() {
    let output_dir = temp_path("session-skill-fork-failed");
    fs::create_dir_all(&output_dir).expect("output dir should exist");
    let output_file = output_dir.join("agent-failed.md");
    let manifest_file = output_dir.join("agent-failed.json");
    fs::write(&output_file, "# Agent Task\n").expect("output file should exist");

    let manifest = super::AgentOutput {
        agent_id: "agent-failed".to_string(),
        name: "planner".to_string(),
        description: "Execute planning fork skill".to_string(),
        subagent_type: Some("Plan".to_string()),
        model: Some("claude-sonnet-4-5".to_string()),
        status: "failed".to_string(),
        output_file: output_file.display().to_string(),
        manifest_file: manifest_file.display().to_string(),
        created_at: "2026-04-12T00:00:00Z".to_string(),
        started_at: Some("2026-04-12T00:00:01Z".to_string()),
        completed_at: Some("2026-04-12T00:00:05Z".to_string()),
        lane_events: Vec::new(),
        current_blocker: None,
        derived_state: "truly_idle".to_string(),
        error: Some("sub-agent thread panicked".to_string()),
    };
    fs::write(
        &manifest_file,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should exist");

    let store = super::SessionCapabilityStore::default();
    store.apply_skill_execution_result(&super::SkillExecutionResult {
        skill: "planner".to_string(),
        path: "/tmp/planner/SKILL.md".to_string(),
        description: Some("Fork planning guidance into a dedicated subagent.".to_string()),
        context: super::skill_runtime::SkillContextKind::Fork,
        messages_to_inject: Vec::new(),
        tool_grants: Vec::new(),
        model_override: None,
        effort_override: None,
        state_updates: vec![
            super::SkillStateUpdate::ContextPrepared {
                context: super::skill_runtime::SkillContextKind::Fork,
            },
            super::SkillStateUpdate::ForkSpawned {
                agent_id: "agent-failed".to_string(),
                subagent_type: Some("Plan".to_string()),
                output_file: output_file.display().to_string(),
                manifest_file: manifest_file.display().to_string(),
            },
        ],
    });

    let snapshot = store.snapshot();
    assert!(snapshot
        .skill_state_updates()
        .contains(&super::SkillStateUpdate::ForkFailed {
            agent_id: "agent-failed".to_string(),
            output_file: output_file.display().to_string(),
            manifest_file: manifest_file.display().to_string(),
            error: Some("sub-agent thread panicked".to_string()),
        }));
    assert!(
        !snapshot.skill_state_updates().iter().any(|update| matches!(
            update,
            super::SkillStateUpdate::ForkRestored { agent_id, .. } if agent_id == "agent-failed"
        ))
    );

    fs::remove_dir_all(output_dir).expect("temp output dir should clean up");
}

#[test]
fn session_capability_store_snapshot_does_not_duplicate_terminal_fork_updates() {
    let output_dir = temp_path("session-skill-fork-terminal-conflict");
    fs::create_dir_all(&output_dir).expect("output dir should exist");
    let output_file = output_dir.join("agent-conflict.md");
    let manifest_file = output_dir.join("agent-conflict.json");
    fs::write(&output_file, "# Agent Task\n").expect("output file should exist");

    let manifest = super::AgentOutput {
        agent_id: "agent-conflict".to_string(),
        name: "planner".to_string(),
        description: "Execute planning fork skill".to_string(),
        subagent_type: Some("Plan".to_string()),
        model: Some("claude-sonnet-4-5".to_string()),
        status: "failed".to_string(),
        output_file: output_file.display().to_string(),
        manifest_file: manifest_file.display().to_string(),
        created_at: "2026-04-12T00:00:00Z".to_string(),
        started_at: Some("2026-04-12T00:00:01Z".to_string()),
        completed_at: Some("2026-04-12T00:00:05Z".to_string()),
        lane_events: Vec::new(),
        current_blocker: None,
        derived_state: "truly_idle".to_string(),
        error: Some("sub-agent thread panicked".to_string()),
    };
    fs::write(
        &manifest_file,
        serde_json::to_string_pretty(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest should exist");

    let store = super::SessionCapabilityStore::default();
    store.apply_skill_state_updates(&[
        super::SkillStateUpdate::ForkSpawned {
            agent_id: "agent-conflict".to_string(),
            subagent_type: Some("Plan".to_string()),
            output_file: output_file.display().to_string(),
            manifest_file: manifest_file.display().to_string(),
        },
        super::SkillStateUpdate::ForkCompleted {
            agent_id: "agent-conflict".to_string(),
            output_file: output_file.display().to_string(),
            manifest_file: manifest_file.display().to_string(),
            completed_at: Some("2026-04-12T00:00:05Z".to_string()),
        },
    ]);

    let snapshot = store.snapshot();
    assert_eq!(
        snapshot
            .skill_state_updates()
            .iter()
            .filter(|update| matches!(
                update,
                super::SkillStateUpdate::ForkCompleted { agent_id, .. } if agent_id == "agent-conflict"
            ))
            .count(),
        1
    );
    assert_eq!(
        snapshot
            .skill_state_updates()
            .iter()
            .filter(|update| matches!(
                update,
                super::SkillStateUpdate::ForkFailed { agent_id, .. } if agent_id == "agent-conflict"
            ))
            .count(),
        0
    );

    fs::remove_dir_all(output_dir).expect("temp output dir should clean up");
}

#[test]
fn skill_tool_grants_deferred_tools_into_visible_surface() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let home = temp_path("skill-tool-home");
    let skill_dir = home.join(".agents").join("skills").join("help");
    fs::create_dir_all(&skill_dir).expect("skill dir should exist");
    fs::write(
        skill_dir.join("SKILL.md"),
        r"---
name: help
description: Help the model decide when to use the workspace guidance skill.
when_to_use: Use when the task asks for workspace orientation.
allowed-tools:
  - WebSearch
model-invocable: true
user-invocable: true
context: inline
---
# help

Guide the model through the workspace.
",
    )
    .expect("skill file should exist");

    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", &home);

    let capability_provider = CapabilityProvider::builtin();
    let capability_runtime = CapabilityRuntime::new(capability_provider.clone());
    let profile = BTreeSet::from([String::from("ToolSearch"), String::from("WebSearch")]);
    let shared_state = std::sync::Arc::new(std::sync::Mutex::new(
        super::SessionCapabilityState::default(),
    ));

    let before = {
        let locked = shared_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        capability_runtime
            .surface_projection(super::CapabilityPlannerInput::new(
                Some(&profile),
                Some(&locked),
            ))
            .expect("planner surface should resolve before grant")
    };
    assert!(!before
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));

    let result = capability_runtime
        .execute_skill_detailed(
            "help",
            Some(json!({"topic":"workspace"})),
            super::CapabilityPlannerInput::default(),
        )
        .expect("prompt skill should succeed");
    {
        let mut locked = shared_state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        locked.apply_skill_execution_result(&result);
    }
    let result_json: serde_json::Value = serde_json::to_value(&result).expect("valid json");
    assert_eq!(result_json["skill"], "help");
    assert_eq!(result_json["context"], "inline");
    assert_eq!(result_json["tool_grants"][0], "WebSearch");
    assert_eq!(result_json["messages_to_inject"][0]["role"], "system");
    assert_eq!(result_json["state_updates"][0]["kind"], "context_prepared");
    assert_eq!(result_json["state_updates"][1]["kind"], "message_injected");
    assert_eq!(result_json["state_updates"][2]["kind"], "tool_granted");
    assert_eq!(result_json["state_updates"][2]["tool"], "WebSearch");

    let locked = shared_state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    assert!(locked.is_tool_granted("WebSearch"));
    let after = capability_runtime
        .surface_projection(super::CapabilityPlannerInput::new(
            Some(&profile),
            Some(&locked),
        ))
        .expect("planner surface should resolve after grant");
    assert!(after
        .visible_tools
        .iter()
        .any(|capability| capability.display_name == "WebSearch"));
    assert!(locked
        .injected_skill_messages()
        .iter()
        .any(|message| message.contains("Guide the model through the workspace")));

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    } else {
        std::env::remove_var("HOME");
    }
    fs::remove_dir_all(home).expect("temp home should clean up");
}

#[test]
fn mcp_prompt_capabilities_without_runtime_executors_stay_hidden_from_skill_discovery() {
    let capability = super::CapabilitySpec {
        capability_id: "mcp-prompt.workspace-guide".to_string(),
        source_kind: super::CapabilitySourceKind::McpPrompt,
        execution_kind: super::CapabilityExecutionKind::PromptSkill,
        provider_key: Some("mcp".to_string()),
        executor_key: Some("mcp-prompt.workspace-guide".to_string()),
        display_name: "workspace-guide".to_string(),
        description: "MCP prompt-backed workspace guidance skill.".to_string(),
        when_to_use: Some("Use when the task needs MCP-provided workspace guidance.".to_string()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "skill": { "type": "string" },
                "arguments": {}
            },
            "required": ["skill"],
            "additionalProperties": false
        }),
        search_hint: Some("workspace guidance".to_string()),
        visibility: super::CapabilityVisibility::DefaultVisible,
        state: super::CapabilityState::Ready,
        permission_profile: crate::capability_runtime::CapabilityPermissionProfile {
            required_permission: PermissionMode::ReadOnly,
        },
        trust_profile: crate::capability_runtime::CapabilityTrustProfile::default(),
        scope_constraints: crate::capability_runtime::CapabilityScopeConstraints::default(),
        invocation_policy: crate::capability_runtime::CapabilityInvocationPolicy {
            selectable: true,
            requires_approval: false,
            requires_auth: false,
        },
        concurrency_policy: super::CapabilityConcurrencyPolicy::Serialized,
    };
    let runtime = capability_runtime_with_provided_capabilities(vec![capability]);

    let surface = runtime
        .surface_projection(super::CapabilityPlannerInput::default())
        .expect("planner should project a capability surface");
    assert!(!surface
        .discoverable_skills
        .iter()
        .any(|skill| skill.display_name == "workspace-guide"));
    assert!(surface
        .hidden_capabilities
        .iter()
        .any(|skill| skill.display_name == "workspace-guide"));

    let discovery = runtime.skill_discovery(
        "workspace guidance",
        10,
        super::CapabilityPlannerInput::default(),
    );
    let output: serde_json::Value =
        serde_json::to_value(discovery).expect("skill discovery output should be json");
    assert!(!output["matches"]
        .as_array()
        .expect("matches")
        .iter()
        .any(|value| value == "workspace-guide"));
}

#[test]
fn agent_persists_handoff_metadata() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = temp_path("agent-store");
    std::env::set_var("CLAWD_AGENT_STORE", &dir);
    let captured = Arc::new(Mutex::new(None::<AgentJob>));
    let captured_for_spawn = Arc::clone(&captured);

    let manifest = spawn_subagent_with_job(
        AgentInput {
            description: "Audit the branch".to_string(),
            prompt: "Check tests and outstanding work.".to_string(),
            subagent_type: Some("Explore".to_string()),
            name: Some("ship-audit".to_string()),
            model: None,
        },
        move |job| {
            *captured_for_spawn
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(job);
            Ok(())
        },
    )
    .expect("Agent should succeed");
    std::env::remove_var("CLAWD_AGENT_STORE");

    assert_eq!(manifest.name, "ship-audit");
    assert_eq!(manifest.subagent_type.as_deref(), Some("Explore"));
    assert_eq!(manifest.status, "running");
    assert!(!manifest.created_at.is_empty());
    assert!(manifest.started_at.is_some());
    assert!(manifest.completed_at.is_none());
    let contents = std::fs::read_to_string(&manifest.output_file).expect("agent file exists");
    let manifest_contents =
        std::fs::read_to_string(&manifest.manifest_file).expect("manifest file exists");
    let manifest_json: serde_json::Value =
        serde_json::from_str(&manifest_contents).expect("manifest should be valid json");
    assert!(contents.contains("Audit the branch"));
    assert!(contents.contains("Check tests and outstanding work."));
    assert!(manifest_contents.contains("\"subagentType\": \"Explore\""));
    assert!(manifest_contents.contains("\"status\": \"running\""));
    assert_eq!(manifest_json["laneEvents"][0]["event"], "lane.started");
    assert_eq!(manifest_json["laneEvents"][0]["status"], "running");
    assert!(manifest_json["currentBlocker"].is_null());
    let captured_job = captured
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("spawn job should be captured");
    assert_eq!(captured_job.prompt, "Check tests and outstanding work.");
    assert!(captured_job
        .capability_profile
        .allowed_tools()
        .contains("read_file"));
    assert!(!captured_job
        .capability_profile
        .allowed_tools()
        .contains("Agent"));

    let normalized = spawn_subagent_with_job(
        AgentInput {
            description: "Verify the branch".to_string(),
            prompt: "Check tests.".to_string(),
            subagent_type: Some("explorer".to_string()),
            name: None,
            model: None,
        },
        |_| Ok(()),
    )
    .expect("subagent helper should normalize built-in aliases");
    assert_eq!(normalized.subagent_type.as_deref(), Some("Explore"));

    let named = spawn_subagent_with_job(
        AgentInput {
            description: "Review the branch".to_string(),
            prompt: "Inspect diff.".to_string(),
            subagent_type: None,
            name: Some("Ship Audit!!!".to_string()),
            model: None,
        },
        |_| Ok(()),
    )
    .expect("subagent helper should normalize explicit names");
    assert_eq!(named.name, "ship-audit");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[allow(clippy::too_many_lines)]
fn agent_fake_runner_can_persist_completion_and_failure() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = temp_path("agent-runner");
    std::env::set_var("CLAWD_AGENT_STORE", &dir);

    let completed = spawn_subagent_with_job(
        AgentInput {
            description: "Complete the task".to_string(),
            prompt: "Do the work".to_string(),
            subagent_type: Some("Explore".to_string()),
            name: Some("complete-task".to_string()),
            model: Some("claude-sonnet-4-6".to_string()),
        },
        |job| {
            persist_agent_terminal_state(
                &job.manifest,
                "completed",
                Some("Finished successfully in commit abc1234"),
                None,
            )
        },
    )
    .expect("completed agent should succeed");

    let completed_manifest =
        std::fs::read_to_string(&completed.manifest_file).expect("completed manifest should exist");
    let completed_manifest_json: serde_json::Value =
        serde_json::from_str(&completed_manifest).expect("completed manifest json");
    let completed_output =
        std::fs::read_to_string(&completed.output_file).expect("completed output should exist");
    assert!(completed_manifest.contains("\"status\": \"completed\""));
    assert!(completed_output.contains("Finished successfully"));
    assert_eq!(
        completed_manifest_json["laneEvents"][0]["event"],
        "lane.started"
    );
    assert_eq!(
        completed_manifest_json["laneEvents"][1]["event"],
        "lane.finished"
    );
    assert_eq!(
        completed_manifest_json["laneEvents"][2]["event"],
        "lane.commit.created"
    );
    assert_eq!(
        completed_manifest_json["laneEvents"][2]["data"]["commit"],
        "abc1234"
    );
    assert!(completed_manifest_json["currentBlocker"].is_null());
    assert_eq!(
        completed_manifest_json["derivedState"],
        "finished_cleanable"
    );

    let failed = spawn_subagent_with_job(
        AgentInput {
            description: "Fail the task".to_string(),
            prompt: "Do the failing work".to_string(),
            subagent_type: Some("Verification".to_string()),
            name: Some("fail-task".to_string()),
            model: None,
        },
        |job| {
            persist_agent_terminal_state(
                &job.manifest,
                "failed",
                None,
                Some(String::from("tool failed: simulated failure")),
            )
        },
    )
    .expect("failed agent should still spawn");

    let failed_manifest =
        std::fs::read_to_string(&failed.manifest_file).expect("failed manifest should exist");
    let failed_manifest_json: serde_json::Value =
        serde_json::from_str(&failed_manifest).expect("failed manifest json");
    let failed_output =
        std::fs::read_to_string(&failed.output_file).expect("failed output should exist");
    assert!(failed_manifest.contains("\"status\": \"failed\""));
    assert!(failed_manifest.contains("simulated failure"));
    assert!(failed_output.contains("simulated failure"));
    assert!(failed_output.contains("failure_class: tool_runtime"));
    assert_eq!(
        failed_manifest_json["currentBlocker"]["failureClass"],
        "tool_runtime"
    );
    assert_eq!(
        failed_manifest_json["laneEvents"][1]["event"],
        "lane.blocked"
    );
    assert_eq!(
        failed_manifest_json["laneEvents"][2]["event"],
        "lane.failed"
    );
    assert_eq!(
        failed_manifest_json["laneEvents"][2]["failureClass"],
        "tool_runtime"
    );
    assert_eq!(failed_manifest_json["derivedState"], "truly_idle");

    let spawn_error = spawn_subagent_with_job(
        AgentInput {
            description: "Spawn error task".to_string(),
            prompt: "Never starts".to_string(),
            subagent_type: None,
            name: Some("spawn-error".to_string()),
            model: None,
        },
        |_| Err(String::from("thread creation failed")),
    )
    .expect_err("spawn errors should surface");
    assert!(spawn_error.contains("failed to spawn sub-agent"));
    let spawn_error_manifest = std::fs::read_dir(&dir)
        .expect("agent dir should exist")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .find_map(|path| {
            let contents = std::fs::read_to_string(&path).ok()?;
            contents
                .contains("\"name\": \"spawn-error\"")
                .then_some(contents)
        })
        .expect("failed manifest should still be written");
    let spawn_error_manifest_json: serde_json::Value =
        serde_json::from_str(&spawn_error_manifest).expect("spawn error manifest json");
    assert!(spawn_error_manifest.contains("\"status\": \"failed\""));
    assert!(spawn_error_manifest.contains("thread creation failed"));
    assert_eq!(
        spawn_error_manifest_json["currentBlocker"]["failureClass"],
        "infra"
    );
    assert_eq!(spawn_error_manifest_json["derivedState"], "truly_idle");

    std::env::remove_var("CLAWD_AGENT_STORE");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn agent_state_classification_covers_finished_and_specific_blockers() {
    assert_eq!(derive_agent_state("running", None, None, None), "working");
    assert_eq!(
        derive_agent_state("completed", Some("done"), None, None),
        "finished_cleanable"
    );
    assert_eq!(
        derive_agent_state("completed", None, None, None),
        "finished_pending_report"
    );
    assert_eq!(
        derive_agent_state("failed", None, Some("mcp handshake timed out"), None),
        "degraded_mcp"
    );
    assert_eq!(
        derive_agent_state(
            "failed",
            None,
            Some("background terminal still running"),
            None
        ),
        "blocked_background_job"
    );
    assert_eq!(
        derive_agent_state("failed", None, Some("merge conflict while rebasing"), None),
        "blocked_merge_conflict"
    );
    assert_eq!(
        derive_agent_state(
            "failed",
            None,
            Some("transport interrupted after partial progress"),
            None
        ),
        "interrupted_transport"
    );
}

#[test]
fn commit_provenance_is_extracted_from_agent_results() {
    let provenance = maybe_commit_provenance(Some("landed as commit deadbee with clean push"))
        .expect("commit provenance");
    assert_eq!(provenance.commit, "deadbee");
    assert_eq!(provenance.canonical_commit.as_deref(), Some("deadbee"));
    assert_eq!(provenance.lineage, vec!["deadbee".to_string()]);
}
#[test]
fn lane_failure_taxonomy_normalizes_common_blockers() {
    let cases = [
        (
            "prompt delivery failed in tmux pane",
            LaneFailureClass::PromptDelivery,
        ),
        (
            "trust prompt is still blocking startup",
            LaneFailureClass::TrustGate,
        ),
        (
            "branch stale against main after divergence",
            LaneFailureClass::BranchDivergence,
        ),
        (
            "compile failed after cargo check",
            LaneFailureClass::Compile,
        ),
        ("targeted tests failed", LaneFailureClass::Test),
        ("plugin bootstrap failed", LaneFailureClass::PluginStartup),
        ("mcp handshake timed out", LaneFailureClass::McpHandshake),
        (
            "mcp startup failed before listing tools",
            LaneFailureClass::McpStartup,
        ),
        (
            "gateway routing rejected the request",
            LaneFailureClass::GatewayRouting,
        ),
        (
            "tool failed: denied tool execution from hook",
            LaneFailureClass::ToolRuntime,
        ),
        ("thread creation failed", LaneFailureClass::Infra),
    ];

    for (message, expected) in cases {
        assert_eq!(classify_lane_failure(message), expected, "{message}");
    }
}

#[test]
fn lane_event_schema_serializes_to_canonical_names() {
    let cases = [
        (LaneEventName::Started, "lane.started"),
        (LaneEventName::Ready, "lane.ready"),
        (LaneEventName::PromptMisdelivery, "lane.prompt_misdelivery"),
        (LaneEventName::Blocked, "lane.blocked"),
        (LaneEventName::Red, "lane.red"),
        (LaneEventName::Green, "lane.green"),
        (LaneEventName::CommitCreated, "lane.commit.created"),
        (LaneEventName::PrOpened, "lane.pr.opened"),
        (LaneEventName::MergeReady, "lane.merge.ready"),
        (LaneEventName::Finished, "lane.finished"),
        (LaneEventName::Failed, "lane.failed"),
        (
            LaneEventName::BranchStaleAgainstMain,
            "branch.stale_against_main",
        ),
    ];

    for (event, expected) in cases {
        assert_eq!(
            serde_json::to_value(event).expect("serialize lane event"),
            json!(expected)
        );
    }
}

#[test]
fn agent_tool_subset_mapping_is_expected() {
    let general = allowed_tools_for_subagent("general-purpose");
    assert!(general.contains("bash"));
    assert!(general.contains("write_file"));
    assert!(!general.contains("Skill"));
    assert!(!general.contains("Agent"));

    let explore = allowed_tools_for_subagent("Explore");
    assert!(explore.contains("read_file"));
    assert!(explore.contains("grep_search"));
    assert!(!explore.contains("Skill"));
    assert!(!explore.contains("bash"));

    let plan = allowed_tools_for_subagent("Plan");
    assert!(plan.contains("TodoWrite"));
    assert!(plan.contains("StructuredOutput"));
    assert!(!plan.contains("Skill"));
    assert!(!plan.contains("Agent"));

    let verification = allowed_tools_for_subagent("Verification");
    assert!(verification.contains("bash"));
    assert!(verification.contains("PowerShell"));
    assert!(!verification.contains("Skill"));
    assert!(!verification.contains("write_file"));
}

#[derive(Debug)]
struct MockSubagentApiClient {
    calls: usize,
    input_path: String,
}

impl runtime::ApiClient for MockSubagentApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        self.calls += 1;
        match self.calls {
            1 => {
                assert_eq!(request.messages.len(), 1);
                Ok(vec![
                    AssistantEvent::ToolUse {
                        id: "tool-1".to_string(),
                        name: "read_file".to_string(),
                        input: json!({ "path": self.input_path }).to_string(),
                    },
                    AssistantEvent::MessageStop,
                ])
            }
            2 => {
                assert!(request.messages.len() >= 3);
                Ok(vec![
                    AssistantEvent::TextDelta("Scope: completed mock review".to_string()),
                    AssistantEvent::MessageStop,
                ])
            }
            _ => unreachable!("extra mock stream call"),
        }
    }
}

#[test]
fn subagent_runtime_executes_tool_loop_with_isolated_session() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let path = temp_path("subagent-input.txt");
    std::fs::write(&path, "hello from child").expect("write input file");

    let mut runtime = ConversationRuntime::new(
        Session::new(),
        MockSubagentApiClient {
            calls: 0,
            input_path: path.display().to_string(),
        },
        SubagentToolExecutor::new(BTreeSet::from([String::from("read_file")])),
        agent_permission_policy(),
        vec![String::from("system prompt")],
    );

    let summary = runtime
        .run_turn("Inspect the delegated file", None)
        .expect("subagent loop should succeed");

    assert_eq!(
        final_assistant_text(&summary),
        "Scope: completed mock review"
    );
    assert!(runtime
        .session()
        .messages
        .iter()
        .flat_map(|message| message.blocks.iter())
        .any(|block| matches!(
            block,
            runtime::ContentBlock::ToolResult { output, .. }
                if output.contains("hello from child")
        )));

    let _ = std::fs::remove_file(path);
}

#[test]
fn agent_rejects_blank_required_fields() {
    let missing_description = spawn_subagent_with_job(
        AgentInput {
            description: "  ".to_string(),
            prompt: "Inspect".to_string(),
            subagent_type: None,
            name: None,
            model: None,
        },
        |_| Ok(()),
    )
    .expect_err("blank description should fail");
    assert!(missing_description.contains("description must not be empty"));

    let missing_prompt = spawn_subagent_with_job(
        AgentInput {
            description: "Inspect branch".to_string(),
            prompt: " ".to_string(),
            subagent_type: None,
            name: None,
            model: None,
        },
        |_| Ok(()),
    )
    .expect_err("blank prompt should fail");
    assert!(missing_prompt.contains("prompt must not be empty"));
}

#[test]
fn notebook_edit_replaces_inserts_and_deletes_cells() {
    let path = temp_path("notebook.ipynb");
    std::fs::write(
            &path,
            r#"{
  "cells": [
    {"cell_type": "code", "id": "cell-a", "metadata": {}, "source": ["print(1)\n"], "outputs": [], "execution_count": null}
  ],
  "metadata": {"kernelspec": {"language": "python"}},
  "nbformat": 4,
  "nbformat_minor": 5
}"#,
        )
        .expect("write notebook");

    let replaced = execute_tool(
        "NotebookEdit",
        &json!({
            "notebook_path": path.display().to_string(),
            "cell_id": "cell-a",
            "new_source": "print(2)\n",
            "edit_mode": "replace"
        }),
    )
    .expect("NotebookEdit replace should succeed");
    let replaced_output: serde_json::Value = serde_json::from_str(&replaced).expect("json");
    assert_eq!(replaced_output["cell_id"], "cell-a");
    assert_eq!(replaced_output["cell_type"], "code");

    let inserted = execute_tool(
        "NotebookEdit",
        &json!({
            "notebook_path": path.display().to_string(),
            "cell_id": "cell-a",
            "new_source": "# heading\n",
            "cell_type": "markdown",
            "edit_mode": "insert"
        }),
    )
    .expect("NotebookEdit insert should succeed");
    let inserted_output: serde_json::Value = serde_json::from_str(&inserted).expect("json");
    assert_eq!(inserted_output["cell_type"], "markdown");
    let appended = execute_tool(
        "NotebookEdit",
        &json!({
            "notebook_path": path.display().to_string(),
            "new_source": "print(3)\n",
            "edit_mode": "insert"
        }),
    )
    .expect("NotebookEdit append should succeed");
    let appended_output: serde_json::Value = serde_json::from_str(&appended).expect("json");
    assert_eq!(appended_output["cell_type"], "code");

    let deleted = execute_tool(
        "NotebookEdit",
        &json!({
            "notebook_path": path.display().to_string(),
            "cell_id": "cell-a",
            "edit_mode": "delete"
        }),
    )
    .expect("NotebookEdit delete should succeed without new_source");
    let deleted_output: serde_json::Value = serde_json::from_str(&deleted).expect("json");
    assert!(deleted_output["cell_type"].is_null());
    assert_eq!(deleted_output["new_source"], "");

    let final_notebook: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read notebook"))
            .expect("valid notebook json");
    let cells = final_notebook["cells"].as_array().expect("cells array");
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0]["cell_type"], "markdown");
    assert!(cells[0].get("outputs").is_none());
    assert_eq!(cells[1]["cell_type"], "code");
    assert_eq!(cells[1]["source"][0], "print(3)\n");
    let _ = std::fs::remove_file(path);
}

#[test]
fn notebook_edit_rejects_invalid_inputs() {
    let text_path = temp_path("notebook.txt");
    fs::write(&text_path, "not a notebook").expect("write text file");
    let wrong_extension = execute_tool(
        "NotebookEdit",
        &json!({
            "notebook_path": text_path.display().to_string(),
            "new_source": "print(1)\n"
        }),
    )
    .expect_err("non-ipynb file should fail");
    assert!(wrong_extension.contains("Jupyter notebook"));
    let _ = fs::remove_file(&text_path);

    let empty_notebook = temp_path("empty.ipynb");
    fs::write(
            &empty_notebook,
            r#"{"cells":[],"metadata":{"kernelspec":{"language":"python"}},"nbformat":4,"nbformat_minor":5}"#,
        )
        .expect("write empty notebook");

    let missing_source = execute_tool(
        "NotebookEdit",
        &json!({
            "notebook_path": empty_notebook.display().to_string(),
            "edit_mode": "insert"
        }),
    )
    .expect_err("insert without source should fail");
    assert!(missing_source.contains("new_source is required"));

    let missing_cell = execute_tool(
        "NotebookEdit",
        &json!({
            "notebook_path": empty_notebook.display().to_string(),
            "edit_mode": "delete"
        }),
    )
    .expect_err("delete on empty notebook should fail");
    assert!(missing_cell.contains("Notebook has no cells to edit"));
    let _ = fs::remove_file(empty_notebook);
}

#[test]
fn bash_tool_reports_success_exit_failure_timeout_and_background() {
    let success =
        execute_tool("bash", &json!({ "command": "printf 'hello'" })).expect("bash should succeed");
    let success_output: serde_json::Value = serde_json::from_str(&success).expect("json");
    assert_eq!(success_output["stdout"], "hello");
    assert_eq!(success_output["interrupted"], false);

    let failure = execute_tool("bash", &json!({ "command": "printf 'oops' >&2; exit 7" }))
        .expect("bash failure should still return structured output");
    let failure_output: serde_json::Value = serde_json::from_str(&failure).expect("json");
    assert_eq!(failure_output["returnCodeInterpretation"], "exit_code:7");
    assert!(failure_output["stderr"]
        .as_str()
        .expect("stderr")
        .contains("oops"));

    let timeout = execute_tool("bash", &json!({ "command": "sleep 1", "timeout": 10 }))
        .expect("bash timeout should return output");
    let timeout_output: serde_json::Value = serde_json::from_str(&timeout).expect("json");
    assert_eq!(timeout_output["interrupted"], true);
    assert_eq!(timeout_output["returnCodeInterpretation"], "timeout");
    assert!(timeout_output["stderr"]
        .as_str()
        .expect("stderr")
        .contains("Command exceeded timeout"));

    let background = execute_tool(
        "bash",
        &json!({ "command": "sleep 1", "run_in_background": true }),
    )
    .expect("bash background should succeed");
    let background_output: serde_json::Value = serde_json::from_str(&background).expect("json");
    assert!(background_output["backgroundTaskId"].as_str().is_some());
    assert_eq!(background_output["noOutputExpected"], true);
}

#[test]
fn bash_workspace_tests_are_blocked_when_branch_is_behind_main() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = temp_path("workspace-test-preflight");
    let original_dir = std::env::current_dir().expect("cwd");
    init_git_repo(&root);
    run_git(&root, &["checkout", "-b", "feature/stale-tests"]);
    run_git(&root, &["checkout", "main"]);
    commit_file(
        &root,
        "hotfix.txt",
        "fix from main\n",
        "fix: unblock workspace tests",
    );
    run_git(&root, &["checkout", "feature/stale-tests"]);
    std::env::set_current_dir(&root).expect("set cwd");

    let output = execute_tool(
        "bash",
        &json!({ "command": "cargo test --workspace --all-targets" }),
    )
    .expect("preflight should return structured output");
    let output_json: serde_json::Value = serde_json::from_str(&output).expect("json");
    assert_eq!(
        output_json["returnCodeInterpretation"],
        "preflight_blocked:branch_divergence"
    );
    assert!(output_json["stderr"]
        .as_str()
        .expect("stderr")
        .contains("branch divergence detected before workspace tests"));
    assert_eq!(
        output_json["structuredContent"][0]["event"],
        "branch.stale_against_main"
    );
    assert_eq!(
        output_json["structuredContent"][0]["failureClass"],
        "branch_divergence"
    );
    assert_eq!(
        output_json["structuredContent"][0]["data"]["missingCommits"][0],
        "fix: unblock workspace tests"
    );

    std::env::set_current_dir(&original_dir).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn bash_targeted_tests_skip_branch_preflight() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = temp_path("targeted-test-no-preflight");
    let original_dir = std::env::current_dir().expect("cwd");
    init_git_repo(&root);
    run_git(&root, &["checkout", "-b", "feature/targeted-tests"]);
    run_git(&root, &["checkout", "main"]);
    commit_file(
        &root,
        "hotfix.txt",
        "fix from main\n",
        "fix: only broad tests should block",
    );
    run_git(&root, &["checkout", "feature/targeted-tests"]);
    std::env::set_current_dir(&root).expect("set cwd");

    let output = execute_tool(
        "bash",
        &json!({ "command": "printf 'targeted ok'; cargo test -p runtime stale_branch" }),
    )
    .expect("targeted commands should still execute");
    let output_json: serde_json::Value = serde_json::from_str(&output).expect("json");
    assert_ne!(
        output_json["returnCodeInterpretation"],
        "preflight_blocked:branch_divergence"
    );

    std::env::set_current_dir(&original_dir).expect("restore cwd");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn file_tools_cover_read_write_and_edit_behaviors() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = temp_path("fs-suite");
    fs::create_dir_all(&root).expect("create root");
    let original_dir = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    let write_create = execute_tool(
        "write_file",
        &json!({ "path": "nested/demo.txt", "content": "alpha\nbeta\nalpha\n" }),
    )
    .expect("write create should succeed");
    let write_create_output: serde_json::Value = serde_json::from_str(&write_create).expect("json");
    assert_eq!(write_create_output["type"], "create");
    assert!(root.join("nested/demo.txt").exists());

    let write_update = execute_tool(
        "write_file",
        &json!({ "path": "nested/demo.txt", "content": "alpha\nbeta\ngamma\n" }),
    )
    .expect("write update should succeed");
    let write_update_output: serde_json::Value = serde_json::from_str(&write_update).expect("json");
    assert_eq!(write_update_output["type"], "update");
    assert_eq!(write_update_output["originalFile"], "alpha\nbeta\nalpha\n");

    let read_full = execute_tool("read_file", &json!({ "path": "nested/demo.txt" }))
        .expect("read full should succeed");
    let read_full_output: serde_json::Value = serde_json::from_str(&read_full).expect("json");
    assert_eq!(read_full_output["file"]["content"], "alpha\nbeta\ngamma");
    assert_eq!(read_full_output["file"]["startLine"], 1);

    let read_slice = execute_tool(
        "read_file",
        &json!({ "path": "nested/demo.txt", "offset": 1, "limit": 1 }),
    )
    .expect("read slice should succeed");
    let read_slice_output: serde_json::Value = serde_json::from_str(&read_slice).expect("json");
    assert_eq!(read_slice_output["file"]["content"], "beta");
    assert_eq!(read_slice_output["file"]["startLine"], 2);

    let read_past_end = execute_tool(
        "read_file",
        &json!({ "path": "nested/demo.txt", "offset": 50 }),
    )
    .expect("read past EOF should succeed");
    let read_past_end_output: serde_json::Value =
        serde_json::from_str(&read_past_end).expect("json");
    assert_eq!(read_past_end_output["file"]["content"], "");
    assert_eq!(read_past_end_output["file"]["startLine"], 4);

    let read_error = execute_tool("read_file", &json!({ "path": "missing.txt" }))
        .expect_err("missing file should fail");
    assert!(!read_error.is_empty());

    let edit_once = execute_tool(
        "edit_file",
        &json!({ "path": "nested/demo.txt", "old_string": "alpha", "new_string": "omega" }),
    )
    .expect("single edit should succeed");
    let edit_once_output: serde_json::Value = serde_json::from_str(&edit_once).expect("json");
    assert_eq!(edit_once_output["replaceAll"], false);
    assert_eq!(
        fs::read_to_string(root.join("nested/demo.txt")).expect("read file"),
        "omega\nbeta\ngamma\n"
    );

    execute_tool(
        "write_file",
        &json!({ "path": "nested/demo.txt", "content": "alpha\nbeta\nalpha\n" }),
    )
    .expect("reset file");
    let edit_all = execute_tool(
        "edit_file",
        &json!({
            "path": "nested/demo.txt",
            "old_string": "alpha",
            "new_string": "omega",
            "replace_all": true
        }),
    )
    .expect("replace all should succeed");
    let edit_all_output: serde_json::Value = serde_json::from_str(&edit_all).expect("json");
    assert_eq!(edit_all_output["replaceAll"], true);
    assert_eq!(
        fs::read_to_string(root.join("nested/demo.txt")).expect("read file"),
        "omega\nbeta\nomega\n"
    );

    let edit_same = execute_tool(
        "edit_file",
        &json!({ "path": "nested/demo.txt", "old_string": "omega", "new_string": "omega" }),
    )
    .expect_err("identical old/new should fail");
    assert!(edit_same.contains("must differ"));

    let edit_missing = execute_tool(
        "edit_file",
        &json!({ "path": "nested/demo.txt", "old_string": "missing", "new_string": "omega" }),
    )
    .expect_err("missing substring should fail");
    assert!(edit_missing.contains("old_string not found"));

    std::env::set_current_dir(&original_dir).expect("restore cwd");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn glob_and_grep_tools_cover_success_and_errors() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = temp_path("search-suite");
    fs::create_dir_all(root.join("nested")).expect("create root");
    let original_dir = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(&root).expect("set cwd");

    fs::write(
        root.join("nested/lib.rs"),
        "fn main() {}\nlet alpha = 1;\nlet alpha = 2;\n",
    )
    .expect("write rust file");
    fs::write(root.join("nested/notes.txt"), "alpha\nbeta\n").expect("write txt file");

    let globbed = execute_tool("glob_search", &json!({ "pattern": "nested/*.rs" }))
        .expect("glob should succeed");
    let globbed_output: serde_json::Value = serde_json::from_str(&globbed).expect("json");
    assert_eq!(globbed_output["numFiles"], 1);
    assert!(globbed_output["filenames"][0]
        .as_str()
        .expect("filename")
        .ends_with("nested/lib.rs"));

    let glob_error = execute_tool("glob_search", &json!({ "pattern": "[" }))
        .expect_err("invalid glob should fail");
    assert!(!glob_error.is_empty());

    let grep_content = execute_tool(
        "grep_search",
        &json!({
            "pattern": "alpha",
            "path": "nested",
            "glob": "*.rs",
            "output_mode": "content",
            "-n": true,
            "head_limit": 1,
            "offset": 1
        }),
    )
    .expect("grep content should succeed");
    let grep_content_output: serde_json::Value = serde_json::from_str(&grep_content).expect("json");
    assert_eq!(grep_content_output["numFiles"], 0);
    assert!(grep_content_output["appliedLimit"].is_null());
    assert_eq!(grep_content_output["appliedOffset"], 1);
    assert!(grep_content_output["content"]
        .as_str()
        .expect("content")
        .contains("let alpha = 2;"));

    let grep_count = execute_tool(
        "grep_search",
        &json!({ "pattern": "alpha", "path": "nested", "output_mode": "count" }),
    )
    .expect("grep count should succeed");
    let grep_count_output: serde_json::Value = serde_json::from_str(&grep_count).expect("json");
    assert_eq!(grep_count_output["numMatches"], 3);

    let grep_error = execute_tool(
        "grep_search",
        &json!({ "pattern": "(alpha", "path": "nested" }),
    )
    .expect_err("invalid regex should fail");
    assert!(!grep_error.is_empty());

    std::env::set_current_dir(&original_dir).expect("restore cwd");
    let _ = fs::remove_dir_all(root);
}

#[test]
fn sleep_waits_and_reports_duration() {
    let started = std::time::Instant::now();
    let result = execute_tool("Sleep", &json!({"duration_ms": 20})).expect("Sleep should succeed");
    let elapsed = started.elapsed();
    let output: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(output["duration_ms"], 20);
    assert!(output["message"]
        .as_str()
        .expect("message")
        .contains("Slept for 20ms"));
    assert!(elapsed >= Duration::from_millis(15));
}

#[test]
fn given_excessive_duration_when_sleep_then_rejects_with_error() {
    let result = execute_tool("Sleep", &json!({"duration_ms": 999_999_999_u64}));
    let error = result.expect_err("excessive sleep should fail");
    assert!(error.contains("exceeds maximum allowed sleep"));
}

#[test]
fn given_zero_duration_when_sleep_then_succeeds() {
    let result =
        execute_tool("Sleep", &json!({"duration_ms": 0})).expect("0ms sleep should succeed");
    let output: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(output["duration_ms"], 0);
}

#[test]
fn brief_returns_sent_message_and_attachment_metadata() {
    let attachment = std::env::temp_dir().join(format!(
        "clawd-brief-{}.png",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    std::fs::write(&attachment, b"png-data").expect("write attachment");

    let result = execute_tool(
        "SendUserMessage",
        &json!({
            "message": "hello user",
            "attachments": [attachment.display().to_string()],
            "status": "normal"
        }),
    )
    .expect("SendUserMessage should succeed");

    let output: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(output["message"], "hello user");
    assert!(output["sentAt"].as_str().is_some());
    assert_eq!(output["attachments"][0]["isImage"], true);
    let _ = std::fs::remove_file(attachment);
}

#[test]
fn config_reads_and_writes_supported_values() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = std::env::temp_dir().join(format!(
        "clawd-config-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let home = root.join("home");
    let cwd = root.join("cwd");
    std::fs::create_dir_all(home.join(".claw")).expect("home dir");
    std::fs::create_dir_all(cwd.join(".claw")).expect("cwd dir");
    std::fs::write(
        home.join(".claw").join("settings.json"),
        r#"{"verbose":false}"#,
    )
    .expect("write global settings");

    let original_home = std::env::var("HOME").ok();
    let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
    let original_dir = std::env::current_dir().expect("cwd");
    std::env::set_var("HOME", &home);
    std::env::remove_var("CLAW_CONFIG_HOME");
    std::env::set_current_dir(&cwd).expect("set cwd");

    let get = execute_tool("Config", &json!({"setting": "verbose"})).expect("get config");
    let get_output: serde_json::Value = serde_json::from_str(&get).expect("json");
    assert_eq!(get_output["value"], false);

    let set = execute_tool(
        "Config",
        &json!({"setting": "permissions.defaultMode", "value": "plan"}),
    )
    .expect("set config");
    let set_output: serde_json::Value = serde_json::from_str(&set).expect("json");
    assert_eq!(set_output["operation"], "set");
    assert_eq!(set_output["newValue"], "plan");

    let invalid = execute_tool(
        "Config",
        &json!({"setting": "permissions.defaultMode", "value": "bogus"}),
    )
    .expect_err("invalid config value should error");
    assert!(invalid.contains("Invalid value"));

    let unknown =
        execute_tool("Config", &json!({"setting": "nope"})).expect("unknown setting result");
    let unknown_output: serde_json::Value = serde_json::from_str(&unknown).expect("json");
    assert_eq!(unknown_output["success"], false);

    std::env::set_current_dir(&original_dir).expect("restore cwd");
    match original_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    match original_config_home {
        Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
        None => std::env::remove_var("CLAW_CONFIG_HOME"),
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn enter_and_exit_plan_mode_round_trip_existing_local_override() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = std::env::temp_dir().join(format!(
        "clawd-plan-mode-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let home = root.join("home");
    let cwd = root.join("cwd");
    std::fs::create_dir_all(home.join(".claw")).expect("home dir");
    std::fs::create_dir_all(cwd.join(".claw")).expect("cwd dir");
    std::fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{"permissions":{"defaultMode":"acceptEdits"}}"#,
    )
    .expect("write local settings");

    let original_home = std::env::var("HOME").ok();
    let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
    let original_dir = std::env::current_dir().expect("cwd");
    std::env::set_var("HOME", &home);
    std::env::remove_var("CLAW_CONFIG_HOME");
    std::env::set_current_dir(&cwd).expect("set cwd");

    let enter = execute_tool("EnterPlanMode", &json!({})).expect("enter plan mode");
    let enter_output: serde_json::Value = serde_json::from_str(&enter).expect("json");
    assert_eq!(enter_output["changed"], true);
    assert_eq!(enter_output["managed"], true);
    assert_eq!(enter_output["previousLocalMode"], "acceptEdits");
    assert_eq!(enter_output["currentLocalMode"], "plan");

    let local_settings = std::fs::read_to_string(cwd.join(".claw").join("settings.local.json"))
        .expect("local settings after enter");
    assert!(local_settings.contains(r#""defaultMode": "plan""#));
    let state =
        std::fs::read_to_string(cwd.join(".claw").join("tool-state").join("plan-mode.json"))
            .expect("plan mode state");
    assert!(state.contains(r#""hadLocalOverride": true"#));
    assert!(state.contains(r#""previousLocalMode": "acceptEdits""#));

    let exit = execute_tool("ExitPlanMode", &json!({})).expect("exit plan mode");
    let exit_output: serde_json::Value = serde_json::from_str(&exit).expect("json");
    assert_eq!(exit_output["changed"], true);
    assert_eq!(exit_output["managed"], false);
    assert_eq!(exit_output["previousLocalMode"], "acceptEdits");
    assert_eq!(exit_output["currentLocalMode"], "acceptEdits");

    let local_settings = std::fs::read_to_string(cwd.join(".claw").join("settings.local.json"))
        .expect("local settings after exit");
    assert!(local_settings.contains(r#""defaultMode": "acceptEdits""#));
    assert!(!cwd
        .join(".claw")
        .join("tool-state")
        .join("plan-mode.json")
        .exists());

    std::env::set_current_dir(&original_dir).expect("restore cwd");
    match original_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    match original_config_home {
        Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
        None => std::env::remove_var("CLAW_CONFIG_HOME"),
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn exit_plan_mode_clears_override_when_enter_created_it_from_empty_local_state() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = std::env::temp_dir().join(format!(
        "clawd-plan-mode-empty-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    let home = root.join("home");
    let cwd = root.join("cwd");
    std::fs::create_dir_all(home.join(".claw")).expect("home dir");
    std::fs::create_dir_all(cwd.join(".claw")).expect("cwd dir");

    let original_home = std::env::var("HOME").ok();
    let original_config_home = std::env::var("CLAW_CONFIG_HOME").ok();
    let original_dir = std::env::current_dir().expect("cwd");
    std::env::set_var("HOME", &home);
    std::env::remove_var("CLAW_CONFIG_HOME");
    std::env::set_current_dir(&cwd).expect("set cwd");

    let enter = execute_tool("EnterPlanMode", &json!({})).expect("enter plan mode");
    let enter_output: serde_json::Value = serde_json::from_str(&enter).expect("json");
    assert_eq!(enter_output["previousLocalMode"], serde_json::Value::Null);
    assert_eq!(enter_output["currentLocalMode"], "plan");

    let exit = execute_tool("ExitPlanMode", &json!({})).expect("exit plan mode");
    let exit_output: serde_json::Value = serde_json::from_str(&exit).expect("json");
    assert_eq!(exit_output["changed"], true);
    assert_eq!(exit_output["currentLocalMode"], serde_json::Value::Null);

    let local_settings = std::fs::read_to_string(cwd.join(".claw").join("settings.local.json"))
        .expect("local settings after exit");
    let local_settings_json: serde_json::Value =
        serde_json::from_str(&local_settings).expect("valid settings json");
    assert_eq!(
        local_settings_json.get("permissions"),
        None,
        "permissions override should be removed on exit"
    );
    assert!(!cwd
        .join(".claw")
        .join("tool-state")
        .join("plan-mode.json")
        .exists());

    std::env::set_current_dir(&original_dir).expect("restore cwd");
    match original_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    match original_config_home {
        Some(value) => std::env::set_var("CLAW_CONFIG_HOME", value),
        None => std::env::remove_var("CLAW_CONFIG_HOME"),
    }
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn structured_output_echoes_input_payload() {
    let result = execute_tool("StructuredOutput", &json!({"ok": true, "items": [1, 2, 3]}))
        .expect("StructuredOutput should succeed");
    let output: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(output["data"], "Structured output provided successfully");
    assert_eq!(output["structured_output"]["ok"], true);
    assert_eq!(output["structured_output"]["items"][1], 2);
}

#[test]
fn given_empty_payload_when_structured_output_then_rejects_with_error() {
    let result = execute_tool("StructuredOutput", &json!({}));
    let error = result.expect_err("empty payload should fail");
    assert!(error.contains("must not be empty"));
}

#[test]
fn repl_executes_python_code() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let result = execute_tool(
        "REPL",
        &json!({"language": "python", "code": "print(1 + 1)", "timeout_ms": 500}),
    )
    .expect("REPL should succeed");
    let output: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(output["language"], "python");
    assert_eq!(output["exitCode"], 0);
    assert!(output["stdout"].as_str().expect("stdout").contains('2'));
}

#[test]
fn given_empty_code_when_repl_then_rejects_with_error() {
    let result = execute_tool("REPL", &json!({"language": "python", "code": "   "}));

    let error = result.expect_err("empty REPL code should fail");
    assert!(error.contains("code must not be empty"));
}

#[test]
fn given_unsupported_language_when_repl_then_rejects_with_error() {
    let result = execute_tool("REPL", &json!({"language": "ruby", "code": "puts 1"}));

    let error = result.expect_err("unsupported REPL language should fail");
    assert!(error.contains("unsupported REPL language: ruby"));
}

#[test]
fn given_timeout_ms_when_repl_blocks_then_returns_timeout_error() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let result = execute_tool(
        "REPL",
        &json!({
            "language": "python",
            "code": "import time\ntime.sleep(1)",
            "timeout_ms": 10
        }),
    );

    let error = result.expect_err("timed out REPL execution should fail");
    assert!(error.contains("REPL execution exceeded timeout of 10 ms"));
}

#[test]
fn powershell_runs_via_stub_shell() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = std::env::temp_dir().join(format!(
        "clawd-pwsh-bin-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create dir");
    let script = dir.join("pwsh");
    std::fs::write(
        &script,
        r#"#!/bin/sh
while [ "$1" != "-Command" ] && [ $# -gt 0 ]; do shift; done
shift
printf 'pwsh:%s' "$1"
"#,
    )
    .expect("write script");
    std::process::Command::new("/bin/chmod")
        .arg("+x")
        .arg(&script)
        .status()
        .expect("chmod");
    let original_path = std::env::var("PATH").unwrap_or_default();
    std::env::set_var("PATH", format!("{}:/bin:/usr/bin", dir.display()));

    let result = execute_tool("PowerShell", &json!({"command": "Write-Output hello"}))
        .expect("PowerShell should succeed");

    let background = execute_tool(
        "PowerShell",
        &json!({"command": "Write-Output hello", "run_in_background": true}),
    )
    .expect("PowerShell background should succeed");

    std::env::set_var("PATH", original_path);
    let _ = std::fs::remove_dir_all(dir);

    let output: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(output["stdout"], "pwsh:Write-Output hello");
    assert!(output["stderr"].as_str().expect("stderr").is_empty());

    let background_output: serde_json::Value = serde_json::from_str(&background).expect("json");
    assert!(background_output["backgroundTaskId"].as_str().is_some());
    assert_eq!(background_output["backgroundedByUser"], true);
    assert_eq!(background_output["assistantAutoBackgrounded"], false);
}

#[test]
fn powershell_errors_when_shell_is_missing() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let original_path = std::env::var("PATH").unwrap_or_default();
    let empty_dir = std::env::temp_dir().join(format!(
        "clawd-empty-bin-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos()
    ));
    std::fs::create_dir_all(&empty_dir).expect("create empty dir");
    std::env::set_var("PATH", empty_dir.display().to_string());

    let err = execute_tool("PowerShell", &json!({"command": "Write-Output hello"}))
        .expect_err("PowerShell should fail when shell is missing");

    std::env::set_var("PATH", original_path);
    let _ = std::fs::remove_dir_all(empty_dir);

    assert!(err.contains("PowerShell executable not found"));
}

fn read_only_capability_runtime() -> CapabilityRuntime {
    let policy = mvp_tool_specs().into_iter().fold(
        PermissionPolicy::new(runtime::PermissionMode::ReadOnly),
        |policy, spec| policy.with_tool_requirement(spec.name, spec.required_permission),
    );
    capability_runtime_from_sources(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Some(PermissionEnforcer::new(policy)),
    )
}

#[test]
fn given_read_only_enforcer_when_bash_then_denied() {
    let runtime = read_only_capability_runtime();
    let err = execute_local_tool_with_runtime(&runtime, "bash", &json!({ "command": "echo hi" }))
        .expect_err("bash should be denied in read-only mode");
    assert!(
        err.contains("current mode is read-only"),
        "should cite active mode: {err}"
    );
}

#[test]
fn given_read_only_enforcer_when_write_file_then_denied() {
    let runtime = read_only_capability_runtime();
    let err = execute_local_tool_with_runtime(
        &runtime,
        "write_file",
        &json!({ "path": "/tmp/x.txt", "content": "x" }),
    )
    .expect_err("write_file should be denied in read-only mode");
    assert!(
        err.contains("current mode is read-only"),
        "should cite active mode: {err}"
    );
}

#[test]
fn given_read_only_enforcer_when_edit_file_then_denied() {
    let runtime = read_only_capability_runtime();
    let err = execute_local_tool_with_runtime(
        &runtime,
        "edit_file",
        &json!({ "path": "/tmp/x.txt", "old_string": "a", "new_string": "b" }),
    )
    .expect_err("edit_file should be denied in read-only mode");
    assert!(
        err.contains("current mode is read-only"),
        "should cite active mode: {err}"
    );
}

#[test]
fn given_read_only_enforcer_when_read_file_then_not_permission_denied() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let root = temp_path("perm-read");
    fs::create_dir_all(&root).expect("create root");
    let file = root.join("readable.txt");
    fs::write(&file, "content\n").expect("write test file");

    let runtime = read_only_capability_runtime();
    let result = execute_local_tool_with_runtime(
        &runtime,
        "read_file",
        &json!({ "path": file.display().to_string() }),
    );
    assert!(result.is_ok(), "read_file should be allowed: {result:?}");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn given_read_only_enforcer_when_glob_search_then_not_permission_denied() {
    let runtime = read_only_capability_runtime();
    let result =
        execute_local_tool_with_runtime(&runtime, "glob_search", &json!({ "pattern": "*.rs" }));
    assert!(
        result.is_ok(),
        "glob_search should be allowed in read-only mode: {result:?}"
    );
}

#[test]
fn given_no_enforcer_when_bash_then_executes_normally() {
    let _guard = env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let runtime = CapabilityRuntime::builtin();
    let result =
        execute_local_tool_with_runtime(&runtime, "bash", &json!({ "command": "printf 'ok'" }))
            .expect("bash should succeed without enforcer");
    let output: serde_json::Value = serde_json::from_str(&result).expect("json");
    assert_eq!(output["stdout"], "ok");
}

#[test]
fn builtin_capability_runtime_matches_provider_built_surface() {
    let runtime = CapabilityRuntime::builtin();
    let provider_runtime =
        capability_runtime_from_sources(Vec::new(), Vec::new(), Vec::new(), None);

    let runtime_defs = runtime
        .planned_tool_definitions(CapabilityPlannerInput::default())
        .expect("builtin runtime definitions");
    let provider_defs = provider_runtime
        .planned_tool_definitions(CapabilityPlannerInput::default())
        .expect("provider runtime definitions");

    let runtime_names = runtime_defs
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();
    let provider_names = provider_defs
        .into_iter()
        .map(|definition| definition.name)
        .collect::<Vec<_>>();
    assert_eq!(runtime_names, provider_names);
}

#[test]
fn capability_runtime_normalize_allowed_tools_matches_provider_resolution() {
    let plugin_tools = vec![PluginTool::new(
        "plugin-demo@external",
        "plugin-demo",
        PluginToolDefinition {
            name: "plugin_echo".into(),
            description: Some("Echo from plugin".into()),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" }
                },
                "required": ["message"],
                "additionalProperties": false
            }),
        },
        "echo",
        Vec::new(),
        PluginToolPermission::WorkspaceWrite,
        None,
    )];
    let runtime_tools = vec![super::RuntimeToolDefinition {
        name: "mcp__demo__echo".into(),
        description: Some("Echo from runtime".into()),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message": { "type": "string" }
            },
            "required": ["message"],
            "additionalProperties": false
        }),
        required_permission: PermissionMode::ReadOnly,
    }];
    let provider = capability_provider_from_sources(plugin_tools, runtime_tools, Vec::new(), None);
    let runtime = CapabilityRuntime::new(provider.clone());
    let values = vec![
        "read,plugin_echo".to_string(),
        "mcp__demo__echo".to_string(),
    ];

    let provider_allowed = provider
        .normalize_allowed_tools(&values)
        .expect("provider allow-list");
    let runtime_allowed = runtime
        .normalize_allowed_tools(&values)
        .expect("runtime allow-list");

    assert_eq!(runtime_allowed, provider_allowed);
}

fn setup_managed_mcp_runtime_fixture(
    include_broken_server: bool,
) -> (PathBuf, PathBuf, super::ManagedMcpRuntime) {
    let config_home = temp_path("managed-mcp-config");
    let workspace = temp_path("managed-mcp-workspace");
    fs::create_dir_all(&config_home).expect("config home should exist");
    fs::create_dir_all(&workspace).expect("workspace should exist");

    let script_path = workspace.join("fixture-mcp.py");
    write_mcp_server_fixture(&script_path);

    let mcp_servers = if include_broken_server {
        format!(
            r#"{{
              "alpha": {{
                "command": "python3",
                "args": ["{}"]
              }},
              "broken": {{
                "command": "python3",
                "args": ["-c", "import sys; sys.exit(0)"]
              }}
            }}"#,
            script_path.to_string_lossy()
        )
    } else {
        format!(
            r#"{{
              "alpha": {{
                "command": "python3",
                "args": ["{}"]
              }}
            }}"#,
            script_path.to_string_lossy()
        )
    };

    fs::write(
        config_home.join("settings.json"),
        format!(r#"{{"mcpServers": {mcp_servers}}}"#),
    )
    .expect("mcp settings should write");

    let loader = ConfigLoader::new(&workspace, &config_home);
    let runtime_config = loader.load().expect("runtime config should load");
    let mcp_runtime = super::ManagedMcpRuntime::new(&runtime_config)
        .expect("managed mcp runtime should build")
        .expect("managed mcp runtime should exist");

    (config_home, workspace, mcp_runtime)
}

fn cleanup_mcp_runtime_fixture(config_home: &Path, workspace: &Path) {
    let _ = fs::remove_dir_all(config_home);
    let _ = fs::remove_dir_all(workspace);
}

fn write_mcp_server_fixture(script_path: &Path) {
    let script = [
        "#!/usr/bin/env python3",
        "import json, sys",
        "",
        "def read_message():",
        "    header = b''",
        r"    while not header.endswith(b'\r\n\r\n'):",
        "        chunk = sys.stdin.buffer.read(1)",
        "        if not chunk:",
        "            return None",
        "        header += chunk",
        "    length = 0",
        r"    for line in header.decode().split('\r\n'):",
        r"        if line.lower().startswith('content-length:'):",
        "            length = int(line.split(':', 1)[1].strip())",
        "    payload = sys.stdin.buffer.read(length)",
        "    return json.loads(payload.decode())",
        "",
        "def send_message(message):",
        "    payload = json.dumps(message).encode()",
        r"    sys.stdout.buffer.write(f'Content-Length: {len(payload)}\r\n\r\n'.encode() + payload)",
        "    sys.stdout.buffer.flush()",
        "",
        "while True:",
        "    request = read_message()",
        "    if request is None:",
        "        break",
        "    method = request['method']",
        "    if method == 'initialize':",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request['id'],",
        "            'result': {",
        "                'protocolVersion': request['params']['protocolVersion'],",
        "                'capabilities': {'tools': {}, 'resources': {}, 'prompts': {}},",
        "                'serverInfo': {'name': 'fixture', 'version': '1.0.0'}",
        "            }",
        "        })",
        "    elif method == 'tools/list':",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request['id'],",
        "            'result': {",
        "                'tools': [",
        "                    {",
        "                        'name': 'echo',",
        "                        'description': 'Echo from MCP fixture',",
        "                        'inputSchema': {",
        "                            'type': 'object',",
        "                            'properties': {'text': {'type': 'string'}},",
        "                            'required': ['text']",
        "                        },",
        "                        'annotations': {'readOnlyHint': True}",
        "                    }",
        "                ]",
        "            }",
        "        })",
        "    elif method == 'tools/call':",
        "        arguments = request['params'].get('arguments') or {}",
        "        text = arguments.get('text', '')",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request['id'],",
        "            'result': {",
        "                'content': [{'type': 'text', 'text': text}],",
        "                'structuredContent': {'echoed': text},",
        "                'isError': False",
        "            }",
        "        })",
        "    elif method == 'prompts/list':",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request['id'],",
        "            'result': {",
        "                'prompts': [",
        "                    {",
        "                        'name': 'workspace-guide',",
        "                        'description': 'MCP workspace guidance',",
        "                        'arguments': [",
        "                            {",
        "                                'name': 'topic',",
        "                                'description': 'Workspace topic',",
        "                                'required': False",
        "                            }",
        "                        ]",
        "                    }",
        "                ]",
        "            }",
        "        })",
        "    elif method == 'prompts/get':",
        "        arguments = request['params'].get('arguments') or {}",
        "        topic = arguments.get('topic', 'workspace')",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request['id'],",
        "            'result': {",
        "                'description': 'MCP workspace guidance',",
        "                'messages': [",
        "                    {",
        "                        'role': 'system',",
        "                        'content': {",
        "                            'type': 'text',",
        "                            'text': f'MCP workspace guidance for {topic}'",
        "                        }",
        "                    }",
        "                ]",
        "            }",
        "        })",
        "    elif method == 'resources/list':",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request['id'],",
        "            'result': {",
        "                'resources': [",
        "                    {",
        "                        'uri': 'file://guide.txt',",
        "                        'name': 'Guide',",
        "                        'description': 'Workspace guide',",
        "                        'mimeType': 'text/plain'",
        "                    }",
        "                ]",
        "            }",
        "        })",
        "    elif method == 'resources/read':",
        "        uri = request['params']['uri']",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request['id'],",
        "            'result': {",
        "                'contents': [",
        "                    {'uri': uri, 'mimeType': 'text/plain', 'text': f'contents for {uri}'}",
        "                ]",
        "            }",
        "        })",
        "    elif method == 'notifications/initialized':",
        "        continue",
        "    else:",
        "        send_message({",
        "            'jsonrpc': '2.0',",
        "            'id': request.get('id'),",
        "            'error': {",
        "                'code': -32601,",
        "                'message': f'unsupported method {method}'",
        "            }",
        "        })",
    ];

    fs::write(script_path, script.join("\n")).expect("fixture mcp script should write");
}

struct TestServer {
    addr: SocketAddr,
    shutdown: Option<std::sync::mpsc::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl TestServer {
    fn spawn(handler: Arc<dyn Fn(&str) -> HttpResponse + Send + Sync + 'static>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        listener
            .set_nonblocking(true)
            .expect("set nonblocking listener");
        let addr = listener.local_addr().expect("local addr");
        let (tx, rx) = std::sync::mpsc::channel::<()>();

        let handle = thread::spawn(move || loop {
            if rx.try_recv().is_ok() {
                break;
            }

            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut buffer = [0_u8; 4096];
                    let size = stream.read(&mut buffer).expect("read request");
                    let request = String::from_utf8_lossy(&buffer[..size]).into_owned();
                    let request_line = request.lines().next().unwrap_or_default().to_string();
                    let response = handler(&request_line);
                    stream
                        .write_all(response.to_bytes().as_slice())
                        .expect("write response");
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("server accept failed: {error}"),
            }
        });

        Self {
            addr,
            shutdown: Some(tx),
            handle: Some(handle),
        }
    }

    fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.handle.take() {
            handle.join().expect("join test server");
        }
    }
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: String,
}

impl HttpResponse {
    fn html(status: u16, reason: &'static str, body: &str) -> Self {
        Self {
            status,
            reason,
            content_type: "text/html; charset=utf-8",
            body: body.to_string(),
        }
    }

    fn text(status: u16, reason: &'static str, body: &str) -> Self {
        Self {
            status,
            reason,
            content_type: "text/plain; charset=utf-8",
            body: body.to_string(),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        format!(
                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                self.status,
                self.reason,
                self.content_type,
                self.body.len(),
                self.body
            )
            .into_bytes()
    }
}
