use std::sync::Arc;

use futures::StreamExt;
use octopus_sdk_contracts::{
    AssistantEvent, CacheBreakpoint, CacheTtl, CompactionResult, CompactionStrategyTag,
    ContentBlock, EventId, Message, Role,
};
use octopus_sdk_model::{
    CacheControlStrategy, ModelError, ModelId, ModelProvider, ModelRequest, ModelRole,
};
use thiserror::Error;

pub struct SessionView<'a> {
    pub messages: &'a mut Vec<Message>,
    pub tokens: u32,
    pub tokens_budget: u32,
    pub event_ids: Vec<EventId>,
}

pub struct Compactor {
    threshold: f32,
    strategy: CompactionStrategyTag,
    provider: Arc<dyn ModelProvider>,
}

impl Compactor {
    #[must_use]
    pub fn new(
        threshold: f32,
        strategy: CompactionStrategyTag,
        provider: Arc<dyn ModelProvider>,
    ) -> Self {
        Self {
            threshold,
            strategy,
            provider,
        }
    }

    pub async fn maybe_compact(
        &self,
        session: &mut SessionView<'_>,
    ) -> Result<Option<CompactionResult>, CompactionError> {
        let usage_ratio = f64::from(session.tokens) / f64::from(session.tokens_budget.max(1));
        if session.tokens_budget == 0 || usage_ratio < f64::from(self.threshold) {
            return Ok(None);
        }

        match self.strategy {
            CompactionStrategyTag::ClearToolResults => {
                let tokens_before = session.tokens;
                let cleared = self.clear_tool_results(session).await;
                Ok(Some(CompactionResult {
                    summary: String::new(),
                    folded_turn_ids: Vec::new(),
                    tool_results_cleared: cleared,
                    tokens_before,
                    tokens_after: session.tokens,
                    strategy: CompactionStrategyTag::ClearToolResults,
                }))
            }
            CompactionStrategyTag::Summarize => Ok(Some(self.summarize(session).await?)),
            CompactionStrategyTag::Hybrid => Ok(Some(self.hybrid(session).await?)),
        }
    }

    pub async fn clear_tool_results(&self, session: &mut SessionView<'_>) -> u32 {
        let mut cleared = 0;
        for message in &mut *session.messages {
            cleared += clear_blocks(&mut message.content);
        }
        session.tokens = estimate_tokens(session.messages);
        cleared
    }

    pub async fn summarize(
        &self,
        session: &mut SessionView<'_>,
    ) -> Result<CompactionResult, CompactionError> {
        if session.messages.is_empty() {
            return Err(CompactionError::Aborted {
                reason: "no messages to compact".into(),
            });
        }

        let split = (session.messages.len() / 2).max(1);
        let prefix_messages = session.messages[..split].to_vec();
        let has_user_message = prefix_messages
            .iter()
            .any(|message| matches!(message.role, Role::User));
        let tokens_before = session.tokens;
        let folded_turn_ids = if session.event_ids.len() >= split {
            session.event_ids[..split].to_vec()
        } else {
            (0..split)
                .map(|index| EventId(format!("synthetic-turn-{index}")))
                .collect()
        };

        let mut stream = self
            .provider
            .complete(ModelRequest {
                model: ModelId("compact".into()),
                system_prompt: vec!["Summarize the earlier conversation turns.".into()],
                messages: prefix_messages,
                tools: Vec::new(),
                role: ModelRole::Compact,
                cache_breakpoints: compact_cache_breakpoints(has_user_message),
                response_format: None,
                thinking: None,
                cache_control: compact_cache_control(has_user_message),
                max_tokens: None,
                temperature: None,
                stream: true,
            })
            .await?;

        let mut summary = String::new();
        while let Some(event) = stream.next().await {
            match event? {
                AssistantEvent::TextDelta(text) => summary.push_str(&text),
                AssistantEvent::ToolUse { .. }
                | AssistantEvent::Usage(_)
                | AssistantEvent::PromptCache(_)
                | AssistantEvent::MessageStop { .. } => {}
            }
        }

        if summary.trim().is_empty() {
            return Err(CompactionError::ModelUnavailable);
        }

        if estimate_text_tokens(&summary) >= tokens_before {
            return Err(CompactionError::SummaryTooLarge);
        }

        session.messages.splice(
            ..split,
            [Message {
                role: Role::System,
                content: vec![ContentBlock::Text {
                    text: summary.clone(),
                }],
            }],
        );
        if session.event_ids.len() >= split {
            session.event_ids.splice(
                ..split,
                [EventId(format!(
                    "summary:{}",
                    folded_turn_ids
                        .first()
                        .map_or("0", |event_id| event_id.0.as_str())
                ))],
            );
        }
        session.tokens = estimate_tokens(session.messages);

        Ok(CompactionResult {
            summary,
            folded_turn_ids,
            tool_results_cleared: 0,
            tokens_before,
            tokens_after: session.tokens,
            strategy: CompactionStrategyTag::Summarize,
        })
    }

    async fn hybrid(
        &self,
        session: &mut SessionView<'_>,
    ) -> Result<CompactionResult, CompactionError> {
        let tokens_before = session.tokens;
        let cleared = self.clear_tool_results(session).await;
        let usage_ratio = f64::from(session.tokens) / f64::from(session.tokens_budget.max(1));
        if session.tokens_budget == 0 || usage_ratio < f64::from(self.threshold) {
            return Ok(CompactionResult {
                summary: String::new(),
                folded_turn_ids: Vec::new(),
                tool_results_cleared: cleared,
                tokens_before,
                tokens_after: session.tokens,
                strategy: CompactionStrategyTag::Hybrid,
            });
        }

        let mut result = self.summarize(session).await?;
        result.tool_results_cleared = cleared;
        result.tokens_before = tokens_before;
        result.strategy = CompactionStrategyTag::Hybrid;
        Ok(result)
    }
}

#[derive(Debug, Error)]
pub enum CompactionError {
    #[error("compaction model unavailable")]
    ModelUnavailable,
    #[error("summary is too large")]
    SummaryTooLarge,
    #[error("compaction aborted: {reason}")]
    Aborted { reason: String },
    #[error(transparent)]
    Provider(#[from] ModelError),
}

fn clear_blocks(blocks: &mut Vec<ContentBlock>) -> u32 {
    let mut cleared = 0;
    for block in blocks {
        match block {
            ContentBlock::ToolResult { content, .. } => {
                if !content.is_empty() {
                    content.clear();
                    cleared += 1;
                }
            }
            ContentBlock::ToolUse { .. }
            | ContentBlock::Text { .. }
            | ContentBlock::Thinking { .. } => {}
        }
    }
    cleared
}

fn estimate_tokens(messages: &[Message]) -> u32 {
    let chars = messages
        .iter()
        .map(|message| estimate_blocks_chars(&message.content))
        .sum::<usize>();
    estimate_chars_tokens(chars)
}

fn estimate_blocks_chars(blocks: &[ContentBlock]) -> usize {
    blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } | ContentBlock::Thinking { text } => text.len(),
            ContentBlock::ToolUse { name, input, .. } => name.len() + input.to_string().len(),
            ContentBlock::ToolResult { content, .. } => estimate_blocks_chars(content),
        })
        .sum()
}

fn estimate_text_tokens(text: &str) -> u32 {
    estimate_chars_tokens(text.len())
}

fn estimate_chars_tokens(chars: usize) -> u32 {
    if chars == 0 {
        0
    } else {
        u32::try_from(chars).unwrap_or(u32::MAX).div_ceil(4)
    }
}

fn compact_cache_breakpoints(has_user_message: bool) -> Vec<CacheBreakpoint> {
    let mut breakpoints = vec![CacheBreakpoint {
        position: 0,
        ttl: CacheTtl::OneHour,
    }];
    if has_user_message {
        breakpoints.push(CacheBreakpoint {
            position: 1,
            ttl: CacheTtl::FiveMinutes,
        });
    }
    breakpoints
}

fn compact_cache_control(has_user_message: bool) -> CacheControlStrategy {
    let mut breakpoints = vec!["system"];
    if has_user_message {
        breakpoints.push("first_user");
    }
    CacheControlStrategy::PromptCaching { breakpoints }
}
