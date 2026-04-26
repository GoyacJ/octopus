#![cfg(feature = "external-slot")]

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use harness_contracts::{
    MemoryActor, MemoryError, MemoryId, MemoryKind, MemorySource, MemoryVisibility, SessionId,
    TenantId,
};
use harness_memory::{
    MemoryKindFilter, MemoryListScope, MemoryManager, MemoryMetadata, MemoryQuery, MemoryRecord,
    MemoryStore, MemoryVisibilityFilter, MockMemoryProvider,
};

#[test]
fn memory_manager_accepts_only_one_external_provider() {
    let manager = MemoryManager::new();

    assert!(!manager.has_external());
    manager
        .set_external(Arc::new(MockMemoryProvider::new("first")))
        .unwrap();

    assert!(manager.has_external());
    assert_eq!(manager.external().unwrap().provider_id(), "first");

    let error = manager
        .set_external(Arc::new(MockMemoryProvider::new("second")))
        .unwrap_err();
    assert!(
        matches!(error, MemoryError::Message(message) if message.contains("external memory provider slot occupied"))
    );
    assert_eq!(manager.external().unwrap().provider_id(), "first");
}

#[tokio::test]
async fn mock_memory_provider_is_tenant_scoped_and_supports_forget() {
    let provider = MockMemoryProvider::new("mock");
    let session_id = SessionId::new();
    let kept = record(
        TenantId::SINGLE,
        MemoryVisibility::Private { session_id },
        "single private preference",
    );
    let leaked = record(
        TenantId::SHARED,
        MemoryVisibility::Tenant,
        "shared tenant fact",
    );

    provider.upsert(kept.clone()).await.unwrap();
    provider.upsert(leaked).await.unwrap();

    let recalled = provider
        .recall(query(TenantId::SINGLE, session_id, 5))
        .await
        .unwrap();
    assert_eq!(recalled, vec![kept.clone()]);

    assert_eq!(
        provider
            .list(MemoryListScope::ForActor(actor(
                TenantId::SINGLE,
                session_id
            )))
            .await
            .unwrap()
            .len(),
        1
    );

    provider.forget(kept.id).await.unwrap();
    assert!(provider
        .list(MemoryListScope::ForActor(actor(
            TenantId::SINGLE,
            session_id
        )))
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn mock_memory_provider_applies_query_limits_and_filters() {
    let provider = MockMemoryProvider::new("mock");
    let session_id = SessionId::new();
    let private = record(
        TenantId::SINGLE,
        MemoryVisibility::Private { session_id },
        "private note",
    );
    let tenant = record(TenantId::SINGLE, MemoryVisibility::Tenant, "tenant note");
    let other_session = record(
        TenantId::SINGLE,
        MemoryVisibility::Private {
            session_id: SessionId::new(),
        },
        "other session note",
    );

    provider.upsert(private.clone()).await.unwrap();
    provider.upsert(tenant.clone()).await.unwrap();
    provider.upsert(other_session).await.unwrap();

    let recalled = provider
        .recall(query(TenantId::SINGLE, session_id, 1))
        .await
        .unwrap();
    assert_eq!(recalled, vec![private]);

    let summaries = provider
        .list(MemoryListScope::ByVisibility(MemoryVisibility::Tenant))
        .await
        .unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].content_preview, tenant.content);
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
