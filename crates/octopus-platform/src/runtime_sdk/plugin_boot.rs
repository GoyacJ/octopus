use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};

use octopus_core::AppError;
use octopus_sdk::{PluginDiscoveryConfig, PluginLifecycle, PluginRegistry, PluginsSnapshot};
use octopus_sdk_plugin::example_bundled_plugins;

pub(crate) fn boot_live_plugins(
    workspace_root: &Path,
) -> Result<(PluginRegistry, PluginsSnapshot), AppError> {
    let mut registry = PluginRegistry::new();
    let config = live_plugin_discovery_config(workspace_root);
    let plugins = example_bundled_plugins();

    PluginLifecycle::run(&mut registry, &config, &plugins)
        .map_err(|error| AppError::runtime(error.to_string()))?;

    let snapshot = registry.get_snapshot();
    Ok((registry, snapshot))
}

fn live_plugin_discovery_config(workspace_root: &Path) -> PluginDiscoveryConfig {
    PluginDiscoveryConfig {
        roots: live_plugin_roots(workspace_root),
        allow: Vec::new(),
        deny: Vec::new(),
    }
}

fn live_plugin_roots(workspace_root: &Path) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();
    roots.insert(bundled_plugin_root());
    roots.insert(workspace_root.join(".octopus/plugins"));

    if let Some(home) = env::var_os("HOME") {
        roots.insert(PathBuf::from(home).join(".octopus/plugins"));
    }

    roots.into_iter().collect()
}

fn bundled_plugin_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../octopus-sdk-plugin/bundled")
}

#[cfg(test)]
mod tests {
    use super::live_plugin_roots;

    #[test]
    fn live_plugin_roots_include_bundled_and_workspace_paths() {
        let workspace = tempfile::tempdir().expect("tempdir should exist");
        let roots = live_plugin_roots(workspace.path());

        assert!(
            roots
                .iter()
                .any(|path| path.ends_with("octopus-sdk-plugin/bundled")),
            "platform live plugin roots should include bundled plugin fixtures"
        );
        assert!(
            roots
                .iter()
                .any(|path| path == &workspace.path().join(".octopus/plugins")),
            "platform live plugin roots should include workspace-local plugins"
        );
    }
}
