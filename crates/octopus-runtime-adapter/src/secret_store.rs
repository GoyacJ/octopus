#[cfg(test)]
use std::{collections::HashMap, sync::Mutex};

use octopus_core::AppError;

pub(super) trait RuntimeSecretStore: Send + Sync {
    fn put_secret(&self, reference: &str, value: &str) -> Result<(), AppError>;
    fn get_secret(&self, reference: &str) -> Result<Option<String>, AppError>;
    fn delete_secret(&self, reference: &str) -> Result<(), AppError>;
}

#[cfg(test)]
#[derive(Debug, Default)]
pub(super) struct MemoryRuntimeSecretStore {
    secrets: Mutex<HashMap<String, String>>,
}

#[cfg(test)]
impl RuntimeSecretStore for MemoryRuntimeSecretStore {
    fn put_secret(&self, reference: &str, value: &str) -> Result<(), AppError> {
        self.secrets
            .lock()
            .map_err(|_| AppError::runtime("runtime secret store mutex poisoned"))?
            .insert(reference.to_string(), value.to_string());
        Ok(())
    }

    fn get_secret(&self, reference: &str) -> Result<Option<String>, AppError> {
        Ok(self
            .secrets
            .lock()
            .map_err(|_| AppError::runtime("runtime secret store mutex poisoned"))?
            .get(reference)
            .cloned())
    }

    fn delete_secret(&self, reference: &str) -> Result<(), AppError> {
        self.secrets
            .lock()
            .map_err(|_| AppError::runtime("runtime secret store mutex poisoned"))?
            .remove(reference);
        Ok(())
    }
}

#[cfg_attr(test, allow(dead_code))]
#[derive(Debug)]
pub(super) struct KeyringRuntimeSecretStore {
    service_name: String,
}

impl KeyringRuntimeSecretStore {
    #[cfg_attr(test, allow(dead_code))]
    pub(super) fn new(workspace_id: &str) -> Self {
        Self {
            service_name: format!("octopus.runtime.{workspace_id}"),
        }
    }

    fn entry(&self, reference: &str) -> Result<keyring::Entry, AppError> {
        keyring::Entry::new(&self.service_name, reference)
            .map_err(|error| AppError::runtime(format!("failed to open keyring entry: {error}")))
    }
}

impl RuntimeSecretStore for KeyringRuntimeSecretStore {
    fn put_secret(&self, reference: &str, value: &str) -> Result<(), AppError> {
        self.entry(reference)?
            .set_password(value)
            .map_err(|error| AppError::runtime(format!("failed to store runtime secret: {error}")))
    }

    fn get_secret(&self, reference: &str) -> Result<Option<String>, AppError> {
        match self.entry(reference)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::runtime(format!(
                "failed to load runtime secret: {error}"
            ))),
        }
    }

    fn delete_secret(&self, reference: &str) -> Result<(), AppError> {
        match self.entry(reference)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(AppError::runtime(format!(
                "failed to delete runtime secret: {error}"
            ))),
        }
    }
}
