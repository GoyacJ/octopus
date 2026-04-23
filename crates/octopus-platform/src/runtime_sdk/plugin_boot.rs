use std::{collections::BTreeSet, env, path::Path};

use octopus_core::AppError;
use octopus_sdk::{
    PluginDiscoveryConfig, PluginDiscoveryRoot, PluginLifecycle, PluginRegistry, PluginsSnapshot,
};
use octopus_sdk_plugin::{bundled_plugin_root, bundled_runtime_catalog};

pub(crate) fn boot_live_plugins(
    workspace_root: &Path,
) -> Result<(PluginRegistry, PluginsSnapshot), AppError> {
    let config = live_plugin_discovery_config(workspace_root);
    let runtimes = bundled_runtime_catalog();
    PluginLifecycle::boot(&config, &runtimes).map_err(|error| AppError::runtime(error.to_string()))
}

fn live_plugin_discovery_config(workspace_root: &Path) -> PluginDiscoveryConfig {
    PluginDiscoveryConfig {
        roots: live_plugin_roots(workspace_root),
        allow: Vec::new(),
        deny: Vec::new(),
    }
}

fn live_plugin_roots(workspace_root: &Path) -> Vec<PluginDiscoveryRoot> {
    let mut local_roots = BTreeSet::new();
    local_roots.insert(workspace_root.join(".octopus/plugins"));

    if let Some(home) = env::var_os("HOME") {
        local_roots.insert(std::path::PathBuf::from(home).join(".octopus/plugins"));
    }

    let mut roots = vec![PluginDiscoveryRoot::bundled(bundled_plugin_root())];
    roots.extend(local_roots.into_iter().map(PluginDiscoveryRoot::local));
    roots
}

#[cfg(test)]
mod tests {
    use super::live_plugin_roots;
    use octopus_sdk::PluginSourceTag;

    #[test]
    fn live_plugin_roots_include_bundled_and_workspace_paths() {
        let workspace = tempfile::tempdir().expect("tempdir should exist");
        let roots = live_plugin_roots(workspace.path());

        assert!(
            roots.iter().any(|root| {
                root.source == PluginSourceTag::Bundled
                    && root.path.ends_with("octopus-sdk-plugin/bundled")
            }),
            "platform live plugin roots should include bundled plugin fixtures"
        );
        assert!(
            roots.iter().any(|root| {
                root.source == PluginSourceTag::Local
                    && root.path == workspace.path().join(".octopus/plugins")
            }),
            "platform live plugin roots should include workspace-local plugins"
        );
    }
}
