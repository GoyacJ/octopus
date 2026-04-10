use super::{
    apply_config_patch, ConfigLoader, ConfigSource, McpServerConfig, McpTransport,
    ResolvedPermissionMode, RuntimeHookConfig, RuntimePluginConfig, CLAW_SETTINGS_SCHEMA_NAME,
};
use crate::json::JsonValue;
use crate::sandbox::FilesystemIsolationMode;
use crate::{config_merge, config_patch, config_secrets, config_sources};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir() -> std::path::PathBuf {
    static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    let unique_id = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("runtime-config-{nanos}-{unique_id}"))
}

#[test]
fn rejects_non_object_settings_files() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(home.join("settings.json"), "[]").expect("write bad settings");

    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");
    assert!(error
        .to_string()
        .contains("top-level settings value must be a JSON object"));

    if root.exists() {
        fs::remove_dir_all(root).expect("cleanup temp dir");
    }
}

#[test]
fn loads_and_merges_claude_code_config_files_by_precedence() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.parent().expect("home parent").join(".claw.json"),
        r#"{"model":"haiku","env":{"A":"1"},"mcpServers":{"home":{"command":"uvx","args":["home"]}}}"#,
    )
    .expect("write user compat config");
    fs::write(
        home.join("settings.json"),
        r#"{"model":"sonnet","env":{"A2":"1"},"hooks":{"PreToolUse":["base"]},"permissions":{"defaultMode":"plan","allow":["Read"],"deny":["Bash(rm -rf)"]}}"#,
    )
    .expect("write user settings");
    fs::write(
        cwd.join(".claw.json"),
        r#"{"model":"project-compat","env":{"B":"2"}}"#,
    )
    .expect("write project compat config");
    fs::write(
        cwd.join(".claw").join("settings.json"),
        r#"{"env":{"C":"3"},"hooks":{"PostToolUse":["project"],"PostToolUseFailure":["project-failure"]},"permissions":{"ask":["Edit"]},"mcpServers":{"project":{"command":"uvx","args":["project"]}}}"#,
    )
    .expect("write project settings");
    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{"model":"opus","permissionMode":"acceptEdits"}"#,
    )
    .expect("write local settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(CLAW_SETTINGS_SCHEMA_NAME, "SettingsSchema");
    assert_eq!(loaded.loaded_entries().len(), 5);
    assert_eq!(loaded.loaded_entries()[0].source, ConfigSource::User);
    assert_eq!(
        loaded.get("model"),
        Some(&JsonValue::String("opus".to_string()))
    );
    assert_eq!(loaded.model(), Some("opus"));
    assert_eq!(
        loaded.permission_mode(),
        Some(ResolvedPermissionMode::WorkspaceWrite)
    );
    assert_eq!(
        loaded
            .get("env")
            .and_then(JsonValue::as_object)
            .expect("env object")
            .len(),
        4
    );
    assert!(loaded
        .get("hooks")
        .and_then(JsonValue::as_object)
        .expect("hooks object")
        .contains_key("PreToolUse"));
    assert!(loaded
        .get("hooks")
        .and_then(JsonValue::as_object)
        .expect("hooks object")
        .contains_key("PostToolUse"));
    assert_eq!(loaded.hooks().pre_tool_use(), &["base".to_string()]);
    assert_eq!(loaded.hooks().post_tool_use(), &["project".to_string()]);
    assert_eq!(
        loaded.hooks().post_tool_use_failure(),
        &["project-failure".to_string()]
    );
    assert_eq!(loaded.permission_rules().allow(), &["Read".to_string()]);
    assert_eq!(
        loaded.permission_rules().deny(),
        &["Bash(rm -rf)".to_string()]
    );
    assert_eq!(loaded.permission_rules().ask(), &["Edit".to_string()]);
    assert!(loaded.mcp().get("home").is_some());
    assert!(loaded.mcp().get("project").is_some());

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_sandbox_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{
          "sandbox": {
            "enabled": true,
            "namespaceRestrictions": false,
            "networkIsolation": true,
            "filesystemMode": "allow-list",
            "allowedMounts": ["logs", "tmp/cache"]
          }
        }"#,
    )
    .expect("write local settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(loaded.sandbox().enabled, Some(true));
    assert_eq!(loaded.sandbox().namespace_restrictions, Some(false));
    assert_eq!(loaded.sandbox().network_isolation, Some(true));
    assert_eq!(
        loaded.sandbox().filesystem_mode,
        Some(FilesystemIsolationMode::AllowList)
    );
    assert_eq!(loaded.sandbox().allowed_mounts, vec!["logs", "tmp/cache"]);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_typed_mcp_and_oauth_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{
          "mcpServers": {
            "stdio-server": {
              "command": "uvx",
              "args": ["mcp-server"],
              "env": {"TOKEN": "secret"}
            },
            "remote-server": {
              "type": "http",
              "url": "https://example.test/mcp",
              "headers": {"Authorization": "Bearer token"},
              "headersHelper": "helper.sh",
              "oauth": {
                "clientId": "mcp-client",
                "callbackPort": 7777,
                "authServerMetadataUrl": "https://issuer.test/.well-known/oauth-authorization-server",
                "xaa": true
              }
            }
          },
          "oauth": {
            "clientId": "runtime-client",
            "authorizeUrl": "https://console.test/oauth/authorize",
            "tokenUrl": "https://console.test/oauth/token",
            "callbackPort": 54545,
            "manualRedirectUrl": "https://console.test/oauth/callback",
            "scopes": ["org:read", "user:write"]
          }
        }"#,
    )
    .expect("write user settings");
    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{
          "mcpServers": {
            "remote-server": {
              "type": "ws",
              "url": "wss://override.test/mcp",
              "headers": {"X-Env": "local"}
            }
          }
        }"#,
    )
    .expect("write local settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    let stdio_server = loaded
        .mcp()
        .get("stdio-server")
        .expect("stdio server should exist");
    assert_eq!(stdio_server.scope, ConfigSource::User);
    assert_eq!(stdio_server.transport(), McpTransport::Stdio);

    let remote_server = loaded
        .mcp()
        .get("remote-server")
        .expect("remote server should exist");
    assert_eq!(remote_server.scope, ConfigSource::Local);
    assert_eq!(remote_server.transport(), McpTransport::Ws);
    match &remote_server.config {
        McpServerConfig::Ws(config) => {
            assert_eq!(config.url, "wss://override.test/mcp");
            assert_eq!(
                config.headers.get("X-Env").map(String::as_str),
                Some("local")
            );
        }
        other => panic!("expected ws config, got {other:?}"),
    }

    let oauth = loaded.oauth().expect("oauth config should exist");
    assert_eq!(oauth.client_id, "runtime-client");
    assert_eq!(oauth.callback_port, Some(54_545));
    assert_eq!(oauth.scopes, vec!["org:read", "user:write"]);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn infers_http_mcp_servers_from_url_only_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(
        home.join("settings.json"),
        r#"{
          "mcpServers": {
            "remote": {
              "url": "https://example.test/mcp"
            }
          }
        }"#,
    )
    .expect("write mcp settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    let remote_server = loaded
        .mcp()
        .get("remote")
        .expect("remote server should exist");
    assert_eq!(remote_server.transport(), McpTransport::Http);
    match &remote_server.config {
        McpServerConfig::Http(config) => {
            assert_eq!(config.url, "https://example.test/mcp");
        }
        other => panic!("expected http config, got {other:?}"),
    }

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_plugin_config_from_enabled_plugins() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{
          "enabledPlugins": {
            "tool-guard@builtin": true,
            "sample-plugin@external": false
          }
        }"#,
    )
    .expect("write user settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(
        loaded.plugins().enabled_plugins().get("tool-guard@builtin"),
        Some(&true)
    );
    assert_eq!(
        loaded
            .plugins()
            .enabled_plugins()
            .get("sample-plugin@external"),
        Some(&false)
    );

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn parses_plugin_config() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{
          "enabledPlugins": {
            "core-helpers@builtin": true
          },
          "plugins": {
            "externalDirectories": ["./external-plugins"],
            "installRoot": "plugin-cache/installed",
            "registryPath": "plugin-cache/installed.json",
            "bundledRoot": "./bundled-plugins"
          }
        }"#,
    )
    .expect("write plugin settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(
        loaded
            .plugins()
            .enabled_plugins()
            .get("core-helpers@builtin"),
        Some(&true)
    );
    assert_eq!(
        loaded.plugins().external_directories(),
        &["./external-plugins".to_string()]
    );
    assert_eq!(
        loaded.plugins().install_root(),
        Some("plugin-cache/installed")
    );
    assert_eq!(
        loaded.plugins().registry_path(),
        Some("plugin-cache/installed.json")
    );
    assert_eq!(loaded.plugins().bundled_root(), Some("./bundled-plugins"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn rejects_invalid_mcp_server_shapes() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(
        home.join("settings.json"),
        r#"{"mcpServers":{"broken":{"type":"http","url":123}}}"#,
    )
    .expect("write broken settings");

    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    assert!(error
        .to_string()
        .contains("mcpServers.broken: missing string field url"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn empty_settings_file_loads_defaults() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(&home).expect("home config dir");
    fs::create_dir_all(&cwd).expect("project dir");
    fs::write(home.join("settings.json"), "").expect("write empty settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("empty settings should still load");

    assert_eq!(loaded.loaded_entries().len(), 1);
    assert_eq!(loaded.permission_mode(), None);
    assert_eq!(loaded.plugins().enabled_plugins().len(), 0);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn load_documents_reports_exists_and_loaded_per_source() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{"model":"sonnet","custom":{"enabled":true}}"#,
    )
    .expect("write user settings");
    fs::write(
        cwd.join(".claw").join("settings.local.json"),
        r#"{"permissionMode":"acceptEdits"}"#,
    )
    .expect("write local settings");

    let documents = ConfigLoader::new(&cwd, &home)
        .load_documents()
        .expect("documents should load");

    assert_eq!(documents.len(), 5);

    let user_settings = documents
        .iter()
        .find(|document| {
            document.source == ConfigSource::User && document.path == home.join("settings.json")
        })
        .expect("user settings document");
    assert!(user_settings.exists);
    assert!(user_settings.loaded);
    assert_eq!(
        user_settings
            .document
            .as_ref()
            .and_then(|document| document.get("model")),
        Some(&JsonValue::String("sonnet".into()))
    );

    let user_legacy = documents
        .iter()
        .find(|document| {
            document.source == ConfigSource::User
                && document.path == home.parent().expect("home parent").join(".claw.json")
        })
        .expect("user legacy document");
    assert!(!user_legacy.exists);
    assert!(!user_legacy.loaded);
    assert!(user_legacy.document.is_none());

    let local_settings = documents
        .iter()
        .find(|document| {
            document.source == ConfigSource::Local
                && document.path == cwd.join(".claw").join("settings.local.json")
        })
        .expect("local settings document");
    assert!(local_settings.exists);
    assert!(local_settings.loaded);

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn deep_merge_objects_merges_nested_maps() {
    let mut target = JsonValue::parse(r#"{"env":{"A":"1","B":"2"},"model":"haiku"}"#)
        .expect("target JSON should parse")
        .as_object()
        .expect("target should be an object")
        .clone();
    let source = JsonValue::parse(r#"{"env":{"B":"override","C":"3"},"sandbox":{"enabled":true}}"#)
        .expect("source JSON should parse")
        .as_object()
        .expect("source should be an object")
        .clone();

    config_merge::deep_merge_objects(&mut target, &source);

    let env = target
        .get("env")
        .and_then(JsonValue::as_object)
        .expect("env should remain an object");
    assert_eq!(env.get("A"), Some(&JsonValue::String("1".to_string())));
    assert_eq!(
        env.get("B"),
        Some(&JsonValue::String("override".to_string()))
    );
    assert_eq!(env.get("C"), Some(&JsonValue::String("3".to_string())));
    assert!(target.contains_key("sandbox"));
}

#[test]
fn apply_config_patch_preserves_unknown_keys_and_removes_nulls() {
    let mut target = JsonValue::parse(
        r#"{
          "permissions":{"defaultMode":"plan"},
          "custom":{"keep":"yes","remove":"soon"},
          "model":"haiku"
        }"#,
    )
    .expect("target JSON should parse")
    .as_object()
    .expect("target should be an object")
    .clone();
    let patch = JsonValue::parse(
        r#"{
          "permissions":{"defaultMode":"acceptEdits"},
          "custom":{"remove":null},
          "model":"sonnet"
        }"#,
    )
    .expect("patch JSON should parse")
    .as_object()
    .expect("patch should be an object")
    .clone();

    apply_config_patch(&mut target, &patch);

    assert_eq!(
        target
            .get("permissions")
            .and_then(JsonValue::as_object)
            .and_then(|object| object.get("defaultMode")),
        Some(&JsonValue::String("acceptEdits".into()))
    );
    assert_eq!(
        target
            .get("custom")
            .and_then(JsonValue::as_object)
            .and_then(|object| object.get("keep")),
        Some(&JsonValue::String("yes".into()))
    );
    assert_eq!(
        target
            .get("custom")
            .and_then(JsonValue::as_object)
            .and_then(|object| object.get("remove")),
        None
    );
    assert_eq!(
        target.get("model"),
        Some(&JsonValue::String("sonnet".into()))
    );
}

#[test]
fn rejects_invalid_hook_entries_before_merge() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    let project_settings = cwd.join(".claw").join("settings.json");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{"hooks":{"PreToolUse":["base"]}}"#,
    )
    .expect("write user settings");
    fs::write(
        &project_settings,
        r#"{"hooks":{"PreToolUse":["project",42]}}"#,
    )
    .expect("write invalid project settings");

    let error = ConfigLoader::new(&cwd, &home)
        .load()
        .expect_err("config should fail");

    let rendered = error.to_string();
    assert!(rendered.contains(&format!(
        "{}: hooks: field PreToolUse must contain only strings",
        project_settings.display()
    )));
    assert!(!rendered.contains("merged settings.hooks"));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn permission_mode_aliases_resolve_to_expected_modes() {
    assert_eq!(
        config_merge::parse_permission_mode_label("plan", "test").expect("plan should resolve"),
        ResolvedPermissionMode::ReadOnly
    );
    assert_eq!(
        config_merge::parse_permission_mode_label("acceptEdits", "test")
            .expect("acceptEdits should resolve"),
        ResolvedPermissionMode::WorkspaceWrite
    );
    assert_eq!(
        config_merge::parse_permission_mode_label("dontAsk", "test")
            .expect("dontAsk should resolve"),
        ResolvedPermissionMode::DangerFullAccess
    );
}

#[test]
fn hook_config_merge_preserves_uniques() {
    let base = RuntimeHookConfig::new(
        vec!["pre-a".to_string()],
        vec!["post-a".to_string()],
        vec!["failure-a".to_string()],
    );
    let overlay = RuntimeHookConfig::new(
        vec!["pre-a".to_string(), "pre-b".to_string()],
        vec!["post-a".to_string(), "post-b".to_string()],
        vec!["failure-b".to_string()],
    );

    let merged = base.merged(&overlay);

    assert_eq!(
        merged.pre_tool_use(),
        &["pre-a".to_string(), "pre-b".to_string()]
    );
    assert_eq!(
        merged.post_tool_use(),
        &["post-a".to_string(), "post-b".to_string()]
    );
    assert_eq!(
        merged.post_tool_use_failure(),
        &["failure-a".to_string(), "failure-b".to_string()]
    );
}

#[test]
fn plugin_state_falls_back_to_default_for_unknown_plugin() {
    let mut config = RuntimePluginConfig::default();
    config.set_plugin_state("known".to_string(), true);

    assert!(config.state_for("known", false));
    assert!(config.state_for("missing", true));
    assert!(!config.state_for("missing", false));
}

#[test]
fn parses_aliases_provider_fallbacks_trusted_roots_and_plugin_max_output_tokens() {
    let root = temp_dir();
    let cwd = root.join("project");
    let home = root.join("home").join(".claw");
    fs::create_dir_all(cwd.join(".claw")).expect("project config dir");
    fs::create_dir_all(&home).expect("home config dir");

    fs::write(
        home.join("settings.json"),
        r#"{
          "aliases": {
            "fast": "gpt-5-mini",
            "deep": "claude-sonnet-4-5"
          },
          "providerFallbacks": {
            "primary": "anthropic",
            "fallbacks": ["openai", "dashscope"]
          },
          "trustedRoots": ["/tmp/worktrees", "/srv/octopus"],
          "plugins": {
            "maxOutputTokens": 2048
          }
        }"#,
    )
    .expect("write user settings");

    let loaded = ConfigLoader::new(&cwd, &home)
        .load()
        .expect("config should load");

    assert_eq!(
        loaded.aliases().get("fast").map(String::as_str),
        Some("gpt-5-mini")
    );
    assert_eq!(loaded.provider_fallbacks().primary(), Some("anthropic"));
    assert_eq!(
        loaded.provider_fallbacks().fallbacks(),
        &["openai".to_string(), "dashscope".to_string()]
    );
    assert_eq!(
        loaded.trusted_roots(),
        &["/tmp/worktrees".to_string(), "/srv/octopus".to_string()]
    );
    assert_eq!(loaded.plugins().max_output_tokens(), Some(2048));

    fs::remove_dir_all(root).expect("cleanup temp dir");
}

#[test]
fn split_config_sources_module_discovers_expected_precedence_order() {
    let cwd = std::path::PathBuf::from("/tmp/project");
    let config_home = std::path::PathBuf::from("/tmp/home/.claw");

    let discovered = config_sources::discovered_entries(&cwd, &config_home);

    assert_eq!(discovered.len(), 5);
    assert_eq!(discovered[0].source, ConfigSource::User);
    assert_eq!(discovered[4].source, ConfigSource::Local);
    assert_eq!(discovered[1].path, config_home.join("settings.json"));
}

#[test]
fn split_config_merge_module_merges_nested_maps() {
    let mut target = JsonValue::parse(r#"{"env":{"A":"1"}}"#)
        .expect("target JSON should parse")
        .as_object()
        .expect("target should be an object")
        .clone();
    let source = JsonValue::parse(r#"{"env":{"B":"2"},"model":"haiku"}"#)
        .expect("source JSON should parse")
        .as_object()
        .expect("source should be an object")
        .clone();

    config_merge::deep_merge_objects(&mut target, &source);

    assert_eq!(
        target
            .get("env")
            .and_then(JsonValue::as_object)
            .and_then(|env| env.get("B")),
        Some(&JsonValue::String("2".into()))
    );
    assert_eq!(
        target.get("model"),
        Some(&JsonValue::String("haiku".into()))
    );
}

#[test]
fn split_config_patch_module_preserves_unknown_keys() {
    let mut target = JsonValue::parse(r#"{"custom":{"keep":"yes","drop":"no"}}"#)
        .expect("target JSON should parse")
        .as_object()
        .expect("target should be an object")
        .clone();
    let patch = JsonValue::parse(r#"{"custom":{"drop":null,"add":"ok"}}"#)
        .expect("patch JSON should parse")
        .as_object()
        .expect("patch should be an object")
        .clone();

    config_patch::apply_config_patch(&mut target, &patch);

    assert_eq!(
        target
            .get("custom")
            .and_then(JsonValue::as_object)
            .and_then(|custom| custom.get("keep")),
        Some(&JsonValue::String("yes".into()))
    );
    assert_eq!(
        target
            .get("custom")
            .and_then(JsonValue::as_object)
            .and_then(|custom| custom.get("drop")),
        None
    );
}

#[test]
fn split_config_secrets_module_parses_trusted_roots() {
    let root = JsonValue::parse(r#"{"trustedRoots":["/tmp/worktrees","/srv/octopus"]}"#)
        .expect("root JSON should parse");

    let trusted_roots = config_secrets::parse_optional_trusted_roots(&root).expect("trusted roots");

    assert_eq!(
        trusted_roots,
        vec!["/tmp/worktrees".to_string(), "/srv/octopus".to_string()]
    );
}
