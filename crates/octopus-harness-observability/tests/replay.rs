#![cfg(feature = "replay")]

use std::sync::Arc;
use std::task::{Context, Poll};

use futures::StreamExt;
use harness_contracts::{
    AssistantMessageCompletedEvent, ConfigHash, EndReason, Event, MessageContent, MessageId,
    MessageMetadata, NoopRedactor, RunEndedEvent, RunId, SessionCreatedEvent, SessionEndedEvent,
    SessionId, SnapshotId, StopReason, TenantId, UsageSnapshot, UserMessageAppendedEvent,
};
use harness_journal::{
    EventStore, InMemoryEventStore, Projection, ReplayCursor, SessionProjection,
};
use harness_observability::{ExportFormat, ReplayEngine};
use tokio::io::AsyncWrite;

#[tokio::test]
async fn replay_stream_respects_event_store_cursor() {
    let tenant = TenantId::SINGLE;
    let session = SessionId::new();
    let store = event_store();
    let events = session_events(session, tenant, "hello", "world", usage(3, 5));
    store.append(tenant, session, &events).await.unwrap();
    let engine = ReplayEngine::new(store);

    let replayed = engine
        .replay(
            tenant,
            session,
            ReplayCursor::FromOffset(harness_contracts::JournalOffset(1)),
        )
        .await
        .unwrap()
        .collect::<Vec<_>>()
        .await;

    assert_eq!(replayed.len(), events.len() - 2);
    assert!(matches!(replayed[0], Event::AssistantMessageCompleted(_)));
}

#[tokio::test]
async fn reconstruct_projection_matches_journal_projection() {
    let tenant = TenantId::SINGLE;
    let session = SessionId::new();
    let store = event_store();
    let events = session_events(session, tenant, "list files", "Cargo.toml", usage(10, 4));
    store.append(tenant, session, &events).await.unwrap();
    let expected = SessionProjection::replay(events.iter()).unwrap();
    let engine = ReplayEngine::new(store);

    let projection = engine
        .reconstruct_projection(tenant, session, ReplayCursor::FromStart)
        .await
        .unwrap();

    assert_eq!(projection.messages, expected.messages);
    assert_eq!(projection.usage, expected.usage);
    assert_eq!(projection.end_reason, Some(EndReason::Completed));
    assert_eq!(projection.last_offset.0, 4);
}

#[tokio::test]
async fn diff_reports_added_messages_and_usage_delta() {
    let tenant = TenantId::SINGLE;
    let first = SessionId::new();
    let second = SessionId::new();
    let store = event_store();
    store
        .append(
            tenant,
            first,
            &session_events(first, tenant, "same", "short", usage(1, 2)),
        )
        .await
        .unwrap();
    store
        .append(
            tenant,
            second,
            &session_events(second, tenant, "same", "longer", usage(3, 8)),
        )
        .await
        .unwrap();
    let engine = ReplayEngine::new(store);

    let diff = engine.diff(tenant, first, second).await.unwrap();

    assert_eq!(diff.added_messages.len(), 2);
    assert_eq!(diff.removed_messages.len(), 2);
    assert_eq!(diff.usage_delta.input_tokens, 2);
    assert_eq!(diff.usage_delta.output_tokens, 6);
    assert!(diff.tool_divergence.is_empty());
}

#[tokio::test]
async fn export_session_writes_json_lines_and_markdown() {
    let tenant = TenantId::SINGLE;
    let session = SessionId::new();
    let store = event_store();
    store
        .append(
            tenant,
            session,
            &session_events(session, tenant, "hi", "there", usage(1, 1)),
        )
        .await
        .unwrap();
    let engine = ReplayEngine::new(store);
    let mut jsonl = MemoryWriter::default();
    engine
        .export_session(tenant, session, ExportFormat::JsonLines, &mut jsonl)
        .await
        .unwrap();
    let jsonl = jsonl.into_string();
    assert_eq!(jsonl.lines().count(), 5);
    assert!(jsonl.contains("assistant_message_completed"));

    let mut markdown = MemoryWriter::default();
    engine
        .export_session(tenant, session, ExportFormat::Markdown, &mut markdown)
        .await
        .unwrap();
    let markdown = markdown.into_string();
    assert!(markdown.contains("## User"));
    assert!(markdown.contains("## Assistant"));
}

fn event_store() -> Arc<InMemoryEventStore> {
    Arc::new(InMemoryEventStore::new(Arc::new(NoopRedactor)))
}

fn session_events(
    session: SessionId,
    tenant: TenantId,
    user: &str,
    assistant: &str,
    usage: UsageSnapshot,
) -> Vec<Event> {
    let run = RunId::new();
    vec![
        Event::SessionCreated(SessionCreatedEvent {
            session_id: session,
            tenant_id: tenant,
            options_hash: [0; 32],
            snapshot_id: SnapshotId::from_u128(1),
            effective_config_hash: ConfigHash([1; 32]),
            created_at: harness_contracts::now(),
        }),
        Event::UserMessageAppended(UserMessageAppendedEvent {
            run_id: run,
            message_id: MessageId::new(),
            content: MessageContent::Text(user.to_owned()),
            metadata: MessageMetadata::default(),
            at: harness_contracts::now(),
        }),
        Event::AssistantMessageCompleted(AssistantMessageCompletedEvent {
            run_id: run,
            message_id: MessageId::new(),
            content: MessageContent::Text(assistant.to_owned()),
            tool_uses: Vec::new(),
            usage: usage.clone(),
            pricing_snapshot_id: None,
            stop_reason: StopReason::EndTurn,
            at: harness_contracts::now(),
        }),
        Event::RunEnded(RunEndedEvent {
            run_id: run,
            reason: EndReason::Completed,
            usage: None,
            ended_at: harness_contracts::now(),
        }),
        Event::SessionEnded(SessionEndedEvent {
            session_id: session,
            tenant_id: tenant,
            reason: EndReason::Completed,
            final_usage: usage,
            at: harness_contracts::now(),
        }),
    ]
}

fn usage(input_tokens: u64, output_tokens: u64) -> UsageSnapshot {
    UsageSnapshot {
        input_tokens,
        output_tokens,
        cache_read_tokens: 0,
        cache_write_tokens: 0,
        cost_micros: 0,
    }
}

#[derive(Default)]
struct MemoryWriter {
    bytes: Vec<u8>,
}

impl MemoryWriter {
    fn into_string(self) -> String {
        String::from_utf8(self.bytes).unwrap()
    }
}

impl AsyncWrite for MemoryWriter {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.bytes.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
