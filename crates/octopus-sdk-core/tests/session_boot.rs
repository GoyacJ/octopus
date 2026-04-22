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
