use std::{fs, path::Path};

use octopus_sdk_plugin::{
    example_bundled_plugins, PluginDiscoveryConfig, PluginError, PluginLifecycle, PluginRegistry,
};
use tempfile::tempdir;

#[test]
fn test_lifecycle_end_to_end() {
    let deny_root = tempdir().expect("tempdir should exist");
    write_manifest(
        deny_root.path(),
        "deny-tool",
        r#"{
  "id": "deny-tool",
  "version": "0.1.0",
  "compat": { "pluginApi": "^1.0.0" },
  "components": []
}"#,
    );

    let config = PluginDiscoveryConfig {
        roots: vec![bundled_root(), deny_root.path().to_path_buf()],
        allow: Vec::new(),
        deny: vec!["deny-tool".into()],
    };
    let mut registry = PluginRegistry::new();
    let plugins = example_bundled_plugins();

    PluginLifecycle::run(&mut registry, &config, &plugins)
        .expect("lifecycle should register bundled plugin");

    let snapshot = registry.get_snapshot();
    assert_eq!(snapshot.plugins.len(), 1);
    assert_eq!(snapshot.plugins[0].id, "example-noop-tool");
    assert_eq!(
        snapshot.plugins[0].source,
        octopus_sdk_contracts::PluginSourceTag::Bundled
    );
    assert!(registry.tools().get("noop-tool").is_some());
    assert!(registry.tools().get("deny-tool").is_none());
}

#[test]
fn test_error_manifest_parse() {
    let root = tempdir().expect("tempdir should exist");
    let plugin_dir = root.path().join("broken-plugin");
    fs::create_dir_all(&plugin_dir).expect("plugin dir should exist");
    fs::write(plugin_dir.join("plugin.json"), "{ invalid json").expect("manifest should write");

    let mut registry = PluginRegistry::new();
    let error = PluginLifecycle::run(
        &mut registry,
        &PluginDiscoveryConfig {
            roots: vec![root.path().to_path_buf()],
            allow: Vec::new(),
            deny: Vec::new(),
        },
        &example_bundled_plugins(),
    )
    .expect_err("invalid manifest should fail");

    assert!(matches!(error, PluginError::ManifestParseError { .. }));
}

#[cfg(unix)]
#[test]
fn test_error_world_writable() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempdir().expect("tempdir should exist");
    let path = write_manifest(
        root.path(),
        "world-writable",
        r#"{
  "id": "world-writable",
  "version": "0.1.0",
  "compat": { "pluginApi": "^1.0.0" },
  "components": []
}"#,
    );
    let mut permissions = fs::metadata(&path)
        .expect("metadata should exist")
        .permissions();
    permissions.set_mode(0o666);
    fs::set_permissions(&path, permissions).expect("permissions should update");

    let mut registry = PluginRegistry::new();
    let error = PluginLifecycle::run(
        &mut registry,
        &PluginDiscoveryConfig {
            roots: vec![root.path().to_path_buf()],
            allow: Vec::new(),
            deny: Vec::new(),
        },
        &example_bundled_plugins(),
    )
    .expect_err("world-writable manifest should fail");

    assert!(matches!(error, PluginError::WorldWritable { .. }));
}

#[test]
fn test_error_path_escape() {
    let root = tempdir().expect("tempdir should exist");
    let plugin_dir = root.path().join("escape-tool");
    let outside_dir = root.path().join("outside");
    fs::create_dir_all(&plugin_dir).expect("plugin dir should exist");
    fs::create_dir_all(&outside_dir).expect("outside dir should exist");
    fs::write(outside_dir.join("agent.md"), "agent").expect("outside file should write");
    fs::write(
        plugin_dir.join("plugin.json"),
        r#"{
  "id": "escape-tool",
  "version": "0.1.0",
  "compat": { "pluginApi": "^1.0.0" },
  "components": [
    {
      "kind": "agent",
      "id": "reviewer",
      "manifest_path": "../outside/agent.md",
      "source": { "kind": "plugin", "plugin_id": "escape-tool" }
    }
  ]
}"#,
    )
    .expect("manifest should write");

    let mut registry = PluginRegistry::new();
    let error = PluginLifecycle::run(
        &mut registry,
        &PluginDiscoveryConfig {
            roots: vec![root.path().to_path_buf()],
            allow: Vec::new(),
            deny: Vec::new(),
        },
        &example_bundled_plugins(),
    )
    .expect_err("path escape should fail");

    assert!(matches!(error, PluginError::PathEscape { .. }));
}

#[test]
fn test_error_duplicate_id() {
    let duplicate_root = tempdir().expect("tempdir should exist");
    write_manifest(
        duplicate_root.path(),
        "duplicate-example",
        r#"{
  "id": "example-noop-tool",
  "version": "0.1.1",
  "compat": { "pluginApi": "^1.0.0" },
  "components": []
}"#,
    );

    let mut registry = PluginRegistry::new();
    let error = PluginLifecycle::run(
        &mut registry,
        &PluginDiscoveryConfig {
            roots: vec![bundled_root(), duplicate_root.path().to_path_buf()],
            allow: Vec::new(),
            deny: Vec::new(),
        },
        &example_bundled_plugins(),
    )
    .expect_err("duplicate id should fail");

    assert_eq!(
        error,
        PluginError::DuplicateId {
            id: "example-noop-tool".into(),
        }
    );
}

fn bundled_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("bundled")
}

fn write_manifest(root: &Path, dir_name: &str, content: &str) -> std::path::PathBuf {
    let plugin_dir = root.join(dir_name);
    fs::create_dir_all(&plugin_dir).expect("plugin dir should exist");
    let path = plugin_dir.join("plugin.json");
    fs::write(&path, content).expect("manifest should write");
    path
}
