use std::collections::HashMap;
use std::path::PathBuf;

use harness_contracts::SkillId;
use serde_json::{Map, Number, Value};
use yaml_rust2::{Yaml, YamlLoader};

use crate::{
    Skill, SkillConfigDecl, SkillError, SkillFrontmatter, SkillHookDecl, SkillParamType,
    SkillParameter, SkillPlatform, SkillPrerequisites, SkillSource,
};

pub fn parse_skill_markdown(
    markdown: &str,
    source: SkillSource,
    raw_path: Option<PathBuf>,
    runtime_platform: SkillPlatform,
) -> Result<Skill, SkillError> {
    let (frontmatter_yaml, body) = split_frontmatter(markdown)?;
    let docs = YamlLoader::load_from_str(frontmatter_yaml)
        .map_err(|error| SkillError::ParseFrontmatter(error.to_string()))?;
    let yaml = docs.first().unwrap_or(&Yaml::BadValue);
    let frontmatter = parse_frontmatter(yaml)?;

    if frontmatter.name.chars().count() > 64 {
        return Err(SkillError::NameTooLong(frontmatter.name.chars().count()));
    }
    if frontmatter.description.chars().count() > 1024 {
        return Err(SkillError::DescriptionTooLong(
            frontmatter.description.chars().count(),
        ));
    }
    if !frontmatter.platforms.is_empty() && !frontmatter.platforms.contains(&runtime_platform) {
        return Err(SkillError::PlatformMismatch {
            required: frontmatter.platforms.clone(),
        });
    }

    let name = frontmatter.name.clone();
    let description = frontmatter.description.clone();
    Ok(Skill {
        id: SkillId(format!("{}:{name}", source_label(&source))),
        name,
        description,
        source,
        frontmatter,
        body: body.trim_start_matches('\n').to_owned(),
        raw_path,
    })
}

fn split_frontmatter(markdown: &str) -> Result<(&str, &str), SkillError> {
    let markdown = markdown.strip_prefix("---\n").ok_or_else(|| {
        SkillError::ParseFrontmatter("missing opening frontmatter delimiter".to_owned())
    })?;
    let Some((frontmatter, body)) = markdown.split_once("\n---") else {
        return Err(SkillError::ParseFrontmatter(
            "missing closing frontmatter delimiter".to_owned(),
        ));
    };
    Ok((
        frontmatter,
        body.trim_start_matches("\r\n").trim_start_matches('\n'),
    ))
}

fn parse_frontmatter(yaml: &Yaml) -> Result<SkillFrontmatter, SkillError> {
    let name = required_string(yaml, "name")?;
    let description = required_string(yaml, "description")?;
    let metadata = yaml_to_map(yaml_hash_get(yaml, "metadata").unwrap_or(&Yaml::BadValue));
    let octopus_meta = yaml_hash_get(
        yaml_hash_get(yaml, "metadata").unwrap_or(&Yaml::BadValue),
        "octopus",
    );

    let tags = string_vec(yaml_hash_get(yaml, "tags"))
        .or_else(|| octopus_meta.and_then(|meta| string_vec(yaml_hash_get(meta, "tags"))))
        .unwrap_or_default();
    let category = optional_string(yaml_hash_get(yaml, "category"))
        .or_else(|| octopus_meta.and_then(|meta| optional_string(yaml_hash_get(meta, "category"))));

    Ok(SkillFrontmatter {
        name,
        description,
        allowlist_agents: string_vec(yaml_hash_get(yaml, "allowlist_agents")),
        parameters: parse_parameters(yaml_hash_get(yaml, "parameters"))?,
        config: parse_config(yaml_hash_get(yaml, "config"))?,
        platforms: parse_platforms(yaml_hash_get(yaml, "platforms"))?,
        prerequisites: parse_prerequisites(yaml_hash_get(yaml, "prerequisites")),
        hooks: parse_hooks(yaml_hash_get(yaml, "hooks"))?,
        tags,
        category,
        metadata,
    })
}

fn parse_parameters(yaml: Option<&Yaml>) -> Result<Vec<SkillParameter>, SkillError> {
    let Some(Yaml::Array(items)) = yaml else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            let param_type = optional_string(yaml_hash_get(item, "type"))
                .as_deref()
                .and_then(SkillParamType::parse)
                .unwrap_or(SkillParamType::String);
            Ok(SkillParameter {
                name: required_string(item, "name")?,
                param_type,
                required: optional_bool(yaml_hash_get(item, "required")).unwrap_or(false),
                default: yaml_hash_get(item, "default").map(yaml_to_json),
                description: optional_string(yaml_hash_get(item, "description")),
            })
        })
        .collect()
}

fn parse_config(yaml: Option<&Yaml>) -> Result<Vec<SkillConfigDecl>, SkillError> {
    let Some(Yaml::Array(items)) = yaml else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            let value_type = optional_string(yaml_hash_get(item, "type"))
                .as_deref()
                .and_then(SkillParamType::parse)
                .unwrap_or(SkillParamType::String);
            Ok(SkillConfigDecl {
                key: required_string(item, "key")?,
                value_type,
                secret: optional_bool(yaml_hash_get(item, "secret")).unwrap_or(false),
                required: optional_bool(yaml_hash_get(item, "required")).unwrap_or(false),
                default: yaml_hash_get(item, "default").map(yaml_to_json),
                description: optional_string(yaml_hash_get(item, "description")),
            })
        })
        .collect()
}

fn parse_platforms(yaml: Option<&Yaml>) -> Result<Vec<SkillPlatform>, SkillError> {
    let Some(Yaml::Array(items)) = yaml else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .filter_map(|item| item.as_str())
        .map(|value| {
            SkillPlatform::parse(value)
                .ok_or_else(|| SkillError::ParseFrontmatter(format!("unknown platform: {value}")))
        })
        .collect()
}

fn parse_prerequisites(yaml: Option<&Yaml>) -> SkillPrerequisites {
    let Some(yaml) = yaml else {
        return SkillPrerequisites::default();
    };
    SkillPrerequisites {
        env_vars: string_vec(yaml_hash_get(yaml, "env_vars")).unwrap_or_default(),
        commands: string_vec(yaml_hash_get(yaml, "commands")).unwrap_or_default(),
    }
}

fn parse_hooks(yaml: Option<&Yaml>) -> Result<Vec<SkillHookDecl>, SkillError> {
    let Some(Yaml::Array(items)) = yaml else {
        return Ok(Vec::new());
    };
    items
        .iter()
        .map(|item| {
            Ok(SkillHookDecl {
                id: required_string(item, "id")?,
            })
        })
        .collect()
}

fn yaml_hash_get<'a>(yaml: &'a Yaml, key: &str) -> Option<&'a Yaml> {
    let Yaml::Hash(hash) = yaml else {
        return None;
    };
    hash.get(&Yaml::String(key.to_owned()))
}

fn required_string(yaml: &Yaml, key: &str) -> Result<String, SkillError> {
    optional_string(yaml_hash_get(yaml, key))
        .ok_or_else(|| SkillError::ParseFrontmatter(format!("missing required field: {key}")))
}

fn optional_string(yaml: Option<&Yaml>) -> Option<String> {
    yaml.and_then(Yaml::as_str).map(ToOwned::to_owned)
}

fn optional_bool(yaml: Option<&Yaml>) -> Option<bool> {
    yaml.and_then(Yaml::as_bool)
}

fn string_vec(yaml: Option<&Yaml>) -> Option<Vec<String>> {
    match yaml? {
        Yaml::Array(values) => Some(
            values
                .iter()
                .filter_map(Yaml::as_str)
                .map(ToOwned::to_owned)
                .collect(),
        ),
        Yaml::String(value) => Some(vec![value.clone()]),
        _ => None,
    }
}

fn yaml_to_map(yaml: &Yaml) -> HashMap<String, Value> {
    let Value::Object(map) = yaml_to_json(yaml) else {
        return HashMap::new();
    };
    map.into_iter().collect()
}

fn yaml_to_json(yaml: &Yaml) -> Value {
    match yaml {
        Yaml::Real(value) => value
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Yaml::Integer(value) => Value::Number(Number::from(*value)),
        Yaml::String(value) => Value::String(value.clone()),
        Yaml::Boolean(value) => Value::Bool(*value),
        Yaml::Array(values) => Value::Array(values.iter().map(yaml_to_json).collect()),
        Yaml::Hash(hash) => {
            let mut map = Map::new();
            for (key, value) in hash {
                if let Some(key) = key.as_str() {
                    map.insert(key.to_owned(), yaml_to_json(value));
                }
            }
            Value::Object(map)
        }
        Yaml::Null | Yaml::BadValue | Yaml::Alias(_) => Value::Null,
    }
}

fn source_label(source: &SkillSource) -> &'static str {
    match source {
        SkillSource::Bundled => "bundled",
        SkillSource::Workspace(_) => "workspace",
        SkillSource::User(_) => "user",
        SkillSource::Plugin(_) => "plugin",
        SkillSource::Mcp(_) => "mcp",
    }
}
