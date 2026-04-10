use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::json::JsonValue;
use crate::usage::TokenUsage;

use super::session_store::{cleanup_rotated_logs, rotate_session_file_if_needed};
use super::{
    ContentBlock, ConversationMessage, MessageRole, Session, SessionFork, MAX_ROTATED_FILES,
    ROTATE_AFTER_BYTES,
};

#[test]
fn persists_and_restores_session_jsonl() {
    let mut session = Session::new();
    session
        .push_user_text("hello")
        .expect("user message should append");
    session
        .push_message(ConversationMessage::assistant_with_usage(
            vec![
                ContentBlock::Text {
                    text: "thinking".to_string(),
                },
                ContentBlock::ToolUse {
                    id: "tool-1".to_string(),
                    name: "bash".to_string(),
                    input: "echo hi".to_string(),
                },
            ],
            Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 4,
                cache_creation_input_tokens: 1,
                cache_read_input_tokens: 2,
            }),
        ))
        .expect("assistant message should append");
    session
        .push_message(ConversationMessage::tool_result(
            "tool-1", "bash", "hi", false,
        ))
        .expect("tool result should append");

    let path = temp_session_path("jsonl");
    session.save_to_path(&path).expect("session should save");
    let restored = Session::load_from_path(&path).expect("session should load");
    fs::remove_file(&path).expect("temp file should be removable");

    assert_eq!(restored, session);
    assert_eq!(restored.messages[2].role, MessageRole::Tool);
    assert_eq!(
        restored.messages[1].usage.expect("usage").total_tokens(),
        17
    );
    assert_eq!(restored.session_id, session.session_id);
}

#[test]
fn loads_legacy_session_json_object() {
    let path = temp_session_path("legacy");
    let legacy = JsonValue::Object(
        [
            ("version".to_string(), JsonValue::Number(1)),
            (
                "messages".to_string(),
                JsonValue::Array(vec![ConversationMessage::user_text("legacy").to_json()]),
            ),
        ]
        .into_iter()
        .collect(),
    );
    fs::write(&path, legacy.render()).expect("legacy file should write");

    let restored = Session::load_from_path(&path).expect("legacy session should load");
    fs::remove_file(&path).expect("temp file should be removable");

    assert_eq!(restored.messages.len(), 1);
    assert_eq!(
        restored.messages[0],
        ConversationMessage::user_text("legacy")
    );
    assert!(!restored.session_id.is_empty());
}

#[test]
fn appends_messages_to_persisted_jsonl_session() {
    let path = temp_session_path("append");
    let mut session = Session::new().with_persistence_path(path.clone());
    session
        .save_to_path(&path)
        .expect("initial save should succeed");
    session
        .push_user_text("hi")
        .expect("user append should succeed");
    session
        .push_message(ConversationMessage::assistant(vec![ContentBlock::Text {
            text: "hello".to_string(),
        }]))
        .expect("assistant append should succeed");

    let restored = Session::load_from_path(&path).expect("session should replay from jsonl");
    fs::remove_file(&path).expect("temp file should be removable");

    assert_eq!(restored.messages.len(), 2);
    assert_eq!(restored.messages[0], ConversationMessage::user_text("hi"));
}

#[test]
fn persists_compaction_metadata() {
    let path = temp_session_path("compaction");
    let mut session = Session::new();
    session
        .push_user_text("before")
        .expect("message should append");
    session.record_compaction("summarized earlier work", 4);
    session.save_to_path(&path).expect("session should save");

    let restored = Session::load_from_path(&path).expect("session should load");
    fs::remove_file(&path).expect("temp file should be removable");

    let compaction = restored.compaction.expect("compaction metadata");
    assert_eq!(compaction.count, 1);
    assert_eq!(compaction.removed_message_count, 4);
    assert!(compaction.summary.contains("summarized"));
}

#[test]
fn forks_sessions_with_branch_metadata_and_persists_it() {
    let path = temp_session_path("fork");
    let mut session = Session::new();
    session
        .push_user_text("before fork")
        .expect("message should append");

    let forked = session
        .fork(Some("investigation".to_string()))
        .with_persistence_path(path.clone());
    forked
        .save_to_path(&path)
        .expect("forked session should save");

    let restored = Session::load_from_path(&path).expect("forked session should load");
    fs::remove_file(&path).expect("temp file should be removable");

    assert_ne!(restored.session_id, session.session_id);
    assert_eq!(
        restored.fork,
        Some(SessionFork {
            parent_session_id: session.session_id,
            branch_name: Some("investigation".to_string()),
        })
    );
    assert_eq!(restored.messages, forked.messages);
}

#[test]
fn rotates_and_cleans_up_large_session_logs() {
    let path = temp_session_path("rotation");
    let oversized_length =
        usize::try_from(ROTATE_AFTER_BYTES + 10).expect("rotate threshold should fit");
    fs::write(&path, "x".repeat(oversized_length)).expect("oversized file should write");

    rotate_session_file_if_needed(&path).expect("rotation should succeed");

    assert!(
        !path.exists(),
        "original path should be rotated away before rewrite"
    );

    for _ in 0..5 {
        let rotated = super::session_store::rotated_log_path(&path);
        fs::write(&rotated, "old").expect("rotated file should write");
    }
    cleanup_rotated_logs(&path).expect("cleanup should succeed");

    let rotated_count = rotation_files(&path).len();
    assert!(rotated_count <= MAX_ROTATED_FILES);
    for rotated in rotation_files(&path) {
        fs::remove_file(rotated).expect("rotated file should be removable");
    }
}

#[test]
fn rejects_jsonl_record_without_type() {
    let path = write_temp_session_file(
        "missing-type",
        r#"{"message":{"role":"user","blocks":[{"type":"text","text":"hello"}]}}"#,
    );

    let error = Session::load_from_path(&path)
        .expect_err("session should reject JSONL records without a type");

    assert!(error.to_string().contains("missing type"));
    fs::remove_file(path).expect("temp file should be removable");
}

#[test]
fn rejects_jsonl_message_record_without_message_payload() {
    let path = write_temp_session_file("missing-message", r#"{"type":"message"}"#);

    let error = Session::load_from_path(&path)
        .expect_err("session should reject JSONL message records without message payload");

    assert!(error.to_string().contains("missing message"));
    fs::remove_file(path).expect("temp file should be removable");
}

#[test]
fn rejects_jsonl_record_with_unknown_type() {
    let path = write_temp_session_file("unknown-type", r#"{"type":"mystery"}"#);

    let error = Session::load_from_path(&path)
        .expect_err("session should reject unknown JSONL record types");

    assert!(error.to_string().contains("unsupported JSONL record type"));
    fs::remove_file(path).expect("temp file should be removable");
}

#[test]
fn rejects_legacy_session_json_without_messages() {
    let session = JsonValue::Object(
        [("version".to_string(), JsonValue::Number(1))]
            .into_iter()
            .collect(),
    );

    let error =
        Session::from_json(&session).expect_err("legacy session objects should require messages");

    assert!(error.to_string().contains("missing messages"));
}

#[test]
fn normalizes_blank_fork_branch_name_to_none() {
    let session = Session::new();

    let forked = session.fork(Some("   ".to_string()));

    assert_eq!(forked.fork.expect("fork metadata").branch_name, None);
}

#[test]
fn rejects_unknown_content_block_type() {
    let block = JsonValue::Object(
        [("type".to_string(), JsonValue::String("unknown".to_string()))]
            .into_iter()
            .collect(),
    );

    let error =
        ContentBlock::from_json(&block).expect_err("content blocks should reject unknown types");

    assert!(error.to_string().contains("unsupported block type"));
}

#[test]
fn preserves_workspace_root_across_json_round_trip() {
    let workspace_root = std::env::temp_dir().join("session-workspace-root-json");
    let session = Session::new().with_workspace_root(&workspace_root);

    let json = session.to_json().expect("session should serialize");
    let restored = Session::from_json(&json).expect("session should deserialize");

    assert_eq!(restored.workspace_root(), Some(workspace_root.as_path()));
}

#[test]
fn persists_workspace_root_in_jsonl_snapshot_and_forks() {
    let path = temp_session_path("workspace-root");
    let workspace_root = std::env::temp_dir().join("session-workspace-root-jsonl");
    let mut session = Session::new().with_workspace_root(&workspace_root);
    session
        .push_user_text("bound to workspace")
        .expect("message should append");
    session.save_to_path(&path).expect("session should save");

    let restored = Session::load_from_path(&path).expect("session should load");
    let forked = restored.fork(Some("workspace-fork".to_string()));
    fs::remove_file(&path).expect("temp file should be removable");

    assert_eq!(restored.workspace_root(), Some(workspace_root.as_path()));
    assert_eq!(forked.workspace_root(), Some(workspace_root.as_path()));
}

fn temp_session_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("runtime-session-{label}-{nanos}.json"))
}

fn write_temp_session_file(label: &str, contents: &str) -> PathBuf {
    let path = temp_session_path(label);
    fs::write(&path, format!("{contents}\n")).expect("temp session file should write");
    path
}

fn rotation_files(path: &Path) -> Vec<PathBuf> {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .expect("temp path should have file stem")
        .to_string();
    fs::read_dir(path.parent().expect("temp path should have parent"))
        .expect("temp dir should read")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|entry_path| {
            entry_path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| {
                    name.starts_with(&format!("{stem}.rot-"))
                        && Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("jsonl"))
                })
        })
        .collect()
}
