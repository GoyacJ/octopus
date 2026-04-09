use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolManifestEntry {
    pub name: String,
    pub source: ToolSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSource {
    Base,
    Conditional,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolRegistry {
    entries: Vec<ToolManifestEntry>,
}

impl ToolRegistry {
    #[must_use]
    pub fn new(entries: Vec<ToolManifestEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn entries(&self) -> &[ToolManifestEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
}

#[derive(Debug, Clone)]
pub struct GlobalToolRegistry {
    plugin_tools: Vec<PluginTool>,
    runtime_tools: Vec<RuntimeToolDefinition>,
    enforcer: Option<PermissionEnforcer>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
    pub required_permission: PermissionMode,
}

impl GlobalToolRegistry {
    #[must_use]
    pub fn builtin() -> Self {
        Self {
            plugin_tools: Vec::new(),
            runtime_tools: Vec::new(),
            enforcer: None,
        }
    }

    pub fn with_plugin_tools(plugin_tools: Vec<PluginTool>) -> Result<Self, String> {
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

        Ok(Self {
            plugin_tools,
            runtime_tools: Vec::new(),
            enforcer: None,
        })
    }

    pub fn with_runtime_tools(
        mut self,
        runtime_tools: Vec<RuntimeToolDefinition>,
    ) -> Result<Self, String> {
        let mut seen_names = mvp_tool_specs()
            .into_iter()
            .map(|spec| spec.name.to_string())
            .chain(
                self.plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .collect::<BTreeSet<_>>();

        for tool in &runtime_tools {
            if !seen_names.insert(tool.name.clone()) {
                return Err(format!(
                    "runtime tool `{}` conflicts with an existing tool name",
                    tool.name
                ));
            }
        }

        self.runtime_tools = runtime_tools;
        Ok(self)
    }

    #[must_use]
    pub fn with_enforcer(mut self, enforcer: PermissionEnforcer) -> Self {
        self.set_enforcer(enforcer);
        self
    }

    pub fn normalize_allowed_tools(
        &self,
        values: &[String],
    ) -> Result<Option<BTreeSet<String>>, String> {
        if values.is_empty() {
            return Ok(None);
        }

        let builtin_specs = mvp_tool_specs();
        let canonical_names = builtin_specs
            .iter()
            .map(|spec| spec.name.to_string())
            .chain(
                self.plugin_tools
                    .iter()
                    .map(|tool| tool.definition().name.clone()),
            )
            .chain(self.runtime_tools.iter().map(|tool| tool.name.clone()))
            .collect::<Vec<_>>();
        let mut name_map = canonical_names
            .iter()
            .map(|name| (normalize_tool_name(name), name.clone()))
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
                let normalized = normalize_tool_name(token);
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

    #[must_use]
    pub fn definitions(&self, allowed_tools: Option<&BTreeSet<String>>) -> Vec<ToolDefinition> {
        let builtin = mvp_tool_specs()
            .into_iter()
            .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
            .map(|spec| ToolDefinition {
                name: spec.name.to_string(),
                description: Some(spec.description.to_string()),
                input_schema: spec.input_schema,
            });
        let runtime = self
            .runtime_tools
            .iter()
            .filter(|tool| allowed_tools.is_none_or(|allowed| allowed.contains(tool.name.as_str())))
            .map(|tool| ToolDefinition {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
            });
        let plugin = self
            .plugin_tools
            .iter()
            .filter(|tool| {
                allowed_tools
                    .is_none_or(|allowed| allowed.contains(tool.definition().name.as_str()))
            })
            .map(|tool| ToolDefinition {
                name: tool.definition().name.clone(),
                description: tool.definition().description.clone(),
                input_schema: tool.definition().input_schema.clone(),
            });
        builtin.chain(runtime).chain(plugin).collect()
    }

    pub fn permission_specs(
        &self,
        allowed_tools: Option<&BTreeSet<String>>,
    ) -> Result<Vec<(String, PermissionMode)>, String> {
        let builtin = mvp_tool_specs()
            .into_iter()
            .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
            .map(|spec| (spec.name.to_string(), spec.required_permission));
        let runtime = self
            .runtime_tools
            .iter()
            .filter(|tool| allowed_tools.is_none_or(|allowed| allowed.contains(tool.name.as_str())))
            .map(|tool| (tool.name.clone(), tool.required_permission));
        let plugin = self
            .plugin_tools
            .iter()
            .filter(|tool| {
                allowed_tools
                    .is_none_or(|allowed| allowed.contains(tool.definition().name.as_str()))
            })
            .map(|tool| {
                permission_mode_from_plugin(tool.required_permission())
                    .map(|permission| (tool.definition().name.clone(), permission))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(builtin.chain(runtime).chain(plugin).collect())
    }

    #[must_use]
    pub fn has_runtime_tool(&self, name: &str) -> bool {
        self.runtime_tools.iter().any(|tool| tool.name == name)
    }

    #[must_use]
    pub fn search(
        &self,
        query: &str,
        max_results: usize,
        pending_mcp_servers: Option<Vec<String>>,
        mcp_degraded: Option<McpDegradedReport>,
    ) -> ToolSearchOutput {
        let query = query.trim().to_string();
        let normalized_query = normalize_tool_search_query(&query);
        let matches = search_tool_specs(&query, max_results.max(1), &self.searchable_tool_specs());

        ToolSearchOutput {
            matches,
            query,
            normalized_query,
            total_deferred_tools: self.searchable_tool_specs().len(),
            pending_mcp_servers,
            mcp_degraded,
        }
    }

    pub fn set_enforcer(&mut self, enforcer: PermissionEnforcer) {
        self.enforcer = Some(enforcer);
    }

    pub fn execute(&self, name: &str, input: &Value) -> Result<String, String> {
        if mvp_tool_specs().iter().any(|spec| spec.name == name) {
            return execute_tool_with_enforcer(self.enforcer.as_ref(), name, input);
        }
        self.plugin_tools
            .iter()
            .find(|tool| tool.definition().name == name)
            .ok_or_else(|| format!("unsupported tool: {name}"))?
            .execute(input)
            .map_err(|error| error.to_string())
    }

    fn searchable_tool_specs(&self) -> Vec<SearchableToolSpec> {
        let builtin = deferred_tool_specs()
            .into_iter()
            .map(|spec| SearchableToolSpec {
                name: spec.name.to_string(),
                description: spec.description.to_string(),
            });
        let runtime = self.runtime_tools.iter().map(|tool| SearchableToolSpec {
            name: tool.name.clone(),
            description: tool.description.clone().unwrap_or_default(),
        });
        let plugin = self.plugin_tools.iter().map(|tool| SearchableToolSpec {
            name: tool.definition().name.clone(),
            description: tool.definition().description.clone().unwrap_or_default(),
        });
        builtin.chain(runtime).chain(plugin).collect()
    }
}

fn normalize_tool_name(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

pub(crate) fn permission_mode_from_plugin(value: &str) -> Result<PermissionMode, String> {
    match value {
        "read-only" => Ok(PermissionMode::ReadOnly),
        "workspace-write" => Ok(PermissionMode::WorkspaceWrite),
        "danger-full-access" => Ok(PermissionMode::DangerFullAccess),
        other => Err(format!("unsupported plugin permission: {other}")),
    }
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn mvp_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "bash",
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
        },
        ToolSpec {
            name: "read_file",
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
        },
        ToolSpec {
            name: "write_file",
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
        },
        ToolSpec {
            name: "edit_file",
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
        },
        ToolSpec {
            name: "glob_search",
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
        },
        ToolSpec {
            name: "grep_search",
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
        },
        ToolSpec {
            name: "WebFetch",
            description:
                "Fetch a URL, convert it into readable text, and answer a prompt about it.",
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
        },
        ToolSpec {
            name: "WebSearch",
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
        },
        ToolSpec {
            name: "TodoWrite",
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
        },
        ToolSpec {
            name: "Skill",
            description: "Load a local skill definition and its instructions.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "skill": { "type": "string" },
                    "args": { "type": "string" }
                },
                "required": ["skill"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "Agent",
            description: "Launch a specialized agent task and persist its handoff metadata.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "description": { "type": "string" },
                    "prompt": { "type": "string" },
                    "subagent_type": { "type": "string" },
                    "name": { "type": "string" },
                    "model": { "type": "string" }
                },
                "required": ["description", "prompt"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "ToolSearch",
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
        },
        ToolSpec {
            name: "NotebookEdit",
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
        },
        ToolSpec {
            name: "Sleep",
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
        },
        ToolSpec {
            name: "SendUserMessage",
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
        },
        ToolSpec {
            name: "Config",
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
        },
        ToolSpec {
            name: "EnterPlanMode",
            description: "Enable a worktree-local planning mode override and remember the previous local setting for ExitPlanMode.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
        },
        ToolSpec {
            name: "ExitPlanMode",
            description: "Restore or clear the worktree-local planning mode override created by EnterPlanMode.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::WorkspaceWrite,
        },
        ToolSpec {
            name: "StructuredOutput",
            description: "Return structured output in the requested format.",
            input_schema: json!({
                "type": "object",
                "additionalProperties": true
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "REPL",
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
        },
        ToolSpec {
            name: "PowerShell",
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
        },
        ToolSpec {
            name: "AskUserQuestion",
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
        },
        ToolSpec {
            name: "TaskCreate",
            description: "Create a background task that runs in a separate subprocess.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "prompt": { "type": "string" },
                    "description": { "type": "string" }
                },
                "required": ["prompt"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "RunTaskPacket",
            description: "Create a background task from a structured task packet.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "objective": { "type": "string" },
                    "scope": { "type": "string" },
                    "repo": { "type": "string" },
                    "branch_policy": { "type": "string" },
                    "acceptance_tests": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "commit_policy": { "type": "string" },
                    "reporting_contract": { "type": "string" },
                    "escalation_policy": { "type": "string" }
                },
                "required": [
                    "objective",
                    "scope",
                    "repo",
                    "branch_policy",
                    "acceptance_tests",
                    "commit_policy",
                    "reporting_contract",
                    "escalation_policy"
                ],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "TaskGet",
            description: "Get the status and details of a background task by ID.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "TaskList",
            description: "List all background tasks and their current status.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "TaskStop",
            description: "Stop a running background task by ID.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "TaskUpdate",
            description: "Send a message or update to a running background task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" },
                    "message": { "type": "string" }
                },
                "required": ["task_id", "message"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "TaskOutput",
            description: "Retrieve the output produced by a background task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string" }
                },
                "required": ["task_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "WorkerCreate",
            description: "Create a coding worker boot session with trust-gate and prompt-delivery guards.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cwd": { "type": "string" },
                    "trusted_roots": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "auto_recover_prompt_misdelivery": { "type": "boolean" }
                },
                "required": ["cwd"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "WorkerGet",
            description: "Fetch the current worker boot state, last error, and event history.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" }
                },
                "required": ["worker_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "WorkerObserve",
            description: "Feed a terminal snapshot into worker boot detection to resolve trust gates, ready handshakes, and prompt misdelivery.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" },
                    "screen_text": { "type": "string" }
                },
                "required": ["worker_id", "screen_text"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "WorkerResolveTrust",
            description: "Resolve a detected trust prompt so worker boot can continue.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" }
                },
                "required": ["worker_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "WorkerAwaitReady",
            description: "Return the current ready-handshake verdict for a coding worker.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" }
                },
                "required": ["worker_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "WorkerSendPrompt",
            description: "Send a task prompt only after the worker reaches ready_for_prompt; can replay a recovered prompt.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" },
                    "prompt": { "type": "string" }
                },
                "required": ["worker_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "WorkerRestart",
            description: "Restart worker boot state after a failed or stale startup.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" }
                },
                "required": ["worker_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "WorkerTerminate",
            description: "Terminate a worker and mark the lane finished from the control plane.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" }
                },
                "required": ["worker_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "WorkerObserveCompletion",
            description: "Report session completion to the worker, classifying finish_reason into finished or failed provider-degraded terminal states.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "worker_id": { "type": "string" },
                    "finish_reason": { "type": "string" },
                    "tokens_output": { "type": "integer", "minimum": 0 }
                },
                "required": ["worker_id", "finish_reason", "tokens_output"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "TeamCreate",
            description: "Create a team of sub-agents for parallel task execution.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "tasks": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "prompt": { "type": "string" },
                                "description": { "type": "string" }
                            },
                            "required": ["prompt"]
                        }
                    }
                },
                "required": ["name", "tasks"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "TeamDelete",
            description: "Delete a team and stop all its running tasks.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "team_id": { "type": "string" }
                },
                "required": ["team_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "CronCreate",
            description: "Create a scheduled recurring task.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "schedule": { "type": "string" },
                    "prompt": { "type": "string" },
                    "description": { "type": "string" }
                },
                "required": ["schedule", "prompt"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "CronDelete",
            description: "Delete a scheduled recurring task by ID.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "cron_id": { "type": "string" }
                },
                "required": ["cron_id"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "CronList",
            description: "List all scheduled recurring tasks.",
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "LSP",
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
        },
        ToolSpec {
            name: "ListMcpResources",
            description: "List available resources from connected MCP servers.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" }
                },
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "ReadMcpResource",
            description: "Read a specific resource from an MCP server by URI.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" },
                    "uri": { "type": "string" }
                },
                "required": ["uri"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::ReadOnly,
        },
        ToolSpec {
            name: "McpAuth",
            description: "Authenticate with an MCP server that requires OAuth or credentials.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" }
                },
                "required": ["server"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "RemoteTrigger",
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
        },
        ToolSpec {
            name: "MCP",
            description: "Execute a tool provided by a connected MCP server.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "server": { "type": "string" },
                    "tool": { "type": "string" },
                    "arguments": { "type": "object" }
                },
                "required": ["server", "tool"],
                "additionalProperties": false
            }),
            required_permission: PermissionMode::DangerFullAccess,
        },
        ToolSpec {
            name: "TestingPermission",
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
        },
    ]
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolSearchOutput {
    matches: Vec<String>,
    query: String,
    normalized_query: String,
    #[serde(rename = "total_deferred_tools")]
    total_deferred_tools: usize,
    #[serde(rename = "pending_mcp_servers")]
    pending_mcp_servers: Option<Vec<String>>,
    #[serde(rename = "mcp_degraded", skip_serializing_if = "Option::is_none")]
    mcp_degraded: Option<McpDegradedReport>,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchableToolSpec {
    name: String,
    description: String,
}

pub(crate) fn tool_specs_for_allowed_tools(
    allowed_tools: Option<&BTreeSet<String>>,
) -> Vec<ToolSpec> {
    mvp_tool_specs()
        .into_iter()
        .filter(|spec| allowed_tools.is_none_or(|allowed| allowed.contains(spec.name)))
        .collect()
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn execute_tool_search(input: ToolSearchInput) -> ToolSearchOutput {
    GlobalToolRegistry::builtin().search(&input.query, input.max_results.unwrap_or(5), None, None)
}

pub(crate) fn deferred_tool_specs() -> Vec<ToolSpec> {
    mvp_tool_specs()
        .into_iter()
        .filter(|spec| {
            !matches!(
                spec.name,
                "bash" | "read_file" | "write_file" | "edit_file" | "glob_search" | "grep_search"
            )
        })
        .collect()
}

pub(crate) fn search_tool_specs(
    query: &str,
    max_results: usize,
    specs: &[SearchableToolSpec],
) -> Vec<String> {
    let lowered = query.to_lowercase();
    if let Some(selection) = lowered.strip_prefix("select:") {
        return selection
            .split(',')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .filter_map(|wanted| {
                let wanted = canonical_tool_token(wanted);
                specs
                    .iter()
                    .find(|spec| canonical_tool_token(&spec.name) == wanted)
                    .map(|spec| spec.name.clone())
            })
            .take(max_results)
            .collect();
    }

    let mut required = Vec::new();
    let mut optional = Vec::new();
    for term in lowered.split_whitespace() {
        if let Some(rest) = term.strip_prefix('+') {
            if !rest.is_empty() {
                required.push(rest);
            }
        } else {
            optional.push(term);
        }
    }
    let terms = if required.is_empty() {
        optional.clone()
    } else {
        required.iter().chain(optional.iter()).copied().collect()
    };

    let mut scored = specs
        .iter()
        .filter_map(|spec| {
            let name = spec.name.to_lowercase();
            let canonical_name = canonical_tool_token(&spec.name);
            let normalized_description = normalize_tool_search_query(&spec.description);
            let haystack = format!(
                "{name} {} {canonical_name}",
                spec.description.to_lowercase()
            );
            let normalized_haystack = format!("{canonical_name} {normalized_description}");
            if required.iter().any(|term| !haystack.contains(term)) {
                return None;
            }

            let mut score = 0_i32;
            for term in &terms {
                let canonical_term = canonical_tool_token(term);
                if haystack.contains(term) {
                    score += 2;
                }
                if name == *term {
                    score += 8;
                }
                if name.contains(term) {
                    score += 4;
                }
                if canonical_name == canonical_term {
                    score += 12;
                }
                if normalized_haystack.contains(&canonical_term) {
                    score += 3;
                }
            }

            if score == 0 && !lowered.is_empty() {
                return None;
            }
            Some((score, spec.name.clone()))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .map(|(_, name)| name)
        .take(max_results)
        .collect()
}

pub(crate) fn normalize_tool_search_query(query: &str) -> String {
    query
        .trim()
        .split(|ch: char| ch.is_whitespace() || ch == ',')
        .filter(|term| !term.is_empty())
        .map(canonical_tool_token)
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn canonical_tool_token(value: &str) -> String {
    let mut canonical = value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if let Some(stripped) = canonical.strip_suffix("tool") {
        canonical = stripped.to_string();
    }
    canonical
}
