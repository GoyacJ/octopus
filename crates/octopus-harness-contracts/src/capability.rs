//! Tool capability marker traits.
//!
//! SPEC: docs/architecture/harness/crates/harness-contracts.md §3.4

use std::{any::Any, collections::HashMap, sync::Arc};

use crate::ToolCapability;

pub trait SubagentRunnerCap: Send + Sync + 'static {}
pub trait TodoStoreCap: Send + Sync + 'static {}
pub trait RunCancellerCap: Send + Sync + 'static {}
pub trait ClarifyChannelCap: Send + Sync + 'static {}
pub trait UserMessengerCap: Send + Sync + 'static {}
pub trait BlobReaderCap: Send + Sync + 'static {}
pub trait HookEmitterCap: Send + Sync + 'static {}
pub trait SkillRegistryCap: Send + Sync + 'static {}
pub trait EmbeddedToolDispatcherCap: Send + Sync + 'static {}
pub trait CodeRuntimeCap: Send + Sync + 'static {}

#[derive(Clone, Default)]
pub struct CapabilityRegistry {
    inner: HashMap<ToolCapability, Arc<dyn Any + Send + Sync>>,
}

impl CapabilityRegistry {
    pub fn install<T>(&mut self, capability: ToolCapability, implementation: Arc<T>)
    where
        T: ?Sized + Send + Sync + 'static,
    {
        self.inner.insert(capability, Arc::new(implementation));
    }

    pub fn get<T>(&self, capability: &ToolCapability) -> Option<Arc<T>>
    where
        T: ?Sized + Send + Sync + 'static,
    {
        let erased = Arc::clone(self.inner.get(capability)?);
        erased
            .downcast::<Arc<T>>()
            .ok()
            .map(|typed| Arc::clone(typed.as_ref()))
    }
}
