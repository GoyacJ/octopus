use async_trait::async_trait;
use futures::StreamExt;
use octopus_core::{
    AppError, CreateDeliverableVersionInput, CreateRuntimeSessionInput, DeliverableDetail,
    DeliverableVersionContent, DeliverableVersionSummary, PromoteDeliverableInput,
    RebindRuntimeSessionConfiguredModelInput, RuntimeBootstrap, RuntimeEventEnvelope,
    RuntimeExecutionClass, RuntimeSessionDetail, RuntimeSessionSummary,
};
use octopus_sdk::{ContentBlock, EventRange, Message, SessionEvent, StartSessionInput};

use crate::runtime::{RuntimeConfigService, RuntimeSessionService};

use super::{
    runtime_message, runtime_trace, RuntimeSdkBridge, RuntimeSessionMetadata,
    RuntimeSessionProjection,
};

struct ResolvedConfiguredModelBinding {
    configured_model_id: Option<String>,
    configured_model_name: Option<String>,
    runtime_model_id: String,
    started_from_scope_set: Vec<String>,
}

#[async_trait]
impl RuntimeSessionService for RuntimeSdkBridge {
    async fn bootstrap(&self) -> Result<RuntimeBootstrap, AppError> {
        Ok(RuntimeBootstrap {
            provider: octopus_core::ProviderConfig {
                provider_id: "sdk-runtime".into(),
                credential_ref: None,
                base_url: None,
                default_model: Some(self.state.default_model.clone()),
                default_surface: None,
                protocol_family: None,
            },
            sessions: self.list_sessions().await?,
        })
    }

    async fn list_sessions(&self) -> Result<Vec<RuntimeSessionSummary>, AppError> {
        let order = self.state.order.lock().await.clone();
        let sessions = self.state.sessions.lock().await;

        Ok(order
            .iter()
            .filter_map(|session_id| {
                sessions
                    .get(session_id)
                    .map(|entry| entry.detail.summary.clone())
            })
            .collect())
    }

    async fn create_session_with_owner_ceiling(
        &self,
        input: CreateRuntimeSessionInput,
        user_id: &str,
        owner_permission_ceiling: Option<&str>,
    ) -> Result<RuntimeSessionDetail, AppError> {
        let requested_permission_mode = owner_permission_ceiling
            .map(|ceiling| {
                octopus_core::clamp_runtime_permission_mode(
                    &input.execution_permission_mode,
                    ceiling,
                )
            })
            .unwrap_or_else(|| input.execution_permission_mode.clone());
        let permission_mode = runtime_permission_mode(
            Some(requested_permission_mode.as_str()),
            self.state.default_permission_mode,
        )?;
        let resolved = self
            .resolve_configured_model_binding(
                input.project_id.as_deref(),
                user_id,
                input.selected_configured_model_id.as_deref(),
            )
            .await?;
        let now = RuntimeSdkBridge::now();
        let config_snapshot_id = format!("runtime-sdk:session:{}", now);
        let effective_config_hash = format!(
            "runtime-sdk:{}:{}",
            self.state.workspace_id, resolved.runtime_model_id
        );
        let handle = self
            .state
            .runtime
            .start_session(StartSessionInput {
                session_id: None,
                working_dir: self.state.workspace_root.clone(),
                permission_mode,
                model: octopus_sdk::ModelId(resolved.runtime_model_id),
                config_snapshot_id: config_snapshot_id.clone(),
                effective_config_hash: effective_config_hash.clone(),
                token_budget: self.state.default_token_budget,
            })
            .await
            .map_err(RuntimeSdkBridge::runtime_error)?;

        let session_kind = input.session_kind.unwrap_or_else(|| "project".into());
        let conversation_id = if input.conversation_id.is_empty() {
            format!("conv-{}", handle.session_id.0)
        } else {
            input.conversation_id
        };
        let metadata = RuntimeSessionMetadata {
            session_id: handle.session_id.clone(),
            conversation_id,
            project_id: input.project_id.unwrap_or_default(),
            title: input.title,
            session_kind,
            selected_actor_ref: input.selected_actor_ref,
            configured_model_id: resolved.configured_model_id,
            configured_model_name: resolved.configured_model_name,
            runtime_model_id: Some(handle.model.0.clone()),
            permission_mode,
            config_snapshot_id,
            effective_config_hash,
            started_from_scope_set: resolved.started_from_scope_set,
        };
        let run = RuntimeSdkBridge::build_run_snapshot(
            &metadata,
            RuntimeSdkBridge::synthetic_run_id(&metadata.session_id.0),
            "draft",
            "ready",
            now,
            Some("submit_turn".into()),
            None,
        );
        let detail =
            RuntimeSdkBridge::build_session_detail(metadata.clone(), "draft", run.clone(), now);
        let events = self
            .collect_runtime_events(&metadata.session_id, None, &detail, 0)
            .await?;
        let head_event_id = self
            .state
            .runtime
            .snapshot(&metadata.session_id)
            .await
            .map_err(RuntimeSdkBridge::runtime_error)?
            .head_event_id;

        let projection = RuntimeSessionProjection {
            metadata,
            detail: detail.clone(),
            events,
            head_event_id: Some(head_event_id),
        };
        self.upsert_projection(Box::new(projection)).await;
        Ok(detail)
    }

    async fn rebind_session_configured_model(
        &self,
        session_id: &str,
        input: RebindRuntimeSessionConfiguredModelInput,
        user_id: &str,
    ) -> Result<RuntimeSessionDetail, AppError> {
        let mut projection = self.projection(session_id).await?;
        let selected_configured_model_id = input.selected_configured_model_id.trim();
        if selected_configured_model_id.is_empty() {
            return Err(RuntimeSdkBridge::invalid_input(
                "selected configured model id is required",
            ));
        }
        if projection.metadata.configured_model_id.as_deref() == Some(selected_configured_model_id)
        {
            return Ok(projection.detail);
        }

        let project_id = projection.metadata.project_id.clone();
        let resolved = self
            .resolve_configured_model_binding(
                if project_id.trim().is_empty() {
                    None
                } else {
                    Some(project_id.as_str())
                },
                user_id,
                Some(selected_configured_model_id),
            )
            .await?;
        let runtime_session_id = projection.metadata.session_id.clone();
        let existing_snapshot = self
            .state
            .runtime
            .snapshot(&runtime_session_id)
            .await
            .map_err(RuntimeSdkBridge::runtime_error)?;
        let now = RuntimeSdkBridge::now();
        let config_snapshot_id = format!("runtime-sdk:session:{}", now);
        let effective_config_hash = format!(
            "runtime-sdk:{}:{}",
            self.state.workspace_id, resolved.runtime_model_id
        );
        let handle = self
            .state
            .runtime
            .start_session(StartSessionInput {
                session_id: Some(runtime_session_id.clone()),
                working_dir: existing_snapshot.working_dir,
                permission_mode: projection.metadata.permission_mode,
                model: octopus_sdk::ModelId(resolved.runtime_model_id.clone()),
                config_snapshot_id: config_snapshot_id.clone(),
                effective_config_hash: effective_config_hash.clone(),
                token_budget: existing_snapshot.token_budget,
            })
            .await
            .map_err(RuntimeSdkBridge::runtime_error)?;

        projection.metadata.configured_model_id = resolved.configured_model_id;
        projection.metadata.configured_model_name = resolved.configured_model_name;
        projection.metadata.runtime_model_id = Some(handle.model.0.clone());
        projection.metadata.config_snapshot_id = config_snapshot_id.clone();
        projection.metadata.effective_config_hash = effective_config_hash.clone();
        projection.metadata.started_from_scope_set = resolved.started_from_scope_set;

        let session_policy = RuntimeSdkBridge::build_session_policy_snapshot(&projection.metadata);
        projection.detail.summary.updated_at = now;
        projection.detail.summary.config_snapshot_id = config_snapshot_id.clone();
        projection.detail.summary.effective_config_hash = effective_config_hash.clone();
        projection.detail.summary.started_from_scope_set =
            projection.metadata.started_from_scope_set.clone();
        projection.detail.summary.session_policy = session_policy.clone();

        projection.detail.session_policy = session_policy;
        projection.detail.run.updated_at = now;
        projection.detail.run.configured_model_id = projection.metadata.configured_model_id.clone();
        projection.detail.run.configured_model_name =
            projection.metadata.configured_model_name.clone();
        projection.detail.run.model_id = projection.metadata.runtime_model_id.clone();
        projection.detail.run.config_snapshot_id = config_snapshot_id;
        projection.detail.run.effective_config_hash = effective_config_hash;
        projection.detail.run.started_from_scope_set =
            projection.metadata.started_from_scope_set.clone();

        let new_events = self
            .collect_runtime_events(
                &runtime_session_id,
                projection.head_event_id.clone(),
                &projection.detail,
                projection.events.len() as u64,
            )
            .await?;
        let head_event_id = self
            .state
            .runtime
            .snapshot(&runtime_session_id)
            .await
            .map_err(RuntimeSdkBridge::runtime_error)?
            .head_event_id;
        projection.head_event_id = Some(head_event_id);
        projection.events.extend(new_events.iter().cloned());

        let sender = self.session_sender(session_id).await;
        for event in &new_events {
            let _ = sender.send(event.clone());
        }

        let detail = projection.detail.clone();
        self.upsert_projection(Box::new(projection)).await;
        Ok(detail)
    }

    async fn get_session(&self, session_id: &str) -> Result<RuntimeSessionDetail, AppError> {
        Ok(self.projection(session_id).await?.detail)
    }

    async fn get_deliverable_detail(
        &self,
        _deliverable_id: &str,
    ) -> Result<DeliverableDetail, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "deliverable detail is not available in the SDK bridge yet",
        ))
    }

    async fn list_deliverable_versions(
        &self,
        _deliverable_id: &str,
    ) -> Result<Vec<DeliverableVersionSummary>, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "deliverable versions are not available in the SDK bridge yet",
        ))
    }

    async fn get_deliverable_version_content(
        &self,
        _deliverable_id: &str,
        _version: u32,
    ) -> Result<DeliverableVersionContent, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "deliverable version content is not available in the SDK bridge yet",
        ))
    }

    async fn create_deliverable_version(
        &self,
        _deliverable_id: &str,
        _input: CreateDeliverableVersionInput,
    ) -> Result<DeliverableDetail, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "deliverable version creation is not available in the SDK bridge yet",
        ))
    }

    async fn promote_deliverable(
        &self,
        _deliverable_id: &str,
        _input: PromoteDeliverableInput,
    ) -> Result<DeliverableDetail, AppError> {
        Err(RuntimeSdkBridge::invalid_input(
            "deliverable promotion is not available in the SDK bridge yet",
        ))
    }

    async fn list_events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
        let projection = self.projection(session_id).await?;
        Ok(match after {
            Some(after) => projection
                .events
                .into_iter()
                .skip_while(|event| event.id != after)
                .skip(1)
                .collect(),
            None => projection.events,
        })
    }

    async fn delete_session(&self, session_id: &str) -> Result<(), AppError> {
        self.state.sessions.lock().await.remove(session_id);
        self.state
            .order
            .lock()
            .await
            .retain(|candidate| candidate != session_id);
        Ok(())
    }
}

impl RuntimeSdkBridge {
    async fn resolve_configured_model_binding(
        &self,
        project_id: Option<&str>,
        user_id: &str,
        selected_configured_model_id: Option<&str>,
    ) -> Result<ResolvedConfiguredModelBinding, AppError> {
        let effective = match project_id {
            Some(project_id) if !project_id.trim().is_empty() => {
                RuntimeConfigService::get_project_config(self, project_id, user_id).await?
            }
            _ => RuntimeConfigService::get_user_config(self, user_id).await?,
        };
        let snapshot = super::build_catalog_snapshot(self, &effective.effective_config)?;
        let selected_configured_model_id = selected_configured_model_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.state.default_model.clone());
        let selected_model = snapshot
            .configured_models
            .iter()
            .find(|record| record.configured_model_id == selected_configured_model_id)
            .cloned();
        let (configured_model_id, configured_model_name, runtime_model_id) = if let Some(
            configured_model,
        ) = selected_model
        {
            let model = snapshot
                .models
                .iter()
                .find(|model| model.model_id == configured_model.model_id)
                .ok_or_else(|| {
                    AppError::invalid_input(format!(
                        "configured model `{selected_configured_model_id}` is not registered"
                    ))
                })?;
            let supports_runtime = model.surface_bindings.iter().any(|binding| {
                binding.enabled
                    && binding
                        .execution_profile
                        .supports(RuntimeExecutionClass::AgentConversation)
            });
            if !supports_runtime {
                return Err(AppError::invalid_input(format!(
                    "configured model `{selected_configured_model_id}` does not expose a runtime-supported surface"
                )));
            }
            (
                Some(configured_model.configured_model_id.clone()),
                Some(configured_model.name.clone()),
                configured_model.model_id,
            )
        } else {
            (
                Some(selected_configured_model_id.clone()),
                Some(selected_configured_model_id.clone()),
                selected_configured_model_id.clone(),
            )
        };

        Ok(ResolvedConfiguredModelBinding {
            configured_model_id,
            configured_model_name,
            runtime_model_id,
            started_from_scope_set: if effective.sources.is_empty() {
                vec!["workspace".into()]
            } else {
                effective
                    .sources
                    .iter()
                    .map(|source| source.scope.clone())
                    .collect()
            },
        })
    }

    pub(crate) async fn collect_runtime_events(
        &self,
        session_id: &octopus_sdk::SessionId,
        after: Option<octopus_sdk::EventId>,
        detail: &RuntimeSessionDetail,
        sequence_offset: u64,
    ) -> Result<Vec<RuntimeEventEnvelope>, AppError> {
        let mut records = self
            .state
            .runtime
            .events(session_id, EventRange { after, limit: None })
            .await
            .map_err(RuntimeSdkBridge::runtime_error)?;
        let mut events = Vec::new();
        while let Some(record) = records.next().await {
            let record = record.map_err(RuntimeSdkBridge::runtime_error)?;
            let emitted_at = RuntimeSdkBridge::now();
            let sequence = sequence_offset + events.len() as u64;
            let event_id = format!("sdk-evt-{}-{sequence}", session_id.0);
            let event = match record {
                SessionEvent::SessionStarted { .. } => RuntimeEventEnvelope {
                    id: event_id,
                    event_type: "runtime.session.started".into(),
                    kind: Some("session.started".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    run: Some(detail.run.clone()),
                    summary: Some(detail.summary.clone()),
                    ..Default::default()
                },
                SessionEvent::UserMessage(Message { content, .. }) => RuntimeEventEnvelope {
                    id: event_id.clone(),
                    event_type: "runtime.message.user".into(),
                    kind: Some("message.user".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    message: Some(runtime_message(
                        format!("msg-{event_id}"),
                        &detail.summary.id,
                        &detail.summary.conversation_id,
                        "user",
                        "User",
                        flatten_content(&content),
                        emitted_at,
                        detail.run.configured_model_id.clone(),
                        detail.run.model_id.clone(),
                    )),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
                SessionEvent::AssistantMessage(Message { content, .. }) => RuntimeEventEnvelope {
                    id: event_id.clone(),
                    event_type: "runtime.message.assistant".into(),
                    kind: Some("message.assistant".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    message: Some(runtime_message(
                        format!("msg-{event_id}"),
                        &detail.summary.id,
                        &detail.summary.conversation_id,
                        "assistant",
                        "Assistant",
                        flatten_content(&content),
                        emitted_at,
                        detail.run.configured_model_id.clone(),
                        detail.run.model_id.clone(),
                    )),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
                SessionEvent::ToolExecuted {
                    call,
                    name,
                    is_error,
                    duration_ms,
                } => RuntimeEventEnvelope {
                    id: event_id.clone(),
                    event_type: "runtime.tool.executed".into(),
                    kind: Some("tool.executed".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    tool_use_id: Some(call.0),
                    outcome: Some(if is_error { "error" } else { "ok" }.into()),
                    trace: Some(runtime_trace(
                        format!("trace-{event_id}"),
                        &detail.summary.id,
                        &detail.run.id,
                        &detail.summary.conversation_id,
                        "tool",
                        &name,
                        format!("duration_ms={duration_ms} error={is_error}"),
                        emitted_at,
                    )),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
                SessionEvent::Render { block, lifecycle } => RuntimeEventEnvelope {
                    id: event_id.clone(),
                    event_type: "runtime.render.block".into(),
                    kind: Some("render.block".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    trace: Some(runtime_trace(
                        format!("trace-{event_id}"),
                        &detail.summary.id,
                        &detail.run.id,
                        &detail.summary.conversation_id,
                        "render",
                        &format!("{:?}", block.kind),
                        format!("lifecycle={lifecycle:?} payload={}", block.payload),
                        emitted_at,
                    )),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
                SessionEvent::Ask { prompt } => RuntimeEventEnvelope {
                    id: event_id.clone(),
                    event_type: "runtime.ask".into(),
                    kind: Some("ask.prompt".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    trace: Some(runtime_trace(
                        format!("trace-{event_id}"),
                        &detail.summary.id,
                        &detail.run.id,
                        &detail.summary.conversation_id,
                        "ask",
                        &prompt.kind,
                        serde_json::to_string(&prompt).map_err(RuntimeSdkBridge::runtime_error)?,
                        emitted_at,
                    )),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
                SessionEvent::Checkpoint { id, .. } => RuntimeEventEnvelope {
                    id: event_id.clone(),
                    event_type: "runtime.checkpoint.created".into(),
                    kind: Some("checkpoint.created".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    trace: Some(runtime_trace(
                        format!("trace-{event_id}"),
                        &detail.summary.id,
                        &detail.run.id,
                        &detail.summary.conversation_id,
                        "checkpoint",
                        "Checkpoint",
                        id,
                        emitted_at,
                    )),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
                SessionEvent::SessionEnded { reason } => RuntimeEventEnvelope {
                    id: event_id,
                    event_type: "runtime.session.ended".into(),
                    kind: Some("session.ended".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    outcome: Some(format!("{reason:?}")),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
                SessionEvent::SessionPluginsSnapshot { plugins_snapshot } => RuntimeEventEnvelope {
                    id: event_id.clone(),
                    event_type: "runtime.session.plugins_snapshot".into(),
                    kind: Some("session.plugins_snapshot".into()),
                    workspace_id: self.state.workspace_id.clone(),
                    project_id: if detail.summary.project_id.is_empty() {
                        None
                    } else {
                        Some(detail.summary.project_id.clone())
                    },
                    session_id: detail.summary.id.clone(),
                    conversation_id: detail.summary.conversation_id.clone(),
                    run_id: Some(detail.run.id.clone()),
                    emitted_at,
                    sequence,
                    trace: Some(runtime_trace(
                        format!("trace-{event_id}"),
                        &detail.summary.id,
                        &detail.run.id,
                        &detail.summary.conversation_id,
                        "plugins",
                        "Plugins snapshot",
                        serde_json::to_string(&plugins_snapshot)
                            .map_err(RuntimeSdkBridge::runtime_error)?,
                        emitted_at,
                    )),
                    run: Some(detail.run.clone()),
                    ..Default::default()
                },
            };
            events.push(event);
        }

        Ok(events)
    }
}

pub(crate) fn flatten_content(content: &[ContentBlock]) -> String {
    let mut parts = Vec::new();
    for block in content {
        match block {
            ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
                parts.push(text.clone())
            }
            ContentBlock::ToolUse { name, .. } => parts.push(format!("[tool:{name}]")),
            ContentBlock::ToolResult { is_error, .. } => parts.push(
                if *is_error {
                    "[tool-error]"
                } else {
                    "[tool-result]"
                }
                .into(),
            ),
        }
    }
    parts.join("\n")
}

pub(crate) fn runtime_permission_mode(
    value: Option<&str>,
    default_mode: octopus_sdk::PermissionMode,
) -> Result<octopus_sdk::PermissionMode, AppError> {
    match value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "" => Ok(default_mode),
        "default" | "auto" | "ask" | "workspace-write" => Ok(octopus_sdk::PermissionMode::Default),
        "accept_edits" | "accept-edits" => Ok(octopus_sdk::PermissionMode::AcceptEdits),
        "plan" | "readonly" | "read-only" => Ok(octopus_sdk::PermissionMode::Plan),
        "bypass_permissions" | "bypass-permissions" | "bypass" | "danger-full-access" => {
            Ok(octopus_sdk::PermissionMode::BypassPermissions)
        }
        other => Err(RuntimeSdkBridge::invalid_input(format!(
            "unsupported runtime permission mode `{other}` for SDK bridge"
        ))),
    }
}
