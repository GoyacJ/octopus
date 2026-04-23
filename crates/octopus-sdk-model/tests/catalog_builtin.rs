use octopus_sdk_model::{ModelCatalog, ModelTrack, ProtocolFamily};

#[test]
fn builtin_catalog_covers_expected_providers() {
    let catalog = ModelCatalog::new_builtin();
    let provider_ids = catalog
        .list_providers()
        .iter()
        .map(|provider| provider.id.0.as_str())
        .collect::<Vec<_>>();

    assert_eq!(provider_ids.len(), 6);
    for expected in [
        "anthropic",
        "deepseek",
        "minimax",
        "moonshot",
        "bigmodel",
        "qwen",
    ] {
        assert!(
            provider_ids.contains(&expected),
            "missing provider {expected}"
        );
    }
}

#[test]
fn resolve_prefers_alias_then_canonical_model_id() {
    let catalog = ModelCatalog::new_builtin();
    let by_alias = catalog.resolve("opus").expect("alias should resolve");
    let by_canonical = catalog
        .resolve("claude-opus-4-6")
        .expect("canonical model id should resolve");

    assert_eq!(by_alias.model.id, by_canonical.model.id);
    assert_eq!(by_alias.provider.id.0, "anthropic");
    assert_eq!(by_alias.surface.protocol, ProtocolFamily::AnthropicMessages);
    assert_eq!(by_alias.model.track, ModelTrack::Stable);
}

#[test]
fn canonicalize_handles_bedrock_style_ids_and_unknown_models() {
    let catalog = ModelCatalog::new_builtin();

    assert_eq!(
        catalog
            .canonicalize("anthropic.claude-opus-4-6-v1:0")
            .unwrap()
            .0,
        "claude-opus-4-6"
    );
    assert!(catalog.resolve("gpt-5").is_none());
    assert!(catalog.resolve("gemini-flash").is_none());
    assert_eq!(
        catalog.resolve("minimax-m2").unwrap().model.id.0,
        "MiniMax-M2"
    );
    assert!(catalog.resolve("unknown/xxx").is_none());
}
