use std::{fs, sync::Arc};

use octopus_infra::build_infra_bundle;
use octopus_platform::ModelRegistryService;
use octopus_runtime_adapter::{CanonicalModelPolicy, MockRuntimeModelDriver, RuntimeAdapter};

fn test_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "octopus-runtime-adapter-canonical-model-policy-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("test root");
    root
}

#[tokio::test]
async fn canonical_policy_and_registry_defaults_match() {
    let policy = CanonicalModelPolicy;
    assert_eq!(policy.default_conversation_model(), "claude-sonnet-4-5");
    assert_eq!(policy.resolve_alias("sonnet"), Some("claude-sonnet-4-5"));
    assert_eq!(
        policy
            .default_selection("responses")
            .expect("responses default"),
        policy.default_responses_selection()
    );

    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        octopus_core::DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let snapshot = adapter.catalog_snapshot().await.expect("catalog snapshot");
    let conversation_default = snapshot
        .default_selections
        .get("conversation")
        .expect("conversation default selection");

    assert_eq!(
        conversation_default.model_id,
        policy.default_conversation_model()
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
