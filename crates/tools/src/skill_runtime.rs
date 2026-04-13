use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::{Mutex, OnceLock};

use glob::glob;
use runtime::ConfigLoader;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::capability_runtime::{
    CapabilityConcurrencyPolicy, CapabilityExecutionKind, CapabilityInvocationPolicy,
    CapabilityPermissionProfile, CapabilitySourceKind, CapabilitySpec, CapabilityState,
    CapabilityVisibility,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SkillContextKind {
    #[default]
    Inline,
    Fork,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SkillInvocationKind {
    User,
    Model,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct SkillArgumentSpec {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub(crate) struct SkillFrontmatter {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, alias = "when_to_use")]
    pub when_to_use: Option<String>,
    #[serde(default, rename = "allowed-tools", alias = "allowed_tools")]
    pub allowed_tools: Vec<String>,
    #[serde(default)]
    pub arguments: Vec<SkillArgumentSpec>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default, rename = "user-invocable", alias = "user_invocable")]
    pub user_invocable: Option<bool>,
    #[serde(default, rename = "model-invocable", alias = "model_invocable")]
    pub model_invocable: Option<bool>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub context: SkillContextKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillCapability {
    pub capability_id: String,
    pub source_kind: CapabilitySourceKind,
    pub display_name: String,
    pub description: String,
    pub when_to_use: Option<String>,
    pub path: PathBuf,
    pub prompt_body: String,
    pub frontmatter: SkillFrontmatter,
}

impl SkillCapability {
    fn search_hint(&self) -> Option<String> {
        self.when_to_use
            .clone()
            .or_else(|| Some(self.description.clone()))
    }

    fn is_model_invocable(&self) -> bool {
        self.frontmatter.model_invocable.unwrap_or(true)
    }

    fn is_user_invocable(&self) -> bool {
        self.frontmatter.user_invocable.unwrap_or(true)
    }

    fn is_trusted(&self, current_dir: Option<&Path>) -> bool {
        if self.source_kind == CapabilitySourceKind::BundledSkill {
            return true;
        }

        let Some(current_dir) = current_dir else {
            return true;
        };

        let trusted_roots = ConfigLoader::default_for(current_dir)
            .load()
            .ok()
            .map(|config| config.trusted_roots().to_vec())
            .unwrap_or_default();
        if trusted_roots.is_empty() {
            return true;
        }

        trusted_roots
            .iter()
            .any(|root| path_matches_trusted_root(current_dir, Path::new(root)))
    }

    fn matches_paths(&self, current_dir: Option<&Path>) -> bool {
        if self.frontmatter.paths.is_empty() {
            return true;
        }
        let Some(current_dir) = current_dir else {
            return false;
        };

        self.frontmatter.paths.iter().any(|pattern| {
            if pattern.trim().is_empty() {
                return false;
            }
            let candidate = current_dir.join(pattern);
            if !pattern.contains('*') && !pattern.contains('?') {
                return candidate.exists();
            }

            glob(&candidate.to_string_lossy())
                .ok()
                .and_then(|mut entries| entries.find_map(Result::ok))
                .is_some()
        })
    }

    fn to_capability_spec(&self) -> CapabilitySpec {
        CapabilitySpec {
            capability_id: self.capability_id.clone(),
            source_kind: self.source_kind,
            execution_kind: CapabilityExecutionKind::PromptSkill,
            provider_key: None,
            executor_key: None,
            display_name: self.display_name.clone(),
            description: self.description.clone(),
            when_to_use: self.when_to_use.clone(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "skill": { "type": "string" },
                    "arguments": {}
                },
                "required": ["skill"],
                "additionalProperties": false
            }),
            search_hint: self.search_hint(),
            visibility: CapabilityVisibility::DefaultVisible,
            state: CapabilityState::Ready,
            permission_profile: CapabilityPermissionProfile {
                required_permission: runtime::PermissionMode::ReadOnly,
            },
            trust_profile: crate::capability_runtime::CapabilityTrustProfile {
                requires_trusted_workspace: self.source_kind == CapabilitySourceKind::LocalSkill,
                requires_explicit_user_trust: false,
            },
            scope_constraints: crate::capability_runtime::CapabilityScopeConstraints {
                workspace_only: !self.frontmatter.paths.is_empty(),
                requires_current_dir: !self.frontmatter.paths.is_empty(),
            },
            invocation_policy: CapabilityInvocationPolicy {
                selectable: self.is_model_invocable(),
                requires_approval: false,
                requires_auth: false,
            },
            concurrency_policy: CapabilityConcurrencyPolicy::Serialized,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillDiscoveryOutput {
    matches: Vec<String>,
    results: Vec<SkillDiscoveryResult>,
    query: String,
    normalized_query: String,
    #[serde(rename = "total_skills")]
    total_skills: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillDiscoveryResult {
    name: String,
    source_kind: CapabilitySourceKind,
    execution_kind: CapabilityExecutionKind,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    when_to_use: Option<String>,
    path: String,
    tool_grants: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effort_override: Option<String>,
    context: SkillContextKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    search_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillInjectedMessage {
    role: String,
    content: String,
}

impl SkillInjectedMessage {
    #[must_use]
    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkillStateUpdate {
    ContextPrepared {
        context: SkillContextKind,
    },
    MessageInjected {
        role: String,
    },
    ForkSpawned {
        agent_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subagent_type: Option<String>,
        output_file: String,
        manifest_file: String,
    },
    ForkRestored {
        agent_id: String,
        status: String,
        derived_state: String,
        output_file: String,
        manifest_file: String,
    },
    ForkCompleted {
        agent_id: String,
        output_file: String,
        manifest_file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        completed_at: Option<String>,
    },
    ForkFailed {
        agent_id: String,
        output_file: String,
        manifest_file: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    ToolGranted {
        tool: String,
    },
    ModelOverride {
        model: String,
    },
    EffortOverride {
        effort: String,
    },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillExecutionResult {
    pub skill: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub context: SkillContextKind,
    pub messages_to_inject: Vec<SkillInjectedMessage>,
    pub tool_grants: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort_override: Option<String>,
    pub state_updates: Vec<SkillStateUpdate>,
}

impl SkillExecutionResult {
    #[must_use]
    pub fn injected_system_sections(&self) -> Vec<String> {
        self.messages_to_inject
            .iter()
            .map(|message| message.content.clone())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillExecutionFailure {
    pub(crate) message: String,
    pub(crate) state_updates: Vec<SkillStateUpdate>,
}

#[derive(Debug, Clone)]
pub struct PromptSkillExecutor {
    capability_executor: Option<crate::CapabilityExecutor>,
}

impl PromptSkillExecutor {
    #[must_use]
    pub fn new(capability_executor: Option<crate::CapabilityExecutor>) -> Self {
        Self {
            capability_executor,
        }
    }

    pub fn execute(
        &self,
        capability: &CapabilitySpec,
        arguments: Option<Value>,
        current_dir: Option<&Path>,
    ) -> Result<SkillExecutionResult, SkillExecutionFailure> {
        match capability.source_kind {
            CapabilitySourceKind::LocalSkill | CapabilitySourceKind::BundledSkill => {
                let skill = discover_skill_capabilities()
                    .into_iter()
                    .find(|skill| {
                        skill.source_kind == capability.source_kind
                            && (skill.capability_id == capability.capability_id
                                || skill.display_name == capability.display_name)
                    })
                    .ok_or_else(|| SkillExecutionFailure {
                        message: format!(
                            "skill `{}` backing definition is no longer available",
                            capability.display_name
                        ),
                        state_updates: Vec::new(),
                    })?;
                execute_resolved_skill_capability_detailed(skill, arguments)
            }
            CapabilitySourceKind::PluginSkill | CapabilitySourceKind::McpPrompt => self
                .capability_executor
                .clone()
                .ok_or_else(|| SkillExecutionFailure {
                    message: format!(
                        "skill `{}` does not have a runtime executor yet",
                        capability.display_name
                    ),
                    state_updates: Vec::new(),
                })?
                .execute_prompt_skill(capability, arguments, current_dir),
            _ => Err(SkillExecutionFailure {
                message: format!(
                    "skill `{}` uses unsupported source `{}`",
                    capability.display_name,
                    serde_json::to_string(&capability.source_kind)
                        .unwrap_or_else(|_| "unknown".to_string())
                ),
                state_updates: Vec::new(),
            }),
        }
    }
}

#[derive(Debug, Clone)]
struct ForkSpawnSnapshot {
    output_file: String,
    manifest_file: String,
}

#[derive(Debug, Default)]
struct ForkLifecycleState {
    spawned: Option<ForkSpawnSnapshot>,
    restored: bool,
    completed: bool,
    failed: bool,
}

pub(crate) fn reconcile_fork_lifecycle_updates(
    existing_updates: &[SkillStateUpdate],
    emit_restore_events: bool,
) -> Vec<SkillStateUpdate> {
    let mut fork_states = BTreeMap::<String, ForkLifecycleState>::new();
    for update in existing_updates {
        match update {
            SkillStateUpdate::ForkSpawned {
                agent_id,
                output_file,
                manifest_file,
                ..
            } => {
                let entry = fork_states.entry(agent_id.clone()).or_default();
                entry.spawned = Some(ForkSpawnSnapshot {
                    output_file: output_file.clone(),
                    manifest_file: manifest_file.clone(),
                });
            }
            SkillStateUpdate::ForkRestored { agent_id, .. } => {
                fork_states.entry(agent_id.clone()).or_default().restored = true;
            }
            SkillStateUpdate::ForkCompleted { agent_id, .. } => {
                fork_states.entry(agent_id.clone()).or_default().completed = true;
            }
            SkillStateUpdate::ForkFailed { agent_id, .. } => {
                fork_states.entry(agent_id.clone()).or_default().failed = true;
            }
            SkillStateUpdate::ContextPrepared { .. }
            | SkillStateUpdate::MessageInjected { .. }
            | SkillStateUpdate::ToolGranted { .. }
            | SkillStateUpdate::ModelOverride { .. }
            | SkillStateUpdate::EffortOverride { .. } => {}
        }
    }

    let mut new_updates = Vec::new();
    for (agent_id, state) in fork_states {
        let Some(spawned) = state.spawned else {
            continue;
        };
        let Some(manifest) = load_fork_agent_manifest(&spawned.manifest_file) else {
            continue;
        };
        let output_file = if manifest.output_file.trim().is_empty() {
            spawned.output_file.clone()
        } else {
            manifest.output_file.clone()
        };
        let manifest_file = if manifest.manifest_file.trim().is_empty() {
            spawned.manifest_file.clone()
        } else {
            manifest.manifest_file.clone()
        };

        if emit_restore_events && !state.restored {
            new_updates.push(SkillStateUpdate::ForkRestored {
                agent_id: agent_id.clone(),
                status: manifest.status.clone(),
                derived_state: manifest.derived_state.clone(),
                output_file: output_file.clone(),
                manifest_file: manifest_file.clone(),
            });
        }

        let terminal_recorded = state.completed || state.failed;
        match manifest.status.trim().to_ascii_lowercase().as_str() {
            "completed" if !terminal_recorded => {
                new_updates.push(SkillStateUpdate::ForkCompleted {
                    agent_id,
                    output_file,
                    manifest_file,
                    completed_at: manifest.completed_at.clone(),
                });
            }
            "failed" if !terminal_recorded => {
                new_updates.push(SkillStateUpdate::ForkFailed {
                    agent_id,
                    output_file,
                    manifest_file,
                    error: manifest.error.clone(),
                });
            }
            _ => {}
        }
    }

    new_updates
}

pub(crate) fn discover_skill_capabilities() -> Vec<SkillCapability> {
    let mut seen_names = BTreeSet::new();
    let mut discovered = Vec::new();

    for (source_kind, root) in resolve_skill_roots() {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path().join("SKILL.md");
            if !path.exists() {
                continue;
            }
            let Ok(Some(skill)) = load_skill_capability_from_path(&path, source_kind) else {
                continue;
            };
            let key = skill.display_name.to_ascii_lowercase();
            if seen_names.insert(key) {
                discovered.push(skill);
            }
        }
    }

    discovered
}

pub(crate) fn execute_skill_capability_from_spec_detailed(
    capability: &CapabilitySpec,
    arguments: Option<Value>,
    current_dir: Option<&Path>,
    executor: Option<crate::CapabilityExecutor>,
) -> Result<SkillExecutionResult, SkillExecutionFailure> {
    PromptSkillExecutor::new(executor).execute(capability, arguments, current_dir)
}

pub(crate) fn prompt_skill_has_runtime_executor(capability: &CapabilitySpec) -> bool {
    capability.execution_kind == CapabilityExecutionKind::PromptSkill
        && match capability.source_kind {
            CapabilitySourceKind::LocalSkill | CapabilitySourceKind::BundledSkill => true,
            CapabilitySourceKind::PluginSkill | CapabilitySourceKind::McpPrompt => capability
                .executor_key
                .as_ref()
                .is_some_and(|key| !key.trim().is_empty()),
            _ => false,
        }
}

pub(crate) fn explain_model_skill_unavailability(
    skill_name: &str,
    current_dir: Option<&Path>,
) -> String {
    resolve_executable_skill(skill_name, current_dir, SkillInvocationKind::Model)
        .map(|_| {
            format!(
                "skill `{}` is not enabled in the current capability surface",
                normalize_requested_skill_name(skill_name)
            )
        })
        .unwrap_or_else(|message| message)
}

pub(crate) fn normalize_requested_skill_name(skill_name: &str) -> String {
    skill_name
        .trim()
        .trim_start_matches('/')
        .trim_start_matches('$')
        .to_string()
}

fn execute_resolved_skill_capability_detailed(
    skill: SkillCapability,
    arguments: Option<Value>,
) -> Result<SkillExecutionResult, SkillExecutionFailure> {
    let rendered_prompt = render_skill_prompt(&skill, arguments);
    let mut state_updates = vec![SkillStateUpdate::ContextPrepared {
        context: skill.frontmatter.context,
    }];
    let messages_to_inject = match skill.frontmatter.context {
        SkillContextKind::Inline => {
            state_updates.push(SkillStateUpdate::MessageInjected {
                role: "system".to_string(),
            });
            vec![SkillInjectedMessage::system(rendered_prompt.clone())]
        }
        SkillContextKind::Fork => {
            let agent = match spawn_fork_skill_agent(&skill, &rendered_prompt) {
                Ok(agent) => agent,
                Err(failure) => {
                    if let Some(manifest) = failure.manifest {
                        state_updates.push(SkillStateUpdate::ForkSpawned {
                            agent_id: manifest.agent_id.clone(),
                            subagent_type: manifest.subagent_type.clone(),
                            output_file: manifest.output_file.clone(),
                            manifest_file: manifest.manifest_file.clone(),
                        });
                        state_updates.push(SkillStateUpdate::ForkFailed {
                            agent_id: manifest.agent_id,
                            output_file: manifest.output_file,
                            manifest_file: manifest.manifest_file,
                            error: Some(failure.error.clone()),
                        });
                    }
                    return Err(SkillExecutionFailure {
                        message: failure.error,
                        state_updates,
                    });
                }
            };
            state_updates.push(SkillStateUpdate::ForkSpawned {
                agent_id: agent.agent_id,
                subagent_type: agent.subagent_type,
                output_file: agent.output_file,
                manifest_file: agent.manifest_file,
            });
            Vec::new()
        }
    };
    state_updates.extend(
        skill
            .frontmatter
            .allowed_tools
            .iter()
            .cloned()
            .map(|tool| SkillStateUpdate::ToolGranted { tool }),
    );
    if let Some(model) = skill.frontmatter.model.clone() {
        state_updates.push(SkillStateUpdate::ModelOverride { model });
    }
    if let Some(effort) = skill.frontmatter.effort.clone() {
        state_updates.push(SkillStateUpdate::EffortOverride { effort });
    }

    Ok(SkillExecutionResult {
        skill: skill.display_name,
        path: skill.path.display().to_string(),
        description: Some(skill.description),
        context: skill.frontmatter.context,
        messages_to_inject,
        tool_grants: skill.frontmatter.allowed_tools.clone(),
        model_override: skill.frontmatter.model.clone(),
        effort_override: skill.frontmatter.effort.clone(),
        state_updates,
    })
}

pub(crate) fn compile_skill_capability_specs(current_dir: Option<&Path>) -> Vec<CapabilitySpec> {
    discover_skill_capabilities()
        .into_iter()
        .filter(|skill| {
            skill.is_model_invocable()
                && skill.matches_paths(current_dir)
                && skill.is_trusted(current_dir)
        })
        .map(|skill| skill.to_capability_spec())
        .collect()
}

pub(crate) fn discover_skills_from_capability_specs(
    query: &str,
    max_results: usize,
    capabilities: &[CapabilitySpec],
) -> SkillDiscoveryOutput {
    let query = query.trim().to_string();
    let normalized_query = crate::normalize_tool_search_query(&query);
    let known_skills = discover_skill_capabilities()
        .into_iter()
        .map(|skill| {
            (
                capability_metadata_key(skill.source_kind, &skill.capability_id),
                SearchableSkillSpec::from(skill),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let searchable = capabilities
        .iter()
        .map(|capability| {
            SearchableSkillSpec::from_capability_spec(
                capability,
                known_skills.get(&capability_metadata_key(
                    capability.source_kind,
                    &capability.capability_id,
                )),
            )
        })
        .collect::<Vec<_>>();
    let names = crate::search_tool_specs(
        &query,
        max_results.max(1),
        &searchable
            .iter()
            .map(SearchableSkillSpec::as_tool_spec)
            .collect::<Vec<_>>(),
    );
    let results = names
        .iter()
        .filter_map(|name| searchable.iter().find(|skill| &skill.name == name))
        .map(SearchableSkillSpec::to_result)
        .collect::<Vec<_>>();

    SkillDiscoveryOutput {
        matches: names,
        results,
        query,
        normalized_query,
        total_skills: searchable.len(),
    }
}

fn capability_metadata_key(source_kind: CapabilitySourceKind, capability_id: &str) -> String {
    format!("{source_kind:?}:{capability_id}")
}

fn resolve_executable_skill(
    skill_name: &str,
    current_dir: Option<&Path>,
    invocation_kind: SkillInvocationKind,
) -> Result<SkillCapability, String> {
    let requested = normalize_requested_skill_name(skill_name);
    let skill = discover_skill_capabilities()
        .into_iter()
        .find(|skill| {
            skill.display_name.eq_ignore_ascii_case(&requested)
                || skill.capability_id.eq_ignore_ascii_case(&requested)
        })
        .ok_or_else(|| format!("unknown skill: {requested}"))?;

    match invocation_kind {
        SkillInvocationKind::User if !skill.is_user_invocable() => {
            return Err(format!("skill `{requested}` is not user invocable"));
        }
        SkillInvocationKind::Model if !skill.is_model_invocable() => {
            return Err(format!("skill `{requested}` is not model invocable"));
        }
        SkillInvocationKind::User | SkillInvocationKind::Model => {}
    }
    if !skill.matches_paths(current_dir) {
        return Err(format!(
            "skill `{requested}` is not visible for the current workspace"
        ));
    }
    if !skill.is_trusted(current_dir) {
        return Err(format!(
            "skill `{requested}` is not trusted for the current workspace"
        ));
    }

    Ok(skill)
}

fn resolve_skill_roots() -> Vec<(CapabilitySourceKind, PathBuf)> {
    let local_roots = resolve_local_skill_roots()
        .into_iter()
        .map(|path| (CapabilitySourceKind::LocalSkill, path));
    let bundled_roots = resolve_bundled_skill_roots()
        .into_iter()
        .map(|path| (CapabilitySourceKind::BundledSkill, path));
    local_roots.chain(bundled_roots).collect()
}

fn resolve_local_skill_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(claw_config_home) = std::env::var("CLAW_CONFIG_HOME") {
        candidates.push(PathBuf::from(claw_config_home).join("skills"));
    }
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        candidates.push(PathBuf::from(codex_home).join("skills"));
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".claw").join("skills"));
        candidates.push(home.join(".agents").join("skills"));
        candidates.push(home.join(".config").join("opencode").join("skills"));
        candidates.push(home.join(".codex").join("skills"));
        candidates.push(home.join(".claude").join("skills"));
    }
    candidates.push(PathBuf::from("/home/bellman/.claw/skills"));
    candidates.push(PathBuf::from("/home/bellman/.codex/skills"));
    candidates
}

fn resolve_bundled_skill_roots() -> Vec<PathBuf> {
    if let Some(roots) = std::env::var_os("OCTOPUS_BUNDLED_SKILLS_ROOTS") {
        let parsed = std::env::split_paths(&roots).collect::<Vec<_>>();
        if !parsed.is_empty() {
            return parsed;
        }
    }

    vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../octopus-infra/seed/builtin-assets/skills")]
}

fn load_skill_capability_from_path(
    path: &Path,
    source_kind: CapabilitySourceKind,
) -> Result<Option<SkillCapability>, String> {
    let contents = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let (frontmatter, prompt_body) = parse_frontmatter(&contents)?;
    let prompt_body = prompt_body.trim().to_string();
    let fallback_name = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("skill")
        .to_string();
    let display_name = frontmatter.name.clone().unwrap_or(fallback_name.clone());
    if display_name.trim().is_empty() {
        return Ok(None);
    }
    let description = frontmatter
        .description
        .clone()
        .or_else(|| parse_skill_description_from_body(&prompt_body))
        .unwrap_or_else(|| format!("Skill `{display_name}`"));

    Ok(Some(SkillCapability {
        capability_id: fallback_name,
        source_kind,
        display_name,
        description,
        when_to_use: frontmatter.when_to_use.clone(),
        path: path.to_path_buf(),
        prompt_body,
        frontmatter,
    }))
}

fn parse_frontmatter(contents: &str) -> Result<(SkillFrontmatter, String), String> {
    let normalized = contents.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Ok((SkillFrontmatter::default(), normalized));
    };
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        return Ok((SkillFrontmatter::default(), normalized));
    };
    let parsed = serde_yaml::from_str::<SkillFrontmatter>(frontmatter)
        .map_err(|error| format!("invalid skill frontmatter: {error}"))?;
    Ok((parsed, body.to_string()))
}

fn parse_skill_description_from_body(contents: &str) -> Option<String> {
    contents
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
}

fn render_skill_prompt(skill: &SkillCapability, arguments: Option<Value>) -> String {
    let mut sections = vec![skill.prompt_body.trim().to_string()];
    if let Some(arguments) = arguments.filter(|value| !value.is_null()) {
        let rendered =
            serde_json::to_string_pretty(&arguments).unwrap_or_else(|_| arguments.to_string());
        sections.push(format!("Skill arguments:\n{rendered}"));
    }
    sections.join("\n\n")
}

fn path_matches_trusted_root(candidate: &Path, trusted_root: &Path) -> bool {
    let candidate = normalize_path(candidate);
    let trusted_root = normalize_path(trusted_root);
    candidate == trusted_root || candidate.starts_with(&trusted_root)
}

fn normalize_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

#[derive(Debug, Clone)]
struct SearchableSkillSpec {
    name: String,
    description: String,
    source_kind: CapabilitySourceKind,
    when_to_use: Option<String>,
    path: String,
    tool_grants: Vec<String>,
    model_override: Option<String>,
    effort_override: Option<String>,
    context: SkillContextKind,
    search_hint: Option<String>,
}

impl SearchableSkillSpec {
    fn from_capability_spec(
        capability: &CapabilitySpec,
        known_skill: Option<&SearchableSkillSpec>,
    ) -> Self {
        let fallback = Self {
            name: capability.display_name.clone(),
            description: capability.description.clone(),
            source_kind: capability.source_kind,
            when_to_use: capability.when_to_use.clone(),
            path: capability.capability_id.clone(),
            tool_grants: Vec::new(),
            model_override: None,
            effort_override: None,
            context: SkillContextKind::Inline,
            search_hint: capability.search_hint.clone(),
        };

        let Some(known_skill) = known_skill else {
            return fallback;
        };

        Self {
            name: capability.display_name.clone(),
            description: capability.description.clone(),
            source_kind: capability.source_kind,
            when_to_use: capability.when_to_use.clone(),
            path: known_skill.path.clone(),
            tool_grants: known_skill.tool_grants.clone(),
            model_override: known_skill.model_override.clone(),
            effort_override: known_skill.effort_override.clone(),
            context: known_skill.context,
            search_hint: capability
                .search_hint
                .clone()
                .or_else(|| known_skill.search_hint.clone()),
        }
    }

    fn as_tool_spec(&self) -> crate::tool_registry::SearchableToolSpec {
        crate::tool_registry::SearchableToolSpec {
            name: self.name.clone(),
            description: format!(
                "{} {}",
                self.description,
                self.when_to_use.clone().unwrap_or_default()
            )
            .trim()
            .to_string(),
            source_kind: self.source_kind,
            permission: runtime::PermissionMode::ReadOnly.as_str().to_string(),
            state: CapabilityState::Ready,
            requires_auth: false,
            requires_approval: false,
            deferred: false,
            search_hint: self.search_hint.clone(),
        }
    }

    fn to_result(&self) -> SkillDiscoveryResult {
        SkillDiscoveryResult {
            name: self.name.clone(),
            source_kind: self.source_kind,
            execution_kind: CapabilityExecutionKind::PromptSkill,
            description: self.description.clone(),
            when_to_use: self.when_to_use.clone(),
            path: self.path.clone(),
            tool_grants: self.tool_grants.clone(),
            model_override: self.model_override.clone(),
            effort_override: self.effort_override.clone(),
            context: self.context,
            search_hint: self.search_hint.clone(),
        }
    }
}

impl From<SkillCapability> for SearchableSkillSpec {
    fn from(skill: SkillCapability) -> Self {
        let search_hint = skill.search_hint();
        Self {
            name: skill.display_name,
            description: skill.description,
            source_kind: skill.source_kind,
            when_to_use: skill.when_to_use,
            path: skill.path.display().to_string(),
            tool_grants: skill.frontmatter.allowed_tools,
            model_override: skill.frontmatter.model,
            effort_override: skill.frontmatter.effort,
            context: skill.frontmatter.context,
            search_hint,
        }
    }
}

#[derive(Debug, Clone)]
struct ForkedSkillAgent {
    agent_id: String,
    subagent_type: Option<String>,
    output_file: String,
    manifest_file: String,
}

fn spawn_fork_skill_agent(
    skill: &SkillCapability,
    rendered_prompt: &str,
) -> Result<ForkedSkillAgent, crate::workspace_runtime::AgentSpawnFailure> {
    let agent = crate::workspace_runtime::execute_agent_with_spawn_detailed(
        crate::workspace_runtime::AgentInput {
            description: format!("Execute fork skill `{}`", skill.display_name),
            prompt: rendered_prompt.to_string(),
            subagent_type: skill.frontmatter.agent.clone(),
            name: Some(skill.capability_id.clone()),
            model: skill.frontmatter.model.clone(),
        },
        spawn_skill_fork_job,
    )?;

    Ok(ForkedSkillAgent {
        agent_id: agent.agent_id,
        subagent_type: agent.subagent_type,
        output_file: agent.output_file,
        manifest_file: agent.manifest_file,
    })
}

#[cfg(test)]
type SkillForkSpawnFn = fn(crate::workspace_runtime::AgentJob) -> Result<(), String>;

#[cfg(test)]
fn skill_fork_spawn_override() -> &'static Mutex<Option<SkillForkSpawnFn>> {
    static OVERRIDE: OnceLock<Mutex<Option<SkillForkSpawnFn>>> = OnceLock::new();
    OVERRIDE.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
pub(crate) fn set_skill_fork_spawn_override(override_fn: Option<SkillForkSpawnFn>) {
    let mut slot = skill_fork_spawn_override()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *slot = override_fn;
}

fn spawn_skill_fork_job(job: crate::workspace_runtime::AgentJob) -> Result<(), String> {
    #[cfg(test)]
    {
        if let Some(override_fn) = *skill_fork_spawn_override()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
        {
            return override_fn(job);
        }
    }

    crate::workspace_runtime::spawn_agent_job(job)
}

fn load_fork_agent_manifest(path: &str) -> Option<crate::workspace_runtime::AgentOutput> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}
