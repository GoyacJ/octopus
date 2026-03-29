use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use keyring::{Entry, Error as KeyringError};
use serde::{Deserialize, Serialize};

const REMOTE_SESSION_SERVICE: &str = "com.octopus.desktop.remote-session";
const REMOTE_SESSION_ACCOUNT: &str = "active-profile";
const DEFAULT_STORAGE_WARNING: &str =
    "Secure session storage is unavailable. Remote sign-in will stay memory-only.";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedHubSession {
    pub session_id: String,
    pub user_id: String,
    pub email: String,
    pub workspace_id: String,
    pub actor_ref: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedRemoteSession {
    pub base_url: String,
    pub workspace_id: String,
    pub email: String,
    pub access_token: String,
    pub session: PersistedHubSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionCacheLoadRequest {
    pub base_url: String,
    pub workspace_id: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionCacheOperationResult {
    pub storage_available: bool,
    pub warning: Option<String>,
}

impl RemoteSessionCacheOperationResult {
    fn available() -> Self {
        Self {
            storage_available: true,
            warning: None,
        }
    }

    fn unavailable(warning: String) -> Self {
        Self {
            storage_available: false,
            warning: Some(warning),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSessionCacheLoadResult {
    pub session: Option<PersistedRemoteSession>,
    pub storage_available: bool,
    pub warning: Option<String>,
}

impl RemoteSessionCacheLoadResult {
    fn available(session: Option<PersistedRemoteSession>) -> Self {
        Self {
            session,
            storage_available: true,
            warning: None,
        }
    }

    fn unavailable(warning: String) -> Self {
        Self {
            session: None,
            storage_available: false,
            warning: Some(warning),
        }
    }
}

pub trait RemoteSessionSecretStore: Send + Sync {
    fn load(&self) -> Result<Option<String>, String>;
    fn save(&self, secret: &str) -> Result<(), String>;
    fn clear(&self) -> Result<(), String>;
}

#[derive(Debug, Clone, Default)]
pub struct MemoryRemoteSessionSecretStore {
    secret: Arc<Mutex<Option<String>>>,
}

impl MemoryRemoteSessionSecretStore {
    pub fn stored_secret(&self) -> Option<String> {
        self.secret.lock().expect("memory secret store poisoned").clone()
    }

    pub fn set_raw_secret(&self, secret: &str) {
        *self.secret.lock().expect("memory secret store poisoned") = Some(secret.to_string());
    }
}

impl RemoteSessionSecretStore for MemoryRemoteSessionSecretStore {
    fn load(&self) -> Result<Option<String>, String> {
        Ok(self.stored_secret())
    }

    fn save(&self, secret: &str) -> Result<(), String> {
        *self.secret.lock().expect("memory secret store poisoned") = Some(secret.to_string());
        Ok(())
    }

    fn clear(&self) -> Result<(), String> {
        *self.secret.lock().expect("memory secret store poisoned") = None;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct KeyringRemoteSessionSecretStore {
    service: String,
    account: String,
}

impl Default for KeyringRemoteSessionSecretStore {
    fn default() -> Self {
        Self::new(REMOTE_SESSION_SERVICE, REMOTE_SESSION_ACCOUNT)
    }
}

impl KeyringRemoteSessionSecretStore {
    pub fn new(service: impl Into<String>, account: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            account: account.into(),
        }
    }

    fn entry(&self) -> Result<Entry, String> {
        Entry::new(&self.service, &self.account).map_err(|error| error.to_string())
    }
}

impl RemoteSessionSecretStore for KeyringRemoteSessionSecretStore {
    fn load(&self) -> Result<Option<String>, String> {
        match self.entry()?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(KeyringError::NoEntry) => Ok(None),
            Err(error) => Err(error.to_string()),
        }
    }

    fn save(&self, secret: &str) -> Result<(), String> {
        self.entry()?
            .set_password(secret)
            .map_err(|error| error.to_string())
    }

    fn clear(&self) -> Result<(), String> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RemoteSessionCacheService<S> {
    store: S,
}

impl<S> RemoteSessionCacheService<S>
where
    S: RemoteSessionSecretStore,
{
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn load(
        &self,
        request: &RemoteSessionCacheLoadRequest,
    ) -> Result<RemoteSessionCacheLoadResult, serde_json::Error> {
        let raw_secret = match self.store.load() {
            Ok(secret) => secret,
            Err(error) => return Ok(RemoteSessionCacheLoadResult::unavailable(storage_warning(error))),
        };

        let Some(raw_secret) = raw_secret else {
            return Ok(RemoteSessionCacheLoadResult::available(None));
        };

        let parsed: PersistedRemoteSession = match serde_json::from_str(&raw_secret) {
            Ok(session) => session,
            Err(_) => return Ok(self.clear_invalid_entry()),
        };

        if session_expired(&parsed.session.expires_at) || !binding_matches(request, &parsed) {
            return Ok(self.clear_invalid_entry());
        }

        Ok(RemoteSessionCacheLoadResult::available(Some(parsed)))
    }

    pub fn save(
        &self,
        session: &PersistedRemoteSession,
    ) -> Result<RemoteSessionCacheOperationResult, serde_json::Error> {
        let serialized = serde_json::to_string(session)?;
        Ok(match self.store.save(&serialized) {
            Ok(()) => RemoteSessionCacheOperationResult::available(),
            Err(error) => RemoteSessionCacheOperationResult::unavailable(storage_warning(error)),
        })
    }

    pub fn clear(&self) -> Result<RemoteSessionCacheOperationResult, serde_json::Error> {
        Ok(match self.store.clear() {
            Ok(()) => RemoteSessionCacheOperationResult::available(),
            Err(error) => RemoteSessionCacheOperationResult::unavailable(storage_warning(error)),
        })
    }

    fn clear_invalid_entry(&self) -> RemoteSessionCacheLoadResult {
        match self.store.clear() {
            Ok(()) => RemoteSessionCacheLoadResult::available(None),
            Err(error) => RemoteSessionCacheLoadResult::unavailable(storage_warning(error)),
        }
    }
}

fn storage_warning(error: String) -> String {
    let trimmed = error.trim();
    if trimmed.is_empty() {
        DEFAULT_STORAGE_WARNING.to_string()
    } else {
        trimmed.to_string()
    }
}

fn binding_matches(
    request: &RemoteSessionCacheLoadRequest,
    session: &PersistedRemoteSession,
) -> bool {
    normalize_text(&request.base_url) == normalize_text(&session.base_url)
        && normalize_text(&request.workspace_id) == normalize_text(&session.workspace_id)
        && normalize_text(&request.email) == normalize_text(&session.email)
}

fn normalize_text(value: &str) -> String {
    value.trim().to_string()
}

fn session_expired(expires_at: &str) -> bool {
    DateTime::parse_from_rfc3339(expires_at)
        .map(|parsed| parsed.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true)
}
