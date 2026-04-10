use super::*;

pub(crate) fn normalize_optional_args(args: Option<&str>) -> Option<&str> {
    args.map(str::trim).filter(|value| !value.is_empty())
}

pub(crate) fn is_help_arg(arg: &str) -> bool {
    matches!(arg, "help" | "-h" | "--help")
}

pub(crate) fn help_path_from_args(args: &str) -> Option<Vec<&str>> {
    let parts = args.split_whitespace().collect::<Vec<_>>();
    let help_index = parts.iter().position(|part| is_help_arg(part))?;
    Some(parts[..help_index].to_vec())
}

pub(crate) fn config_source_label(source: ConfigSource) -> &'static str {
    match source {
        ConfigSource::User => "user",
        ConfigSource::Project => "project",
        ConfigSource::Local => "local",
    }
}

pub(crate) fn mcp_transport_label(config: &McpServerConfig) -> &'static str {
    match config {
        McpServerConfig::Stdio(_) => "stdio",
        McpServerConfig::Sse(_) => "sse",
        McpServerConfig::Http(_) => "http",
        McpServerConfig::Ws(_) => "ws",
        McpServerConfig::Sdk(_) => "sdk",
        McpServerConfig::ManagedProxy(_) => "managed-proxy",
    }
}

pub(crate) fn mcp_server_summary(config: &McpServerConfig) -> String {
    match config {
        McpServerConfig::Stdio(config) => {
            if config.args.is_empty() {
                config.command.clone()
            } else {
                format!("{} {}", config.command, config.args.join(" "))
            }
        }
        McpServerConfig::Sse(config) | McpServerConfig::Http(config) => config.url.clone(),
        McpServerConfig::Ws(config) => config.url.clone(),
        McpServerConfig::Sdk(config) => config.name.clone(),
        McpServerConfig::ManagedProxy(config) => format!("{} ({})", config.id, config.url),
    }
}

pub(crate) fn format_optional_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_string()
    } else {
        values.join(" ")
    }
}

pub(crate) fn format_optional_keys(mut keys: Vec<String>) -> String {
    if keys.is_empty() {
        return "<none>".to_string();
    }
    keys.sort();
    keys.join(", ")
}

pub(crate) fn format_mcp_oauth(oauth: Option<&McpOAuthConfig>) -> String {
    let Some(oauth) = oauth else {
        return "<none>".to_string();
    };

    let mut parts = Vec::new();
    if let Some(client_id) = &oauth.client_id {
        parts.push(format!("client_id={client_id}"));
    }
    if let Some(port) = oauth.callback_port {
        parts.push(format!("callback_port={port}"));
    }
    if let Some(url) = &oauth.auth_server_metadata_url {
        parts.push(format!("metadata_url={url}"));
    }
    if let Some(xaa) = oauth.xaa {
        parts.push(format!("xaa={xaa}"));
    }
    if parts.is_empty() {
        "enabled".to_string()
    } else {
        parts.join(", ")
    }
}

pub(crate) fn config_source_id(source: ConfigSource) -> &'static str {
    match source {
        ConfigSource::User => "user",
        ConfigSource::Project => "project",
        ConfigSource::Local => "local",
    }
}

pub(crate) fn config_source_json(source: ConfigSource) -> Value {
    json!({
        "id": config_source_id(source),
        "label": config_source_label(source),
    })
}

pub(crate) fn mcp_transport_json(config: &McpServerConfig) -> Value {
    let label = mcp_transport_label(config);
    json!({
        "id": label,
        "label": label,
    })
}

pub(crate) fn mcp_oauth_json(oauth: Option<&McpOAuthConfig>) -> Value {
    let Some(oauth) = oauth else {
        return Value::Null;
    };
    json!({
        "client_id": &oauth.client_id,
        "callback_port": oauth.callback_port,
        "auth_server_metadata_url": &oauth.auth_server_metadata_url,
        "xaa": oauth.xaa,
    })
}

pub(crate) fn mcp_server_details_json(config: &McpServerConfig) -> Value {
    match config {
        McpServerConfig::Stdio(config) => json!({
            "command": &config.command,
            "args": &config.args,
            "env_keys": config.env.keys().cloned().collect::<Vec<_>>(),
            "tool_call_timeout_ms": config.tool_call_timeout_ms,
        }),
        McpServerConfig::Sse(config) | McpServerConfig::Http(config) => json!({
            "url": &config.url,
            "header_keys": config.headers.keys().cloned().collect::<Vec<_>>(),
            "headers_helper": &config.headers_helper,
            "oauth": mcp_oauth_json(config.oauth.as_ref()),
        }),
        McpServerConfig::Ws(config) => json!({
            "url": &config.url,
            "header_keys": config.headers.keys().cloned().collect::<Vec<_>>(),
            "headers_helper": &config.headers_helper,
        }),
        McpServerConfig::Sdk(config) => json!({
            "name": &config.name,
        }),
        McpServerConfig::ManagedProxy(config) => json!({
            "url": &config.url,
            "id": &config.id,
        }),
    }
}
