//! Message, turn input, and tool result contracts.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md §3.5

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BlobRef, MemoryId, MessageId, ToolUseId, TranscriptRef, UsageSnapshot};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TurnInput {
    pub message: Message,
    pub metadata: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Message {
    pub id: MessageId,
    pub role: MessageRole,
    pub parts: Vec<MessagePart>,
    pub created_at: DateTime<Utc>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MessagePart {
    Text {
        text: String,
    },
    Image {
        mime_type: String,
        blob_ref: BlobRef,
    },
    Audio {
        mime_type: String,
        blob_ref: BlobRef,
    },
    File {
        name: String,
        mime_type: String,
        blob_ref: BlobRef,
    },
    ToolResult {
        tool_use_id: ToolUseId,
        result: ToolResult,
    },
    Thinking(ThinkingBlock),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ThinkingBlock {
    pub text: String,
    pub signature: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolResult {
    Text(String),
    Structured(Value),
    Blob {
        content_type: String,
        blob_ref: BlobRef,
    },
    Mixed(Vec<ToolResultPart>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolResultEnvelope {
    pub result: ToolResult,
    pub usage: Option<UsageSnapshot>,
    pub is_error: bool,
    pub overflow: Option<crate::OverflowMetadata>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolResultPart {
    Text {
        text: String,
    },
    Structured {
        value: Value,
        schema_ref: Option<String>,
    },
    Blob {
        content_type: String,
        blob_ref: BlobRef,
        summary: Option<String>,
    },
    Code {
        language: String,
        text: String,
    },
    Reference {
        reference_kind: ReferenceKind,
        title: Option<String>,
        summary: Option<String>,
    },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<Value>>,
        caption: Option<String>,
    },
    Progress {
        stage: String,
        ratio: Option<f32>,
        detail: Option<String>,
    },
    Error {
        code: String,
        message: String,
        retriable: bool,
    },
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "ref_kind", rename_all = "snake_case")]
pub enum ReferenceKind {
    Url {
        url: String,
    },
    File {
        path: PathBuf,
        line_range: Option<(u32, u32)>,
    },
    Transcript(TranscriptRef),
    ToolUse {
        tool_use_id: ToolUseId,
    },
    Memory {
        memory_id: MemoryId,
    },
}
