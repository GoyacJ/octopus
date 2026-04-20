use std::fmt;

use async_trait::async_trait;
use thiserror::Error;
use zeroize::Zeroizing;

#[async_trait]
pub trait SecretVault: Send + Sync {
    async fn get(&self, ref_id: &str) -> Result<SecretValue, VaultError>;
    async fn put(&self, ref_id: &str, value: SecretValue) -> Result<(), VaultError>;
}

pub struct SecretValue(Zeroizing<Vec<u8>>);

impl SecretValue {
    #[must_use]
    pub fn new(value: impl AsRef<[u8]>) -> Self {
        Self(Zeroizing::new(value.as_ref().to_vec()))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue(REDACTED)")
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VaultError {
    #[error("secret not found")]
    NotFound,
    #[error("secret backend error: {0}")]
    Backend(String),
    #[error("secret value redacted")]
    Redacted,
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::{SecretValue, VaultError};

    fn assert_debug<T: Debug>(_: &T) {}

    #[test]
    fn secret_value_debug_is_redacted() {
        let secret = SecretValue::new(b"sk-test-123");

        assert_debug(&secret);
        assert_eq!(format!("{secret:?}"), "SecretValue(REDACTED)");
    }

    #[test]
    fn vault_error_backend_preserves_message() {
        let error = VaultError::Backend("kms unavailable".into());

        assert_eq!(error.to_string(), "secret backend error: kms unavailable");
    }
}
