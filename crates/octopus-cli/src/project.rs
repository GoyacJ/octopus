#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRecord {
    pub id: String,
    pub version: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginsCommandResult {
    pub message: String,
    pub reload_runtime: bool,
}

#[must_use]
pub fn render_plugins_report(plugins: &[PluginRecord]) -> String {
    let mut lines = vec!["Plugins".to_string()];
    if plugins.is_empty() {
        lines.push("  No plugins installed.".to_string());
        return lines.join("\n");
    }
    for plugin in plugins {
        let enabled = if plugin.enabled {
            "enabled"
        } else {
            "disabled"
        };
        lines.push(format!(
            "  {name:<20} v{version:<10} {enabled}",
            name = plugin.id,
            version = plugin.version,
        ));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{render_plugins_report, PluginRecord};

    #[test]
    fn renders_plugin_report() {
        let rendered = render_plugins_report(&[PluginRecord {
            id: "demo".into(),
            version: "1.0.0".into(),
            enabled: true,
        }]);
        assert!(rendered.contains("demo"));
        assert!(rendered.contains("enabled"));
    }
}
