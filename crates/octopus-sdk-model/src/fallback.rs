//! Fallback policy logic lands here in later W2 tasks.

use std::collections::HashMap;

use crate::{ModelError, ModelId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FallbackTrigger {
    Overloaded,
    Upstream5xx,
    PromptTooLong,
    ModelDeprecated,
}

#[derive(Debug, Clone)]
pub struct FallbackPolicy {
    chain: Vec<ModelId>,
    triggers: Vec<FallbackTrigger>,
    next_by_model: HashMap<ModelId, ModelId>,
}

impl FallbackPolicy {
    #[must_use]
    pub fn with_route(mut self, current: ModelId, next: ModelId) -> Self {
        if !self.chain.contains(&current) {
            self.chain.push(current.clone());
        }
        if !self.chain.contains(&next) {
            self.chain.push(next.clone());
        }
        self.next_by_model.insert(current, next);
        self
    }

    #[must_use]
    pub fn should_fallback(&self, err: &ModelError) -> Option<FallbackTrigger> {
        let trigger = match err {
            ModelError::Overloaded { .. } => Some(FallbackTrigger::Overloaded),
            ModelError::UpstreamStatus { status, .. } if *status >= 500 => {
                Some(FallbackTrigger::Upstream5xx)
            }
            ModelError::PromptTooLong { .. } => Some(FallbackTrigger::PromptTooLong),
            _ => None,
        }?;

        self.triggers.contains(&trigger).then_some(trigger)
    }

    #[must_use]
    pub fn next_model(&self, current: &ModelId) -> Option<&ModelId> {
        self.next_by_model.get(current).or_else(|| {
            self.chain
                .windows(2)
                .find(|window| window[0] == *current)
                .map(|window| &window[1])
        })
    }
}

impl Default for FallbackPolicy {
    fn default() -> Self {
        Self {
            chain: Vec::new(),
            triggers: vec![
                FallbackTrigger::Overloaded,
                FallbackTrigger::Upstream5xx,
                FallbackTrigger::PromptTooLong,
                FallbackTrigger::ModelDeprecated,
            ],
            next_by_model: HashMap::new(),
        }
    }
}
