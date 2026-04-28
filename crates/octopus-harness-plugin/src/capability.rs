use std::collections::BTreeSet;
use std::sync::Arc;

use async_trait::async_trait;
use harness_hook::HookHandler;
use harness_mcp::McpServerSpec;
use harness_skill::Skill;
use harness_tool::Tool;
use parking_lot::Mutex;

use crate::{CapabilitySlot, PluginManifest, RegistrationError};

#[async_trait]
pub trait ToolRegistration: Send + Sync {
    async fn register(&self, tool: Box<dyn Tool>) -> Result<(), RegistrationError>;
    fn pending_declared(&self) -> Vec<String>;
}

#[async_trait]
pub trait HookRegistration: Send + Sync {
    async fn register(&self, handler: Box<dyn HookHandler>) -> Result<(), RegistrationError>;
    fn pending_declared(&self) -> Vec<String>;
}

#[async_trait]
pub trait McpRegistration: Send + Sync {
    async fn register(
        &self,
        server: McpServerSpec,
    ) -> Result<harness_contracts::McpServerId, RegistrationError>;
    fn pending_declared(&self) -> Vec<String>;
}

#[async_trait]
pub trait SkillRegistration: Send + Sync {
    async fn register(&self, skill: Skill) -> Result<(), RegistrationError>;
    fn pending_declared(&self) -> Vec<String>;
}

#[async_trait]
pub trait MemoryProviderRegistration: Send + Sync {
    async fn register(
        &self,
        provider: Arc<dyn harness_memory::MemoryProvider>,
    ) -> Result<(), RegistrationError>;
}

#[async_trait]
pub trait CoordinatorStrategy: Send + Sync + 'static {}

#[async_trait]
pub trait CoordinatorStrategyRegistration: Send + Sync {
    async fn register(
        &self,
        strategy: Arc<dyn CoordinatorStrategy>,
    ) -> Result<(), RegistrationError>;
}

#[derive(Debug, Default)]
pub(crate) struct CapabilityRegistrationState {
    tools: Mutex<BTreeSet<String>>,
    hooks: Mutex<BTreeSet<String>>,
    mcp: Mutex<BTreeSet<String>>,
    skills: Mutex<BTreeSet<String>>,
    memory_registered: Mutex<bool>,
    coordinator_registered: Mutex<bool>,
}

impl CapabilityRegistrationState {
    pub(crate) fn registered_tools(&self) -> Vec<String> {
        sorted_strings(&self.tools)
    }

    pub(crate) fn registered_hooks(&self) -> Vec<String> {
        sorted_strings(&self.hooks)
    }

    pub(crate) fn registered_mcp(&self) -> Vec<String> {
        sorted_strings(&self.mcp)
    }

    pub(crate) fn registered_skills(&self) -> Vec<String> {
        sorted_strings(&self.skills)
    }

    pub(crate) fn coordinator_registered(&self) -> bool {
        *self.coordinator_registered.lock()
    }
}

fn sorted_strings(values: &Mutex<BTreeSet<String>>) -> Vec<String> {
    values.lock().iter().cloned().collect()
}

pub(crate) struct ScopedToolRegistration {
    declared: BTreeSet<String>,
    state: Arc<CapabilityRegistrationState>,
}

impl ScopedToolRegistration {
    pub(crate) fn new(manifest: &PluginManifest, state: Arc<CapabilityRegistrationState>) -> Self {
        Self {
            declared: manifest
                .capabilities
                .tools
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            state,
        }
    }
}

#[async_trait]
impl ToolRegistration for ScopedToolRegistration {
    async fn register(&self, tool: Box<dyn Tool>) -> Result<(), RegistrationError> {
        let name = tool.descriptor().name.clone();
        if !self.declared.contains(&name) {
            return Err(RegistrationError::UndeclaredTool { name });
        }
        self.state.tools.lock().insert(name);
        Ok(())
    }

    fn pending_declared(&self) -> Vec<String> {
        pending(&self.declared, &self.state.tools)
    }
}

pub(crate) struct ScopedHookRegistration {
    declared: BTreeSet<String>,
    state: Arc<CapabilityRegistrationState>,
}

impl ScopedHookRegistration {
    pub(crate) fn new(manifest: &PluginManifest, state: Arc<CapabilityRegistrationState>) -> Self {
        Self {
            declared: manifest
                .capabilities
                .hooks
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            state,
        }
    }
}

#[async_trait]
impl HookRegistration for ScopedHookRegistration {
    async fn register(&self, handler: Box<dyn HookHandler>) -> Result<(), RegistrationError> {
        let name = handler.handler_id().to_owned();
        if !self.declared.contains(&name) {
            return Err(RegistrationError::UndeclaredHook { name });
        }
        self.state.hooks.lock().insert(name);
        Ok(())
    }

    fn pending_declared(&self) -> Vec<String> {
        pending(&self.declared, &self.state.hooks)
    }
}

pub(crate) struct ScopedMcpRegistration {
    declared: BTreeSet<String>,
    state: Arc<CapabilityRegistrationState>,
}

impl ScopedMcpRegistration {
    pub(crate) fn new(manifest: &PluginManifest, state: Arc<CapabilityRegistrationState>) -> Self {
        Self {
            declared: manifest
                .capabilities
                .mcp_servers
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            state,
        }
    }
}

#[async_trait]
impl McpRegistration for ScopedMcpRegistration {
    async fn register(
        &self,
        server: McpServerSpec,
    ) -> Result<harness_contracts::McpServerId, RegistrationError> {
        let name = server.server_id.0.clone();
        if !self.declared.contains(&name) {
            return Err(RegistrationError::UndeclaredMcp { name });
        }
        self.state.mcp.lock().insert(server.server_id.0.clone());
        Ok(server.server_id)
    }

    fn pending_declared(&self) -> Vec<String> {
        pending(&self.declared, &self.state.mcp)
    }
}

pub(crate) struct ScopedSkillRegistration {
    declared: BTreeSet<String>,
    state: Arc<CapabilityRegistrationState>,
}

impl ScopedSkillRegistration {
    pub(crate) fn new(manifest: &PluginManifest, state: Arc<CapabilityRegistrationState>) -> Self {
        Self {
            declared: manifest
                .capabilities
                .skills
                .iter()
                .map(|entry| entry.name.clone())
                .collect(),
            state,
        }
    }
}

#[async_trait]
impl SkillRegistration for ScopedSkillRegistration {
    async fn register(&self, skill: Skill) -> Result<(), RegistrationError> {
        let name = skill.name.clone();
        if !self.declared.contains(&name) {
            return Err(RegistrationError::UndeclaredSkill { name });
        }
        self.state.skills.lock().insert(name);
        Ok(())
    }

    fn pending_declared(&self) -> Vec<String> {
        pending(&self.declared, &self.state.skills)
    }
}

pub(crate) struct ScopedMemoryProviderRegistration {
    state: Arc<CapabilityRegistrationState>,
}

impl ScopedMemoryProviderRegistration {
    pub(crate) fn new(state: Arc<CapabilityRegistrationState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl MemoryProviderRegistration for ScopedMemoryProviderRegistration {
    async fn register(
        &self,
        _provider: Arc<dyn harness_memory::MemoryProvider>,
    ) -> Result<(), RegistrationError> {
        let mut registered = self.state.memory_registered.lock();
        if *registered {
            return Err(RegistrationError::DuplicateSlot {
                slot: CapabilitySlot::MemoryProvider,
            });
        }
        *registered = true;
        Ok(())
    }
}

pub struct ScopedCoordinatorStrategyRegistration {
    state: Arc<CapabilityRegistrationState>,
}

impl ScopedCoordinatorStrategyRegistration {
    pub(crate) fn new(state: Arc<CapabilityRegistrationState>) -> Self {
        Self { state }
    }
}

#[async_trait]
impl CoordinatorStrategyRegistration for ScopedCoordinatorStrategyRegistration {
    async fn register(
        &self,
        _strategy: Arc<dyn CoordinatorStrategy>,
    ) -> Result<(), RegistrationError> {
        let mut registered = self.state.coordinator_registered.lock();
        if *registered {
            return Err(RegistrationError::DuplicateSlot {
                slot: CapabilitySlot::CoordinatorStrategy,
            });
        }
        *registered = true;
        Ok(())
    }
}

fn pending(declared: &BTreeSet<String>, registered: &Mutex<BTreeSet<String>>) -> Vec<String> {
    let registered = registered.lock();
    declared.difference(&registered).cloned().collect()
}
