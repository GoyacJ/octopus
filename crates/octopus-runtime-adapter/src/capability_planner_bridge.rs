use super::*;

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone)]
pub(crate) struct CapabilityProjection {
    pub(crate) plan_summary: RuntimeCapabilityPlanSummary,
    pub(crate) provider_state_summary: Vec<RuntimeCapabilityProviderState>,
    pub(crate) capability_state_ref: String,
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
        config_snapshot_id: &str,
        capability_state_ref: impl Into<String>,
        store: &tools::SessionCapabilityStore,
    ) -> Result<CapabilityProjection, AppError> {
        let adapter = self.clone();
        let manifest = manifest.clone();
        let config_snapshot_id = config_snapshot_id.to_string();
        let capability_state_ref = capability_state_ref.into();
        let store = store.clone();
        tokio::task::spawn_blocking(move || {
            adapter.project_capability_state(
                &manifest,
                &config_snapshot_id,
                capability_state_ref,
                &store,
            )
        })
        .await
        .map_err(|error| AppError::runtime(format!("capability projection task failed: {error}")))?
    }

    pub(crate) fn project_capability_state(
        &self,
        manifest: &actor_manifest::CompiledActorManifest,
        config_snapshot_id: &str,
        capability_state_ref: impl Into<String>,
        store: &tools::SessionCapabilityStore,
    ) -> Result<CapabilityProjection, AppError> {
        let capability_state_ref = capability_state_ref.into();
        self.persist_capability_store(&capability_state_ref, store)?;
        let effective_config = self.effective_config_for_snapshot(config_snapshot_id)?;
        let runtime_config = self.runtime_config_from_effective_value(&effective_config)?;

        let allowed_tools = manifest
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
        let mut provided_capabilities = Vec::new();
        let configured_mcp_servers = runtime_config
            .mcp()
            .servers()
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();

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
                            capability.provider_key.as_deref().map_or(true, |provider| {
                                allowed_mcp_servers.is_empty()
                                    || allowed_mcp_servers.contains(provider)
                            })
                        }),
                );
                for server_name in mcp_runtime.pending_servers().unwrap_or_default() {
                    if allowed_mcp_servers.is_empty() || allowed_mcp_servers.contains(&server_name)
                    {
                        if provider_state_summary
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
                }
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

        let provider = tools::CapabilityProvider::from_sources_checked(
            Vec::new(),
            Vec::new(),
            provided_capabilities,
            None,
        )
        .map_err(AppError::runtime)?;
        let capability_runtime = tools::CapabilityRuntime::new(provider);
        let session_state = store.snapshot();
        let plan = capability_runtime
            .execution_plan(
                tools::CapabilityPlannerInput::new(Some(&allowed_tools), Some(&session_state))
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

        Ok(CapabilityProjection {
            plan_summary: RuntimeCapabilityPlanSummary {
                visible_tools: capability_names(&plan.visible_tools),
                deferred_tools: capability_names(&plan.deferred_tools),
                discoverable_skills,
                available_resources: capability_names(&plan.available_resources),
                hidden_capabilities,
                activated_tools: plan.activated_tools,
                granted_tools: plan.granted_tools,
                pending_tools: plan.pending_tools,
                approved_tools: plan.approved_tools,
                auth_resolved_tools: plan.auth_resolved_tools,
                provider_fallbacks: plan.provider_fallbacks,
            },
            provider_state_summary,
            capability_state_ref,
        })
    }
}
