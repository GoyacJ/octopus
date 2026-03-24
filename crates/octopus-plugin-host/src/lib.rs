//! Plugin host boundary placeholders.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginManifestRef {
    pub plugin_id: String,
    pub version: String,
}
