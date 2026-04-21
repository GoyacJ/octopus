use octopus_sdk_contracts::{ContentBlock, RenderBlock};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ToolResult {
    pub content: Vec<ContentBlock>,
    pub is_error: bool,
    pub duration_ms: u64,
    pub render: Option<RenderBlock>,
}

#[cfg(test)]
mod tests {
    use octopus_sdk_contracts::{ContentBlock, EventId, RenderBlock, RenderKind, RenderMeta};
    use serde_json::json;

    use super::ToolResult;

    #[test]
    fn tool_result_keeps_content_duration_and_render() {
        let result = ToolResult {
            content: vec![ContentBlock::Text {
                text: "done".into(),
            }],
            is_error: false,
            duration_ms: 42,
            render: Some(RenderBlock {
                kind: RenderKind::Record,
                payload: json!({ "ok": true }),
                meta: RenderMeta {
                    id: EventId("event-1".into()),
                    parent: None,
                    ts_ms: 42,
                },
            }),
        };

        assert_eq!(result.duration_ms, 42);
        assert!(!result.is_error);
        assert!(result.render.is_some());
    }
}
