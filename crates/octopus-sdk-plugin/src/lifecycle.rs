use std::{collections::BTreeMap, path::Path};

use octopus_sdk_contracts::PluginSourceTag;
use walkdir::WalkDir;

use crate::{PluginApi, PluginDiscoveryConfig, PluginError, PluginManifest, PluginRegistry};

pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;

    fn source(&self) -> PluginSourceTag {
        PluginSourceTag::Local
    }

    fn register(&self, api: &mut PluginApi<'_>) -> Result<(), PluginError>;
}

pub struct PluginLifecycle;

impl PluginLifecycle {
    pub fn run(
        registry: &mut PluginRegistry,
        config: &PluginDiscoveryConfig,
        plugins: &[Box<dyn Plugin>],
    ) -> Result<(), PluginError> {
        let loaded_plugins = plugin_map(plugins)?;

        for manifest in discover_manifests(config)? {
            let plugin_id = manifest.id.clone();
            let Some(plugin) = loaded_plugins.get(plugin_id.as_str()) else {
                continue;
            };

            registry.register_plugin(manifest, plugin.source())?;
            let mut api = PluginApi::new(registry, &plugin_id)?;
            plugin.register(&mut api)?;
        }

        Ok(())
    }
}

fn plugin_map(plugins: &[Box<dyn Plugin>]) -> Result<BTreeMap<String, &dyn Plugin>, PluginError> {
    let mut loaded = BTreeMap::new();

    for plugin in plugins {
        let plugin_id = plugin.manifest().id.clone();
        if loaded.insert(plugin_id.clone(), plugin.as_ref()).is_some() {
            return Err(PluginError::DuplicateId { id: plugin_id });
        }
    }

    Ok(loaded)
}

fn discover_manifests(config: &PluginDiscoveryConfig) -> Result<Vec<PluginManifest>, PluginError> {
    let mut paths = Vec::new();

    for root in &config.roots {
        if !root.exists() {
            continue;
        }

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file() && entry.file_name() == "plugin.json")
        {
            paths.push(entry.into_path());
        }
    }

    paths.sort();

    let mut manifests = Vec::new();
    for path in paths {
        let manifest = PluginManifest::load_from_path(Path::new(&path))?;
        if config.deny.iter().any(|id| id == &manifest.id) {
            continue;
        }
        if !config.allow.is_empty() && !config.allow.iter().any(|id| id == &manifest.id) {
            continue;
        }
        manifests.push(manifest);
    }

    Ok(manifests)
}
