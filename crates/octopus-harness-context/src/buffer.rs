use std::collections::HashMap;
use std::sync::Arc;

use harness_contracts::{
    BlobRef, Message, MessageId, SessionId, TenantId, ToolDescriptor, ToolUseId,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContextBuffer {
    pub identity: ContextIdentity,
    pub frozen: FrozenContext,
    pub active: ActiveContext,
    pub patches: Vec<ContextPatch>,
    pub deferred_tools_delta: Option<DeferredToolsDelta>,
    pub bookkeeping: ContextBookkeeping,
}

impl ContextBuffer {
    pub fn new(tenant_id: TenantId, session_id: SessionId) -> Self {
        Self {
            identity: ContextIdentity {
                tenant_id,
                session_id,
            },
            ..Self::default()
        }
    }

    pub fn rebuild_tool_use_pairs(&mut self) {
        self.active.tool_use_pairs = rebuild_tool_use_pairs(&self.active.history);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextIdentity {
    pub tenant_id: TenantId,
    pub session_id: SessionId,
}

impl Default for ContextIdentity {
    fn default() -> Self {
        Self {
            tenant_id: TenantId::SINGLE,
            session_id: SessionId::from_u128(0),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrozenContext {
    pub system_header: Option<Arc<str>>,
    pub tools_snapshot: Arc<ContextToolSnapshot>,
    pub memory_snapshot_id: Option<String>,
    pub bootstrap_snapshot_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContextToolSnapshot {
    pub descriptors: Vec<ToolDescriptor>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ActiveContext {
    pub history: Vec<Message>,
    pub tool_use_pairs: Vec<ToolUsePair>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolUsePair {
    pub tool_use_id: ToolUseId,
    pub tool_use_message_id: MessageId,
    pub tool_result_message_id: Option<MessageId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextPatch {
    MemoryRecall {
        fence: String,
        lifecycle: ContentLifecycle,
    },
    SkillInjection {
        skill_id: String,
        body: String,
        lifecycle: ContentLifecycle,
    },
    HookAddContext {
        handler_id: String,
        body: String,
        lifecycle: ContentLifecycle,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentLifecycle {
    Transient,
    Persistent { ttl_turns: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeferredToolsDelta {
    pub summary: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ContextBookkeeping {
    pub offloads: HashMap<MessageId, BlobRef>,
    pub budget_snapshot: TokenBudget,
    pub estimated_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenBudget {
    pub max_tokens_per_turn: u64,
    pub max_tokens_per_session: u64,
    pub soft_budget_ratio: f32,
    pub hard_budget_ratio: f32,
    pub per_tool_max_chars: u64,
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self {
            max_tokens_per_turn: 200_000,
            max_tokens_per_session: 1_000_000,
            soft_budget_ratio: 0.8,
            hard_budget_ratio: 0.95,
            per_tool_max_chars: 30_000,
        }
    }
}

pub(crate) fn rebuild_tool_use_pairs(messages: &[Message]) -> Vec<ToolUsePair> {
    let mut pairs = Vec::new();

    for message in messages {
        for part in &message.parts {
            if let harness_contracts::MessagePart::ToolUse { id, .. } = part {
                pairs.push(ToolUsePair {
                    tool_use_id: *id,
                    tool_use_message_id: message.id,
                    tool_result_message_id: None,
                });
            }
        }
    }

    for message in messages {
        for part in &message.parts {
            if let harness_contracts::MessagePart::ToolResult { tool_use_id, .. } = part {
                if let Some(pair) = pairs
                    .iter_mut()
                    .find(|pair| pair.tool_use_id == *tool_use_id)
                {
                    pair.tool_result_message_id = Some(message.id);
                }
            }
        }
    }

    pairs
}
