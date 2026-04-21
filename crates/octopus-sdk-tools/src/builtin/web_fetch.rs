use std::time::{Duration, Instant};

use async_trait::async_trait;
use octopus_sdk_contracts::ContentBlock;
use regex::Regex;
use serde::Deserialize;
use serde_json::json;

use crate::{Tool, ToolCategory, ToolContext, ToolError, ToolResult, ToolSpec};

const FETCH_TIMEOUT_SECS: u64 = 30;
const MAX_FETCH_CHARS: usize = 30_000;
const TRUNCATION_HINT: &str = "\n\n[content truncated after 30000 chars]";

#[derive(Debug, Deserialize)]
struct WebFetchInput {
    url: String,
}

pub struct WebFetchTool {
    spec: ToolSpec,
}

impl WebFetchTool {
    #[must_use]
    pub fn new() -> Self {
        Self {
            spec: ToolSpec {
                name: "web_fetch".into(),
                description: "Fetch a URL and return readable text from the response body.".into(),
                input_schema: json!({
                    "type": "object",
                    "required": ["url"],
                    "properties": {
                        "url": { "type": "string", "format": "uri" }
                    }
                }),
                category: ToolCategory::Network,
            },
        }
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn spec(&self) -> &ToolSpec {
        &self.spec
    }

    fn is_concurrency_safe(&self, _input: &serde_json::Value) -> bool {
        true
    }

    async fn execute(
        &self,
        ctx: ToolContext,
        input: serde_json::Value,
    ) -> Result<ToolResult, ToolError> {
        let started = Instant::now();
        let input: WebFetchInput = serde_json::from_value(input)?;
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
            .build()?;

        let response = tokio::select! {
            () = ctx.cancellation.cancelled() => {
                return Err(ToolError::Cancelled { message: "web fetch was cancelled before completion".into() });
            }
            response = client.get(&input.url).send() => response?,
        }
        .error_for_status()?;

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response.text().await?;
        let text = truncate_text(
            &normalize_body(content_type.as_deref(), &body),
            MAX_FETCH_CHARS,
        );

        Ok(ToolResult {
            content: vec![ContentBlock::Text { text }],
            is_error: false,
            duration_ms: started.elapsed().as_millis() as u64,
            render: None,
        })
    }
}

fn normalize_body(content_type: Option<&str>, body: &str) -> String {
    if content_type.is_some_and(|value| value.contains("html")) || body.contains("<html") {
        return strip_html(body);
    }

    body.trim().to_string()
}

fn strip_html(body: &str) -> String {
    let without_script = Regex::new(r"(?is)<script[^>]*>.*?</script>")
        .expect("script regex should compile")
        .replace_all(body, " ");
    let without_style = Regex::new(r"(?is)<style[^>]*>.*?</style>")
        .expect("style regex should compile")
        .replace_all(&without_script, " ");
    let with_breaks =
        Regex::new(r"(?i)</?(p|div|section|article|main|header|footer|li|ul|ol|br|h[1-6])[^>]*>")
            .expect("block regex should compile")
            .replace_all(&without_style, "\n");
    let without_tags = Regex::new(r"(?is)<[^>]+>")
        .expect("tag regex should compile")
        .replace_all(&with_breaks, " ");
    let decoded = without_tags
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    decoded
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_text(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }

    let mut end = 0_usize;
    for (count, (index, ch)) in text.char_indices().enumerate() {
        if count == limit {
            break;
        }
        end = index + ch.len_utf8();
    }

    format!("{}{}", &text[..end], TRUNCATION_HINT)
}
