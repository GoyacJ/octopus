use std::collections::BTreeSet;

use octopus_core::{AppError, ExportWorkspaceAgentBundleInput, ExportWorkspaceAgentBundleResult};
use rusqlite::Connection;

use crate::{agent_assets, WorkspacePaths};

pub(crate) fn export_assets(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    target: agent_assets::AssetTargetScope<'_>,
    input: ExportWorkspaceAgentBundleInput,
) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
    let context =
        agent_assets::build_export_context(connection, paths, workspace_id, target, input)?;
    let root_dir_name = context.root_dir_name.clone();
    let mut files = Vec::new();

    for team in &context.teams {
        files.extend(agent_assets::export_team_files(
            paths,
            &context,
            team,
            &root_dir_name,
        )?);
    }

    let team_member_refs = context
        .teams
        .iter()
        .flat_map(|team| team.member_refs.iter().cloned())
        .collect::<BTreeSet<_>>();
    for agent in &context.agents {
        if team_member_refs.contains(&agent.id)
            || team_member_refs.contains(&crate::canonical_agent_ref(&agent.id))
        {
            continue;
        }
        files.extend(agent_assets::export_agent_files(
            paths,
            &context,
            agent,
            None,
            &root_dir_name,
        )?);
    }

    Ok(ExportWorkspaceAgentBundleResult {
        root_dir_name,
        file_count: files.len() as u64,
        agent_count: context.agents.len() as u64,
        team_count: context.teams.len() as u64,
        skill_count: context.skill_paths.len() as u64,
        mcp_count: context.mcp_configs.len() as u64,
        avatar_count: context
            .avatar_payloads
            .values()
            .filter(|payload| payload.is_some())
            .count() as u64,
        bundle_manifest: context.bundle_manifest.clone(),
        translation_report: context.translation_report.clone(),
        files,
        issues: context.issues,
    })
}
