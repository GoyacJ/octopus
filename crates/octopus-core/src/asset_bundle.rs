use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::{AssetBundleManifestV2, AssetTranslationReport, WorkspaceDirectoryUploadEntry};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceAgentBundlePreviewInput {
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceAgentBundleInput {
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportIssue {
    pub severity: String,
    pub scope: String,
    pub code: String,
    pub stage: String,
    pub source_id: Option<String>,
    pub source_path: Option<String>,
    pub dependency_ref: Option<String>,
    pub asset_kind: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
    pub details: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAgentPreviewItem {
    pub source_id: String,
    pub agent_id: Option<String>,
    pub name: String,
    pub department: String,
    pub action: String,
    pub manifest_revision: String,
    pub skill_slugs: Vec<String>,
    pub mcp_server_names: Vec<String>,
    pub task_domains: Vec<String>,
    pub translation_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedTeamPreviewItem {
    pub source_id: String,
    pub team_id: Option<String>,
    pub name: String,
    pub action: String,
    pub leader_name: Option<String>,
    pub member_names: Vec<String>,
    pub agent_source_ids: Vec<String>,
    pub manifest_revision: String,
    pub task_domains: Vec<String>,
    pub translation_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedSkillPreviewItem {
    pub slug: String,
    pub skill_id: String,
    pub name: String,
    pub action: String,
    pub content_hash: String,
    pub file_count: u64,
    pub source_ids: Vec<String>,
    pub departments: Vec<String>,
    pub agent_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedMcpPreviewItem {
    pub server_name: String,
    pub action: String,
    pub content_hash: Option<String>,
    pub source_ids: Vec<String>,
    pub consumer_names: Vec<String>,
    pub referenced_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedAvatarPreviewItem {
    pub source_id: String,
    pub owner_kind: String,
    pub owner_name: String,
    pub file_name: String,
    pub generated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceAgentBundlePreview {
    pub departments: Vec<String>,
    pub bundle_manifest: AssetBundleManifestV2,
    pub department_count: u64,
    pub detected_agent_count: u64,
    pub importable_agent_count: u64,
    pub detected_team_count: u64,
    pub importable_team_count: u64,
    pub create_count: u64,
    pub update_count: u64,
    pub skip_count: u64,
    pub failure_count: u64,
    pub unique_skill_count: u64,
    pub unique_mcp_count: u64,
    pub agent_count: u64,
    pub team_count: u64,
    pub skill_count: u64,
    pub mcp_count: u64,
    pub avatar_count: u64,
    pub filtered_file_count: u64,
    pub agents: Vec<ImportedAgentPreviewItem>,
    pub teams: Vec<ImportedTeamPreviewItem>,
    pub skills: Vec<ImportedSkillPreviewItem>,
    pub mcps: Vec<ImportedMcpPreviewItem>,
    pub avatars: Vec<ImportedAvatarPreviewItem>,
    pub issues: Vec<ImportIssue>,
    pub translation_report: AssetTranslationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportWorkspaceAgentBundleResult {
    pub departments: Vec<String>,
    pub bundle_manifest: AssetBundleManifestV2,
    pub department_count: u64,
    pub detected_agent_count: u64,
    pub importable_agent_count: u64,
    pub detected_team_count: u64,
    pub importable_team_count: u64,
    pub create_count: u64,
    pub update_count: u64,
    pub skip_count: u64,
    pub failure_count: u64,
    pub unique_skill_count: u64,
    pub unique_mcp_count: u64,
    pub agent_count: u64,
    pub team_count: u64,
    pub skill_count: u64,
    pub mcp_count: u64,
    pub avatar_count: u64,
    pub filtered_file_count: u64,
    pub agents: Vec<ImportedAgentPreviewItem>,
    pub teams: Vec<ImportedTeamPreviewItem>,
    pub skills: Vec<ImportedSkillPreviewItem>,
    pub mcps: Vec<ImportedMcpPreviewItem>,
    pub avatars: Vec<ImportedAvatarPreviewItem>,
    pub issues: Vec<ImportIssue>,
    pub translation_report: AssetTranslationReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportWorkspaceAgentBundleInput {
    pub mode: String,
    pub agent_ids: Vec<String>,
    pub team_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportWorkspaceAgentBundleResult {
    pub root_dir_name: String,
    pub file_count: u64,
    pub agent_count: u64,
    pub team_count: u64,
    pub skill_count: u64,
    pub mcp_count: u64,
    pub avatar_count: u64,
    pub bundle_manifest: AssetBundleManifestV2,
    pub translation_report: AssetTranslationReport,
    pub files: Vec<WorkspaceDirectoryUploadEntry>,
    pub issues: Vec<ImportIssue>,
}
