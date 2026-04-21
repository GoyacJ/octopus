use std::{fs, path::Path};

use octopus_sdk_contracts::PluginSourceTag;
use octopus_sdk_plugin::{
    default_roots, PluginDiscoveryConfig, PluginError, PluginManifest, SDK_PLUGIN_API_VERSION,
};
use tempfile::tempdir;

#[test]
fn test_parse_minimal() {
    let dir = tempdir().expect("tempdir should exist");
    let path = write_manifest(
        dir.path(),
        &format!(
            r#"{{
  "id": "example-noop-tool",
  "version": "0.1.0",
  "git_sha": "0123456789abcdef0123456789abcdef01234567",
  "source": "local",
  "compat": {{ "pluginApi": "^{}" }},
  "components": [
    {{
      "kind": "tool",
      "id": "noop-tool",
      "name": "noop",
      "description": "No-op tool",
      "schema": {{ "type": "object" }},
      "source": {{ "kind": "plugin", "plugin_id": "example-noop-tool" }}
    }},
    {{
      "kind": "hook",
      "id": "noop-hook",
      "point": "pre_tool_use",
      "source": {{ "kind": "plugin", "plugin_id": "example-noop-tool" }}
    }}
  ]
}}"#,
            SDK_PLUGIN_API_VERSION
        ),
    );

    let manifest = PluginManifest::load_from_path(&path).expect("manifest should parse");

    assert_eq!(manifest.id, "example-noop-tool");
    assert_eq!(manifest.components.len(), 2);
    assert_eq!(manifest.source, PluginSourceTag::Local);
}

#[test]
fn test_compat_mismatch() {
    let dir = tempdir().expect("tempdir should exist");
    let path = write_manifest(
        dir.path(),
        r#"{
  "id": "example-noop-tool",
  "version": "0.1.0",
  "source": "local",
  "compat": { "pluginApi": "^2.0.0" },
  "components": []
}"#,
    );

    let error = PluginManifest::load_from_path(&path).expect_err("compat mismatch should fail");
    assert_eq!(
        error,
        PluginError::IncompatibleApi {
            actual: "1.0.0".into(),
            required: "^2.0.0".into(),
        }
    );
}

#[test]
fn test_non_local_source_rejected() {
    let dir = tempdir().expect("tempdir should exist");
    let path = write_manifest(
        dir.path(),
        r#"{
  "id": "example-noop-tool",
  "version": "0.1.0",
  "source": "bundled",
  "compat": { "pluginApi": "^1.0.0" },
  "components": []
}"#,
    );

    let error = PluginManifest::load_from_path(&path).expect_err("bundled source should fail");
    assert_eq!(
        error,
        PluginError::UnsupportedSource {
            source_kind: "bundled".into(),
        }
    );
}

#[test]
fn test_default_roots_empty() {
    assert!(default_roots().is_empty());
    assert_eq!(
        PluginDiscoveryConfig::default().roots,
        Vec::<std::path::PathBuf>::new()
    );
}

fn write_manifest(root: &Path, content: &str) -> std::path::PathBuf {
    let path = root.join("plugin.json");
    fs::write(&path, content).expect("manifest should write");
    path
}
