use std::{fs, sync::Arc};

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
async fn catalog_marks_tool_runtime_support_per_surface() {
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

    assert!(anthropic_binding.runtime_support.prompt);
    assert!(anthropic_binding.runtime_support.conversation);
    assert!(anthropic_binding.runtime_support.tool_loop);
    assert!(!anthropic_binding.runtime_support.streaming);

    assert!(responses_binding.runtime_support.prompt);
    assert!(responses_binding.runtime_support.conversation);
    assert!(!responses_binding.runtime_support.tool_loop);
    assert!(!responses_binding.runtime_support.streaming);

    assert!(!vendor_native_binding.runtime_support.prompt);
    assert!(!vendor_native_binding.runtime_support.conversation);
    assert!(!vendor_native_binding.runtime_support.tool_loop);
    assert!(!vendor_native_binding.runtime_support.streaming);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}
