use crate::RuntimeAdapter;
use octopus_core::{AppError, ResolvedExecutionTarget};

pub(crate) const CREDENTIAL_SOURCE_CONFIGURED_MODEL_OVERRIDE: &str = "configured_model_override";
pub(crate) const CREDENTIAL_SOURCE_PROBE_OVERRIDE: &str = "probe_override";
pub(crate) const CREDENTIAL_SOURCE_PROVIDER_INHERITED: &str = "provider_inherited";
pub(crate) const CREDENTIAL_SOURCE_UNCONFIGURED: &str = "unconfigured";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedModelAuthMode {
    BearerToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModelAuthSource {
    pub source: String,
    pub reference_kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModelAuth {
    pub mode: ResolvedModelAuthMode,
    pub credential: String,
    pub source: String,
    pub reference_kind: String,
}

pub(crate) enum CredentialReference<'a> {
    Env(&'a str),
    ManagedSecret(&'a str),
    Inline(&'a str),
}

pub fn resolve_model_auth_source(
    target: &ResolvedExecutionTarget,
) -> Result<ResolvedModelAuthSource, AppError> {
    let source = target.credential_source.trim();
    if source.is_empty() {
        return Err(AppError::invalid_input(format!(
            "resolved execution target `{}` is missing credential source metadata",
            target.configured_model_id
        )));
    }

    let reference = parse_model_credential_reference(target.credential_ref.as_deref())?
        .ok_or_else(|| {
            AppError::invalid_input(format!(
                "resolved execution target `{}` is missing a credential reference",
                target.configured_model_id
            ))
        })?;

    Ok(ResolvedModelAuthSource {
        source: source.to_string(),
        reference_kind: reference.kind().to_string(),
    })
}

pub(crate) fn parse_model_credential_reference(
    reference: Option<&str>,
) -> Result<Option<CredentialReference<'_>>, AppError> {
    let Some(reference) = reference.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    if let Some(env_key) = reference.strip_prefix("env:") {
        return (!env_key.trim().is_empty())
            .then_some(CredentialReference::Env(env_key))
            .map(Some)
            .ok_or_else(|| AppError::invalid_input("credential env reference must not be empty"));
    }

    if reference.starts_with("secret-ref:") {
        return (reference != "secret-ref:")
            .then_some(CredentialReference::ManagedSecret(reference))
            .map(Some)
            .ok_or_else(|| {
                AppError::invalid_input("managed credential reference must not be empty")
            });
    }

    if looks_like_unsupported_reference(reference) {
        return Err(AppError::invalid_input(format!(
            "unsupported credential reference `{reference}`; only `env:` and `secret-ref:` are supported"
        )));
    }

    Ok(Some(CredentialReference::Inline(reference)))
}

pub(crate) fn validate_runtime_credential_reference(
    reference: &str,
    context: &str,
) -> Result<Option<String>, AppError> {
    match parse_model_credential_reference(Some(reference))? {
        Some(CredentialReference::Env(env_key)) => Ok(Some(format!("env:{env_key}"))),
        Some(CredentialReference::ManagedSecret(reference)) => Ok(Some(reference.to_string())),
        Some(CredentialReference::Inline(_)) => Err(AppError::invalid_input(format!(
            "{context} must use `env:` or `secret-ref:` credential references; inline credentials are not allowed"
        ))),
        None => Ok(None),
    }
}

impl RuntimeAdapter {
    pub fn resolve_model_auth(
        &self,
        target: &ResolvedExecutionTarget,
    ) -> Result<ResolvedModelAuth, AppError> {
        let auth_source = resolve_model_auth_source(target)?;
        let reference = parse_model_credential_reference(target.credential_ref.as_deref())?
            .ok_or_else(|| {
                AppError::invalid_input(format!(
                    "resolved execution target `{}` is missing a credential reference",
                    target.configured_model_id
                ))
            })?;

        let credential = match reference {
            CredentialReference::Env(env_key) => std::env::var(env_key).map_err(|_| {
                AppError::invalid_input(format!(
                    "missing configured credential env var `{env_key}` for provider `{}`",
                    target.provider_id
                ))
            })?,
            CredentialReference::ManagedSecret(reference) => self
                .state
                .secret_store
                .get_secret(reference)?
                .ok_or_else(|| {
                    AppError::invalid_input(format!(
                        "missing managed credential `{reference}` for provider `{}`",
                        target.provider_id
                    ))
                })?,
            CredentialReference::Inline(value) => value.to_string(),
        };

        Ok(ResolvedModelAuth {
            mode: ResolvedModelAuthMode::BearerToken,
            credential,
            source: auth_source.source,
            reference_kind: auth_source.reference_kind,
        })
    }
}

impl CredentialReference<'_> {
    fn kind(&self) -> &'static str {
        match self {
            Self::Env(_) => "env",
            Self::ManagedSecret(_) => "managed_secret",
            Self::Inline(_) => "inline",
        }
    }
}

fn looks_like_unsupported_reference(reference: &str) -> bool {
    reference.starts_with("keychain:")
        || reference.starts_with("vault:")
        || reference.starts_with("op://")
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use crate::{secret_store::MemoryRuntimeSecretStore, MockRuntimeModelDriver, RuntimeAdapter};
    use octopus_core::{CapabilityDescriptor, ResolvedExecutionTarget, DEFAULT_WORKSPACE_ID};
    use octopus_infra::build_infra_bundle;

    use super::{
        resolve_model_auth_source, ResolvedModelAuthMode,
        CREDENTIAL_SOURCE_CONFIGURED_MODEL_OVERRIDE, CREDENTIAL_SOURCE_PROVIDER_INHERITED,
    };

    fn test_root() -> std::path::PathBuf {
        std::env::set_var("ANTHROPIC_API_KEY", "sk-ant-env");
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
            request_policy: octopus_core::ResolvedRequestPolicyInput {
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

    #[tokio::test]
    async fn resolves_secret_ref_and_env_ref_into_runtime_auth() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let adapter = RuntimeAdapter::new_with_executor_and_secret_store(
            DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            infra.authorization.clone(),
            Arc::new(MockRuntimeModelDriver),
            Arc::new(MemoryRuntimeSecretStore::default()),
        );

        let stored_reference = adapter.configured_model_secret_reference("configured-model");
        adapter
            .state
            .secret_store
            .put_secret(&stored_reference, "sk-ant-secret")
            .expect("store managed secret");

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

        let inherited_auth = adapter
            .resolve_model_auth(&target(
                Some("env:ANTHROPIC_API_KEY"),
                CREDENTIAL_SOURCE_PROVIDER_INHERITED,
            ))
            .expect("resolve inherited env auth");
        assert_eq!(inherited_auth.mode, ResolvedModelAuthMode::BearerToken);
        assert_eq!(inherited_auth.credential, "sk-ant-env");
        assert_eq!(inherited_auth.source, CREDENTIAL_SOURCE_PROVIDER_INHERITED);

        fs::remove_dir_all(root).expect("cleanup temp dir");
    }

    #[tokio::test]
    async fn rejects_unsupported_reference_schemes_fail_closed() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let adapter = RuntimeAdapter::new_with_executor_and_secret_store(
            DEFAULT_WORKSPACE_ID,
            infra.paths.clone(),
            infra.observation.clone(),
            infra.authorization.clone(),
            Arc::new(MockRuntimeModelDriver),
            Arc::new(MemoryRuntimeSecretStore::default()),
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
}
