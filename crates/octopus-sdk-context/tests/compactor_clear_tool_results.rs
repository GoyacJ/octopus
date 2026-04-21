use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_context::{Compactor, SessionView};
use octopus_sdk_contracts::{CompactionStrategyTag, ContentBlock, EventId, Message, Role};
use octopus_sdk_model::{ModelError, ModelProvider, ModelRequest, ModelStream, ProtocolFamily, ProviderDescriptor, ProviderId};

struct NoopProvider;

#[async_trait]
impl ModelProvider for NoopProvider {
    async fn complete(&self, _req: ModelRequest) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(futures::stream::empty()))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("noop".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

#[tokio::test]
async fn test_below_threshold_noop() {
    let mut messages = sample_messages();
    let original = messages.clone();
    let mut session = SessionView {
        tokens: 10,
        tokens_budget: 100,
        event_ids: sample_event_ids(messages.len()),
        messages: &mut messages,
    };
    let compactor = Compactor::new(
        0.5,
        CompactionStrategyTag::ClearToolResults,
        Arc::new(NoopProvider),
    );

    let result = compactor
        .maybe_compact(&mut session)
        .await
        .expect("noop path should succeed");

    assert_eq!(result, None);
    assert_eq!(*session.messages, original);
}

#[tokio::test]
async fn clear_tool_results_updates_message_bodies_and_token_count() {
    let mut messages = sample_messages();
    let tokens_before = estimate_tokens(&messages);
    let removed_tokens = removed_tool_result_tokens(&messages);
    let mut session = SessionView {
        tokens: tokens_before,
        tokens_budget: tokens_before,
        event_ids: sample_event_ids(messages.len()),
        messages: &mut messages,
    };
    let compactor = Compactor::new(
        0.5,
        CompactionStrategyTag::ClearToolResults,
        Arc::new(NoopProvider),
    );

    let result = compactor
        .maybe_compact(&mut session)
        .await
        .expect("clear strategy should succeed")
        .expect("clear strategy should return a result");

    assert_eq!(result.tool_results_cleared, 3);
    assert_eq!(result.strategy, CompactionStrategyTag::ClearToolResults);
    assert_eq!(result.tokens_before - result.tokens_after, removed_tokens);
    assert!(
        session.messages.iter().all(|message| {
            message.content.iter().all(|block| match block {
                ContentBlock::ToolResult { content, .. } => content.is_empty(),
                _ => true,
            })
        })
    );
}

#[tokio::test]
async fn hybrid_strategy_is_explicitly_aborted() {
    let mut messages = sample_messages();
    let mut session = SessionView {
        tokens: estimate_tokens(&messages),
        tokens_budget: estimate_tokens(&messages),
        event_ids: sample_event_ids(messages.len()),
        messages: &mut messages,
    };
    let compactor = Compactor::new(0.1, CompactionStrategyTag::Hybrid, Arc::new(NoopProvider));

    let error = compactor
        .maybe_compact(&mut session)
        .await
        .expect_err("hybrid should abort");

    assert_eq!(error.to_string(), "compaction aborted: hybrid not implemented in W4");
}

fn sample_messages() -> Vec<Message> {
    vec![
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "Inspect the file".into(),
            }],
        },
        Message {
            role: Role::Tool,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: octopus_sdk_contracts::ToolCallId("call-1".into()),
                content: vec![ContentBlock::Text {
                    text: "line one from grep".into(),
                }],
                is_error: false,
            }],
        },
        Message {
            role: Role::Tool,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: octopus_sdk_contracts::ToolCallId("call-2".into()),
                content: vec![ContentBlock::Text {
                    text: "line two from grep".into(),
                }],
                is_error: false,
            }],
        },
        Message {
            role: Role::Tool,
            content: vec![ContentBlock::ToolResult {
                tool_use_id: octopus_sdk_contracts::ToolCallId("call-3".into()),
                content: vec![ContentBlock::Text {
                    text: "line three from grep".into(),
                }],
                is_error: false,
            }],
        },
    ]
}

fn sample_event_ids(count: usize) -> Vec<EventId> {
    (0..count)
        .map(|index| EventId(format!("event-{index}")))
        .collect()
}

fn removed_tool_result_tokens(messages: &[Message]) -> u32 {
    messages
        .iter()
        .flat_map(|message| message.content.iter())
        .map(|block| match block {
            ContentBlock::ToolResult { content, .. } => estimate_blocks_tokens(content),
            _ => 0,
        })
        .sum()
}

fn estimate_tokens(messages: &[Message]) -> u32 {
    messages
        .iter()
        .map(|message| estimate_blocks_tokens(&message.content))
        .sum()
}

fn estimate_blocks_tokens(blocks: &[ContentBlock]) -> u32 {
    blocks
        .iter()
        .map(|block| match block {
            ContentBlock::Text { text } | ContentBlock::Thinking { text } => chars_to_tokens(text.len()),
            ContentBlock::ToolUse { name, input, .. } => {
                chars_to_tokens(name.len() + input.to_string().len())
            }
            ContentBlock::ToolResult { content, .. } => estimate_blocks_tokens(content),
        })
        .sum()
}

fn chars_to_tokens(chars: usize) -> u32 {
    if chars == 0 {
        0
    } else {
        ((chars as u32) + 3) / 4
    }
}
