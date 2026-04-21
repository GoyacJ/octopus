use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Note,
    Decision,
    Todo,
    SkillLog,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub kind: MemoryKind,
    pub payload: serde_json::Value,
    pub created_at_ms: i64,
}

#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("memory item not found")]
    NotFound,
    #[error("memory backend error: {reason}")]
    Backend { reason: String },
    #[error("memory serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
