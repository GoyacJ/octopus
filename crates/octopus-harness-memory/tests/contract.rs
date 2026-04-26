#![cfg(any(feature = "builtin", feature = "external-slot"))]

use std::collections::BTreeSet;
#[cfg(all(feature = "builtin", feature = "external-slot"))]
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use harness_contracts::{
    MemoryActor, MemoryId, MemoryKind, MemorySource, MemoryVisibility, SessionId, TenantId,
};
use harness_memory::{
    MemoryKindFilter, MemoryListScope, MemoryMetadata, MemoryQuery, MemoryRecord, MemoryStore,
    MemoryVisibilityFilter,
};

#[cfg(all(feature = "builtin", feature = "external-slot"))]
use harness_memory::MemoryManager;
#[cfg(feature = "external-slot")]
use harness_memory::MockMemoryProvider;
#[cfg(feature = "builtin")]
use harness_memory::{BuiltinMemory, MemdirFile};

#[cfg(feature = "external-slot")]
#[tokio::test]
async fn mock_provider_contract_upserts_lists_and_forgets() {
    let provider = MockMemoryProvider::new("mock-contract");
    let session_id = SessionId::new();
    let kept = record(
        TenantId::SINGLE,
        MemoryVisibility::Private { session_id },
        "tenant scoped memory",
    );
    let other_tenant = record(TenantId::SHARED, MemoryVisibility::Tenant, "other tenant");

    assert_eq!(provider.upsert(kept.clone()).await.unwrap(), kept.id);
    provider.upsert(other_tenant).await.unwrap();

    let recalled = provider
        .recall(query(TenantId::SINGLE, session_id, 8))
        .await
        .unwrap();
    assert_eq!(recalled, vec![kept.clone()]);

    let summaries = provider
        .list(MemoryListScope::ForActor(actor(
            TenantId::SINGLE,
            session_id,
        )))
        .await
        .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, kept.id);

    provider.forget(kept.id).await.unwrap();
    assert!(provider
        .recall(query(TenantId::SINGLE, session_id, 8))
        .await
        .unwrap()
        .is_empty());
}

#[cfg(feature = "external-slot")]
#[tokio::test]
async fn mock_provider_contract_enforces_tenant_isolation() {
    let provider = MockMemoryProvider::new("mock-contract");
    let session_id = SessionId::new();
    let kept = record(TenantId::SINGLE, MemoryVisibility::Tenant, "single tenant");
    let leaked = record(TenantId::SHARED, MemoryVisibility::Tenant, "shared tenant");

    provider.upsert(kept.clone()).await.unwrap();
    provider.upsert(leaked).await.unwrap();

    let recalled = provider
        .recall(query(TenantId::SINGLE, session_id, 8))
        .await
        .unwrap();

    assert_eq!(recalled, vec![kept]);
}

#[cfg(feature = "external-slot")]
#[tokio::test]
async fn mock_provider_contract_enforces_private_visibility() {
    let provider = MockMemoryProvider::new("mock-contract");
    let session_id = SessionId::new();
    let visible = record(
        TenantId::SINGLE,
        MemoryVisibility::Private { session_id },
        "same session",
    );
    let hidden = record(
        TenantId::SINGLE,
        MemoryVisibility::Private {
            session_id: SessionId::new(),
        },
        "other session",
    );

    provider.upsert(visible.clone()).await.unwrap();
    provider.upsert(hidden).await.unwrap();

    let recalled = provider
        .recall(query(TenantId::SINGLE, session_id, 8))
        .await
        .unwrap();

    assert_eq!(recalled, vec![visible]);
}

#[cfg(all(feature = "builtin", feature = "external-slot"))]
#[tokio::test]
async fn builtin_memdir_does_not_participate_in_external_recall() {
    let root = tempfile::tempdir().unwrap();
    let builtin = BuiltinMemory::at(root.path(), TenantId::SINGLE);
    builtin
        .append_section(MemdirFile::Memory, "profile", "prefers concise answers")
        .await
        .unwrap();

    let manager = MemoryManager::new();
    let session_id = SessionId::new();

    assert!(manager
        .recall(query(TenantId::SINGLE, session_id, 8))
        .await
        .unwrap()
        .is_empty());

    let external = Arc::new(MockMemoryProvider::new("mock-contract"));
    external
        .upsert(record(
            TenantId::SINGLE,
            MemoryVisibility::Private { session_id },
            "external preference",
        ))
        .await
        .unwrap();
    manager.set_external(external).unwrap();

    let recalled = manager
        .recall(query(TenantId::SINGLE, session_id, 8))
        .await
        .unwrap();
    assert_eq!(recalled.len(), 1);
    assert_eq!(recalled[0].content, "external preference");
}

fn query(tenant_id: TenantId, session_id: SessionId, max_records: u32) -> MemoryQuery {
    MemoryQuery {
        text: "memory".to_owned(),
        kind_filter: Some(MemoryKindFilter::OnlyKinds(BTreeSet::from([
            MemoryKind::UserPreference,
        ]))),
        visibility_filter: MemoryVisibilityFilter::EffectiveFor(actor(tenant_id, session_id)),
        max_records,
        min_similarity: 0.0,
        tenant_id,
        session_id: Some(session_id),
        deadline: Some(Duration::from_millis(200)),
    }
}

fn actor(tenant_id: TenantId, session_id: SessionId) -> MemoryActor {
    MemoryActor {
        tenant_id,
        user_id: Some("user-1".to_owned()),
        team_id: None,
        session_id: Some(session_id),
    }
}

fn record(tenant_id: TenantId, visibility: MemoryVisibility, content: &str) -> MemoryRecord {
    let now = Utc::now();
    MemoryRecord {
        id: MemoryId::new(),
        tenant_id,
        kind: MemoryKind::UserPreference,
        visibility,
        content: content.to_owned(),
        metadata: MemoryMetadata {
            tags: Vec::new(),
            source: MemorySource::UserInput,
            confidence: 1.0,
            access_count: 0,
            last_accessed_at: None,
            recall_score: 1.0,
            ttl: None,
            redacted_segments: 0,
        },
        created_at: now,
        updated_at: now,
    }
}
