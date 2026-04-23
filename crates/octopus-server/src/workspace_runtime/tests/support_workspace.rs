use super::*;
use std::{fs, path::Path};

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
};
use octopus_core::{
    AccessUserUpsertRequest, CreateProjectDeletionRequestInput, CreateProjectRequest,
    CreateRuntimeSessionInput, CreateTaskInterventionRequest, CreateTaskRequest,
    DataPolicyUpsertRequest, LaunchTaskRequest, LoginRequest, ProjectDeletionRequest,
    ProjectPermissionOverrides, RegisterBootstrapAdminRequest, RerunTaskRequest,
    ReviewProjectDeletionRequestInput, RoleBindingUpsertRequest, RoleUpsertRequest,
    SubmitRuntimeTurnInput, TaskContextBundle, TaskContextRef, UpdateWorkspaceRequest,
    WorkspaceSummary, DEFAULT_PROJECT_ID, DEFAULT_WORKSPACE_ID,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::test_runtime_sdk::test_server_state;

pub(super) const APPROVAL_AGENT_ID: &str = "agent-task-runtime-approval";
pub(super) const APPROVAL_AGENT_REF: &str = "agent:agent-task-runtime-approval";
pub(super) const CHAINED_APPROVAL_TEAM_REF: &str = "team:team-spawn-workflow-approval";

pub(super) fn sample_session() -> SessionRecord {
    SessionRecord {
        id: "sess-1".into(),
        workspace_id: "ws-local".into(),
        user_id: "user-owner".into(),
        client_app_id: "octopus-desktop".into(),
        token: "token".into(),
        status: "active".into(),
        created_at: 1,
        expires_at: None,
    }
}

pub(super) fn avatar_payload() -> octopus_core::AvatarUploadPayload {
    octopus_core::AvatarUploadPayload {
        file_name: "avatar.png".into(),
        content_type: "image/png".into(),
        data_base64: "iVBORw0KGgo=".into(),
        byte_size: 8,
    }
}

pub(super) fn update_request_from_project(project: ProjectRecord) -> UpdateProjectRequest {
    UpdateProjectRequest {
        name: project.name,
        description: project.description,
        status: project.status,
        resource_directory: project.resource_directory,
        owner_user_id: Some(project.owner_user_id),
        member_user_ids: Some(project.member_user_ids),
        permission_overrides: Some(project.permission_overrides),
        leader_agent_id: project.leader_agent_id,
        manager_user_id: project.manager_user_id,
        preset_code: project.preset_code,
        linked_workspace_assets: None,
        assignments: None,
    }
}

pub(super) fn project_scoped_agent_input(
    record: &octopus_core::AgentRecord,
    project_id: &str,
) -> octopus_core::UpsertAgentInput {
    octopus_core::UpsertAgentInput {
        workspace_id: record.workspace_id.clone(),
        project_id: Some(project_id.into()),
        scope: "project".into(),
        name: format!("{} Project Copy", record.name),
        avatar: None,
        remove_avatar: None,
        personality: record.personality.clone(),
        tags: record.tags.clone(),
        prompt: record.prompt.clone(),
        builtin_tool_keys: record.builtin_tool_keys.clone(),
        skill_ids: record.skill_ids.clone(),
        mcp_server_names: record.mcp_server_names.clone(),
        task_domains: record.task_domains.clone(),
        default_model_strategy: Some(record.default_model_strategy.clone()),
        capability_policy: Some(record.capability_policy.clone()),
        permission_envelope: Some(record.permission_envelope.clone()),
        memory_policy: Some(record.memory_policy.clone()),
        delegation_policy: Some(record.delegation_policy.clone()),
        approval_preference: Some(record.approval_preference.clone()),
        output_contract: Some(record.output_contract.clone()),
        shared_capability_policy: Some(record.shared_capability_policy.clone()),
        description: record.description.clone(),
        status: "active".into(),
    }
}

pub(super) fn auth_headers(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {token}")).expect("bearer header"),
    );
    headers.insert(
        "x-workspace-id",
        HeaderValue::from_static(DEFAULT_WORKSPACE_ID),
    );
    headers
}

pub(super) async fn visible_workspace_agent_actor_ref(state: &ServerState) -> String {
    let agent = state
        .services
        .workspace
        .list_agents()
        .await
        .expect("list agents")
        .into_iter()
        .find(|record| {
            record.project_id.is_none()
                && record.status == "active"
                && agent_visible_in_generic_catalog(record)
        })
        .expect("workspace agent");
    format!("agent:{}", agent.id)
}

pub(super) async fn bootstrap_owner(state: &ServerState) -> SessionRecord {
    state
        .services
        .auth
        .register_bootstrap_admin(RegisterBootstrapAdminRequest {
            client_app_id: "octopus-desktop".into(),
            username: "owner".into(),
            display_name: "Owner".into(),
            password: "password123".into(),
            confirm_password: "password123".into(),
            avatar: avatar_payload(),
            workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
            mapped_directory: None,
        })
        .await
        .expect("bootstrap admin")
        .session
}

pub(super) async fn create_user_session(
    state: &ServerState,
    username: &str,
    display_name: &str,
) -> SessionRecord {
    state
        .services
        .access_control
        .create_user(AccessUserUpsertRequest {
            username: username.into(),
            display_name: display_name.into(),
            status: "active".into(),
            password: Some("password123".into()),
            confirm_password: Some("password123".into()),
            reset_password: Some(false),
        })
        .await
        .expect("create user");
    state
        .services
        .auth
        .login(LoginRequest {
            client_app_id: "octopus-desktop".into(),
            username: username.into(),
            password: "password123".into(),
            workspace_id: Some(DEFAULT_WORKSPACE_ID.into()),
        })
        .await
        .expect("login user")
        .session
}

pub(super) async fn seed_task_run(
    state: &ServerState,
    task: &ProjectTaskRecord,
    user_id: &str,
    status: &str,
) -> ProjectTaskRunRecord {
    let started_at = timestamp_now();
    let attention_reasons = match status {
        "waiting_approval" => vec!["needs_approval".into()],
        "waiting_input" => vec!["waiting_input".into()],
        "failed" => vec!["failed".into()],
        _ => Vec::new(),
    };
    let run = ProjectTaskRunRecord {
        id: format!("task-run-{}-{status}", task.id),
        workspace_id: task.workspace_id.clone(),
        project_id: task.project_id.clone(),
        task_id: task.id.clone(),
        trigger_type: "manual".into(),
        status: status.into(),
        session_id: Some(format!("runtime-session-{}-{status}", task.id)),
        conversation_id: Some(format!("conversation-{}-{status}", task.id)),
        runtime_run_id: Some(format!("runtime-run-{}-{status}", task.id)),
        actor_ref: task.default_actor_ref.clone(),
        started_at,
        completed_at: None,
        result_summary: None,
        pending_approval_id: (status == "waiting_approval")
            .then_some(format!("approval-task-run-{}-{status}", task.id)),
        failure_category: None,
        failure_summary: None,
        view_status: if attention_reasons.is_empty() {
            "healthy".into()
        } else {
            "attention".into()
        },
        attention_updated_at: if attention_reasons.is_empty() {
            None
        } else {
            Some(started_at)
        },
        attention_reasons,
        deliverable_refs: Vec::new(),
        artifact_refs: Vec::new(),
        latest_transition: Some(TaskStateTransitionSummary {
            kind: status.into(),
            summary: match status {
                "waiting_approval" => "Task run is waiting for approval.".into(),
                "waiting_input" => "Task run is waiting for input.".into(),
                "failed" => "Task run failed in the runtime.".into(),
                "completed" => "Task run completed in the runtime.".into(),
                _ => "Task run started in the runtime.".into(),
            },
            at: started_at,
            run_id: Some(format!("runtime-run-{}-{status}", task.id)),
        }),
    };
    state
        .services
        .project_tasks
        .save_task_run(run.clone())
        .await
        .expect("save seeded task run");
    state
        .services
        .project_tasks
        .save_task(update_task_record_from_run(task, &run, user_id))
        .await
        .expect("save seeded task projection");
    run
}

pub(super) fn write_runtime_workspace_config(root: &Path) {
    std::env::set_var(
        "OCTOPUS_TEST_ANTHROPIC_API_KEY",
        "test-octopus-server-anthropic-key",
    );
    let path = root.join("config").join("runtime").join("workspace.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("runtime config dir");
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "configuredModels": {
                "opus": {
                    "configuredModelId": "opus",
                    "name": "Opus",
                    "providerId": "anthropic",
                    "modelId": "claude-opus-4-6",
                    "credentialRef": "env:OCTOPUS_TEST_ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                },
                "quota-model": {
                    "configuredModelId": "quota-model",
                    "name": "Quota Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": "env:OCTOPUS_TEST_ANTHROPIC_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("runtime config json"),
    )
    .expect("write runtime config");
}

pub(super) fn write_runtime_workspace_config_with_generation_model(root: &Path) {
    std::env::set_var(
        "OCTOPUS_TEST_OPENAI_API_KEY",
        "test-octopus-server-openai-key",
    );
    let path = root.join("config").join("runtime").join("workspace.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("runtime config dir");
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "providerOverrides": {
                "openai": {
                    "label": "OpenAI",
                    "enabled": true,
                    "surfaces": [
                        {
                            "surface": "conversation",
                            "protocolFamily": "openai_chat",
                            "baseUrl": "https://api.openai.com/v1"
                        },
                        {
                            "surface": "responses",
                            "protocolFamily": "openai_responses",
                            "baseUrl": "https://api.openai.com/v1"
                        }
                    ]
                }
            },
            "modelRegistry": {
                "models": {
                    "gpt-4o-generate": {
                        "providerId": "openai",
                        "label": "GPT-4o Generate",
                        "description": "Workspace-scoped single-shot generation model for runtime route tests.",
                        "track": "generation",
                        "availability": "configured",
                        "defaultPermission": "auto",
                        "surfaceBindings": [
                            {
                                "surface": "responses",
                                "protocolFamily": "openai_responses",
                                "enabled": true,
                                "executionProfile": {
                                    "executionClass": "single_shot_generation",
                                    "toolLoop": false,
                                    "upstreamStreaming": true
                                }
                            }
                        ]
                    }
                }
            },
            "configuredModels": {
                "generation-only-model": {
                    "configuredModelId": "generation-only-model",
                    "name": "Generation Only Model",
                    "providerId": "openai",
                    "modelId": "gpt-4o-generate",
                    "credentialRef": "env:OCTOPUS_TEST_OPENAI_API_KEY",
                    "enabled": true,
                    "source": "workspace"
                }
            }
        }))
        .expect("runtime config json"),
    )
    .expect("write runtime config");
}
