use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::EventId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderMeta {
    pub id: EventId,
    pub parent: Option<EventId>,
    pub ts_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderKind {
    Text,
    Markdown,
    Code,
    Diff,
    ListSummary,
    Progress,
    ArtifactRef,
    Record,
    Error,
    Raw,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderBlock {
    pub kind: RenderKind,
    pub payload: Value,
    pub meta: RenderMeta,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderLifecycle {
    OnToolUse,
    OnToolProgress,
    OnToolResult,
    OnToolRejected,
    OnToolError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskPrompt {
    pub kind: String,
    pub questions: Vec<AskQuestion>,
}

impl Default for AskPrompt {
    fn default() -> Self {
        Self {
            kind: "ask-user".into(),
            questions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskQuestion {
    pub id: String,
    pub question: String,
    pub header: String,
    pub multi_select: bool,
    pub options: Vec<AskOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskOption {
    pub id: String,
    pub label: String,
    pub description: String,
    pub preview: Option<String>,
    pub preview_format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Markdown,
    Code,
    Html,
    Svg,
    Mermaid,
    React,
    Json,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactStatus {
    Draft,
    Review,
    Approved,
    Published,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    pub kind: String,
    pub artifact_id: String,
    pub artifact_kind: ArtifactKind,
    pub title: Option<String>,
    pub preview: Option<String>,
    pub preview_format: Option<String>,
    pub version: Option<u32>,
    pub parent_version: Option<u32>,
    pub status: Option<ArtifactStatus>,
    pub content_type: Option<String>,
    pub superseded_by_version: Option<u32>,
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::{
        AskOption, AskPrompt, AskQuestion, ArtifactKind, ArtifactRef, ArtifactStatus, RenderBlock,
        RenderKind, RenderLifecycle, RenderMeta,
    };
    use crate::EventId;

    #[test]
    fn render_lifecycle_uses_hook_phase_names() {
        let value =
            serde_json::to_value(RenderLifecycle::OnToolProgress).expect("lifecycle should serialize");

        assert_eq!(value, Value::String("on_tool_progress".into()));
    }

    #[test]
    fn ask_prompt_serializes_questions_and_options() {
        let prompt = AskPrompt {
            kind: "ask-user".into(),
            questions: vec![AskQuestion {
                id: "q-1".into(),
                question: "Choose one path?".into(),
                header: "Path".into(),
                multi_select: false,
                options: vec![
                    AskOption {
                        id: "opt-a".into(),
                        label: "A".into(),
                        description: "Take path A".into(),
                        preview: Some("Preview A".into()),
                        preview_format: Some("markdown".into()),
                    },
                    AskOption {
                        id: "opt-b".into(),
                        label: "B".into(),
                        description: "Take path B".into(),
                        preview: None,
                        preview_format: None,
                    },
                ],
            }],
        };

        let value = serde_json::to_value(&prompt).expect("ask prompt should serialize");

        assert_eq!(value.get("kind"), Some(&Value::String("ask-user".into())));
        assert_eq!(value["questions"][0]["options"][0]["previewFormat"], "markdown");
    }

    #[test]
    fn artifact_ref_serializes_all_contract_fields() {
        let artifact_ref = ArtifactRef {
            kind: "artifact-ref".into(),
            artifact_id: "artifact-1".into(),
            artifact_kind: ArtifactKind::Markdown,
            title: Some("Sprint Summary".into()),
            preview: Some("Weekly notes".into()),
            preview_format: Some("markdown".into()),
            version: Some(3),
            parent_version: Some(2),
            status: Some(ArtifactStatus::Review),
            content_type: Some("text/markdown".into()),
            superseded_by_version: Some(4),
        };

        let value = serde_json::to_value(&artifact_ref).expect("artifact ref should serialize");

        assert_eq!(value["artifactKind"], "markdown");
        assert_eq!(value["status"], "review");
        assert_eq!(value["supersededByVersion"], 4);
    }

    #[test]
    fn render_block_keeps_payload_and_meta_contract() {
        let block = RenderBlock {
            kind: RenderKind::Record,
            payload: json!({
                "rows": [{ "label": "key", "value": "value" }],
                "title": "Record"
            }),
            meta: RenderMeta {
                id: EventId("event-1".into()),
                parent: Some(EventId("event-0".into())),
                ts_ms: 1_713_692_800_123,
            },
        };

        let value = serde_json::to_value(&block).expect("render block should serialize");

        assert_eq!(value["kind"], "record");
        assert_eq!(value["payload"]["title"], "Record");
        assert_eq!(value["meta"]["id"], "event-1");
    }
}
