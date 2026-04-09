use super::*;

pub fn handle_mcp_slash_command(
    args: Option<&str>,
    cwd: &Path,
) -> Result<String, runtime::ConfigError> {
    let loader = ConfigLoader::default_for(cwd);
    render_mcp_report_for(&loader, cwd, args)
}

pub fn handle_mcp_slash_command_json(
    args: Option<&str>,
    cwd: &Path,
) -> Result<Value, runtime::ConfigError> {
    let loader = ConfigLoader::default_for(cwd);
    render_mcp_report_json_for(&loader, cwd, args)
}

pub(crate) fn render_mcp_report_for(
    loader: &ConfigLoader,
    cwd: &Path,
    args: Option<&str>,
) -> Result<String, runtime::ConfigError> {
    if let Some(args) = normalize_optional_args(args) {
        if let Some(help_path) = help_path_from_args(args) {
            return Ok(match help_path.as_slice() {
                [] => render_mcp_usage(None),
                ["show", ..] => render_mcp_usage(Some("show")),
                _ => render_mcp_usage(Some(&help_path.join(" "))),
            });
        }
    }

    match normalize_optional_args(args) {
        None | Some("list") => {
            let runtime_config = loader.load()?;
            Ok(render_mcp_summary_report(
                cwd,
                runtime_config.mcp().servers(),
            ))
        }
        Some(args) if is_help_arg(args) => Ok(render_mcp_usage(None)),
        Some("show") => Ok(render_mcp_usage(Some("show"))),
        Some(args) if args.split_whitespace().next() == Some("show") => {
            let mut parts = args.split_whitespace();
            let _ = parts.next();
            let Some(server_name) = parts.next() else {
                return Ok(render_mcp_usage(Some("show")));
            };
            if parts.next().is_some() {
                return Ok(render_mcp_usage(Some(args)));
            }
            let runtime_config = loader.load()?;
            Ok(render_mcp_server_report(
                cwd,
                server_name,
                runtime_config.mcp().get(server_name),
            ))
        }
        Some(args) => Ok(render_mcp_usage(Some(args))),
    }
}

pub(crate) fn render_mcp_report_json_for(
    loader: &ConfigLoader,
    cwd: &Path,
    args: Option<&str>,
) -> Result<Value, runtime::ConfigError> {
    if let Some(args) = normalize_optional_args(args) {
        if let Some(help_path) = help_path_from_args(args) {
            return Ok(match help_path.as_slice() {
                [] => render_mcp_usage_json(None),
                ["show", ..] => render_mcp_usage_json(Some("show")),
                _ => render_mcp_usage_json(Some(&help_path.join(" "))),
            });
        }
    }

    match normalize_optional_args(args) {
        None | Some("list") => {
            let runtime_config = loader.load()?;
            Ok(render_mcp_summary_report_json(
                cwd,
                runtime_config.mcp().servers(),
            ))
        }
        Some(args) if is_help_arg(args) => Ok(render_mcp_usage_json(None)),
        Some("show") => Ok(render_mcp_usage_json(Some("show"))),
        Some(args) if args.split_whitespace().next() == Some("show") => {
            let mut parts = args.split_whitespace();
            let _ = parts.next();
            let Some(server_name) = parts.next() else {
                return Ok(render_mcp_usage_json(Some("show")));
            };
            if parts.next().is_some() {
                return Ok(render_mcp_usage_json(Some(args)));
            }
            let runtime_config = loader.load()?;
            Ok(render_mcp_server_report_json(
                cwd,
                server_name,
                runtime_config.mcp().get(server_name),
            ))
        }
        Some(args) => Ok(render_mcp_usage_json(Some(args))),
    }
}

pub(crate) fn render_mcp_summary_report(
    cwd: &Path,
    servers: &BTreeMap<String, ScopedMcpServerConfig>,
) -> String {
    let mut lines = vec![
        "MCP".to_string(),
        format!("  Working directory {}", cwd.display()),
        format!("  Configured servers {}", servers.len()),
    ];
    if servers.is_empty() {
        lines.push("  No MCP servers configured.".to_string());
        return lines.join("\n");
    }

    lines.push(String::new());
    for (name, server) in servers {
        lines.push(format!(
            "  {name:<16} {transport:<13} {scope:<7} {summary}",
            transport = mcp_transport_label(&server.config),
            scope = config_source_label(server.scope),
            summary = mcp_server_summary(&server.config)
        ));
    }

    lines.join("\n")
}

pub(crate) fn render_mcp_summary_report_json(
    cwd: &Path,
    servers: &BTreeMap<String, ScopedMcpServerConfig>,
) -> Value {
    json!({
        "kind": "mcp",
        "action": "list",
        "working_directory": cwd.display().to_string(),
        "configured_servers": servers.len(),
        "servers": servers
            .iter()
            .map(|(name, server)| mcp_server_json(name, server))
            .collect::<Vec<_>>(),
    })
}

pub(crate) fn render_mcp_server_report(
    cwd: &Path,
    server_name: &str,
    server: Option<&ScopedMcpServerConfig>,
) -> String {
    let Some(server) = server else {
        return format!(
            "MCP\n  Working directory {}\n  Result            server `{server_name}` is not configured",
            cwd.display()
        );
    };

    let mut lines = vec![
        "MCP".to_string(),
        format!("  Working directory {}", cwd.display()),
        format!("  Name              {server_name}"),
        format!("  Scope             {}", config_source_label(server.scope)),
        format!(
            "  Transport         {}",
            mcp_transport_label(&server.config)
        ),
    ];

    match &server.config {
        McpServerConfig::Stdio(config) => {
            lines.push(format!("  Command           {}", config.command));
            lines.push(format!(
                "  Args              {}",
                format_optional_list(&config.args)
            ));
            lines.push(format!(
                "  Env keys          {}",
                format_optional_keys(config.env.keys().cloned().collect())
            ));
            lines.push(format!(
                "  Tool timeout      {}",
                config
                    .tool_call_timeout_ms
                    .map_or_else(|| "<default>".to_string(), |value| format!("{value} ms"))
            ));
        }
        McpServerConfig::Sse(config) | McpServerConfig::Http(config) => {
            lines.push(format!("  URL               {}", config.url));
            lines.push(format!(
                "  Header keys       {}",
                format_optional_keys(config.headers.keys().cloned().collect())
            ));
            lines.push(format!(
                "  Header helper     {}",
                config.headers_helper.as_deref().unwrap_or("<none>")
            ));
            lines.push(format!(
                "  OAuth             {}",
                format_mcp_oauth(config.oauth.as_ref())
            ));
        }
        McpServerConfig::Ws(config) => {
            lines.push(format!("  URL               {}", config.url));
            lines.push(format!(
                "  Header keys       {}",
                format_optional_keys(config.headers.keys().cloned().collect())
            ));
            lines.push(format!(
                "  Header helper     {}",
                config.headers_helper.as_deref().unwrap_or("<none>")
            ));
        }
        McpServerConfig::Sdk(config) => {
            lines.push(format!("  SDK name          {}", config.name));
        }
        McpServerConfig::ManagedProxy(config) => {
            lines.push(format!("  URL               {}", config.url));
            lines.push(format!("  Proxy id          {}", config.id));
        }
    }

    lines.join("\n")
}

pub(crate) fn render_mcp_server_report_json(
    cwd: &Path,
    server_name: &str,
    server: Option<&ScopedMcpServerConfig>,
) -> Value {
    match server {
        Some(server) => json!({
            "kind": "mcp",
            "action": "show",
            "working_directory": cwd.display().to_string(),
            "found": true,
            "server": mcp_server_json(server_name, server),
        }),
        None => json!({
            "kind": "mcp",
            "action": "show",
            "working_directory": cwd.display().to_string(),
            "found": false,
            "server_name": server_name,
            "message": format!("server `{server_name}` is not configured"),
        }),
    }
}

pub(crate) fn render_mcp_usage(unexpected: Option<&str>) -> String {
    let mut lines = vec![
        "MCP".to_string(),
        "  Usage            /mcp [list|show <server>|help]".to_string(),
        "  Direct CLI       claw mcp [list|show <server>|help]".to_string(),
        "  Sources          .claw/settings.json, .claw/settings.local.json".to_string(),
    ];
    if let Some(args) = unexpected {
        lines.push(format!("  Unexpected       {args}"));
    }
    lines.join("\n")
}

pub(crate) fn render_mcp_usage_json(unexpected: Option<&str>) -> Value {
    json!({
        "kind": "mcp",
        "action": "help",
        "usage": {
            "slash_command": "/mcp [list|show <server>|help]",
            "direct_cli": "claw mcp [list|show <server>|help]",
            "sources": [".claw/settings.json", ".claw/settings.local.json"],
        },
        "unexpected": unexpected,
    })
}

pub(crate) fn mcp_server_json(name: &str, server: &ScopedMcpServerConfig) -> Value {
    json!({
        "name": name,
        "scope": config_source_json(server.scope),
        "transport": mcp_transport_json(&server.config),
        "summary": mcp_server_summary(&server.config),
        "details": mcp_server_details_json(&server.config),
    })
}
