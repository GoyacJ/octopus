use serde::{Deserialize, Serialize};

use crate::{
    CompactionResult, CompactionStrategyTag, ContentBlock, Message, RenderBlock, SessionId,
    ToolCallRequest, ToolCategory,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HookToolResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub render: Option<RenderBlock>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompactionCtx {
    pub session: SessionId,
    pub strategy: CompactionStrategyTag,
    pub threshold: f32,
    pub tokens_current: u32,
    pub tokens_budget: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    Normal,
    MaxTurns,
    UserCancelled,
    Error(String),
    Compaction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RewritePayload {
    ToolCall { call: ToolCallRequest },
    ToolResult { result: HookToolResult },
    UserPrompt { message: Message },
    Compaction { ctx: CompactionCtx },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookDecision {
    Continue,
    Rewrite(RewritePayload),
    Abort { reason: String },
    InjectMessage(Message),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HookEvent {
    PreToolUse {
        call: ToolCallRequest,
        category: ToolCategory,
    },
    PostToolUse {
        call: ToolCallRequest,
        result: HookToolResult,
    },
    Stop {
        session: SessionId,
    },
    SessionStart {
        session: SessionId,
    },
    SessionEnd {
        session: SessionId,
        reason: EndReason,
    },
    UserPromptSubmit {
        message: Message,
    },
    PreCompact {
        session: SessionId,
        ctx: CompactionCtx,
    },
    PostCompact {
        session: SessionId,
        result: CompactionResult,
    },
}

impl HookEvent {
    pub const VARIANT_COUNT: usize = 8;
}
