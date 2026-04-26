use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use harness_contracts::HookEventKind;
use parking_lot::RwLock;

use crate::HookHandler;

#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum RegistrationError {
    #[error("duplicate hook handler id: {0}")]
    Duplicate(String),
    #[error("invalid hook handler: {0}")]
    InvalidHandler(String),
}

#[derive(Clone)]
pub struct HookRegistry {
    inner: Arc<RwLock<HookRegistryInner>>,
}

#[derive(Default)]
struct HookRegistryInner {
    handlers: Vec<Arc<dyn HookHandler>>,
    ids: HashSet<String>,
    generation: u64,
}

impl HookRegistry {
    pub fn builder() -> HookRegistryBuilder {
        HookRegistryBuilder::new()
    }

    pub fn register(&self, handler: Box<dyn HookHandler>) -> Result<(), RegistrationError> {
        validate_handler(handler.as_ref())?;

        let id = handler.handler_id().to_owned();
        let handler: Arc<dyn HookHandler> = handler.into();
        let mut inner = self.inner.write();
        if !inner.ids.insert(id.clone()) {
            return Err(RegistrationError::Duplicate(id));
        }

        inner.handlers.push(handler);
        inner.generation += 1;
        Ok(())
    }

    pub fn snapshot(&self) -> HookRegistrySnapshot {
        let inner = self.inner.read();
        HookRegistrySnapshot::from_handlers(inner.handlers.clone(), inner.generation)
    }
}

pub struct HookRegistryBuilder {
    handlers: Vec<Box<dyn HookHandler>>,
}

impl HookRegistryBuilder {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_hook(mut self, handler: Box<dyn HookHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    pub fn build(self) -> Result<HookRegistry, RegistrationError> {
        let registry = HookRegistry {
            inner: Arc::new(RwLock::new(HookRegistryInner::default())),
        };

        for handler in self.handlers {
            registry.register(handler)?;
        }

        Ok(registry)
    }
}

impl Default for HookRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default)]
pub struct HookRegistrySnapshot {
    handlers_by_event: Arc<HashMap<HookEventKind, Vec<Arc<dyn HookHandler>>>>,
    generation: u64,
}

impl HookRegistrySnapshot {
    fn from_handlers(handlers: Vec<Arc<dyn HookHandler>>, generation: u64) -> Self {
        let mut handlers_by_event: HashMap<HookEventKind, Vec<Arc<dyn HookHandler>>> =
            HashMap::new();

        for handler in handlers {
            for event in handler.interested_events() {
                handlers_by_event
                    .entry(event.clone())
                    .or_default()
                    .push(Arc::clone(&handler));
            }
        }

        for handlers in handlers_by_event.values_mut() {
            handlers.sort_by(|left, right| {
                right
                    .priority()
                    .cmp(&left.priority())
                    .then_with(|| left.handler_id().cmp(right.handler_id()))
            });
        }

        Self {
            handlers_by_event: Arc::new(handlers_by_event),
            generation,
        }
    }

    pub fn handlers_for(&self, event: HookEventKind) -> Vec<Arc<dyn HookHandler>> {
        self.handlers_by_event
            .get(&event)
            .cloned()
            .unwrap_or_default()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

fn validate_handler(handler: &dyn HookHandler) -> Result<(), RegistrationError> {
    if handler.handler_id().trim().is_empty() {
        return Err(RegistrationError::InvalidHandler(
            "handler_id must not be empty".to_owned(),
        ));
    }
    if handler.interested_events().is_empty() {
        return Err(RegistrationError::InvalidHandler(
            "interested_events must not be empty".to_owned(),
        ));
    }
    Ok(())
}
