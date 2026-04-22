use std::sync::Arc;

use octopus_sdk_contracts::{PluginSourceTag, PluginSummary, PluginsSnapshot};
use octopus_sdk_core::{AgentRuntime, RuntimeError};

mod support;

#[tokio::test]
async fn test_builder_requires_runtime_dependencies() {
    let error = AgentRuntime::builder()
        .build()
        .err()
        .expect("builder without deps should fail");

    assert!(matches!(
        error,
        RuntimeError::MissingBuilderField {
            field: "session_store"
        }
    ));
}

#[tokio::test]
async fn test_builder_rejects_plugin_snapshot_mismatch() {
    let (_root, store) = support::temp_store();
    let model = Arc::new(support::ScriptedModelProvider::new(vec![vec![]]));
    let error = support::runtime_builder(model, store)
        .with_plugins_snapshot(PluginsSnapshot {
            api_version: "1.0.0".into(),
            plugins: vec![PluginSummary {
                id: "mismatch".into(),
                version: "0.1.0".into(),
                git_sha: None,
                source: PluginSourceTag::Local,
                enabled: true,
                components_count: 1,
            }],
        })
        .build()
        .err()
        .expect("snapshot mismatch should fail");

    assert!(matches!(error, RuntimeError::PluginSnapshotMismatch));
}
