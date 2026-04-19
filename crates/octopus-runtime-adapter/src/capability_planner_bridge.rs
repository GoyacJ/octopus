use super::*;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub(crate) struct CapabilityProjection {
    pub(crate) plan_summary: RuntimeCapabilityPlanSummary,
    pub(crate) provider_state_summary: Vec<RuntimeCapabilityProviderState>,
    pub(crate) auth_state_summary: RuntimeAuthStateSummary,
    pub(crate) policy_decision_summary: RuntimePolicyDecisionSummary,
    pub(crate) capability_state_ref: String,
}

pub(crate) struct PreparedCapabilityRuntime {
    pub(crate) capability_runtime: tools::CapabilityRuntime,
    pub(crate) visible_capabilities: Vec<tools::CapabilitySpec>,
    pub(crate) projection: CapabilityProjection,
    pub(crate) managed_mcp_runtime: Option<Arc<Mutex<tools::ManagedMcpRuntime>>>,
    pub(crate) planned_tool_names: BTreeSet<String>,
}

fn capability_names(capabilities: &[tools::CapabilitySpec]) -> Vec<String> {
    capabilities
        .iter()
        .map(|capability| capability.display_name.clone())
        .collect()
}

fn matches_manifest_skill(
    capability: &tools::CapabilitySpec,
    skill_ids: &[String],
    allowed_mcp_servers: &BTreeSet<String>,
) -> bool {
    match capability.source_kind {
        tools::CapabilitySourceKind::LocalSkill
        | tools::CapabilitySourceKind::BundledSkill
        | tools::CapabilitySourceKind::PluginSkill => skill_ids.iter().any(|skill_id| {
            capability.display_name.eq_ignore_ascii_case(skill_id)
                || capability.capability_id.eq_ignore_ascii_case(skill_id)
        }),
        tools::CapabilitySourceKind::McpPrompt => capability
            .provider_key
            .as_deref()
            .is_some_and(|provider| allowed_mcp_servers.contains(provider)),
        _ => true,
    }
}

fn provider_state_label(state: tools::CapabilityState) -> &'static str {
    match state {
        tools::CapabilityState::Ready => "ready",
        tools::CapabilityState::Pending => "pending",
        tools::CapabilityState::AuthRequired => "auth_required",
        tools::CapabilityState::ApprovalRequired => "approval_required",
        tools::CapabilityState::Degraded => "degraded",
        tools::CapabilityState::Unavailable => "unavailable",
    }
}

fn filtered_capability_surface(
    session_policy: &session_policy::CompiledSessionPolicy,
    plan: &tools::CapabilityExecutionPlan,
) -> (
    Vec<tools::CapabilitySpec>,
    Vec<tools::CapabilitySpec>,
    Vec<String>,
    u64,
) {
    let mut visible_tools = Vec::new();
    let mut deferred_tools = Vec::new();
    let mut hidden_capabilities = Vec::new();
    let mut denied_exposures = 0_u64;

    for capability in &plan.visible_tools {
        let decision = policy_compiler::policy_decision_for_capability(session_policy, capability);
        if decision.hidden || decision.action == "deny" {
            hidden_capabilities.push(capability.display_name.clone());
            denied_exposures += 1;
        } else {
            visible_tools.push(capability.clone());
        }
    }

    for capability in &plan.deferred_tools {
        let decision = policy_compiler::policy_decision_for_capability(session_policy, capability);
        if decision.hidden || decision.action == "deny" {
            hidden_capabilities.push(capability.display_name.clone());
            denied_exposures += 1;
        } else {
            deferred_tools.push(capability.clone());
        }
    }

    (
        visible_tools,
        deferred_tools,
        hidden_capabilities,
        denied_exposures,
    )
}

fn resolve_plugin_path(root: &Path, config_home: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        path
    } else if value.starts_with('.') {
        root.join(path)
    } else {
        config_home.join(path)
    }
}

fn build_plugin_manager(
    config_loader: &ConfigLoader,
    root: &Path,
    runtime_config: &runtime::RuntimeConfig,
) -> plugins::PluginManager {
    let plugin_settings = runtime_config.plugins();
    let mut plugin_config = plugins::PluginManagerConfig::new(config_loader.config_home());
    plugin_config.enabled_plugins = plugin_settings.enabled_plugins().clone();
    plugin_config.external_dirs = plugin_settings
        .external_directories()
        .iter()
        .map(|path| resolve_plugin_path(root, config_loader.config_home(), path))
        .collect();
    plugin_config.install_root = plugin_settings
        .install_root()
        .map(|path| resolve_plugin_path(root, config_loader.config_home(), path));
    plugin_config.registry_path = plugin_settings
        .registry_path()
        .map(|path| resolve_plugin_path(root, config_loader.config_home(), path));
    plugin_config.bundled_root = plugin_settings
        .bundled_root()
        .map(|path| resolve_plugin_path(root, config_loader.config_home(), path));
    plugins::PluginManager::new(plugin_config)
}

fn selected_plugin_tools(
    manifest: &actor_manifest::CompiledActorManifest,
    config_loader: &ConfigLoader,
    root: &Path,
    runtime_config: &runtime::RuntimeConfig,
) -> Result<
    (
        Vec<plugins::PluginTool>,
        Vec<RuntimeCapabilityProviderState>,
    ),
    AppError,
> {
    let selected_refs = manifest
        .plugin_capability_refs()
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if selected_refs.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let plugin_manager = build_plugin_manager(config_loader, root, runtime_config);
    let plugin_registry = plugin_manager
        .plugin_registry()
        .map_err(|error| AppError::runtime(error.to_string()))?;
    let tools = plugin_registry
        .aggregated_tools()
        .map_err(|error| AppError::runtime(error.to_string()))?;

    let mut provider_state_summary = Vec::new();
    let mut seen_plugin_ids = BTreeSet::new();
    let mut selected_tools = Vec::new();
    for tool in tools {
        if !selected_refs.contains(tool.definition().name.as_str()) {
            continue;
        }
        if seen_plugin_ids.insert(tool.plugin_id().to_string()) {
            provider_state_summary.push(RuntimeCapabilityProviderState {
                provider_key: tool.plugin_id().to_string(),
                state: "ready".into(),
                detail: Some(format!(
                    "plugin tool `{}` is available",
                    tool.definition().name
                )),
                degraded: false,
            });
        }
        selected_tools.push(tool);
    }

    Ok((selected_tools, provider_state_summary))
}

impl RuntimeAdapter {
    fn effective_config_for_snapshot(&self, snapshot_id: &str) -> Result<Value, AppError> {
        if let Some(value) = self
            .state
            .config_snapshots
            .lock()
            .map_err(|_| AppError::runtime("runtime config snapshots mutex poisoned"))?
            .get(snapshot_id)
            .cloned()
        {
            return Ok(value);
        }

        let value = self
            .open_db()?
            .query_row(
                "SELECT effective_config_json FROM runtime_config_snapshots WHERE id = ?1",
                [snapshot_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?
            .flatten()
            .as_deref()
            .map(serde_json::from_str::<Value>)
            .transpose()?
            .unwrap_or_else(|| json!({}));
        self.state
            .config_snapshots
            .lock()
            .map_err(|_| AppError::runtime("runtime config snapshots mutex poisoned"))?
            .insert(snapshot_id.to_string(), value.clone());
        Ok(value)
    }

    fn runtime_config_from_effective_value(
        &self,
        effective_config: &Value,
    ) -> Result<runtime::RuntimeConfig, AppError> {
        let Some(document) = effective_config.as_object() else {
            return Ok(runtime::RuntimeConfig::empty());
        };

        let runtime_document = document
            .iter()
            .map(|(key, value)| Ok((key.clone(), Self::serde_to_runtime_json(value)?)))
            .collect::<Result<BTreeMap<_, _>, AppError>>()?;
        self.state
            .config_loader
            .load_from_documents(&[ConfigDocument {
                source: ConfigSource::Local,
                path: self.workspace_config_path(),
                exists: true,
                loaded: true,
                document: Some(runtime_document),
            }])
            .map_err(|error| AppError::runtime(error.to_string()))
    }

    pub(crate) async fn project_capability_state_async(
        &self,
        manifest: &actor_manifest::CompiledActorManifest,
        session_policy: &session_policy::CompiledSessionPolicy,
        config_snapshot_id: &str,
        capability_state_ref: impl Into<String>,
        store: &tools::SessionCapabilityStore,
    ) -> Result<CapabilityProjection, AppError> {
        let adapter = self.clone();
        let manifest = manifest.clone();
        let session_policy = session_policy.clone();
        let config_snapshot_id = config_snapshot_id.to_string();
        let capability_state_ref = capability_state_ref.into();
        let store = store.clone();
        tokio::task::spawn_blocking(move || {
            adapter.project_capability_state(
                &manifest,
                &session_policy,
                &config_snapshot_id,
                capability_state_ref,
                &store,
            )
        })
        .await
        .map_err(|error| AppError::runtime(format!("capability projection task failed: {error}")))?
    }

    pub(crate) async fn prepare_capability_runtime_async(
        &self,
        manifest: &actor_manifest::CompiledActorManifest,
        session_policy: &session_policy::CompiledSessionPolicy,
        config_snapshot_id: &str,
        capability_state_ref: impl Into<String>,
        store: &tools::SessionCapabilityStore,
    ) -> Result<PreparedCapabilityRuntime, AppError> {
        let adapter = self.clone();
        let manifest = manifest.clone();
        let session_policy = session_policy.clone();
        let config_snapshot_id = config_snapshot_id.to_string();
        let capability_state_ref = capability_state_ref.into();
        let store = store.clone();
        tokio::task::spawn_blocking(move || {
            adapter.prepare_capability_runtime(
                &manifest,
                &session_policy,
                &config_snapshot_id,
                capability_state_ref,
                &store,
            )
        })
        .await
        .map_err(|error| AppError::runtime(format!("capability runtime task failed: {error}")))?
    }

    pub(crate) fn prepare_capability_runtime(
        &self,
        manifest: &actor_manifest::CompiledActorManifest,
        session_policy: &session_policy::CompiledSessionPolicy,
        config_snapshot_id: &str,
        capability_state_ref: impl Into<String>,
        store: &tools::SessionCapabilityStore,
    ) -> Result<PreparedCapabilityRuntime, AppError> {
        let capability_state_ref = capability_state_ref.into();
        self.persist_capability_store(&capability_state_ref, store)?;
        let effective_config = self.effective_config_for_snapshot(config_snapshot_id)?;
        let runtime_config = self.runtime_config_from_effective_value(&effective_config)?;

        let builtin_tool_names = manifest
            .builtin_tool_keys()
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let allowed_mcp_servers = manifest
            .mcp_server_names()
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let skill_ids = manifest.skill_ids().to_vec();

        let mut provider_state_summary = Vec::new();
        let mut runtime_tools: Vec<tools::RuntimeToolDefinition> = Vec::new();
        let (plugin_tools, mut plugin_provider_state_summary) = selected_plugin_tools(
            manifest,
            &self.state.config_loader,
            self.state.paths.root.as_path(),
            &runtime_config,
        )?;
        provider_state_summary.append(&mut plugin_provider_state_summary);
        let mut provided_capabilities = Vec::new();
        let configured_mcp_servers = runtime_config
            .mcp()
            .servers()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut managed_mcp_runtime = None;

        match tools::ManagedMcpRuntime::new(&runtime_config) {
            Ok(Some(mcp_runtime)) => {
                for projection in mcp_runtime.connection_projections() {
                    if !allowed_mcp_servers.is_empty()
                        && !allowed_mcp_servers.contains(&projection.server_name)
                    {
                        continue;
                    }
                    provider_state_summary.push(RuntimeCapabilityProviderState {
                        provider_key: projection.server_name.clone(),
                        state: provider_state_label(projection.state).to_string(),
                        detail: projection.status_detail.clone(),
                        degraded: matches!(
                            projection.state,
                            tools::CapabilityState::Degraded | tools::CapabilityState::Unavailable
                        ),
                    });
                }
                provided_capabilities.extend(
                    mcp_runtime
                        .provided_capabilities()
                        .into_iter()
                        .filter(|capability| {
                            capability.provider_key.as_deref().is_none_or(|provider| {
                                allowed_mcp_servers.is_empty()
                                    || allowed_mcp_servers.contains(provider)
                            })
                        }),
                );
                for server_name in mcp_runtime.pending_servers().unwrap_or_default() {
                    if (allowed_mcp_servers.is_empty()
                        || allowed_mcp_servers.contains(&server_name))
                        && provider_state_summary
                            .iter()
                            .all(|provider| provider.provider_key != server_name)
                    {
                        provider_state_summary.push(RuntimeCapabilityProviderState {
                            provider_key: server_name,
                            state: "pending".into(),
                            detail: Some("server discovery is pending".into()),
                            degraded: false,
                        });
                    }
                }
                managed_mcp_runtime = Some(Arc::new(Mutex::new(mcp_runtime)));
            }
            Ok(None) => {}
            Err(error) => {
                for server_name in &allowed_mcp_servers {
                    provider_state_summary.push(RuntimeCapabilityProviderState {
                        provider_key: server_name.clone(),
                        state: "degraded".into(),
                        detail: Some(error.to_string()),
                        degraded: true,
                    });
                }
            }
        }

        for server_name in &allowed_mcp_servers {
            if !configured_mcp_servers.contains(server_name)
                && provider_state_summary
                    .iter()
                    .all(|provider| provider.provider_key != *server_name)
            {
                provider_state_summary.push(RuntimeCapabilityProviderState {
                    provider_key: server_name.clone(),
                    state: "unavailable".into(),
                    detail: Some("server is not configured in runtime config".into()),
                    degraded: true,
                });
            }
        }

        let mut planned_tool_names = builtin_tool_names;
        planned_tool_names.extend(
            plugin_tools
                .iter()
                .map(|tool| tool.definition().name.clone()),
        );
        planned_tool_names.extend(runtime_tools.iter().map(|tool| tool.name.clone()));
        planned_tool_names.extend(
            provided_capabilities
                .iter()
                .filter(|capability| {
                    capability.execution_kind == tools::CapabilityExecutionKind::Tool
                })
                .map(|capability| capability.display_name.clone()),
        );

        let provider = tools::CapabilityProvider::from_sources_checked(
            plugin_tools,
            std::mem::take(&mut runtime_tools),
            provided_capabilities.clone(),
            None,
        )
        .map_err(AppError::runtime)?;
        let capability_runtime = tools::CapabilityRuntime::new(provider);
        if let Some(mcp_runtime) = managed_mcp_runtime.as_ref() {
            for capability in &provided_capabilities {
                match capability.execution_kind {
                    tools::CapabilityExecutionKind::PromptSkill => {
                        if let Some(executor_key) = capability.executor_key.clone() {
                            let runtime = Arc::clone(mcp_runtime);
                            capability_runtime.register_prompt_skill_executor(
                                executor_key,
                                move |capability, arguments, _current_dir| {
                                    runtime
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                                        .execute_prompt_skill(capability, arguments)
                                },
                            );
                        }
                    }
                    tools::CapabilityExecutionKind::Resource => {
                        if let Some(executor_key) = capability.executor_key.clone() {
                            let runtime = Arc::clone(mcp_runtime);
                            capability_runtime.register_resource_executor(
                                executor_key,
                                move |capability, _input, _current_dir| {
                                    runtime
                                        .lock()
                                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                                        .read_resource_capability(capability)
                                },
                            );
                        }
                    }
                    tools::CapabilityExecutionKind::Tool => {}
                }
            }
        }

        let session_state = store.snapshot();
        let plan = capability_runtime
            .execution_plan(
                tools::CapabilityPlannerInput::new(Some(&planned_tool_names), Some(&session_state))
                    .with_current_dir(Some(self.state.paths.root.as_path())),
            )
            .map_err(AppError::runtime)?;

        let discoverable_skills = plan
            .discoverable_skills
            .iter()
            .filter(|capability| {
                matches_manifest_skill(capability, &skill_ids, &allowed_mcp_servers)
            })
            .map(|capability| capability.display_name.clone())
            .collect::<Vec<_>>();
        let mut hidden_capabilities = capability_names(&plan.hidden_capabilities);
        hidden_capabilities.extend(
            plan.discoverable_skills
                .iter()
                .filter(|capability| {
                    !matches_manifest_skill(capability, &skill_ids, &allowed_mcp_servers)
                })
                .map(|capability| capability.display_name.clone()),
        );

        let (visible_capabilities, deferred_capabilities, denied_hidden, denied_exposures) =
            filtered_capability_surface(session_policy, &plan);
        hidden_capabilities.extend(denied_hidden);
        hidden_capabilities.sort();
        hidden_capabilities.dedup();

        let pending_auth_challenge = None;
        let auth_state_summary =
            auth_mediation::summarize_auth_state(&provider_state_summary, pending_auth_challenge);
        let compiled_target_allow_count = session_policy
            .target_decisions
            .values()
            .filter(|decision| decision.action == "allow")
            .count() as u64;
        let compiled_target_approval_count = session_policy
            .target_decisions
            .values()
            .filter(|decision| decision.requires_approval)
            .count() as u64;
        let compiled_target_auth_count = session_policy
            .target_decisions
            .values()
            .filter(|decision| decision.requires_auth)
            .count() as u64;
        let policy_decision_summary = RuntimePolicyDecisionSummary {
            allow_count: (visible_capabilities.len() + discoverable_skills.len()) as u64
                + compiled_target_allow_count,
            approval_required_count: compiled_target_approval_count,
            auth_required_count: auth_state_summary.pending_challenge_count
                + compiled_target_auth_count,
            compiled_at: Some(timestamp_now()),
            deferred_capability_count: deferred_capabilities.len() as u64,
            denied_exposure_count: denied_exposures,
            hidden_capability_count: hidden_capabilities.len() as u64,
        };

        let plan_summary = RuntimeCapabilityPlanSummary {
            visible_tools: capability_names(&visible_capabilities),
            deferred_tools: capability_names(&deferred_capabilities),
            discoverable_skills,
            available_resources: capability_names(&plan.available_resources),
            hidden_capabilities,
            discovered_tools: plan.discovered_tools,
            activated_tools: plan.activated_tools,
            exposed_tools: plan.exposed_tools,
            granted_tools: plan.granted_tools,
            pending_tools: plan.pending_tools,
            approved_tools: plan.approved_tools,
            auth_resolved_tools: plan.auth_resolved_tools,
            provider_fallbacks: plan.provider_fallbacks,
        };

        Ok(PreparedCapabilityRuntime {
            capability_runtime,
            visible_capabilities,
            managed_mcp_runtime,
            planned_tool_names,
            projection: CapabilityProjection {
                plan_summary,
                provider_state_summary,
                auth_state_summary,
                policy_decision_summary,
                capability_state_ref,
            },
        })
    }

    pub(crate) fn project_capability_state(
        &self,
        manifest: &actor_manifest::CompiledActorManifest,
        session_policy: &session_policy::CompiledSessionPolicy,
        config_snapshot_id: &str,
        capability_state_ref: impl Into<String>,
        store: &tools::SessionCapabilityStore,
    ) -> Result<CapabilityProjection, AppError> {
        self.prepare_capability_runtime(
            manifest,
            session_policy,
            config_snapshot_id,
            capability_state_ref,
            store,
        )
        .map(|prepared| prepared.projection)
    }
}
