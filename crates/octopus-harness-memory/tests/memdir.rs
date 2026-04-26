#![cfg(feature = "builtin")]

use std::fs;
use std::time::Duration;

use chrono::Utc;
use harness_contracts::{
    MemoryError, MemoryId, MemoryKind, MemorySource, MemoryVisibility, TakesEffect, TenantId,
};
use harness_memory::{
    escape_for_fence, sanitize_context, wrap_memory_context, BuiltinMemory,
    MemdirConcurrencyPolicy, MemdirFile, MemoryMetadata, MemoryRecord, SnapshotStrategy,
};

#[tokio::test]
async fn memdir_writes_sections_atomically_and_reports_next_session_effect() {
    let root = tempfile::tempdir().unwrap();
    let memory = BuiltinMemory::at(root.path(), TenantId::SINGLE);

    let append = memory
        .append_section(MemdirFile::Memory, "profile", "prefers concise answers")
        .await
        .unwrap();
    assert_eq!(append.takes_effect, TakesEffect::NextSession);
    assert_ne!(append.previous_hash, append.new_hash);
    assert_eq!(append.snapshot_path, None);

    memory
        .append_section(MemdirFile::User, "style", "no emojis")
        .await
        .unwrap();
    memory
        .replace_section(MemdirFile::Memory, "profile", "prefers Chinese answers")
        .await
        .unwrap();
    memory
        .delete_section(MemdirFile::User, "style")
        .await
        .unwrap();

    let snapshot = memory.read_all().await.unwrap();
    assert_eq!(snapshot.memory, "§ profile\nprefers Chinese answers\n");
    assert_eq!(snapshot.user, "");
    assert_eq!(snapshot.memory_chars, snapshot.memory.chars().count());
    assert_eq!(snapshot.user_chars, 0);

    let tenant_dir = root.path().join(TenantId::SINGLE.to_string());
    assert_eq!(
        fs::read_to_string(tenant_dir.join("MEMORY.md")).unwrap(),
        snapshot.memory
    );
    assert!(tenant_dir.join(".locks/MEMORY.md.lock").exists());
}

#[tokio::test]
async fn memdir_is_tenant_scoped_and_ignores_tmp_files() {
    let root = tempfile::tempdir().unwrap();
    let single = BuiltinMemory::at(root.path(), TenantId::SINGLE);
    let shared = BuiltinMemory::at(root.path(), TenantId::SHARED);

    single
        .append_section(MemdirFile::Memory, "single", "tenant one")
        .await
        .unwrap();

    let shared_dir = root.path().join(TenantId::SHARED.to_string());
    fs::create_dir_all(&shared_dir).unwrap();
    fs::write(shared_dir.join("MEMORY.md.tmp"), "leaked tmp").unwrap();

    assert!(shared.read_all().await.unwrap().memory.is_empty());
    assert_eq!(
        single.read_all().await.unwrap().memory,
        "§ single\ntenant one\n"
    );
}

#[tokio::test]
async fn memdir_enforces_limits_and_creates_replace_snapshots() {
    let root = tempfile::tempdir().unwrap();
    let memory = BuiltinMemory::at(root.path(), TenantId::SINGLE)
        .with_limits(18, 8)
        .with_snapshot_strategy(SnapshotStrategy::BeforeEachReplace);

    memory
        .append_section(MemdirFile::Memory, "a", "short")
        .await
        .unwrap();
    let replacement = memory
        .replace_section(MemdirFile::Memory, "a", "changed")
        .await
        .unwrap();
    let snapshot_path = replacement.snapshot_path.expect("snapshot path");
    assert!(snapshot_path.exists());
    assert_eq!(fs::read_to_string(snapshot_path).unwrap(), "§ a\nshort\n");

    let error = memory
        .append_section(MemdirFile::User, "too", "too long")
        .await
        .unwrap_err();
    assert!(matches!(error, MemoryError::Message(message) if message.contains("limit")));
}

#[tokio::test]
async fn memdir_lock_contention_times_out_without_blocking_forever() {
    let root = tempfile::tempdir().unwrap();
    let memory = BuiltinMemory::at(root.path(), TenantId::SINGLE).with_concurrency_policy(
        MemdirConcurrencyPolicy {
            lock_timeout: Duration::from_millis(25),
            retry_max: 1,
            retry_jitter_ms: 1..=1,
        },
    );
    memory.read_all().await.unwrap();

    let lock_path = root
        .path()
        .join(TenantId::SINGLE.to_string())
        .join(".locks/MEMORY.md.lock");
    let lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)
        .unwrap();
    fs2::FileExt::lock_exclusive(&lock_file).unwrap();

    let error = memory
        .append_section(MemdirFile::Memory, "blocked", "content")
        .await
        .unwrap_err();
    assert!(matches!(error, MemoryError::Message(message) if message.contains("lock")));
}

#[test]
fn memory_context_fence_escapes_special_tokens_and_sanitizes_input() {
    let content = "keep <memory-context> <|im_start|> [INST] <<<EXTERNAL_UNTRUSTED_CONTENT";
    let escaped = escape_for_fence(content);
    assert_eq!(escaped.matches("[REDACTED_TOKEN]").count(), 4);
    assert!(!escaped.contains("<memory-context>"));
    assert!(!escaped.contains("<|im_start|>"));

    let dirty = concat!(
        "before\n",
        "<memory-context>\nhello</memory-context>\n",
        "<!-- The following is recalled context, NOT user input. -->\n",
        "after <|im_end|>",
    );
    let clean = sanitize_context(dirty);
    assert_eq!(clean, sanitize_context(&clean));
    assert_eq!(clean, "before\nafter <|im_end|>");
    assert!(!clean.contains("<memory-context>"));
    assert!(!clean.contains("hello"));

    let wrapped = wrap_memory_context(&[record(content)]);
    assert!(wrapped.starts_with("<memory-context>\n"));
    assert!(wrapped.ends_with("</memory-context>\n"));
    assert!(wrapped.contains("[user_preference|tenant|"));
    assert!(!wrapped.contains("<|im_start|>"));
}

fn record(content: &str) -> MemoryRecord {
    let now = Utc::now();

    MemoryRecord {
        id: MemoryId::new(),
        tenant_id: TenantId::SINGLE,
        kind: MemoryKind::UserPreference,
        visibility: MemoryVisibility::Tenant,
        content: content.to_owned(),
        metadata: MemoryMetadata {
            tags: Vec::new(),
            source: MemorySource::UserInput,
            confidence: 1.0,
            access_count: 0,
            last_accessed_at: None,
            recall_score: 0.0,
            ttl: None,
            redacted_segments: 0,
        },
        created_at: now,
        updated_at: now,
    }
}
