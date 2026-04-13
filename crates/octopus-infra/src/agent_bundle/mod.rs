use octopus_core::{
    AppError, ExportWorkspaceAgentBundleInput, ExportWorkspaceAgentBundleResult,
    ImportWorkspaceAgentBundleInput, ImportWorkspaceAgentBundlePreview,
    ImportWorkspaceAgentBundlePreviewInput, ImportWorkspaceAgentBundleResult,
    WorkspaceDirectoryUploadEntry,
};
use rusqlite::Connection;

use crate::{agent_assets, WorkspacePaths};

pub(crate) mod builtin;
pub(crate) mod export;
pub(crate) mod import;
pub(crate) mod manifest_v2;
pub(crate) mod seed;
pub(crate) mod shared;
pub(crate) mod translation;

#[derive(Debug, Clone, Copy)]
pub(crate) enum BundleTarget<'a> {
    Workspace,
    Project(&'a str),
}

pub(crate) fn target_scope(target: BundleTarget<'_>) -> agent_assets::AssetTargetScope<'_> {
    match target {
        BundleTarget::Workspace => agent_assets::AssetTargetScope::Workspace,
        BundleTarget::Project(project_id) => agent_assets::AssetTargetScope::Project(project_id),
    }
}

pub(crate) fn preview_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: BundleTarget<'_>,
    input: ImportWorkspaceAgentBundlePreviewInput,
) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
    import::preview_import(connection, paths, workspace_id, target_scope(target), input)
}

pub(crate) fn execute_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: BundleTarget<'_>,
    input: ImportWorkspaceAgentBundleInput,
) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
    import::execute_import(connection, paths, workspace_id, target_scope(target), input)
}

pub(crate) fn export_assets(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: BundleTarget<'_>,
    input: ExportWorkspaceAgentBundleInput,
) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
    export::export_assets(connection, paths, workspace_id, target_scope(target), input)
}

pub(crate) fn extract_builtin_agent_template_files(
    agent_id: &str,
) -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, AppError> {
    builtin::extract_builtin_agent_template_files(agent_id)
}

pub(crate) fn extract_builtin_team_template_files(
    team_id: &str,
) -> Result<Option<Vec<WorkspaceDirectoryUploadEntry>>, AppError> {
    builtin::extract_builtin_team_template_files(team_id)
}

pub(crate) fn list_builtin_skill_assets() -> Result<Vec<agent_assets::BuiltinSkillAsset>, AppError>
{
    builtin::list_builtin_skill_assets()
}

pub(crate) fn find_builtin_skill_asset_by_id(
    skill_id: &str,
) -> Result<Option<agent_assets::BuiltinSkillAsset>, AppError> {
    builtin::find_builtin_skill_asset_by_id(skill_id)
}

pub(crate) fn list_builtin_agent_templates(
    workspace_id: &str,
) -> Result<Vec<octopus_core::AgentRecord>, AppError> {
    builtin::list_builtin_agent_templates(workspace_id)
}

pub(crate) fn list_builtin_team_templates(
    workspace_id: &str,
) -> Result<Vec<octopus_core::TeamRecord>, AppError> {
    builtin::list_builtin_team_templates(workspace_id)
}

pub(crate) fn list_builtin_mcp_assets() -> Result<Vec<agent_assets::BuiltinMcpAsset>, AppError> {
    builtin::list_builtin_mcp_assets()
}

pub(crate) fn find_builtin_mcp_asset(
    server_name: &str,
) -> Result<Option<agent_assets::BuiltinMcpAsset>, AppError> {
    builtin::find_builtin_mcp_asset(server_name)
}

#[cfg(test)]
mod tests {
    #[test]
    fn bundle_target_maps_to_agent_asset_scope() {
        assert!(matches!(
            super::target_scope(super::BundleTarget::Workspace),
            crate::agent_assets::AssetTargetScope::Workspace
        ));
        assert!(matches!(
            super::target_scope(super::BundleTarget::Project("proj-local")),
            crate::agent_assets::AssetTargetScope::Project("proj-local")
        ));
    }
}
