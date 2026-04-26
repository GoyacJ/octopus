use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use harness_contracts::{
    ShadowReason, ToolCapability, ToolDescriptor, ToolGroup, ToolOrigin, TrustLevel,
};
use parking_lot::RwLock;

use crate::{Tool, ToolRegistryBuilder};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RegistrationError {
    #[error("duplicate tool name: {0}")]
    Duplicate(String),
    #[error("trust violation: required {required:?}, got {provided:?}")]
    TrustViolation {
        required: TrustLevel,
        provided: TrustLevel,
    },
    #[error("capability not permitted for trust level {trust:?}: {cap}")]
    CapabilityNotPermitted {
        trust: TrustLevel,
        cap: ToolCapability,
    },
    #[error("invalid descriptor: {0}")]
    InvalidDescriptor(String),
    #[error("tool not found: {0}")]
    NotFound(String),
}

#[derive(Clone)]
pub struct ToolRegistry {
    inner: Arc<RwLock<ToolRegistryInner>>,
}

#[derive(Default)]
struct ToolRegistryInner {
    tools: BTreeMap<String, RegisteredTool>,
    shadowed: Vec<ShadowedRegistration>,
    generation: u64,
}

#[derive(Clone)]
struct RegisteredTool {
    tool: Arc<dyn Tool>,
    descriptor: Arc<ToolDescriptor>,
    origin: ToolOrigin,
    trust_level: TrustLevel,
}

impl ToolRegistry {
    pub fn builder() -> ToolRegistryBuilder {
        ToolRegistryBuilder::new()
    }

    pub(crate) fn empty() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ToolRegistryInner::default())),
        }
    }

    pub fn register(&self, tool: Box<dyn Tool>) -> Result<(), RegistrationError> {
        let descriptor = tool.descriptor().clone();
        validate_descriptor(&descriptor)?;
        validate_capabilities(&descriptor)?;

        let name = descriptor.name.clone();
        let origin = descriptor.origin.clone();
        let trust_level = descriptor.trust_level;
        let registered = RegisteredTool {
            tool: tool.into(),
            descriptor: Arc::new(descriptor),
            origin,
            trust_level,
        };

        let mut inner = self.inner.write();
        if let Some(existing) = inner.tools.get(&name).cloned() {
            match resolve_shadow(&existing, &registered) {
                RegistrationDecision::KeepExisting(reason) => {
                    inner.shadowed.push(ShadowedRegistration {
                        name,
                        kept: existing.origin,
                        rejected: registered.origin,
                        reason,
                        at: Utc::now(),
                    });
                    inner.generation += 1;
                    return Ok(());
                }
                RegistrationDecision::ReplaceExisting(reason) => {
                    inner.shadowed.push(ShadowedRegistration {
                        name: name.clone(),
                        kept: registered.origin.clone(),
                        rejected: existing.origin,
                        reason,
                        at: Utc::now(),
                    });
                    inner.tools.insert(name, registered);
                    inner.generation += 1;
                    return Ok(());
                }
            }
        }

        inner.tools.insert(name, registered);
        inner.generation += 1;
        Ok(())
    }

    pub fn deregister(&self, name: &str) -> Result<(), RegistrationError> {
        let mut inner = self.inner.write();
        if inner.tools.remove(name).is_none() {
            return Err(RegistrationError::NotFound(name.to_owned()));
        }
        inner.generation += 1;
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.inner
            .read()
            .tools
            .get(name)
            .map(|tool| Arc::clone(&tool.tool))
    }

    pub fn snapshot(&self) -> ToolRegistrySnapshot {
        let inner = self.inner.read();
        ToolRegistrySnapshot {
            tools: Arc::new(
                inner
                    .tools
                    .iter()
                    .map(|(name, tool)| (name.clone(), Arc::clone(&tool.tool)))
                    .collect(),
            ),
            descriptors: Arc::new(
                inner
                    .tools
                    .iter()
                    .map(|(name, tool)| (name.clone(), Arc::clone(&tool.descriptor)))
                    .collect(),
            ),
            generation: inner.generation,
        }
    }

    pub fn shadowed(&self) -> Vec<ShadowedRegistration> {
        self.inner.read().shadowed.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowedRegistration {
    pub name: String,
    pub kept: ToolOrigin,
    pub rejected: ToolOrigin,
    pub reason: ShadowReason,
    pub at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct ToolRegistrySnapshot {
    tools: Arc<BTreeMap<String, Arc<dyn Tool>>>,
    descriptors: Arc<BTreeMap<String, Arc<ToolDescriptor>>>,
    generation: u64,
}

impl ToolRegistrySnapshot {
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    pub fn descriptor(&self, name: &str) -> Option<&Arc<ToolDescriptor>> {
        self.descriptors.get(name)
    }

    pub fn iter_sorted(&self) -> impl Iterator<Item = (&String, &Arc<dyn Tool>)> {
        self.tools.iter()
    }

    pub fn as_descriptors(&self) -> Vec<&ToolDescriptor> {
        self.descriptors
            .values()
            .map(std::convert::AsRef::as_ref)
            .collect()
    }

    pub fn by_group(&self, group: &ToolGroup) -> Vec<&Arc<dyn Tool>> {
        self.tools
            .iter()
            .filter_map(|(name, tool)| {
                (self.descriptors.get(name)?.group == *group).then_some(tool)
            })
            .collect()
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

enum RegistrationDecision {
    KeepExisting(ShadowReason),
    ReplaceExisting(ShadowReason),
}

fn resolve_shadow(existing: &RegisteredTool, incoming: &RegisteredTool) -> RegistrationDecision {
    match (&existing.origin, &incoming.origin) {
        (ToolOrigin::Builtin, ToolOrigin::Builtin) => {
            RegistrationDecision::KeepExisting(ShadowReason::Duplicate)
        }
        (ToolOrigin::Builtin, _) => RegistrationDecision::KeepExisting(ShadowReason::BuiltinWins),
        (_, ToolOrigin::Builtin) => {
            RegistrationDecision::ReplaceExisting(ShadowReason::BuiltinWins)
        }
        _ if trust_rank(incoming.trust_level) > trust_rank(existing.trust_level) => {
            RegistrationDecision::ReplaceExisting(ShadowReason::HigherTrust)
        }
        _ if trust_rank(incoming.trust_level) < trust_rank(existing.trust_level) => {
            RegistrationDecision::KeepExisting(ShadowReason::HigherTrust)
        }
        _ => RegistrationDecision::KeepExisting(ShadowReason::Duplicate),
    }
}

fn trust_rank(trust: TrustLevel) -> u8 {
    match trust {
        TrustLevel::AdminTrusted => 1,
        _ => 0,
    }
}

fn validate_descriptor(descriptor: &ToolDescriptor) -> Result<(), RegistrationError> {
    if descriptor.name.trim().is_empty() {
        return Err(RegistrationError::InvalidDescriptor(
            "tool name must not be empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_capabilities(descriptor: &ToolDescriptor) -> Result<(), RegistrationError> {
    for cap in &descriptor.required_capabilities {
        if !capability_allowed(descriptor.trust_level, cap) {
            return Err(RegistrationError::CapabilityNotPermitted {
                trust: descriptor.trust_level,
                cap: cap.clone(),
            });
        }
    }
    Ok(())
}

fn capability_allowed(trust: TrustLevel, cap: &ToolCapability) -> bool {
    match cap {
        ToolCapability::BlobReader | ToolCapability::TodoStore => true,
        _ => trust == TrustLevel::AdminTrusted,
    }
}
