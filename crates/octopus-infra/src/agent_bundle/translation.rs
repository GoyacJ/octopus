use octopus_core::{
    normalize_task_domains, AppError, AssetTranslationDiagnostic, AssetTranslationReport,
    ImportIssue, ImportWorkspaceAgentBundlePreview, ImportedAgentPreviewItem,
    ImportedAvatarPreviewItem, ImportedMcpPreviewItem, ImportedSkillPreviewItem,
    ImportedTeamPreviewItem, ASSET_MANIFEST_REVISION_V2,
};

use crate::agent_assets::{BundlePlan, ImportAction};

use super::manifest_v2;

pub(crate) fn plan_to_preview(plan: &BundlePlan) -> ImportWorkspaceAgentBundlePreview {
    let mut create_count = 0_u64;
    let mut update_count = 0_u64;
    let mut skip_count = 0_u64;
    for action in plan
        .skills
        .iter()
        .map(|item| item.action)
        .chain(plan.mcps.iter().map(|item| item.action))
        .chain(plan.agents.iter().map(|item| item.action))
        .chain(plan.teams.iter().map(|item| item.action))
        .chain(plan.descriptor_assets.iter().map(|item| item.action))
    {
        increment_action_counts(
            action,
            &mut create_count,
            &mut update_count,
            &mut skip_count,
        );
    }

    ImportWorkspaceAgentBundlePreview {
        departments: plan.departments.clone(),
        bundle_manifest: manifest_v2::bundle_manifest_from_plan(plan),
        department_count: plan.departments.len() as u64,
        detected_agent_count: plan.detected_agent_count,
        importable_agent_count: plan.agents.len() as u64,
        detected_team_count: plan.detected_team_count,
        importable_team_count: plan.teams.len() as u64,
        create_count,
        update_count,
        skip_count,
        failure_count: plan
            .issues
            .iter()
            .filter(|item| item.severity == crate::agent_assets::ISSUE_ERROR)
            .count() as u64,
        unique_skill_count: plan.skills.len() as u64,
        unique_mcp_count: plan.mcps.len() as u64,
        agent_count: plan.agents.len() as u64,
        team_count: plan.teams.len() as u64,
        skill_count: plan.skills.len() as u64,
        mcp_count: plan.mcps.len() as u64,
        avatar_count: plan.avatars.len() as u64,
        filtered_file_count: plan.filtered_file_count,
        agents: plan
            .agents
            .iter()
            .map(|agent| ImportedAgentPreviewItem {
                source_id: agent.source_id.clone(),
                agent_id: agent.agent_id.clone(),
                name: agent.name.clone(),
                department: agent.department.clone(),
                action: agent.action.as_str().into(),
                manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
                skill_slugs: agent.skill_slugs.clone(),
                mcp_server_names: agent.mcp_server_names.clone(),
                task_domains: normalize_task_domains(agent.tags.clone()),
                translation_mode: import_action_translation_mode(agent.action).into(),
            })
            .collect(),
        teams: plan
            .teams
            .iter()
            .map(|team| ImportedTeamPreviewItem {
                source_id: team.source_id.clone(),
                team_id: team.team_id.clone(),
                name: team.name.clone(),
                action: team.action.as_str().into(),
                leader_name: team.leader_name.clone(),
                member_names: team.member_names.clone(),
                agent_source_ids: team.agent_source_ids.clone(),
                manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
                task_domains: normalize_task_domains(team.tags.clone()),
                translation_mode: import_action_translation_mode(team.action).into(),
            })
            .collect(),
        skills: plan
            .skills
            .iter()
            .map(|skill| ImportedSkillPreviewItem {
                slug: skill.slug.clone(),
                skill_id: skill.skill_id.clone(),
                name: skill.name.clone(),
                action: skill.action.as_str().into(),
                content_hash: skill.content_hash.clone(),
                file_count: skill.file_count as u64,
                source_ids: skill.source_ids.clone(),
                departments: Vec::new(),
                agent_names: skill.consumer_names.clone(),
            })
            .collect(),
        mcps: plan
            .mcps
            .iter()
            .map(|mcp| ImportedMcpPreviewItem {
                server_name: mcp.server_name.clone(),
                action: mcp.action.as_str().into(),
                content_hash: mcp.content_hash.clone(),
                source_ids: mcp.source_ids.clone(),
                consumer_names: mcp.consumer_names.clone(),
                referenced_only: mcp.referenced_only,
            })
            .collect(),
        avatars: plan
            .avatars
            .iter()
            .map(|avatar| ImportedAvatarPreviewItem {
                source_id: avatar.source_id.clone(),
                owner_kind: avatar.owner_kind.clone(),
                owner_name: avatar.owner_name.clone(),
                file_name: avatar.file_name.clone(),
                generated: avatar.generated,
            })
            .collect(),
        issues: plan.issues.clone(),
        translation_report: translation_report_from_issues(
            plan,
            &plan.issues,
            plan.dependency_resolution.clone(),
        ),
    }
}

pub(crate) fn translation_report_from_issues(
    plan: &BundlePlan,
    issues: &[ImportIssue],
    dependencies: Vec<octopus_core::AssetDependencyResolution>,
) -> AssetTranslationReport {
    let diagnostics = issues
        .iter()
        .map(|issue| AssetTranslationDiagnostic {
            severity: issue.severity.clone(),
            code: issue.code.clone(),
            stage: issue.stage.clone(),
            asset_kind: issue.asset_kind.clone(),
            asset_id: issue.source_id.clone(),
            source_path: issue.source_path.clone(),
            dependency_ref: issue.dependency_ref.clone(),
            message: issue.message.clone(),
            suggestion: issue.suggestion.clone(),
            details: issue.details.clone(),
        })
        .collect::<Vec<_>>();
    let rejected_count = plan
        .agents
        .iter()
        .filter(|item| item.action == ImportAction::Failed)
        .count() as u64
        + plan
            .teams
            .iter()
            .filter(|item| item.action == ImportAction::Failed)
            .count() as u64
        + plan
            .skills
            .iter()
            .filter(|item| item.action == ImportAction::Failed)
            .count() as u64
        + plan
            .mcps
            .iter()
            .filter(|item| item.action == ImportAction::Failed)
            .count() as u64;
    let downgraded_count = 0;
    let translated_count = 0;
    let trust_warnings = issues
        .iter()
        .filter(|issue| issue.severity == crate::agent_assets::ISSUE_WARNING)
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    let unsupported_features = issues
        .iter()
        .filter(|issue| issue.code.contains("unsupported"))
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    let status = if rejected_count > 0 {
        "rejected"
    } else if downgraded_count > 0 {
        "downgraded"
    } else if translated_count > 0 {
        "translated"
    } else {
        "native"
    };
    AssetTranslationReport {
        status: status.into(),
        translated_count,
        downgraded_count,
        rejected_count,
        unsupported_features,
        trust_warnings,
        dependency_resolution: dependencies,
        diagnostics,
    }
}

pub(crate) fn increment_action_counts(
    action: ImportAction,
    create_count: &mut u64,
    update_count: &mut u64,
    skip_count: &mut u64,
) {
    match action {
        ImportAction::Create => *create_count += 1,
        ImportAction::Update => *update_count += 1,
        ImportAction::Skip => *skip_count += 1,
        ImportAction::Failed => {}
    }
}

pub(crate) fn import_action_translation_mode(action: ImportAction) -> &'static str {
    match action {
        ImportAction::Create | ImportAction::Update | ImportAction::Skip => "native",
        ImportAction::Failed => "reject",
    }
}

#[allow(dead_code)]
pub(crate) fn _assert_translation_module_compiles(_: Result<(), AppError>) {}
