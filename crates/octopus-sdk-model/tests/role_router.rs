use octopus_sdk_model::{ModelCatalog, ModelId, ModelRole, RoleRouter};

#[test]
fn builtin_role_router_resolves_all_primary_roles() {
    let catalog = ModelCatalog::new_builtin();
    let router = RoleRouter::new_builtin(&catalog);

    for role in [
        ModelRole::Main,
        ModelRole::Fast,
        ModelRole::Best,
        ModelRole::Plan,
        ModelRole::Compact,
        ModelRole::Vision,
        ModelRole::WebExtract,
        ModelRole::Eval,
        ModelRole::SubagentDefault,
    ] {
        assert!(
            router.resolve(role.clone()).is_some(),
            "missing default for {role:?}"
        );
    }
}

#[test]
fn overrides_take_precedence_over_defaults() {
    let catalog = ModelCatalog::new_builtin();
    let router = RoleRouter::new_builtin(&catalog)
        .with_override(ModelRole::Fast, ModelId("glm-5-turbo".to_string()));

    assert_eq!(
        router.resolve(ModelRole::Fast),
        Some(ModelId("glm-5-turbo".to_string()))
    );
}
