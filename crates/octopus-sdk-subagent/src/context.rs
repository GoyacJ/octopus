use std::{collections::BTreeSet, sync::Arc};

use async_trait::async_trait;
use octopus_sdk_context::DurableScratchpad;
use octopus_sdk_contracts::{
    PermissionGate, PermissionMode, PermissionOutcome, SessionId, SubagentSpec, TaskBudget,
    ToolCallRequest, Usage,
};
use octopus_sdk_hooks::HookRunner;
use octopus_sdk_model::ModelProvider;
use octopus_sdk_session::SessionStore;
use octopus_sdk_tools::ToolRegistry;

#[derive(Clone)]
pub struct ParentSessionContext {
    pub session_id: SessionId,
    pub session_store: Arc<dyn SessionStore>,
    pub model: Arc<dyn ModelProvider>,
    pub tools: Arc<ToolRegistry>,
    pub permissions: Arc<dyn PermissionGate>,
    pub scratchpad: DurableScratchpad,
}

pub struct SubagentContext {
    pub parent_session: SessionId,
    pub session_store: Arc<dyn SessionStore>,
    pub model: Arc<dyn ModelProvider>,
    pub tools: Arc<ToolRegistry>,
    pub permissions: Arc<dyn PermissionGate>,
    pub hooks: Arc<HookRunner>,
    pub scratchpad: DurableScratchpad,
    pub spec: SubagentSpec,
    pub depth: u8,
    turns: u16,
    tokens_used: u32,
}

impl SubagentContext {
    #[must_use]
    pub fn new(
        parent_session: SessionId,
        session_store: Arc<dyn SessionStore>,
        model: Arc<dyn ModelProvider>,
        tools: Arc<ToolRegistry>,
        permissions: Arc<dyn PermissionGate>,
        scratchpad: DurableScratchpad,
        spec: SubagentSpec,
    ) -> Self {
        let depth = spec.depth;
        Self {
            parent_session,
            session_store,
            model,
            tools,
            permissions,
            hooks: Arc::new(HookRunner::new()),
            scratchpad,
            spec,
            depth,
            turns: 0,
            tokens_used: 0,
        }
    }

    #[must_use]
    pub fn from_parent(parent: ParentSessionContext, spec: SubagentSpec) -> Self {
        let allowed = spec.allowed_tools.iter().cloned().collect::<BTreeSet<_>>();
        let tools = Arc::new(filtered_registry(parent.tools.as_ref(), &allowed));
        let permissions = Arc::new(FilteredPermissionGate::new(parent.permissions, allowed));

        Self::new(
            parent.session_id,
            parent.session_store,
            parent.model,
            tools,
            permissions,
            parent.scratchpad,
            spec,
        )
    }

    #[must_use]
    pub fn for_evaluator(parent: ParentSessionContext, draft: &crate::Draft) -> Self {
        let evaluator_id = match &draft.content {
            octopus_sdk_contracts::SubagentOutput::Summary { meta, .. }
            | octopus_sdk_contracts::SubagentOutput::FileRef { meta, .. }
            | octopus_sdk_contracts::SubagentOutput::Json { meta, .. } => {
                format!("{}-evaluator", meta.session_id.0)
            }
        };

        Self::new(
            parent.session_id,
            parent.session_store,
            parent.model,
            Arc::new(ToolRegistry::new()),
            Arc::new(FilteredPermissionGate::new(
                parent.permissions,
                BTreeSet::new(),
            )),
            parent.scratchpad,
            SubagentSpec {
                id: evaluator_id,
                system_prompt: "Judge the generator draft.".into(),
                allowed_tools: Vec::new(),
                model_role: "subagent-evaluator".into(),
                permission_mode: PermissionMode::Default,
                task_budget: TaskBudget::default(),
                max_turns: 1,
                depth: 1,
            },
        )
    }

    #[must_use]
    pub fn allowed_tools(&self) -> Vec<String> {
        self.tools
            .iter()
            .map(|(name, _)| name.to_string())
            .collect::<Vec<_>>()
    }

    pub fn on_turn_end(&mut self, usage: &Usage) {
        self.turns = self.turns.saturating_add(1);
        self.tokens_used = self.tokens_used.saturating_add(total_usage_tokens(usage));
    }

    #[must_use]
    pub fn completion_threshold_reached(&self) -> bool {
        if self.spec.task_budget.total == 0 {
            return false;
        }

        let threshold = (f64::from(self.spec.task_budget.total)
            * f64::from(self.spec.task_budget.completion_threshold))
        .ceil();
        f64::from(self.tokens_used) >= threshold
    }

    #[must_use]
    pub const fn turns(&self) -> u16 {
        self.turns
    }

    #[must_use]
    pub const fn tokens_used(&self) -> u32 {
        self.tokens_used
    }
}

#[derive(Clone)]
struct FilteredPermissionGate {
    parent: Arc<dyn PermissionGate>,
    allowed_tools: BTreeSet<String>,
}

impl FilteredPermissionGate {
    fn new(parent: Arc<dyn PermissionGate>, allowed_tools: BTreeSet<String>) -> Self {
        Self {
            parent,
            allowed_tools,
        }
    }
}

#[async_trait]
impl PermissionGate for FilteredPermissionGate {
    async fn check(&self, call: &ToolCallRequest) -> PermissionOutcome {
        if !self.allowed_tools.contains(call.name.as_str()) {
            return PermissionOutcome::Deny {
                reason: format!("tool '{}' is not allowed for subagent", call.name),
            };
        }

        self.parent.check(call).await
    }
}

fn filtered_registry(parent: &ToolRegistry, allowed_tools: &BTreeSet<String>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    for (name, tool) in parent.iter() {
        if !allowed_tools.contains(name) {
            continue;
        }

        registry
            .register(Arc::clone(tool))
            .expect("filtered tool registry should stay duplicate-free");
    }

    registry
}

fn total_usage_tokens(usage: &Usage) -> u32 {
    usage.input_tokens
        + usage.output_tokens
        + usage.cache_creation_input_tokens
        + usage.cache_read_input_tokens
}
