use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use getrandom::getrandom;
use octopus_core::AppError;
use octopus_core::timestamp_now;
use octopus_infra::WorkspacePaths;
use rusqlite::{params, Connection, OptionalExtension};

pub(super) trait RuntimeSecretStore: Send + Sync {
    fn put_secret(&self, reference: &str, value: &str) -> Result<(), AppError>;
    fn get_secret(&self, reference: &str) -> Result<Option<String>, AppError>;
    fn delete_secret(&self, reference: &str) -> Result<(), AppError>;
}

#[derive(Debug, Default)]
pub(super) struct MemoryRuntimeSecretStore {
    secrets: Mutex<HashMap<String, String>>,
}

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

#[derive(Debug)]
pub(super) struct UnavailableRuntimeSecretStore {
    message: String,
}

impl UnavailableRuntimeSecretStore {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl RuntimeSecretStore for UnavailableRuntimeSecretStore {
    fn put_secret(&self, _reference: &str, _value: &str) -> Result<(), AppError> {
        Err(AppError::runtime(self.message.clone()))
    }

    fn get_secret(&self, _reference: &str) -> Result<Option<String>, AppError> {
        Err(AppError::runtime(self.message.clone()))
    }

    fn delete_secret(&self, _reference: &str) -> Result<(), AppError> {
        Err(AppError::runtime(self.message.clone()))
    }
}

const RUNTIME_SECRET_KEY_VERSION: i64 = 1;
const RUNTIME_SECRET_MASTER_KEY_BYTES: usize = 32;
const RUNTIME_SECRET_NONCE_BYTES: usize = 24;

#[derive(Debug)]
pub(super) struct SqliteEncryptedRuntimeSecretStore {
    workspace_id: String,
    db_path: PathBuf,
    master_key: [u8; RUNTIME_SECRET_MASTER_KEY_BYTES],
}

impl SqliteEncryptedRuntimeSecretStore {
    pub(super) fn new(workspace_id: &str, paths: &WorkspacePaths) -> Result<Self, AppError> {
        Ok(Self {
            workspace_id: workspace_id.to_string(),
            db_path: paths.db_path.clone(),
            master_key: Self::load_or_create_master_key(&paths.runtime_secret_master_key_path)?,
        })
    }

    fn load_or_create_master_key(
        path: &Path,
    ) -> Result<[u8; RUNTIME_SECRET_MASTER_KEY_BYTES], AppError> {
        if path.exists() {
            return Self::read_master_key(path);
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let key = Self::random_bytes::<RUNTIME_SECRET_MASTER_KEY_BYTES>()?;
        match OpenOptions::new().create_new(true).write(true).open(path) {
            Ok(mut file) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;

                    file.set_permissions(fs::Permissions::from_mode(0o600))?;
                }
                file.write_all(&key)?;
                file.sync_all()?;
                Ok(key)
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                Self::read_master_key(path)
            }
            Err(error) => Err(error.into()),
        }
    }

    fn read_master_key(path: &Path) -> Result<[u8; RUNTIME_SECRET_MASTER_KEY_BYTES], AppError> {
        let bytes = fs::read(path)?;
        if bytes.len() != RUNTIME_SECRET_MASTER_KEY_BYTES {
            return Err(AppError::runtime(format!(
                "runtime secret master key at `{}` must be {} bytes",
                path.display(),
                RUNTIME_SECRET_MASTER_KEY_BYTES
            )));
        }

        let mut key = [0_u8; RUNTIME_SECRET_MASTER_KEY_BYTES];
        key.copy_from_slice(&bytes);
        Ok(key)
    }

    fn random_bytes<const N: usize>() -> Result<[u8; N], AppError> {
        let mut bytes = [0_u8; N];
        getrandom(&mut bytes)
            .map_err(|error| AppError::runtime(format!("failed to generate runtime secret bytes: {error}")))?;
        Ok(bytes)
    }

    fn open_db(&self) -> Result<Connection, AppError> {
        Connection::open(&self.db_path).map_err(|error| AppError::database(error.to_string()))
    }

    fn encrypt(&self, value: &str) -> Result<(Vec<u8>, [u8; RUNTIME_SECRET_NONCE_BYTES]), AppError> {
        let nonce = Self::random_bytes::<RUNTIME_SECRET_NONCE_BYTES>()?;
        let cipher = XChaCha20Poly1305::new_from_slice(&self.master_key)
            .map_err(|_| AppError::runtime("runtime secret master key is invalid"))?;
        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce), value.as_bytes())
            .map_err(|error| AppError::runtime(format!("failed to encrypt runtime secret: {error}")))?;
        Ok((ciphertext, nonce))
    }

    fn decrypt(
        &self,
        ciphertext: &[u8],
        nonce: &[u8],
        key_version: i64,
    ) -> Result<String, AppError> {
        if key_version != RUNTIME_SECRET_KEY_VERSION {
            return Err(AppError::runtime(format!(
                "unsupported runtime secret key version `{key_version}`"
            )));
        }
        if nonce.len() != RUNTIME_SECRET_NONCE_BYTES {
            return Err(AppError::runtime("runtime secret nonce is invalid"));
        }

        let cipher = XChaCha20Poly1305::new_from_slice(&self.master_key)
            .map_err(|_| AppError::runtime("runtime secret master key is invalid"))?;
        let plaintext = cipher
            .decrypt(XNonce::from_slice(nonce), ciphertext)
            .map_err(|error| AppError::runtime(format!("failed to decrypt runtime secret: {error}")))?;

        String::from_utf8(plaintext)
            .map_err(|error| AppError::runtime(format!("runtime secret payload is not valid UTF-8: {error}")))
    }
}

impl RuntimeSecretStore for SqliteEncryptedRuntimeSecretStore {
    fn put_secret(&self, reference: &str, value: &str) -> Result<(), AppError> {
        let (ciphertext, nonce) = self.encrypt(value)?;
        let now = i64::try_from(timestamp_now())
            .map_err(|_| AppError::runtime("runtime secret timestamp overflow"))?;

        self.open_db()?
            .execute(
                "INSERT INTO runtime_secret_records
                 (reference, workspace_id, ciphertext, nonce, key_version, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(reference) DO UPDATE SET
                   workspace_id = excluded.workspace_id,
                   ciphertext = excluded.ciphertext,
                   nonce = excluded.nonce,
                   key_version = excluded.key_version,
                   updated_at = excluded.updated_at",
                params![
                    reference,
                    &self.workspace_id,
                    ciphertext,
                    nonce.to_vec(),
                    RUNTIME_SECRET_KEY_VERSION,
                    now,
                    now,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        Ok(())
    }

    fn get_secret(&self, reference: &str) -> Result<Option<String>, AppError> {
        let record = self
            .open_db()?
            .query_row(
                "SELECT ciphertext, nonce, key_version
                 FROM runtime_secret_records
                 WHERE reference = ?1 AND workspace_id = ?2",
                params![reference, &self.workspace_id],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|error| AppError::database(error.to_string()))?;

        record
            .map(|(ciphertext, nonce, key_version)| self.decrypt(&ciphertext, &nonce, key_version))
            .transpose()
    }

    fn delete_secret(&self, reference: &str) -> Result<(), AppError> {
        self.open_db()?
            .execute(
                "DELETE FROM runtime_secret_records WHERE reference = ?1 AND workspace_id = ?2",
                params![reference, &self.workspace_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use octopus_infra::build_infra_bundle;
    use rusqlite::{params, Connection};
    use uuid::Uuid;

    use super::*;

    fn test_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("octopus-runtime-secret-store-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).expect("test root");
        root
    }

    #[test]
    fn sqlite_secret_store_round_trips_encrypted_values() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let store = SqliteEncryptedRuntimeSecretStore::new(octopus_core::DEFAULT_WORKSPACE_ID, &infra.paths)
            .expect("sqlite secret store");
        let reference = "secret-ref:workspace:model:anthropic-inline";
        let secret = "sk-ant-sqlite-secret";

        store.put_secret(reference, secret).expect("store secret");

        assert_eq!(
            store.get_secret(reference).expect("load secret"),
            Some(secret.to_string())
        );

        let connection = Connection::open(&infra.paths.db_path).expect("db");
        let (workspace_id, ciphertext, nonce): (String, Vec<u8>, Vec<u8>) = connection
            .query_row(
                "SELECT workspace_id, ciphertext, nonce
                 FROM runtime_secret_records
                 WHERE reference = ?1",
                params![reference],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("runtime secret record");

        assert_eq!(workspace_id, octopus_core::DEFAULT_WORKSPACE_ID);
        assert!(!ciphertext.is_empty());
        assert_ne!(ciphertext, secret.as_bytes());
        assert_eq!(nonce.len(), 24);

        let master_key = fs::read(&infra.paths.runtime_secret_master_key_path)
            .expect("master key file");
        assert_eq!(master_key.len(), 32);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_secret_store_returns_none_for_missing_reference() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        let store = SqliteEncryptedRuntimeSecretStore::new(octopus_core::DEFAULT_WORKSPACE_ID, &infra.paths)
            .expect("sqlite secret store");

        assert_eq!(
            store
                .get_secret("secret-ref:workspace:model:missing")
                .expect("missing secret"),
            None
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn sqlite_secret_store_rejects_invalid_master_key_files() {
        let root = test_root();
        let infra = build_infra_bundle(&root).expect("infra bundle");
        fs::write(&infra.paths.runtime_secret_master_key_path, b"short").expect("invalid master key");

        let error = SqliteEncryptedRuntimeSecretStore::new(octopus_core::DEFAULT_WORKSPACE_ID, &infra.paths)
            .expect_err("invalid key should fail");

        assert!(error.to_string().contains("must be 32 bytes"));

        let _ = fs::remove_dir_all(root);
    }
}
