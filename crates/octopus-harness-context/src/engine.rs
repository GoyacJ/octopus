use std::sync::Arc;

use harness_contracts::{ContextError, ContextStageId, MessagePart, ToolResultEnvelope, TurnInput};
use harness_model::AuxModelProvider;

use crate::{
    AssembledPrompt, AutocompactProvider, CompactHint, ContextBuffer, ContextOutcome,
    ContextProvider, ContextSessionView, FrozenContext, MicrocompactProvider, PromptCachePolicy,
    TokenBudget,
};

const COMPACT_STAGE_ORDER: [ContextStageId; 5] = [
    ContextStageId::ToolResultBudget,
    ContextStageId::Snip,
    ContextStageId::Microcompact,
    ContextStageId::Collapse,
    ContextStageId::Autocompact,
];

#[derive(Clone)]
pub struct ContextEngine {
    providers: Vec<Arc<dyn ContextProvider>>,
    budget: TokenBudget,
    cache_policy: PromptCachePolicy,
}

impl ContextEngine {
    pub fn builder() -> ContextEngineBuilder {
        ContextEngineBuilder::default()
    }

    pub fn compact_stage_order() -> &'static [ContextStageId] {
        &COMPACT_STAGE_ORDER
    }

    pub async fn compact(
        &self,
        ctx: &mut ContextBuffer,
        hint: CompactHint,
    ) -> Result<ContextOutcome, ContextError> {
        let mut bytes_saved = 0_u64;
        let mut modified = false;

        for stage in &COMPACT_STAGE_ORDER {
            for provider in self
                .providers
                .iter()
                .filter(|provider| provider.stage() == *stage)
            {
                let frozen_before = ctx.frozen.clone();
                let outcome = provider.apply(ctx, &hint).await?;
                self.refresh_context(ctx, &frozen_before)?;

                match outcome {
                    ContextOutcome::NoChange => {}
                    ContextOutcome::Modified { bytes_saved: saved } => {
                        modified = true;
                        bytes_saved = bytes_saved.saturating_add(saved);
                    }
                    forked @ ContextOutcome::Forked { .. } => return Ok(forked),
                }
            }
        }

        if modified {
            Ok(ContextOutcome::Modified { bytes_saved })
        } else {
            Ok(ContextOutcome::NoChange)
        }
    }

    fn refresh_context(
        &self,
        ctx: &mut ContextBuffer,
        frozen_before: &FrozenContext,
    ) -> Result<(), ContextError> {
        if &ctx.frozen != frozen_before {
            return Err(ContextError::Internal(
                "context provider mutated frozen context".to_owned(),
            ));
        }

        ctx.rebuild_tool_use_pairs();
        ctx.bookkeeping.estimated_tokens =
            estimate_tokens(ctx.frozen.system_header.as_deref(), &ctx.active.history);
        ctx.bookkeeping.budget_snapshot = self.budget;
        Ok(())
    }

    pub async fn assemble(
        &self,
        session: &dyn ContextSessionView,
        turn_input: &TurnInput,
    ) -> Result<AssembledPrompt, ContextError> {
        let mut messages = session.messages();
        messages.push(turn_input.message.clone());

        let tokens_estimate = estimate_tokens(session.system().as_deref(), &messages);
        let budget_utilization =
            budget_utilization(tokens_estimate, self.budget.max_tokens_per_turn);

        Ok(AssembledPrompt {
            messages,
            system: session.system(),
            tools_snapshot: session.tools_snapshot(),
            cache_breakpoints: Vec::with_capacity(self.cache_policy.max_breakpoints),
            tokens_estimate,
            budget_utilization,
        })
    }

    pub async fn after_turn(
        &self,
        _session: &dyn ContextSessionView,
        _results: &[ToolResultEnvelope],
    ) -> Result<ContextOutcome, ContextError> {
        Ok(ContextOutcome::NoChange)
    }
}

#[derive(Default)]
pub struct ContextEngineBuilder {
    providers: Vec<Arc<dyn ContextProvider>>,
    aux_provider: Option<Arc<dyn AuxModelProvider>>,
    budget: TokenBudget,
    cache_policy: PromptCachePolicy,
}

impl ContextEngineBuilder {
    #[must_use]
    pub fn with_provider(mut self, provider: impl ContextProvider) -> Self {
        self.providers.push(Arc::new(provider));
        self
    }

    #[must_use]
    pub fn with_budget(mut self, budget: TokenBudget) -> Self {
        self.budget = budget;
        self
    }

    #[must_use]
    pub fn with_cache_policy(mut self, cache_policy: PromptCachePolicy) -> Self {
        self.cache_policy = cache_policy;
        self
    }

    #[must_use]
    pub fn with_aux_provider(mut self, aux_provider: Arc<dyn AuxModelProvider>) -> Self {
        self.aux_provider = Some(aux_provider);
        self
    }

    pub fn build(mut self) -> Result<ContextEngine, ContextError> {
        if let Some(aux_provider) = &self.aux_provider {
            self.providers
                .push(Arc::new(MicrocompactProvider::new(aux_provider.clone())));
            self.providers.push(Arc::new(AutocompactProvider::new(Some(
                aux_provider.clone(),
            ))));
        }
        self.providers
            .sort_by(|left, right| compare_providers(left.as_ref(), right.as_ref()));

        Ok(ContextEngine {
            providers: self.providers,
            budget: self.budget,
            cache_policy: self.cache_policy,
        })
    }
}

fn compare_providers(
    left: &dyn ContextProvider,
    right: &dyn ContextProvider,
) -> std::cmp::Ordering {
    stage_rank(left.stage())
        .cmp(&stage_rank(right.stage()))
        .then_with(|| left.provider_id().cmp(right.provider_id()))
}

fn stage_rank(stage: ContextStageId) -> usize {
    COMPACT_STAGE_ORDER
        .iter()
        .position(|candidate| *candidate == stage)
        .unwrap_or(COMPACT_STAGE_ORDER.len())
}

fn estimate_tokens(system: Option<&str>, messages: &[harness_contracts::Message]) -> u64 {
    let mut chars = system.map(str::len).unwrap_or_default();
    for message in messages {
        for part in &message.parts {
            chars += match part {
                MessagePart::Text(text) => text.len(),
                MessagePart::Image { .. } => 512,
                MessagePart::ToolUse { input, .. } => input.to_string().len(),
                MessagePart::ToolResult { content, .. } => format!("{content:?}").len(),
                MessagePart::Thinking(thinking) => {
                    thinking.text.as_deref().map(str::len).unwrap_or(0)
                }
                _ => 0,
            };
        }
    }
    std::cmp::max(1, chars.div_ceil(4) as u64)
}

pub(crate) fn estimate_message_tokens(message: &harness_contracts::Message) -> u64 {
    estimate_tokens(None, std::slice::from_ref(message))
}

fn budget_utilization(tokens_estimate: u64, max_tokens: u64) -> f32 {
    if max_tokens == 0 {
        return 0.0;
    }
    let per_mille = tokens_estimate
        .saturating_mul(1_000)
        .checked_div(max_tokens)
        .unwrap_or_default()
        .min(u64::from(u16::MAX)) as u16;
    f32::from(per_mille) / 1_000.0
}
