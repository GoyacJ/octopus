use std::collections::HashSet;
use std::sync::Arc;

use futures::StreamExt;
use harness_contracts::{Message, SessionId, TenantId, UsageSnapshot};
use harness_journal::{
    EventStore, EventStream, Projection, ReplayCursor,
    SessionProjection as JournalSessionProjection,
};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::ObservabilityError;

#[derive(Clone)]
pub struct ReplayEngine {
    store: Arc<dyn EventStore>,
}

impl ReplayEngine {
    #[must_use]
    pub fn new(store: Arc<dyn EventStore>) -> Self {
        Self { store }
    }

    pub async fn replay(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        cursor: ReplayCursor,
    ) -> Result<EventStream, ObservabilityError> {
        Ok(self.store.read(tenant, session_id, cursor).await?)
    }

    pub async fn reconstruct_projection(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        cursor: ReplayCursor,
    ) -> Result<JournalSessionProjection, ObservabilityError> {
        let envelopes = self
            .store
            .read_envelopes(tenant, session_id, cursor)
            .await?
            .collect::<Vec<_>>()
            .await;
        let last_offset = envelopes.last().map(|envelope| envelope.offset);
        let events = envelopes
            .iter()
            .map(|envelope| &envelope.payload)
            .collect::<Vec<_>>();
        let mut projection = JournalSessionProjection::replay(events)?;
        if let Some(last_offset) = last_offset {
            projection.last_offset = last_offset;
        }
        Ok(projection)
    }

    pub async fn diff(
        &self,
        tenant: TenantId,
        session_a: SessionId,
        session_b: SessionId,
    ) -> Result<SessionDiff, ObservabilityError> {
        let a = self
            .reconstruct_projection(tenant, session_a, ReplayCursor::FromStart)
            .await?;
        let b = self
            .reconstruct_projection(tenant, session_b, ReplayCursor::FromStart)
            .await?;

        Ok(SessionDiff::between(&a, &b))
    }

    pub async fn export_session<W>(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        format: ExportFormat,
        mut out: W,
    ) -> Result<(), ObservabilityError>
    where
        W: AsyncWrite + Unpin,
    {
        match format {
            ExportFormat::Json => {
                let projection = self
                    .reconstruct_projection(tenant, session_id, ReplayCursor::FromStart)
                    .await?;
                write_json(&mut out, &SessionExport::from_projection(&projection)).await?;
            }
            ExportFormat::JsonLines => {
                let mut events = self
                    .store
                    .read(tenant, session_id, ReplayCursor::FromStart)
                    .await?;
                while let Some(event) = events.next().await {
                    write_json_line(&mut out, &event).await?;
                }
            }
            ExportFormat::Markdown => {
                let projection = self
                    .reconstruct_projection(tenant, session_id, ReplayCursor::FromStart)
                    .await?;
                write_markdown(&mut out, &projection.messages).await?;
            }
            ExportFormat::Har => {
                return Err(ObservabilityError::Replay(
                    "HAR export requires HTTP exchange projection and is not available yet"
                        .to_owned(),
                ));
            }
        }
        out.flush()
            .await
            .map_err(|error| ObservabilityError::Exporter(error.to_string()))
    }
}

#[derive(Serialize)]
struct SessionExport<'a> {
    messages: &'a [Message],
    usage: &'a UsageSnapshot,
    end_reason: &'a Option<harness_contracts::EndReason>,
    last_offset: harness_contracts::JournalOffset,
}

impl<'a> SessionExport<'a> {
    fn from_projection(projection: &'a JournalSessionProjection) -> Self {
        Self {
            messages: &projection.messages,
            usage: &projection.usage,
            end_reason: &projection.end_reason,
            last_offset: projection.last_offset,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionDiff {
    pub added_messages: Vec<Message>,
    pub removed_messages: Vec<Message>,
    pub tool_divergence: Vec<ToolDivergence>,
    pub usage_delta: UsageSnapshot,
}

impl SessionDiff {
    fn between(a: &JournalSessionProjection, b: &JournalSessionProjection) -> Self {
        let a_ids = a
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<HashSet<_>>();
        let b_ids = b
            .messages
            .iter()
            .map(|message| message.id)
            .collect::<HashSet<_>>();
        let added_messages = b
            .messages
            .iter()
            .filter(|message| !a_ids.contains(&message.id))
            .cloned()
            .collect();
        let removed_messages = a
            .messages
            .iter()
            .filter(|message| !b_ids.contains(&message.id))
            .cloned()
            .collect();

        Self {
            added_messages,
            removed_messages,
            tool_divergence: Vec::new(),
            usage_delta: usage_delta(&a.usage, &b.usage),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolDivergence {
    pub tool_use_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    JsonLines,
    Markdown,
    Har,
}

fn usage_delta(a: &UsageSnapshot, b: &UsageSnapshot) -> UsageSnapshot {
    UsageSnapshot {
        input_tokens: b.input_tokens.saturating_sub(a.input_tokens),
        output_tokens: b.output_tokens.saturating_sub(a.output_tokens),
        cache_read_tokens: b.cache_read_tokens.saturating_sub(a.cache_read_tokens),
        cache_write_tokens: b.cache_write_tokens.saturating_sub(a.cache_write_tokens),
        cost_micros: b.cost_micros.saturating_sub(a.cost_micros),
    }
}

async fn write_json<W, T>(out: &mut W, value: &T) -> Result<(), ObservabilityError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| ObservabilityError::Replay(error.to_string()))?;
    out.write_all(&bytes)
        .await
        .map_err(|error| ObservabilityError::Exporter(error.to_string()))
}

async fn write_json_line<W, T>(out: &mut W, value: &T) -> Result<(), ObservabilityError>
where
    W: AsyncWrite + Unpin,
    T: Serialize,
{
    let bytes =
        serde_json::to_vec(value).map_err(|error| ObservabilityError::Replay(error.to_string()))?;
    out.write_all(&bytes)
        .await
        .map_err(|error| ObservabilityError::Exporter(error.to_string()))?;
    out.write_all(b"\n")
        .await
        .map_err(|error| ObservabilityError::Exporter(error.to_string()))
}

async fn write_markdown<W>(out: &mut W, messages: &[Message]) -> Result<(), ObservabilityError>
where
    W: AsyncWrite + Unpin,
{
    for message in messages {
        out.write_all(format!("## {:?}\n\n", message.role).as_bytes())
            .await
            .map_err(|error| ObservabilityError::Exporter(error.to_string()))?;
        for part in &message.parts {
            out.write_all(format!("{part:?}\n\n").as_bytes())
                .await
                .map_err(|error| ObservabilityError::Exporter(error.to_string()))?;
        }
    }
    Ok(())
}
