use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PluginLoadedEvent {
    pub plugin_id: PluginId,
    pub tenant_id: TenantId,
    pub name: String,
    pub version: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PluginRejectedEvent {
    pub tenant_id: TenantId,
    pub manifest_origin: ManifestOriginRef,
    pub reason: PluginRejectedReason,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ManifestValidationFailedEvent {
    pub tenant_id: TenantId,
    pub manifest_origin: ManifestOriginRef,
    pub partial_name: Option<String>,
    pub partial_version: Option<String>,
    pub raw_bytes_hash: [u8; 32],
    pub failure: ManifestValidationFailure,
    pub at: DateTime<Utc>,
}
