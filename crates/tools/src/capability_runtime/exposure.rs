use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CapabilityExposureSnapshot {
    discovered_tools: BTreeSet<String>,
    activated_tools: BTreeSet<String>,
    exposed_tools: BTreeSet<String>,
}

impl CapabilityExposureSnapshot {
    pub fn discover_tool(&mut self, tool_name: impl Into<String>) {
        self.discovered_tools.insert(tool_name.into());
    }

    pub fn activate_tool(&mut self, tool_name: impl Into<String>) {
        let tool_name = tool_name.into();
        self.discovered_tools.insert(tool_name.clone());
        self.activated_tools.insert(tool_name);
    }

    pub fn expose_tool(&mut self, tool_name: impl Into<String>) {
        let tool_name = tool_name.into();
        self.discovered_tools.insert(tool_name.clone());
        self.activated_tools.insert(tool_name.clone());
        self.exposed_tools.insert(tool_name);
    }

    #[must_use]
    pub fn is_tool_discovered(&self, tool_name: &str) -> bool {
        self.discovered_tools.contains(tool_name)
    }

    #[must_use]
    pub fn is_tool_activated(&self, tool_name: &str) -> bool {
        self.activated_tools.contains(tool_name)
    }

    #[must_use]
    pub fn is_tool_exposed(&self, tool_name: &str) -> bool {
        self.exposed_tools.contains(tool_name)
    }

    #[must_use]
    pub fn discovered_tools(&self) -> &BTreeSet<String> {
        &self.discovered_tools
    }

    #[must_use]
    pub fn activated_tools(&self) -> &BTreeSet<String> {
        &self.activated_tools
    }

    #[must_use]
    pub fn exposed_tools(&self) -> &BTreeSet<String> {
        &self.exposed_tools
    }
}
