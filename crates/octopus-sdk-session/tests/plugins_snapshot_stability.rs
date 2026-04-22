use std::fs;

use octopus_sdk_contracts::{
    PluginSourceTag, PluginSummary, PluginsSnapshot, SessionEvent, SessionId,
};
use octopus_sdk_plugin::{PluginCompat, PluginManifest, PluginRegistry};
use octopus_sdk_session::{SessionStore, SqliteJsonlSessionStore};
use uuid::Uuid;

#[tokio::test]
async fn test_append_session_started() {
    let (root, db_path, jsonl_root) = temp_paths("append-started");
    let store = SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");
    let session_id = SessionId("session-plugins-embedded".into());
    let plugins_snapshot = sample_plugins_snapshot();

    store
        .append_session_started(
            &session_id,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-embedded".into(),
            "hash-embedded".into(),
            8_192,
            Some(plugins_snapshot.clone()),
        )
        .await
        .expect("session started should append");

    let json = fs::read_to_string(jsonl_root.join(format!("{}.jsonl", session_id.0)))
        .expect("jsonl should exist");
    assert!(json.contains("\"kind\":\"session_started\""));
    assert!(json.contains("\"plugins_snapshot\""));
    assert!(json.contains("\"api_version\":\"1.0.0\""));
    assert!(json.contains("\"id\":\"alpha-tool\""));

    let snapshot = store
        .snapshot(&session_id)
        .await
        .expect("snapshot should load");
    assert_eq!(snapshot.plugins_snapshot, plugins_snapshot);

    drop(store);
    fs::remove_dir_all(root).expect("temp root should delete");
}

#[tokio::test]
async fn test_append_session_plugins_snapshot() {
    let (root, db_path, jsonl_root) = temp_paths("append-second-event");
    let store = SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");
    let session_id = SessionId("session-plugins-second-event".into());
    let plugins_snapshot = sample_plugins_snapshot();

    store
        .append_session_started(
            &session_id,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-fallback".into(),
            "hash-fallback".into(),
            8_192,
            None,
        )
        .await
        .expect("session started should append");
    store
        .append(
            &session_id,
            SessionEvent::SessionPluginsSnapshot {
                plugins_snapshot: plugins_snapshot.clone(),
            },
        )
        .await
        .expect("plugins snapshot event should append");

    let path = jsonl_root.join(format!("{}.jsonl", session_id.0));
    let lines = fs::read_to_string(path)
        .expect("jsonl should exist")
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].contains("\"kind\":\"session_started\""));
    assert!(lines[0].contains("\"plugins_snapshot\":null"));
    assert!(lines[1].contains("\"kind\":\"session_plugins_snapshot\""));
    assert!(lines[1].contains("\"id\":\"alpha-tool\""));

    let snapshot = store
        .snapshot(&session_id)
        .await
        .expect("snapshot should load");
    assert_eq!(snapshot.plugins_snapshot, plugins_snapshot);

    drop(store);
    fs::remove_dir_all(root).expect("temp root should delete");
}

#[test]
fn test_snapshot_byte_stable() {
    let mut registry = PluginRegistry::new();
    registry
        .register_plugin(manifest("beta-tool", "0.2.0"), PluginSourceTag::Local)
        .expect("beta should register");
    registry
        .register_plugin(manifest("alpha-tool", "0.1.0"), PluginSourceTag::Local)
        .expect("alpha should register");

    let first = serde_json::to_vec(&registry.get_snapshot()).expect("snapshot should serialize");
    let second = serde_json::to_vec(&registry.get_snapshot()).expect("snapshot should serialize");
    let third = serde_json::to_vec(&registry.get_snapshot()).expect("snapshot should serialize");

    assert_eq!(first, second);
    assert_eq!(second, third);

    let snapshot: PluginsSnapshot =
        serde_json::from_slice(&first).expect("snapshot should deserialize");
    let ids = snapshot
        .plugins
        .iter()
        .map(|plugin| plugin.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["alpha-tool", "beta-tool"]);
}

#[tokio::test]
async fn test_snapshot_replay_embedded() {
    let (root, db_path, jsonl_root) = temp_paths("replay-embedded");
    let session_id = SessionId("session-replay-embedded".into());
    let plugins_snapshot = sample_plugins_snapshot();

    let store = SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");
    store
        .append_session_started(
            &session_id,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-replay-a".into(),
            "hash-replay-a".into(),
            8_192,
            Some(plugins_snapshot.clone()),
        )
        .await
        .expect("session started should append");
    drop(store);

    fs::remove_file(&db_path).expect("db file should delete");
    let reopened =
        SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should reopen");
    let snapshot = reopened
        .snapshot(&session_id)
        .await
        .expect("snapshot should replay from jsonl");

    assert_eq!(snapshot.plugins_snapshot, plugins_snapshot);

    drop(reopened);
    fs::remove_dir_all(root).expect("temp root should delete");
}

#[tokio::test]
async fn test_snapshot_replay_second_event() {
    let (root, db_path, jsonl_root) = temp_paths("replay-second-event");
    let session_id = SessionId("session-replay-second-event".into());
    let plugins_snapshot = sample_plugins_snapshot();

    let store = SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should open");
    store
        .append_session_started(
            &session_id,
            std::path::PathBuf::from("."),
            octopus_sdk_contracts::PermissionMode::Default,
            "main".into(),
            "cfg-replay-b".into(),
            "hash-replay-b".into(),
            8_192,
            None,
        )
        .await
        .expect("session started should append");
    store
        .append(
            &session_id,
            SessionEvent::SessionPluginsSnapshot {
                plugins_snapshot: plugins_snapshot.clone(),
            },
        )
        .await
        .expect("plugins snapshot event should append");
    drop(store);

    fs::remove_file(&db_path).expect("db file should delete");
    let reopened =
        SqliteJsonlSessionStore::open(&db_path, &jsonl_root).expect("store should reopen");
    let snapshot = reopened
        .snapshot(&session_id)
        .await
        .expect("snapshot should replay from jsonl");

    assert_eq!(snapshot.plugins_snapshot, plugins_snapshot);

    drop(reopened);
    fs::remove_dir_all(root).expect("temp root should delete");
}

fn temp_paths(label: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "octopus-sdk-session-plugins-snapshot-{label}-{}",
        Uuid::new_v4()
    ));
    let db_path = root.join("data").join("main.db");
    let jsonl_root = root.join("runtime").join("events");
    fs::create_dir_all(db_path.parent().expect("db parent")).expect("db dir should exist");
    (root, db_path, jsonl_root)
}

fn manifest(id: &str, version: &str) -> PluginManifest {
    PluginManifest {
        id: id.into(),
        version: version.into(),
        git_sha: Some(format!("{id:0<40}").chars().take(40).collect()),
        source: PluginSourceTag::Local,
        compat: PluginCompat {
            plugin_api: "^1.0.0".into(),
        },
        components: Vec::new(),
    }
}

fn sample_plugins_snapshot() -> PluginsSnapshot {
    PluginsSnapshot {
        api_version: "1.0.0".into(),
        plugins: vec![
            PluginSummary {
                id: "alpha-tool".into(),
                version: "0.1.0".into(),
                git_sha: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
                source: PluginSourceTag::Bundled,
                enabled: true,
                components_count: 1,
            },
            PluginSummary {
                id: "beta-tool".into(),
                version: "0.2.0".into(),
                git_sha: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into()),
                source: PluginSourceTag::Local,
                enabled: true,
                components_count: 2,
            },
        ],
    }
}
