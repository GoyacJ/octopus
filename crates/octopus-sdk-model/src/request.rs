//! Canonical model request types land here in Task 3.

use std::pin::Pin;

use futures::Stream;
use serde::Serialize;
use sha2::{Digest, Sha256};

use octopus_sdk_contracts::{AssistantEvent, CacheBreakpoint, Message};

use crate::{ModelError, ModelId, ModelRole};

pub use octopus_sdk_contracts::ToolSchema;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelRequest {
    pub model: ModelId,
    pub system_prompt: Vec<String>,
    pub messages: Vec<Message>,
    pub tools: Vec<ToolSchema>,
    pub role: ModelRole,
    pub cache_breakpoints: Vec<CacheBreakpoint>,
    pub response_format: Option<ResponseFormat>,
    pub thinking: Option<ThinkingConfig>,
    pub cache_control: CacheControlStrategy,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: bool,
}

impl ModelRequest {
    #[must_use]
    pub fn tools_fingerprint(&self) -> String {
        let joined = self
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>()
            .join("\t");

        format!("{:x}", Sha256::digest(joined.as_bytes()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    Json { schema: serde_json::Value },
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheControlStrategy {
    None,
    PromptCaching { breakpoints: Vec<&'static str> },
    ContextCacheObject { cache_id: String },
}

pub type ModelStream = Pin<Box<dyn Stream<Item = Result<AssistantEvent, ModelError>> + Send>>;

#[cfg(test)]
mod tests {
    use serde_json::{json, to_value};

    use octopus_sdk_contracts::{CacheBreakpoint, CacheTtl, Message, Role, ToolSchema};

    use super::{CacheControlStrategy, ModelRequest, ResponseFormat, ThinkingConfig};
    use crate::{ModelId, ModelRole};

    fn text_message(role: Role, text: &str) -> Message {
        Message {
            role,
            content: vec![octopus_sdk_contracts::ContentBlock::Text {
                text: text.to_string(),
            }],
        }
    }

    #[test]
    fn tools_fingerprint_preserves_declared_order() {
        let request = ModelRequest {
            model: ModelId("claude-opus-4-6".to_string()),
            system_prompt: vec!["You are precise.".to_string()],
            messages: vec![text_message(Role::User, "Summarize this.")],
            tools: vec![
                ToolSchema {
                    name: "search".to_string(),
                    description: "Search docs".to_string(),
                    input_schema: json!({"type": "object"}),
                },
                ToolSchema {
                    name: "bash".to_string(),
                    description: "Run shell".to_string(),
                    input_schema: json!({"type": "object"}),
                },
            ],
            role: ModelRole::Main,
            cache_breakpoints: vec![CacheBreakpoint {
                position: 0,
                ttl: CacheTtl::FiveMinutes,
            }],
            response_format: Some(ResponseFormat::Json {
                schema: json!({"type": "object", "properties": {"answer": {"type": "string"}}}),
            }),
            thinking: Some(ThinkingConfig {
                enabled: true,
                budget_tokens: Some(512),
            }),
            cache_control: CacheControlStrategy::PromptCaching {
                breakpoints: vec!["system", "tools"],
            },
            max_tokens: Some(4096),
            temperature: Some(0.2),
            stream: true,
        };

        assert_eq!(
            request.tools_fingerprint(),
            "019eca04052e4bb6366195d8a1dbf2816318c956568c9364e0b54b47ced76c78"
        );
    }

    #[test]
    fn model_request_serializes_canonical_optional_fields() {
        let value = to_value(ModelRequest {
            model: ModelId("claude-opus-4-6".to_string()),
            system_prompt: vec!["System".to_string()],
            messages: vec![text_message(Role::User, "Hello")],
            tools: vec![],
            role: ModelRole::Plan,
            cache_breakpoints: vec![],
            response_format: Some(ResponseFormat::Text),
            thinking: Some(ThinkingConfig {
                enabled: false,
                budget_tokens: None,
            }),
            cache_control: CacheControlStrategy::ContextCacheObject {
                cache_id: "ctx-123".to_string(),
            },
            max_tokens: Some(256),
            temperature: None,
            stream: false,
        })
        .unwrap();

        assert_eq!(value["response_format"], json!("text"));
        assert_eq!(value["thinking"]["enabled"], json!(false));
        assert_eq!(
            value["cache_control"]["context_cache_object"]["cache_id"],
            json!("ctx-123")
        );
        assert_eq!(value["stream"], json!(false));
    }
}
