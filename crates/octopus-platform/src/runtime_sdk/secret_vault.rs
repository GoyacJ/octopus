use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::Mutex,
};

use async_trait::async_trait;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    XChaCha20Poly1305, XNonce,
};
use getrandom::getrandom;
use octopus_core::{timestamp_now, AppError};
use octopus_persistence::{Database, Migration};
use octopus_sdk::{SecretValue, SecretVault, VaultError};
use rusqlite::{params, Connection, OptionalExtension};

use super::RuntimeSdkPaths;

const IN_MEMORY_SECRET_STORE_ENV: &str = "OCTOPUS_TEST_USE_IN_MEMORY_SECRET_STORE";
const RUNTIME_SECRET_KEY_VERSION: i64 = 1;
const RUNTIME_SECRET_MASTER_KEY_BYTES: usize = 32;
const RUNTIME_SECRET_NONCE_BYTES: usize = 24;

fn create_runtime_secret_records_table(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS runtime_secret_records (
                reference TEXT PRIMARY KEY,
                workspace_id TEXT NOT NULL,
                ciphertext BLOB NOT NULL,
                nonce BLOB NOT NULL,
                key_version INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );",
        )
        .map_err(|error| AppError::database(error.to_string()))
}

static RUNTIME_SECRET_MIGRATIONS: &[Migration] = &[Migration {
    key: "0001-runtime-secret-records",
    apply: create_runtime_secret_records_table,
}];

enum RuntimeSecretVaultBackend {
    Memory {
        secrets: Mutex<HashMap<String, Vec<u8>>>,
    },
    Sqlite {
        database: Database,
        master_key: [u8; RUNTIME_SECRET_MASTER_KEY_BYTES],
    },
    Unavailable {
        message: String,
    },
}

pub(crate) struct RuntimeSecretVault {
    workspace_id: String,
    backend: RuntimeSecretVaultBackend,
}

impl RuntimeSecretVault {
    pub(crate) fn open(
        workspace_id: &str,
        paths: &RuntimeSdkPaths,
        database: Database,
    ) -> Result<std::sync::Arc<Self>, AppError> {
        if std::env::var_os(IN_MEMORY_SECRET_STORE_ENV).is_some() {
            return Ok(std::sync::Arc::new(Self::in_memory(workspace_id)));
        }

        match Self::sqlite_backend(paths, database) {
            Ok(backend) => Ok(std::sync::Arc::new(Self {
                workspace_id: workspace_id.to_string(),
                backend,
            })),
            Err(error) => Ok(std::sync::Arc::new(Self {
                workspace_id: workspace_id.to_string(),
                backend: RuntimeSecretVaultBackend::Unavailable {
                    message: error.to_string(),
                },
            })),
        }
    }

    fn in_memory(workspace_id: &str) -> Self {
        Self {
            workspace_id: workspace_id.to_string(),
            backend: RuntimeSecretVaultBackend::Memory {
                secrets: Mutex::new(HashMap::new()),
            },
        }
    }

    fn sqlite_backend(
        paths: &RuntimeSdkPaths,
        database: Database,
    ) -> Result<RuntimeSecretVaultBackend, AppError> {
        let master_key = Self::load_or_create_master_key(&paths.runtime_secret_master_key_path)?;
        let database = database.with_migrations(RUNTIME_SECRET_MIGRATIONS);
        database.run_migrations()?;

        Ok(RuntimeSecretVaultBackend::Sqlite {
            database,
            master_key,
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
        getrandom(&mut bytes).map_err(|error| {
            AppError::runtime(format!("failed to generate runtime secret bytes: {error}"))
        })?;
        Ok(bytes)
    }

    fn to_vault_error(error: AppError) -> VaultError {
        VaultError::Backend(error.to_string())
    }

    fn app_error(&self) -> AppError {
        match &self.backend {
            RuntimeSecretVaultBackend::Unavailable { message } => {
                AppError::runtime(message.clone())
            }
            _ => AppError::runtime("runtime secret vault backend error"),
        }
    }

    fn put_bytes(&self, reference: &str, value: &[u8]) -> Result<(), AppError> {
        match &self.backend {
            RuntimeSecretVaultBackend::Memory { secrets } => {
                secrets
                    .lock()
                    .map_err(|_| AppError::runtime("runtime secret store mutex poisoned"))?
                    .insert(reference.to_string(), value.to_vec());
                Ok(())
            }
            RuntimeSecretVaultBackend::Sqlite {
                database,
                master_key,
            } => {
                let (ciphertext, nonce) = Self::encrypt(master_key, value)?;
                let now = i64::try_from(timestamp_now())
                    .map_err(|_| AppError::runtime("runtime secret timestamp overflow"))?;
                database
                    .acquire()?
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
            RuntimeSecretVaultBackend::Unavailable { .. } => Err(self.app_error()),
        }
    }

    pub(crate) fn get_optional_bytes(&self, reference: &str) -> Result<Option<Vec<u8>>, AppError> {
        match &self.backend {
            RuntimeSecretVaultBackend::Memory { secrets } => Ok(secrets
                .lock()
                .map_err(|_| AppError::runtime("runtime secret store mutex poisoned"))?
                .get(reference)
                .cloned()),
            RuntimeSecretVaultBackend::Sqlite {
                database,
                master_key,
            } => {
                let record = database
                    .acquire()?
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
                    .map(|(ciphertext, nonce, key_version)| {
                        Self::decrypt(master_key, &ciphertext, &nonce, key_version)
                    })
                    .transpose()
            }
            RuntimeSecretVaultBackend::Unavailable { .. } => Err(self.app_error()),
        }
    }

    pub(crate) fn get_optional_utf8(&self, reference: &str) -> Result<Option<String>, AppError> {
        self.get_optional_bytes(reference)?
            .map(|value| {
                String::from_utf8(value).map_err(|error| {
                    AppError::runtime(format!(
                        "runtime secret payload is not valid UTF-8: {error}"
                    ))
                })
            })
            .transpose()
    }

    pub(crate) fn put_utf8(&self, reference: &str, value: &str) -> Result<(), AppError> {
        self.put_bytes(reference, value.as_bytes())
    }

    pub(crate) fn delete(&self, reference: &str) -> Result<(), AppError> {
        match &self.backend {
            RuntimeSecretVaultBackend::Memory { secrets } => {
                secrets
                    .lock()
                    .map_err(|_| AppError::runtime("runtime secret store mutex poisoned"))?
                    .remove(reference);
                Ok(())
            }
            RuntimeSecretVaultBackend::Sqlite { database, .. } => {
                database
                    .acquire()?
                    .execute(
                        "DELETE FROM runtime_secret_records WHERE reference = ?1 AND workspace_id = ?2",
                        params![reference, &self.workspace_id],
                    )
                    .map_err(|error| AppError::database(error.to_string()))?;
                Ok(())
            }
            RuntimeSecretVaultBackend::Unavailable { .. } => Err(self.app_error()),
        }
    }

    fn encrypt(
        master_key: &[u8; RUNTIME_SECRET_MASTER_KEY_BYTES],
        value: &[u8],
    ) -> Result<(Vec<u8>, [u8; RUNTIME_SECRET_NONCE_BYTES]), AppError> {
        let nonce = Self::random_bytes::<RUNTIME_SECRET_NONCE_BYTES>()?;
        let cipher = XChaCha20Poly1305::new_from_slice(master_key)
            .map_err(|_| AppError::runtime("runtime secret master key is invalid"))?;
        let ciphertext = cipher
            .encrypt(XNonce::from_slice(&nonce), value)
            .map_err(|error| {
                AppError::runtime(format!("failed to encrypt runtime secret: {error}"))
            })?;
        Ok((ciphertext, nonce))
    }

    fn decrypt(
        master_key: &[u8; RUNTIME_SECRET_MASTER_KEY_BYTES],
        ciphertext: &[u8],
        nonce: &[u8],
        key_version: i64,
    ) -> Result<Vec<u8>, AppError> {
        if key_version != RUNTIME_SECRET_KEY_VERSION {
            return Err(AppError::runtime(format!(
                "unsupported runtime secret key version `{key_version}`"
            )));
        }
        if nonce.len() != RUNTIME_SECRET_NONCE_BYTES {
            return Err(AppError::runtime("runtime secret nonce is invalid"));
        }
        let cipher = XChaCha20Poly1305::new_from_slice(master_key)
            .map_err(|_| AppError::runtime("runtime secret master key is invalid"))?;
        cipher
            .decrypt(XNonce::from_slice(nonce), ciphertext)
            .map_err(|error| {
                AppError::runtime(format!("failed to decrypt runtime secret: {error}"))
            })
    }
}

#[async_trait]
impl SecretVault for RuntimeSecretVault {
    async fn get(&self, ref_id: &str) -> Result<SecretValue, VaultError> {
        self.get_optional_bytes(ref_id)
            .map_err(Self::to_vault_error)?
            .map(SecretValue::new)
            .ok_or(VaultError::NotFound)
    }

    async fn put(&self, ref_id: &str, value: SecretValue) -> Result<(), VaultError> {
        self.put_bytes(ref_id, value.as_bytes())
            .map_err(Self::to_vault_error)
    }
}
