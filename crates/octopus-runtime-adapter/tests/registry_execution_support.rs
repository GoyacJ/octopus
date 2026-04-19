use std::{fs, sync::Arc};

use octopus_core::RuntimeExecutionClass;
use octopus_infra::build_infra_bundle;
use octopus_platform::ModelRegistryService;
use octopus_runtime_adapter::{MockRuntimeModelDriver, RuntimeAdapter};

fn test_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "octopus-runtime-adapter-registry-execution-support-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("test root");
    root
}

#[tokio::test]
async fn catalog_marks_execution_profile_per_surface() {
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
    let minimax = snapshot
        .models
        .iter()
        .find(|model| model.model_id == "MiniMax-M2.7")
        .expect("minimax model");
    let anthropic_binding = minimax
        .surface_bindings
        .iter()
        .find(|binding| binding.protocol_family == "anthropic_messages")
        .expect("anthropic surface binding");
    let vendor_native_binding = minimax
        .surface_bindings
        .iter()
        .find(|binding| binding.protocol_family == "vendor_native")
        .expect("vendor native surface binding");
    let gpt54 = snapshot
        .models
        .iter()
        .find(|model| model.model_id == "gpt-5.4")
        .expect("gpt-5.4 model");
    let responses_binding = gpt54
        .surface_bindings
        .iter()
        .find(|binding| binding.protocol_family == "openai_responses")
        .expect("responses surface binding");

    assert_eq!(
        anthropic_binding.execution_profile.execution_class,
        RuntimeExecutionClass::AgentConversation
    );
    assert!(anthropic_binding.execution_profile.tool_loop);
    assert!(anthropic_binding.execution_profile.upstream_streaming);

    assert_eq!(
        responses_binding.execution_profile.execution_class,
        RuntimeExecutionClass::SingleShotGeneration
    );
    assert!(!responses_binding.execution_profile.tool_loop);
    assert!(!responses_binding.execution_profile.upstream_streaming);

    assert_eq!(
        vendor_native_binding.execution_profile.execution_class,
        RuntimeExecutionClass::Unsupported
    );
    assert!(!vendor_native_binding.execution_profile.tool_loop);
    assert!(!vendor_native_binding.execution_profile.upstream_streaming);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
