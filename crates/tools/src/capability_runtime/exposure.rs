use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CapabilityExposureSnapshot {
    discovered: BTreeSet<String>,
    activated: BTreeSet<String>,
    exposed: BTreeSet<String>,
}

impl CapabilityExposureSnapshot {
    pub fn discover_tool(&mut self, tool_name: impl Into<String>) {
        self.discovered.insert(tool_name.into());
    }

    pub fn activate_tool(&mut self, tool_name: impl Into<String>) {
        let tool_name = tool_name.into();
        self.discovered.insert(tool_name.clone());
        self.activated.insert(tool_name);
    }

    pub fn expose_tool(&mut self, tool_name: impl Into<String>) {
        let tool_name = tool_name.into();
        self.discovered.insert(tool_name.clone());
        self.activated.insert(tool_name.clone());
        self.exposed.insert(tool_name);
    }

    #[must_use]
    pub fn is_tool_discovered(&self, tool_name: &str) -> bool {
        self.discovered.contains(tool_name)
    }

    #[must_use]
    pub fn is_tool_activated(&self, tool_name: &str) -> bool {
        self.activated.contains(tool_name)
    }

    #[must_use]
    pub fn is_tool_exposed(&self, tool_name: &str) -> bool {
        self.exposed.contains(tool_name)
    }

    #[must_use]
    pub fn discovered_tools(&self) -> &BTreeSet<String> {
        &self.discovered
    }

    #[must_use]
    pub fn activated_tools(&self) -> &BTreeSet<String> {
        &self.activated
    }

    #[must_use]
    pub fn exposed_tools(&self) -> &BTreeSet<String> {
        &self.exposed
    }
}
