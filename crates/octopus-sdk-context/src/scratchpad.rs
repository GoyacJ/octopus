use std::io::Write;
use std::path::PathBuf;

use octopus_sdk_contracts::{MemoryError, SessionId};
use tempfile::NamedTempFile;

#[derive(Debug, Clone)]
pub struct DurableScratchpad {
    base: PathBuf,
}

impl DurableScratchpad {
    #[must_use]
    pub fn new(base: PathBuf) -> Self {
        Self { base }
    }

    pub async fn read(&self, session: &SessionId) -> Result<Option<String>, MemoryError> {
        let path = self.path_for(session);
        match tokio::fs::read_to_string(path).await {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(MemoryError::Backend {
                reason: error.to_string(),
            }),
        }
    }

    pub async fn write(&self, session: &SessionId, content: &str) -> Result<(), MemoryError> {
        let path = self.path_for(session);
        let Some(parent) = path.parent() else {
            return Err(MemoryError::Backend {
                reason: "scratchpad path has no parent".into(),
            });
        };
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| MemoryError::Backend {
                reason: error.to_string(),
            })?;

        let mut temp = NamedTempFile::new_in(parent).map_err(|error| MemoryError::Backend {
            reason: error.to_string(),
        })?;
        temp.write_all(content.as_bytes())
            .map_err(|error| MemoryError::Backend {
                reason: error.to_string(),
            })?;
        temp.flush().map_err(|error| MemoryError::Backend {
            reason: error.to_string(),
        })?;

        temp.persist(&path).map_err(|error| MemoryError::Backend {
            reason: error.error.to_string(),
        })?;
        Ok(())
    }

    fn path_for(&self, session: &SessionId) -> PathBuf {
        self.base
            .join("runtime")
            .join("notes")
            .join(format!("{}.md", session.0))
    }
}
