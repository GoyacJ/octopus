use std::{collections::BTreeSet, fs};

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

    let team_member_ids = context
        .teams
        .iter()
        .flat_map(|team| team.member_agent_ids.iter().cloned())
        .collect::<BTreeSet<_>>();
    for agent in &context.agents {
        if team_member_ids.contains(&agent.id) {
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

    for descriptor in &context.descriptors {
        let absolute_path = paths.root.join(&descriptor.storage_path);
        if !absolute_path.is_file() {
            continue;
        }
        files.push(agent_assets::encode_file(
            &format!("{root_dir_name}/{}", descriptor.source_path),
            agent_assets::content_type_for_export(&descriptor.source_path),
            fs::read(absolute_path)?,
        ));
    }

    files.push(agent_assets::encode_file(
        &format!("{root_dir_name}/.octopus/manifest.json"),
        "application/json",
        serde_json::to_vec_pretty(&context.bundle_manifest)?,
    ));
    if !context.asset_state.is_empty() {
        files.push(agent_assets::encode_file(
            &format!("{root_dir_name}/{}", agent_assets::BUNDLE_ASSET_STATE_PATH),
            "application/json",
            serde_json::to_vec_pretty(&context.asset_state)?,
        ));
    }

    Ok(ExportWorkspaceAgentBundleResult {
        root_dir_name,
        file_count: files.len() as u64,
        agent_count: context.agents.len() as u64,
        team_count: context.teams.len() as u64,
        skill_count: context.skill_paths.len() as u64,
        mcp_count: context.mcp_configs.len() as u64,
        avatar_count: context.avatar_payloads.len() as u64,
        bundle_manifest: context.bundle_manifest.clone(),
        translation_report: context.translation_report.clone(),
        files,
        issues: context.issues,
    })
}
