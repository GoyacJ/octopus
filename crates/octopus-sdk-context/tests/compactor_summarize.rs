use std::sync::Arc;

use async_trait::async_trait;
use octopus_sdk_context::{Compactor, SessionView};
use octopus_sdk_contracts::{
    AssistantEvent, CacheBreakpoint, CacheTtl, CompactionStrategyTag, ContentBlock, EventId,
    Message, Role,
};
use octopus_sdk_model::{
    CacheControlStrategy, ModelError, ModelProvider, ModelRequest, ModelRole, ModelStream,
    ProtocolFamily, ProviderDescriptor, ProviderId,
};

struct MockProvider {
    summary: String,
}

#[async_trait]
impl ModelProvider for MockProvider {
    async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError> {
        assert_eq!(req.role, ModelRole::Compact);
        assert_eq!(
            req.cache_breakpoints,
            vec![
                CacheBreakpoint {
                    position: 0,
                    ttl: CacheTtl::OneHour,
                },
                CacheBreakpoint {
                    position: 1,
                    ttl: CacheTtl::FiveMinutes,
                },
            ]
        );
        assert_eq!(
            req.cache_control,
            CacheControlStrategy::PromptCaching {
                breakpoints: vec!["system", "first_user"],
            }
        );
        let summary = self.summary.clone();
        Ok(Box::pin(futures::stream::iter(vec![Ok(
            AssistantEvent::TextDelta(summary),
        )])))
    }

    fn describe(&self) -> ProviderDescriptor {
        ProviderDescriptor {
            id: ProviderId("mock".into()),
            supported_families: vec![ProtocolFamily::VendorNative],
            catalog_version: "test".into(),
        }
    }
}

#[tokio::test]
async fn summarize_rewrites_prefix_into_single_system_message() {
    for _ in 0..3 {
        let mut messages = sample_messages();
        let mut session = SessionView {
            tokens: estimate_tokens(&messages),
            tokens_budget: estimate_tokens(&messages),
            event_ids: sample_event_ids(messages.len()),
            messages: &mut messages,
        };
        let compactor = Compactor::new(
            0.5,
            CompactionStrategyTag::Summarize,
            Arc::new(MockProvider {
                summary: "SUMMARY: turns [0..1] condensed".into(),
            }),
        );

        let result = compactor
            .maybe_compact(&mut session)
            .await
            .expect("summarize should succeed")
            .expect("summarize should produce a result");

        assert_eq!(result.summary, "SUMMARY: turns [0..1] condensed");
        assert_eq!(
            result.folded_turn_ids,
            vec![EventId("event-0".into()), EventId("event-1".into())]
        );
        assert_eq!(result.strategy, CompactionStrategyTag::Summarize);
        assert_eq!(session.messages.len(), 3);
        assert!(matches!(session.messages[0].role, Role::System));
        assert_eq!(
            session.messages[0].content,
            vec![ContentBlock::Text {
                text: "SUMMARY: turns [0..1] condensed".into(),
            }]
        );
    }
}

fn sample_messages() -> Vec<Message> {
    vec![
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "first turn".into(),
            }],
        },
        Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: "second turn".into(),
            }],
        },
        Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: "third turn".into(),
            }],
        },
        Message {
            role: Role::Assistant,
            content: vec![ContentBlock::Text {
                text: "fourth turn".into(),
            }],
        },
    ]
}

fn sample_event_ids(count: usize) -> Vec<EventId> {
    (0..count)
        .map(|index| EventId(format!("event-{index}")))
        .collect()
}

fn estimate_tokens(messages: &[Message]) -> u32 {
    messages
        .iter()
        .map(|message| {
            message
                .content
                .iter()
                .map(|block| match block {
                    ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
                        chars_to_tokens(text.len())
                    }
                    ContentBlock::ToolUse { name, input, .. } => {
                        chars_to_tokens(name.len() + input.to_string().len())
                    }
                    ContentBlock::ToolResult { content, .. } => content
                        .iter()
                        .map(|nested| match nested {
                            ContentBlock::Text { text } | ContentBlock::Thinking { text } => {
                                chars_to_tokens(text.len())
                            }
                            ContentBlock::ToolUse { name, input, .. } => {
                                chars_to_tokens(name.len() + input.to_string().len())
                            }
                            ContentBlock::ToolResult { .. } => 0,
                        })
                        .sum(),
                })
                .sum::<u32>()
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
