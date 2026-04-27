use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;

use crate::{ConfigResolveError, RenderError, Skill};

#[async_trait]
pub trait SkillConfigResolver: Send + Sync + 'static {
    async fn resolve(&self, key: &str) -> Result<Value, ConfigResolveError>;
    async fn resolve_secret(&self, key: &str) -> Result<String, ConfigResolveError>;
}

#[derive(Clone)]
pub struct SkillRenderer {
    config_resolver: Arc<dyn SkillConfigResolver>,
    shell_allowlist: HashSet<String>,
    max_shell_output: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedSkill {
    pub skill_id: harness_contracts::SkillId,
    pub skill_name: String,
    pub content: String,
    pub shell_invocations: Vec<ShellInvocation>,
    pub consumed_config_keys: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShellInvocation {
    pub command: String,
    pub stdout_truncated: bool,
    pub exit_code: i32,
}

impl SkillRenderer {
    #[must_use]
    pub fn new(config_resolver: Arc<dyn SkillConfigResolver>) -> Self {
        Self {
            config_resolver,
            shell_allowlist: HashSet::new(),
            max_shell_output: 4_000,
        }
    }

    #[must_use]
    pub fn with_shell_allowlist(mut self, cmds: impl IntoIterator<Item = String>) -> Self {
        self.shell_allowlist.extend(cmds);
        self
    }

    #[must_use]
    pub fn with_max_shell_output(mut self, max_shell_output: usize) -> Self {
        self.max_shell_output = max_shell_output;
        self
    }

    pub async fn render(&self, skill: &Skill, params: Value) -> Result<RenderedSkill, RenderError> {
        for parameter in &skill.frontmatter.parameters {
            if parameter.required
                && parameter.default.is_none()
                && params.get(&parameter.name).is_none()
            {
                return Err(RenderError::MissingParam(parameter.name.clone()));
            }
        }

        let mut content = skill.body.clone();
        let mut consumed_config_keys = Vec::new();

        for parameter in &skill.frontmatter.parameters {
            let value = params
                .get(&parameter.name)
                .cloned()
                .or_else(|| parameter.default.clone());
            if let Some(value) = value {
                content = content.replace(
                    &format!("${{{}}}", parameter.name),
                    &render_value_for_template(&value),
                );
            }
        }

        for config in &skill.frontmatter.config {
            let secret_pattern = format!("${{config.{}:secret}}", config.key);
            if content.contains(&secret_pattern) {
                let value = self.config_resolver.resolve_secret(&config.key).await?;
                content = content.replace(&secret_pattern, &value);
                consumed_config_keys.push(config.key.clone());
            }

            let pattern = format!("${{config.{}}}", config.key);
            if content.contains(&pattern) {
                let value = self.config_resolver.resolve(&config.key).await?;
                content = content.replace(&pattern, &render_value_for_template(&value));
                if !consumed_config_keys.iter().any(|key| key == &config.key) {
                    consumed_config_keys.push(config.key.clone());
                }
            }
        }

        let (content, shell_invocations) = self.render_shell_blocks(&content)?;

        Ok(RenderedSkill {
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone(),
            content,
            shell_invocations,
            consumed_config_keys,
        })
    }

    fn render_shell_blocks(
        &self,
        content: &str,
    ) -> Result<(String, Vec<ShellInvocation>), RenderError> {
        let mut output = String::with_capacity(content.len());
        let mut remaining = content;
        let mut invocations = Vec::new();

        while let Some(start) = remaining.find("!`") {
            output.push_str(&remaining[..start]);
            let after_start = &remaining[start + 2..];
            let Some(end) = after_start.find('`') else {
                output.push_str(&remaining[start..]);
                return Ok((output, invocations));
            };
            let command = &after_start[..end];
            if self.command_allowed(command) {
                let rendered = self.execute_shell(command)?;
                output.push_str(&rendered.0);
                invocations.push(rendered.1);
            } else {
                output.push_str("[SHELL_NOT_ALLOWED]");
            }
            remaining = &after_start[end + 1..];
        }

        output.push_str(remaining);
        Ok((output, invocations))
    }

    fn command_allowed(&self, command: &str) -> bool {
        command
            .split_whitespace()
            .next()
            .map(|name| self.shell_allowlist.contains(name))
            .unwrap_or(false)
    }

    fn execute_shell(&self, command: &str) -> Result<(String, ShellInvocation), RenderError> {
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()?;
        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (stdout, stdout_truncated) = truncate_chars(&stdout, self.max_shell_output);
        Ok((
            stdout,
            ShellInvocation {
                command: command.to_owned(),
                stdout_truncated,
                exit_code,
            },
        ))
    }
}

fn render_value_for_template(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> (String, bool) {
    if value.chars().count() <= max_chars {
        return (value.to_owned(), false);
    }

    let truncated = value.chars().take(max_chars).collect::<String>();
    (format!("{truncated}[...truncated]"), true)
}
