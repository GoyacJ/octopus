use std::path::PathBuf;

use harness_contracts::PluginId;

use crate::{
    DirectorySourceKind, LoadReport, SkillError, SkillLoader, SkillPlatform, SkillSourceConfig,
};

#[derive(Debug, Clone)]
pub struct PluginSource {
    plugin_id: PluginId,
    plugin_root: PathBuf,
}

impl PluginSource {
    #[must_use]
    pub fn new(plugin_id: PluginId, plugin_root: PathBuf) -> Self {
        Self {
            plugin_id,
            plugin_root,
        }
    }

    pub async fn load(&self, runtime_platform: SkillPlatform) -> Result<LoadReport, SkillError> {
        SkillLoader::default()
            .with_source(SkillSourceConfig::Directory {
                path: self.plugin_root.join("skills"),
                source_kind: DirectorySourceKind::Plugin(self.plugin_id.clone()),
            })
            .with_runtime_platform(runtime_platform)
            .load_all()
            .await
    }
}
