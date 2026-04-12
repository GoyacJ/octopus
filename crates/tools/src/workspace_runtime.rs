use super::*;

/// Global task registry shared across tool invocations within a session.
fn global_lsp_registry() -> &'static LspRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<LspRegistry> = OnceLock::new();
    REGISTRY.get_or_init(LspRegistry::new)
}

fn global_mcp_registry() -> &'static McpToolRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<McpToolRegistry> = OnceLock::new();
    REGISTRY.get_or_init(McpToolRegistry::new)
}

fn global_team_registry() -> &'static TeamRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<TeamRegistry> = OnceLock::new();
    REGISTRY.get_or_init(TeamRegistry::new)
}

fn global_cron_registry() -> &'static CronRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<CronRegistry> = OnceLock::new();
    REGISTRY.get_or_init(CronRegistry::new)
}

fn global_task_registry() -> &'static TaskRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<TaskRegistry> = OnceLock::new();
    REGISTRY.get_or_init(TaskRegistry::new)
}

fn global_worker_registry() -> &'static WorkerRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<WorkerRegistry> = OnceLock::new();
    REGISTRY.get_or_init(WorkerRegistry::new)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_task_create(input: TaskCreateInput) -> Result<String, String> {
    let registry = global_task_registry();
    let task = registry.create(&input.prompt, input.description.as_deref());
    to_pretty_json(json!({
        "task_id": task.task_id,
        "status": task.status,
        "prompt": task.prompt,
        "description": task.description,
        "task_packet": task.task_packet,
        "created_at": task.created_at
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_task_packet(input: TaskPacket) -> Result<String, String> {
    let registry = global_task_registry();
    let task = registry
        .create_from_packet(input)
        .map_err(|error| error.to_string())?;

    to_pretty_json(json!({
        "task_id": task.task_id,
        "status": task.status,
        "prompt": task.prompt,
        "description": task.description,
        "task_packet": task.task_packet,
        "created_at": task.created_at
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_task_get(input: TaskIdInput) -> Result<String, String> {
    let registry = global_task_registry();
    match registry.get(&input.task_id) {
        Some(task) => to_pretty_json(json!({
            "task_id": task.task_id,
            "status": task.status,
            "prompt": task.prompt,
            "description": task.description,
            "task_packet": task.task_packet,
            "created_at": task.created_at,
            "updated_at": task.updated_at,
            "messages": task.messages,
            "team_id": task.team_id
        })),
        None => Err(format!("task not found: {}", input.task_id)),
    }
}

pub(crate) fn run_task_list(_input: Value) -> Result<String, String> {
    let registry = global_task_registry();
    let tasks: Vec<_> = registry
        .list(None)
        .into_iter()
        .map(|t| {
            json!({
                "task_id": t.task_id,
                "status": t.status,
                "prompt": t.prompt,
                "description": t.description,
                "task_packet": t.task_packet,
                "created_at": t.created_at,
                "updated_at": t.updated_at,
                "team_id": t.team_id
            })
        })
        .collect();
    to_pretty_json(json!({
        "tasks": tasks,
        "count": tasks.len()
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_task_stop(input: TaskIdInput) -> Result<String, String> {
    let registry = global_task_registry();
    match registry.stop(&input.task_id) {
        Ok(task) => to_pretty_json(json!({
            "task_id": task.task_id,
            "status": task.status,
            "message": "Task stopped"
        })),
        Err(e) => Err(e),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_task_update(input: TaskUpdateInput) -> Result<String, String> {
    let registry = global_task_registry();
    match registry.update(&input.task_id, &input.message) {
        Ok(task) => to_pretty_json(json!({
            "task_id": task.task_id,
            "status": task.status,
            "message_count": task.messages.len(),
            "last_message": input.message
        })),
        Err(e) => Err(e),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_task_output(input: TaskIdInput) -> Result<String, String> {
    let registry = global_task_registry();
    match registry.output(&input.task_id) {
        Ok(output) => to_pretty_json(json!({
            "task_id": input.task_id,
            "output": output,
            "has_output": !output.is_empty()
        })),
        Err(e) => Err(e),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_create(input: WorkerCreateInput) -> Result<String, String> {
    let config_roots = ConfigLoader::default_for(&input.cwd)
        .load()
        .ok()
        .map(|config| config.trusted_roots().to_vec())
        .unwrap_or_default();
    let trusted_roots = config_roots
        .into_iter()
        .chain(input.trusted_roots.iter().cloned())
        .collect::<Vec<_>>();
    let worker = global_worker_registry().create(
        &input.cwd,
        &trusted_roots,
        input.auto_recover_prompt_misdelivery,
    );
    to_pretty_json(worker)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_get(input: WorkerIdInput) -> Result<String, String> {
    global_worker_registry().get(&input.worker_id).map_or_else(
        || Err(format!("worker not found: {}", input.worker_id)),
        to_pretty_json,
    )
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_observe(input: WorkerObserveInput) -> Result<String, String> {
    let worker = global_worker_registry().observe(&input.worker_id, &input.screen_text)?;
    to_pretty_json(worker)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_resolve_trust(input: WorkerIdInput) -> Result<String, String> {
    let worker = global_worker_registry().resolve_trust(&input.worker_id)?;
    to_pretty_json(worker)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_await_ready(input: WorkerIdInput) -> Result<String, String> {
    let snapshot: WorkerReadySnapshot = global_worker_registry().await_ready(&input.worker_id)?;
    to_pretty_json(snapshot)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_send_prompt(input: WorkerSendPromptInput) -> Result<String, String> {
    let worker = global_worker_registry().send_prompt(&input.worker_id, input.prompt.as_deref())?;
    to_pretty_json(worker)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_restart(input: WorkerIdInput) -> Result<String, String> {
    let worker = global_worker_registry().restart(&input.worker_id)?;
    to_pretty_json(worker)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_terminate(input: WorkerIdInput) -> Result<String, String> {
    let worker = global_worker_registry().terminate(&input.worker_id)?;
    to_pretty_json(worker)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_worker_observe_completion(
    input: WorkerObserveCompletionInput,
) -> Result<String, String> {
    let worker = global_worker_registry().observe_completion(
        &input.worker_id,
        &input.finish_reason,
        input.tokens_output,
    )?;
    to_pretty_json(worker)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_team_create(input: TeamCreateInput) -> Result<String, String> {
    let task_ids: Vec<String> = input
        .tasks
        .iter()
        .filter_map(|t| t.get("task_id").and_then(|v| v.as_str()).map(str::to_owned))
        .collect();
    let team = global_team_registry().create(&input.name, task_ids);
    // Register team assignment on each task
    for task_id in &team.task_ids {
        let _ = global_task_registry().assign_team(task_id, &team.team_id);
    }
    to_pretty_json(json!({
        "team_id": team.team_id,
        "name": team.name,
        "task_count": team.task_ids.len(),
        "task_ids": team.task_ids,
        "status": team.status,
        "created_at": team.created_at
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_team_delete(input: TeamDeleteInput) -> Result<String, String> {
    match global_team_registry().delete(&input.team_id) {
        Ok(team) => to_pretty_json(json!({
            "team_id": team.team_id,
            "name": team.name,
            "status": team.status,
            "message": "Team deleted"
        })),
        Err(e) => Err(e),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_cron_create(input: CronCreateInput) -> Result<String, String> {
    let entry =
        global_cron_registry().create(&input.schedule, &input.prompt, input.description.as_deref());
    to_pretty_json(json!({
        "cron_id": entry.cron_id,
        "schedule": entry.schedule,
        "prompt": entry.prompt,
        "description": entry.description,
        "enabled": entry.enabled,
        "created_at": entry.created_at
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_cron_delete(input: CronDeleteInput) -> Result<String, String> {
    match global_cron_registry().delete(&input.cron_id) {
        Ok(entry) => to_pretty_json(json!({
            "cron_id": entry.cron_id,
            "schedule": entry.schedule,
            "status": "deleted",
            "message": "Cron entry removed"
        })),
        Err(e) => Err(e),
    }
}

pub(crate) fn run_cron_list(_input: Value) -> Result<String, String> {
    let entries: Vec<_> = global_cron_registry()
        .list(false)
        .into_iter()
        .map(|e| {
            json!({
                "cron_id": e.cron_id,
                "schedule": e.schedule,
                "prompt": e.prompt,
                "description": e.description,
                "enabled": e.enabled,
                "run_count": e.run_count,
                "last_run_at": e.last_run_at,
                "created_at": e.created_at
            })
        })
        .collect();
    to_pretty_json(json!({
        "crons": entries,
        "count": entries.len()
    }))
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_lsp(input: LspInput) -> Result<String, String> {
    let registry = global_lsp_registry();
    let action = &input.action;
    let path = input.path.as_deref();
    let line = input.line;
    let character = input.character;
    let query = input.query.as_deref();

    match registry.dispatch(action, path, line, character, query) {
        Ok(result) => to_pretty_json(result),
        Err(e) => to_pretty_json(json!({
            "action": action,
            "error": e,
            "status": "error"
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_list_mcp_resources(input: McpResourceInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    let server = input.server.as_deref().unwrap_or("default");
    match registry.list_resources(server) {
        Ok(resources) => {
            let items: Vec<_> = resources
                .iter()
                .map(|r| {
                    json!({
                        "uri": r.uri,
                        "name": r.name,
                        "description": r.description,
                        "mime_type": r.mime_type,
                    })
                })
                .collect();
            to_pretty_json(json!({
                "server": server,
                "resources": items,
                "count": items.len()
            }))
        }
        Err(e) => to_pretty_json(json!({
            "server": server,
            "resources": [],
            "error": e
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_read_mcp_resource(input: McpResourceInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    let uri = input.uri.as_deref().unwrap_or("");
    let server = input.server.as_deref().unwrap_or("default");
    match registry.read_resource(server, uri) {
        Ok(resource) => to_pretty_json(json!({
            "server": server,
            "uri": resource.uri,
            "name": resource.name,
            "description": resource.description,
            "mime_type": resource.mime_type
        })),
        Err(e) => to_pretty_json(json!({
            "server": server,
            "uri": uri,
            "error": e
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_mcp_auth(input: McpAuthInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    match registry.get_server(&input.server) {
        Some(state) => to_pretty_json(json!({
            "server": input.server,
            "status": state.status,
            "server_info": state.server_info,
            "tool_count": state.tools.len(),
            "resource_count": state.resources.len()
        })),
        None => to_pretty_json(json!({
            "server": input.server,
            "status": "disconnected",
            "message": "Server not registered. Use MCP tool to connect first."
        })),
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_mcp_tool(input: McpToolInput) -> Result<String, String> {
    let registry = global_mcp_registry();
    let args = input.arguments.unwrap_or(serde_json::json!({}));
    match registry.call_tool(&input.server, &input.tool, &args) {
        Ok(result) => to_pretty_json(json!({
            "server": input.server,
            "tool": input.tool,
            "result": result,
            "status": "success"
        })),
        Err(e) => to_pretty_json(json!({
            "server": input.server,
            "tool": input.tool,
            "error": e,
            "status": "error"
        })),
    }
}

pub(crate) fn run_agent(input: AgentInput) -> Result<String, String> {
    to_pretty_json(execute_agent(input)?)
}

#[derive(Debug, Deserialize)]
pub(crate) struct AgentInput {
    pub(crate) description: String,
    pub(crate) prompt: String,
    pub(crate) subagent_type: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) model: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskCreateInput {
    prompt: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskIdInput {
    task_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TaskUpdateInput {
    task_id: String,
    message: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkerCreateInput {
    cwd: String,
    #[serde(default)]
    trusted_roots: Vec<String>,
    #[serde(default = "default_auto_recover_prompt_misdelivery")]
    auto_recover_prompt_misdelivery: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkerIdInput {
    worker_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkerObserveInput {
    worker_id: String,
    screen_text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkerSendPromptInput {
    worker_id: String,
    #[serde(default)]
    prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WorkerObserveCompletionInput {
    worker_id: String,
    finish_reason: String,
    tokens_output: u64,
}

const fn default_auto_recover_prompt_misdelivery() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamCreateInput {
    name: String,
    tasks: Vec<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TeamDeleteInput {
    team_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CronCreateInput {
    schedule: String,
    prompt: String,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CronDeleteInput {
    cron_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LspInput {
    action: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    line: Option<u32>,
    #[serde(default)]
    character: Option<u32>,
    #[serde(default)]
    query: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpResourceInput {
    #[serde(default)]
    server: Option<String>,
    #[serde(default)]
    uri: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpAuthInput {
    server: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct McpToolInput {
    server: String,
    tool: String,
    #[serde(default)]
    arguments: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentOutput {
    #[serde(rename = "agentId")]
    pub(crate) agent_id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    #[serde(rename = "subagentType")]
    pub(crate) subagent_type: Option<String>,
    pub(crate) model: Option<String>,
    pub(crate) status: String,
    #[serde(rename = "outputFile")]
    pub(crate) output_file: String,
    #[serde(rename = "manifestFile")]
    pub(crate) manifest_file: String,
    #[serde(rename = "createdAt")]
    pub(crate) created_at: String,
    #[serde(rename = "startedAt", skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(rename = "completedAt", skip_serializing_if = "Option::is_none")]
    pub(crate) completed_at: Option<String>,
    #[serde(rename = "laneEvents", default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) lane_events: Vec<LaneEvent>,
    #[serde(rename = "currentBlocker", skip_serializing_if = "Option::is_none")]
    pub(crate) current_blocker: Option<LaneEventBlocker>,
    #[serde(rename = "derivedState")]
    pub(crate) derived_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentJob {
    pub(crate) manifest: AgentOutput,
    pub(crate) prompt: String,
    system_prompt: Vec<String>,
    pub(crate) capability_profile: CapabilityProfile,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentSpawnFailure {
    pub(crate) manifest: Option<AgentOutput>,
    pub(crate) error: String,
}

impl AgentSpawnFailure {
    fn new(error: impl Into<String>) -> Self {
        Self {
            manifest: None,
            error: error.into(),
        }
    }
}

const DEFAULT_AGENT_MODEL: &str = "claude-opus-4-6";

const DEFAULT_AGENT_SYSTEM_DATE: &str = "2026-03-31";

const DEFAULT_AGENT_MAX_ITERATIONS: usize = 32;

fn execute_agent(input: AgentInput) -> Result<AgentOutput, String> {
    execute_agent_with_spawn(input, spawn_agent_job)
}

pub(crate) fn execute_agent_with_spawn<F>(
    input: AgentInput,
    spawn_fn: F,
) -> Result<AgentOutput, String>
where
    F: FnOnce(AgentJob) -> Result<(), String>,
{
    execute_agent_with_spawn_detailed(input, spawn_fn).map_err(|failure| failure.error)
}

pub(crate) fn execute_agent_with_spawn_detailed<F>(
    input: AgentInput,
    spawn_fn: F,
) -> Result<AgentOutput, AgentSpawnFailure>
where
    F: FnOnce(AgentJob) -> Result<(), String>,
{
    if input.description.trim().is_empty() {
        return Err(AgentSpawnFailure::new("description must not be empty"));
    }
    if input.prompt.trim().is_empty() {
        return Err(AgentSpawnFailure::new("prompt must not be empty"));
    }

    let agent_id = make_agent_id();
    let output_dir = agent_store_dir().map_err(AgentSpawnFailure::new)?;
    std::fs::create_dir_all(&output_dir)
        .map_err(|error| AgentSpawnFailure::new(error.to_string()))?;
    let output_file = output_dir.join(format!("{agent_id}.md"));
    let manifest_file = output_dir.join(format!("{agent_id}.json"));
    let normalized_subagent_type = normalize_subagent_type(input.subagent_type.as_deref());
    let model = resolve_agent_model(input.model.as_deref());
    let agent_name = input
        .name
        .as_deref()
        .map(slugify_agent_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| slugify_agent_name(&input.description));
    let created_at = iso8601_now();
    let system_prompt =
        build_agent_system_prompt(&normalized_subagent_type).map_err(AgentSpawnFailure::new)?;
    let capability_profile = capability_profile_for_subagent(&normalized_subagent_type);

    let output_contents = format!(
        "# Agent Task

- id: {}
- name: {}
- description: {}
- subagent_type: {}
- created_at: {}

## Prompt

{}
",
        agent_id, agent_name, input.description, normalized_subagent_type, created_at, input.prompt
    );
    std::fs::write(&output_file, output_contents)
        .map_err(|error| AgentSpawnFailure::new(error.to_string()))?;

    let manifest = AgentOutput {
        agent_id,
        name: agent_name,
        description: input.description,
        subagent_type: Some(normalized_subagent_type),
        model: Some(model),
        status: String::from("running"),
        output_file: output_file.display().to_string(),
        manifest_file: manifest_file.display().to_string(),
        created_at: created_at.clone(),
        started_at: Some(created_at),
        completed_at: None,
        lane_events: vec![LaneEvent::started(iso8601_now())],
        current_blocker: None,
        derived_state: String::from("working"),
        error: None,
    };
    write_agent_manifest(&manifest).map_err(AgentSpawnFailure::new)?;

    let manifest_for_spawn = manifest.clone();
    let job = AgentJob {
        manifest: manifest_for_spawn,
        prompt: input.prompt,
        system_prompt,
        capability_profile,
    };
    if let Err(error) = spawn_fn(job) {
        let error = format!("failed to spawn sub-agent: {error}");
        persist_agent_terminal_state(&manifest, "failed", None, Some(error.clone())).map_err(
            |persist_error| AgentSpawnFailure {
                manifest: Some(manifest.clone()),
                error: persist_error,
            },
        )?;
        return Err(AgentSpawnFailure {
            manifest: Some(manifest),
            error,
        });
    }

    Ok(manifest)
}

pub(crate) fn spawn_agent_job(job: AgentJob) -> Result<(), String> {
    let thread_name = format!("clawd-agent-{}", job.manifest.agent_id);
    std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run_agent_job(&job)));
            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    let _ =
                        persist_agent_terminal_state(&job.manifest, "failed", None, Some(error));
                }
                Err(_) => {
                    let _ = persist_agent_terminal_state(
                        &job.manifest,
                        "failed",
                        None,
                        Some(String::from("sub-agent thread panicked")),
                    );
                }
            }
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn run_agent_job(job: &AgentJob) -> Result<(), String> {
    let mut runtime = build_agent_runtime(job)?.with_max_iterations(DEFAULT_AGENT_MAX_ITERATIONS);
    let summary = runtime
        .run_turn(job.prompt.clone(), None)
        .map_err(|error| error.to_string())?;
    let final_text = final_assistant_text(&summary);
    persist_agent_terminal_state(&job.manifest, "completed", Some(final_text.as_str()), None)
}

fn build_agent_runtime(
    job: &AgentJob,
) -> Result<ConversationRuntime<ProviderRuntimeClient, SubagentToolExecutor>, String> {
    let model = job
        .manifest
        .model
        .clone()
        .unwrap_or_else(|| DEFAULT_AGENT_MODEL.to_string());
    let capability_profile = job.capability_profile.clone();
    let capability_provider = CapabilityProvider::builtin();
    let capability_state =
        std::sync::Arc::new(std::sync::Mutex::new(SessionCapabilityState::default()));
    let api_client = ProviderRuntimeClient::from_capability_provider(
        model,
        capability_provider.clone(),
        capability_profile.clone(),
        capability_state.clone(),
    )?;
    let permission_policy = agent_permission_policy();
    let tool_executor = SubagentToolExecutor::from_capability_provider(
        capability_profile,
        capability_provider,
        capability_state,
    )
    .with_enforcer(PermissionEnforcer::new(permission_policy.clone()));
    Ok(ConversationRuntime::new(
        Session::new(),
        api_client,
        tool_executor,
        permission_policy,
        job.system_prompt.clone(),
    ))
}

fn build_agent_system_prompt(subagent_type: &str) -> Result<Vec<String>, String> {
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    let mut prompt = load_system_prompt(
        cwd,
        DEFAULT_AGENT_SYSTEM_DATE.to_string(),
        std::env::consts::OS,
        "unknown",
    )
    .map_err(|error| error.to_string())?;
    prompt.push(format!(
        "You are a background sub-agent of type `{subagent_type}`. Work only on the delegated task, use only the tools available to you, do not ask the user questions, and finish with a concise result."
    ));
    Ok(prompt)
}

fn resolve_agent_model(model: Option<&str>) -> String {
    model
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .unwrap_or(DEFAULT_AGENT_MODEL)
        .to_string()
}

fn capability_profile_for_subagent(subagent_type: &str) -> CapabilityProfile {
    let tools = match subagent_type {
        "Explore" => vec![
            "read_file",
            "glob_search",
            "grep_search",
            "WebFetch",
            "WebSearch",
            "ToolSearch",
            "SkillDiscovery",
            "SkillTool",
            "StructuredOutput",
        ],
        "Plan" => vec![
            "read_file",
            "glob_search",
            "grep_search",
            "WebFetch",
            "WebSearch",
            "ToolSearch",
            "SkillDiscovery",
            "SkillTool",
            "TodoWrite",
            "StructuredOutput",
            "SendUserMessage",
        ],
        "Verification" => vec![
            "bash",
            "read_file",
            "glob_search",
            "grep_search",
            "WebFetch",
            "WebSearch",
            "ToolSearch",
            "SkillDiscovery",
            "SkillTool",
            "TodoWrite",
            "StructuredOutput",
            "SendUserMessage",
            "PowerShell",
        ],
        "claw-guide" => vec![
            "read_file",
            "glob_search",
            "grep_search",
            "WebFetch",
            "WebSearch",
            "ToolSearch",
            "SkillDiscovery",
            "SkillTool",
            "StructuredOutput",
            "SendUserMessage",
        ],
        "statusline-setup" => vec![
            "bash",
            "read_file",
            "write_file",
            "edit_file",
            "glob_search",
            "grep_search",
            "ToolSearch",
        ],
        _ => vec![
            "bash",
            "read_file",
            "write_file",
            "edit_file",
            "glob_search",
            "grep_search",
            "WebFetch",
            "WebSearch",
            "TodoWrite",
            "SkillDiscovery",
            "SkillTool",
            "ToolSearch",
            "NotebookEdit",
            "Sleep",
            "SendUserMessage",
            "Config",
            "StructuredOutput",
            "REPL",
            "PowerShell",
        ],
    };
    CapabilityProfile::from_tools(tools.into_iter().map(str::to_string).collect())
}

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn allowed_tools_for_subagent(subagent_type: &str) -> BTreeSet<String> {
    capability_profile_for_subagent(subagent_type)
        .allowed_tools()
        .clone()
}

pub(crate) fn agent_permission_policy() -> PermissionPolicy {
    mvp_tool_specs().into_iter().fold(
        PermissionPolicy::new(PermissionMode::DangerFullAccess),
        |policy, spec| policy.with_tool_requirement(spec.name, spec.required_permission),
    )
}

fn write_agent_manifest(manifest: &AgentOutput) -> Result<(), String> {
    let mut normalized = manifest.clone();
    normalized.lane_events = dedupe_superseded_commit_events(&normalized.lane_events);
    std::fs::write(
        &normalized.manifest_file,
        serde_json::to_string_pretty(&normalized).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn persist_agent_terminal_state(
    manifest: &AgentOutput,
    status: &str,
    result: Option<&str>,
    error: Option<String>,
) -> Result<(), String> {
    let blocker = error.as_deref().map(classify_lane_blocker);
    append_agent_output(
        &manifest.output_file,
        &format_agent_terminal_output(status, result, blocker.as_ref(), error.as_deref()),
    )?;
    let mut next_manifest = manifest.clone();
    next_manifest.status = status.to_string();
    next_manifest.completed_at = Some(iso8601_now());
    next_manifest.current_blocker.clone_from(&blocker);
    next_manifest.derived_state =
        derive_agent_state(status, result, error.as_deref(), blocker.as_ref()).to_string();
    next_manifest.error = error;
    if let Some(blocker) = blocker {
        next_manifest
            .lane_events
            .push(LaneEvent::blocked(iso8601_now(), &blocker));
        next_manifest
            .lane_events
            .push(LaneEvent::failed(iso8601_now(), &blocker));
    } else {
        next_manifest.current_blocker = None;
        let compressed_detail = result
            .filter(|value| !value.trim().is_empty())
            .map(|value| compress_summary_text(value.trim()));
        next_manifest
            .lane_events
            .push(LaneEvent::finished(iso8601_now(), compressed_detail));
        if let Some(provenance) = maybe_commit_provenance(result) {
            next_manifest.lane_events.push(LaneEvent::commit_created(
                iso8601_now(),
                Some(format!("commit {}", provenance.commit)),
                provenance,
            ));
        }
    }
    write_agent_manifest(&next_manifest)
}

pub(crate) fn derive_agent_state(
    status: &str,
    result: Option<&str>,
    error: Option<&str>,
    blocker: Option<&LaneEventBlocker>,
) -> &'static str {
    let normalized_status = status.trim().to_ascii_lowercase();
    let normalized_error = error.unwrap_or_default().to_ascii_lowercase();

    if normalized_status == "running" {
        return "working";
    }
    if normalized_status == "completed" {
        return if result.is_some_and(|value| !value.trim().is_empty()) {
            "finished_cleanable"
        } else {
            "finished_pending_report"
        };
    }
    if normalized_error.contains("background") {
        return "blocked_background_job";
    }
    if normalized_error.contains("merge conflict") || normalized_error.contains("cherry-pick") {
        return "blocked_merge_conflict";
    }
    if normalized_error.contains("mcp") {
        return "degraded_mcp";
    }
    if normalized_error.contains("transport")
        || normalized_error.contains("broken pipe")
        || normalized_error.contains("connection")
        || normalized_error.contains("interrupted")
    {
        return "interrupted_transport";
    }
    if blocker.is_some() {
        return "truly_idle";
    }
    "truly_idle"
}

pub(crate) fn maybe_commit_provenance(result: Option<&str>) -> Option<LaneCommitProvenance> {
    let commit = extract_commit_sha(result?)?;
    let branch = current_git_branch().unwrap_or_else(|| "unknown".to_string());
    let worktree = std::env::current_dir()
        .ok()
        .map(|path| path.display().to_string());
    Some(LaneCommitProvenance {
        commit: commit.clone(),
        branch,
        worktree,
        canonical_commit: Some(commit.clone()),
        superseded_by: None,
        lineage: vec![commit],
    })
}

fn extract_commit_sha(result: &str) -> Option<String> {
    result
        .split(|c: char| !c.is_ascii_hexdigit())
        .find(|token| token.len() >= 7 && token.len() <= 40)
        .map(str::to_string)
}

fn current_git_branch() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn append_agent_output(path: &str, suffix: &str) -> Result<(), String> {
    use std::io::Write as _;

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(suffix.as_bytes())
        .map_err(|error| error.to_string())
}

fn format_agent_terminal_output(
    status: &str,
    result: Option<&str>,
    blocker: Option<&LaneEventBlocker>,
    error: Option<&str>,
) -> String {
    let mut sections = vec![format!("\n## Result\n\n- status: {status}\n")];
    if let Some(blocker) = blocker {
        sections.push(format!(
            "\n### Blocker\n\n- failure_class: {}\n- detail: {}\n",
            serde_json::to_string(&blocker.failure_class)
                .unwrap_or_else(|_| "\"infra\"".to_string())
                .trim_matches('"'),
            blocker.detail.trim()
        ));
    }
    if let Some(result) = result.filter(|value| !value.trim().is_empty()) {
        sections.push(format!("\n### Final response\n\n{}\n", result.trim()));
    }
    if let Some(error) = error.filter(|value| !value.trim().is_empty()) {
        sections.push(format!("\n### Error\n\n{}\n", error.trim()));
    }
    sections.join("")
}

fn classify_lane_blocker(error: &str) -> LaneEventBlocker {
    let detail = error.trim().to_string();
    LaneEventBlocker {
        failure_class: classify_lane_failure(error),
        detail,
    }
}

pub(crate) fn classify_lane_failure(error: &str) -> LaneFailureClass {
    let normalized = error.to_ascii_lowercase();

    if normalized.contains("prompt") && normalized.contains("deliver") {
        LaneFailureClass::PromptDelivery
    } else if normalized.contains("trust") {
        LaneFailureClass::TrustGate
    } else if normalized.contains("branch")
        && (normalized.contains("stale") || normalized.contains("diverg"))
    {
        LaneFailureClass::BranchDivergence
    } else if normalized.contains("gateway") || normalized.contains("routing") {
        LaneFailureClass::GatewayRouting
    } else if normalized.contains("compile")
        || normalized.contains("build failed")
        || normalized.contains("cargo check")
    {
        LaneFailureClass::Compile
    } else if normalized.contains("test") {
        LaneFailureClass::Test
    } else if normalized.contains("tool failed")
        || normalized.contains("runtime tool")
        || normalized.contains("tool runtime")
    {
        LaneFailureClass::ToolRuntime
    } else if normalized.contains("plugin") {
        LaneFailureClass::PluginStartup
    } else if normalized.contains("mcp") && normalized.contains("handshake") {
        LaneFailureClass::McpHandshake
    } else if normalized.contains("mcp") {
        LaneFailureClass::McpStartup
    } else {
        LaneFailureClass::Infra
    }
}

struct ProviderRuntimeClient {
    runtime: tokio::runtime::Runtime,
    client: ProviderClient,
    model: String,
    capability_runtime: CapabilityRuntime,
    capability_profile: CapabilityProfile,
    session_capability_store: SessionCapabilityStore,
}

impl ProviderRuntimeClient {
    #[allow(clippy::needless_pass_by_value)]
    fn from_capability_provider(
        model: String,
        capability_provider: CapabilityProvider,
        capability_profile: CapabilityProfile,
        session_capability_state: SharedSessionCapabilityState,
    ) -> Result<Self, String> {
        Self::with_capability_runtime(
            model,
            CapabilityRuntime::new(capability_provider),
            capability_profile,
            session_capability_state,
        )
    }

    #[allow(clippy::needless_pass_by_value)]
    fn with_capability_runtime(
        model: String,
        capability_runtime: CapabilityRuntime,
        capability_profile: CapabilityProfile,
        session_capability_state: SharedSessionCapabilityState,
    ) -> Result<Self, String> {
        let model = resolve_model_alias(&model).clone();
        let client = ProviderClient::from_model(&model).map_err(|error| error.to_string())?;
        Ok(Self {
            runtime: tokio::runtime::Runtime::new().map_err(|error| error.to_string())?,
            client,
            model,
            capability_runtime,
            capability_profile,
            session_capability_store: SessionCapabilityStore::from_shared(session_capability_state),
        })
    }
}

impl ApiClient for ProviderRuntimeClient {
    #[allow(clippy::too_many_lines)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let current_dir = std::env::current_dir().ok();
        let (tools, request_overrides) = {
            let state = self.session_capability_store.snapshot();
            let tools = self
                .capability_runtime
                .planned_tool_definitions(
                    CapabilityPlannerInput::new(
                        Some(self.capability_profile.allowed_tools()),
                        Some(&state),
                    )
                    .with_current_dir(current_dir.as_deref()),
                )
                .map_err(RuntimeError::new)?;
            let overrides =
                apply_skill_session_overrides(&self.model, request.system_prompt.clone(), &state);
            (tools, overrides)
        };
        let tools_enabled = !tools.is_empty();
        let message_request = MessageRequest {
            model: request_overrides.model.clone(),
            max_tokens: max_tokens_for_model(&request_overrides.model),
            messages: convert_messages(&request.messages),
            system: (!request_overrides.system_sections.is_empty())
                .then(|| request_overrides.system_sections.join("\n\n")),
            tools: tools_enabled.then_some(tools),
            tool_choice: tools_enabled.then_some(ToolChoice::Auto),
            stream: true,
            temperature: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            reasoning_effort: request_overrides.reasoning_effort.clone(),
        };

        self.runtime.block_on(async {
            let mut stream = self
                .client
                .stream_message(&message_request)
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut events = Vec::new();
            let mut pending_tools: BTreeMap<u32, (String, String, String)> = BTreeMap::new();
            let mut saw_stop = false;

            while let Some(event) = stream
                .next_event()
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?
            {
                match event {
                    ApiStreamEvent::MessageStart(start) => {
                        for block in start.message.content {
                            push_output_block(block, 0, &mut events, &mut pending_tools, true);
                        }
                    }
                    ApiStreamEvent::ContentBlockStart(start) => {
                        push_output_block(
                            start.content_block,
                            start.index,
                            &mut events,
                            &mut pending_tools,
                            true,
                        );
                    }
                    ApiStreamEvent::ContentBlockDelta(delta) => match delta.delta {
                        ContentBlockDelta::TextDelta { text } => {
                            if !text.is_empty() {
                                events.push(AssistantEvent::TextDelta(text));
                            }
                        }
                        ContentBlockDelta::InputJsonDelta { partial_json } => {
                            if let Some((_, _, input)) = pending_tools.get_mut(&delta.index) {
                                input.push_str(&partial_json);
                            }
                        }
                        ContentBlockDelta::ThinkingDelta { .. }
                        | ContentBlockDelta::SignatureDelta { .. } => {}
                    },
                    ApiStreamEvent::ContentBlockStop(stop) => {
                        if let Some((id, name, input)) = pending_tools.remove(&stop.index) {
                            events.push(AssistantEvent::ToolUse { id, name, input });
                        }
                    }
                    ApiStreamEvent::MessageDelta(delta) => {
                        events.push(AssistantEvent::Usage(delta.usage.token_usage()));
                    }
                    ApiStreamEvent::MessageStop(_) => {
                        saw_stop = true;
                        events.push(AssistantEvent::MessageStop);
                    }
                }
            }

            push_prompt_cache_record(&self.client, &mut events);

            if !saw_stop
                && events.iter().any(|event| {
                    matches!(event, AssistantEvent::TextDelta(text) if !text.is_empty())
                        || matches!(event, AssistantEvent::ToolUse { .. })
                })
            {
                events.push(AssistantEvent::MessageStop);
            }

            if events
                .iter()
                .any(|event| matches!(event, AssistantEvent::MessageStop))
            {
                return Ok(events);
            }

            let response = self
                .client
                .send_message(&MessageRequest {
                    stream: false,
                    ..message_request.clone()
                })
                .await
                .map_err(|error| RuntimeError::new(error.to_string()))?;
            let mut events = response_to_events(response);
            push_prompt_cache_record(&self.client, &mut events);
            Ok(events)
        })
    }
}

pub(crate) struct SubagentToolExecutor {
    capability_runtime: CapabilityRuntime,
    capability_profile: CapabilityProfile,
    session_capability_store: SessionCapabilityStore,
}

impl SubagentToolExecutor {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn new(allowed_tools: BTreeSet<String>) -> Self {
        Self::from_capability_provider(
            CapabilityProfile::from_tools(allowed_tools),
            CapabilityProvider::builtin(),
            std::sync::Arc::new(std::sync::Mutex::new(SessionCapabilityState::default())),
        )
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn from_capability_provider(
        capability_profile: CapabilityProfile,
        capability_provider: CapabilityProvider,
        session_capability_state: SharedSessionCapabilityState,
    ) -> Self {
        Self::from_capability_runtime(
            capability_profile,
            CapabilityRuntime::new(capability_provider),
            session_capability_state,
        )
    }

    pub(crate) fn from_capability_runtime(
        capability_profile: CapabilityProfile,
        capability_runtime: CapabilityRuntime,
        session_capability_state: SharedSessionCapabilityState,
    ) -> Self {
        Self {
            capability_runtime,
            capability_profile,
            session_capability_store: SessionCapabilityStore::from_shared(session_capability_state),
        }
    }

    pub(crate) fn with_enforcer(mut self, enforcer: PermissionEnforcer) -> Self {
        self.capability_runtime.set_enforcer(enforcer);
        self
    }
}

impl ToolExecutor for SubagentToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        let value: Value = serde_json::from_str(input)
            .map_err(|error| ToolError::new(format!("invalid tool input JSON: {error}")))?;
        let current_dir = std::env::current_dir().ok();
        let state = self.session_capability_store.snapshot();
        let capability_runtime = self.capability_runtime.clone();
        let capability_store = self.session_capability_store.clone();
        capability_runtime.execute_tool(
            tool_name,
            value,
            CapabilityPlannerInput::new(
                Some(self.capability_profile.allowed_tools()),
                Some(&state),
            )
            .with_current_dir(current_dir.as_deref()),
            &capability_store,
            None,
            None,
            move |_dispatch_kind, name, _value| {
                Err(ToolError::new(format!(
                    "runtime capability `{name}` is not available for this sub-agent"
                )))
            },
        )
    }
}

fn convert_messages(messages: &[ConversationMessage]) -> Vec<InputMessage> {
    messages
        .iter()
        .filter_map(|message| {
            let role = match message.role {
                MessageRole::System | MessageRole::User | MessageRole::Tool => "user",
                MessageRole::Assistant => "assistant",
            };
            let content = message
                .blocks
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } => InputContentBlock::Text { text: text.clone() },
                    ContentBlock::ToolUse { id, name, input } => InputContentBlock::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(input)
                            .unwrap_or_else(|_| serde_json::json!({ "raw": input })),
                    },
                    ContentBlock::ToolResult {
                        tool_use_id,
                        output,
                        is_error,
                        ..
                    } => InputContentBlock::ToolResult {
                        tool_use_id: tool_use_id.clone(),
                        content: vec![ToolResultContentBlock::Text {
                            text: output.clone(),
                        }],
                        is_error: *is_error,
                    },
                })
                .collect::<Vec<_>>();
            (!content.is_empty()).then(|| InputMessage {
                role: role.to_string(),
                content,
            })
        })
        .collect()
}

pub(crate) fn push_output_block(
    block: OutputContentBlock,
    block_index: u32,
    events: &mut Vec<AssistantEvent>,
    pending_tools: &mut BTreeMap<u32, (String, String, String)>,
    streaming_tool_input: bool,
) {
    match block {
        OutputContentBlock::Text { text } => {
            if !text.is_empty() {
                events.push(AssistantEvent::TextDelta(text));
            }
        }
        OutputContentBlock::ToolUse { id, name, input } => {
            let initial_input = if streaming_tool_input
                && input.is_object()
                && input.as_object().is_some_and(serde_json::Map::is_empty)
            {
                String::new()
            } else {
                input.to_string()
            };
            pending_tools.insert(block_index, (id, name, initial_input));
        }
        OutputContentBlock::Thinking { .. } | OutputContentBlock::RedactedThinking { .. } => {}
    }
}

fn response_to_events(response: MessageResponse) -> Vec<AssistantEvent> {
    let mut events = Vec::new();
    let mut pending_tools = BTreeMap::new();

    for (index, block) in response.content.into_iter().enumerate() {
        let index = u32::try_from(index).expect("response block index overflow");
        push_output_block(block, index, &mut events, &mut pending_tools, false);
        if let Some((id, name, input)) = pending_tools.remove(&index) {
            events.push(AssistantEvent::ToolUse { id, name, input });
        }
    }

    events.push(AssistantEvent::Usage(response.usage.token_usage()));
    events.push(AssistantEvent::MessageStop);
    events
}

fn push_prompt_cache_record(client: &ProviderClient, events: &mut Vec<AssistantEvent>) {
    if let Some(record) = client.take_last_prompt_cache_record() {
        if let Some(event) = prompt_cache_record_to_runtime_event(record) {
            events.push(AssistantEvent::PromptCache(event));
        }
    }
}

fn prompt_cache_record_to_runtime_event(
    record: api::PromptCacheRecord,
) -> Option<PromptCacheEvent> {
    let cache_break = record.cache_break?;
    Some(PromptCacheEvent {
        unexpected: cache_break.unexpected,
        reason: cache_break.reason,
        previous_cache_read_input_tokens: cache_break.previous_cache_read_input_tokens,
        current_cache_read_input_tokens: cache_break.current_cache_read_input_tokens,
        token_drop: cache_break.token_drop,
    })
}

pub(crate) fn final_assistant_text(summary: &runtime::TurnSummary) -> String {
    summary
        .assistant_messages
        .last()
        .map(|message| {
            message
                .blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

fn agent_store_dir() -> Result<std::path::PathBuf, String> {
    if let Ok(path) = std::env::var("CLAWD_AGENT_STORE") {
        return Ok(std::path::PathBuf::from(path));
    }
    let cwd = std::env::current_dir().map_err(|error| error.to_string())?;
    if let Some(workspace_root) = cwd.ancestors().nth(2) {
        return Ok(workspace_root.join(".clawd-agents"));
    }
    Ok(cwd.join(".clawd-agents"))
}

fn make_agent_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("agent-{nanos}")
}

fn slugify_agent_name(description: &str) -> String {
    let mut out = description
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').chars().take(32).collect()
}

fn normalize_subagent_type(subagent_type: Option<&str>) -> String {
    let trimmed = subagent_type.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        return String::from("general-purpose");
    }

    match canonical_tool_token(trimmed).as_str() {
        "general" | "generalpurpose" | "generalpurposeagent" => String::from("general-purpose"),
        "explore" | "explorer" | "exploreagent" => String::from("Explore"),
        "plan" | "planagent" => String::from("Plan"),
        "verification" | "verificationagent" | "verify" | "verifier" => {
            String::from("Verification")
        }
        "clawguide" | "clawguideagent" | "guide" => String::from("claw-guide"),
        "statusline" | "statuslinesetup" => String::from("statusline-setup"),
        _ => trimmed.to_string(),
    }
}

pub(crate) fn iso8601_now() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}
