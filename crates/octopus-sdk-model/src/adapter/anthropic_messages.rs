use std::collections::BTreeMap;

use async_stream::try_stream;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::header::{HeaderName, HeaderValue};
use serde_json::{json, Value};

use octopus_sdk_contracts::SecretVault;
use octopus_sdk_contracts::{AssistantEvent, ToolCallId, Usage};

use crate::{ModelError, ModelRequest, ModelStream, ProtocolAdapter, ProtocolFamily, Provider};

use super::{
    header_value_from_secret, json_error, map_stop_reason, prompt_cache_event,
    sse::{IncrementalSseParser, SseEvent},
    StreamBytes,
};

#[derive(Debug, Default)]
pub struct AnthropicMessagesAdapter;

#[async_trait]
impl ProtocolAdapter for AnthropicMessagesAdapter {
    fn family(&self) -> ProtocolFamily {
        ProtocolFamily::AnthropicMessages
    }

    fn to_request(&self, req: &ModelRequest) -> Result<Value, ModelError> {
        Ok(json!({
            "model": req.model.0,
            "system": req.system_prompt,
            "messages": req.messages.iter().map(anthropic_message).collect::<Result<Vec<_>, _>>()?,
            "tools": req.tools.iter().map(|tool| {
                json!({
                    "name": tool.name,
                    "description": tool.description,
                    "input_schema": tool.input_schema,
                })
            }).collect::<Vec<_>>(),
            "max_tokens": req.max_tokens,
            "stream": req.stream,
            "cache_control": anthropic_cache_control(&req.cache_control),
            "response_format": req.response_format,
            "thinking": req.thinking,
        }))
    }

    fn parse_stream(&self, raw: StreamBytes) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(try_stream! {
            let mut raw = raw;
            let mut sse = IncrementalSseParser::default();
            let mut format = None;
            let mut json_body = Vec::new();
            let mut state = AnthropicStreamState::default();

            while let Some(chunk) = raw.next().await {
                let chunk = chunk?;
                if format.is_none() {
                    format = Some(if starts_with_json(&chunk) {
                        WireFormat::Json
                    } else {
                        WireFormat::Sse
                    });
                }

                match format.expect("format is set") {
                    WireFormat::Json => json_body.extend_from_slice(&chunk),
                    WireFormat::Sse => {
                        for event in sse.push(&chunk)? {
                            for assistant_event in state.ingest(event)? {
                                yield assistant_event;
                            }
                        }
                    }
                }
            }

            match format.unwrap_or(WireFormat::Json) {
                WireFormat::Json => {
                    for event in parse_json_response(&json_body)? {
                        yield event;
                    }
                }
                WireFormat::Sse => {
                    for event in sse.finish()? {
                        for assistant_event in state.ingest(event)? {
                            yield assistant_event;
                        }
                    }
                    for assistant_event in state.finish()? {
                        yield assistant_event;
                    }
                }
            }
        }))
    }

    async fn auth_headers(
        &self,
        vault: &dyn SecretVault,
        provider: &Provider,
    ) -> Result<Vec<(HeaderName, HeaderValue)>, ModelError> {
        let secret = vault
            .get(&format!("{}_api_key", provider.id.0))
            .await
            .map_err(|_| ModelError::AuthMissing {
                provider: provider.id.clone(),
            })?;

        Ok(vec![
            (
                HeaderName::from_static("x-api-key"),
                header_value_from_secret(secret)?,
            ),
            (
                HeaderName::from_static("anthropic-version"),
                HeaderValue::from_static("2023-06-01"),
            ),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireFormat {
    Json,
    Sse,
}

#[derive(Debug, Default)]
struct AnthropicStreamState {
    tools: BTreeMap<u32, AnthropicToolState>,
    stop_reason: Option<octopus_sdk_contracts::StopReason>,
    stopped: bool,
}

impl AnthropicStreamState {
    fn ingest(&mut self, event: SseEvent) -> Result<Vec<AssistantEvent>, ModelError> {
        if event.data == "[DONE]" {
            return self.finish();
        }

        let payload: Value = serde_json::from_str(&event.data)?;
        let event_name = event
            .event
            .as_deref()
            .or_else(|| payload.get("type").and_then(Value::as_str))
            .unwrap_or_default();
        let mut out = Vec::new();

        match event_name {
            "content_block_start" => {
                if payload
                    .get("content_block")
                    .and_then(|value| value.get("type"))
                    .and_then(Value::as_str)
                    == Some("tool_use")
                {
                    let index = value_as_u32(payload.get("index"))?;
                    self.tools.insert(
                        index,
                        AnthropicToolState {
                            id: payload["content_block"]["id"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string(),
                            name: payload["content_block"]["name"]
                                .as_str()
                                .unwrap_or_default()
                                .to_string(),
                            input_json: match payload["content_block"].get("input") {
                                Some(Value::Object(object)) if object.is_empty() => String::new(),
                                Some(value) => value.to_string(),
                                None => String::new(),
                            },
                        },
                    );
                }
            }
            "content_block_delta" => {
                let index = value_as_u32(payload.get("index"))?;
                match payload["delta"]["type"].as_str() {
                    Some("text_delta") => {
                        if let Some(text) = payload["delta"]["text"].as_str() {
                            out.push(AssistantEvent::TextDelta(text.to_string()));
                        }
                    }
                    Some("input_json_delta") => {
                        if let Some(tool_state) = self.tools.get_mut(&index) {
                            tool_state.input_json.push_str(
                                payload["delta"]["partial_json"]
                                    .as_str()
                                    .unwrap_or_default(),
                            );
                        }
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                let index = value_as_u32(payload.get("index"))?;
                if let Some(tool_state) = self.tools.remove(&index) {
                    out.push(tool_state.into_event()?);
                }
            }
            "message_delta" => {
                let usage = anthropic_usage(payload.get("usage"));
                self.stop_reason = Some(map_stop_reason(
                    payload
                        .get("delta")
                        .and_then(|delta| delta.get("stop_reason"))
                        .and_then(Value::as_str),
                ));
                out.push(AssistantEvent::Usage(usage));
                if let Some(event) = prompt_cache_event(&usage) {
                    out.push(AssistantEvent::PromptCache(event));
                }
            }
            "message_stop" => {
                self.stopped = true;
                out.push(AssistantEvent::MessageStop {
                    stop_reason: self
                        .stop_reason
                        .clone()
                        .unwrap_or(octopus_sdk_contracts::StopReason::EndTurn),
                });
            }
            _ => {}
        }

        Ok(out)
    }

    fn finish(&mut self) -> Result<Vec<AssistantEvent>, ModelError> {
        let mut out = self
            .tools
            .values()
            .cloned()
            .map(AnthropicToolState::into_event)
            .collect::<Result<Vec<_>, _>>()?;
        self.tools.clear();
        if !self.stopped {
            self.stopped = true;
            out.push(AssistantEvent::MessageStop {
                stop_reason: self
                    .stop_reason
                    .clone()
                    .unwrap_or(octopus_sdk_contracts::StopReason::EndTurn),
            });
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, Default)]
struct AnthropicToolState {
    id: String,
    name: String,
    input_json: String,
}

impl AnthropicToolState {
    fn into_event(self) -> Result<AssistantEvent, ModelError> {
        Ok(AssistantEvent::ToolUse {
            id: ToolCallId(self.id),
            name: self.name,
            input: parse_json_or_raw(&self.input_json),
        })
    }
}

fn anthropic_message(message: &octopus_sdk_contracts::Message) -> Result<Value, ModelError> {
    let role = match message.role {
        octopus_sdk_contracts::Role::Assistant => "assistant",
        _ => "user",
    };

    let content = message
        .content
        .iter()
        .map(|block| match block {
            octopus_sdk_contracts::ContentBlock::ToolUse { id, name, input } => json!({
                "type": "tool_use",
                "id": id.0,
                "name": name,
                "input": input,
            }),
            octopus_sdk_contracts::ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => json!({
                "type": "tool_result",
                "tool_use_id": tool_use_id.0,
                "content": tool_result_text(content),
                "is_error": is_error,
            }),
            octopus_sdk_contracts::ContentBlock::Text { text }
            | octopus_sdk_contracts::ContentBlock::Thinking { text } => json!({
                "type": "text",
                "text": text,
            }),
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "role": role,
        "content": content,
    }))
}

fn tool_result_text(content: &[octopus_sdk_contracts::ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            octopus_sdk_contracts::ContentBlock::Text { text }
            | octopus_sdk_contracts::ContentBlock::Thinking { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn anthropic_cache_control(cache_control: &crate::CacheControlStrategy) -> Value {
    match cache_control {
        crate::CacheControlStrategy::None => Value::Null,
        crate::CacheControlStrategy::PromptCaching { breakpoints } => json!({
            "type": "prompt_caching",
            "breakpoints": breakpoints,
        }),
        crate::CacheControlStrategy::ContextCacheObject { cache_id } => json!({
            "type": "context_cache_object",
            "cache_id": cache_id,
        }),
    }
}

fn parse_json_response(body: &[u8]) -> Result<Vec<AssistantEvent>, ModelError> {
    let payload: Value = serde_json::from_slice(body)?;
    let mut out = Vec::new();

    for block in payload
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    out.push(AssistantEvent::TextDelta(text.to_string()));
                }
            }
            Some("tool_use") => out.push(AssistantEvent::ToolUse {
                id: ToolCallId(
                    block
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                ),
                name: block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                input: block.get("input").cloned().unwrap_or(Value::Null),
            }),
            _ => {}
        }
    }

    let usage = anthropic_usage(payload.get("usage"));
    out.push(AssistantEvent::Usage(usage));
    if let Some(event) = prompt_cache_event(&usage) {
        out.push(AssistantEvent::PromptCache(event));
    }
    out.push(AssistantEvent::MessageStop {
        stop_reason: map_stop_reason(payload.get("stop_reason").and_then(Value::as_str)),
    });

    Ok(out)
}

fn anthropic_usage(value: Option<&Value>) -> Usage {
    Usage {
        input_tokens: value
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        output_tokens: value
            .and_then(|usage| usage.get("output_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        cache_creation_input_tokens: value
            .and_then(|usage| usage.get("cache_creation_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        cache_read_input_tokens: value
            .and_then(|usage| usage.get("cache_read_input_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
    }
}

fn parse_json_or_raw(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| json!({ "raw": value }))
}

fn starts_with_json(chunk: &[u8]) -> bool {
    chunk
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| matches!(byte, b'{' | b'['))
}

fn value_as_u32(value: Option<&Value>) -> Result<u32, ModelError> {
    value
        .and_then(Value::as_u64)
        .map(|value| value as u32)
        .ok_or_else(|| json_error("missing SSE index"))
}
