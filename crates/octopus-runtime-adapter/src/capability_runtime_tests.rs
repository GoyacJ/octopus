use super::adapter_test_support::*;
use super::*;

#[tokio::test]
async fn create_session_populates_real_capability_plan_and_state_snapshot() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-capability-plan",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Capability Plan Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&vec!["bash", "WebFetch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for capability plan tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert capability agent");
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
                "conv-capability-plan",
                octopus_core::DEFAULT_PROJECT_ID,
                "Capability Plan Session",
                "agent:agent-capability-plan",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert_eq!(
        session.capability_summary.visible_tools,
        vec!["bash".to_string()]
    );
    assert_eq!(
        session.capability_summary.deferred_tools,
        vec!["WebFetch".to_string()]
    );
    assert!(session.provider_state_summary.is_empty());
    assert!(session.pending_mediation.is_none());
    assert!(session.last_execution_outcome.is_none());
    assert!(session.capability_state_ref.is_some());
    assert_eq!(
        session.run.capability_plan_summary,
        session.capability_summary
    );
    assert_eq!(
        session.run.checkpoint.capability_plan_summary,
        session.capability_summary
    );
    assert_eq!(session.run.checkpoint.current_iteration_index, 0);
    assert!(session.run.checkpoint.capability_state_ref.is_some());

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let persisted: (String, String, i64, i64, i64, i64) = connection
        .query_row(
            "SELECT capability_plan_summary_json, capability_state_ref, granted_tool_count, injected_skill_message_count, deferred_capability_count, hidden_capability_count
             FROM runtime_session_projections
             WHERE id = ?1",
            [&session.summary.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .expect("session capability projection");
    let summary: RuntimeCapabilityPlanSummary =
        serde_json::from_str(&persisted.0).expect("capability plan summary json");
    assert_eq!(summary.visible_tools, vec!["bash".to_string()]);
    assert_eq!(summary.deferred_tools, vec!["WebFetch".to_string()]);
    assert_eq!(
        persisted.1,
        session.capability_state_ref.clone().expect("state ref")
    );
    assert_eq!(persisted.2, 0);
    assert_eq!(persisted.3, 0);
    assert_eq!(persisted.4, 1);
    assert!(persisted.5 >= 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
#[tokio::test]
async fn create_session_includes_selected_plugin_tools_in_capability_plan() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-plugin-capability-plan",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Plugin Capability Plan Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use plugin tools through the runtime capability planner.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for plugin capability plan tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert plugin capability agent");
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
                "conv-plugin-capability-plan",
                octopus_core::DEFAULT_PROJECT_ID,
                "Plugin Capability Plan Session",
                "agent:agent-plugin-capability-plan",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert!(
        session
            .capability_summary
            .deferred_tools
            .contains(&"plugin_echo".to_string()),
        "selected plugin tool should be planned as a deferred runtime capability"
    );
    assert!(
        session.provider_state_summary.iter().any(|provider| {
            provider.provider_key == "sample-plugin@external" && provider.state == "ready"
        }),
        "selected plugin provider should surface as ready"
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_replans_and_executes_selected_plugin_tools_through_capability_runtime() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-plugin-runtime-loop",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Plugin Runtime Loop Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Discover and use plugin tools through the runtime capability planner.",
                serde_json::to_string(&vec!["ToolSearch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for plugin runtime loop tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert plugin runtime loop agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Selecting the plugin tool.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-select-plugin-echo".into(),
                name: "ToolSearch".into(),
                input: serde_json::json!({
                    "query": "select:plugin_echo",
                    "max_results": 5
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Running the selected plugin tool.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-plugin-echo".into(),
                name: "plugin_echo".into(),
                input: serde_json::json!({
                    "message": "hello from plugin"
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 6,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Plugin tool completed.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-plugin-runtime-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Plugin Runtime Loop Session",
                "agent:agent-plugin-runtime-loop",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Discover and run the plugin tool", None),
        )
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.current_step, "completed");
    assert_eq!(executor.request_count(), 3);

    let requests = executor.requests();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["ToolSearch"]
    );
    assert!(
        requests[1]
            .tools
            .iter()
            .any(|tool| tool.name == "plugin_echo"),
        "selected plugin tool should be replanned into the next model request"
    );
    assert!(
        requests[2]
            .messages
            .iter()
            .any(|message| matches!(message.role, runtime::MessageRole::Tool)),
        "final model request should include the plugin tool result"
    );

    let output_artifact_ref = run
        .artifact_refs
        .first()
        .cloned()
        .expect("runtime output artifact ref");
    let output_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("data")
                .join("artifacts")
                .join("runtime")
                .join(format!("{output_artifact_ref}.json")),
        )
        .expect("runtime output artifact"),
    )
    .expect("runtime output artifact json");
    let serialized_messages = output_artifact
        .get("serializedSession")
        .and_then(|value| value.get("session"))
        .and_then(|value| value.get("messages"))
        .and_then(serde_json::Value::as_array)
        .expect("serialized runtime messages");
    let plugin_tool_result = serialized_messages
        .iter()
        .flat_map(|message| {
            message
                .get("blocks")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|block| {
            block.get("type").and_then(serde_json::Value::as_str) == Some("tool_result")
                && block.get("toolName").and_then(serde_json::Value::as_str) == Some("plugin_echo")
        })
        .expect("plugin tool result");
    assert_eq!(
        plugin_tool_result
            .get("isError")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "plugin tool should execute successfully through the runtime capability bridge"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"tool.completed"));
    assert!(
        !event_kinds.contains(&"tool.failed"),
        "plugin capability execution should not fall back to a failed tool event"
    );
    let planner_completed = events
        .iter()
        .filter(|event| event.kind.as_deref() == Some("planner.completed"))
        .collect::<Vec<_>>();
    assert!(
        planner_completed.len() >= 2,
        "runtime loop should emit planner completion events for real replans"
    );
    assert!(planner_completed.iter().any(|event| {
        event
            .capability_plan_summary
            .as_ref()
            .is_some_and(|summary| summary.visible_tools.contains(&"plugin_echo".to_string()))
    }));
    assert!(planner_completed.iter().any(|event| {
        event.capability_plan_summary.as_ref().is_some_and(|summary| {
            summary.discovered_tools.contains(&"plugin_echo".to_string())
                && summary.exposed_tools.contains(&"plugin_echo".to_string())
        })
    }));

    let capability_snapshot = adapter
        .load_capability_state_snapshot(run.capability_state_ref.as_deref())
        .expect("capability snapshot")
        .expect("persisted capability snapshot");
    assert!(capability_snapshot
        .discovered_tools
        .contains(&"plugin_echo".to_string()));
    assert!(capability_snapshot
        .exposed_tools
        .contains(&"plugin_echo".to_string()));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_direct_deferred_tool_call_returns_retry_hint_without_exposing_tool() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-plugin-runtime-guard",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Plugin Runtime Guard Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Do not bypass deferred capability exposure.",
                serde_json::to_string(&vec!["ToolSearch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for deferred capability guard tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert plugin runtime guard agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Calling the deferred plugin directly.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-plugin-echo-direct".into(),
                name: "plugin_echo".into(),
                input: serde_json::json!({
                    "message": "hello without selection"
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Observed the retry hint.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 6,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-plugin-runtime-guard",
                octopus_core::DEFAULT_PROJECT_ID,
                "Plugin Runtime Guard Session",
                "agent:agent-plugin-runtime-guard",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Try using the deferred plugin directly", None),
        )
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(executor.request_count(), 2);

    let requests = executor.requests();
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["ToolSearch"]
    );
    assert_eq!(
        requests[1]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["ToolSearch"]
    );

    let output_artifact_ref = run
        .artifact_refs
        .first()
        .cloned()
        .expect("runtime output artifact ref");
    let output_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("data")
                .join("artifacts")
                .join("runtime")
                .join(format!("{output_artifact_ref}.json")),
        )
        .expect("runtime output artifact"),
    )
    .expect("runtime output artifact json");
    let serialized_messages = output_artifact
        .get("serializedSession")
        .and_then(|value| value.get("session"))
        .and_then(|value| value.get("messages"))
        .and_then(serde_json::Value::as_array)
        .expect("serialized runtime messages");
    let plugin_tool_result = serialized_messages
        .iter()
        .flat_map(|message| {
            message
                .get("blocks")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|block| {
            block.get("type").and_then(serde_json::Value::as_str) == Some("tool_result")
                && block.get("toolName").and_then(serde_json::Value::as_str) == Some("plugin_echo")
        })
        .expect("plugin tool result");
    assert_eq!(
        plugin_tool_result
            .get("isError")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );
    let result_output = plugin_tool_result
        .get("output")
        .and_then(serde_json::Value::as_str)
        .expect("tool result output");
    assert!(result_output.contains("ToolSearch"));
    assert!(result_output.contains("select:plugin_echo"));
    assert!(result_output.contains("not exposed"));

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"tool.failed"));
    let planner_completed = events
        .iter()
        .filter(|event| event.kind.as_deref() == Some("planner.completed"))
        .collect::<Vec<_>>();
    assert!(planner_completed.iter().all(|event| {
        event
            .capability_plan_summary
            .as_ref()
            .is_none_or(|summary| !summary.visible_tools.contains(&"plugin_echo".to_string()))
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn non_coding_research_docs_agent_runs_through_same_capability_trunk() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, task_domains, description, capability_policy_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                "agent-research-docs-runtime-loop",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Research Docs Agent",
                Option::<String>::None,
                "Evidence-driven researcher",
                serde_json::to_string(&vec!["research", "docs"]).expect("tags"),
                "Discover research helpers and summarize documentation sources.",
                serde_json::to_string(&vec!["ToolSearch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                serde_json::to_string(&vec!["research", "docs"]).expect("task domains"),
                "Agent for non-coding research/docs runtime acceptance tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert research docs agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Selecting the research helper.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-select-plugin-echo".into(),
                name: "ToolSearch".into(),
                input: serde_json::json!({
                    "query": "select:plugin_echo",
                    "max_results": 5
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Running the research helper.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-plugin-echo".into(),
                name: "plugin_echo".into(),
                input: serde_json::json!({
                    "message": "Summarize the docs findings"
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 6,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Research summary prepared.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-research-docs-runtime-loop",
                octopus_core::DEFAULT_PROJECT_ID,
                "Research Docs Runtime Loop Session",
                "agent:agent-research-docs-runtime-loop",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    assert!(
        session
            .capability_summary
            .deferred_tools
            .contains(&"plugin_echo".to_string()),
        "research/docs actor should receive the same deferred capability surface"
    );

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Research the docs and summarize the findings", None),
        )
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.current_step, "completed");
    assert_eq!(executor.request_count(), 3);

    let requests = executor.requests();
    assert_eq!(requests.len(), 3);
    let first_prompt = requests[0].system_prompt.join("\n\n");
    assert!(
        first_prompt.contains("Task domains: research, docs."),
        "system prompt should preserve the non-coding research/docs actor domain"
    );
    assert_eq!(
        requests[0]
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        vec!["ToolSearch"]
    );
    assert!(
        requests[1]
            .tools
            .iter()
            .any(|tool| tool.name == "plugin_echo"),
        "selected capability should be replanned into the next model request"
    );
    assert!(
        requests[2]
            .messages
            .iter()
            .any(|message| matches!(message.role, runtime::MessageRole::Tool)),
        "final request should include the research helper result"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    let event_kinds = events
        .iter()
        .map(|event| event.kind.as_deref().unwrap_or(event.event_type.as_str()))
        .collect::<Vec<_>>();
    assert!(event_kinds.contains(&"tool.completed"));
    assert!(
        !event_kinds.contains(&"tool.failed"),
        "non-coding research/docs actor should stay on the same successful capability trunk"
    );
    let planner_completed = events
        .iter()
        .filter(|event| event.kind.as_deref() == Some("planner.completed"))
        .collect::<Vec<_>>();
    assert!(
        planner_completed.len() >= 2,
        "research/docs runtime loop should emit real replans"
    );
    assert!(planner_completed.iter().any(|event| {
        event
            .capability_plan_summary
            .as_ref()
            .is_some_and(|summary| summary.visible_tools.contains(&"plugin_echo".to_string()))
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
#[tokio::test]
async fn submit_turn_requiring_approval_persists_real_mediation_and_outcome() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                "agent-capability-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Capability Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for capability approval tests.",
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
        .expect("upsert approval agent");
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
                "conv-capability-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Capability Approval Session",
                "agent:agent-capability-approval",
                Some("quota-model"),
                "workspace-write",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do the write action", Some("workspace-write")),
        )
        .await
        .expect("run");
    let expected_target_ref = format!("model-execution:{}:quota-model", run.id);

    let pending = run.pending_mediation.clone().expect("pending mediation");
    assert_eq!(
        pending.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(pending.mediation_kind, "approval");
    assert_eq!(pending.target_kind, "model-execution");
    assert_eq!(pending.target_ref, expected_target_ref);
    assert_eq!(pending.dispatch_kind.as_deref(), Some("model_execution"));
    assert_eq!(pending.concurrency_policy.as_deref(), Some("serialized"));
    assert_eq!(
        pending
            .input
            .as_ref()
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        pending
            .input
            .as_ref()
            .and_then(|value| value.get("requestedPermissionMode")),
        Some(&json!("workspace-write"))
    );
    assert!(run.capability_state_ref.is_some());
    assert_eq!(run.checkpoint.current_iteration_index, 0);
    assert_eq!(
        run.checkpoint.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(
        run.checkpoint.dispatch_kind.as_deref(),
        Some("model_execution")
    );
    assert_eq!(
        run.checkpoint.concurrency_policy.as_deref(),
        Some("serialized")
    );
    assert_eq!(
        run.checkpoint
            .input
            .as_ref()
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("model-execution")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.target_ref.as_deref()),
        Some(expected_target_ref.as_str())
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.dispatch_kind.as_deref()),
        Some("model_execution")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.concurrency_policy.as_deref()),
        Some("serialized")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.input.as_ref())
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert!(run
        .checkpoint
        .checkpoint_artifact_ref
        .as_deref()
        .is_some_and(|value| value.contains("runtime/checkpoints/mediation/")));
    let outcome = run
        .last_execution_outcome
        .clone()
        .expect("last execution outcome");
    assert_eq!(
        outcome.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(outcome.outcome, "require_approval");
    assert!(outcome.requires_approval);
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.tool_name.as_deref()),
        Some("Capability Approval Agent")
    );
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.dispatch_kind.as_deref()),
        Some("model_execution")
    );
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.concurrency_policy.as_deref()),
        Some("serialized")
    );
    assert_eq!(
        run.checkpoint
            .pending_mediation
            .as_ref()
            .and_then(|value| value.input.as_ref())
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        run.checkpoint.target_kind.as_deref(),
        Some("model-execution")
    );
    assert_eq!(
        run.checkpoint.target_ref.as_deref(),
        Some(expected_target_ref.as_str())
    );
    assert_eq!(
        run.checkpoint
            .last_execution_outcome
            .as_ref()
            .map(|value| value.outcome.as_str()),
        Some("require_approval")
    );
    assert_eq!(
        run.checkpoint
            .last_execution_outcome
            .as_ref()
            .and_then(|value| value.dispatch_kind.as_deref()),
        Some("model_execution")
    );
    assert_eq!(
        run.checkpoint
            .last_execution_outcome
            .as_ref()
            .and_then(|value| value.concurrency_policy.as_deref()),
        Some("serialized")
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    let persisted: (String, String, String, String, i64) = connection
        .query_row(
            "SELECT pending_mediation_json, capability_state_ref, last_execution_outcome_json, capability_plan_summary_json, deferred_capability_count
             FROM runtime_run_projections
             WHERE id = ?1",
            [&run.id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("run capability projection");
    let persisted_pending: RuntimePendingMediationSummary =
        serde_json::from_str(&persisted.0).expect("pending mediation json");
    assert_eq!(
        persisted_pending.tool_name.as_deref(),
        Some("Capability Approval Agent")
    );
    assert_eq!(persisted_pending.target_kind, "model-execution");
    assert_eq!(persisted_pending.target_ref, expected_target_ref);
    assert_eq!(
        persisted_pending.dispatch_kind.as_deref(),
        Some("model_execution")
    );
    assert_eq!(
        persisted_pending.concurrency_policy.as_deref(),
        Some("serialized")
    );
    assert_eq!(
        persisted_pending
            .input
            .as_ref()
            .and_then(|value| value.get("content")),
        Some(&json!("Do the write action"))
    );
    assert_eq!(
        persisted.1,
        run.capability_state_ref.clone().expect("state ref")
    );
    let persisted_outcome: RuntimeCapabilityExecutionOutcome =
        serde_json::from_str(&persisted.2).expect("outcome json");
    assert_eq!(persisted_outcome.outcome, "require_approval");
    assert_eq!(
        persisted_outcome.dispatch_kind.as_deref(),
        Some("model_execution")
    );
    assert_eq!(
        persisted_outcome.concurrency_policy.as_deref(),
        Some("serialized")
    );
    let persisted_plan: RuntimeCapabilityPlanSummary =
        serde_json::from_str(&persisted.3).expect("plan json");
    assert_eq!(persisted_plan.visible_tools, vec!["bash".to_string()]);
    assert_eq!(persisted.4, 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn resolve_approval_replays_selected_deferred_tool_from_checkpoint_capability_state() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_external_plugin(&root, "sample-plugin", "sample-plugin", "plugin_echo");
    write_workspace_config_with_plugins(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
        json!({
            "sample-plugin@external": true
        }),
        &["./external-plugins"],
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, capability_policy_json, default_model_strategy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-plugin-approval-replay",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Plugin Approval Replay Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Select the deferred plugin before executing it.",
                serde_json::to_string(&vec!["ToolSearch"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for approval replay exposure tests.",
                serde_json::to_string(&json!({
                    "pluginCapabilityRefs": ["plugin_echo"]
                }))
                .expect("capability policy"),
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
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
        .expect("upsert approval replay agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![
        vec![
            runtime::AssistantEvent::TextDelta("Selecting the plugin tool.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-select-plugin-echo".into(),
                name: "ToolSearch".into(),
                input: serde_json::json!({
                    "query": "select:plugin_echo",
                    "max_results": 5
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 5,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Calling the selected plugin tool.".into()),
            runtime::AssistantEvent::ToolUse {
                id: "tool-plugin-echo".into(),
                name: "plugin_echo".into(),
                input: serde_json::json!({
                    "message": "hello after approval"
                })
                .to_string(),
            },
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 6,
                output_tokens: 4,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
        vec![
            runtime::AssistantEvent::TextDelta("Replay completed after approval.".into()),
            runtime::AssistantEvent::Usage(runtime::TokenUsage {
                input_tokens: 7,
                output_tokens: 5,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
            runtime::AssistantEvent::MessageStop,
        ],
    ]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-plugin-approval-replay",
                octopus_core::DEFAULT_PROJECT_ID,
                "Plugin Approval Replay Session",
                "agent:agent-plugin-approval-replay",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_READ_ONLY,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Select the plugin and pause for approval", None),
        )
        .await
        .expect("pending run");
    assert_eq!(pending_run.status, "waiting_approval");
    let checkpoint_capability_state_ref = pending_run
        .checkpoint
        .capability_state_ref
        .clone()
        .expect("checkpoint capability state ref");

    let approval_id = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail")
        .pending_approval
        .map(|approval| approval.id)
        .expect("approval id");

    {
        let mut sessions = adapter.state.sessions.lock().expect("sessions mutex");
        let aggregate = sessions
            .get_mut(&session.summary.id)
            .expect("runtime aggregate");
        aggregate.detail.run.capability_state_ref = Some("corrupted-capability-state".into());
        aggregate.detail.capability_state_ref = Some("corrupted-capability-state".into());
    }

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

    assert_eq!(resolved.status, "completed");
    assert_eq!(executor.request_count(), 3);
    assert_eq!(
        resolved.capability_state_ref.as_deref(),
        Some(checkpoint_capability_state_ref.as_str()),
        "approval replay should keep using the checkpoint capability snapshot ref"
    );
    assert!(
        resolved
            .capability_plan_summary
            .visible_tools
            .contains(&"plugin_echo".to_string()),
        "approval replay should rebuild the visible surface from the durable capability snapshot"
    );

    let output_artifact_ref = resolved
        .artifact_refs
        .first()
        .cloned()
        .expect("runtime output artifact ref");
    let output_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(
            root.join("data")
                .join("artifacts")
                .join("runtime")
                .join(format!("{output_artifact_ref}.json")),
        )
        .expect("runtime output artifact"),
    )
    .expect("runtime output artifact json");
    let serialized_messages = output_artifact
        .get("serializedSession")
        .and_then(|value| value.get("session"))
        .and_then(|value| value.get("messages"))
        .and_then(serde_json::Value::as_array)
        .expect("serialized runtime messages");
    let plugin_tool_result = serialized_messages
        .iter()
        .flat_map(|message| {
            message
                .get("blocks")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
        })
        .find(|block| {
            block.get("type").and_then(serde_json::Value::as_str) == Some("tool_result")
                && block.get("toolName").and_then(serde_json::Value::as_str) == Some("plugin_echo")
        })
        .expect("plugin tool result");
    assert_eq!(
        plugin_tool_result
            .get("isError")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "approval replay should execute the previously selected deferred tool"
    );

    let events = adapter
        .list_events(&session.summary.id, None)
        .await
        .expect("events");
    assert!(events.iter().any(|event| {
        event.kind.as_deref() == Some("approval.resolved")
            && event.capability_plan_summary.as_ref().is_some_and(|summary| {
                summary.discovered_tools.contains(&"plugin_echo".to_string())
                    && summary.exposed_tools.contains(&"plugin_echo".to_string())
            })
    }));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_with_workspace_write_does_not_require_blanket_model_execution_approval() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workspace-write-submit",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Workspace Write Agent",
                Option::<String>::None,
                "Operator",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Use the runtime capability planner.",
                serde_json::to_string(&vec!["bash"]).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for workspace-write submit tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert workspace-write agent");
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
                "conv-workspace-write-submit",
                octopus_core::DEFAULT_PROJECT_ID,
                "Workspace Write Session",
                "agent:agent-workspace-write-submit",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_WORKSPACE_WRITE,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do the workspace write action", Some("workspace-write")),
        )
        .await
        .expect("run");

    assert_eq!(run.status, "completed");
    assert_eq!(run.current_step, "completed");
    assert!(run.pending_mediation.is_none());
    assert!(run.approval_target.is_none());
    assert_eq!(run.approval_state, "not-required");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn submit_turn_uses_compiled_model_execution_policy_for_tool_execution_approval() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-tool-execution-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Tool Execution Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Require approval before model execution starts.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for model-execution policy tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert tool execution approval agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![vec![
        runtime::AssistantEvent::TextDelta("This should never execute.".into()),
        runtime::AssistantEvent::MessageStop,
    ]]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-tool-execution-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Tool Execution Approval Session",
                "agent:agent-tool-execution-approval",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");
    let execution_policy = session_policy
        .target_decisions
        .get("model-execution:quota-model")
        .expect("model execution policy decision");
    assert_eq!(execution_policy.target_kind, "model-execution");
    assert_eq!(execution_policy.action, "requireApproval");
    assert!(execution_policy.deferred);
    assert!(execution_policy.requires_approval);
    assert_eq!(
        execution_policy.required_permission.as_deref(),
        Some("read-only")
    );

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do not start model execution yet", None),
        )
        .await
        .expect("pending run");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(run.current_step, "awaiting_approval");
    assert_eq!(executor.request_count(), 0);
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("model-execution")
    );
    assert_eq!(
        run.approval_target
            .as_ref()
            .and_then(|approval| approval.target_kind.as_deref()),
        Some("model-execution")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn resolving_a_consumed_runtime_approval_returns_conflict() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-consumed-approval",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Consumed Approval Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Require approval before model execution starts.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for consumed approval tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
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
        .expect("upsert consumed approval agent");
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
                "conv-consumed-approval",
                octopus_core::DEFAULT_PROJECT_ID,
                "Consumed Approval Session",
                "agent:agent-consumed-approval",
                Some("quota-model"),
                octopus_core::RUNTIME_PERMISSION_READ_ONLY,
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let pending_run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Pause for approval first", None),
        )
        .await
        .expect("pending run");
    assert_eq!(pending_run.status, "waiting_approval");

    let approval_id = adapter
        .get_session(&session.summary.id)
        .await
        .expect("session detail")
        .pending_approval
        .map(|approval| approval.id)
        .expect("approval id");

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
    assert_eq!(resolved.status, "completed");

    let error = adapter
        .resolve_approval(
            &session.summary.id,
            &approval_id,
            ResolveRuntimeApprovalInput {
                decision: "approve".into(),
            },
        )
        .await
        .expect_err("consumed approval should conflict");
    assert!(matches!(error, AppError::Conflict(_)));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn partial_approval_preference_json_merges_with_defaults_for_policy_compilation() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-partial-approval-policy",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Partial Approval Policy Agent",
                Option::<String>::None,
                "Approver",
                serde_json::to_string(&vec!["project", "runtime"]).expect("tags"),
                "Require approval before model execution starts.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&Vec::<String>::new()).expect("mcp server names"),
                "Agent for partial approval preference tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert partial approval policy agent");
    drop(connection);

    let executor = Arc::new(ScriptedConversationRuntimeModelDriver::new(vec![vec![
        runtime::AssistantEvent::TextDelta("This should never execute.".into()),
        runtime::AssistantEvent::MessageStop,
    ]]));
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        executor.clone(),
    );

    let session = adapter
        .create_session(
            session_input(
                "conv-partial-approval-policy",
                octopus_core::DEFAULT_PROJECT_ID,
                "Partial Approval Policy Session",
                "agent:agent-partial-approval-policy",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");
    let execution_policy = session_policy
        .target_decisions
        .get("model-execution:quota-model")
        .expect("model execution policy decision");
    assert_eq!(execution_policy.target_kind, "model-execution");
    assert_eq!(execution_policy.action, "requireApproval");
    assert!(execution_policy.deferred);
    assert!(execution_policy.requires_approval);
    assert_eq!(
        execution_policy.required_permission.as_deref(),
        Some("read-only")
    );

    let run = adapter
        .submit_turn(
            &session.summary.id,
            turn_input("Do not start model execution yet", None),
        )
        .await
        .expect("pending run");

    assert_eq!(run.status, "waiting_approval");
    assert_eq!(run.current_step, "awaiting_approval");
    assert_eq!(executor.request_count(), 0);
    assert_eq!(
        run.pending_mediation
            .as_ref()
            .map(|mediation| mediation.target_kind.as_str()),
        Some("model-execution")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn policy_compiler_uses_runtime_config_enablement_for_model_and_mcp_targets() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_json(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        json!({
            "configuredModels": {
                "quota-model": {
                    "configuredModelId": "quota-model",
                    "name": "Quota Model",
                    "providerId": "anthropic",
                    "modelId": "claude-sonnet-4-5",
                    "credentialRef": TEST_ANTHROPIC_CREDENTIAL_REF,
                    "enabled": false,
                    "source": "workspace"
                }
            }
        }),
    );
    grant_owner_permissions(&infra, "user-owner");

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, default_model_strategy_json, capability_policy_json, permission_envelope_json, memory_policy_json, delegation_policy_json, approval_preference_json, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                "agent-runtime-config-enablement",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Runtime Config Enablement Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["policy"]).expect("tags"),
                "Respect the frozen runtime policy.",
                serde_json::to_string(&Vec::<String>::new()).expect("builtin tool keys"),
                serde_json::to_string(&Vec::<String>::new()).expect("skill ids"),
                serde_json::to_string(&vec!["missing-mcp"]).expect("mcp server names"),
                "Agent for runtime config enablement policy tests.",
                serde_json::to_string(&json!({})).expect("default model strategy"),
                serde_json::to_string(&json!({})).expect("capability policy"),
                serde_json::to_string(&json!({})).expect("permission envelope"),
                serde_json::to_string(&json!({})).expect("memory policy"),
                serde_json::to_string(&json!({})).expect("delegation policy"),
                serde_json::to_string(&json!({
                    "toolExecution": "auto",
                    "memoryWrite": "require-approval",
                    "mcpAuth": "require-approval",
                    "teamSpawn": "require-approval",
                    "workflowEscalation": "require-approval"
                }))
                .expect("approval preference"),
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert enablement agent");
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
                "conv-runtime-config-enablement",
                octopus_core::DEFAULT_PROJECT_ID,
                "Runtime Config Enablement Session",
                "agent:agent-runtime-config-enablement",
                Some("quota-model"),
                "readonly",
            ),
            "user-owner",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");

    let execution_policy = session_policy
        .target_decisions
        .get("model-execution:quota-model")
        .expect("model execution policy decision");
    assert_eq!(execution_policy.action, "deny");
    assert!(execution_policy.hidden);
    assert_eq!(
        execution_policy.reason.as_deref(),
        Some("configured model is disabled or unavailable in the frozen runtime config")
    );

    let provider_auth_policy = session_policy
        .target_decisions
        .get("provider-auth:missing-mcp")
        .expect("provider auth policy decision");
    assert_eq!(provider_auth_policy.action, "deny");
    assert!(provider_auth_policy.hidden);
    assert_eq!(
        provider_auth_policy.reason.as_deref(),
        Some("provider or MCP server is not configured in the frozen runtime config")
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn policy_compiler_uses_workspace_authorization_for_capability_buckets() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    write_workspace_config(
        &infra.paths.runtime_config_dir.join("workspace.json"),
        Some(100),
    );

    let connection = Connection::open(&infra.paths.db_path).expect("db");
    connection
        .execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                "agent-workspace-authz",
                octopus_core::DEFAULT_WORKSPACE_ID,
                octopus_core::DEFAULT_PROJECT_ID,
                "project",
                "Workspace Authz Agent",
                Option::<String>::None,
                "Planner",
                serde_json::to_string(&vec!["policy"]).expect("tags"),
                "Respect workspace authorization.",
                serde_json::to_string(&vec!["shell"]).expect("builtin tool keys"),
                serde_json::to_string(&vec!["repo_search"]).expect("skill ids"),
                serde_json::to_string(&vec!["repo-mcp"]).expect("mcp server names"),
                "Agent for workspace authorization policy tests.",
                "active",
                timestamp_now() as i64,
            ],
        )
        .expect("upsert authz agent");
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
                "conv-workspace-authz",
                octopus_core::DEFAULT_PROJECT_ID,
                "Workspace Authz Session",
                "agent:agent-workspace-authz",
                Some("quota-model"),
                "readonly",
            ),
            "user-without-role",
        )
        .await
        .expect("session");

    let session_policy = adapter
        .load_session_policy_snapshot(&format!("{}-policy", session.summary.id))
        .expect("session policy snapshot");

    assert_eq!(session_policy.capability_decisions.builtin.action, "deny");
    assert!(session_policy.capability_decisions.builtin.hidden);
    assert_eq!(session_policy.capability_decisions.skill.action, "deny");
    assert!(session_policy.capability_decisions.skill.hidden);
    assert_eq!(session_policy.capability_decisions.mcp.action, "deny");
    assert!(session_policy.capability_decisions.mcp.hidden);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
