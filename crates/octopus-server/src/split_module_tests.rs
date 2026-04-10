use super::*;
use crate::handlers::refresh_browser_host_update_status_with_endpoint;

use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    sync::Arc,
};

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request},
    routing::get,
    Json,
};
use octopus_core::{
    ApiErrorEnvelope, AuditRecord, ClientAppRecord, CreateRuntimeSessionInput,
    CreateWorkspaceUserRequest, InboxItemRecord, KnowledgeEntryRecord, LoginRequest, LoginResponse,
    RegisterWorkspaceOwnerRequest, RegisterWorkspaceOwnerResponse, ResolveRuntimeApprovalInput,
    RuntimeConfigPatch, RuntimeConfigValidationResult, RuntimeEffectiveConfig,
    RuntimeEventEnvelope, RuntimeRunSnapshot, RuntimeSessionDetail, SessionRecord,
    SubmitRuntimeTurnInput,
};
use octopus_infra::{build_infra_bundle, InfraBundle};
use octopus_platform::{ObservationService, PlatformServices};
use octopus_runtime_adapter::{MockRuntimeModelExecutor, RuntimeAdapter};
use rusqlite::Connection;
use serde_json::{json, Value};
use tokio_stream::StreamExt;
use tower::ServiceExt;

#[derive(Clone)]
struct TestHarness {
    router: Router,
    infra: InfraBundle,
    state: ServerState,
}

fn test_harness() -> TestHarness {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    let preferences_path = root.join("shell-preferences.json");
    let workspace_connections_path = root.join("shell-workspace-connections.json");
    std::mem::forget(temp);
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let runtime = Arc::new(RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        Arc::new(MockRuntimeModelExecutor),
    ));
    let services = PlatformServices {
        workspace: infra.workspace.clone(),
        auth: infra.auth.clone(),
        app_registry: infra.app_registry.clone(),
        rbac: infra.rbac.clone(),
        runtime_session: runtime.clone(),
        runtime_execution: runtime.clone(),
        runtime_config: runtime.clone(),
        runtime_registry: runtime.clone(),
        artifact: infra.artifact.clone(),
        inbox: infra.inbox.clone(),
        knowledge: infra.knowledge.clone(),
        observation: infra.observation.clone(),
    };
    let state = ServerState {
        services,
        host_auth_token: "desktop-test-token".into(),
        transport_security: "loopback".into(),
        idempotency_cache: Arc::new(Mutex::new(HashMap::new())),
        host_state: octopus_core::default_host_state("0.1.0-test".into(), true),
        host_connections: octopus_core::default_connection_stubs(),
        host_preferences_path: preferences_path,
        host_workspace_connections_path: workspace_connections_path,
        host_default_preferences: octopus_core::default_preferences(
            octopus_core::DEFAULT_WORKSPACE_ID,
            octopus_core::DEFAULT_PROJECT_ID,
        ),
        backend_connection: DesktopBackendConnection {
            base_url: Some("http://127.0.0.1:43127".into()),
            auth_token: Some("desktop-test-token".into()),
            state: "ready".into(),
            transport: "http".into(),
        },
    };
    let router = build_router(state.clone());

    TestHarness {
        router,
        infra,
        state,
    }
}

async fn decode_json<T: serde::de::DeserializeOwned>(response: Response) -> T {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("json body")
}

async fn register_owner_session(router: &Router, client_app_id: &str) -> SessionRecord {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/register-owner")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&RegisterWorkspaceOwnerRequest {
                        client_app_id: client_app_id.into(),
                        username: "owner".into(),
                        display_name: "Workspace Owner".into(),
                        password: "owner-owner".into(),
                        confirm_password: "owner-owner".into(),
                        avatar: octopus_core::AvatarUploadPayload {
                            file_name: "owner-avatar.png".into(),
                            content_type: "image/png".into(),
                            data_base64: "iVBORw0KGgo=".into(),
                            byte_size: 8,
                        },
                        workspace_id: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RegisterWorkspaceOwnerResponse>(response)
        .await
        .session
}

async fn login_owner_session(router: &Router, client_app_id: &str) -> SessionRecord {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&LoginRequest {
                        client_app_id: client_app_id.into(),
                        username: "owner".into(),
                        password: "owner-owner".into(),
                        workspace_id: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    if response.status() == StatusCode::UNAUTHORIZED {
        return register_owner_session(router, client_app_id).await;
    }
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<LoginResponse>(response).await.session
}

async fn create_member_session(router: &Router, client_app_id: &str) -> SessionRecord {
    let owner = login_owner_session(router, client_app_id).await;
    let create_user_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/workspace/rbac/users")
                .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateWorkspaceUserRequest {
                        username: "member-alpha".into(),
                        display_name: "Member Alpha".into(),
                        status: "active".into(),
                        role_ids: vec!["role-member".into()],
                        scope_project_ids: Vec::new(),
                        avatar: None,
                        use_default_avatar: Some(true),
                        password: Some("member-member".into()),
                        confirm_password: Some("member-member".into()),
                        use_default_password: Some(false),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create_user_response.status(), StatusCode::OK);

    let login_response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&LoginRequest {
                        client_app_id: client_app_id.into(),
                        username: "member-alpha".into(),
                        password: "member-member".into(),
                        workspace_id: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(login_response.status(), StatusCode::OK);
    decode_json::<LoginResponse>(login_response).await.session
}

async fn create_runtime_session(
    router: &Router,
    token: &str,
    title: &str,
    idempotency_key: Option<&str>,
) -> RuntimeSessionDetail {
    let mut request = Request::builder();
    request = request
        .method(Method::POST)
        .uri("/api/v1/runtime/sessions")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(idempotency_key) = idempotency_key {
        request = request.header("Idempotency-Key", idempotency_key);
    }
    let response = router
        .clone()
        .oneshot(
            request
                .body(Body::from(
                    serde_json::to_vec(&CreateRuntimeSessionInput {
                        conversation_id: "conv-1".into(),
                        project_id: "proj-redesign".into(),
                        title: title.into(),
                        session_kind: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeSessionDetail>(response).await
}

async fn create_runtime_session_for_project(
    router: &Router,
    token: &str,
    title: &str,
    project_id: &str,
) -> RuntimeSessionDetail {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/runtime/sessions")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateRuntimeSessionInput {
                        conversation_id: "conv-1".into(),
                        project_id: project_id.into(),
                        title: title.into(),
                        session_kind: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeSessionDetail>(response).await
}

async fn get_runtime_config(router: &Router, token: &str) -> RuntimeEffectiveConfig {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/runtime/config")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeEffectiveConfig>(response).await
}

async fn get_runtime_config_without_session(router: &Router) -> RuntimeEffectiveConfig {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/runtime/config")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeEffectiveConfig>(response).await
}

async fn get_tool_catalog(router: &Router, token: &str) -> Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace/catalog/tool-catalog")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<Value>(response).await
}

async fn patch_tool_catalog_disabled(router: &Router, token: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri("/api/v1/workspace/catalog/tool-catalog/disable")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn create_workspace_skill(router: &Router, token: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/workspace/catalog/skills")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn get_workspace_skill(router: &Router, token: &str, skill_id: &str) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn get_workspace_skill_tree(router: &Router, token: &str, skill_id: &str) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}/tree"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn get_workspace_skill_file(
    router: &Router,
    token: &str,
    skill_id: &str,
    relative_path: &str,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/workspace/catalog/skills/{skill_id}/files/{relative_path}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn update_workspace_skill_file(
    router: &Router,
    token: &str,
    skill_id: &str,
    relative_path: &str,
    body: Value,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!(
                    "/api/v1/workspace/catalog/skills/{skill_id}/files/{relative_path}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn copy_workspace_skill_to_managed(
    router: &Router,
    token: &str,
    skill_id: &str,
    body: Value,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/workspace/catalog/skills/{skill_id}/copy-to-managed"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn import_workspace_skill_folder(router: &Router, token: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/workspace/catalog/skills/import-folder")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn update_workspace_skill(
    router: &Router,
    token: &str,
    skill_id: &str,
    body: Value,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn delete_workspace_skill(router: &Router, token: &str, skill_id: &str) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!("/api/v1/workspace/catalog/skills/{skill_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn create_workspace_mcp_server(router: &Router, token: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/workspace/catalog/mcp-servers")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn get_workspace_mcp_server(router: &Router, token: &str, server_name: &str) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!(
                    "/api/v1/workspace/catalog/mcp-servers/{server_name}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn update_workspace_mcp_server(
    router: &Router,
    token: &str,
    server_name: &str,
    body: Value,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!(
                    "/api/v1/workspace/catalog/mcp-servers/{server_name}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn delete_workspace_mcp_server(router: &Router, token: &str, server_name: &str) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::DELETE)
                .uri(format!(
                    "/api/v1/workspace/catalog/mcp-servers/{server_name}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn copy_workspace_mcp_server_to_managed(
    router: &Router,
    token: &str,
    server_name: &str,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/workspace/catalog/mcp-servers/{server_name}/copy-to-managed"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn get_model_catalog(router: &Router, token: &str) -> Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace/catalog/models")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<Value>(response).await
}

async fn create_project(router: &Router, token: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/projects")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn create_workspace_agent(router: &Router, token: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/workspace/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn list_workspace_agents(router: &Router, token: &str) -> Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<Value>(response).await
}

async fn copy_workspace_agent_from_builtin(
    router: &Router,
    token: &str,
    agent_id: &str,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/workspace/agents/{agent_id}/copy-to-workspace"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn copy_project_agent_from_builtin(
    router: &Router,
    token: &str,
    project_id: &str,
    agent_id: &str,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/projects/{project_id}/agents/{agent_id}/copy-to-project"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn create_workspace_team(router: &Router, token: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/workspace/teams")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn list_workspace_teams(router: &Router, token: &str) -> Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace/teams")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<Value>(response).await
}

async fn copy_workspace_team_from_builtin(
    router: &Router,
    token: &str,
    team_id: &str,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/workspace/teams/{team_id}/copy-to-workspace"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn copy_project_team_from_builtin(
    router: &Router,
    token: &str,
    project_id: &str,
    team_id: &str,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/projects/{project_id}/teams/{team_id}/copy-to-project"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn create_project_agent_link(
    router: &Router,
    token: &str,
    project_id: &str,
    body: Value,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{project_id}/agent-links"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn export_project_agent_bundle(
    router: &Router,
    token: &str,
    project_id: &str,
    body: Value,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{project_id}/agents/export"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn import_project_agent_bundle(
    router: &Router,
    token: &str,
    project_id: &str,
    body: Value,
) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!("/api/v1/projects/{project_id}/agents/import"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn update_project(router: &Router, token: &str, project_id: &str, body: Value) -> Response {
    router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/projects/{project_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response")
}

async fn list_projects(router: &Router, token: &str) -> Value {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/projects")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<Value>(response).await
}

async fn validate_runtime_config_without_session(
    router: &Router,
    patch: RuntimeConfigPatch,
) -> RuntimeConfigValidationResult {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/runtime/config/validate")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeConfigValidationResult>(response).await
}

async fn probe_runtime_configured_model_without_session(
    router: &Router,
    input: RuntimeConfiguredModelProbeInput,
) -> RuntimeConfiguredModelProbeResult {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/runtime/config/configured-models/probe")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&input).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeConfiguredModelProbeResult>(response).await
}

async fn save_runtime_config(
    router: &Router,
    token: &str,
    scope: &str,
    patch: RuntimeConfigPatch,
) -> RuntimeEffectiveConfig {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/runtime/config/scopes/{scope}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeEffectiveConfig>(response).await
}

async fn save_runtime_config_without_session(
    router: &Router,
    scope: &str,
    patch: RuntimeConfigPatch,
) -> RuntimeEffectiveConfig {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/runtime/config/scopes/{scope}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    serde_json::from_slice::<RuntimeEffectiveConfig>(&body).expect("runtime config json")
}

async fn get_project_runtime_config(
    router: &Router,
    token: &str,
    project_id: &str,
) -> RuntimeEffectiveConfig {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeEffectiveConfig>(response).await
}

async fn save_project_runtime_config(
    router: &Router,
    token: &str,
    project_id: &str,
    patch: RuntimeConfigPatch,
) -> RuntimeEffectiveConfig {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&patch).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeEffectiveConfig>(response).await
}

fn expected_catalog_hash_id(prefix: &str, value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{prefix}-{:x}", hasher.finish())
}

async fn submit_turn(
    router: &Router,
    token: &str,
    session_id: &str,
    permission_mode: &str,
    idempotency_key: Option<&str>,
) -> RuntimeRunSnapshot {
    submit_turn_with_input(
        router,
        token,
        session_id,
        SubmitRuntimeTurnInput {
            content: "hello".into(),
            model_id: Some("claude-sonnet-4-5".into()),
            configured_model_id: None,
            permission_mode: permission_mode.into(),
            actor_kind: None,
            actor_id: None,
        },
        idempotency_key,
    )
    .await
}

async fn submit_turn_with_input(
    router: &Router,
    token: &str,
    session_id: &str,
    input: SubmitRuntimeTurnInput,
    idempotency_key: Option<&str>,
) -> RuntimeRunSnapshot {
    let mut request = Request::builder();
    request = request
        .method(Method::POST)
        .uri(format!("/api/v1/runtime/sessions/{session_id}/turns"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(idempotency_key) = idempotency_key {
        request = request.header("Idempotency-Key", idempotency_key);
    }
    let response = router
        .clone()
        .oneshot(
            request
                .body(Body::from(serde_json::to_vec(&input).expect("json")))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeRunSnapshot>(response).await
}

async fn runtime_session_detail(
    router: &Router,
    token: &str,
    session_id: &str,
) -> RuntimeSessionDetail {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/runtime/sessions/{session_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<RuntimeSessionDetail>(response).await
}

async fn runtime_events_after(
    router: &Router,
    token: &str,
    session_id: &str,
    after: &str,
) -> Vec<RuntimeEventEnvelope> {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/runtime/sessions/{session_id}/events?after={after}"
                ))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    decode_json::<Vec<RuntimeEventEnvelope>>(response).await
}

async fn next_sse_event(
    router: &Router,
    token: &str,
    session_id: &str,
    last_event_id: Option<&str>,
    emit_turn: bool,
) -> RuntimeEventEnvelope {
    let mut request = Request::builder();
    request = request
        .uri(format!("/api/v1/runtime/sessions/{session_id}/events"))
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .header(header::ACCEPT, "text/event-stream");
    if let Some(last_event_id) = last_event_id {
        request = request.header("Last-Event-ID", last_event_id);
    }
    let response = router
        .clone()
        .oneshot(request.body(Body::empty()).expect("request"))
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );

    let mut stream = response.into_body().into_data_stream();
    if emit_turn {
        submit_turn(router, token, session_id, "ask", None).await;
    }

    let chunk = tokio::time::timeout(std::time::Duration::from_secs(1), stream.next())
        .await
        .expect("sse event timeout")
        .expect("sse chunk")
        .expect("sse bytes");
    let payload = String::from_utf8(chunk.to_vec()).expect("utf8");
    let data = payload
        .lines()
        .find_map(|line| line.strip_prefix("data:"))
        .map(str::trim)
        .expect("sse data line");
    serde_json::from_str(data).expect("sse envelope")
}

#[tokio::test]
async fn health_route_reports_ready_backend() {
    let response = test_harness()
        .router
        .oneshot(
            Request::builder()
                .uri("/health")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn system_bootstrap_and_login_are_public_entrypoints() {
    let harness = test_harness();
    let bootstrap_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/system/bootstrap")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(bootstrap_response.status(), StatusCode::OK);
    let bootstrap: serde_json::Value = decode_json(bootstrap_response).await;
    assert_eq!(bootstrap["setupRequired"], true);
    assert_eq!(bootstrap["ownerReady"], false);

    let register_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/register-owner")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&RegisterWorkspaceOwnerRequest {
                        client_app_id: "octopus-desktop".into(),
                        username: "owner".into(),
                        display_name: "Workspace Owner".into(),
                        password: "owner-owner".into(),
                        confirm_password: "owner-owner".into(),
                        avatar: octopus_core::AvatarUploadPayload {
                            file_name: "owner-avatar.png".into(),
                            content_type: "image/png".into(),
                            data_base64: "iVBORw0KGgo=".into(),
                            byte_size: 8,
                        },
                        workspace_id: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(register_response.status(), StatusCode::OK);

    let login_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&LoginRequest {
                        client_app_id: "octopus-desktop".into(),
                        username: "owner".into(),
                        password: "owner-owner".into(),
                        workspace_id: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(login_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn project_management_routes_create_and_update_projects() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let create_response = create_project(
        &harness.router,
        &session.token,
        json!({
            "name": "Agent Studio",
            "description": "Project management workspace surface.",
            "assignments": {
                "models": {
                    "configuredModelIds": ["anthropic-primary"],
                    "defaultConfiguredModelId": "anthropic-primary"
                },
                "tools": {
                    "sourceKeys": ["builtin:bash"]
                },
                "agents": {
                    "agentIds": ["agent-architect"],
                    "teamIds": ["team-studio"]
                }
            }
        }),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let created: Value = decode_json(create_response).await;
    assert_eq!(created["name"], "Agent Studio");
    assert_eq!(created["status"], "active");
    assert_eq!(
        created["assignments"]["models"]["configuredModelIds"],
        json!(["anthropic-primary"])
    );
    assert_eq!(
        created["assignments"]["tools"]["sourceKeys"],
        json!(["builtin:bash"])
    );

    let created_id = created["id"].as_str().expect("project id");
    let update_response = update_project(
        &harness.router,
        &session.token,
        created_id,
        json!({
            "name": "Agent Studio",
            "description": "Updated project workspace surface.",
            "status": "archived",
            "assignments": {
                "models": {
                    "configuredModelIds": ["anthropic-alt"],
                    "defaultConfiguredModelId": "anthropic-alt"
                },
                "tools": {
                    "sourceKeys": ["builtin:bash", "mcp:ops"]
                },
                "agents": {
                    "agentIds": ["agent-architect"],
                    "teamIds": []
                }
            }
        }),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: Value = decode_json(update_response).await;
    assert_eq!(updated["status"], "archived");
    assert_eq!(updated["description"], "Updated project workspace surface.");
    assert_eq!(
        updated["assignments"]["models"]["configuredModelIds"],
        json!(["anthropic-alt"])
    );
    assert_eq!(
        updated["assignments"]["tools"]["sourceKeys"],
        json!(["builtin:bash", "mcp:ops"])
    );
}

#[tokio::test]
async fn project_management_routes_reject_blank_names() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let response = create_project(
        &harness.router,
        &session.token,
        json!({
            "name": "   ",
            "description": "Project management workspace surface."
        }),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error: ApiErrorEnvelope = decode_json(response).await;
    assert_eq!(error.error.code, "INVALID_INPUT");
}

#[tokio::test]
async fn project_management_routes_protect_the_last_active_project() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let create_response = create_project(
        &harness.router,
        &session.token,
        json!({
            "name": "Workspace Governance",
            "description": "RBAC, menu policies, and audit automation."
        }),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let created: Value = decode_json(create_response).await;
    let governance_project_id = created["id"].as_str().expect("project id").to_string();

    let first_archive = update_project(
        &harness.router,
        &session.token,
        "proj-redesign",
        json!({
            "name": "Desktop Redesign",
            "description": "Real workspace API migration for the desktop surface.",
            "status": "archived"
        }),
    )
    .await;
    assert_eq!(first_archive.status(), StatusCode::OK);

    let second_archive = update_project(
        &harness.router,
        &session.token,
        &governance_project_id,
        json!({
            "name": "Workspace Governance",
            "description": "RBAC, menu policies, and audit automation.",
            "status": "archived"
        }),
    )
    .await;
    assert_eq!(second_archive.status(), StatusCode::BAD_REQUEST);

    let projects = list_projects(&harness.router, &session.token).await;
    let active_count = projects
        .as_array()
        .expect("projects array")
        .iter()
        .filter(|project| project["status"] == "active")
        .count();
    assert_eq!(active_count, 1);
}

#[tokio::test]
async fn workspace_tool_catalog_returns_runtime_backed_entries() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let skill_dir = harness.infra.paths.root.join("data/skills/help");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: help\ndescription: Helpful local skill.\n---\n\nUse this skill to help.\n",
    )
    .expect("skill file");

    let _ = save_runtime_config_without_session(
        &harness.router,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "mcpServers": {
                    "ops": {
                        "type": "http",
                        "url": "https://ops.example.test/mcp"
                    }
                }
            }),
        },
    )
    .await;

    let payload = get_tool_catalog(&harness.router, &token).await;
    let entries = payload["entries"].as_array().expect("entries array");

    assert!(entries
        .iter()
        .any(|entry| entry["kind"] == "builtin" && entry["name"] == "bash"));
    let skill_entry = entries
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["name"] == "help")
        .expect("skill entry");
    assert_eq!(skill_entry["disabled"], Value::Bool(false));
    assert_eq!(skill_entry["workspaceOwned"], Value::Bool(true));
    assert_eq!(
        skill_entry["relativePath"],
        Value::String("data/skills/help/SKILL.md".into())
    );
    assert_eq!(skill_entry["management"]["canEdit"], Value::Bool(true));
    assert_eq!(skill_entry["management"]["canDelete"], Value::Bool(true));
    assert_eq!(skill_entry["management"]["canDisable"], Value::Bool(true));
    assert!(entries
        .iter()
        .any(|entry| entry["kind"] == "mcp" && entry["serverName"] == "ops"));
    let builtin_entry = entries
        .iter()
        .find(|entry| entry["kind"] == "builtin" && entry["name"] == "bash")
        .expect("builtin entry");
    assert_eq!(builtin_entry["disabled"], Value::Bool(false));
    assert_eq!(builtin_entry["management"]["canEdit"], Value::Bool(false));
    assert_eq!(builtin_entry["management"]["canDelete"], Value::Bool(false));
    assert_eq!(builtin_entry["management"]["canDisable"], Value::Bool(true));
}

#[tokio::test]
async fn workspace_tool_catalog_disable_route_persists_runtime_overrides() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let skill_dir = harness.infra.paths.root.join("data/skills/help");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: help\ndescription: Helpful local skill.\n---\n",
    )
    .expect("skill file");

    let _ = save_runtime_config_without_session(
        &harness.router,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "mcpServers": {
                    "ops": {
                        "type": "http",
                        "url": "https://ops.example.test/mcp"
                    }
                }
            }),
        },
    )
    .await;

    let before = get_tool_catalog(&harness.router, &token).await;
    let entries = before["entries"].as_array().expect("entries array");
    let builtin_source_key = entries
        .iter()
        .find(|entry| entry["kind"] == "builtin" && entry["name"] == "bash")
        .and_then(|entry| entry["sourceKey"].as_str())
        .expect("builtin source key")
        .to_string();
    let skill_source_key = entries
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["name"] == "help")
        .and_then(|entry| entry["sourceKey"].as_str())
        .expect("skill source key")
        .to_string();
    let mcp_source_key = entries
        .iter()
        .find(|entry| entry["kind"] == "mcp" && entry["serverName"] == "ops")
        .and_then(|entry| entry["sourceKey"].as_str())
        .expect("mcp source key")
        .to_string();

    for source_key in [&builtin_source_key, &skill_source_key, &mcp_source_key] {
        let response = patch_tool_catalog_disabled(
            &harness.router,
            &token,
            json!({
                "sourceKey": source_key,
                "disabled": true
            }),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    let after = get_tool_catalog(&harness.router, &token).await;
    let entries = after["entries"].as_array().expect("entries array");
    for source_key in [&builtin_source_key, &skill_source_key, &mcp_source_key] {
        let entry = entries
            .iter()
            .find(|entry| entry["sourceKey"] == Value::String(source_key.clone()))
            .expect("updated entry");
        assert_eq!(entry["disabled"], Value::Bool(true));
    }

    let written = std::fs::read_to_string(
        harness
            .infra
            .paths
            .runtime_config_dir
            .join("workspace.json"),
    )
    .expect("workspace config");
    assert!(written.contains("\"toolCatalog\""));
    assert!(written.contains(&builtin_source_key));
    assert!(written.contains(&skill_source_key));
    assert!(written.contains(&mcp_source_key));
}

#[tokio::test]
async fn workspace_skill_routes_create_update_and_delete_workspace_owned_skills() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let create_response = create_workspace_skill(
            &harness.router,
            &token,
            json!({
                "slug": "ops-helper",
                "content": "---\nname: ops-helper\ndescription: First version.\n---\n\nUse this skill to help ops.\n"
            }),
        )
        .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let created: Value = decode_json(create_response).await;
    let skill_id = created["id"].as_str().expect("skill id").to_string();
    assert_eq!(created["name"], "ops-helper");
    assert_eq!(created["workspaceOwned"], Value::Bool(true));

    let skill_path = harness
        .infra
        .paths
        .root
        .join("data/skills/ops-helper/SKILL.md");
    assert!(skill_path.exists());

    let get_response = get_workspace_skill(&harness.router, &token, &skill_id).await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let fetched: Value = decode_json(get_response).await;
    assert!(fetched["tree"].is_array());

    let update_response = update_workspace_skill_file(
            &harness.router,
            &token,
            &skill_id,
            "SKILL.md",
            json!({
                "content": "---\nname: ops-helper\ndescription: Updated version.\n---\n\nUse this skill to help ops better.\n"
            }),
        )
        .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: Value = decode_json(update_response).await;
    assert_eq!(updated["content"], Value::String("---\nname: ops-helper\ndescription: Updated version.\n---\n\nUse this skill to help ops better.\n".into()));

    let written = std::fs::read_to_string(&skill_path).expect("skill file");
    assert!(written.contains("Updated version."));

    let delete_response = delete_workspace_skill(&harness.router, &token, &skill_id).await;
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);
    assert!(!skill_path.exists());
}

#[tokio::test]
async fn workspace_skill_routes_expose_tree_and_file_documents_for_managed_skills() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let skill_dir = harness.infra.paths.root.join("data/skills/ops-helper");
    std::fs::create_dir_all(skill_dir.join("templates")).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: ops-helper\ndescription: Managed skill.\n---\n",
    )
    .expect("skill file");
    std::fs::write(skill_dir.join("templates/prompt.md"), "# Prompt\n").expect("text file");
    std::fs::write(skill_dir.join("icon.bin"), [0_u8, 159, 146, 150]).expect("binary file");

    let payload = get_tool_catalog(&harness.router, &token).await;
    let skill_id = payload["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["name"] == "ops-helper")
        .and_then(|entry| entry["id"].as_str())
        .expect("skill id")
        .to_string();

    let tree_response = get_workspace_skill_tree(&harness.router, &token, &skill_id).await;
    assert_eq!(tree_response.status(), StatusCode::OK);
    let tree: Value = decode_json(tree_response).await;
    let nodes = tree["tree"].as_array().expect("tree nodes");
    assert!(nodes.iter().any(|node| node["path"] == "SKILL.md"));
    assert!(nodes.iter().any(|node| node["path"] == "templates"));

    let file_response =
        get_workspace_skill_file(&harness.router, &token, &skill_id, "templates/prompt.md").await;
    assert_eq!(file_response.status(), StatusCode::OK);
    let file: Value = decode_json(file_response).await;
    assert_eq!(file["path"], "templates/prompt.md");
    assert_eq!(file["isText"], Value::Bool(true));
    assert_eq!(file["content"], "# Prompt\n");

    let binary_response =
        get_workspace_skill_file(&harness.router, &token, &skill_id, "icon.bin").await;
    assert_eq!(binary_response.status(), StatusCode::OK);
    let binary: Value = decode_json(binary_response).await;
    assert_eq!(binary["isText"], Value::Bool(false));
    assert_eq!(binary["content"], Value::Null);
}

#[tokio::test]
async fn workspace_skill_routes_reject_mutating_non_workspace_owned_entries() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let skill_dir = harness
        .infra
        .paths
        .root
        .join(".claude/skills/external-help");
    std::fs::create_dir_all(&skill_dir).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: external-help\ndescription: External skill.\n---\n",
    )
    .expect("skill file");

    let payload = get_tool_catalog(&harness.router, &token).await;
    let skill_id = payload["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["name"] == "external-help")
        .and_then(|entry| entry["id"].as_str())
        .expect("external skill id")
        .to_string();

    let update_response = update_workspace_skill(
        &harness.router,
        &token,
        &skill_id,
        json!({
            "content": "---\nname: external-help\ndescription: Updated.\n---\n"
        }),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::BAD_REQUEST);

    let delete_response = delete_workspace_skill(&harness.router, &token, &skill_id).await;
    assert_eq!(delete_response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn workspace_skill_routes_copy_external_skill_to_managed_root() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let skill_dir = harness
        .infra
        .paths
        .root
        .join(".claude/skills/external-help");
    std::fs::create_dir_all(skill_dir.join("templates")).expect("skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: external-help\ndescription: External skill.\n---\n",
    )
    .expect("skill file");
    std::fs::write(skill_dir.join("templates/prompt.md"), "hello\n").expect("template file");

    let payload = get_tool_catalog(&harness.router, &token).await;
    let skill_id = payload["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["name"] == "external-help")
        .and_then(|entry| entry["id"].as_str())
        .expect("external skill id")
        .to_string();

    let response = copy_workspace_skill_to_managed(
        &harness.router,
        &token,
        &skill_id,
        json!({ "slug": "external-help-copy" }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let copied: Value = decode_json(response).await;
    assert_eq!(copied["workspaceOwned"], Value::Bool(true));
    assert_eq!(
        copied["relativePath"],
        Value::String("data/skills/external-help-copy/SKILL.md".into())
    );

    let copied_skill_root = harness
        .infra
        .paths
        .root
        .join("data/skills/external-help-copy");
    assert!(copied_skill_root.join("SKILL.md").exists());
    assert!(copied_skill_root.join("templates/prompt.md").exists());
}

#[tokio::test]
async fn workspace_tool_catalog_surfaces_builtin_skills_as_readonly_and_copyable() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let payload = get_tool_catalog(&harness.router, &token).await;
    let builtin_skill_id = expected_catalog_hash_id(
        "skill",
        "builtin-assets/skills/financial-calculator/SKILL.md",
    );
    let builtin_skill_entry = payload["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["id"] == builtin_skill_id)
        .cloned()
        .expect("builtin financial calculator skill entry");

    assert_eq!(builtin_skill_entry["name"], "financial-calculator");
    assert_eq!(builtin_skill_entry["ownerScope"], "builtin");
    assert_eq!(builtin_skill_entry["ownerLabel"], "Builtin");
    assert_eq!(builtin_skill_entry["management"]["canEdit"], Value::Bool(false));
    assert_eq!(
        builtin_skill_entry["management"]["canDelete"],
        Value::Bool(false)
    );

    let get_response = get_workspace_skill(&harness.router, &token, &builtin_skill_id).await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let document: Value = decode_json(get_response).await;
    assert_eq!(document["workspaceOwned"], Value::Bool(false));
    assert_eq!(
        document["displayPath"],
        Value::String("builtin-assets/skills/financial-calculator/SKILL.md".into())
    );

    let file_response = get_workspace_skill_file(
        &harness.router,
        &token,
        &builtin_skill_id,
        "scripts/calculate.py",
    )
    .await;
    assert_eq!(file_response.status(), StatusCode::OK);
    let file_document: Value = decode_json(file_response).await;
    assert_eq!(file_document["readonly"], Value::Bool(true));
    assert!(file_document["content"]
        .as_str()
        .expect("builtin skill file content")
        .contains("def "));

    let copy_response = copy_workspace_skill_to_managed(
        &harness.router,
        &token,
        &builtin_skill_id,
        json!({ "slug": "financial-calculator-copy" }),
    )
    .await;
    assert_eq!(copy_response.status(), StatusCode::OK);
    let copied: Value = decode_json(copy_response).await;
    assert_eq!(copied["workspaceOwned"], Value::Bool(true));
    assert_eq!(
        copied["relativePath"],
        Value::String("data/skills/financial-calculator-copy/SKILL.md".into())
    );
    assert!(harness
        .infra
        .paths
        .root
        .join("data/skills/financial-calculator-copy/scripts/calculate.py")
        .exists());
}

#[tokio::test]
async fn workspace_agent_and_team_lists_include_builtin_templates() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let agents = list_workspace_agents(&harness.router, &token).await;
    let builtin_agent = agents
        .as_array()
        .expect("agents")
        .iter()
        .find(|entry| {
            entry["name"] == "财务分析师"
                && entry["integrationSource"]["kind"] == "builtin-template"
        })
        .cloned()
        .expect("builtin agent template");
    assert_eq!(
        builtin_agent["integrationSource"]["sourceId"],
        Value::String("财务分析师".into())
    );
    assert_eq!(builtin_agent["projectId"], Value::Null);

    let teams = list_workspace_teams(&harness.router, &token).await;
    let builtin_team = teams
        .as_array()
        .expect("teams")
        .iter()
        .find(|entry| {
            entry["name"] == "财务部" && entry["integrationSource"]["kind"] == "builtin-template"
        })
        .cloned()
        .expect("builtin team template");
    assert_eq!(
        builtin_team["integrationSource"]["sourceId"],
        Value::String("财务部".into())
    );
    assert_eq!(builtin_team["projectId"], Value::Null);
}

#[tokio::test]
async fn workspace_builtin_agent_copy_imports_workspace_assets() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let agents = list_workspace_agents(&harness.router, &token).await;
    let builtin_agent_id = agents
        .as_array()
        .expect("agents")
        .iter()
        .find(|entry| {
            entry["name"] == "财务分析师"
                && entry["integrationSource"]["kind"] == "builtin-template"
        })
        .and_then(|entry| entry["id"].as_str())
        .expect("builtin agent id")
        .to_string();

    let response =
        copy_workspace_agent_from_builtin(&harness.router, &token, &builtin_agent_id).await;
    assert_eq!(response.status(), StatusCode::OK);
    let copied_result: Value = decode_json(response).await;
    assert_eq!(copied_result["agentCount"], Value::from(1));
    assert_eq!(copied_result["teamCount"], Value::from(0));
    assert!(copied_result["skillCount"].as_u64().unwrap_or(0) >= 1);
    assert!(copied_result["mcpCount"].as_u64().unwrap_or(0) >= 1);

    let agents = list_workspace_agents(&harness.router, &token).await;
    assert!(agents
        .as_array()
        .expect("agents")
        .iter()
        .any(|entry| entry["name"] == "财务分析师" && entry.get("integrationSource").is_none()));

    assert!(harness
        .infra
        .paths
        .root
        .join("data/skills/financial-calculator/SKILL.md")
        .exists());

    let workspace_runtime_path = harness
        .infra
        .paths
        .root
        .join("config/runtime/workspace.json");
    let workspace_runtime = fs::read_to_string(workspace_runtime_path).expect("workspace runtime");
    assert!(workspace_runtime.contains("\"finance-data\""));
}

#[tokio::test]
async fn project_builtin_team_copy_imports_project_scoped_assets() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let created_project = create_project(
        &harness.router,
        &token,
        json!({
            "name": "Builtin Asset Project",
            "description": "Copies builtin templates into the project scope.",
        }),
    )
    .await;
    assert_eq!(created_project.status(), StatusCode::OK);
    let created_project: Value = decode_json(created_project).await;
    let project_id = created_project["id"]
        .as_str()
        .expect("project id")
        .to_string();

    let teams = list_workspace_teams(&harness.router, &token).await;
    let builtin_team_id = teams
        .as_array()
        .expect("teams")
        .iter()
        .find(|entry| {
            entry["name"] == "财务部" && entry["integrationSource"]["kind"] == "builtin-template"
        })
        .and_then(|entry| entry["id"].as_str())
        .expect("builtin team id")
        .to_string();

    let response =
        copy_project_team_from_builtin(&harness.router, &token, &project_id, &builtin_team_id)
            .await;
    assert_eq!(response.status(), StatusCode::OK);
    let copied_result: Value = decode_json(response).await;
    assert_eq!(copied_result["teamCount"], Value::from(1));
    assert!(copied_result["agentCount"].as_u64().unwrap_or(0) >= 2);
    assert!(copied_result["skillCount"].as_u64().unwrap_or(0) >= 1);
    assert!(copied_result["mcpCount"].as_u64().unwrap_or(0) >= 1);

    let teams = list_workspace_teams(&harness.router, &token).await;
    assert!(teams.as_array().expect("teams").iter().any(|entry| {
        entry["name"] == "财务部"
            && entry["projectId"] == Value::String(project_id.clone())
            && entry.get("integrationSource").is_none()
    }));

    assert!(harness
        .infra
        .paths
        .root
        .join(format!("data/projects/{project_id}/skills/financial-calculator/SKILL.md"))
        .exists());

    let project_runtime = fs::read_to_string(
        harness
            .infra
            .paths
            .root
            .join(format!("config/runtime/projects/{project_id}.json")),
    )
    .expect("project runtime");
    assert!(project_runtime.contains("\"finance-data\""));
}

#[tokio::test]
async fn workspace_skill_routes_import_folder_into_managed_root() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let response = import_workspace_skill_folder(
            &harness.router,
            &token,
            json!({
                "slug": "imported-skill",
                "files": [
                    {
                        "relativePath": "wrapped/SKILL.md",
                        "fileName": "SKILL.md",
                        "contentType": "text/markdown",
                        "dataBase64": "LS0tCm5hbWU6IGltcG9ydGVkLXNraWxsCmRlc2NyaXB0aW9uOiBJbXBvcnRlZCBza2lsbC4KLS0tCg==",
                        "byteSize": 58
                    },
                    {
                        "relativePath": "wrapped/templates/prompt.md",
                        "fileName": "prompt.md",
                        "contentType": "text/markdown",
                        "dataBase64": "IyBQcm9tcHQK",
                        "byteSize": 9
                    }
                ]
            }),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let imported: Value = decode_json(response).await;
    assert_eq!(
        imported["relativePath"],
        Value::String("data/skills/imported-skill/SKILL.md".into())
    );
    assert!(harness
        .infra
        .paths
        .root
        .join("data/skills/imported-skill/templates/prompt.md")
        .exists());
}

#[tokio::test]
async fn workspace_mcp_routes_create_update_and_delete_servers() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let create_response = create_workspace_mcp_server(
        &harness.router,
        &token,
        json!({
            "serverName": "ops",
            "config": {
                "type": "http",
                "url": "https://ops.example.test/mcp"
            }
        }),
    )
    .await;
    assert_eq!(create_response.status(), StatusCode::OK);
    let created: Value = decode_json(create_response).await;
    assert_eq!(created["serverName"], "ops");

    let get_response = get_workspace_mcp_server(&harness.router, &token, "ops").await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let fetched: Value = decode_json(get_response).await;
    assert_eq!(fetched["config"]["url"], "https://ops.example.test/mcp");

    let update_response = update_workspace_mcp_server(
        &harness.router,
        &token,
        "ops",
        json!({
            "serverName": "ops",
            "config": {
                "type": "http",
                "url": "https://ops-alt.example.test/mcp"
            }
        }),
    )
    .await;
    assert_eq!(update_response.status(), StatusCode::OK);
    let updated: Value = decode_json(update_response).await;
    assert_eq!(updated["config"]["url"], "https://ops-alt.example.test/mcp");

    let delete_response = delete_workspace_mcp_server(&harness.router, &token, "ops").await;
    assert_eq!(delete_response.status(), StatusCode::NO_CONTENT);

    let payload = get_tool_catalog(&harness.router, &token).await;
    assert!(!payload["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .any(|entry| entry["kind"] == "mcp" && entry["serverName"] == "ops"));
}

#[tokio::test]
async fn workspace_tool_catalog_surfaces_builtin_mcp_and_supports_copy_to_managed() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let payload = get_tool_catalog(&harness.router, &token).await;
    let builtin_mcp = payload["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["kind"] == "mcp" && entry["serverName"] == "finance-data")
        .cloned()
        .expect("builtin finance-data mcp entry");
    assert_eq!(builtin_mcp["ownerScope"], "builtin");
    assert_eq!(builtin_mcp["scope"], "builtin");
    assert_eq!(builtin_mcp["management"]["canEdit"], Value::Bool(false));
    assert_eq!(builtin_mcp["management"]["canDelete"], Value::Bool(false));

    let get_response = get_workspace_mcp_server(&harness.router, &token, "finance-data").await;
    assert_eq!(get_response.status(), StatusCode::OK);
    let document: Value = decode_json(get_response).await;
    assert_eq!(document["scope"], "builtin");
    assert_eq!(document["serverName"], "finance-data");

    let copy_response =
        copy_workspace_mcp_server_to_managed(&harness.router, &token, "finance-data").await;
    assert_eq!(copy_response.status(), StatusCode::OK);
    let copied: Value = decode_json(copy_response).await;
    assert_eq!(copied["scope"], "workspace");
    assert_eq!(copied["serverName"], "finance-data");

    let workspace_runtime_path = harness
        .infra
        .paths
        .root
        .join("config/runtime/workspace.json");
    let workspace_runtime = fs::read_to_string(workspace_runtime_path).expect("workspace runtime");
    assert!(workspace_runtime.contains("\"finance-data\""));

    let payload = get_tool_catalog(&harness.router, &token).await;
    let managed_mcp = payload["entries"]
        .as_array()
        .expect("entries")
        .iter()
        .find(|entry| entry["kind"] == "mcp" && entry["serverName"] == "finance-data")
        .cloned()
        .expect("managed finance-data mcp entry");
    assert_eq!(managed_mcp["ownerScope"], "workspace");
    assert_eq!(managed_mcp["scope"], "workspace");
}

#[tokio::test]
async fn workspace_tool_catalog_includes_project_owned_assets_and_consumers() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let created_project = create_project(
        &harness.router,
        &token,
        json!({
            "name": "Atlas Project",
            "description": "Project-owned agent assets.",
        }),
    )
    .await;
    assert_eq!(created_project.status(), StatusCode::OK);
    let created_project: Value = decode_json(created_project).await;
    let project_id = created_project["id"]
        .as_str()
        .expect("project id")
        .to_string();

    let workspace_skill_response = create_workspace_skill(
        &harness.router,
        &token,
        json!({
            "slug": "workspace-managed-skill",
            "content": "---\nname: Workspace Managed Skill\ndescription: Shared workspace skill.\n---\n\n# Workspace skill\n",
        }),
    )
    .await;
    assert_eq!(workspace_skill_response.status(), StatusCode::OK);
    let workspace_skill: Value = decode_json(workspace_skill_response).await;
    let workspace_skill_id = workspace_skill["id"]
        .as_str()
        .expect("workspace skill id")
        .to_string();

    let project_skill_slug = "project-plan-skill";
    let project_skill_root = harness
        .infra
        .paths
        .project_skills_root(&project_id)
        .join(project_skill_slug);
    fs::create_dir_all(&project_skill_root).expect("create project skill root");
    fs::write(
        project_skill_root.join("SKILL.md"),
        "---\nname: Project Plan Skill\ndescription: Project-scoped planning skill.\n---\n\n# Project plan skill\n",
    )
    .expect("write project skill");
    let project_skill_id = expected_catalog_hash_id(
        "skill",
        &format!("data/projects/{project_id}/skills/{project_skill_slug}/SKILL.md"),
    );

    let workspace_mcp_response = create_workspace_mcp_server(
        &harness.router,
        &token,
        json!({
            "serverName": "workspace-ops",
            "config": {
                "type": "http",
                "url": "https://workspace.example.test/mcp"
            }
        }),
    )
    .await;
    assert_eq!(workspace_mcp_response.status(), StatusCode::OK);

    let _project_runtime = save_project_runtime_config(
        &harness.router,
        &token,
        &project_id,
        RuntimeConfigPatch {
            scope: "project".into(),
            patch: json!({
                "mcpServers": {
                    "project-ops": {
                        "type": "http",
                        "url": "https://project.example.test/mcp"
                    }
                }
            }),
        },
    )
    .await;

    let workspace_agent_response = create_workspace_agent(
        &harness.router,
        &token,
        json!({
            "workspaceId": "ws-local",
            "projectId": Value::Null,
            "scope": "workspace",
            "name": "Workspace Analyst",
            "personality": "Owns workspace-level operations.",
            "tags": ["ops"],
            "prompt": "Handle workspace tasks.",
            "builtinToolKeys": ["bash"],
            "skillIds": [workspace_skill_id],
            "mcpServerNames": ["workspace-ops"],
            "description": "Workspace agent.",
            "status": "active"
        }),
    )
    .await;
    assert_eq!(workspace_agent_response.status(), StatusCode::OK);
    let workspace_agent: Value = decode_json(workspace_agent_response).await;
    let workspace_agent_id = workspace_agent["id"]
        .as_str()
        .expect("workspace agent id")
        .to_string();

    let workspace_team_response = create_workspace_team(
        &harness.router,
        &token,
        json!({
            "workspaceId": "ws-local",
            "projectId": Value::Null,
            "scope": "workspace",
            "name": "Workspace Crew",
            "personality": "Coordinates workspace delivery.",
            "tags": ["ops"],
            "prompt": "Coordinate workspace delivery.",
            "builtinToolKeys": ["bash"],
            "skillIds": [workspace_skill["id"].as_str().expect("workspace skill id")],
            "mcpServerNames": ["workspace-ops"],
            "leaderAgentId": workspace_agent_id,
            "memberAgentIds": [workspace_agent["id"].as_str().expect("workspace agent id")],
            "description": "Workspace team.",
            "status": "active"
        }),
    )
    .await;
    assert_eq!(workspace_team_response.status(), StatusCode::OK);

    let project_agent_response = create_workspace_agent(
        &harness.router,
        &token,
        json!({
            "workspaceId": "ws-local",
            "projectId": project_id,
            "scope": "project",
            "name": "Project Planner",
            "personality": "Owns project planning.",
            "tags": ["planning"],
            "prompt": "Plan the project.",
            "builtinToolKeys": ["bash"],
            "skillIds": [project_skill_id],
            "mcpServerNames": ["project-ops"],
            "description": "Project agent.",
            "status": "active"
        }),
    )
    .await;
    assert_eq!(project_agent_response.status(), StatusCode::OK);
    let project_agent: Value = decode_json(project_agent_response).await;

    let project_team_response = create_workspace_team(
        &harness.router,
        &token,
        json!({
            "workspaceId": "ws-local",
            "projectId": project_id,
            "scope": "project",
            "name": "Project Delivery",
            "personality": "Coordinates project delivery.",
            "tags": ["delivery"],
            "prompt": "Coordinate project delivery.",
            "builtinToolKeys": ["bash"],
            "skillIds": [project_skill_id],
            "mcpServerNames": ["project-ops"],
            "leaderAgentId": project_agent["id"].as_str().expect("project agent id"),
            "memberAgentIds": [project_agent["id"].as_str().expect("project agent id")],
            "description": "Project team.",
            "status": "active"
        }),
    )
    .await;
    assert_eq!(project_team_response.status(), StatusCode::OK);

    let payload = get_tool_catalog(&harness.router, &token).await;
    let entries = payload["entries"].as_array().expect("entries");

    let builtin = entries
        .iter()
        .find(|entry| entry["kind"] == "builtin" && entry["builtinKey"] == "bash")
        .expect("builtin bash entry");
    assert_eq!(builtin["ownerScope"], "builtin");
    assert!(builtin["consumers"].is_array(), "builtin entry must expose consumers");
    let builtin_consumers = builtin["consumers"].as_array().expect("builtin consumers");
    assert!(builtin_consumers.iter().any(|consumer| {
        consumer["kind"] == "agent"
            && consumer["name"] == "Workspace Analyst"
            && consumer["scope"] == "workspace"
    }));
    assert!(builtin_consumers.iter().any(|consumer| {
        consumer["kind"] == "team"
            && consumer["name"] == "Project Delivery"
            && consumer["scope"] == "project"
            && consumer["ownerId"] == project_id
            && consumer["ownerLabel"] == "Atlas Project"
    }));

    let workspace_skill_entry = entries
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["id"] == workspace_skill["id"])
        .expect("workspace skill entry");
    assert_eq!(workspace_skill_entry["ownerScope"], "workspace");
    assert!(workspace_skill_entry["consumers"].is_array());
    assert!(workspace_skill_entry["consumers"]
        .as_array()
        .expect("workspace skill consumers")
        .iter()
        .any(|consumer| consumer["name"] == "Workspace Crew" && consumer["kind"] == "team"));

    let project_skill_entry = entries
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["id"] == project_skill_id)
        .expect("project skill entry");
    assert_eq!(project_skill_entry["ownerScope"], "project");
    assert_eq!(project_skill_entry["ownerId"], project_id);
    assert_eq!(project_skill_entry["ownerLabel"], "Atlas Project");
    assert!(project_skill_entry["consumers"].is_array());
    assert!(project_skill_entry["consumers"]
        .as_array()
        .expect("project skill consumers")
        .iter()
        .any(|consumer| consumer["name"] == "Project Planner" && consumer["kind"] == "agent"));

    let workspace_mcp_entry = entries
        .iter()
        .find(|entry| entry["kind"] == "mcp" && entry["serverName"] == "workspace-ops")
        .expect("workspace mcp entry");
    assert_eq!(workspace_mcp_entry["ownerScope"], "workspace");
    assert!(workspace_mcp_entry["consumers"].is_array());
    assert!(workspace_mcp_entry["consumers"]
        .as_array()
        .expect("workspace mcp consumers")
        .iter()
        .any(|consumer| consumer["name"] == "Workspace Analyst"));

    let project_mcp_entry = entries
        .iter()
        .find(|entry| entry["kind"] == "mcp" && entry["serverName"] == "project-ops")
        .expect("project mcp entry");
    assert_eq!(project_mcp_entry["ownerScope"], "project");
    assert_eq!(project_mcp_entry["ownerId"], project_id);
    assert_eq!(project_mcp_entry["ownerLabel"], "Atlas Project");
    assert!(project_mcp_entry["consumers"].is_array());
    assert!(project_mcp_entry["consumers"]
        .as_array()
        .expect("project mcp consumers")
        .iter()
        .any(|consumer| consumer["name"] == "Project Delivery" && consumer["kind"] == "team"));
}

#[tokio::test]
async fn project_export_route_materializes_linked_workspace_builtin_dependencies_and_roundtrips() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let source_project_response = create_project(
        &harness.router,
        &token,
        json!({
            "name": "Source Export Project",
            "description": "Exports linked workspace assets.",
        }),
    )
    .await;
    assert_eq!(source_project_response.status(), StatusCode::OK);
    let source_project: Value = decode_json(source_project_response).await;
    let source_project_id = source_project["id"]
        .as_str()
        .expect("source project id")
        .to_string();

    let target_project_response = create_project(
        &harness.router,
        &token,
        json!({
            "name": "Imported Export Project",
            "description": "Imports exported linked workspace assets.",
        }),
    )
    .await;
    assert_eq!(target_project_response.status(), StatusCode::OK);
    let target_project: Value = decode_json(target_project_response).await;
    let target_project_id = target_project["id"]
        .as_str()
        .expect("target project id")
        .to_string();

    let tool_catalog = get_tool_catalog(&harness.router, &token).await;
    let builtin_skill_id = tool_catalog["entries"]
        .as_array()
        .expect("catalog entries")
        .iter()
        .find(|entry| entry["kind"] == "skill" && entry["name"] == "financial-calculator")
        .and_then(|entry| entry["id"].as_str())
        .expect("builtin skill id")
        .to_string();

    let workspace_agent_response = create_workspace_agent(
        &harness.router,
        &token,
        json!({
            "workspaceId": "ws-local",
            "projectId": Value::Null,
            "scope": "workspace",
            "name": "Linked Finance Agent",
            "personality": "Exports workspace-linked builtin dependencies.",
            "tags": ["finance", "linked"],
            "prompt": "Handle linked finance requests.",
            "builtinToolKeys": ["bash"],
            "skillIds": [builtin_skill_id],
            "mcpServerNames": ["finance-data"],
            "description": "Workspace linked finance agent.",
            "status": "active"
        }),
    )
    .await;
    assert_eq!(workspace_agent_response.status(), StatusCode::OK);
    let workspace_agent: Value = decode_json(workspace_agent_response).await;
    let workspace_agent_id = workspace_agent["id"]
        .as_str()
        .expect("workspace agent id")
        .to_string();

    let link_response = create_project_agent_link(
        &harness.router,
        &token,
        &source_project_id,
        json!({
            "projectId": source_project_id,
            "agentId": workspace_agent_id,
        }),
    )
    .await;
    assert_eq!(link_response.status(), StatusCode::OK);

    let export_response = export_project_agent_bundle(
        &harness.router,
        &token,
        &source_project_id,
        json!({
            "mode": "single",
            "agentIds": [workspace_agent_id],
            "teamIds": [],
        }),
    )
    .await;
    assert_eq!(export_response.status(), StatusCode::OK);
    let exported: Value = decode_json(export_response).await;
    let exported_files = exported["files"].as_array().expect("exported files");

    assert_eq!(exported["rootDirName"], "Linked Finance Agent");
    assert!(exported_files.iter().any(|file| {
        file["relativePath"] == Value::String("Linked Finance Agent/.octopus/manifest.json".into())
    }));
    assert!(exported_files.iter().any(|file| {
        file["relativePath"]
            == Value::String("Linked Finance Agent/skills/financial-calculator/SKILL.md".into())
    }));
    assert!(exported_files.iter().any(|file| {
        file["relativePath"]
            == Value::String("Linked Finance Agent/mcps/finance-data.json".into())
    }));
    assert!(exported_files.iter().any(|file| {
        file["relativePath"]
            .as_str()
            .is_some_and(|path| path.starts_with("Linked Finance Agent/") && path.ends_with(".png"))
    }));

    let import_response = import_project_agent_bundle(
        &harness.router,
        &token,
        &target_project_id,
        json!({
            "files": exported_files,
        }),
    )
    .await;
    assert_eq!(import_response.status(), StatusCode::OK);
    let imported: Value = decode_json(import_response).await;
    assert_eq!(imported["failureCount"], Value::from(0));
    assert_eq!(imported["agentCount"], Value::from(1));
    assert_eq!(imported["teamCount"], Value::from(0));
    assert_eq!(imported["skillCount"], Value::from(1));
    assert_eq!(imported["mcpCount"], Value::from(1));
}

#[tokio::test]
async fn workspace_model_catalog_returns_registry_snapshot_shape() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let payload = get_model_catalog(&harness.router, &token).await;

    assert!(
        payload.get("providers").is_some(),
        "missing providers snapshot"
    );
    assert!(payload.get("models").is_some(), "missing models snapshot");
    assert!(
        payload.get("defaultSelections").is_some(),
        "missing default selections"
    );
    assert!(payload.get("diagnostics").is_some(), "missing diagnostics");
}

#[tokio::test]
async fn workspace_model_catalog_reflects_runtime_registry_overrides_without_restart() {
    let harness = test_harness();
    let token = register_owner_session(&harness.router, "octopus-desktop")
        .await
        .token;

    let _saved = save_runtime_config_without_session(
        &harness.router,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "modelRegistry": {
                    "providers": {
                        "deepseek": {
                            "providerId": "deepseek",
                            "label": "DeepSeek",
                            "enabled": true,
                            "surfaces": [
                                {
                                    "surface": "conversation",
                                    "protocolFamily": "openai_chat",
                                    "authStrategy": "bearer",
                                    "baseUrl": "https://api.deepseek.com",
                                    "baseUrlPolicy": "allow_override"
                                }
                            ]
                        }
                    },
                    "models": {
                        "deepseek-chat": {
                            "modelId": "deepseek-chat",
                            "providerId": "deepseek",
                            "label": "DeepSeek Chat",
                            "family": "deepseek-chat",
                            "track": "latest_alias",
                            "enabled": true,
                            "surfaceBindings": [
                                {
                                    "surface": "conversation",
                                    "protocolFamily": "openai_chat"
                                }
                            ],
                            "capabilities": ["streaming", "tool_calling"],
                            "metadata": {
                                "source": "workspace-override"
                            }
                        }
                    },
                    "defaultSelections": {
                        "conversation": {
                            "providerId": "deepseek",
                            "modelId": "deepseek-chat",
                            "surface": "conversation"
                        }
                    }
                }
            }),
        },
    )
    .await;

    let payload = get_model_catalog(&harness.router, &token).await;
    let models = payload["models"].as_array().expect("models array");
    let defaults = &payload["defaultSelections"];

    assert!(models
        .iter()
        .any(|model| model["modelId"] == "deepseek-chat"));
    assert_eq!(defaults["conversation"]["modelId"], "deepseek-chat");
    assert_eq!(defaults["conversation"]["providerId"], "deepseek");
}

#[tokio::test]
async fn host_routes_require_a_valid_host_token_and_persist_preferences() {
    let harness = test_harness();

    let unauthorized = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/host/bootstrap")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

    let bootstrap_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/host/bootstrap")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(bootstrap_response.status(), StatusCode::OK);
    let bootstrap: serde_json::Value = decode_json(bootstrap_response).await;
    assert_eq!(bootstrap["hostState"]["platform"], "tauri");
    assert_eq!(bootstrap["backend"]["state"], "ready");

    let update_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PUT)
                .uri("/api/v1/host/preferences")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&octopus_core::ShellPreferences {
                        theme: "dark".into(),
                        locale: "en-US".into(),
                        font_size: 15,
                        font_family: "Inter, sans-serif".into(),
                        font_style: "sans".into(),
                        compact_sidebar: true,
                        left_sidebar_collapsed: true,
                        right_sidebar_collapsed: false,
                        update_channel: "preview".into(),
                        default_workspace_id: "ws-local".into(),
                        last_visited_route: "/workspaces/ws-local/overview?project=proj-redesign"
                            .into(),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(update_response.status(), StatusCode::OK);

    let preferences_response = harness
        .router
        .oneshot(
            Request::builder()
                .uri("/api/v1/host/preferences")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(preferences_response.status(), StatusCode::OK);
    let preferences = decode_json::<octopus_core::ShellPreferences>(preferences_response).await;
    assert_eq!(preferences.theme, "dark");
    assert_eq!(preferences.locale, "en-US");
    assert_eq!(preferences.update_channel, "preview");
    assert!(preferences.left_sidebar_collapsed);
}

#[tokio::test]
async fn host_update_routes_return_browser_safe_contracts() {
    let manifest_router = Router::new()
            .route(
                "/formal/latest.json",
                get(|| async {
                    Json(json!({
                        "version": "0.2.0",
                        "notes": "Formal release body",
                        "pub_date": "2026-04-08T11:30:00Z",
                        "channel": "formal",
                        "notesUrl": "https://github.com/GoyacJ/octopus/releases/tag/v0.2.0",
                        "platforms": {
                            "darwin-aarch64": {
                                "signature": "formal-signature",
                                "url": "https://github.com/GoyacJ/octopus/releases/download/v0.2.0/Octopus.app.tar.gz"
                            }
                        }
                    }))
                }),
            )
            .route(
                "/preview/latest.json",
                get(|| async {
                    Json(json!({
                        "version": "0.2.0-preview.4",
                        "notes": "Preview release body",
                        "pub_date": "2026-04-09T09:15:00Z",
                        "channel": "preview",
                        "notesUrl": "https://github.com/GoyacJ/octopus/releases/tag/v0.2.0-preview.4",
                        "platforms": {
                            "darwin-aarch64": {
                                "signature": "preview-signature",
                                "url": "https://github.com/GoyacJ/octopus/releases/download/v0.2.0-preview.4/Octopus.app.tar.gz"
                            }
                        }
                    }))
                }),
            );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("manifest listener");
    let address = listener.local_addr().expect("manifest addr");
    let manifest_server = tokio::spawn(async move {
        axum::serve(listener, manifest_router)
            .await
            .expect("manifest server");
    });

    let harness = test_harness();
    let formal_status = refresh_browser_host_update_status_with_endpoint(
        &harness.state,
        "formal",
        Some(&format!("http://{address}/formal/latest.json")),
    )
    .await
    .expect("formal update status");
    assert_eq!(formal_status.current_version, "0.1.0-test");
    assert_eq!(formal_status.current_channel, "formal");
    assert_eq!(formal_status.state, "update_available");
    assert_eq!(
        formal_status
            .latest_release
            .as_ref()
            .map(|release| release.version.as_str()),
        Some("0.2.0")
    );
    assert_eq!(
        formal_status
            .latest_release
            .as_ref()
            .and_then(|release| release.notes_url.as_deref()),
        Some("https://github.com/GoyacJ/octopus/releases/tag/v0.2.0")
    );
    assert!(!formal_status.capabilities.can_download);
    assert!(!formal_status.capabilities.can_install);
    assert!(formal_status.capabilities.supports_channels);
    assert!(formal_status.last_checked_at.is_some());

    let preview_status = refresh_browser_host_update_status_with_endpoint(
        &harness.state,
        "preview",
        Some(&format!("http://{address}/preview/latest.json")),
    )
    .await
    .expect("preview update status");
    assert_eq!(preview_status.current_channel, "preview");
    assert_eq!(preview_status.state, "update_available");
    assert_eq!(
        preview_status
            .latest_release
            .as_ref()
            .map(|release| release.version.as_str()),
        Some("0.2.0-preview.4")
    );
    assert!(preview_status.last_checked_at.is_some());

    let download_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/host/update-download")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(download_response.status(), StatusCode::OK);
    let download_status: serde_json::Value = decode_json(download_response).await;
    assert_eq!(download_status["state"], "error");
    assert_eq!(download_status["errorCode"], "UPDATE_DOWNLOAD_UNSUPPORTED");

    let install_response = harness
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/host/update-install")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(install_response.status(), StatusCode::OK);
    let install_status: serde_json::Value = decode_json(install_response).await;
    assert_eq!(install_status["state"], "error");
    assert_eq!(install_status["errorCode"], "UPDATE_INSTALL_UNSUPPORTED");
    manifest_server.abort();
}

#[tokio::test]
async fn host_notification_routes_roundtrip_and_preserve_history() {
    let harness = test_harness();

    let create_workspace_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/host/notifications")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateNotificationInput {
                        scope_kind: "workspace".into(),
                        scope_owner_id: Some("ws-local".into()),
                        level: "success".into(),
                        title: "Workspace synced".into(),
                        body: "The workspace is ready.".into(),
                        source: "workspace-store".into(),
                        toast_duration_ms: Some(30_000),
                        route_to: Some("/workspaces/ws-local/overview".into()),
                        action_label: Some("Open workspace".into()),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create_workspace_response.status(), StatusCode::OK);
    let workspace_notification = decode_json::<NotificationRecord>(create_workspace_response).await;

    let create_user_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/host/notifications")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateNotificationInput {
                        scope_kind: "user".into(),
                        scope_owner_id: Some("user-local".into()),
                        level: "warning".into(),
                        title: "Profile incomplete".into(),
                        body: "Add your preferred contact details.".into(),
                        source: "personal-center".into(),
                        toast_duration_ms: Some(15_000),
                        route_to: None,
                        action_label: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create_user_response.status(), StatusCode::OK);
    let user_notification = decode_json::<NotificationRecord>(create_user_response).await;

    let unread_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/host/notifications/unread-summary")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(unread_response.status(), StatusCode::OK);
    let unread = decode_json::<NotificationUnreadSummary>(unread_response).await;
    assert_eq!(unread.total, 2);
    assert_eq!(unread.by_scope.workspace, 1);
    assert_eq!(unread.by_scope.user, 1);

    let filtered_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/host/notifications?scope=workspace")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(filtered_response.status(), StatusCode::OK);
    let filtered = decode_json::<NotificationListResponse>(filtered_response).await;
    assert_eq!(filtered.notifications.len(), 1);
    assert_eq!(filtered.notifications[0].id, workspace_notification.id);
    assert_eq!(filtered.unread.total, 1);

    let marked_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!(
                    "/api/v1/host/notifications/{}/read",
                    workspace_notification.id
                ))
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(marked_response.status(), StatusCode::OK);
    let marked = decode_json::<NotificationRecord>(marked_response).await;
    assert!(marked.read_at.is_some());

    let dismissed_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(&format!(
                    "/api/v1/host/notifications/{}/dismiss-toast",
                    workspace_notification.id
                ))
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(dismissed_response.status(), StatusCode::OK);
    let dismissed = decode_json::<NotificationRecord>(dismissed_response).await;
    assert_eq!(dismissed.toast_visible_until, None);

    let mark_all_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/host/notifications/read-all")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&NotificationFilter {
                        scope: Some("user".into()),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(mark_all_response.status(), StatusCode::OK);
    let next_summary = decode_json::<NotificationUnreadSummary>(mark_all_response).await;
    assert_eq!(next_summary.total, 0);

    let listed_response = harness
        .router
        .oneshot(
            Request::builder()
                .uri("/api/v1/host/notifications?scope=all")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(listed_response.status(), StatusCode::OK);
    let listed = decode_json::<NotificationListResponse>(listed_response).await;
    assert_eq!(listed.notifications.len(), 2);
    assert_eq!(listed.notifications[0].id, user_notification.id);
    assert_eq!(listed.notifications[1].id, workspace_notification.id);
    assert!(listed
        .notifications
        .iter()
        .all(|notification| notification.read_at.is_some()));
    assert_eq!(listed.notifications[1].toast_visible_until, None);
}

#[tokio::test]
async fn host_routes_accept_browser_cors_preflight_for_local_dev_origin() {
    let response = test_harness()
        .router
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/host/bootstrap")
                .header(header::ORIGIN, "http://127.0.0.1:15420")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "authorization,content-type",
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("http://127.0.0.1:15420")
    );
}

#[tokio::test]
async fn workspace_routes_accept_packaged_tauri_https_origin() {
    let response = test_harness()
        .router
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/system/bootstrap")
                .header(header::ORIGIN, "https://tauri.localhost")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "x-request-id,content-type",
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("https://tauri.localhost")
    );
}

#[tokio::test]
async fn workspace_routes_accept_packaged_tauri_http_origin() {
    let response = test_harness()
        .router
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/auth/register-owner")
                .header(header::ORIGIN, "http://tauri.localhost")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(
                    header::ACCESS_CONTROL_REQUEST_HEADERS,
                    "content-type,x-request-id",
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("http://tauri.localhost")
    );
}

#[tokio::test]
async fn legacy_runtime_aliases_are_not_available() {
    let harness = test_harness();
    let response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/runtime/sessions")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn host_token_is_rejected_for_protected_workspace_and_runtime_routes() {
    let harness = test_harness();
    let workspace_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/workspace")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(workspace_response.status(), StatusCode::UNAUTHORIZED);
    let workspace_error = decode_json::<ApiErrorEnvelope>(workspace_response).await;
    assert_eq!(workspace_error.error.code, "UNAUTHENTICATED");

    let runtime_response = harness
        .router
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/runtime/sessions")
                .header(header::AUTHORIZATION, "Bearer desktop-test-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&CreateRuntimeSessionInput {
                        conversation_id: "conv-1".into(),
                        project_id: "proj-redesign".into(),
                        title: "Session".into(),
                        session_kind: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(runtime_response.status(), StatusCode::UNAUTHORIZED);
    let runtime_error = decode_json::<ApiErrorEnvelope>(runtime_response).await;
    assert_eq!(runtime_error.error.code, "UNAUTHENTICATED");
}

#[tokio::test]
async fn apps_routes_roundtrip_through_http_contract() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let list_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/apps")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(list_response.status(), StatusCode::OK);
    let listed = decode_json::<Vec<ClientAppRecord>>(list_response).await;
    assert!(listed.iter().any(|record| record.id == "octopus-web"));

    let register_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/apps")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ClientAppRecord {
                        id: "octopus-desktop-preview".into(),
                        name: "Octopus Desktop Preview".into(),
                        platform: "desktop".into(),
                        status: "active".into(),
                        first_party: true,
                        allowed_origins: vec!["http://127.0.0.1".into()],
                        allowed_hosts: vec!["127.0.0.1".into()],
                        session_policy: "session_token".into(),
                        default_scopes: vec!["workspace".into(), "runtime".into()],
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(register_response.status(), StatusCode::OK);
    let registered = decode_json::<ClientAppRecord>(register_response).await;
    assert_eq!(registered.id, "octopus-desktop-preview");
}

#[tokio::test]
async fn audit_inbox_and_knowledge_routes_return_transport_records() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let audit_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(audit_response.status(), StatusCode::OK);
    let _audit_records = decode_json::<Vec<AuditRecord>>(audit_response).await;

    let inbox_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/inbox")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(inbox_response.status(), StatusCode::OK);
    let _inbox_records = decode_json::<Vec<InboxItemRecord>>(inbox_response).await;

    let knowledge_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/knowledge")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(knowledge_response.status(), StatusCode::OK);
    let _knowledge_records = decode_json::<Vec<KnowledgeEntryRecord>>(knowledge_response).await;
}

#[tokio::test]
async fn apps_audit_inbox_and_knowledge_routes_reject_non_owner_sessions() {
    let harness = test_harness();
    let member = create_member_session(&harness.router, "octopus-desktop").await;

    let apps_read = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/apps")
                .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(apps_read.status(), StatusCode::FORBIDDEN);
    let apps_read_error = decode_json::<ApiErrorEnvelope>(apps_read).await;
    assert_eq!(apps_read_error.error.code, "FORBIDDEN");

    let apps_write = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/apps")
                .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ClientAppRecord {
                        id: "octopus-member-preview".into(),
                        name: "Octopus Member Preview".into(),
                        platform: "desktop".into(),
                        status: "active".into(),
                        first_party: true,
                        allowed_origins: vec!["http://127.0.0.1".into()],
                        allowed_hosts: vec!["127.0.0.1".into()],
                        session_policy: "session_token".into(),
                        default_scopes: vec!["workspace".into()],
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(apps_write.status(), StatusCode::FORBIDDEN);
    let apps_write_error = decode_json::<ApiErrorEnvelope>(apps_write).await;
    assert_eq!(apps_write_error.error.code, "FORBIDDEN");

    let audit_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/audit")
                .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(audit_response.status(), StatusCode::FORBIDDEN);
    let audit_error = decode_json::<ApiErrorEnvelope>(audit_response).await;
    assert_eq!(audit_error.error.code, "FORBIDDEN");

    let inbox_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/inbox")
                .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(inbox_response.status(), StatusCode::FORBIDDEN);
    let inbox_error = decode_json::<ApiErrorEnvelope>(inbox_response).await;
    assert_eq!(inbox_error.error.code, "FORBIDDEN");

    let knowledge_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/knowledge")
                .header(header::AUTHORIZATION, format!("Bearer {}", member.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(knowledge_response.status(), StatusCode::FORBIDDEN);
    let knowledge_error = decode_json::<ApiErrorEnvelope>(knowledge_response).await;
    assert_eq!(knowledge_error.error.code, "FORBIDDEN");
}

#[tokio::test]
async fn runtime_session_flow_supports_json_event_polling_and_observation_with_session_token() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created = create_runtime_session(&harness.router, &session.token, "Session", None).await;

    let run = submit_turn(
        &harness.router,
        &session.token,
        &created.summary.id,
        "ask",
        None,
    )
    .await;
    assert_eq!(run.status, "waiting_approval");

    let events_response = harness
        .router
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/api/v1/runtime/sessions/{}/events?after=missing",
                    created.summary.id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(events_response.status(), StatusCode::OK);
    let events = decode_json::<Vec<RuntimeEventEnvelope>>(events_response).await;
    assert!(events
        .iter()
        .any(|event| event.event_type == "runtime.approval.requested"));
    assert!(events
        .iter()
        .any(|event| event.event_type == "runtime.run.updated"));

    let trace_events = harness
        .infra
        .observation
        .list_trace_events()
        .await
        .expect("trace events");
    let audit_records = harness
        .infra
        .observation
        .list_audit_records()
        .await
        .expect("audit records");
    assert!(trace_events
        .iter()
        .any(|event| event.event_kind == "turn_submitted"));
    assert!(audit_records
        .iter()
        .any(|record| record.action == "runtime.submit_turn"));
}

#[tokio::test]
async fn runtime_submit_turn_executes_model_and_records_resolved_target() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created =
        create_runtime_session(&harness.router, &session.token, "Execution Session", None).await;

    let run = submit_turn_with_input(
        &harness.router,
        &session.token,
        &created.summary.id,
        SubmitRuntimeTurnInput {
            content: "Reply with a short acknowledgement.".into(),
            model_id: Some("claude-sonnet-4-5".into()),
            configured_model_id: None,
            permission_mode: "readonly".into(),
            actor_kind: None,
            actor_id: None,
        },
        None,
    )
    .await;

    assert_eq!(run.status, "completed");

    let detail = runtime_session_detail(&harness.router, &session.token, &created.summary.id).await;
    let assistant_message = detail
        .messages
        .iter()
        .find(|message| message.sender_type == "assistant")
        .expect("assistant message");
    assert!(!assistant_message.content.is_empty());

    let run_value = serde_json::to_value(&run).expect("serialize run");
    assert_eq!(run_value["resolvedTarget"]["providerId"], "anthropic");
    assert_eq!(run_value["resolvedTarget"]["modelId"], "claude-sonnet-4-5");
    assert_eq!(run_value["resolvedTarget"]["surface"], "conversation");
}

#[tokio::test]
async fn workspace_model_catalog_exposes_configured_models_and_submit_turn_accepts_configured_model_id(
) {
    let harness = test_harness();
    let owner = register_owner_session(&harness.router, "octopus-desktop").await;

    let _saved = save_runtime_config_without_session(
        &harness.router,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "configuredModels": {
                    "anthropic-primary": {
                        "configuredModelId": "anthropic-primary",
                        "name": "Claude Primary",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": "env:ANTHROPIC_API_KEY",
                        "enabled": true,
                        "source": "workspace"
                    },
                    "anthropic-alt": {
                        "configuredModelId": "anthropic-alt",
                        "name": "Claude Alt",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": "env:ANTHROPIC_ALT_API_KEY",
                        "baseUrl": "https://anthropic.alt.example.test",
                        "enabled": true,
                        "source": "workspace"
                    }
                },
                "defaultSelections": {
                    "conversation": {
                        "configuredModelId": "anthropic-primary",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "surface": "conversation"
                    }
                }
            }),
        },
    )
    .await;

    let catalog_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace/catalog/models")
                .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_value = decode_json::<serde_json::Value>(catalog_response).await;
    let configured_models = catalog_value["configuredModels"]
        .as_array()
        .expect("configured models array");
    assert!(configured_models
        .iter()
        .any(|model| model["configuredModelId"] == "anthropic-primary"
            && model["name"] == "Claude Primary"));
    assert!(configured_models.iter().any(
        |model| model["configuredModelId"] == "anthropic-alt" && model["name"] == "Claude Alt"
    ));
    assert_eq!(
        catalog_value["defaultSelections"]["conversation"]["configuredModelId"],
        "anthropic-primary"
    );

    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created = create_runtime_session(
        &harness.router,
        &session.token,
        "Configured Model Session",
        None,
    )
    .await;

    let response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/runtime/sessions/{}/turns",
                    created.summary.id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "content": "Reply with a short acknowledgement.",
                        "configuredModelId": "anthropic-alt",
                        "permissionMode": "readonly"
                    }))
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body");
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let run_value = serde_json::from_slice::<serde_json::Value>(&body).expect("run json");
    assert_eq!(run_value["configuredModelId"], "anthropic-alt");
    assert_eq!(
        run_value["resolvedTarget"]["configuredModelId"],
        "anthropic-alt"
    );
    assert_eq!(
        run_value["resolvedTarget"]["configuredModelName"],
        "Claude Alt"
    );
    assert_eq!(
        run_value["resolvedTarget"]["credentialRef"],
        "env:ANTHROPIC_ALT_API_KEY"
    );
}

#[tokio::test]
async fn workspace_model_catalog_exposes_token_usage_and_runtime_blocks_exhausted_quota() {
    let harness = test_harness();
    let owner = register_owner_session(&harness.router, "octopus-desktop").await;

    let _saved = save_runtime_config_without_session(
        &harness.router,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "configuredModels": {
                    "quota-model": {
                        "configuredModelId": "quota-model",
                        "name": "Quota Model",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": "env:ANTHROPIC_API_KEY",
                        "tokenQuota": {
                            "totalTokens": 32
                        },
                        "enabled": true,
                        "source": "workspace"
                    }
                }
            }),
        },
    )
    .await;

    let first_session =
        create_runtime_session(&harness.router, &owner.token, "Quota Session 1", None).await;
    let first_run = submit_turn_with_input(
        &harness.router,
        &owner.token,
        &first_session.summary.id,
        SubmitRuntimeTurnInput {
            content: "Use the whole quota.".into(),
            model_id: None,
            configured_model_id: Some("quota-model".into()),
            permission_mode: "readonly".into(),
            actor_kind: None,
            actor_id: None,
        },
        None,
    )
    .await;
    assert_eq!(first_run.consumed_tokens, Some(32));

    let catalog_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/v1/workspace/catalog/models")
                .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(catalog_response.status(), StatusCode::OK);
    let catalog_value = decode_json::<serde_json::Value>(catalog_response).await;
    let quota_model = catalog_value["configuredModels"]
        .as_array()
        .expect("configured models array")
        .iter()
        .find(|model| model["configuredModelId"] == "quota-model")
        .cloned()
        .expect("quota model in catalog");
    assert_eq!(quota_model["tokenQuota"]["totalTokens"], 32);
    assert_eq!(quota_model["tokenUsage"]["usedTokens"], 32);
    assert_eq!(quota_model["tokenUsage"]["remainingTokens"], 0);
    assert_eq!(quota_model["tokenUsage"]["exhausted"], true);

    let second_session =
        create_runtime_session(&harness.router, &owner.token, "Quota Session 2", None).await;
    let response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/runtime/sessions/{}/turns",
                    second_session.summary.id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "content": "This should be blocked.",
                        "configuredModelId": "quota-model",
                        "permissionMode": "readonly"
                    }))
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error = decode_json::<ApiErrorEnvelope>(response).await;
    assert_eq!(error.error.code, "INVALID_INPUT");
    assert!(error
        .error
        .message
        .contains("has reached its total token limit"));
}

#[tokio::test]
async fn runtime_submit_turn_rejects_unknown_registry_model() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created = create_runtime_session(
        &harness.router,
        &session.token,
        "Unknown Model Session",
        None,
    )
    .await;

    let response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/runtime/sessions/{}/turns",
                    created.summary.id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&SubmitRuntimeTurnInput {
                        content: "hello".into(),
                        model_id: Some("missing-model".into()),
                        configured_model_id: None,
                        permission_mode: "readonly".into(),
                        actor_kind: None,
                        actor_id: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let error = decode_json::<ApiErrorEnvelope>(response).await;
    assert_eq!(error.error.code, "INVALID_INPUT");
    assert!(error.error.message.contains("missing-model"));
}

#[tokio::test]
async fn project_runtime_model_settings_filter_allowed_models_and_override_default_selection() {
    let harness = test_harness();
    let owner = register_owner_session(&harness.router, "octopus-desktop").await;

    let _saved_workspace = save_runtime_config_without_session(
        &harness.router,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: json!({
                "configuredModels": {
                    "anthropic-primary": {
                        "configuredModelId": "anthropic-primary",
                        "name": "Claude Primary",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": "env:ANTHROPIC_API_KEY",
                        "enabled": true,
                        "source": "workspace"
                    },
                    "anthropic-alt": {
                        "configuredModelId": "anthropic-alt",
                        "name": "Claude Alt",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": "env:ANTHROPIC_ALT_API_KEY",
                        "enabled": true,
                        "source": "workspace"
                    }
                },
                "defaultSelections": {
                    "conversation": {
                        "configuredModelId": "anthropic-primary",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "surface": "conversation"
                    }
                }
            }),
        },
    )
    .await;

    let create_project_response = create_project(
        &harness.router,
        &owner.token,
        json!({
            "name": "Project Runtime Models",
            "description": "Project-specific model selection coverage.",
            "assignments": {
                "models": {
                    "configuredModelIds": ["anthropic-alt"],
                    "defaultConfiguredModelId": "anthropic-alt"
                }
            }
        }),
    )
    .await;
    assert_eq!(create_project_response.status(), StatusCode::OK);
    let created_project = decode_json::<Value>(create_project_response).await;
    let project_id = created_project["id"]
        .as_str()
        .expect("project id")
        .to_string();

    let _saved_project = save_project_runtime_config(
        &harness.router,
        &owner.token,
        &project_id,
        RuntimeConfigPatch {
            scope: "project".into(),
            patch: json!({
                "projectSettings": {
                    "models": {
                        "allowedConfiguredModelIds": ["anthropic-alt"],
                        "defaultConfiguredModelId": "anthropic-alt"
                    }
                }
            }),
        },
    )
    .await;

    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created = create_runtime_session_for_project(
        &harness.router,
        &session.token,
        "Project Model Session",
        &project_id,
    )
    .await;

    let defaulted_run = submit_turn_with_input(
        &harness.router,
        &session.token,
        &created.summary.id,
        SubmitRuntimeTurnInput {
            content: "Reply with a short acknowledgement.".into(),
            model_id: None,
            configured_model_id: None,
            permission_mode: "readonly".into(),
            actor_kind: None,
            actor_id: None,
        },
        None,
    )
    .await;
    assert_eq!(
        defaulted_run
            .resolved_target
            .expect("resolved target")
            .configured_model_id,
        "anthropic-alt"
    );

    let denied_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/runtime/sessions/{}/turns",
                    created.summary.id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&SubmitRuntimeTurnInput {
                        content: "Try the workspace default.".into(),
                        model_id: None,
                        configured_model_id: Some("anthropic-primary".into()),
                        permission_mode: "readonly".into(),
                        actor_kind: None,
                        actor_id: None,
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(denied_response.status(), StatusCode::BAD_REQUEST);
    let error = decode_json::<ApiErrorEnvelope>(denied_response).await;
    assert_eq!(error.error.code, "INVALID_INPUT");
    assert!(error.error.message.contains("anthropic-primary"));
}

#[tokio::test]
async fn project_runtime_settings_reject_unassigned_tools_and_agents() {
    let harness = test_harness();
    let owner = register_owner_session(&harness.router, "octopus-desktop").await;

    let create_project_response = create_project(
        &harness.router,
        &owner.token,
        json!({
            "name": "Project Runtime Actors",
            "description": "Project-specific tool and actor assignment coverage.",
            "assignments": {
                "tools": {
                    "sourceKeys": ["builtin:bash"]
                },
                "agents": {
                    "agentIds": ["agent-architect"],
                    "teamIds": ["team-studio"]
                }
            }
        }),
    )
    .await;
    assert_eq!(create_project_response.status(), StatusCode::OK);
    let created_project = decode_json::<Value>(create_project_response).await;
    let project_id = created_project["id"]
        .as_str()
        .expect("project id")
        .to_string();

    let invalid_tool_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&RuntimeConfigPatch {
                        scope: "project".into(),
                        patch: json!({
                            "projectSettings": {
                                "tools": {
                                    "enabledSourceKeys": ["builtin:terminal"],
                                    "overrides": {
                                        "builtin:terminal": { "permissionMode": "readonly" }
                                    }
                                }
                            }
                        }),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(invalid_tool_response.status(), StatusCode::BAD_REQUEST);
    let invalid_tool_error = decode_json::<ApiErrorEnvelope>(invalid_tool_response).await;
    assert!(invalid_tool_error
        .error
        .message
        .contains("unassigned sourceKey `builtin:terminal`"));

    let invalid_agent_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::PATCH)
                .uri(format!("/api/v1/projects/{project_id}/runtime-config"))
                .header(header::AUTHORIZATION, format!("Bearer {}", owner.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&RuntimeConfigPatch {
                        scope: "project".into(),
                        patch: json!({
                            "projectSettings": {
                                "agents": {
                                    "enabledAgentIds": ["agent-reviewer"],
                                    "enabledTeamIds": ["team-studio"]
                                }
                            }
                        }),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(invalid_agent_response.status(), StatusCode::BAD_REQUEST);
    let invalid_agent_error = decode_json::<ApiErrorEnvelope>(invalid_agent_response).await;
    assert!(invalid_agent_error
        .error
        .message
        .contains("unassigned agent `agent-reviewer`"));
}

#[tokio::test]
async fn runtime_events_support_sse_and_polling_consistency_for_session_tokens() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created =
        create_runtime_session(&harness.router, &session.token, "SSE Session", None).await;
    let initial_events = runtime_events_after(
        &harness.router,
        &session.token,
        &created.summary.id,
        "missing",
    )
    .await;
    let baseline_event = initial_events.last().expect("baseline event").id.clone();

    let sse_event = next_sse_event(
        &harness.router,
        &session.token,
        &created.summary.id,
        None,
        true,
    )
    .await;
    let polled_events = runtime_events_after(
        &harness.router,
        &session.token,
        &created.summary.id,
        &baseline_event,
    )
    .await;

    assert!(polled_events.iter().any(|event| event.id == sse_event.id));
    assert!(polled_events
        .iter()
        .any(|event| event.event_type == "runtime.approval.requested"));
}

#[tokio::test]
async fn runtime_events_support_sse_backlog_replay_with_last_event_id() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created =
        create_runtime_session(&harness.router, &session.token, "Replay Session", None).await;
    submit_turn(
        &harness.router,
        &session.token,
        &created.summary.id,
        "ask",
        None,
    )
    .await;

    let initial_events = runtime_events_after(
        &harness.router,
        &session.token,
        &created.summary.id,
        "missing",
    )
    .await;
    let baseline_event = initial_events.first().expect("baseline event").id.clone();

    let replayed_event = next_sse_event(
        &harness.router,
        &session.token,
        &created.summary.id,
        Some(&baseline_event),
        false,
    )
    .await;

    assert_ne!(replayed_event.id, baseline_event);
    assert!(replayed_event.sequence > 1);
    assert_eq!(replayed_event.event_type, "runtime.message.created");
}

#[tokio::test]
async fn runtime_mutations_replay_when_the_same_idempotency_key_is_reused() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let created_first = create_runtime_session(
        &harness.router,
        &session.token,
        "Idempotent Session",
        Some("idem-session-1"),
    )
    .await;
    let created_second = create_runtime_session(
        &harness.router,
        &session.token,
        "Idempotent Session",
        Some("idem-session-1"),
    )
    .await;

    assert_eq!(created_first.summary.id, created_second.summary.id);

    let first_run = submit_turn(
        &harness.router,
        &session.token,
        &created_first.summary.id,
        "ask",
        Some("idem-turn-1"),
    )
    .await;
    let second_run = submit_turn(
        &harness.router,
        &session.token,
        &created_first.summary.id,
        "ask",
        Some("idem-turn-1"),
    )
    .await;

    assert_eq!(first_run.id, second_run.id);
    let sessions_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/runtime/sessions")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(sessions_response.status(), StatusCode::OK);
    let sessions = decode_json::<Vec<octopus_core::RuntimeSessionSummary>>(sessions_response).await;
    assert_eq!(sessions.len(), 1);
}

#[tokio::test]
async fn protected_routes_reject_workspace_scope_mismatch() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/workspace")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header("X-Workspace-Id", "ws-other")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let error = decode_json::<ApiErrorEnvelope>(response).await;
    assert_eq!(error.error.code, "FORBIDDEN");
}

#[tokio::test]
async fn approval_resolution_updates_run_status_and_observation_records() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let approved_session =
        create_runtime_session(&harness.router, &session.token, "Approve Session", None).await;
    submit_turn(
        &harness.router,
        &session.token,
        &approved_session.summary.id,
        "ask",
        None,
    )
    .await;
    let detail = runtime_session_detail(
        &harness.router,
        &session.token,
        &approved_session.summary.id,
    )
    .await;
    let approval = detail.pending_approval.expect("pending approval");
    let approve_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/runtime/sessions/{}/approvals/{}",
                    approved_session.summary.id, approval.id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ResolveRuntimeApprovalInput {
                        decision: "approve".into(),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(approve_response.status(), StatusCode::OK);
    let approved_run = decode_json::<RuntimeRunSnapshot>(approve_response).await;
    assert_eq!(approved_run.status, "completed");
    let approved_detail = runtime_session_detail(
        &harness.router,
        &session.token,
        &approved_session.summary.id,
    )
    .await;
    assert!(approved_detail
        .messages
        .iter()
        .any(|message| message.sender_type == "assistant"));

    let approved_events = runtime_events_after(
        &harness.router,
        &session.token,
        &approved_session.summary.id,
        "missing",
    )
    .await;
    let approved_resolution = approved_events
        .iter()
        .find(|event| event.event_type == "runtime.approval.resolved")
        .expect("approval resolved event");
    assert_eq!(
        approved_resolution
            .approval
            .as_ref()
            .expect("approved payload")
            .status,
        "approved"
    );

    let rejected_session =
        create_runtime_session(&harness.router, &session.token, "Reject Session", None).await;
    submit_turn(
        &harness.router,
        &session.token,
        &rejected_session.summary.id,
        "ask",
        None,
    )
    .await;
    let reject_detail = runtime_session_detail(
        &harness.router,
        &session.token,
        &rejected_session.summary.id,
    )
    .await;
    let reject_approval = reject_detail.pending_approval.expect("pending approval");
    let reject_response = harness
        .router
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/api/v1/runtime/sessions/{}/approvals/{}",
                    rejected_session.summary.id, reject_approval.id
                ))
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&ResolveRuntimeApprovalInput {
                        decision: "reject".into(),
                    })
                    .expect("json"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(reject_response.status(), StatusCode::OK);
    let rejected_run = decode_json::<RuntimeRunSnapshot>(reject_response).await;
    assert_eq!(rejected_run.status, "blocked");

    let rejected_events = runtime_events_after(
        &harness.router,
        &session.token,
        &rejected_session.summary.id,
        "missing",
    )
    .await;
    let rejected_resolution = rejected_events
        .iter()
        .find(|event| event.event_type == "runtime.approval.resolved")
        .expect("approval resolved event");
    assert_eq!(
        rejected_resolution
            .approval
            .as_ref()
            .expect("rejected payload")
            .status,
        "rejected"
    );

    let trace_events = harness
        .infra
        .observation
        .list_trace_events()
        .await
        .expect("trace events");
    let audit_records = harness
        .infra
        .observation
        .list_audit_records()
        .await
        .expect("audit records");
    assert!(trace_events
        .iter()
        .any(|event| event.event_kind == "approval_resolved"));
    assert!(audit_records
        .iter()
        .any(|record| record.action == "runtime.resolve_approval"));
}

#[tokio::test]
async fn runtime_config_routes_load_validate_and_save_scoped_documents() {
    let harness = test_harness();

    let initial = get_runtime_config_without_session(&harness.router).await;
    assert!(initial.validation.valid);
    assert_eq!(initial.sources.len(), 1);
    assert!(initial
        .sources
        .iter()
        .any(|source| source.scope == "workspace" && !source.exists));

    let validation = validate_runtime_config_without_session(
        &harness.router,
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: serde_json::json!({
                "model": "claude-sonnet-4-5",
                "permissions": {
                    "defaultMode": "plan"
                }
            }),
        },
    )
    .await;
    assert!(validation.valid);

    let saved = save_runtime_config_without_session(
        &harness.router,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: serde_json::json!({
                "model": "claude-sonnet-4-5",
                "permissions": {
                    "defaultMode": "plan"
                }
            }),
        },
    )
    .await;

    assert_eq!(
        saved.effective_config.get("model"),
        Some(&serde_json::json!("claude-sonnet-4-5"))
    );
    assert!(saved.sources.iter().any(|source| {
        source.scope == "workspace"
            && source.source_key == "workspace"
            && source.display_path == "config/runtime/workspace.json"
            && source.exists
    }));

    let workspace_settings = harness
        .infra
        .paths
        .config_dir
        .join("runtime")
        .join("workspace.json");
    let written = std::fs::read_to_string(workspace_settings).expect("workspace settings written");
    assert!(written.contains("\"model\": \"claude-sonnet-4-5\""));
    assert!(written.contains("\"defaultMode\": \"plan\""));
}

#[tokio::test]
async fn runtime_config_probe_route_executes_real_configured_model_request() {
    let harness = test_harness();

    let probe = probe_runtime_configured_model_without_session(
        &harness.router,
        RuntimeConfiguredModelProbeInput {
            scope: "workspace".into(),
            configured_model_id: "anthropic-primary".into(),
            patch: serde_json::json!({
                "configuredModels": {
                    "anthropic-primary": {
                        "configuredModelId": "anthropic-primary",
                        "name": "Claude Primary",
                        "providerId": "anthropic",
                        "modelId": "claude-sonnet-4-5",
                        "credentialRef": "env:ANTHROPIC_API_KEY",
                        "enabled": true,
                        "source": "workspace"
                    }
                }
            }),
        },
    )
    .await;

    assert!(probe.valid);
    assert!(probe.reachable);
    assert_eq!(probe.configured_model_id, "anthropic-primary");
    assert_eq!(
        probe.configured_model_name.as_deref(),
        Some("Claude Primary")
    );
    assert_eq!(probe.consumed_tokens, Some(32));
    assert!(probe.errors.is_empty());
}

#[tokio::test]
async fn runtime_config_routes_expose_workspace_relative_source_metadata() {
    let harness = test_harness();

    let config = get_runtime_config_without_session(&harness.router).await;
    let serialized = serde_json::to_value(&config).expect("serialize config");

    let workspace_source = config
        .sources
        .iter()
        .find(|source| source.scope == "workspace")
        .expect("workspace source");

    assert_eq!(workspace_source.source_key, "workspace");
    assert_eq!(
        workspace_source.display_path,
        "config/runtime/workspace.json"
    );
    assert!(workspace_source.owner_id.is_none());
    assert!(serialized.to_string().contains("\"displayPath\""));
    assert!(!serialized.to_string().contains("\"path\""));
}

#[tokio::test]
async fn runtime_public_workspace_config_route_stays_workspace_only() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let runtime_dir = harness.infra.paths.runtime_config_dir.clone();
    std::fs::create_dir_all(harness.infra.paths.runtime_user_config_dir.clone())
        .expect("user runtime dir");
    std::fs::write(
        harness
            .infra
            .paths
            .runtime_user_config_dir
            .join(format!("{}.json", session.user_id)),
        r#"{
              "model": "user-model",
              "provider": {
                "defaultModel": "user-default"
              }
            }"#,
    )
    .expect("write user settings");
    std::fs::write(
        runtime_dir.join("workspace.json"),
        r#"{
              "model": "workspace-model"
            }"#,
    )
    .expect("write workspace settings");

    let config = get_runtime_config_without_session(&harness.router).await;
    assert_eq!(
        config
            .sources
            .iter()
            .map(|source| source.scope.clone())
            .collect::<Vec<_>>(),
        vec!["workspace".to_string()]
    );
    assert_eq!(
        config.effective_config.get("model"),
        Some(&json!("workspace-model"))
    );
    assert_eq!(config.effective_config.get("provider"), None);
}

#[tokio::test]
async fn project_runtime_config_routes_include_current_user_precedence() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;
    let project_id = "proj-redesign";

    std::fs::write(
        harness
            .infra
            .paths
            .runtime_user_config_dir
            .join(format!("{}.json", session.user_id)),
        r#"{
              "model": "user-model",
              "provider": {
                "defaultModel": "user-default"
              },
              "permissions": {
                "defaultMode": "readonly"
              }
            }"#,
    )
    .expect("write user settings");
    std::fs::write(
        harness
            .infra
            .paths
            .runtime_config_dir
            .join("workspace.json"),
        r#"{
              "model": "workspace-model",
              "permissions": {
                "defaultMode": "plan"
              }
            }"#,
    )
    .expect("write workspace settings");
    std::fs::write(
        harness
            .infra
            .paths
            .runtime_project_config_dir
            .join(format!("{project_id}.json")),
        r#"{
              "model": "project-model"
            }"#,
    )
    .expect("write project settings");

    let fetched = get_project_runtime_config(&harness.router, &session.token, project_id).await;
    assert_eq!(
        fetched
            .sources
            .iter()
            .map(|source| source.source_key.clone())
            .collect::<Vec<_>>(),
        vec![
            format!("user:{}", session.user_id),
            "workspace".to_string(),
            format!("project:{project_id}"),
        ]
    );
    assert_eq!(
        fetched.effective_config.get("model"),
        Some(&json!("project-model"))
    );
    assert_eq!(
        fetched.effective_config.pointer("/permissions/defaultMode"),
        Some(&json!("plan"))
    );
    assert_eq!(
        fetched.effective_config.pointer("/provider/defaultModel"),
        Some(&json!("user-default"))
    );

    let saved = save_project_runtime_config(
        &harness.router,
        &session.token,
        project_id,
        RuntimeConfigPatch {
            scope: "project".into(),
            patch: json!({
                "provider": {
                    "defaultModel": "project-default"
                }
            }),
        },
    )
    .await;
    assert_eq!(
        saved.effective_config.pointer("/provider/defaultModel"),
        Some(&json!("project-default"))
    );
    assert_eq!(
        saved
            .sources
            .iter()
            .map(|source| source.source_key.clone())
            .collect::<Vec<_>>(),
        vec![
            format!("user:{}", session.user_id),
            "workspace".to_string(),
            format!("project:{project_id}"),
        ]
    );
}

#[tokio::test]
async fn runtime_config_routes_redact_plaintext_secrets_from_api_payloads() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    let project_dir = harness.infra.paths.config_dir.join("runtime");
    std::fs::create_dir_all(&project_dir).expect("workspace settings dir");
    std::fs::write(
        project_dir.join("workspace.json"),
        r#"{
              "provider": {
                "apiKey": "super-secret-key"
              },
              "mcpServers": {
                "remote": {
                  "type": "http",
                  "url": "https://example.test/mcp",
                  "headers": {
                    "Authorization": "Bearer secret-token"
                  }
                }
              }
            }"#,
    )
    .expect("write project settings");

    let config = get_runtime_config(&harness.router, &session.token).await;
    let project_source = config
        .sources
        .iter()
        .find(|source| {
            source.scope == "workspace" && source.display_path == "config/runtime/workspace.json"
        })
        .expect("workspace source");

    assert_eq!(
        project_source
            .document
            .as_ref()
            .and_then(|document| document.get("provider"))
            .and_then(|provider| provider.get("apiKey")),
        Some(&serde_json::json!("***"))
    );
    assert!(config.secret_references.iter().any(|secret| {
        secret.path.ends_with("provider.apiKey") && secret.status == "inline-redacted"
    }));
    assert!(config.secret_references.iter().any(|secret| {
        secret
            .path
            .ends_with("mcpServers.remote.headers.Authorization")
            && secret.status == "inline-redacted"
    }));
}

#[tokio::test]
async fn runtime_session_creation_persists_config_snapshot_and_sqlite_projection() {
    let harness = test_harness();
    let session = login_owner_session(&harness.router, "octopus-desktop").await;

    std::fs::write(
        harness
            .infra
            .paths
            .runtime_user_config_dir
            .join(format!("{}.json", session.user_id)),
        r#"{
              "model": "user-model"
            }"#,
    )
    .expect("write user settings");
    let _saved = save_runtime_config(
        &harness.router,
        &session.token,
        "workspace",
        RuntimeConfigPatch {
            scope: "workspace".into(),
            patch: serde_json::json!({
                "model": "workspace-model",
                "permissions": {
                    "defaultMode": "plan"
                }
            }),
        },
    )
    .await;
    let _project_saved = save_project_runtime_config(
        &harness.router,
        &session.token,
        "proj-redesign",
        RuntimeConfigPatch {
            scope: "project".into(),
            patch: serde_json::json!({
                "model": "project-model"
            }),
        },
    )
    .await;

    let created =
        create_runtime_session(&harness.router, &session.token, "Projection Session", None).await;
    assert!(!created.summary.config_snapshot_id.is_empty());
    assert!(!created.summary.effective_config_hash.is_empty());
    assert!(created
        .summary
        .started_from_scope_set
        .iter()
        .zip(["user", "workspace", "project"])
        .all(|(actual, expected)| actual == expected));
    assert_eq!(
        created.summary.started_from_scope_set,
        vec![
            "user".to_string(),
            "workspace".to_string(),
            "project".to_string()
        ]
    );
    assert_eq!(
        created.run.config_snapshot_id,
        created.summary.config_snapshot_id
    );

    let connection =
        Connection::open(&harness.infra.paths.db_path).expect("open runtime projection db");

    let stored_snapshot_id: String = connection
        .query_row(
            "SELECT config_snapshot_id FROM runtime_session_projections WHERE id = ?1",
            [&created.summary.id],
            |row| row.get(0),
        )
        .expect("runtime session projection");
    assert_eq!(stored_snapshot_id, created.summary.config_snapshot_id);

    let stored_hash: String = connection
        .query_row(
            "SELECT effective_config_hash FROM runtime_config_snapshots WHERE id = ?1",
            [&created.summary.config_snapshot_id],
            |row| row.get(0),
        )
        .expect("runtime config snapshot");
    assert_eq!(stored_hash, created.summary.effective_config_hash);

    let stored_source_refs: String = connection
        .query_row(
            "SELECT source_refs FROM runtime_config_snapshots WHERE id = ?1",
            [&created.summary.config_snapshot_id],
            |row| row.get(0),
        )
        .expect("runtime config source refs");
    assert_eq!(
        stored_source_refs,
        serde_json::json!([
            format!("user:{}", session.user_id),
            "workspace",
            "project:proj-redesign"
        ])
        .to_string()
    );
}
