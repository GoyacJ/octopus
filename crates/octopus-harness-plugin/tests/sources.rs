use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use harness_contracts::{PluginId, TrustLevel};
use harness_plugin::{
    DiscoverySource, FileManifestLoader, ManifestLoaderError, ManifestOrigin, Plugin,
    PluginActivationContext, PluginActivationResult, PluginError, PluginManifest,
    PluginManifestLoader, PluginRegistry, PluginRuntimeLoader, RuntimeLoaderError,
    StaticLinkRuntimeLoader,
};

#[tokio::test]
async fn workspace_source_scans_admin_plugin_json() {
    let root = tempfile::tempdir().unwrap();
    write_manifest(
        &root.path().join("data/plugins/admin-a/plugin.json"),
        manifest_json("admin-a", TrustLevel::AdminTrusted),
    );

    let records = FileManifestLoader
        .enumerate(&DiscoverySource::Workspace(root.path().into()))
        .await
        .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].manifest.plugin_id().0, "admin-a@0.1.0");
    assert_eq!(records[0].manifest.trust_level, TrustLevel::AdminTrusted);
    assert!(matches!(records[0].origin, ManifestOrigin::File { .. }));
}

#[tokio::test]
async fn user_source_scans_user_controlled_plugin_json() {
    let home = tempfile::tempdir().unwrap();
    write_manifest(
        &home.path().join(".octopus/plugins/user-a/plugin.json"),
        manifest_json("user-a", TrustLevel::UserControlled),
    );

    let records = FileManifestLoader
        .enumerate(&DiscoverySource::User(home.path().into()))
        .await
        .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].manifest.plugin_id().0, "user-a@0.1.0");
    assert_eq!(records[0].manifest.trust_level, TrustLevel::UserControlled);
}

#[tokio::test]
async fn project_source_scans_project_plugin_json() {
    let project = tempfile::tempdir().unwrap();
    write_manifest(
        &project
            .path()
            .join(".octopus/plugins/project-a/plugin.json"),
        manifest_json("project-a", TrustLevel::UserControlled),
    );

    let records = FileManifestLoader
        .enumerate(&DiscoverySource::Project(project.path().into()))
        .await
        .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].manifest.plugin_id().0, "project-a@0.1.0");
    assert_eq!(records[0].manifest.trust_level, TrustLevel::UserControlled);
}

#[tokio::test]
async fn yaml_manifest_is_parsed_through_file_loader() {
    let root = tempfile::tempdir().unwrap();
    write_manifest(
        &root.path().join("data/plugins/admin-yaml/plugin.yaml"),
        r#"
manifest_schema_version: 1
name: admin-yaml
version: 0.1.0
trust_level: admin_trusted
min_harness_version: ">=0.0.0"
capabilities:
  tools:
    - name: yaml-tool
      destructive: false
"#,
    );

    let records = FileManifestLoader
        .enumerate(&DiscoverySource::Workspace(root.path().into()))
        .await
        .unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].manifest.plugin_id().0, "admin-yaml@0.1.0");
    assert_eq!(records[0].manifest.capabilities.tools[0].name, "yaml-tool");
}

#[tokio::test]
async fn source_trust_mismatch_is_rejected() {
    let home = tempfile::tempdir().unwrap();
    write_manifest(
        &home.path().join(".octopus/plugins/bad-trust/plugin.json"),
        manifest_json("bad-trust", TrustLevel::AdminTrusted),
    );

    let error = FileManifestLoader
        .enumerate(&DiscoverySource::User(home.path().into()))
        .await
        .unwrap_err();

    assert!(matches!(error, ManifestLoaderError::Validation(_)));
}

#[tokio::test]
async fn malformed_manifest_returns_validation_error() {
    let root = tempfile::tempdir().unwrap();
    write_manifest(
        &root.path().join("data/plugins/bad/plugin.json"),
        "{ this is not json",
    );

    let error = FileManifestLoader
        .enumerate(&DiscoverySource::Workspace(root.path().into()))
        .await
        .unwrap_err();

    assert!(matches!(error, ManifestLoaderError::Validation(_)));
}

#[tokio::test]
async fn default_builder_discovers_file_manifests_without_custom_loader() {
    let root = tempfile::tempdir().unwrap();
    write_manifest(
        &root
            .path()
            .join(".octopus/plugins/user-default/plugin.json"),
        manifest_json("user-default", TrustLevel::UserControlled),
    );
    let registry = PluginRegistry::builder()
        .with_source(DiscoverySource::Project(root.path().into()))
        .build()
        .unwrap();

    let discovered = registry.discover().await.unwrap();

    assert_eq!(discovered.len(), 1);
    assert_eq!(
        discovered[0].record.manifest.plugin_id().0,
        "user-default@0.1.0"
    );
}

#[tokio::test]
async fn static_link_runtime_loader_loads_only_during_activate() {
    let root = tempfile::tempdir().unwrap();
    write_manifest(
        &root.path().join(".octopus/plugins/static-a/plugin.json"),
        manifest_json("static-a", TrustLevel::UserControlled),
    );
    let load_count = Arc::new(AtomicUsize::new(0));
    let manifest = manifest("static-a", TrustLevel::UserControlled);
    let plugin: Arc<dyn Plugin> = Arc::new(NoopPlugin { manifest });
    let runtime = StaticLinkRuntimeLoader::default().with_factory(
        PluginId("static-a@0.1.0".to_owned()),
        counting_factory(Arc::clone(&load_count), plugin),
    );
    let registry = PluginRegistry::builder()
        .with_source(DiscoverySource::Project(root.path().into()))
        .with_runtime_loader(Arc::new(runtime))
        .build()
        .unwrap();

    registry.discover().await.unwrap();
    assert_eq!(load_count.load(Ordering::SeqCst), 0);

    registry
        .activate(&PluginId("static-a@0.1.0".to_owned()))
        .await
        .unwrap();
    assert_eq!(load_count.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "dynamic-load")]
#[tokio::test]
async fn dylib_runtime_loader_is_explicitly_unsupported_without_unsafe() {
    use harness_plugin::DylibRuntimeLoader;

    let manifest = manifest("dylib-a", TrustLevel::AdminTrusted);
    let result = DylibRuntimeLoader
        .load(
            &manifest,
            &ManifestOrigin::File {
                path: "/tmp/plugin.dylib".into(),
            },
        )
        .await;
    let Err(error) = result else {
        panic!("dylib loading must be unsupported");
    };

    assert!(
        matches!(error, RuntimeLoaderError::LoadFailed(message) if message.contains("unsupported"))
    );
}

fn write_manifest(path: &std::path::Path, content: impl AsRef<str>) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content.as_ref()).unwrap();
}

fn manifest_json(name: &str, trust_level: TrustLevel) -> String {
    let trust_level = match trust_level {
        TrustLevel::AdminTrusted => "admin_trusted",
        TrustLevel::UserControlled => "user_controlled",
        _ => unreachable!("test only uses known trust levels"),
    };
    format!(
        r#"{{
  "manifest_schema_version": 1,
  "name": "{name}",
  "version": "0.1.0",
  "trust_level": "{trust_level}",
  "min_harness_version": ">=0.0.0",
  "capabilities": {{}}
}}"#
    )
}

fn manifest(name: &str, trust_level: TrustLevel) -> PluginManifest {
    serde_json::from_str(&manifest_json(name, trust_level)).unwrap()
}

fn counting_factory(
    load_count: Arc<AtomicUsize>,
    plugin: Arc<dyn Plugin>,
) -> impl Fn() -> Arc<dyn Plugin> + Send + Sync + 'static {
    move || {
        load_count.fetch_add(1, Ordering::SeqCst);
        Arc::clone(&plugin)
    }
}

struct NoopPlugin {
    manifest: PluginManifest,
}

#[async_trait]
impl Plugin for NoopPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    async fn activate(
        &self,
        _ctx: PluginActivationContext,
    ) -> Result<PluginActivationResult, PluginError> {
        Ok(PluginActivationResult::default())
    }

    async fn deactivate(&self) -> Result<(), PluginError> {
        Ok(())
    }
}
