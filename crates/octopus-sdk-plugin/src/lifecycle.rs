use std::{collections::BTreeMap, path::Path, sync::Arc};

use octopus_sdk_contracts::{PluginSourceTag, PluginsSnapshot};
use walkdir::WalkDir;

use crate::{PluginApi, PluginDiscoveryConfig, PluginError, PluginManifest, PluginRegistry};

pub trait PluginRuntime: Send + Sync {
    fn register(&self, api: &mut PluginApi<'_>) -> Result<(), PluginError>;
}

pub struct PluginRuntimeCatalog {
    bundled: BTreeMap<String, Arc<dyn PluginRuntime>>,
    local: BTreeMap<String, Arc<dyn PluginRuntime>>,
}

impl Default for PluginRuntimeCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRuntimeCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bundled: BTreeMap::new(),
            local: BTreeMap::new(),
        }
    }

    pub fn register_bundled(
        &mut self,
        plugin_id: impl Into<String>,
        runtime: Arc<dyn PluginRuntime>,
    ) -> Result<(), PluginError> {
        register_runtime(&mut self.bundled, plugin_id.into(), runtime)
    }

    pub fn register_local(
        &mut self,
        plugin_id: impl Into<String>,
        runtime: Arc<dyn PluginRuntime>,
    ) -> Result<(), PluginError> {
        register_runtime(&mut self.local, plugin_id.into(), runtime)
    }

    #[must_use]
    pub fn resolve(&self, manifest: &PluginManifest) -> Option<Arc<dyn PluginRuntime>> {
        match manifest.source {
            PluginSourceTag::Bundled => self.bundled.get(&manifest.id),
            PluginSourceTag::Local => self.local.get(&manifest.id),
        }
        .cloned()
    }
}

pub struct PluginLifecycle;

impl PluginLifecycle {
    pub fn boot(
        config: &PluginDiscoveryConfig,
        runtimes: &PluginRuntimeCatalog,
    ) -> Result<(PluginRegistry, PluginsSnapshot), PluginError> {
        let mut registry = PluginRegistry::new();
        Self::run(&mut registry, config, runtimes)?;
        let snapshot = registry.get_snapshot();
        Ok((registry, snapshot))
    }

    pub fn run(
        registry: &mut PluginRegistry,
        config: &PluginDiscoveryConfig,
        runtimes: &PluginRuntimeCatalog,
    ) -> Result<(), PluginError> {
        for manifest in discover_manifests(config)? {
            let plugin_id = manifest.id.clone();
            registry.register_plugin(manifest.clone(), manifest.source)?;

            let Some(runtime) = runtimes.resolve(&manifest) else {
                continue;
            };

            let mut api = PluginApi::new(registry, &plugin_id)?;
            runtime.register(&mut api)?;
        }

        Ok(())
    }
}

fn discover_manifests(config: &PluginDiscoveryConfig) -> Result<Vec<PluginManifest>, PluginError> {
    let mut paths = Vec::new();

    for root in &config.roots {
        if !root.path.exists() {
            continue;
        }

        for entry in WalkDir::new(&root.path)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file() && entry.file_name() == "plugin.json")
        {
            paths.push((entry.into_path(), root.source));
        }
    }

    paths.sort_by(|left, right| left.0.cmp(&right.0));

    let mut manifests = Vec::new();
    for (path, source) in paths {
        let manifest = PluginManifest::load_from_path_with_source(Path::new(&path), source)?;
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

fn register_runtime(
    slot: &mut BTreeMap<String, Arc<dyn PluginRuntime>>,
    plugin_id: String,
    runtime: Arc<dyn PluginRuntime>,
) -> Result<(), PluginError> {
    if slot.insert(plugin_id.clone(), runtime).is_some() {
        return Err(PluginError::DuplicateId { id: plugin_id });
    }

    Ok(())
}
