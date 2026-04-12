mod builtin_exec;
mod capability_runtime;
mod fs_shell;
mod skill_runtime;
#[cfg(test)]
mod split_module_tests;
mod tool_registry;
mod web_external;
mod workspace_runtime;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use api::{
    ContentBlockDelta, InputContentBlock, InputMessage, MessageRequest, MessageResponse,
    OutputContentBlock, ProviderClient, StreamEvent as ApiStreamEvent, ToolChoice,
    ToolResultContentBlock, max_tokens_for_model, resolve_model_alias,
};
use reqwest::blocking::Client;
use runtime::{
    ApiClient, ApiRequest, AssistantEvent, BashCommandInput, BashCommandOutput, BranchFreshness,
    ConfigLoader, ContentBlock, ConversationMessage, ConversationRuntime, GrepSearchInput,
    LaneCommitProvenance, LaneEvent, LaneEventBlocker, LaneEventName, LaneEventStatus,
    LaneFailureClass, McpDegradedReport, MessageRole, PermissionMode, PermissionPolicy,
    PromptCacheEvent, RuntimeError, Session, TaskPacket, ToolError, ToolExecutor, check_freshness,
    dedupe_superseded_commit_events, edit_file, execute_bash, glob_search, grep_search,
    load_system_prompt,
    lsp_client::LspRegistry,
    mcp_tool_bridge::McpToolRegistry,
    permission_enforcer::{EnforcementResult, PermissionEnforcer},
    read_file,
    summary_compression::compress_summary_text,
    task_registry::TaskRegistry,
    team_cron_registry::{CronRegistry, TeamRegistry},
    worker_boot::{WorkerReadySnapshot, WorkerRegistry},
    write_file,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub use builtin_exec::{enforce_permission_check, execute_tool};
pub use capability_runtime::{
    CapabilityActivation, CapabilityCompiler, CapabilityConcurrencyPolicy, CapabilityDispatchKind,
    CapabilityExecutionEvent, CapabilityExecutionKind, CapabilityExecutionPhase,
    CapabilityExecutionRequest, CapabilityExecutor, CapabilityMediationDecision, CapabilityPlanner,
    CapabilityPlannerInput, CapabilityProfile, CapabilityProvider, CapabilityRequestOverride,
    CapabilityRuntime, CapabilitySourceKind, CapabilitySpec, CapabilityState,
    CapabilitySurfaceProjection, CapabilityVisibility, ManagedMcpRuntime, McpCapabilityDescriptor,
    McpCapabilityProvider, McpConnectionProjection, SessionCapabilityState, SessionCapabilityStore,
    SharedSessionCapabilityState, apply_skill_session_overrides,
    mcp_resource_capability_descriptor, mcp_tool_capability_descriptor,
    permission_mode_for_mcp_tool,
};
pub use skill_runtime::{SkillDiscoveryOutput, SkillExecutionResult, SkillStateUpdate};
pub use tool_registry::{
    RuntimeToolDefinition, ToolManifestEntry, ToolRegistry, ToolSearchOutput, ToolSource, ToolSpec,
    mvp_tool_specs,
};

#[allow(unused_imports)]
pub(crate) use builtin_exec::{
    AskUserQuestionInput, BriefInput, ConfigInput, EnterPlanModeInput, ExitPlanModeInput,
    SkillDiscoveryInput, SkillToolInput, SleepInput, StructuredOutputInput, TestingPermissionInput,
    TodoWriteInput, ToolSearchInput, execute_tool_with_enforcer, run_brief, run_config,
    run_enter_plan_mode, run_exit_plan_mode, run_skill_discovery, run_skill_tool, run_sleep,
    run_structured_output, run_testing_permission, run_todo_write, run_tool_search, to_pretty_json,
};
#[allow(unused_imports)]
pub(crate) use fs_shell::{
    EditFileInput, GlobSearchInputValue, NotebookEditInput, PowerShellInput, ReadFileInput,
    ReplInput, WriteFileInput, run_bash, run_edit_file, run_glob_search, run_grep_search,
    run_notebook_edit, run_powershell, run_read_file, run_repl, run_write_file,
    workspace_test_branch_preflight,
};
#[allow(unused_imports)]
pub(crate) use tool_registry::{
    canonical_tool_token, deferred_tool_specs, execute_tool_search, normalize_tool_search_query,
    permission_mode_from_plugin, search_tool_specs,
};
#[allow(unused_imports)]
pub(crate) use web_external::{
    RemoteTriggerInput, WebFetchInput, WebSearchInput, run_remote_trigger, run_web_fetch,
    run_web_search,
};
#[allow(unused_imports)]
pub(crate) use workspace_runtime::{
    AgentInput, AgentJob, AgentOutput, CronCreateInput, CronDeleteInput, LspInput, McpAuthInput,
    McpResourceInput, McpToolInput, SubagentToolExecutor, TaskCreateInput, TaskIdInput,
    TaskUpdateInput, TeamCreateInput, TeamDeleteInput, WorkerCreateInput, WorkerIdInput,
    WorkerObserveCompletionInput, WorkerObserveInput, WorkerSendPromptInput,
    agent_permission_policy, allowed_tools_for_subagent, classify_lane_failure, derive_agent_state,
    execute_agent_with_spawn, final_assistant_text, iso8601_now, maybe_commit_provenance,
    persist_agent_terminal_state, push_output_block, run_agent, run_cron_create, run_cron_delete,
    run_cron_list, run_list_mcp_resources, run_lsp, run_mcp_auth, run_mcp_tool,
    run_read_mcp_resource, run_task_create, run_task_get, run_task_list, run_task_output,
    run_task_packet, run_task_stop, run_task_update, run_team_create, run_team_delete,
    run_worker_await_ready, run_worker_create, run_worker_get, run_worker_observe,
    run_worker_observe_completion, run_worker_resolve_trust, run_worker_restart,
    run_worker_send_prompt, run_worker_terminate,
};

pub mod lane_completion;
