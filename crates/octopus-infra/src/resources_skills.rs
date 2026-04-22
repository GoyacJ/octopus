use super::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use octopus_core::{
    CapabilityAssetManifest, CapabilityManagementEntry, CapabilityManagementProjection,
    McpServerPackageManifest, SkillPackageManifest, WorkspaceToolCatalogEntry,
    WorkspaceToolConsumerSummary,
};
use octopus_sdk_mcp::{
    discover_mcp_server_capabilities_best_effort, mcp_endpoint as sdk_mcp_endpoint,
    parse_mcp_server_config, parse_mcp_servers, qualified_mcp_resource_name,
    DiscoveredMcpServerCapabilities, McpServerConfig,
};
use octopus_sdk_tools::{builtin_tool_catalog, BuiltinToolPermission};

use crate::{
    agent_assets::BuiltinSkillAsset,
    agent_bundle::{
        find_builtin_mcp_asset, find_builtin_skill_asset_by_id, list_builtin_agent_templates,
        list_builtin_mcp_assets, list_builtin_skill_assets, list_builtin_team_templates,
    },
};

const BUILTIN_SKILL_SOURCE_ORIGIN: &str = "builtin_bundle";
const REQUIRED_CONFIGURED_MODEL_FIELDS: &[&str] = &["providerId", "modelId", "name"];

#[path = "resources_skills/catalog.rs"]
mod catalog;
#[path = "resources_skills/mcp_catalog.rs"]
mod mcp_catalog;
#[path = "resources_skills/runtime_state.rs"]
mod runtime_state;
#[path = "resources_skills/service.rs"]
mod service;
#[path = "resources_skills/skill_documents.rs"]
mod skill_documents;

pub(crate) use catalog::*;
pub(crate) use mcp_catalog::*;
pub(crate) use runtime_state::*;
pub(crate) use skill_documents::*;

#[cfg(test)]
mod tests;
