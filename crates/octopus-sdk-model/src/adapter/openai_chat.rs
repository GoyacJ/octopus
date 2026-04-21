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
    json_error, map_stop_reason,
    sse::{IncrementalSseParser, SseEvent},
    StreamBytes,
};

#[derive(Debug, Default)]
pub struct OpenAiChatAdapter;

#[async_trait]
impl ProtocolAdapter for OpenAiChatAdapter {
    fn family(&self) -> ProtocolFamily {
        ProtocolFamily::OpenAiChat
    }

    fn to_request(&self, req: &ModelRequest) -> Result<Value, ModelError> {
        Ok(json!({
            "model": req.model.0,
            "messages": openai_messages(req),
            "tools": req.tools.iter().map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.input_schema,
                    }
                })
            }).collect::<Vec<_>>(),
            "tool_choice": if req.tools.is_empty() { Value::Null } else { json!("auto") },
            "stream": req.stream,
            "stream_options": if req.stream { json!({"include_usage": true}) } else { Value::Null },
            "max_tokens": req.max_tokens,
            "temperature": req.temperature,
            "response_format": req.response_format,
        }))
    }

    fn parse_stream(&self, raw: StreamBytes) -> Result<ModelStream, ModelError> {
        Ok(Box::pin(try_stream! {
            let mut raw = raw;
            let mut sse = IncrementalSseParser::default();
            let mut format = None;
            let mut json_body = Vec::new();
            let mut state = OpenAiStreamState::default();

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
        let secret = super::secret_to_string(secret)?;

        Ok(vec![(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str(&format!("Bearer {secret}"))
                .map_err(|_| json_error("invalid bearer token"))?,
        )])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WireFormat {
    Json,
    Sse,
}

#[derive(Debug, Default)]
struct OpenAiStreamState {
    tools: BTreeMap<u32, OpenAiToolState>,
    stop_reason: Option<octopus_sdk_contracts::StopReason>,
    stopped: bool,
}

impl OpenAiStreamState {
    fn ingest(&mut self, event: SseEvent) -> Result<Vec<AssistantEvent>, ModelError> {
        if event.data == "[DONE]" {
            return self.finish();
        }

        let payload: Value = serde_json::from_str(&event.data)?;
        let mut out = Vec::new();

        if let Some(usage) = payload.get("usage").filter(|usage| usage.is_object()) {
            out.push(AssistantEvent::Usage(openai_usage(Some(usage))));
        }

        for choice in payload
            .get("choices")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
        {
            if let Some(text) = choice
                .get("delta")
                .and_then(|delta| delta.get("content"))
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
            {
                out.push(AssistantEvent::TextDelta(text.to_string()));
            }

            for tool_call in choice
                .get("delta")
                .and_then(|delta| delta.get("tool_calls"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
            {
                let index = value_as_u32(tool_call.get("index"))?;
                let state = self.tools.entry(index).or_default();
                state.index = index;
                if let Some(id) = tool_call.get("id").and_then(Value::as_str) {
                    state.id = id.to_string();
                }
                if let Some(name) = tool_call
                    .get("function")
                    .and_then(|function| function.get("name"))
                    .and_then(Value::as_str)
                {
                    state.name = name.to_string();
                }
                if let Some(arguments) = tool_call
                    .get("function")
                    .and_then(|function| function.get("arguments"))
                    .and_then(Value::as_str)
                {
                    state.arguments.push_str(arguments);
                }
            }

            if let Some(reason) =
                choice
                    .get("finish_reason")
                    .and_then(Value::as_str)
                    .or_else(|| {
                        choice
                            .get("delta")
                            .and_then(|delta| delta.get("finish_reason"))
                            .and_then(Value::as_str)
                    })
            {
                self.stop_reason = Some(map_stop_reason(Some(reason)));
                if reason == "tool_calls" {
                    out.extend(self.take_tool_events()?);
                }
            }
        }

        Ok(out)
    }

    fn finish(&mut self) -> Result<Vec<AssistantEvent>, ModelError> {
        let mut out = self.take_tool_events()?;
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

    fn take_tool_events(&mut self) -> Result<Vec<AssistantEvent>, ModelError> {
        let mut keys = self.tools.keys().copied().collect::<Vec<_>>();
        keys.sort_unstable();
        let mut out = Vec::new();
        for key in keys {
            if let Some(state) = self.tools.remove(&key) {
                out.push(state.into_event()?);
            }
        }
        Ok(out)
    }
}

#[derive(Debug, Default)]
struct OpenAiToolState {
    index: u32,
    id: String,
    name: String,
    arguments: String,
}

impl OpenAiToolState {
    fn into_event(self) -> Result<AssistantEvent, ModelError> {
        Ok(AssistantEvent::ToolUse {
            id: ToolCallId(if self.id.is_empty() {
                format!("tool_call_{}", self.index)
            } else {
                self.id
            }),
            name: self.name,
            input: parse_json_or_raw(&self.arguments),
        })
    }
}

fn openai_messages(req: &ModelRequest) -> Vec<Value> {
    let mut messages = req
        .system_prompt
        .iter()
        .map(|system| {
            json!({
                "role": "system",
                "content": system,
            })
        })
        .collect::<Vec<_>>();

    for message in &req.messages {
        match message.role {
            octopus_sdk_contracts::Role::Assistant => {
                let mut content = String::new();
                let mut tool_calls = Vec::new();
                for block in &message.content {
                    match block {
                        octopus_sdk_contracts::ContentBlock::Text { text }
                        | octopus_sdk_contracts::ContentBlock::Thinking { text } => {
                            content.push_str(text);
                        }
                        octopus_sdk_contracts::ContentBlock::ToolUse { id, name, input } => {
                            tool_calls.push(json!({
                                "id": id.0,
                                "type": "function",
                                "function": {
                                    "name": name,
                                    "arguments": input.to_string(),
                                }
                            }));
                        }
                        octopus_sdk_contracts::ContentBlock::ToolResult { .. } => {}
                    }
                }

                messages.push(json!({
                    "role": "assistant",
                    "content": (!content.is_empty()).then_some(content),
                    "tool_calls": tool_calls,
                }));
            }
            octopus_sdk_contracts::Role::Tool => {
                for block in &message.content {
                    if let octopus_sdk_contracts::ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } = block
                    {
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tool_use_id.0,
                            "content": tool_result_text(content),
                        }));
                    }
                }
            }
            _ => {
                for block in &message.content {
                    if let octopus_sdk_contracts::ContentBlock::Text { text }
                    | octopus_sdk_contracts::ContentBlock::Thinking { text } = block
                    {
                        messages.push(json!({
                            "role": "user",
                            "content": text,
                        }));
                    }
                }
            }
        }
    }

    messages
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

fn parse_json_response(body: &[u8]) -> Result<Vec<AssistantEvent>, ModelError> {
    let payload: Value = serde_json::from_slice(body)?;
    let mut out = Vec::new();

    let choice = payload
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .cloned()
        .ok_or_else(|| json_error("openai response missing choices"))?;

    if let Some(text) = choice
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    {
        out.push(AssistantEvent::TextDelta(text.to_string()));
    }

    for tool_call in choice
        .get("message")
        .and_then(|message| message.get("tool_calls"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        out.push(AssistantEvent::ToolUse {
            id: ToolCallId(
                tool_call
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
            ),
            name: tool_call
                .get("function")
                .and_then(|function| function.get("name"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            input: parse_json_or_raw(
                tool_call
                    .get("function")
                    .and_then(|function| function.get("arguments"))
                    .and_then(Value::as_str)
                    .unwrap_or("{}"),
            ),
        });
    }

    out.push(AssistantEvent::Usage(openai_usage(payload.get("usage"))));
    out.push(AssistantEvent::MessageStop {
        stop_reason: map_stop_reason(choice.get("finish_reason").and_then(Value::as_str)),
    });
    Ok(out)
}

fn openai_usage(value: Option<&Value>) -> Usage {
    Usage {
        input_tokens: value
            .and_then(|usage| usage.get("prompt_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        output_tokens: value
            .and_then(|usage| usage.get("completion_tokens"))
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
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
        .ok_or_else(|| json_error("missing tool call index"))
}
