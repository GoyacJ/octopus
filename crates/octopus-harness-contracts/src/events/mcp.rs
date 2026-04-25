use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpToolInjectedEvent {
    pub server_id: McpServerId,
    pub tool_name: String,
    pub canonical_name: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpConnectionLostEvent {
    pub server_id: McpServerId,
    pub reason: McpConnectionLostReason,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpConnectionRecoveredEvent {
    pub server_id: McpServerId,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpElicitationRequestedEvent {
    pub server_id: McpServerId,
    pub request_id: RequestId,
    pub prompt: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpElicitationResolvedEvent {
    pub server_id: McpServerId,
    pub request_id: RequestId,
    pub value: Option<Value>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpToolsListChangedEvent {
    pub server_id: McpServerId,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpResourceUpdatedEvent {
    pub server_id: McpServerId,
    pub uri: String,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct McpSamplingRequestedEvent {
    pub server_id: McpServerId,
    pub request_id: RequestId,
    pub model_hint: Option<String>,
    pub at: DateTime<Utc>,
}
