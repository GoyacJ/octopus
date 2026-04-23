use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use serde_json::{Map, Value};

use crate::{HttpTransport, McpClient, McpPrompt, StdioTransport};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpOAuthConfig {
    pub client_id: Option<String>,
    pub callback_port: Option<u16>,
    pub auth_server_metadata_url: Option<String>,
    pub xaa: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpStdioServerConfig {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub tool_call_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpRemoteServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
    pub oauth: Option<McpOAuthConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpWebSocketServerConfig {
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub headers_helper: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpSdkServerConfig {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpManagedProxyServerConfig {
    pub url: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpServerConfig {
    Stdio(McpStdioServerConfig),
    Sse(McpRemoteServerConfig),
    Http(McpRemoteServerConfig),
    Ws(McpWebSocketServerConfig),
    Sdk(McpSdkServerConfig),
    ManagedProxy(McpManagedProxyServerConfig),
}

impl McpServerConfig {
    #[must_use]
    pub fn endpoint(&self) -> String {
        match self {
            Self::Stdio(config) => {
                if config.args.is_empty() {
                    format!("stdio: {}", config.command)
                } else {
                    format!("stdio: {} {}", config.command, config.args.join(" "))
                }
            }
            Self::Sse(config) | Self::Http(config) => config.url.clone(),
            Self::Ws(config) => config.url.clone(),
            Self::Sdk(config) => format!("sdk: {}", config.name),
            Self::ManagedProxy(config) => config.url.clone(),
        }
    }

    #[must_use]
    pub const fn transport_label(&self) -> &'static str {
        match self {
            Self::Stdio(_) => "stdio",
            Self::Sse(_) => "sse",
            Self::Http(_) => "http",
            Self::Ws(_) => "ws",
            Self::Sdk(_) => "sdk",
            Self::ManagedProxy(_) => "claudeai-proxy",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredMcpToolDefinition {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredMcpPromptDefinition {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredMcpResource {
    pub uri: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpTool {
    pub server_name: String,
    pub qualified_name: String,
    pub raw_name: String,
    pub tool: DiscoveredMcpToolDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedMcpPrompt {
    pub server_name: String,
    pub qualified_name: String,
    pub raw_name: String,
    pub prompt: DiscoveredMcpPromptDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscoveredMcpServerCapabilities {
    pub tools: Vec<ManagedMcpTool>,
    pub prompts: Vec<ManagedMcpPrompt>,
    pub resources: Vec<DiscoveredMcpResource>,
    pub status_detail: Option<String>,
    pub availability: String,
}

impl DiscoveredMcpServerCapabilities {
    #[must_use]
    pub fn finalize(mut self) -> Self {
        if self.availability.is_empty() {
            self.availability = if self.status_detail.is_some() {
                "attention".into()
            } else if self.tools.is_empty() && self.prompts.is_empty() && self.resources.is_empty()
            {
                "configured".into()
            } else {
                "healthy".into()
            };
        }
        self
    }
}

#[must_use]
pub fn parse_mcp_servers(document: &Map<String, Value>) -> BTreeMap<String, McpServerConfig> {
    document
        .get("mcpServers")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|servers| servers.iter())
        .filter_map(|(server_name, value)| {
            parse_mcp_server_config(value).map(|config| (server_name.clone(), config))
        })
        .collect()
}

#[must_use]
pub fn parse_mcp_server_config(value: &Value) -> Option<McpServerConfig> {
    let object = value.as_object()?;
    let config = match object.get("type").and_then(Value::as_str) {
        Some("stdio") => McpServerConfig::Stdio(McpStdioServerConfig {
            command: object.get("command")?.as_str()?.to_string(),
            args: object
                .get("args")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| item.as_str().map(ToOwned::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
            env: string_map_from_json(object.get("env")),
            tool_call_timeout_ms: object.get("toolCallTimeoutMs").and_then(Value::as_u64),
        }),
        Some("sse") => McpServerConfig::Sse(McpRemoteServerConfig {
            url: object.get("url")?.as_str()?.to_string(),
            headers: string_map_from_json(object.get("headers")),
            headers_helper: object
                .get("headersHelper")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            oauth: mcp_oauth_config_from_json(object.get("oauth")),
        }),
        Some("http") => McpServerConfig::Http(McpRemoteServerConfig {
            url: object.get("url")?.as_str()?.to_string(),
            headers: string_map_from_json(object.get("headers")),
            headers_helper: object
                .get("headersHelper")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            oauth: mcp_oauth_config_from_json(object.get("oauth")),
        }),
        Some("ws") => McpServerConfig::Ws(McpWebSocketServerConfig {
            url: object.get("url")?.as_str()?.to_string(),
            headers: string_map_from_json(object.get("headers")),
            headers_helper: object
                .get("headersHelper")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        }),
        Some("sdk") => McpServerConfig::Sdk(McpSdkServerConfig {
            name: object.get("name")?.as_str()?.to_string(),
        }),
        Some("claudeai-proxy") => McpServerConfig::ManagedProxy(McpManagedProxyServerConfig {
            url: object.get("url")?.as_str()?.to_string(),
            id: object.get("id")?.as_str()?.to_string(),
        }),
        _ => return None,
    };

    Some(config)
}

#[must_use]
pub fn mcp_endpoint(config: &McpServerConfig) -> String {
    config.endpoint()
}

#[must_use]
pub fn qualified_mcp_tool_name(server_name: &str, tool_name: &str) -> String {
    format!("mcp_tool__{server_name}__{tool_name}")
}

#[must_use]
pub fn qualified_mcp_prompt_name(server_name: &str, prompt_name: &str) -> String {
    format!("mcp_prompt__{server_name}__{prompt_name}")
}

#[must_use]
pub fn qualified_mcp_resource_name(server_name: &str, uri: &str) -> String {
    format!(
        "mcp_resource__{server_name}__{}",
        sanitize_mcp_resource_segment(uri)
    )
}

pub async fn discover_mcp_server_capabilities_best_effort(
    servers: &BTreeMap<String, McpServerConfig>,
) -> BTreeMap<String, DiscoveredMcpServerCapabilities> {
    let mut discovered = servers
        .keys()
        .cloned()
        .map(|server_name| (server_name, DiscoveredMcpServerCapabilities::default()))
        .collect::<BTreeMap<_, _>>();

    for (server_name, config) in servers {
        let entry = discovered.entry(server_name.clone()).or_default();
        let client = match client_for_server(server_name, config) {
            Ok(client) => client,
            Err(message) => {
                entry.status_detail = Some(message);
                entry.availability = "attention".into();
                continue;
            }
        };

        match client.list_tools().await {
            Ok(tools) => {
                entry.tools = tools
                    .into_iter()
                    .map(|tool| ManagedMcpTool {
                        server_name: server_name.clone(),
                        qualified_name: qualified_mcp_tool_name(server_name, &tool.name),
                        raw_name: tool.name.clone(),
                        tool: DiscoveredMcpToolDefinition {
                            name: tool.name,
                            description: (!tool.description.trim().is_empty())
                                .then_some(tool.description),
                        },
                    })
                    .collect();
            }
            Err(error) => {
                entry.status_detail = Some(error.to_string());
                entry.availability = "attention".into();
                continue;
            }
        }

        match client.list_prompts().await {
            Ok(prompts) => {
                entry.prompts = prompts
                    .into_iter()
                    .map(|prompt| managed_prompt_from_wire(server_name, prompt))
                    .collect();
            }
            Err(error) => {
                entry.status_detail = Some(error.to_string());
                entry.availability = "attention".into();
                continue;
            }
        }

        match client.list_resources().await {
            Ok(resources) => {
                entry.resources = resources
                    .into_iter()
                    .map(|resource| DiscoveredMcpResource {
                        uri: resource.uri,
                        name: Some(resource.name),
                        description: resource.description,
                        mime_type: resource.mime_type,
                    })
                    .collect();
            }
            Err(error) => {
                entry.status_detail = Some(error.to_string());
                entry.availability = "attention".into();
            }
        }
    }

    discovered
        .into_iter()
        .map(|(server_name, capabilities)| (server_name, capabilities.finalize()))
        .collect()
}

fn client_for_server(server_name: &str, config: &McpServerConfig) -> Result<McpClient, String> {
    match config {
        McpServerConfig::Stdio(config) => {
            let transport = StdioTransport::spawn(
                &config.command,
                config.args.clone(),
                config
                    .env
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<HashMap<_, _>>(),
            )
            .map_err(|error| error.to_string())?;
            Ok(McpClient::new(server_name.to_string(), Arc::new(transport)))
        }
        McpServerConfig::Http(config) => {
            let transport = HttpTransport::new(
                config.url.clone(),
                config
                    .headers
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<HashMap<_, _>>(),
            )
            .map_err(|error| error.to_string())?;
            Ok(McpClient::new(server_name.to_string(), Arc::new(transport)))
        }
        unsupported => Err(format!(
            "transport {} is not supported by SDK MCP discovery (not supported transport)",
            unsupported.transport_label()
        )),
    }
}

fn managed_prompt_from_wire(server_name: &str, prompt: McpPrompt) -> ManagedMcpPrompt {
    ManagedMcpPrompt {
        server_name: server_name.to_string(),
        qualified_name: qualified_mcp_prompt_name(server_name, &prompt.name),
        raw_name: prompt.name.clone(),
        prompt: DiscoveredMcpPromptDefinition {
            name: prompt.name,
            description: prompt.description,
        },
    }
}

fn string_map_from_json(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| value.as_str().map(|item| (key.clone(), item.into())))
                .collect()
        })
        .unwrap_or_default()
}

fn mcp_oauth_config_from_json(value: Option<&Value>) -> Option<McpOAuthConfig> {
    let object = value?.as_object()?;
    Some(McpOAuthConfig {
        client_id: object
            .get("clientId")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        callback_port: object
            .get("callbackPort")
            .and_then(Value::as_u64)
            .and_then(|item| u16::try_from(item).ok()),
        auth_server_metadata_url: object
            .get("authServerMetadataUrl")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        xaa: object.get("xaa").and_then(Value::as_bool),
    })
}

fn sanitize_mcp_resource_segment(value: &str) -> String {
    value.replace(|ch: char| !ch.is_ascii_alphanumeric(), "_")
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use serde_json::json;

    use super::{
        discover_mcp_server_capabilities_best_effort, mcp_endpoint, parse_mcp_server_config,
        parse_mcp_servers, qualified_mcp_prompt_name, qualified_mcp_resource_name,
        qualified_mcp_tool_name, McpServerConfig,
    };

    fn echo_server_binary() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
        let executable_name = if cfg!(windows) {
            "mcp-echo-server.exe"
        } else {
            "mcp-echo-server"
        };
        let candidate = manifest_dir
            .join("target")
            .join(&profile)
            .join(executable_name);
        if candidate.exists() {
            return candidate;
        }

        let workspace_candidate = manifest_dir
            .parent()
            .and_then(Path::parent)
            .map(|workspace_root| {
                workspace_root
                    .join("target")
                    .join(profile)
                    .join(executable_name)
            })
            .expect("workspace target should resolve");
        assert!(
            workspace_candidate.exists(),
            "mcp-echo-server binary missing at {}",
            workspace_candidate.display()
        );
        workspace_candidate
    }

    fn temp_socket_path() -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should work")
            .as_nanos();
        std::env::temp_dir().join(format!("octopus-sdk-mcp-{suffix}.sock"))
    }

    #[test]
    fn parses_mcp_server_config_documents() {
        let config = parse_mcp_server_config(&json!({
            "type": "stdio",
            "command": "uvx",
            "args": ["server"],
            "env": { "TOKEN": "secret" },
            "toolCallTimeoutMs": 1500
        }))
        .expect("stdio config should parse");

        match config {
            McpServerConfig::Stdio(config) => {
                assert_eq!(config.command, "uvx");
                assert_eq!(config.args, vec!["server"]);
                assert_eq!(config.env.get("TOKEN").map(String::as_str), Some("secret"));
                assert_eq!(config.tool_call_timeout_ms, Some(1500));
            }
            other => panic!("expected stdio config, got {other:?}"),
        }

        let document = json!({
            "mcpServers": {
                "alpha": {
                    "type": "http",
                    "url": "https://vendor.example/mcp"
                },
                "invalid": {
                    "type": "http"
                }
            }
        });
        let parsed = parse_mcp_servers(document.as_object().expect("document object"));
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed.get("alpha").map(mcp_endpoint),
            Some("https://vendor.example/mcp".into())
        );
    }

    #[test]
    fn builds_qualified_catalog_ids() {
        assert_eq!(
            qualified_mcp_tool_name("ops", "tail_logs"),
            "mcp_tool__ops__tail_logs"
        );
        assert_eq!(
            qualified_mcp_prompt_name("ops", "deploy_review"),
            "mcp_prompt__ops__deploy_review"
        );
        assert_eq!(
            qualified_mcp_resource_name("ops", "file://ops-guide.txt"),
            "mcp_resource__ops__file___ops_guide_txt"
        );
    }

    #[tokio::test]
    async fn discovers_stdio_capabilities_and_marks_unsupported_transports() {
        let socket_path = temp_socket_path();
        let binary = echo_server_binary();
        let servers = BTreeMap::from([
            (
                "echo".to_string(),
                parse_mcp_server_config(&json!({
                    "type": "stdio",
                    "command": binary.to_string_lossy(),
                    "args": ["--socket", socket_path.to_string_lossy()]
                }))
                .expect("stdio config should parse"),
            ),
            (
                "remote".to_string(),
                parse_mcp_server_config(&json!({
                    "type": "ws",
                    "url": "wss://vendor.example/mcp"
                }))
                .expect("ws config should parse"),
            ),
        ]);

        let discovered = discover_mcp_server_capabilities_best_effort(&servers).await;

        let echo = discovered
            .get("echo")
            .expect("echo server should be discovered");
        assert_eq!(echo.availability, "healthy");
        assert!(echo
            .tools
            .iter()
            .any(|tool| tool.qualified_name == "mcp_tool__echo__echo"));
        assert!(echo.prompts.is_empty());
        assert!(echo.resources.is_empty());

        let remote = discovered
            .get("remote")
            .expect("unsupported server should still be listed");
        assert_eq!(remote.availability, "attention");
        assert!(remote
            .status_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("transport ws")));

        let _ = fs::remove_file(socket_path);
    }
}
