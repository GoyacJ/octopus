use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::Path,
};

use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use include_dir::{include_dir, Dir, DirEntry};
use octopus_core::{
    timestamp_now, AgentRecord, AppError, ImportIssue, ImportWorkspaceAgentBundleInput,
    ImportWorkspaceAgentBundlePreview, ImportWorkspaceAgentBundlePreviewInput,
    ImportWorkspaceAgentBundleResult, ImportedAgentPreviewItem, ImportedSkillPreviewItem,
    WorkspaceDirectoryUploadEntry,
};
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    catalog_hash_id, validate_skill_file_relative_path, validate_skill_slug, WorkspacePaths,
};

const SOURCE_KIND_USER_IMPORT: &str = "user_import";
const SOURCE_KIND_BUNDLED_SEED: &str = "bundled_seed";
const SOURCE_SCOPE_BUNDLE: &str = "bundle";
const SOURCE_SCOPE_AGENT: &str = "agent";
const SOURCE_SCOPE_SKILL: &str = "skill";
const ISSUE_WARNING: &str = "warning";
const ISSUE_ERROR: &str = "error";
const MANAGED_SKILL_ROOT_PREFIX: &str = "data/skills";
const SKILL_FRONTMATTER_FILE: &str = "SKILL.md";
const FILTERED_DIR_NAMES: &[&str] = &[
    "node_modules",
    ".git",
    ".cache",
    ".turbo",
    "dist",
    "build",
    "coverage",
    "__pycache__",
    ".venv",
    "venv",
];

static BUNDLED_AGENT_SEED_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/seed/agent-bundle");

#[derive(Debug, Clone)]
struct BundleFile {
    relative_path: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct ParsedAgent {
    source_id: String,
    department: String,
    name: String,
    description: String,
    personality: String,
    prompt: String,
    skill_source_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct ParsedSkillSource {
    source_id: String,
    department: String,
    agent_name: String,
    name: String,
    canonical_slug: String,
    content_hash: String,
    files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
struct PlannedSkill {
    slug: String,
    skill_id: String,
    name: String,
    action: ImportAction,
    content_hash: String,
    file_count: usize,
    source_ids: Vec<String>,
    departments: Vec<String>,
    agent_names: Vec<String>,
    files: Vec<(String, Vec<u8>)>,
}

#[derive(Debug, Clone)]
struct PlannedAgent {
    source_id: String,
    agent_id: Option<String>,
    name: String,
    department: String,
    action: ImportAction,
    description: String,
    personality: String,
    prompt: String,
    skill_slugs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportAction {
    Create,
    Update,
    Skip,
    Failed,
}

impl ImportAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Skip => "skip",
            Self::Failed => "failed",
        }
    }
}

#[derive(Debug)]
struct BundlePlan {
    departments: Vec<String>,
    detected_agent_count: u64,
    filtered_file_count: u64,
    issues: Vec<ImportIssue>,
    skills: Vec<PlannedSkill>,
    agents: Vec<PlannedAgent>,
}

#[derive(Debug, Clone)]
struct ExistingAgentImportSource {
    agent_id: String,
}

#[derive(Debug, Clone)]
struct ExistingSkillImportSource {
    skill_slug: String,
}

#[derive(Debug, Clone)]
struct ExistingManagedSkill {
    slug: String,
    content_hash: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundledAgentSeedManifest {
    agents: Vec<BundledAgentSeedAgent>,
    skill_assets: Vec<BundledAgentSeedSkillAsset>,
    skill_sources: Vec<BundledAgentSeedSkillSource>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundledAgentSeedAgent {
    source_id: String,
    department: String,
    name: String,
    description: String,
    personality: String,
    prompt: String,
    skill_slugs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundledAgentSeedSkillAsset {
    slug: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BundledAgentSeedSkillSource {
    source_id: String,
    department: String,
    slug: String,
    content_hash: String,
}

pub(crate) fn ensure_import_source_tables(connection: &Connection) -> Result<(), AppError> {
    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS agent_import_sources (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                department TEXT NOT NULL,
                last_imported_at INTEGER NOT NULL,
                PRIMARY KEY (source_kind, source_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS skill_import_sources (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                skill_slug TEXT NOT NULL,
                department TEXT NOT NULL,
                last_imported_at INTEGER NOT NULL,
                PRIMARY KEY (source_kind, source_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS team_import_sources (
                source_kind TEXT NOT NULL,
                source_id TEXT NOT NULL,
                source_path TEXT NOT NULL,
                content_hash TEXT NOT NULL,
                team_id TEXT NOT NULL,
                department TEXT NOT NULL,
                last_imported_at INTEGER NOT NULL,
                PRIMARY KEY (source_kind, source_id)
            )",
            [],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

    Ok(())
}

pub(crate) fn preview_agent_bundle_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    input: ImportWorkspaceAgentBundlePreviewInput,
) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
    let plan = build_bundle_plan(
        connection,
        paths,
        workspace_id,
        &input.files,
        SOURCE_KIND_USER_IMPORT,
    )?;
    Ok(plan_to_preview(&plan))
}

pub(crate) fn execute_agent_bundle_import(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    input: ImportWorkspaceAgentBundleInput,
) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
    let plan = build_bundle_plan(
        connection,
        paths,
        workspace_id,
        &input.files,
        SOURCE_KIND_USER_IMPORT,
    )?;
    let builtin_tool_keys = builtin_tool_keys();
    let mut issues = plan.issues.clone();
    let mut failed_skill_slugs = BTreeSet::new();
    let mut skill_results = Vec::new();
    let now = timestamp_now();

    for skill in &plan.skills {
        let mut action = skill.action;
        if matches!(skill.action, ImportAction::Create | ImportAction::Update) {
            if let Err(error) = write_managed_skill(paths, &skill.slug, &skill.files) {
                action = ImportAction::Failed;
                failed_skill_slugs.insert(skill.slug.clone());
                issues.push(ImportIssue {
                    severity: ISSUE_ERROR.into(),
                    scope: SOURCE_SCOPE_SKILL.into(),
                    source_id: skill.source_ids.first().cloned(),
                    message: format!("failed to import skill '{}': {}", skill.slug, error),
                });
            }
        }

        if action != ImportAction::Failed {
            for source_id in &skill.source_ids {
                upsert_skill_import_source(
                    connection,
                    SOURCE_KIND_USER_IMPORT,
                    source_id,
                    &skill.content_hash,
                    &skill.slug,
                    skill
                        .departments
                        .first()
                        .map(String::as_str)
                        .unwrap_or_default(),
                    now,
                )?;
            }
        }

        skill_results.push(ImportedSkillPreviewItem {
            slug: skill.slug.clone(),
            skill_id: skill.skill_id.clone(),
            name: skill.name.clone(),
            action: action.as_str().into(),
            content_hash: skill.content_hash.clone(),
            file_count: skill.file_count as u64,
            source_ids: skill.source_ids.clone(),
            departments: skill.departments.clone(),
            agent_names: skill.agent_names.clone(),
        });
    }

    let existing_agents = load_existing_agents(connection)?;
    let mut agent_results = Vec::new();
    let mut create_count = 0_u64;
    let mut update_count = 0_u64;
    let mut skip_count = 0_u64;

    for agent in &plan.agents {
        let usable_skill_slugs = agent
            .skill_slugs
            .iter()
            .filter(|slug| !failed_skill_slugs.contains(*slug))
            .cloned()
            .collect::<Vec<_>>();
        let skill_ids = usable_skill_slugs
            .iter()
            .map(|slug| managed_skill_id(slug))
            .collect::<Vec<_>>();
        let desired_hash = compute_agent_hash(
            workspace_id,
            &agent.name,
            &agent.description,
            &agent.personality,
            &agent.prompt,
            &[agent.department.clone()],
            &builtin_tool_keys,
            &skill_ids,
        )?;

        let existing_record = agent
            .agent_id
            .as_deref()
            .and_then(|agent_id| existing_agents.get(agent_id))
            .cloned();
        let existing_hash = existing_record
            .as_ref()
            .map(compute_existing_agent_hash)
            .transpose()?;

        let actual_action = if existing_record.is_none() {
            ImportAction::Create
        } else if existing_hash.as_deref() == Some(desired_hash.as_str()) {
            ImportAction::Skip
        } else {
            ImportAction::Update
        };

        let mut result_action = actual_action;
        let agent_id = if let Some(existing_id) = agent.agent_id.clone() {
            existing_id
        } else {
            deterministic_seeded_agent_id(&agent.source_id, "agent-import")
        };

        if matches!(actual_action, ImportAction::Create | ImportAction::Update) {
            let record = build_imported_agent_record(
                workspace_id,
                &agent_id,
                &agent.name,
                &agent.department,
                &agent.description,
                &agent.personality,
                &agent.prompt,
                &builtin_tool_keys,
                &skill_ids,
            );
            if let Err(error) =
                write_agent_record(connection, &record, actual_action == ImportAction::Update)
            {
                result_action = ImportAction::Failed;
                issues.push(ImportIssue {
                    severity: ISSUE_ERROR.into(),
                    scope: SOURCE_SCOPE_AGENT.into(),
                    source_id: Some(agent.source_id.clone()),
                    message: format!("failed to import agent '{}': {}", agent.name, error),
                });
            } else {
                upsert_agent_import_source(
                    connection,
                    SOURCE_KIND_USER_IMPORT,
                    &agent.source_id,
                    &desired_hash,
                    &agent_id,
                    &agent.department,
                    now,
                )?;
            }
        } else if actual_action == ImportAction::Skip {
            upsert_agent_import_source(
                connection,
                SOURCE_KIND_USER_IMPORT,
                &agent.source_id,
                &desired_hash,
                &agent_id,
                &agent.department,
                now,
            )?;
        }

        match result_action {
            ImportAction::Create => create_count += 1,
            ImportAction::Update => update_count += 1,
            ImportAction::Skip => skip_count += 1,
            ImportAction::Failed => {}
        }

        agent_results.push(ImportedAgentPreviewItem {
            source_id: agent.source_id.clone(),
            agent_id: Some(agent_id),
            name: agent.name.clone(),
            department: agent.department.clone(),
            action: result_action.as_str().into(),
            skill_slugs: usable_skill_slugs,
            mcp_server_names: Vec::new(),
        });
    }

    Ok(ImportWorkspaceAgentBundleResult {
        departments: plan.departments.clone(),
        department_count: plan.departments.len() as u64,
        detected_agent_count: plan.detected_agent_count,
        importable_agent_count: plan.agents.len() as u64,
        detected_team_count: 0,
        importable_team_count: 0,
        create_count,
        update_count,
        skip_count,
        failure_count: issues
            .iter()
            .filter(|issue| issue.severity == ISSUE_ERROR)
            .count() as u64,
        unique_skill_count: skill_results.len() as u64,
        unique_mcp_count: 0,
        avatar_count: 0,
        filtered_file_count: plan.filtered_file_count,
        agents: agent_results,
        teams: Vec::new(),
        skills: skill_results,
        mcps: Vec::new(),
        avatars: Vec::new(),
        issues,
    })
}

pub(crate) fn seed_bundled_agent_bundle(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
) -> Result<Vec<String>, AppError> {
    let Some(manifest) = bundled_seed_manifest()? else {
        return Ok(Vec::new());
    };

    let builtin_tool_keys = builtin_tool_keys();
    let now = timestamp_now();

    for asset in &manifest.skill_assets {
        let skill_dir = paths.managed_skills_dir.join(&asset.slug);
        if skill_dir.exists() {
            continue;
        }
        let Some(source_dir) = BUNDLED_AGENT_SEED_DIR.get_dir(format!("skills/{}", asset.slug))
        else {
            continue;
        };
        write_embedded_directory(source_dir, &skill_dir)?;
    }

    for source in &manifest.skill_sources {
        upsert_skill_import_source(
            connection,
            SOURCE_KIND_BUNDLED_SEED,
            &source.source_id,
            &source.content_hash,
            &source.slug,
            &source.department,
            now,
        )?;
    }

    let mut seeded_agent_ids = Vec::new();
    for agent in &manifest.agents {
        let agent_id = deterministic_seeded_agent_id(&agent.source_id, "agent-seed");
        let skill_ids = agent
            .skill_slugs
            .iter()
            .map(|slug| managed_skill_id(slug))
            .collect::<Vec<_>>();
        let record = build_imported_agent_record(
            workspace_id,
            &agent_id,
            &agent.name,
            &agent.department,
            &agent.description,
            &agent.personality,
            &agent.prompt,
            &builtin_tool_keys,
            &skill_ids,
        );
        write_agent_record(connection, &record, false)?;
        let hash = compute_existing_agent_hash(&record)?;
        upsert_agent_import_source(
            connection,
            SOURCE_KIND_BUNDLED_SEED,
            &agent.source_id,
            &hash,
            &agent_id,
            &agent.department,
            now,
        )?;
        seeded_agent_ids.push(agent_id);
    }

    Ok(seeded_agent_ids)
}

fn build_bundle_plan(
    connection: &Connection,
    paths: &WorkspacePaths,
    workspace_id: &str,
    files: &[WorkspaceDirectoryUploadEntry],
    source_kind: &str,
) -> Result<BundlePlan, AppError> {
    let (bundle_files, filtered_file_count, mut issues) = normalize_bundle_files(files)?;
    let grouped_by_root = group_agent_root_files(&bundle_files);
    let mut parsed_agents = Vec::new();
    let mut parsed_skill_sources = Vec::new();
    let mut detected_agent_count = 0_u64;

    for ((department, agent_dir), root_files) in grouped_by_root {
        let expected_markdown = format!("{department}/{agent_dir}/{agent_dir}.md");
        let Some(agent_file) = root_files
            .iter()
            .find(|file| file.relative_path == expected_markdown)
        else {
            issues.push(ImportIssue {
                severity: ISSUE_WARNING.into(),
                scope: SOURCE_SCOPE_AGENT.into(),
                source_id: Some(format!("{department}/{agent_dir}")),
                message: format!(
                    "skipped agent '{}': missing required '{}'",
                    agent_dir, expected_markdown
                ),
            });
            continue;
        };

        detected_agent_count += 1;
        let agent_source_id = format!("{department}/{agent_dir}");
        let agent_source = String::from_utf8_lossy(&agent_file.bytes).to_string();
        let (frontmatter, body) = split_frontmatter(&agent_source);
        let agent_name = frontmatter
            .get("name")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| agent_dir.clone());
        let agent_description = frontmatter
            .get("description")
            .cloned()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| first_non_empty_paragraph(&body))
            .unwrap_or_else(|| agent_name.clone());
        let agent_personality =
            first_paragraph_after_heading(&body, "\u{89d2}\u{8272}\u{5b9a}\u{4e49}")
                .or_else(|| first_paragraph_after_heading(&body, "Role Definition"))
                .unwrap_or_else(|| agent_name.clone());
        let agent_prompt = body.trim().to_string();

        let grouped_skills = group_agent_skill_files(&department, &agent_dir, &root_files);
        let mut skill_source_ids = Vec::new();
        for (skill_dir, skill_files) in grouped_skills {
            let Some(skill_file) = skill_files
                .iter()
                .find(|(path, _)| path == SKILL_FRONTMATTER_FILE)
            else {
                issues.push(ImportIssue {
                    severity: ISSUE_WARNING.into(),
                    scope: SOURCE_SCOPE_SKILL.into(),
                    source_id: Some(format!("{agent_source_id}/skills/{skill_dir}")),
                    message: format!(
                        "skipped skill '{}': missing required '{}'",
                        skill_dir, SKILL_FRONTMATTER_FILE
                    ),
                });
                continue;
            };

            let skill_source = String::from_utf8_lossy(&skill_file.1).to_string();
            let (skill_frontmatter, _) = split_frontmatter(&skill_source);
            let skill_name = skill_frontmatter
                .get("name")
                .cloned()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| skill_dir.clone());
            let canonical_slug = validate_skill_slug(&slugify_skill_name(&skill_name, "skill"))?;
            let content_hash = hash_bundle_files(&skill_files);
            let skill_source_id = format!("{agent_source_id}/skills/{skill_dir}");
            skill_source_ids.push(skill_source_id.clone());
            parsed_skill_sources.push(ParsedSkillSource {
                source_id: skill_source_id,
                department: department.clone(),
                agent_name: agent_name.clone(),
                name: skill_name,
                canonical_slug,
                content_hash,
                files: skill_files,
            });
        }

        parsed_agents.push(ParsedAgent {
            source_id: agent_source_id,
            department,
            name: agent_name,
            description: agent_description,
            personality: agent_personality,
            prompt: agent_prompt,
            skill_source_ids,
        });
    }

    if parsed_agents.is_empty() {
        issues.push(ImportIssue {
            severity: ISSUE_ERROR.into(),
            scope: SOURCE_SCOPE_BUNDLE.into(),
            source_id: None,
            message: String::from("no compatible agents were found in the selected bundle"),
        });
    }

    let existing_skill_sources = load_existing_skill_import_sources(connection, source_kind)?;
    let existing_agent_sources = load_existing_agent_import_sources(connection, source_kind)?;
    let existing_managed_skills = load_existing_managed_skills(paths)?;
    let existing_agents = load_existing_agents(connection)?;
    let builtin_tool_keys = builtin_tool_keys();

    let mut unique_skills = BTreeMap::<(String, String), PlannedSkill>::new();
    for skill_source in &parsed_skill_sources {
        let entry = unique_skills
            .entry((
                skill_source.canonical_slug.clone(),
                skill_source.content_hash.clone(),
            ))
            .or_insert_with(|| PlannedSkill {
                slug: String::new(),
                skill_id: String::new(),
                name: skill_source.name.clone(),
                action: ImportAction::Create,
                content_hash: skill_source.content_hash.clone(),
                file_count: skill_source.files.len(),
                source_ids: Vec::new(),
                departments: Vec::new(),
                agent_names: Vec::new(),
                files: skill_source.files.clone(),
            });
        entry.source_ids.push(skill_source.source_id.clone());
        if !entry
            .departments
            .iter()
            .any(|item| item == &skill_source.department)
        {
            entry.departments.push(skill_source.department.clone());
        }
        if !entry
            .agent_names
            .iter()
            .any(|item| item == &skill_source.agent_name)
        {
            entry.agent_names.push(skill_source.agent_name.clone());
        }
    }

    let mut planned_skills = Vec::new();
    for ((canonical_slug, content_hash), mut planned_skill) in unique_skills {
        let mut mapped_slugs = planned_skill
            .source_ids
            .iter()
            .filter_map(|source_id| existing_skill_sources.get(source_id))
            .filter_map(|mapping| existing_managed_skills.get(&mapping.skill_slug))
            .map(|skill| skill.slug.clone())
            .collect::<BTreeSet<_>>();

        if mapped_slugs.len() > 1 {
            issues.push(ImportIssue {
                severity: ISSUE_WARNING.into(),
                scope: SOURCE_SCOPE_SKILL.into(),
                source_id: planned_skill.source_ids.first().cloned(),
                message: format!(
                    "multiple existing skill mappings resolved for '{}'; using '{}'",
                    canonical_slug,
                    mapped_slugs.first().cloned().unwrap_or_default()
                ),
            });
        }

        let (slug, action) = if let Some(mapped_slug) = mapped_slugs.pop_first() {
            let existing = existing_managed_skills.get(&mapped_slug);
            if existing.is_some_and(|skill| skill.content_hash == content_hash) {
                (mapped_slug, ImportAction::Skip)
            } else {
                (mapped_slug, ImportAction::Update)
            }
        } else if let Some(existing) = existing_managed_skills.get(&canonical_slug) {
            if existing.content_hash == content_hash {
                (canonical_slug.clone(), ImportAction::Skip)
            } else {
                let candidate = format!("{}-{}", canonical_slug, short_hash(&content_hash));
                let action = if existing_managed_skills
                    .get(&candidate)
                    .is_some_and(|skill| skill.content_hash == content_hash)
                {
                    ImportAction::Skip
                } else {
                    ImportAction::Create
                };
                (candidate, action)
            }
        } else {
            (canonical_slug.clone(), ImportAction::Create)
        };

        planned_skill.slug = slug.clone();
        planned_skill.skill_id = managed_skill_id(&slug);
        planned_skill.action = action;
        planned_skills.push(planned_skill);
    }

    let skill_slug_by_source_id = planned_skills
        .iter()
        .flat_map(|skill| {
            skill
                .source_ids
                .iter()
                .cloned()
                .map(move |source_id| (source_id, skill.slug.clone()))
        })
        .collect::<HashMap<_, _>>();

    let mut planned_agents = Vec::new();
    for parsed_agent in parsed_agents {
        let skill_slugs = parsed_agent
            .skill_source_ids
            .iter()
            .filter_map(|source_id| skill_slug_by_source_id.get(source_id))
            .cloned()
            .collect::<Vec<_>>();
        let skill_ids = skill_slugs
            .iter()
            .map(|slug| managed_skill_id(slug))
            .collect::<Vec<_>>();
        let desired_hash = compute_agent_hash(
            workspace_id,
            &parsed_agent.name,
            &parsed_agent.description,
            &parsed_agent.personality,
            &parsed_agent.prompt,
            &[parsed_agent.department.clone()],
            &builtin_tool_keys,
            &skill_ids,
        )?;

        let mapping = existing_agent_sources.get(&parsed_agent.source_id);
        let existing_record = mapping.and_then(|item| existing_agents.get(&item.agent_id));
        let action = if let Some(record) = existing_record {
            let existing_hash = compute_existing_agent_hash(record)?;
            if existing_hash == desired_hash {
                ImportAction::Skip
            } else {
                ImportAction::Update
            }
        } else {
            if mapping.is_some() {
                issues.push(ImportIssue {
                    severity: ISSUE_WARNING.into(),
                    scope: SOURCE_SCOPE_AGENT.into(),
                    source_id: Some(parsed_agent.source_id.clone()),
                    message: String::from("stale agent import mapping ignored because the target agent no longer exists"),
                });
            }
            ImportAction::Create
        };

        planned_agents.push(PlannedAgent {
            source_id: parsed_agent.source_id,
            agent_id: existing_record.map(|record| record.id.clone()),
            name: parsed_agent.name,
            department: parsed_agent.department,
            action,
            description: parsed_agent.description,
            personality: parsed_agent.personality,
            prompt: parsed_agent.prompt,
            skill_slugs,
        });
    }

    let departments = planned_agents
        .iter()
        .map(|agent| agent.department.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    planned_skills.sort_by(|left, right| left.slug.cmp(&right.slug));
    planned_agents.sort_by(|left, right| left.source_id.cmp(&right.source_id));

    Ok(BundlePlan {
        departments,
        detected_agent_count,
        filtered_file_count,
        issues,
        skills: planned_skills,
        agents: planned_agents,
    })
}

fn plan_to_preview(plan: &BundlePlan) -> ImportWorkspaceAgentBundlePreview {
    ImportWorkspaceAgentBundlePreview {
        departments: plan.departments.clone(),
        department_count: plan.departments.len() as u64,
        detected_agent_count: plan.detected_agent_count,
        importable_agent_count: plan.agents.len() as u64,
        create_count: plan
            .agents
            .iter()
            .filter(|agent| agent.action == ImportAction::Create)
            .count() as u64,
        update_count: plan
            .agents
            .iter()
            .filter(|agent| agent.action == ImportAction::Update)
            .count() as u64,
        skip_count: plan
            .agents
            .iter()
            .filter(|agent| agent.action == ImportAction::Skip)
            .count() as u64,
        failure_count: plan
            .issues
            .iter()
            .filter(|issue| issue.severity == ISSUE_ERROR)
            .count() as u64,
        unique_skill_count: plan.skills.len() as u64,
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
                skill_slugs: agent.skill_slugs.clone(),
                mcp_server_names: Vec::new(),
            })
            .collect(),
        teams: Vec::new(),
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
                departments: skill.departments.clone(),
                agent_names: skill.agent_names.clone(),
            })
            .collect(),
        mcps: Vec::new(),
        avatars: Vec::new(),
        issues: plan.issues.clone(),
        detected_team_count: 0,
        importable_team_count: 0,
        unique_mcp_count: 0,
        avatar_count: 0,
    }
}

fn normalize_bundle_files(
    files: &[WorkspaceDirectoryUploadEntry],
) -> Result<(Vec<BundleFile>, u64, Vec<ImportIssue>), AppError> {
    if files.is_empty() {
        return Err(AppError::invalid_input("agent bundle files are required"));
    }

    let mut normalized = Vec::new();
    let mut filtered_file_count = 0_u64;
    let mut issues = Vec::new();

    for file in files {
        let relative_path = validate_skill_file_relative_path(&file.relative_path)?;
        if path_contains_filtered_directory(&relative_path) {
            filtered_file_count += 1;
            continue;
        }
        let bytes = match BASE64_STANDARD.decode(&file.data_base64) {
            Ok(bytes) => bytes,
            Err(error) => {
                issues.push(ImportIssue {
                    severity: ISSUE_WARNING.into(),
                    scope: SOURCE_SCOPE_BUNDLE.into(),
                    source_id: Some(relative_path),
                    message: format!("skipped file with invalid base64 payload: {error}"),
                });
                continue;
            }
        };
        normalized.push(BundleFile {
            relative_path,
            bytes,
        });
    }

    Ok((normalized, filtered_file_count, issues))
}

fn group_agent_root_files(files: &[BundleFile]) -> BTreeMap<(String, String), Vec<BundleFile>> {
    let mut grouped = BTreeMap::<(String, String), Vec<BundleFile>>::new();
    for file in files {
        let segments = file.relative_path.split('/').collect::<Vec<_>>();
        if segments.len() < 2 {
            continue;
        }
        grouped
            .entry((segments[0].to_string(), segments[1].to_string()))
            .or_default()
            .push(file.clone());
    }
    grouped
}

fn group_agent_skill_files(
    department: &str,
    agent_dir: &str,
    files: &[BundleFile],
) -> BTreeMap<String, Vec<(String, Vec<u8>)>> {
    let prefix = format!("{department}/{agent_dir}/skills/");
    let mut grouped = BTreeMap::<String, Vec<(String, Vec<u8>)>>::new();

    for file in files {
        if !file.relative_path.starts_with(&prefix) {
            continue;
        }
        let suffix = &file.relative_path[prefix.len()..];
        let segments = suffix.split('/').collect::<Vec<_>>();
        if segments.len() < 2 {
            continue;
        }
        let skill_dir = segments[0].to_string();
        let relative_path = segments[1..].join("/");
        grouped
            .entry(skill_dir)
            .or_default()
            .push((relative_path, file.bytes.clone()));
    }

    for files in grouped.values_mut() {
        files.sort_by(|left, right| left.0.cmp(&right.0));
    }

    grouped
}

fn split_frontmatter(contents: &str) -> (BTreeMap<String, String>, String) {
    let normalized = contents.replace("\r\n", "\n");
    let lines = normalized.lines().collect::<Vec<_>>();
    let Some(first) = lines.first() else {
        return (BTreeMap::new(), String::new());
    };
    if !is_frontmatter_delimiter(first) {
        return (BTreeMap::new(), normalized);
    }

    let mut frontmatter = BTreeMap::new();
    let mut body_index = 1_usize;
    while body_index < lines.len() {
        let line = lines[body_index];
        if is_frontmatter_delimiter(line) {
            body_index += 1;
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            let trimmed_key = key.trim();
            let trimmed_value = unquote_frontmatter_value(value.trim());
            if !trimmed_key.is_empty() && !trimmed_value.is_empty() {
                frontmatter.insert(trimmed_key.to_string(), trimmed_value);
            }
        }
        body_index += 1;
    }

    let body = lines[body_index..].join("\n");
    (frontmatter, body)
}

fn is_frontmatter_delimiter(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.len() >= 3 && trimmed.chars().all(|value| value == '-')
}

fn unquote_frontmatter_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|trimmed| trimmed.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|trimmed| trimmed.strip_suffix('\''))
        })
        .unwrap_or(value)
        .trim()
        .to_string()
}

fn first_non_empty_paragraph(body: &str) -> Option<String> {
    let mut paragraph = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(trimmed.to_string());
    }

    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph.join(" "))
    }
}

fn first_paragraph_after_heading(body: &str, heading: &str) -> Option<String> {
    let mut heading_found = false;
    let mut paragraph = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !heading_found {
            let candidate = trimmed.trim_start_matches('#').trim();
            if trimmed.starts_with('#') && candidate == heading {
                heading_found = true;
            }
            continue;
        }
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            break;
        }
        paragraph.push(trimmed.to_string());
    }

    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph.join(" "))
    }
}

fn path_contains_filtered_directory(relative_path: &str) -> bool {
    relative_path.split('/').any(|segment| {
        FILTERED_DIR_NAMES
            .iter()
            .any(|candidate| candidate == &segment)
    })
}

fn slugify_skill_name(value: &str, fallback_prefix: &str) -> String {
    let mut slug = String::new();
    let mut last_was_separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_separator = false;
            continue;
        }
        if matches!(character, '-' | '_' | '.' | ' ') && !last_was_separator && !slug.is_empty() {
            slug.push('-');
            last_was_separator = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        format!("{fallback_prefix}-{}", short_hash(&hash_text(value)))
    } else {
        slug
    }
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256-{:x}", hasher.finalize())
}

fn hash_bundle_files(files: &[(String, Vec<u8>)]) -> String {
    let mut hasher = Sha256::new();
    for (relative_path, bytes) in files {
        hasher.update(relative_path.as_bytes());
        hasher.update(b"\n");
        hasher.update(bytes);
        hasher.update(b"\n");
    }
    format!("sha256-{:x}", hasher.finalize())
}

fn short_hash(value: &str) -> String {
    value
        .rsplit('-')
        .next()
        .unwrap_or(value)
        .chars()
        .take(8)
        .collect()
}

fn managed_skill_id(slug: &str) -> String {
    catalog_hash_id(
        "skill",
        &format!("{MANAGED_SKILL_ROOT_PREFIX}/{slug}/{SKILL_FRONTMATTER_FILE}"),
    )
}

fn builtin_tool_keys() -> Vec<String> {
    tools::mvp_tool_specs()
        .iter()
        .map(|spec| spec.name.to_string())
        .collect()
}

fn build_imported_agent_record(
    workspace_id: &str,
    agent_id: &str,
    name: &str,
    department: &str,
    description: &str,
    personality: &str,
    prompt: &str,
    builtin_tool_keys: &[String],
    skill_ids: &[String],
) -> AgentRecord {
    AgentRecord {
        id: agent_id.to_string(),
        workspace_id: workspace_id.to_string(),
        project_id: None,
        scope: "workspace".into(),
        name: name.trim().to_string(),
        avatar_path: None,
        avatar: None,
        personality: personality.trim().to_string(),
        tags: vec![department.to_string()],
        prompt: prompt.trim().to_string(),
        builtin_tool_keys: builtin_tool_keys.to_vec(),
        skill_ids: skill_ids.to_vec(),
        mcp_server_names: Vec::new(),
        integration_source: None,
        description: description.trim().to_string(),
        status: "active".into(),
        updated_at: timestamp_now(),
    }
}

fn compute_agent_hash(
    workspace_id: &str,
    name: &str,
    description: &str,
    personality: &str,
    prompt: &str,
    tags: &[String],
    builtin_tool_keys: &[String],
    skill_ids: &[String],
) -> Result<String, AppError> {
    let payload = json!({
        "workspaceId": workspace_id,
        "scope": "workspace",
        "name": name.trim(),
        "description": description.trim(),
        "personality": personality.trim(),
        "prompt": prompt.trim(),
        "tags": tags,
        "builtinToolKeys": builtin_tool_keys,
        "skillIds": skill_ids,
        "mcpServerNames": [],
        "status": "active",
    });
    Ok(hash_text(&serde_json::to_string(&payload)?))
}

fn compute_existing_agent_hash(record: &AgentRecord) -> Result<String, AppError> {
    compute_agent_hash(
        &record.workspace_id,
        &record.name,
        &record.description,
        &record.personality,
        &record.prompt,
        &record.tags,
        &record.builtin_tool_keys,
        &record.skill_ids,
    )
}

fn write_managed_skill(
    paths: &WorkspacePaths,
    slug: &str,
    files: &[(String, Vec<u8>)],
) -> Result<(), AppError> {
    let slug = validate_skill_slug(slug)?;
    let target_dir = paths.managed_skills_dir.join(&slug);
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;
    for (relative_path, bytes) in files {
        let safe_relative_path = validate_skill_file_relative_path(relative_path)?;
        let target_path = target_dir.join(&safe_relative_path);
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)?;
    }
    Ok(())
}

fn write_agent_record(
    connection: &Connection,
    record: &AgentRecord,
    replace: bool,
) -> Result<(), AppError> {
    let verb = if replace {
        "INSERT OR REPLACE"
    } else {
        "INSERT"
    };
    connection
        .execute(
            &format!(
                "{verb} INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"
            ),
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn upsert_skill_import_source(
    connection: &Connection,
    source_kind: &str,
    source_id: &str,
    content_hash: &str,
    skill_slug: &str,
    department: &str,
    last_imported_at: u64,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT OR REPLACE INTO skill_import_sources
             (source_kind, source_id, source_path, content_hash, skill_slug, department, last_imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source_kind,
                source_id,
                source_id,
                content_hash,
                skill_slug,
                department,
                last_imported_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn upsert_agent_import_source(
    connection: &Connection,
    source_kind: &str,
    source_id: &str,
    content_hash: &str,
    agent_id: &str,
    department: &str,
    last_imported_at: u64,
) -> Result<(), AppError> {
    connection
        .execute(
            "INSERT OR REPLACE INTO agent_import_sources
             (source_kind, source_id, source_path, content_hash, agent_id, department, last_imported_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                source_kind,
                source_id,
                source_id,
                content_hash,
                agent_id,
                department,
                last_imported_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
    Ok(())
}

fn load_existing_skill_import_sources(
    connection: &Connection,
    source_kind: &str,
) -> Result<HashMap<String, ExistingSkillImportSource>, AppError> {
    let mut stmt = connection
        .prepare("SELECT source_id, skill_slug FROM skill_import_sources WHERE source_kind = ?1")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map(params![source_kind], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ExistingSkillImportSource {
                    skill_slug: row.get(1)?,
                },
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    let mut mappings = HashMap::new();
    for row in rows {
        let (source_id, mapping) = row.map_err(|error| AppError::database(error.to_string()))?;
        mappings.insert(source_id, mapping);
    }
    Ok(mappings)
}

fn load_existing_agent_import_sources(
    connection: &Connection,
    source_kind: &str,
) -> Result<HashMap<String, ExistingAgentImportSource>, AppError> {
    let mut stmt = connection
        .prepare("SELECT source_id, agent_id FROM agent_import_sources WHERE source_kind = ?1")
        .map_err(|error| AppError::database(error.to_string()))?;
    let rows = stmt
        .query_map(params![source_kind], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ExistingAgentImportSource {
                    agent_id: row.get(1)?,
                },
            ))
        })
        .map_err(|error| AppError::database(error.to_string()))?;

    let mut mappings = HashMap::new();
    for row in rows {
        let (source_id, mapping) = row.map_err(|error| AppError::database(error.to_string()))?;
        mappings.insert(source_id, mapping);
    }
    Ok(mappings)
}

fn load_existing_agents(connection: &Connection) -> Result<HashMap<String, AgentRecord>, AppError> {
    super::load_agents(connection).map(|agents| {
        agents
            .into_iter()
            .map(|agent| (agent.id.clone(), agent))
            .collect::<HashMap<_, _>>()
    })
}

fn load_existing_managed_skills(
    paths: &WorkspacePaths,
) -> Result<HashMap<String, ExistingManagedSkill>, AppError> {
    let mut skills = HashMap::new();
    if !paths.managed_skills_dir.is_dir() {
        return Ok(skills);
    }

    for entry in fs::read_dir(&paths.managed_skills_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let slug = entry.file_name().to_string_lossy().to_string();
        let skill_file = path.join(SKILL_FRONTMATTER_FILE);
        if !skill_file.is_file() {
            continue;
        }
        let files = read_directory_files(&path)?;
        skills.insert(
            slug.clone(),
            ExistingManagedSkill {
                slug,
                content_hash: hash_bundle_files(&files),
            },
        );
    }

    Ok(skills)
}

fn read_directory_files(root: &Path) -> Result<Vec<(String, Vec<u8>)>, AppError> {
    let mut files = Vec::new();
    read_directory_files_recursive(root, root, &mut files)?;
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(files)
}

fn read_directory_files_recursive(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, Vec<u8>)>,
) -> Result<(), AppError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            read_directory_files_recursive(root, &path, files)?;
            continue;
        }
        let relative_path = path
            .strip_prefix(root)
            .map_err(|error| AppError::invalid_input(error.to_string()))?
            .to_string_lossy()
            .replace('\\', "/");
        files.push((relative_path, fs::read(&path)?));
    }
    Ok(())
}

fn bundled_seed_manifest() -> Result<Option<BundledAgentSeedManifest>, AppError> {
    let Some(file) = BUNDLED_AGENT_SEED_DIR.get_file("manifest.json") else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(file.contents())?))
}

fn write_embedded_directory(source: &Dir<'_>, target: &Path) -> Result<(), AppError> {
    fs::create_dir_all(target)?;
    for entry in source.entries() {
        match entry {
            DirEntry::Dir(child) => {
                let name = child
                    .path()
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| AppError::invalid_input("invalid embedded directory path"))?;
                write_embedded_directory(child, &target.join(name))?;
            }
            DirEntry::File(file) => {
                let name = file
                    .path()
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| AppError::invalid_input("invalid embedded file path"))?;
                fs::write(target.join(name), file.contents())?;
            }
        }
    }
    Ok(())
}

fn deterministic_seeded_agent_id(source_id: &str, prefix: &str) -> String {
    format!("{prefix}-{}", short_hash(&hash_text(source_id)))
}

pub(crate) fn workspace_has_managed_skills(paths: &WorkspacePaths) -> Result<bool, AppError> {
    if !paths.managed_skills_dir.is_dir() {
        return Ok(false);
    }
    Ok(fs::read_dir(&paths.managed_skills_dir)?.next().is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded_file(relative_path: &str, content: &str) -> WorkspaceDirectoryUploadEntry {
        WorkspaceDirectoryUploadEntry {
            relative_path: relative_path.into(),
            file_name: Path::new(relative_path)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .into(),
            content_type: "text/markdown".into(),
            data_base64: BASE64_STANDARD.encode(content.as_bytes()),
            byte_size: content.len() as u64,
        }
    }

    fn ensure_test_agent_table(connection: &Connection) {
        connection
            .execute_batch(
                "CREATE TABLE agents (
                    id TEXT PRIMARY KEY,
                    workspace_id TEXT NOT NULL,
                    project_id TEXT,
                    scope TEXT NOT NULL,
                    name TEXT NOT NULL,
                    avatar_path TEXT,
                    personality TEXT NOT NULL,
                    tags TEXT NOT NULL,
                    prompt TEXT NOT NULL,
                    builtin_tool_keys TEXT NOT NULL,
                    skill_ids TEXT NOT NULL,
                    mcp_server_names TEXT NOT NULL,
                    description TEXT NOT NULL,
                    status TEXT NOT NULL,
                    updated_at INTEGER NOT NULL
                );",
            )
            .expect("agents table");
    }

    #[test]
    fn split_frontmatter_accepts_dash_delimiters() {
        let (frontmatter, body) = split_frontmatter(
            "---\nname: 通用助手\ndescription: 测试\n-----------\n\n# 角色定义\n你好\n",
        );
        assert_eq!(
            frontmatter.get("name").map(String::as_str),
            Some("通用助手")
        );
        assert!(body.contains("# 角色定义"));
    }

    #[test]
    fn preview_plan_deduplicates_shared_skills() {
        let temp = tempfile::tempdir().expect("tempdir");
        let paths = WorkspacePaths::new(temp.path());
        paths.ensure_layout().expect("layout");
        let connection = Connection::open(paths.db_path.clone()).expect("db");
        ensure_test_agent_table(&connection);
        ensure_import_source_tables(&connection).expect("tables");

        let preview = preview_agent_bundle_import(
            &connection,
            &paths,
            "ws-local",
            ImportWorkspaceAgentBundlePreviewInput {
                files: vec![
                    encoded_file(
                        "系统通用/通用助手/通用助手.md",
                        "---\nname: 通用助手\ndescription: 通用处理\n---\n\n# 角色定义\n通用处理专家\n",
                    ),
                    encoded_file(
                        "系统通用/通用助手/skills/shared-skill/SKILL.md",
                        "---\nname: shared-skill\ndescription: shared\n---\n\n# Shared\n",
                    ),
                    encoded_file(
                        "产品部/产品经理/产品经理.md",
                        "---\nname: 产品经理\ndescription: 产品工作\n---\n\n# 角色定义\n产品专家\n",
                    ),
                    encoded_file(
                        "产品部/产品经理/skills/shared-skill/SKILL.md",
                        "---\nname: shared-skill\ndescription: shared\n---\n\n# Shared\n",
                    ),
                ],
            },
        )
        .expect("preview");

        assert_eq!(preview.detected_agent_count, 2);
        assert_eq!(preview.unique_skill_count, 1);
        assert_eq!(preview.create_count, 2);
        assert_eq!(preview.skills[0].source_ids.len(), 2);
    }

    #[test]
    fn first_paragraph_after_heading_supports_localized_and_english_role_headers() {
        let localized = "\n# \u{89d2}\u{8272}\u{5b9a}\u{4e49}\n\u{7cbe}\u{7ec6}\u{6267}\u{884c}\u{4e13}\u{5bb6}\n";
        let english = "\n# Role Definition\nPrecise execution specialist\n";

        assert_eq!(
            first_paragraph_after_heading(localized, "\u{89d2}\u{8272}\u{5b9a}\u{4e49}"),
            Some("\u{7cbe}\u{7ec6}\u{6267}\u{884c}\u{4e13}\u{5bb6}".into())
        );
        assert_eq!(
            first_paragraph_after_heading(english, "Role Definition"),
            Some("Precise execution specialist".into())
        );
    }
}
