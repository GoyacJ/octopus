use std::{fs, sync::Arc};

use octopus_core::{
    CapabilityDescriptor, ResolvedExecutionTarget, ResolvedRequestPolicyInput, RuntimeConfigPatch,
    RuntimeConfiguredModelCredentialInput, DEFAULT_WORKSPACE_ID,
};
use octopus_infra::build_infra_bundle;
use octopus_platform::RuntimeConfigService;
use octopus_runtime_adapter::{
    resolve_model_auth_source, MockRuntimeModelDriver, ResolvedModelAuthMode, RuntimeAdapter,
};
use serde_json::json;

const CREDENTIAL_SOURCE_CONFIGURED_MODEL_OVERRIDE: &str = "configured_model_override";
const CREDENTIAL_SOURCE_PROVIDER_INHERITED: &str = "provider_inherited";
const IN_MEMORY_SECRET_STORE_ENV: &str = "OCTOPUS_TEST_USE_IN_MEMORY_SECRET_STORE";

fn test_root() -> std::path::PathBuf {
    std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-env");
    std::env::set_var(IN_MEMORY_SECRET_STORE_ENV, "1");
    let root = std::env::temp_dir().join(format!(
        "octopus-runtime-adapter-model-auth-resolution-{}",
        uuid::Uuid::new_v4()
    ));
    fs::create_dir_all(&root).expect("test root");
    root
}

fn target(reference: Option<&str>, credential_source: &str) -> ResolvedExecutionTarget {
    ResolvedExecutionTarget {
        configured_model_id: "configured-model".into(),
        configured_model_name: "Configured Model".into(),
        provider_id: "anthropic".into(),
        registry_model_id: "claude-sonnet-4-5".into(),
        model_id: "claude-sonnet-4-5".into(),
        surface: "conversation".into(),
        protocol_family: "anthropic_messages".into(),
        credential_ref: reference.map(ToOwned::to_owned),
        credential_source: credential_source.into(),
        request_policy: ResolvedRequestPolicyInput {
            auth_strategy: "x_api_key".into(),
            base_url_policy: "allow_override".into(),
            default_base_url: "https://api.anthropic.com".into(),
            provider_base_url: None,
            configured_base_url: None,
        },
        base_url: Some("https://api.anthropic.com".into()),
        max_output_tokens: Some(4096),
        capabilities: vec![CapabilityDescriptor {
            capability_id: "reasoning".into(),
            label: "reasoning".into(),
        }],
    }
}

async fn save_workspace_managed_credential(adapter: &RuntimeAdapter) -> String {
    let saved = adapter
        .save_config(
            "workspace",
            RuntimeConfigPatch {
                scope: "workspace".into(),
                patch: json!({
                    "configuredModels": {
                        "configured-model": {
                            "configuredModelId": "configured-model",
                            "name": "Configured Model",
                            "providerId": "anthropic",
                            "modelId": "claude-sonnet-4-5",
                            "enabled": true,
                            "source": "workspace"
                        }
                    }
                }),
                configured_model_credentials: vec![RuntimeConfiguredModelCredentialInput {
                    configured_model_id: "configured-model".into(),
                    api_key: "sk-ant-secret".into(),
                }],
            },
        )
        .await
        .expect("save runtime config with managed credential");

    saved
        .sources
        .iter()
        .find(|source| source.scope == "workspace")
        .and_then(|source| source.document.as_ref())
        .and_then(|document| {
            document
                .pointer("/configuredModels/configured-model/credentialRef")
                .and_then(serde_json::Value::as_str)
        })
        .map(str::to_owned)
        .expect("workspace managed credential reference")
}

#[tokio::test]
async fn resolves_secret_ref_and_env_ref_into_runtime_auth() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let stored_reference = save_workspace_managed_credential(&adapter).await;
    let managed_auth = adapter
        .resolve_model_auth(&target(
            Some(&stored_reference),
            CREDENTIAL_SOURCE_CONFIGURED_MODEL_OVERRIDE,
        ))
        .expect("resolve managed secret auth");
    assert_eq!(managed_auth.mode, ResolvedModelAuthMode::BearerToken);
    assert_eq!(managed_auth.credential, "sk-ant-secret");
    assert_eq!(
        managed_auth.source,
        CREDENTIAL_SOURCE_CONFIGURED_MODEL_OVERRIDE
    );
    assert_eq!(managed_auth.reference_kind, "managed_secret");

    let inherited_auth = adapter
        .resolve_model_auth(&target(
            Some("env:ANTHROPIC_API_KEY"),
            CREDENTIAL_SOURCE_PROVIDER_INHERITED,
        ))
        .expect("resolve inherited env auth");
    assert_eq!(inherited_auth.mode, ResolvedModelAuthMode::BearerToken);
    assert_eq!(inherited_auth.credential, "sk-ant-env");
    assert_eq!(inherited_auth.source, CREDENTIAL_SOURCE_PROVIDER_INHERITED);
    assert_eq!(inherited_auth.reference_kind, "env");

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[tokio::test]
async fn rejects_unsupported_reference_schemes_fail_closed() {
    let root = test_root();
    let infra = build_infra_bundle(&root).expect("infra bundle");
    let adapter = RuntimeAdapter::new_with_executor(
        DEFAULT_WORKSPACE_ID,
        infra.paths.clone(),
        infra.observation.clone(),
        infra.authorization.clone(),
        Arc::new(MockRuntimeModelDriver),
    );

    let error = adapter
        .resolve_model_auth(&target(
            Some("op://vault/item"),
            CREDENTIAL_SOURCE_PROVIDER_INHERITED,
        ))
        .expect_err("unsupported references must fail closed");

    assert!(error
        .to_string()
        .contains("unsupported credential reference"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn reports_provider_inherited_auth_source_explicitly() {
    let auth_source = resolve_model_auth_source(&target(
        Some("env:ANTHROPIC_API_KEY"),
        CREDENTIAL_SOURCE_PROVIDER_INHERITED,
    ))
    .expect("resolve auth source");

    assert_eq!(auth_source.source, CREDENTIAL_SOURCE_PROVIDER_INHERITED);
    assert_eq!(auth_source.reference_kind, "env");
}
