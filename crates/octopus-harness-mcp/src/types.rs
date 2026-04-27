use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use harness_contracts::{McpServerId, McpServerSource, SessionId, TrustLevel};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct McpServerSpec {
    pub server_id: McpServerId,
    pub display_name: String,
    pub transport: TransportChoice,
    pub auth: McpClientAuth,
    pub capabilities_expected: McpExpectedCapabilities,
    pub source: McpServerSource,
    pub trust: TrustLevel,
    pub timeouts: McpTimeouts,
    pub reconnect: ReconnectPolicy,
    pub tool_filter: McpToolFilter,
    pub sampling: SamplingPolicy,
}

impl McpServerSpec {
    pub fn new(
        server_id: McpServerId,
        display_name: impl Into<String>,
        transport: TransportChoice,
        source: McpServerSource,
    ) -> Self {
        let trust = trust_level_for_source(&source);
        Self {
            server_id,
            display_name: display_name.into(),
            transport,
            auth: McpClientAuth::None,
            capabilities_expected: McpExpectedCapabilities::default(),
            source,
            trust,
            timeouts: McpTimeouts::default(),
            reconnect: ReconnectPolicy::default(),
            tool_filter: McpToolFilter::default(),
            sampling: SamplingPolicy::denied(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum TransportChoice {
    Stdio {
        command: String,
        args: Vec<String>,
        env: StdioEnv,
        policy: StdioPolicy,
    },
    Http {
        url: String,
        headers: BTreeMap<String, String>,
    },
    WebSocket {
        url: String,
        headers: BTreeMap<String, String>,
    },
    Sse {
        url: String,
        headers: BTreeMap<String, String>,
    },
    InProcess,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpClientAuth {
    None,
    Bearer(String),
    OAuth {
        authorize_url: String,
        token_url: String,
        client_id: String,
        client_secret: String,
        scopes: Vec<String>,
    },
    Xaa {
        parent_session: SessionId,
        scopes: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpExpectedCapabilities {
    pub tools: bool,
    pub resources: bool,
    pub prompts: bool,
    pub sampling: bool,
}

impl Default for McpExpectedCapabilities {
    fn default() -> Self {
        Self {
            tools: true,
            resources: false,
            prompts: false,
            sampling: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpTimeouts {
    pub handshake: Duration,
    pub call_default: Duration,
    pub sampling: Duration,
    pub idle: Duration,
}

impl Default for McpTimeouts {
    fn default() -> Self {
        Self {
            handshake: Duration::from_secs(5),
            call_default: Duration::from_secs(30),
            sampling: Duration::from_secs(60),
            idle: Duration::from_secs(300),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReconnectPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_jitter: f32,
    pub success_reset_after: Duration,
    pub keep_deferred_during_reconnect: bool,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 0,
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
            backoff_jitter: 0.2,
            success_reset_after: Duration::from_secs(300),
            keep_deferred_during_reconnect: true,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StdioEnv {
    Allowlist {
        inherit: BTreeSet<String>,
        extra: BTreeMap<String, String>,
    },
    InheritWithDeny {
        deny: BTreeSet<String>,
        extra: BTreeMap<String, String>,
    },
    Empty {
        extra: BTreeMap<String, String>,
    },
}

impl StdioEnv {
    pub fn default_deny_envs() -> BTreeSet<String> {
        [
            "OPENAI_API_KEY",
            "OPENAI_ORG",
            "ANTHROPIC_API_KEY",
            "GOOGLE_API_KEY",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_SESSION_TOKEN",
            "AZURE_OPENAI_KEY",
            "AZURE_CLIENT_SECRET",
            "GOOGLE_APPLICATION_CREDENTIALS",
            "KUBECONFIG",
            "KUBE_TOKEN",
            "GITHUB_TOKEN",
            "GITLAB_TOKEN",
            "DOCKER_AUTH_CONFIG",
            "NPM_TOKEN",
            "CARGO_REGISTRY_TOKEN",
            "HARNESS_*",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }
}

impl Default for StdioEnv {
    fn default() -> Self {
        Self::InheritWithDeny {
            deny: Self::default_deny_envs(),
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdioPolicy {
    pub stderr_line_max_bytes: u32,
    pub redact_stderr: bool,
    pub graceful_kill_after: Duration,
    pub working_dir: Option<std::path::PathBuf>,
}

impl Default for StdioPolicy {
    fn default() -> Self {
        Self {
            stderr_line_max_bytes: 4096,
            redact_stderr: true,
            graceful_kill_after: Duration::from_secs(5),
            working_dir: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolFilter {
    pub allow: Vec<McpToolGlob>,
    pub deny: Vec<McpToolGlob>,
    pub on_conflict: FilterConflict,
}

impl Default for McpToolFilter {
    fn default() -> Self {
        Self {
            allow: Vec::new(),
            deny: Vec::new(),
            on_conflict: FilterConflict::DenyWins,
        }
    }
}

impl McpToolFilter {
    pub fn evaluate(&self, canonical_name: &str) -> FilterDecision {
        let allow_match = self.allow.iter().any(|glob| glob.matches(canonical_name));
        let deny_match = self.deny.iter().any(|glob| glob.matches(canonical_name));

        if allow_match && deny_match {
            return match self.on_conflict {
                FilterConflict::DenyWins => FilterDecision::Skip {
                    reason: "allow and deny matched; deny wins".to_owned(),
                },
                FilterConflict::AllowWins => FilterDecision::Inject,
                FilterConflict::Reject => FilterDecision::Reject {
                    reason: "allow and deny matched; reject configured".to_owned(),
                },
            };
        }

        if !self.allow.is_empty() && !allow_match {
            return FilterDecision::Skip {
                reason: "no allow glob matched".to_owned(),
            };
        }

        if deny_match {
            return FilterDecision::Skip {
                reason: "deny glob matched".to_owned(),
            };
        }

        FilterDecision::Inject
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpToolGlob(pub String);

impl McpToolGlob {
    pub fn matches(&self, candidate: &str) -> bool {
        glob_matches(&self.0, candidate)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterConflict {
    DenyWins,
    AllowWins,
    Reject,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterDecision {
    Inject,
    Skip { reason: String },
    Reject { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SamplingPolicy {
    mode: SamplingMode,
}

impl SamplingPolicy {
    pub fn denied() -> Self {
        Self {
            mode: SamplingMode::Denied,
        }
    }

    pub fn is_denied(&self) -> bool {
        self.mode == SamplingMode::Denied
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SamplingMode {
    Denied,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
    #[serde(rename = "outputSchema")]
    pub output_schema: Option<Value>,
    #[serde(rename = "_meta", default)]
    pub meta: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

impl McpToolResult {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![McpContent::Text { text: text.into() }],
            is_error: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpContent {
    Text { text: String },
    Json { value: Value },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpResourceContents {
    pub uri: String,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpPrompt {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpPromptMessages {
    pub messages: Vec<Value>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServerScope {
    Global,
    Session(SessionId),
    Agent(harness_contracts::AgentId),
}

pub fn trust_level_for_source(source: &McpServerSource) -> TrustLevel {
    if matches!(
        source,
        McpServerSource::Workspace | McpServerSource::Policy | McpServerSource::Managed { .. }
    ) {
        TrustLevel::AdminTrusted
    } else {
        // Plugin source only carries PluginId here, not plugin trust. Fail closed until the
        // plugin registry supplies that trust during composition.
        TrustLevel::UserControlled
    }
}

fn glob_matches(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let parts = pattern.split('*').collect::<Vec<_>>();
    if parts.len() == 1 {
        return pattern == candidate;
    }

    let mut remaining = candidate;
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if index == 0 {
            let Some(stripped) = remaining.strip_prefix(part) else {
                return false;
            };
            remaining = stripped;
            continue;
        }

        let Some(position) = remaining.find(part) else {
            return false;
        };
        remaining = &remaining[position + part.len()..];
    }

    pattern.ends_with('*')
        || parts
            .last()
            .is_some_and(|last| remaining.is_empty() || last.is_empty())
}
