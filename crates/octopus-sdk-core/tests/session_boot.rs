use std::sync::Arc;

use octopus_sdk_contracts::PluginsSnapshot;
use octopus_sdk_plugin::SDK_PLUGIN_API_VERSION;
use octopus_sdk_session::SessionStore;

mod support;

#[tokio::test]
async fn test_start_session_writes_snapshot() {
    let (root, store) = support::temp_store();
    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![vec![]])),
        store.clone(),
    )
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    let snapshot = store
        .snapshot(&handle.session_id)
        .await
        .expect("snapshot should exist");

    assert_eq!(snapshot.working_dir, root.path());
    assert_eq!(
        snapshot.permission_mode,
        octopus_sdk_contracts::PermissionMode::Default
    );
    assert_eq!(snapshot.model, "test-model");
    assert_eq!(snapshot.config_snapshot_id, "cfg-1");
    assert_eq!(snapshot.effective_config_hash, "hash-1");
    assert_eq!(snapshot.token_budget, 8_192);
    assert_eq!(
        snapshot.plugins_snapshot,
        PluginsSnapshot {
            api_version: SDK_PLUGIN_API_VERSION.into(),
            plugins: Vec::new(),
        }
    );
}

#[tokio::test]
async fn test_resume_reads_session_store() {
    let (root, store) = support::temp_store();
    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![vec![]])),
        store.clone(),
    )
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");
    let resumed = runtime
        .resume(&handle.session_id)
        .await
        .expect("resume should read session store");

    assert_eq!(resumed.session_id, handle.session_id);
    assert_eq!(resumed.working_dir, root.path());
    assert_eq!(
        resumed.permission_mode,
        octopus_sdk_contracts::PermissionMode::Default
    );
    assert_eq!(resumed.model.0, "test-model");
    assert_eq!(resumed.config_snapshot_id, "cfg-1");
    assert_eq!(resumed.effective_config_hash, "hash-1");
    assert_eq!(resumed.token_budget, 8_192);
}

#[tokio::test]
async fn test_resume_rehydrates_runtime_state_for_submit_turn() {
    let (root, store) = support::temp_store();
    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![vec![]])),
        store.clone(),
    )
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");

    let resumed_runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![vec![
            octopus_sdk_contracts::AssistantEvent::TextDelta("resumed ok".into()),
            octopus_sdk_contracts::AssistantEvent::MessageStop {
                stop_reason: octopus_sdk_contracts::StopReason::EndTurn,
            },
        ]])),
        store,
    )
    .build()
    .expect("runtime should build");

    resumed_runtime
        .resume(&handle.session_id)
        .await
        .expect("resume should work");
    resumed_runtime
        .submit_turn(octopus_sdk_core::SubmitTurnInput {
            session_id: handle.session_id,
            message: support::text_message("continue"),
        })
        .await
        .expect("submit_turn should use restored state");
}

#[tokio::test]
async fn test_resume_restores_notes_and_todos_into_runtime_state() {
    let (root, store) = support::temp_store();
    let runtime = support::runtime_builder(
        Arc::new(support::ScriptedModelProvider::new(vec![vec![]])),
        store.clone(),
    )
    .build()
    .expect("runtime should build");

    let handle = runtime
        .start_session(support::start_input(&root))
        .await
        .expect("session should start");

    std::fs::write(root.path().join("NOTES.md"), "Project decision").expect("notes should write");
    std::fs::create_dir_all(root.path().join("runtime/notes")).expect("notes dir should exist");
    std::fs::write(
        root.path()
            .join("runtime/notes")
            .join(format!("{}.md", handle.session_id.0)),
        "Current state\nNext action",
    )
    .expect("session notes should write");
    std::fs::create_dir_all(root.path().join("runtime/todos")).expect("todo dir should exist");
    std::fs::write(
        root.path()
            .join("runtime/todos")
            .join(format!("{}.json", handle.session_id.0)),
        r#"[{"content":"Implement cache","activeForm":"Implementing cache","status":"in_progress"}]"#,
    )
    .expect("todos should write");

    let provider = Arc::new(support::ScriptedModelProvider::new(vec![vec![
        octopus_sdk_contracts::AssistantEvent::TextDelta("resumed ok".into()),
        octopus_sdk_contracts::AssistantEvent::MessageStop {
            stop_reason: octopus_sdk_contracts::StopReason::EndTurn,
        },
    ]]));
    let resumed_runtime = support::runtime_builder(provider.clone(), store)
        .build()
        .expect("runtime should build");

    resumed_runtime
        .resume(&handle.session_id)
        .await
        .expect("resume should work");

    let events = support::collect_events(&resumed_runtime, &handle.session_id).await;
    assert!(events.iter().any(|event| matches!(
        event,
        octopus_sdk_contracts::SessionEvent::Render { blocks, .. }
            if blocks.iter().any(|block| {
                block.payload["title"] == "context_restored"
                    && block.payload["rows"].to_string().contains("runtime/notes")
                    && block.payload["rows"].to_string().contains("runtime/todos")
            })
    )));

    resumed_runtime
        .submit_turn(octopus_sdk_core::SubmitTurnInput {
            session_id: handle.session_id.clone(),
            message: support::text_message("continue"),
        })
        .await
        .expect("submit_turn should use restored state");

    let requests = provider.requests();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].messages.iter().any(|message| {
        message.role == octopus_sdk_contracts::Role::System
            && message.content.iter().any(|block| {
                matches!(
                    block,
                    octopus_sdk_contracts::ContentBlock::Text { text }
                        if text.contains("<context_restored>")
                            && text.contains("Project decision")
                            && text.contains("Current state")
                            && text.contains("Implement cache")
                )
            })
    }));
}
