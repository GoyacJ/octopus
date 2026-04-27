use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use harness_contracts::{
    AgentId, SkillFilter, SkillParameterInfo, SkillStatus, SkillSummary, SkillView,
};

use crate::{Skill, SkillParamType, SkillSource};

#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    inner: Arc<RwLock<SkillRegistryInner>>,
}

#[derive(Debug, Clone, Default)]
struct SkillRegistryInner {
    by_name: BTreeMap<String, Arc<Skill>>,
    status: BTreeMap<String, SkillStatus>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillRegistryBuilder {
    skills: Vec<Skill>,
}

impl SkillRegistry {
    #[must_use]
    pub fn builder() -> SkillRegistryBuilder {
        SkillRegistryBuilder::default()
    }

    pub fn register(&self, skill: Skill) {
        let mut inner = self.inner.write().expect("skill registry poisoned");
        let status = status_for(&skill);
        match inner.by_name.get(&skill.name) {
            Some(existing) if source_rank(&existing.source) >= source_rank(&skill.source) => {}
            _ => {
                inner.status.insert(skill.name.clone(), status);
                inner.by_name.insert(skill.name.clone(), Arc::new(skill));
            }
        }
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<Skill>> {
        self.inner
            .read()
            .expect("skill registry poisoned")
            .by_name
            .get(name)
            .cloned()
    }

    #[must_use]
    pub fn list_available_for_agent(&self, agent: &AgentId) -> Vec<Arc<Skill>> {
        self.inner
            .read()
            .expect("skill registry poisoned")
            .by_name
            .values()
            .filter(|skill| visible_to_agent(skill, agent))
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn list_summaries_for_agent(
        &self,
        agent: &AgentId,
        filter: SkillFilter,
    ) -> Vec<SkillSummary> {
        let inner = self.inner.read().expect("skill registry poisoned");
        inner
            .by_name
            .values()
            .filter(|skill| visible_to_agent(skill, agent))
            .filter_map(|skill| {
                let status = inner
                    .status
                    .get(&skill.name)
                    .cloned()
                    .unwrap_or(SkillStatus::Ready);
                if !filter.include_prerequisite_missing
                    && matches!(status, SkillStatus::PrerequisiteMissing { .. })
                {
                    return None;
                }
                if let Some(tag) = &filter.tag {
                    if !skill
                        .frontmatter
                        .tags
                        .iter()
                        .any(|candidate| candidate == tag)
                    {
                        return None;
                    }
                }
                if let Some(category) = &filter.category {
                    if skill.frontmatter.category.as_ref() != Some(category) {
                        return None;
                    }
                }
                Some(SkillSummary {
                    name: skill.name.clone(),
                    description: skill.description.clone(),
                    tags: skill.frontmatter.tags.clone(),
                    category: skill.frontmatter.category.clone(),
                    source: skill.source.to_kind(),
                    status,
                })
            })
            .collect()
    }

    #[must_use]
    pub fn view(&self, agent: &AgentId, name: &str, full: bool) -> Option<SkillView> {
        let inner = self.inner.read().expect("skill registry poisoned");
        let skill = inner.by_name.get(name)?;
        if !visible_to_agent(skill, agent) {
            return None;
        }
        let status = inner
            .status
            .get(&skill.name)
            .cloned()
            .unwrap_or(SkillStatus::Ready);
        Some(SkillView {
            summary: SkillSummary {
                name: skill.name.clone(),
                description: skill.description.clone(),
                tags: skill.frontmatter.tags.clone(),
                category: skill.frontmatter.category.clone(),
                source: skill.source.to_kind(),
                status,
            },
            parameters: skill
                .frontmatter
                .parameters
                .iter()
                .map(|parameter| SkillParameterInfo {
                    name: parameter.name.clone(),
                    param_type: param_type_name(parameter.param_type).to_owned(),
                    required: parameter.required,
                    default: parameter.default.clone(),
                    description: parameter.description.clone(),
                })
                .collect(),
            config_keys: skill
                .frontmatter
                .config
                .iter()
                .map(|config| config.key.clone())
                .collect(),
            body_preview: preview_chars(&skill.body, 1024),
            body_full: full.then(|| skill.body.clone()),
        })
    }
}

impl SkillRegistryBuilder {
    #[must_use]
    pub fn with_skill(mut self, skill: Skill) -> Self {
        self.skills.push(skill);
        self
    }

    #[must_use]
    pub fn with_skills(mut self, skills: Vec<Skill>) -> Self {
        self.skills.extend(skills);
        self
    }

    #[must_use]
    pub fn build(self) -> SkillRegistry {
        let registry = SkillRegistry::default();
        for skill in self.skills {
            registry.register(skill);
        }
        registry
    }
}

fn visible_to_agent(skill: &Skill, agent: &AgentId) -> bool {
    skill
        .frontmatter
        .allowlist_agents
        .as_ref()
        .map(|list| list.iter().any(|candidate| candidate == &agent.to_string()))
        .unwrap_or(true)
}

fn status_for(skill: &Skill) -> SkillStatus {
    let missing = skill
        .frontmatter
        .prerequisites
        .env_vars
        .iter()
        .filter(|name| std::env::var_os(name).is_none())
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        SkillStatus::Ready
    } else {
        SkillStatus::PrerequisiteMissing { env_vars: missing }
    }
}

fn source_rank(source: &SkillSource) -> u8 {
    match source {
        SkillSource::Bundled => 0,
        SkillSource::Plugin(_) => 1,
        SkillSource::Mcp(_) => 2,
        SkillSource::User(_) => 3,
        SkillSource::Workspace(_) => 4,
    }
}

fn param_type_name(param_type: SkillParamType) -> &'static str {
    match param_type {
        SkillParamType::String => "string",
        SkillParamType::Number => "number",
        SkillParamType::Boolean => "boolean",
        SkillParamType::Path => "path",
        SkillParamType::Url => "url",
    }
}

fn preview_chars(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}
