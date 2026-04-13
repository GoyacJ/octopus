use std::{fs, path::Path};

use include_dir::{include_dir, Dir, DirEntry};
use octopus_core::{timestamp_now, AppError};
use rusqlite::Connection;
use serde::Deserialize;

use crate::{infra_state::write_agent_record, WorkspacePaths};

use super::shared::{
    build_imported_agent_record, builtin_tool_keys, compute_existing_agent_hash,
    deterministic_seeded_agent_id, managed_skill_id, upsert_agent_import_source,
    upsert_skill_import_source,
};

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

pub(crate) use super::shared::ensure_import_source_tables;

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
            &source.skill_path(),
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
            .map(|slug| managed_skill_id(&managed_skill_path(slug)))
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

fn bundled_seed_manifest() -> Result<Option<BundledAgentSeedManifest>, AppError> {
    let Some(file) = BUNDLED_AGENT_SEED_DIR.get_file(".octopus/manifest.json") else {
        return Ok(None);
    };
    Ok(Some(serde_json::from_slice(file.contents())?))
}

fn write_embedded_directory(source: &Dir<'_>, target: &Path) -> Result<(), AppError> {
    fs::create_dir_all(target)?;
    for entry in source.entries() {
        match entry {
            DirEntry::Dir(dir) => {
                let next_target = target.join(dir.path());
                write_embedded_directory(dir, &next_target)?;
            }
            DirEntry::File(file) => {
                let relative_path = file.path().to_string_lossy();
                let destination = target.join(relative_path.as_ref());
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(destination, file.contents())?;
            }
        }
    }

    Ok(())
}

fn managed_skill_path(slug: &str) -> String {
    format!("{MANAGED_SKILL_ROOT_PREFIX}/{slug}/{SKILL_FRONTMATTER_FILE}")
}

impl BundledAgentSeedSkillSource {
    fn skill_path(&self) -> String {
        managed_skill_path(&self.slug)
    }
}
