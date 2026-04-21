mod builtin_catalog;
mod builtin_exec;
mod fs_shell;
mod lsp_runtime;
mod tool_registry;
mod web_external;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

#[allow(unused_imports)]
use api::{
    max_tokens_for_model, resolve_model_alias, ContentBlockDelta, InputContentBlock, InputMessage,
    MessageRequest, MessageResponse, OutputContentBlock, ProviderClient,
    StreamEvent as ApiStreamEvent, ToolChoice, ToolResultContentBlock,
};
use reqwest::blocking::Client;
#[allow(unused_imports)]
use runtime::{
    check_freshness, dedupe_superseded_commit_events, edit_file, execute_bash, glob_search,
    grep_search, load_system_prompt,
    lsp_client::LspRegistry,
    permission_enforcer::{EnforcementResult, PermissionEnforcer},
    read_file,
    summary_compression::compress_summary_text,
    write_file, ApiClient, ApiRequest, AssistantEvent, BashCommandInput, BashCommandOutput,
    BranchFreshness, ContentBlock, ConversationMessage, ConversationRuntime, GrepSearchInput,
    LaneCommitProvenance, LaneEvent, LaneEventBlocker, LaneEventName, LaneEventStatus,
    LaneFailureClass, McpDegradedReport, MessageRole, PermissionMode, PermissionPolicy,
    PromptCacheEvent, RuntimeError, Session, ToolError, ToolExecutionOutcome, ToolExecutor,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub use builtin_catalog::{
    builtin_capability_catalog, BuiltinCapability, BuiltinCapabilityCatalog,
    BuiltinCapabilityCategory, BuiltinHandlerKey, BuiltinRoleAvailability,
};
pub use builtin_exec::{enforce_permission_check, execute_tool};
pub use tool_registry::{mvp_tool_specs, RuntimeToolDefinition, ToolSearchOutput, ToolSpec};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentOutput {
    pub agent_id: String,
    pub name: String,
    pub description: String,
    pub subagent_type: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub output_file: String,
    pub manifest_file: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub lane_events: Vec<Value>,
    pub derived_state: String,
    pub current_blocker: Option<String>,
    pub error: Option<String>,
}

#[allow(unused_imports)]
pub(crate) use builtin_exec::{
    execute_tool_with_enforcer, run_brief, run_config, run_enter_plan_mode, run_exit_plan_mode,
    run_sleep, run_structured_output, run_testing_permission, run_todo_write, run_tool_search,
    to_pretty_json, AskUserQuestionInput, BriefInput, ConfigInput, EnterPlanModeInput,
    ExitPlanModeInput, SleepInput, StructuredOutputInput, TestingPermissionInput, TodoWriteInput,
    ToolSearchInput,
};
#[allow(unused_imports)]
pub(crate) use fs_shell::{
    run_bash, run_edit_file, run_glob_search, run_grep_search, run_notebook_edit, run_powershell,
    run_read_file, run_repl, run_write_file, workspace_test_branch_preflight, EditFileInput,
    GlobSearchInputValue, NotebookEditInput, PowerShellInput, ReadFileInput, ReplInput,
    WriteFileInput,
};
#[allow(unused_imports)]
pub(crate) use lsp_runtime::{run_lsp, LspInput};
#[allow(unused_imports)]
pub(crate) use tool_registry::{
    canonical_tool_token, deferred_tool_specs, execute_tool_search, normalize_tool_search_query,
    permission_mode_from_plugin, search_tool_specs,
};
#[allow(unused_imports)]
pub(crate) use web_external::{
    run_remote_trigger, run_web_fetch, run_web_search, RemoteTriggerInput, WebFetchInput,
    WebSearchInput,
};

pub mod lane_completion;

#[must_use]
pub fn iso8601_now() -> String {
    format!("{:?}", std::time::SystemTime::now())
}
