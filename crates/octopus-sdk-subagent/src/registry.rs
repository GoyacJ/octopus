use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use octopus_sdk_contracts::{PermissionMode, SubagentError, SubagentSpec, TaskBudget};
use serde::Deserialize;
use tracing::warn;
use walkdir::WalkDir;

#[derive(Debug)]
pub struct AgentRegistry {
    specs: HashMap<String, SubagentSpec>,
}

impl AgentRegistry {
    pub fn discover(roots: &[PathBuf]) -> Result<Self, SubagentError> {
        let mut specs = HashMap::new();

        for root in roots {
            for entry in WalkDir::new(root)
                .follow_links(false)
                .sort_by_file_name()
                .into_iter()
                .filter_map(Result::ok)
            {
                let path = entry.path();
                if !entry.file_type().is_file()
                    || path.extension().and_then(|ext| ext.to_str()) != Some("md")
                    || !is_agent_path(root, path)
                {
                    continue;
                }

                let markdown =
                    fs::read_to_string(path).map_err(|error| SubagentError::Storage {
                        reason: error.to_string(),
                    })?;
                let spec = parse_agent_markdown(&markdown)?;
                specs.insert(spec.id.clone(), spec);
            }
        }

        Ok(Self { specs })
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<&SubagentSpec> {
        self.specs.get(name)
    }

    #[must_use]
    pub fn list(&self) -> Vec<&SubagentSpec> {
        let mut specs = self.specs.values().collect::<Vec<_>>();
        specs.sort_by(|left, right| left.id.cmp(&right.id));
        specs
    }
}

fn is_agent_path(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| relative.components().next())
        .map(|component| component.as_os_str() == ".agents")
        .unwrap_or(false)
}

fn parse_agent_markdown(markdown: &str) -> Result<SubagentSpec, SubagentError> {
    let (frontmatter, body) = split_frontmatter(markdown)?;
    let id = frontmatter.name.trim().to_string();

    validate_agent_id(&id)?;

    Ok(SubagentSpec {
        id,
        system_prompt: body.trim().to_string(),
        allowed_tools: frontmatter.allowed_tools.unwrap_or_default(),
        agent_role: frontmatter.agent_role.unwrap_or_else(|| "worker".into()),
        model_role: normalize_model_role(frontmatter.model.as_deref()),
        permission_mode: PermissionMode::Default,
        task_budget: TaskBudget {
            total: frontmatter.task_budget.unwrap_or_default(),
            completion_threshold: 0.9,
        },
        max_turns: frontmatter.max_turns.unwrap_or(0),
        depth: 1,
    })
}

fn split_frontmatter(markdown: &str) -> Result<(AgentFrontmatter, String), SubagentError> {
    let normalized = markdown.replace("\r\n", "\n");
    let Some(rest) = normalized.strip_prefix("---\n") else {
        return Err(SubagentError::Storage {
            reason: "missing frontmatter".into(),
        });
    };
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        return Err(SubagentError::Storage {
            reason: "missing frontmatter terminator".into(),
        });
    };

    let frontmatter = serde_yaml::from_str::<AgentFrontmatter>(frontmatter).map_err(|error| {
        SubagentError::Storage {
            reason: error.to_string(),
        }
    })?;

    Ok((frontmatter, body.to_string()))
}

fn validate_agent_id(id: &str) -> Result<(), SubagentError> {
    let valid = !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');

    if valid {
        Ok(())
    } else {
        Err(SubagentError::Storage {
            reason: "invalid agent id".into(),
        })
    }
}

fn normalize_model_role(model: Option<&str>) -> String {
    let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) else {
        return "main".into();
    };

    let normalized = model.to_ascii_lowercase().replace('-', "_");
    if matches!(
        normalized.as_str(),
        "main"
            | "fast"
            | "best"
            | "plan"
            | "compact"
            | "vision"
            | "web_extract"
            | "embedding"
            | "eval"
            | "subagent_default"
    ) {
        return normalized;
    }

    warn!(
        model = model,
        "agent registry model hint unresolved; falling back to main"
    );
    "main".into()
}

#[derive(Debug, Deserialize)]
struct AgentFrontmatter {
    name: String,
    model: Option<String>,
    allowed_tools: Option<Vec<String>>,
    agent_role: Option<String>,
    max_turns: Option<u16>,
    task_budget: Option<u32>,
}
