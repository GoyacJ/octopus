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
    use super::{
        parse_model_credential_reference, validate_runtime_credential_reference,
        CredentialReference,
    };

    #[test]
    fn classifies_supported_credential_reference_kinds() {
        assert!(matches!(
            parse_model_credential_reference(Some("env:ANTHROPIC_API_KEY")),
            Ok(Some(CredentialReference::Env("ANTHROPIC_API_KEY")))
        ));
        assert!(matches!(
            parse_model_credential_reference(Some("secret-ref:workspace:test")),
            Ok(Some(CredentialReference::ManagedSecret(
                "secret-ref:workspace:test"
            )))
        ));
        assert!(matches!(
            parse_model_credential_reference(Some("sk-inline")),
            Ok(Some(CredentialReference::Inline("sk-inline")))
        ));
    }

    #[test]
    fn rejects_inline_values_when_runtime_config_requires_references() {
        assert_eq!(
            validate_runtime_credential_reference("env:ANTHROPIC_API_KEY", "credentialRef")
                .expect("env reference should be preserved")
                .as_deref(),
            Some("env:ANTHROPIC_API_KEY")
        );

        let error = validate_runtime_credential_reference("sk-inline", "credentialRef")
            .expect_err("inline values must be rejected");
        assert!(error
            .to_string()
            .contains("inline credentials are not allowed"));
    }
}
