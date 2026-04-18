use super::adapter_test_support::*;
use super::*;

#[tokio::test]
async fn submit_turn_with_configured_mcp_server_stays_async_safe() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config_with_http_mcp(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        "remote",
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-mcp-runtime",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "MCP Runtime Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Exercise capability planning with MCP config.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&vec!["remote"]).expect("mcp server names"),
                "Agent for MCP runtime projection tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert mcp runtime agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-mcp-runtime",
                octopus_core::DEFAULT_PROJECT_ID,
                "MCP Runtime Session",
                "agent:agent-mcp-runtime",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");
    assert!(session
        .provider_state_summary
        .iter()
        .any(|provider| provider.provider_key == "remote"));

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Inspect runtime state", None),
        )
        .await
        .expect("run");
    assert!(run
        .provider_state_summary
        .iter()
        .any(|provider| provider.provider_key == "remote"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
#[tokio::test]
async fn resolve_approval_with_configured_mcp_server_stays_async_safe() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config_with_http_mcp(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        "remote",
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-mcp-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "MCP Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Exercise approval resume with MCP config.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&vec!["remote"]).expect("mcp server names"),
                "Agent for MCP approval projection tests.",
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "auto",
                    "mcpAuth": "auto",
                    "teamSpawn": "auto",
                    "workflowEscalation": "auto"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert mcp approval agent");
    drop(connection);

    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-mcp-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "MCP Approval Session",
                "agent:agent-mcp-approval",
                Some("quota-model"),
                "workspace-write",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Run the approval gated action", Some("workspace-write")),
        )
        .await
        .expect("pending approval run");

    let detail = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail");
    let approval_id = detail
        .pending_approval
        .as_ref()
        .map(|approval| approval.id.clone())
        .expect("pending approval id");

    let resolved = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect("resolved approval");
    assert_eq!(resolved.approval_state, "approved");
    assert_eq!(resolved.checkpoint.current_iteration_index, 1);
    assert!(resolved
        .provider_state_summary
        .iter()
        .any(|provider| provider.provider_key == "remote"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
