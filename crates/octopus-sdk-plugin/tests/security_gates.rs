use std::{fs, path::Path};

use octopus_sdk_contracts::{DeclSource, PluginSourceTag};
use octopus_sdk_plugin::{
    ensure_not_world_writable, ensure_path_within_root, validate_plugin_id, AgentDecl,
    PluginCompat, PluginComponent, PluginError, PluginManifest,
};
use tempfile::{tempdir, tempdir_in};

#[test]
fn test_path_escape_rejected() {
    let root = tempdir().expect("tempdir should exist");
    let outside = tempdir_in(root.path().parent().expect("tempdir parent should exist"))
        .expect("outside tempdir should exist");
    let escaped = outside.path().join("agent.md");
    fs::write(&escaped, "agent").expect("outside file should write");

    let relative = Path::new("..")
        .join(
            outside
                .path()
                .file_name()
                .expect("outside dir should have file name"),
        )
        .join("agent.md");
    let error = ensure_path_within_root(root.path(), &relative).expect_err("path should escape");

    assert!(matches!(error, PluginError::PathEscape { .. }));
}

#[test]
fn test_world_writable_rejected() {
    let root = tempdir().expect("tempdir should exist");
    let path = root.path().join("plugin.json");
    fs::write(&path, "{}").expect("manifest should write");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&path)
            .expect("metadata should exist")
            .permissions();
        permissions.set_mode(0o666);
        fs::set_permissions(&path, permissions).expect("permissions should update");

        let error = ensure_not_world_writable(&path).expect_err("world-writable file should fail");
        assert_eq!(error, PluginError::WorldWritable { path });
    }

    #[cfg(not(unix))]
    {
        assert!(ensure_not_world_writable(&path).is_ok());
    }
}

#[test]
fn test_invalid_name_rejected() {
    let error = validate_plugin_id("插件").expect_err("non-ascii plugin id should fail");
    assert_eq!(
        error,
        PluginError::ManifestValidationError {
            cause: "invalid plugin id".into(),
        }
    );
}

#[test]
fn test_manifest_security_checks_agent_paths() {
    let root = tempdir().expect("tempdir should exist");
    let plugin_dir = root.path();
    let nested = plugin_dir.join("agents");
    fs::create_dir_all(&nested).expect("agents dir should create");
    fs::write(nested.join("reviewer.md"), "agent").expect("agent file should write");
    let manifest_path = plugin_dir.join("plugin.json");
    fs::write(&manifest_path, "{}").expect("manifest should write");

    let manifest = PluginManifest {
        id: "example-noop-tool".into(),
        version: "0.1.0".into(),
        git_sha: None,
        source: PluginSourceTag::Local,
        compat: PluginCompat {
            plugin_api: "^1.0.0".into(),
        },
        components: vec![PluginComponent::Agent(AgentDecl {
            id: "reviewer".into(),
            manifest_path: Path::new("agents/reviewer.md").into(),
            source: DeclSource::Plugin {
                plugin_id: "example-noop-tool".into(),
            },
        })],
    };

    manifest
        .validate(&manifest_path)
        .expect("manifest security validation should pass");
}
