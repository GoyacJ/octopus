use futures::future::BoxFuture;
use harness_contracts::{
    AgentId, RenderedSkill as ContractRenderedSkill, SkillFilter, SkillInjectionId,
    SkillInvocationReceipt, SkillRegistryCap, SkillShellInvocation, SkillSummary, SkillView,
    ToolError,
};
use serde_json::Value;

use crate::{RenderError, SkillRegistry, SkillRenderer};

#[derive(Clone)]
pub struct SkillRegistryService {
    registry: SkillRegistry,
    renderer: SkillRenderer,
}

impl SkillRegistryService {
    #[must_use]
    pub fn new(registry: SkillRegistry, renderer: SkillRenderer) -> Self {
        Self { registry, renderer }
    }

    #[must_use]
    pub fn list_summaries(&self, agent: &AgentId, filter: SkillFilter) -> Vec<SkillSummary> {
        self.registry.list_summaries_for_agent(agent, filter)
    }

    #[must_use]
    pub fn view(&self, agent: &AgentId, name: &str, full: bool) -> Option<SkillView> {
        self.registry.view(agent, name, full)
    }

    pub async fn render(
        &self,
        agent: &AgentId,
        name: &str,
        params: Value,
    ) -> Result<ContractRenderedSkill, RenderError> {
        if self.registry.view(agent, name, false).is_none() {
            return Err(RenderError::SkillNotVisible(name.to_owned()));
        }

        let skill = self
            .registry
            .get(name)
            .ok_or_else(|| RenderError::SkillNotVisible(name.to_owned()))?;
        self.renderer
            .render(&skill, params)
            .await
            .map(ContractRenderedSkill::from)
    }

    pub async fn invoke(
        &self,
        agent: &AgentId,
        name: &str,
        params: Value,
    ) -> Result<SkillInvocationReceipt, RenderError> {
        let rendered = self.render(agent, name, params).await?;
        Ok(SkillInvocationReceipt {
            skill_name: rendered.skill_name,
            injection_id: SkillInjectionId(new_injection_id(name)),
            bytes_injected: rendered.content.as_bytes().len() as u64,
            consumed_config_keys: rendered.consumed_config_keys,
        })
    }
}

impl SkillRegistryCap for SkillRegistryService {
    fn list_summaries(&self, agent: &AgentId, filter: SkillFilter) -> Vec<SkillSummary> {
        self.list_summaries(agent, filter)
    }

    fn view(&self, agent: &AgentId, name: &str, full: bool) -> Option<SkillView> {
        self.view(agent, name, full)
    }

    fn render(
        &self,
        agent: &AgentId,
        name: String,
        params: Value,
    ) -> BoxFuture<'static, Result<ContractRenderedSkill, ToolError>> {
        let service = self.clone();
        let agent = *agent;
        Box::pin(async move {
            service
                .render(&agent, &name, params)
                .await
                .map_err(|error| ToolError::Validation(error.to_string()))
        })
    }
}

impl From<crate::RenderedSkill> for ContractRenderedSkill {
    fn from(rendered: crate::RenderedSkill) -> Self {
        Self {
            skill_id: rendered.skill_id,
            skill_name: rendered.skill_name,
            content: rendered.content,
            shell_invocations: rendered
                .shell_invocations
                .into_iter()
                .map(|invocation| SkillShellInvocation {
                    command: invocation.command,
                    stdout_truncated: invocation.stdout_truncated,
                    exit_code: invocation.exit_code,
                })
                .collect(),
            consumed_config_keys: rendered.consumed_config_keys,
        }
    }
}

fn new_injection_id(name: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("skill:{name}:{nanos}")
}
