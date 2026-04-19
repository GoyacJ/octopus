use std::collections::BTreeSet;
use std::sync::OnceLock;

use runtime::PermissionMode;
use serde_json::{json, Value};

use crate::capability_runtime::CapabilityVisibility;
use crate::tool_registry::ToolSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinCapabilityCategory {
    WorkerPrimitive,
    WebContext,
    ControlPlane,
    Orchestration,
    TestOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinHandlerKey {
    Bash,
    ReadFile,
    WriteFile,
    EditFile,
    GlobSearch,
    GrepSearch,
    WebFetch,
    WebSearch,
    TodoWrite,
    ToolSearch,
    NotebookEdit,
    Sleep,
    Brief,
    Config,
    EnterPlanMode,
    ExitPlanMode,
    StructuredOutput,
    Repl,
    PowerShell,
    AskUserQuestion,
    Lsp,
    RemoteTrigger,
    TestingPermission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BuiltinRoleAvailability {
    pub main_thread_only: bool,
    pub async_worker_allowed: bool,
    pub plan_mode_only: bool,
    pub non_interactive_blocked: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinCapability {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub description: &'static str,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
    pub category: BuiltinCapabilityCategory,
    pub visibility: CapabilityVisibility,
    pub handler_key: BuiltinHandlerKey,
    pub search_hint: Option<&'static str>,
    pub role_availability: BuiltinRoleAvailability,
}

impl BuiltinCapability {
    #[must_use]
    pub fn to_tool_spec(&self) -> ToolSpec {
        ToolSpec {
            name: self.name,
            description: self.description,
            input_schema: self.input_schema.clone(),
            required_permission: self.required_permission,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BuiltinCapabilityCatalog {
    entries: &'static [BuiltinCapability],
}

impl BuiltinCapabilityCatalog {
    #[must_use]
    pub fn entries(self) -> &'static [BuiltinCapability] {
        self.entries
    }

    #[must_use]
    pub fn tool_specs(self) -> Vec<ToolSpec> {
        self.entries
            .iter()
            .map(BuiltinCapability::to_tool_spec)
            .collect()
    }

    #[must_use]
    pub fn builtin_names(self) -> BTreeSet<String> {
        self.entries
            .iter()
            .map(|entry| entry.name.to_string())
            .collect()
    }

    #[must_use]
    pub fn find_tool(self, name: &str) -> Option<&'static BuiltinCapability> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    #[must_use]
    pub fn resolve(self, name: &str) -> Option<&'static BuiltinCapability> {
        self.entries.iter().find(|entry| {
            entry.name.eq_ignore_ascii_case(name)
                || entry
                    .aliases
                    .iter()
                    .any(|alias| alias.eq_ignore_ascii_case(name))
        })
    }
}

#[must_use]
pub fn builtin_capability_catalog() -> BuiltinCapabilityCatalog {
    static CATALOG: OnceLock<Vec<BuiltinCapability>> = OnceLock::new();
    let entries = CATALOG.get_or_init(build_builtin_capabilities);
    BuiltinCapabilityCatalog {
        entries: entries.as_slice(),
    }
}

fn build_builtin_capabilities() -> Vec<BuiltinCapability> {
    vec![
        BuiltinCapability {
            name: "bash",
            aliases: &[],
            description: "Execute a shell command in the current workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "description": { "type": "string" },
                    "run_in_background": { "type": "boolean" },
                    "dangerouslyDisableSandbox": { "type": "boolean" },
                    "namespaceRestrictions": { "type": "boolean" },
                    "isolateNetwork": { "type": "boolean" },
                    "filesystemMode": { "type": "string", "enum": ["off", "workspace-only", "allow-list"] },
                    "allowedMounts": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::Bash,
            search_hint: None,
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "read_file",
            aliases: &["read"],
            description: "Read a text file from the workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "offset": { "type": "integer", "minimum": 0 },
                    "limit": { "type": "integer", "minimum": 1 }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::ReadFile,
            search_hint: None,
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "write_file",
            aliases: &["write"],
            description: "Write a text file in the workspace.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "content": { "type": "string" }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::WriteFile,
            search_hint: None,
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "edit_file",
            aliases: &["edit"],
            description: "Replace text in a workspace file.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "old_string": { "type": "string" },
                    "new_string": { "type": "string" },
                    "replace_all": { "type": "boolean" }
                },
                "required": ["path", "old_string", "new_string"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::EditFile,
            search_hint: None,
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "glob_search",
            aliases: &["glob"],
            description: "Find files by glob pattern.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::GlobSearch,
            search_hint: Some("file glob patterns"),
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "grep_search",
            aliases: &["grep"],
            description: "Search file contents with a regex pattern.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string" },
                    "path": { "type": "string" },
                    "glob": { "type": "string" },
                    "output_mode": { "type": "string" },
                    "-B": { "type": "integer", "minimum": 0 },
                    "-A": { "type": "integer", "minimum": 0 },
                    "-C": { "type": "integer", "minimum": 0 },
                    "context": { "type": "integer", "minimum": 0 },
                    "-n": { "type": "boolean" },
                    "-i": { "type": "boolean" },
                    "type": { "type": "string" },
                    "head_limit": { "type": "integer", "minimum": 1 },
                    "offset": { "type": "integer", "minimum": 0 },
                    "multiline": { "type": "boolean" }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::GrepSearch,
            search_hint: Some("regex content search"),
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "WebFetch",
            aliases: &[],
            description: "Fetch a URL, convert it into readable text, and answer a prompt about it.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string", "format": "uri" },
                    "prompt": { "type": "string" }
                },
                "required": ["url", "prompt"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::WebContext,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::WebFetch,
            search_hint: Some("read web pages"),
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "WebSearch",
            aliases: &["WebSearchTool"],
            description: "Search the web for current information and return cited results.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "minLength": 2 },
                    "allowed_domains": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "blocked_domains": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::WebContext,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::WebSearch,
            search_hint: Some("current internet research"),
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "TodoWrite",
            aliases: &[],
            description: "Update the structured task list for the current session.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": { "type": "string" },
                                "activeForm": { "type": "string" },
                                "status": {
                                    "type": "string",
                                    "enum": ["pending", "in_progress", "completed"]
                                }
                            },
                            "required": ["content", "activeForm", "status"],
                            "additionalProperties": false
                        }
                    }
                },
                "required": ["todos"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::TodoWrite,
            search_hint: Some("session task list"),
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "ToolSearch",
            aliases: &[],
            description: "Search for deferred or specialized tools by exact name or keywords.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "max_results": { "type": "integer", "minimum": 1 }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::ToolSearch,
            search_hint: Some("discover deferred tools"),
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "NotebookEdit",
            aliases: &[],
            description: "Replace, insert, or delete a cell in a Jupyter notebook.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "notebook_path": { "type": "string" },
                    "cell_id": { "type": "string" },
                    "new_source": { "type": "string" },
                    "cell_type": { "type": "string", "enum": ["code", "markdown"] },
                    "edit_mode": { "type": "string", "enum": ["replace", "insert", "delete"] }
                },
                "required": ["notebook_path"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::NotebookEdit,
            search_hint: Some("jupyter notebook cells"),
            role_availability: BuiltinRoleAvailability {
                async_worker_allowed: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "Sleep",
            aliases: &[],
            description: "Wait for a specified duration without holding a shell process.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "duration_ms": { "type": "integer", "minimum": 0 }
                },
                "required": ["duration_ms"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::Orchestration,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::Sleep,
            search_hint: Some("scheduled wait"),
            role_availability: BuiltinRoleAvailability::default(),
        },
        BuiltinCapability {
            name: "SendUserMessage",
            aliases: &["Brief"],
            description: "Send a message to the user.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string" },
                    "attachments": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "status": {
                        "type": "string",
                        "enum": ["normal", "proactive"]
                    }
                },
                "required": ["message", "status"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::Brief,
            search_hint: Some("reply to user"),
            role_availability: BuiltinRoleAvailability {
                main_thread_only: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "Config",
            aliases: &[],
            description: "Get or set Claude Code settings.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "setting": { "type": "string" },
                    "value": {
                        "type": ["string", "boolean", "number"]
                    }
                },
                "required": ["setting"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::Config,
            search_hint: Some("runtime settings"),
            role_availability: BuiltinRoleAvailability {
                main_thread_only: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "EnterPlanMode",
            aliases: &[],
            description: "Enable a worktree-local planning mode override and remember the previous local setting for ExitPlanMode.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::EnterPlanMode,
            search_hint: Some("enter planning mode"),
            role_availability: BuiltinRoleAvailability {
                main_thread_only: true,
                plan_mode_only: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "ExitPlanMode",
            aliases: &[],
            description: "Restore or clear the worktree-local planning mode override created by EnterPlanMode.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::ExitPlanMode,
            search_hint: Some("leave planning mode"),
            role_availability: BuiltinRoleAvailability {
                main_thread_only: true,
                plan_mode_only: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "StructuredOutput",
            aliases: &[],
            description: "Return structured output in the requested format.",
            input_schema: json!({
                "type": "object",
                "additionalProperties": true
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::StructuredOutput,
            search_hint: Some("structured final answer"),
            role_availability: BuiltinRoleAvailability {
                main_thread_only: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "REPL",
            aliases: &[],
            description: "Execute code in a REPL-like subprocess.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "code": { "type": "string" },
                    "language": { "type": "string" },
                    "timeout_ms": { "type": "integer", "minimum": 1 }
                },
                "required": ["code", "language"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::Repl,
            search_hint: Some("code execution sandbox"),
            role_availability: BuiltinRoleAvailability::default(),
        },
        BuiltinCapability {
            name: "PowerShell",
            aliases: &[],
            description: "Execute a PowerShell command with optional timeout.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "timeout": { "type": "integer", "minimum": 1 },
                    "description": { "type": "string" },
                    "run_in_background": { "type": "boolean" }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::PowerShell,
            search_hint: Some("powershell commands"),
            role_availability: BuiltinRoleAvailability::default(),
        },
        BuiltinCapability {
            name: "AskUserQuestion",
            aliases: &[],
            description: "Ask the user a question and wait for their response.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "question": { "type": "string" },
                    "options": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                },
                "required": ["question"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::ControlPlane,
            visibility: CapabilityVisibility::DefaultVisible,
            handler_key: BuiltinHandlerKey::AskUserQuestion,
            search_hint: Some("clarify with user"),
            role_availability: BuiltinRoleAvailability {
                main_thread_only: true,
                non_interactive_blocked: true,
                ..BuiltinRoleAvailability::default()
            },
        },
        BuiltinCapability {
            name: "LSP",
            aliases: &[],
            description: "Query Language Server Protocol for code intelligence (symbols, references, diagnostics).",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["symbols", "references", "diagnostics", "definition", "hover"] },
                    "path": { "type": "string" },
                    "line": { "type": "integer", "minimum": 0 },
                    "character": { "type": "integer", "minimum": 0 },
                    "query": { "type": "string" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
            category: BuiltinCapabilityCategory::WorkerPrimitive,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::Lsp,
            search_hint: Some("language server intelligence"),
            role_availability: BuiltinRoleAvailability::default(),
        },
        BuiltinCapability {
            name: "RemoteTrigger",
            aliases: &[],
            description: "Trigger a remote action or webhook endpoint.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" },
                    "method": { "type": "string", "enum": ["GET", "POST", "PUT", "DELETE"] },
                    "headers": { "type": "object" },
                    "body": { "type": "string" }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            category: BuiltinCapabilityCategory::Orchestration,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::RemoteTrigger,
            search_hint: Some("trigger external workflow"),
            role_availability: BuiltinRoleAvailability::default(),
        },
        BuiltinCapability {
            name: "TestingPermission",
            aliases: &[],
            description: "Test-only tool for verifying permission enforcement behavior.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string" }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
            category: BuiltinCapabilityCategory::TestOnly,
            visibility: CapabilityVisibility::Deferred,
            handler_key: BuiltinHandlerKey::TestingPermission,
            search_hint: Some("permission test stub"),
            role_availability: BuiltinRoleAvailability::default(),
        },
    ]
}
