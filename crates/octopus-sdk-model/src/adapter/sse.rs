use crate::ModelError;

use super::json_error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SseEvent {
    pub(crate) event: Option<String>,
    pub(crate) data: String,
}

#[derive(Debug, Default)]
pub(crate) struct IncrementalSseParser {
    buffer: String,
}

impl IncrementalSseParser {
    pub(crate) fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseEvent>, ModelError> {
        let decoded = std::str::from_utf8(chunk)
            .map_err(|_| json_error("invalid UTF-8 in SSE stream"))?
            .replace("\r\n", "\n");
        self.buffer.push_str(&decoded);
        self.drain_complete_frames()
    }

    pub(crate) fn finish(&mut self) -> Result<Vec<SseEvent>, ModelError> {
        let mut events = self.drain_complete_frames()?;
        if !self.buffer.trim().is_empty() {
            let frame = std::mem::take(&mut self.buffer);
            if let Some(event) = parse_frame(&frame) {
                events.push(event);
            }
        }
        Ok(events)
    }

    fn drain_complete_frames(&mut self) -> Result<Vec<SseEvent>, ModelError> {
        let mut events = Vec::new();
        while let Some(end) = self.buffer.find("\n\n") {
            let frame = self.buffer[..end].to_string();
            self.buffer.drain(..end + 2);
            if let Some(event) = parse_frame(&frame) {
                events.push(event);
            }
        }
        Ok(events)
    }
}

fn parse_frame(frame: &str) -> Option<SseEvent> {
    let mut event = None;
    let mut data_lines = Vec::new();

    for raw_line in frame.lines() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        if let Some(value) = line.strip_prefix("event:") {
            event = Some(value.trim().to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("data:") {
            data_lines.push(value.trim_start().to_string());
        }
    }

    if event.is_none() && data_lines.is_empty() {
        return None;
    }

    Some(SseEvent {
        event,
        data: data_lines.join("\n"),
    })
}

#[cfg(test)]
mod tests {
    use super::{IncrementalSseParser, SseEvent};

    #[test]
    fn parses_multiple_events_from_one_buffer() {
        let mut parser = IncrementalSseParser::default();
        let events = parser
            .push(
                b"event: message_start\ndata: {\"id\":\"msg_1\"}\n\nevent: message_stop\ndata: {}\n\n",
            )
            .expect("parser should accept complete frames");

        assert_eq!(
            events,
            vec![
                SseEvent {
                    event: Some("message_start".to_string()),
                    data: "{\"id\":\"msg_1\"}".to_string(),
                },
                SseEvent {
                    event: Some("message_stop".to_string()),
                    data: "{}".to_string(),
                },
            ]
        );
    }

    #[test]
    fn handles_frames_split_across_chunks() {
        let mut parser = IncrementalSseParser::default();

        assert!(parser
            .push(b"event: content_block_delta\ndata: {\"delta\":\"hel")
            .expect("parser should buffer partial frames")
            .is_empty());

        let events = parser
            .push(b"lo\"}\n\n")
            .expect("parser should flush completed frame");

        assert_eq!(
            events,
            vec![SseEvent {
                event: Some("content_block_delta".to_string()),
                data: "{\"delta\":\"hello\"}".to_string(),
            }]
        );
    }

    #[test]
    fn preserves_done_sentinel_as_regular_event_data() {
        let mut parser = IncrementalSseParser::default();
        let events = parser
            .push(b"data: [DONE]\n\n")
            .expect("done sentinel should still parse");

        assert_eq!(
            events,
            vec![SseEvent {
                event: None,
                data: "[DONE]".to_string(),
            }]
        );
    }
}
