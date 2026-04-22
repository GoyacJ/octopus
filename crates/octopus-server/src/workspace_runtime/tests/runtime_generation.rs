use super::support::*;
use super::*;

#[tokio::test]
async fn create_runtime_session_rejects_single_shot_generation_model_selection() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config_with_generation_model(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let headers = auth_headers(&session.token);
    let actor_ref = visible_workspace_agent_actor_ref(&state).await;

    let error = create_runtime_session(
        State(state),
        headers,
        Json(CreateRuntimeSessionInput {
            conversation_id: "conv-generation-only".into(),
            project_id: None,
            title: "Generation Only Session".into(),
            session_kind: None,
            selected_actor_ref: actor_ref,
            selected_configured_model_id: Some("generation-only-model".into()),
            execution_permission_mode: octopus_core::RUNTIME_PERMISSION_READ_ONLY.into(),
        }),
    )
    .await
    .expect_err("single-shot generation model should be rejected");

    assert!(
        error
            .source
            .to_string()
            .contains("does not expose a runtime-supported surface"),
        "unexpected error: {:?}",
        error
    );
}

#[tokio::test]
async fn runtime_generation_route_executes_single_shot_generation_models() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config_with_generation_model(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let response = crate::routes::build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runtime/generations")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "configuredModelId": "generation-only-model",
                        "content": "Write a haiku about runtime boundaries.",
                        "systemPrompt": "Reply in one line."
                    }))
                    .expect("generation request json"),
                ))
                .expect("generation request"),
        )
        .await
        .expect("generation response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("generation body");
    let payload: Value = serde_json::from_slice(&body).expect("generation payload");
    assert_eq!(payload["configuredModelId"], "generation-only-model");
    assert_eq!(payload["configuredModelName"], "Generation Only Model");
    assert_eq!(payload["requestId"], "mock-request-id");
    assert_eq!(payload["consumedTokens"], 32);
    assert!(
        payload["content"]
            .as_str()
            .expect("generation content")
            .contains("Write a haiku about runtime boundaries."),
        "unexpected generation payload: {payload:?}"
    );
}

#[tokio::test]
async fn runtime_generation_route_rejects_agent_conversation_models() {
    let temp = tempfile::tempdir().expect("tempdir");
    write_runtime_workspace_config(temp.path());
    let state = test_server_state(temp.path());
    let session = bootstrap_owner(&state).await;
    let response = crate::routes::build_router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/runtime/generations")
                .header(header::AUTHORIZATION, format!("Bearer {}", session.token))
                .header("x-workspace-id", DEFAULT_WORKSPACE_ID)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "configuredModelId": "quota-model",
                        "content": "Summarize the latest run."
                    }))
                    .expect("generation request json"),
                ))
                .expect("generation request"),
        )
        .await
        .expect("generation response");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("generation body");
    let payload: Value = serde_json::from_slice(&body).expect("generation payload");
    assert_eq!(payload["error"]["code"], "INVALID_INPUT");
    assert!(
        payload["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("does not expose a runtime-supported surface"),
        "unexpected generation error payload: {payload:?}"
    );
}
