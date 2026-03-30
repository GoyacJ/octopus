use std::{
    fs,
    path::{Path, PathBuf},
};

use octopus_governance::{
    ModelCatalogItemRecord, ModelProfileRecord, ModelProviderRecord, SqliteGovernanceStore,
    TenantModelPolicyRecord,
};
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

static MIGRATOR: Migrator = sqlx::migrate!("../runtime/migrations");

async fn open_store(path: &Path) -> SqliteGovernanceStore {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .unwrap();
    MIGRATOR.run(&pool).await.unwrap();
    SqliteGovernanceStore::new(pool)
}

fn sample_db_path(base: &Path) -> PathBuf {
    base.join("model-governance.sqlite")
}

#[tokio::test]
async fn model_governance_records_round_trip_through_governance_store() {
    let tempdir = tempfile::tempdir().unwrap();
    let store = open_store(&sample_db_path(tempdir.path())).await;

    let provider = ModelProviderRecord {
        id: "provider-openai".to_string(),
        display_name: "OpenAI".to_string(),
        provider_family: "openai".to_string(),
        status: "active".to_string(),
        default_base_url: Some("https://api.openai.com/v1".to_string()),
        protocol_families: vec!["openai_responses_compatible".to_string()],
        created_at: "2026-03-30T10:00:00Z".to_string(),
        updated_at: "2026-03-30T10:00:00Z".to_string(),
    };
    let catalog_item = ModelCatalogItemRecord {
        id: "catalog-openai-gpt-5-4".to_string(),
        provider_id: provider.id.clone(),
        model_key: "openai:gpt-5.4".to_string(),
        provider_model_id: "gpt-5.4".to_string(),
        release_channel: "ga".to_string(),
        modality_tags: vec!["text_in".to_string(), "text_out".to_string()],
        feature_tags: vec![
            "supports_structured_output".to_string(),
            "supports_builtin_web_search".to_string(),
        ],
        context_window: 1_050_000,
        max_output_tokens: Some(128_000),
        created_at: "2026-03-30T10:00:00Z".to_string(),
        updated_at: "2026-03-30T10:00:00Z".to_string(),
    };
    let profile = ModelProfileRecord {
        id: "profile-default-reasoning".to_string(),
        display_name: "Default Reasoning".to_string(),
        scope_ref: "tenant:tenant-alpha".to_string(),
        primary_model_key: catalog_item.model_key.clone(),
        fallback_model_keys: vec!["openai:gpt-5.4-mini".to_string()],
        created_at: "2026-03-30T10:00:00Z".to_string(),
        updated_at: "2026-03-30T10:00:00Z".to_string(),
    };
    let tenant_policy = TenantModelPolicyRecord {
        id: "tenant-policy-default".to_string(),
        tenant_id: "tenant-alpha".to_string(),
        allowed_model_keys: vec![
            catalog_item.model_key.clone(),
            "openai:gpt-5.4-mini".to_string(),
        ],
        denied_model_keys: vec![],
        allowed_provider_ids: vec![provider.id.clone()],
        denied_release_channels: vec!["experimental".to_string()],
        require_approval_for_preview: true,
        created_at: "2026-03-30T10:00:00Z".to_string(),
        updated_at: "2026-03-30T10:00:00Z".to_string(),
    };

    store.upsert_model_provider(&provider).await.unwrap();
    store
        .upsert_model_catalog_item(&catalog_item)
        .await
        .unwrap();
    store.upsert_model_profile(&profile).await.unwrap();
    store
        .upsert_tenant_model_policy(&tenant_policy)
        .await
        .unwrap();

    assert_eq!(
        store.fetch_model_provider(&provider.id).await.unwrap(),
        Some(provider.clone())
    );
    assert_eq!(
        store
            .fetch_model_catalog_item(&catalog_item.id)
            .await
            .unwrap(),
        Some(catalog_item.clone())
    );
    assert_eq!(
        store.fetch_model_profile(&profile.id).await.unwrap(),
        Some(profile.clone())
    );
    assert_eq!(
        store
            .fetch_tenant_model_policy(&tenant_policy.tenant_id)
            .await
            .unwrap(),
        Some(tenant_policy.clone())
    );

    assert_eq!(store.list_model_providers().await.unwrap(), vec![provider]);
    assert_eq!(
        store.list_model_catalog_items().await.unwrap(),
        vec![catalog_item]
    );
    assert_eq!(store.list_model_profiles().await.unwrap(), vec![profile]);
    assert_eq!(
        store.list_tenant_model_policies().await.unwrap(),
        vec![tenant_policy]
    );
}
