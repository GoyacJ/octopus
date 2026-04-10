use std::{fs, path::Path};

use include_dir::{include_dir, Dir, DirEntry};
use octopus_core::{timestamp_now, AgentRecord, AppError};
use rusqlite::{params, Connection};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{catalog_hash_id, WorkspacePaths};

const SOURCE_KIND_BUNDLED_SEED: &str = "bundled_seed";
const MANAGED_SKILL_ROOT_PREFIX: &str = "data/skills";
const SKILL_FRONTMATTER_FILE: &str = "SKILL.md";

static BUNDLED_AGENT_SEED_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/seed/agent-bundle");

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

pub(crate) fn workspace_has_managed_skills(paths: &WorkspacePaths) -> Result<bool, AppError> {
    if !paths.managed_skills_dir.is_dir() {
        return Ok(false);
    }
    Ok(fs::read_dir(&paths.managed_skills_dir)?.next().is_some())
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

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
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
