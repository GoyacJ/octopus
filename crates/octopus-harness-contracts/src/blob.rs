//! Blob references and storage contracts.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md §3.7

use bytes::Bytes;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{BlobId, SessionId, TenantId};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct BlobRef {
    pub id: BlobId,
    pub size: u64,
    pub content_hash: [u8; 32],
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct TranscriptRef {
    pub blob: BlobRef,
    pub from_offset: crate::JournalOffset,
    pub to_offset: crate::JournalOffset,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct BlobMeta {
    pub content_type: Option<String>,
    pub size: u64,
    pub content_hash: [u8; 32],
    pub created_at: DateTime<Utc>,
    pub retention: BlobRetention,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BlobRetention {
    SessionScoped(SessionId),
    TenantScoped,
    RetainForever,
    TtlDays(u32),
}

#[non_exhaustive]
#[derive(
    Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema, thiserror::Error,
)]
#[serde(rename_all = "snake_case")]
pub enum BlobError {
    #[error("blob not found: {0:?}")]
    NotFound(BlobId),
    #[error("content hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("size exceeds limit: {size} > {limit}")]
    TooLarge { size: u64, limit: u64 },
    #[error("tenant denied: {0:?}")]
    TenantDenied(TenantId),
    #[error("io: {0}")]
    Io(String),
    #[error("backend: {0}")]
    Backend(String),
}

pub trait BlobStore: Send + Sync + 'static {
    fn store_id(&self) -> &str;

    fn put(&self, tenant: TenantId, bytes: Bytes, meta: BlobMeta) -> Result<BlobRef, BlobError>;

    fn get(&self, tenant: TenantId, blob: &BlobRef) -> Result<Bytes, BlobError>;

    fn head(&self, tenant: TenantId, blob: &BlobRef) -> Result<Option<BlobMeta>, BlobError>;

    fn delete(&self, tenant: TenantId, blob: &BlobRef) -> Result<(), BlobError>;
}
