use std::{
    fs,
    time::{Duration, Instant},
};

use octopus_sdk_plugin::{PluginManifest, PluginRegistry};
use tempfile::tempdir;

#[test]
fn test_manifest_parse_no_side_effect() {
    let root = tempdir().expect("tempdir should create");
    let plugin_dir = root.path().join("parse-only");
    let manifest_path = plugin_dir.join("plugin.json");
    let registry = PluginRegistry::new();

    fs::create_dir_all(&plugin_dir).expect("plugin dir should exist");
    fs::write(
        &manifest_path,
        r#"{
  "id": "parse-only",
  "version": "0.1.0",
  "compat": { "pluginApi": "^1.0.0" },
  "components": []
}"#,
    )
    .expect("manifest should write");

    let started_at = Instant::now();
    let manifest = PluginManifest::load_from_path(&manifest_path).expect("manifest should parse");
    let elapsed = started_at.elapsed();

    assert_eq!(manifest.id, "parse-only");
    assert!(
        elapsed < Duration::from_millis(10),
        "parse+validate took {:?}, expected < 10ms",
        elapsed
    );
    assert!(registry.get_snapshot().plugins.is_empty());
    assert!(registry.tools().get("parse-only").is_none());
}
