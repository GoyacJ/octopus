use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::Utc;
use harness_contracts::{
    BlobError, BlobMeta, BlobRetention, BlobStore, ContextError, ContextStageId, Message,
    MessageId, MessagePart, ToolResult, ToolUseId,
};

use crate::{estimate_message_tokens, CompactHint, ContextBuffer, ContextOutcome, ContextProvider};

const TRUNCATED_CONTENT_TYPE: &str = "text/plain; charset=utf-8";

pub struct ToolResultBudgetProvider {
    per_tool_max_chars: u64,
    blob_offload: Arc<dyn BlobStore>,
}

impl ToolResultBudgetProvider {
    pub fn new(per_tool_max_chars: u64, blob_offload: Arc<dyn BlobStore>) -> Self {
        Self {
            per_tool_max_chars,
            blob_offload,
        }
    }
}

#[async_trait]
impl ContextProvider for ToolResultBudgetProvider {
    fn provider_id(&self) -> &'static str {
        "tool-result-budget"
    }

    fn stage(&self) -> ContextStageId {
        ContextStageId::ToolResultBudget
    }

    async fn apply(
        &self,
        ctx: &mut ContextBuffer,
        _hint: &CompactHint,
    ) -> Result<ContextOutcome, ContextError> {
        let mut bytes_saved = 0_u64;

        for message in &mut ctx.active.history {
            if ctx.bookkeeping.offloads.contains_key(&message.id) {
                continue;
            }

            for part in &mut message.parts {
                let MessagePart::ToolResult { content, .. } = part else {
                    continue;
                };
                let Some(text) = tool_result_text(content) else {
                    continue;
                };
                if text.chars().count() as u64 <= self.per_tool_max_chars {
                    continue;
                }

                let bytes = Bytes::from(text.clone());
                let meta = BlobMeta {
                    content_type: Some(TRUNCATED_CONTENT_TYPE.to_owned()),
                    size: bytes.len() as u64,
                    content_hash: *blake3::hash(&bytes).as_bytes(),
                    created_at: Utc::now(),
                    retention: BlobRetention::SessionScoped(ctx.identity.session_id),
                };
                let blob_ref = self
                    .blob_offload
                    .put(ctx.identity.tenant_id, bytes, meta)
                    .await
                    .map_err(offload_error)?;

                *content = ToolResult::Text(format!(
                    "[TOOL_RESULT_TRUNCATED: see blob {} size={}]",
                    blob_ref.id,
                    text.len()
                ));
                ctx.bookkeeping.offloads.insert(message.id, blob_ref);
                bytes_saved = bytes_saved.saturating_add(text.len() as u64);
            }
        }

        if bytes_saved == 0 {
            Ok(ContextOutcome::NoChange)
        } else {
            Ok(ContextOutcome::Modified { bytes_saved })
        }
    }
}

pub struct SnipProvider {
    protected_recent_n: usize,
}

impl SnipProvider {
    pub fn new(protected_recent_n: usize) -> Self {
        Self { protected_recent_n }
    }
}

#[async_trait]
impl ContextProvider for SnipProvider {
    fn provider_id(&self) -> &'static str {
        "snip"
    }

    fn stage(&self) -> ContextStageId {
        ContextStageId::Snip
    }

    async fn apply(
        &self,
        ctx: &mut ContextBuffer,
        hint: &CompactHint,
    ) -> Result<ContextOutcome, ContextError> {
        let Some(target_tokens) = hint.target_tokens else {
            return Ok(ContextOutcome::NoChange);
        };

        let mut estimated = effective_estimate(ctx, hint);
        if estimated <= target_tokens {
            return Ok(ContextOutcome::NoChange);
        }

        let mut dropped = HashSet::new();
        let mut bytes_saved = 0_u64;

        loop {
            let Some(drop_ids) = next_snip_group(ctx, self.protected_recent_n) else {
                break;
            };

            bytes_saved = bytes_saved.saturating_add(remove_messages(ctx, &drop_ids));
            dropped.extend(drop_ids);
            estimated = ctx
                .active
                .history
                .iter()
                .map(estimate_message_tokens)
                .sum::<u64>();

            if estimated <= target_tokens {
                break;
            }
        }

        if dropped.is_empty() {
            Ok(ContextOutcome::NoChange)
        } else {
            Ok(ContextOutcome::Modified { bytes_saved })
        }
    }
}

pub struct CollapseProvider {
    merge_threshold_chars: usize,
}

impl CollapseProvider {
    pub fn new(merge_threshold_chars: usize) -> Self {
        Self {
            merge_threshold_chars,
        }
    }
}

#[async_trait]
impl ContextProvider for CollapseProvider {
    fn provider_id(&self) -> &'static str {
        "collapse"
    }

    fn stage(&self) -> ContextStageId {
        ContextStageId::Collapse
    }

    async fn apply(
        &self,
        ctx: &mut ContextBuffer,
        _hint: &CompactHint,
    ) -> Result<ContextOutcome, ContextError> {
        let tool_names = tool_use_names(&ctx.active.history);
        let mut index = 0_usize;
        let mut bytes_saved = 0_u64;

        while index + 2 < ctx.active.history.len() {
            let first_result = first_tool_result(&ctx.active.history[index]);
            let between_tool_use = first_tool_use(&ctx.active.history[index + 1]);
            let second_result = first_tool_result(&ctx.active.history[index + 2]);

            let (
                Some((first_use, first_text)),
                Some((second_use, second_text)),
                Some((between_use, between_name)),
            ) = (first_result, second_result, between_tool_use)
            else {
                index += 1;
                continue;
            };

            if second_use != between_use {
                index += 1;
                continue;
            }

            if tool_names.get(&first_use) != Some(&between_name)
                || tool_names.get(&second_use) != Some(&between_name)
            {
                index += 1;
                continue;
            }

            if first_text.len().saturating_add(second_text.len()) > self.merge_threshold_chars {
                index += 1;
                continue;
            }

            let removed = ctx.active.history.remove(index + 2);
            let mut removed_parts = removed.parts;
            ctx.active.history[index].parts.append(&mut removed_parts);
            bytes_saved =
                bytes_saved.saturating_add(first_message_bytes(&ctx.active.history[index]));
            ctx.bookkeeping.offloads.remove(&removed.id);
        }

        if bytes_saved == 0 {
            Ok(ContextOutcome::NoChange)
        } else {
            Ok(ContextOutcome::Modified { bytes_saved })
        }
    }
}

fn effective_estimate(ctx: &ContextBuffer, hint: &CompactHint) -> u64 {
    if hint.estimated_tokens > 0 {
        hint.estimated_tokens
    } else {
        ctx.bookkeeping.estimated_tokens
    }
}

fn next_snip_group(ctx: &ContextBuffer, protected_recent_n: usize) -> Option<HashSet<MessageId>> {
    let protected_start = ctx.active.history.len().saturating_sub(protected_recent_n);
    let protected_ids = ctx.active.history[protected_start..]
        .iter()
        .map(|message| message.id)
        .collect::<HashSet<_>>();
    let pairs = crate::buffer::rebuild_tool_use_pairs(&ctx.active.history);

    for candidate in &ctx.active.history[..protected_start] {
        if let Some(pair) = pairs
            .iter()
            .find(|pair| pair.tool_use_message_id == candidate.id)
        {
            let Some(result_id) = pair.tool_result_message_id else {
                continue;
            };
            let group = HashSet::from([candidate.id, result_id]);
            if group.is_disjoint(&protected_ids) {
                return Some(group);
            }
            continue;
        }

        if pairs.iter().any(|pair| {
            pair.tool_result_message_id == Some(candidate.id)
                && pair.tool_use_message_id != candidate.id
        }) {
            let pair = pairs
                .iter()
                .find(|pair| pair.tool_result_message_id == Some(candidate.id))?;
            let group = HashSet::from([candidate.id, pair.tool_use_message_id]);
            if group.is_disjoint(&protected_ids) {
                return Some(group);
            }
            continue;
        }

        return Some(HashSet::from([candidate.id]));
    }

    None
}

fn remove_messages(ctx: &mut ContextBuffer, drop_ids: &HashSet<MessageId>) -> u64 {
    let mut bytes_saved = 0_u64;
    ctx.active.history.retain(|message| {
        if drop_ids.contains(&message.id) {
            bytes_saved = bytes_saved.saturating_add(message_bytes(message) as u64);
            false
        } else {
            true
        }
    });

    for id in drop_ids {
        ctx.bookkeeping.offloads.remove(id);
    }

    bytes_saved
}

fn tool_use_names(messages: &[Message]) -> HashMap<ToolUseId, String> {
    let mut names = HashMap::new();
    for message in messages {
        for part in &message.parts {
            if let MessagePart::ToolUse { id, name, .. } = part {
                names.insert(*id, name.clone());
            }
        }
    }
    names
}

fn first_tool_use(message: &Message) -> Option<(ToolUseId, String)> {
    message.parts.iter().find_map(|part| match part {
        MessagePart::ToolUse { id, name, .. } => Some((*id, name.clone())),
        _ => None,
    })
}

fn first_tool_result(message: &Message) -> Option<(ToolUseId, String)> {
    message.parts.iter().find_map(|part| match part {
        MessagePart::ToolResult {
            tool_use_id,
            content,
        } => tool_result_text(content).map(|text| (*tool_use_id, text)),
        _ => None,
    })
}

fn tool_result_text(result: &ToolResult) -> Option<String> {
    match result {
        ToolResult::Text(text) => Some(text.clone()),
        ToolResult::Structured(value) => Some(value.to_string()),
        ToolResult::Mixed(parts) => {
            let mut text = String::new();
            for part in parts {
                match part {
                    harness_contracts::ToolResultPart::Text { text: part_text } => {
                        text.push_str(part_text);
                    }
                    harness_contracts::ToolResultPart::Structured { value, .. } => {
                        text.push_str(&value.to_string());
                    }
                    harness_contracts::ToolResultPart::Code { text: code, .. } => {
                        text.push_str(code);
                    }
                    _ => {}
                }
            }
            Some(text)
        }
        _ => None,
    }
}

fn message_bytes(message: &Message) -> usize {
    message
        .parts
        .iter()
        .map(|part| format!("{part:?}").len())
        .sum()
}

fn first_message_bytes(message: &Message) -> u64 {
    message_bytes(message) as u64
}

fn offload_error(error: BlobError) -> ContextError {
    match error {
        BlobError::Backend(detail) => ContextError::OffloadFailed(detail),
        other => ContextError::OffloadFailed(other.to_string()),
    }
}
