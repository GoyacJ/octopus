use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use octopus_sdk_contracts::{
    ModelProviderDecl, PluginSourceTag, PluginSummary, PluginsSnapshot, SkillDecl, ToolDecl,
};
use octopus_sdk_hooks::HookRunner;
use octopus_sdk_tools::{RegistryError, ToolRegistry};

use crate::{
    AgentDecl, ChannelDecl, CommandDecl, ContextEngineDecl, LspServerDecl, McpServerDecl,
    MemoryBackendDecl, OutputStyleDecl, PluginComponent, PluginError, PluginHookRegistration,
    PluginManifest, PluginToolRegistration, SDK_PLUGIN_API_VERSION,
};

struct RegisteredPlugin {
    version: String,
    git_sha: Option<String>,
    source: PluginSourceTag,
    enabled: bool,
    components: BTreeSet<String>,
}

pub struct PluginRegistry {
    tools: ToolRegistry,
    hooks: Arc<HookRunner>,
    plugins: BTreeMap<String, RegisteredPlugin>,
    tool_decls: BTreeMap<String, ToolDecl>,
    skill_decls: BTreeMap<String, SkillDecl>,
    command_decls: BTreeMap<String, CommandDecl>,
    agent_decls: BTreeMap<String, AgentDecl>,
    output_style_decls: BTreeMap<String, OutputStyleDecl>,
    hook_decls: BTreeMap<String, octopus_sdk_contracts::HookDecl>,
    mcp_server_decls: BTreeMap<String, McpServerDecl>,
    lsp_server_decls: BTreeMap<String, LspServerDecl>,
    model_provider_decls: BTreeMap<String, ModelProviderDecl>,
    channel_decls: BTreeMap<String, ChannelDecl>,
    context_engine_decls: BTreeMap<String, ContextEngineDecl>,
    memory_backend_decls: BTreeMap<String, MemoryBackendDecl>,
    runtime_tools: BTreeSet<String>,
    runtime_hooks: BTreeSet<String>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: ToolRegistry::new(),
            hooks: Arc::new(HookRunner::new()),
            plugins: BTreeMap::new(),
            tool_decls: BTreeMap::new(),
            skill_decls: BTreeMap::new(),
            command_decls: BTreeMap::new(),
            agent_decls: BTreeMap::new(),
            output_style_decls: BTreeMap::new(),
            hook_decls: BTreeMap::new(),
            mcp_server_decls: BTreeMap::new(),
            lsp_server_decls: BTreeMap::new(),
            model_provider_decls: BTreeMap::new(),
            channel_decls: BTreeMap::new(),
            context_engine_decls: BTreeMap::new(),
            memory_backend_decls: BTreeMap::new(),
            runtime_tools: BTreeSet::new(),
            runtime_hooks: BTreeSet::new(),
        }
    }

    pub fn register_plugin(
        &mut self,
        manifest: PluginManifest,
        source: PluginSourceTag,
    ) -> Result<(), PluginError> {
        if self.plugins.contains_key(&manifest.id) {
            return Err(PluginError::DuplicateId {
                id: manifest.id.clone(),
            });
        }

        self.ensure_manifest_component_ids_available(&manifest)?;

        let plugin_id = manifest.id.clone();
        self.plugins.insert(
            plugin_id.clone(),
            RegisteredPlugin {
                version: manifest.version.clone(),
                git_sha: manifest.git_sha.clone(),
                source,
                enabled: true,
                components: BTreeSet::new(),
            },
        );

        for component in &manifest.components {
            self.register_decl_from_manifest(&plugin_id, component)?;
        }

        Ok(())
    }

    #[must_use]
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    #[must_use]
    pub fn hooks(&self) -> &HookRunner {
        self.hooks.as_ref()
    }

    #[must_use]
    pub fn hooks_arc(&self) -> Arc<HookRunner> {
        Arc::clone(&self.hooks)
    }

    #[must_use]
    pub fn has_runtime_tool(&self, id: &str) -> bool {
        self.runtime_tools.contains(&component_key("tool", id))
    }

    #[must_use]
    pub fn has_runtime_hook(&self, id: &str) -> bool {
        self.runtime_hooks.contains(&component_key("hook", id))
    }

    #[must_use]
    pub fn get_snapshot(&self) -> PluginsSnapshot {
        PluginsSnapshot {
            api_version: SDK_PLUGIN_API_VERSION.into(),
            plugins: self
                .plugins
                .iter()
                .map(|(id, plugin)| PluginSummary {
                    id: id.clone(),
                    version: plugin.version.clone(),
                    git_sha: plugin.git_sha.clone(),
                    source: plugin.source,
                    enabled: plugin.enabled,
                    components_count: plugin.components.len() as u16,
                })
                .collect(),
        }
    }

    pub(crate) fn ensure_plugin_exists(&self, plugin_id: &str) -> Result<(), PluginError> {
        if self.plugins.contains_key(plugin_id) {
            Ok(())
        } else {
            Err(PluginError::PluginNotFound {
                plugin_id: plugin_id.into(),
            })
        }
    }

    pub(crate) fn register_tool_runtime(
        &mut self,
        plugin_id: &str,
        reg: PluginToolRegistration,
    ) -> Result<(), PluginError> {
        self.ensure_plugin_exists(plugin_id)?;
        upsert_decl(&mut self.tool_decls, &reg.decl.id, reg.decl.clone())?;

        let runtime_key = component_key("tool", &reg.decl.id);
        if !self.runtime_tools.insert(runtime_key) {
            return Err(PluginError::DuplicateId {
                id: reg.decl.id.clone(),
            });
        }

        self.tools.register(reg.tool).map_err(map_registry_error)?;
        self.track_component(plugin_id, "tool", &reg.decl.id);
        Ok(())
    }

    pub(crate) fn register_hook_runtime(
        &mut self,
        plugin_id: &str,
        reg: PluginHookRegistration,
    ) -> Result<(), PluginError> {
        self.ensure_plugin_exists(plugin_id)?;
        upsert_decl(&mut self.hook_decls, &reg.decl.id, reg.decl.clone())?;

        let runtime_key = component_key("hook", &reg.decl.id);
        if !self.runtime_hooks.insert(runtime_key) {
            return Err(PluginError::DuplicateId {
                id: reg.decl.id.clone(),
            });
        }

        self.hooks
            .register(&reg.decl.id, reg.hook, reg.source, reg.priority);
        self.track_component(plugin_id, "hook", &reg.decl.id);
        Ok(())
    }

    pub(crate) fn register_skill_decl(
        &mut self,
        plugin_id: &str,
        decl: SkillDecl,
    ) -> Result<(), PluginError> {
        self.ensure_plugin_exists(plugin_id)?;
        upsert_decl(&mut self.skill_decls, &decl.id, decl.clone())?;
        self.track_component(plugin_id, "skill", &decl.id);
        Ok(())
    }

    pub(crate) fn register_model_provider_decl(
        &mut self,
        plugin_id: &str,
        decl: ModelProviderDecl,
    ) -> Result<(), PluginError> {
        self.ensure_plugin_exists(plugin_id)?;
        upsert_decl(&mut self.model_provider_decls, &decl.id, decl.clone())?;
        self.track_component(plugin_id, "model_provider", &decl.id);
        Ok(())
    }

    fn ensure_manifest_component_ids_available(
        &self,
        manifest: &PluginManifest,
    ) -> Result<(), PluginError> {
        for component in &manifest.components {
            match component {
                PluginComponent::Tool(decl) => ensure_slot_free(&self.tool_decls, &decl.id)?,
                PluginComponent::Skill(decl) => ensure_slot_free(&self.skill_decls, &decl.id)?,
                PluginComponent::Command(decl) => ensure_slot_free(&self.command_decls, &decl.id)?,
                PluginComponent::Agent(decl) => ensure_slot_free(&self.agent_decls, &decl.id)?,
                PluginComponent::OutputStyle(decl) => {
                    ensure_slot_free(&self.output_style_decls, &decl.id)?
                }
                PluginComponent::Hook(decl) => ensure_slot_free(&self.hook_decls, &decl.id)?,
                PluginComponent::McpServer(decl) => {
                    ensure_slot_free(&self.mcp_server_decls, &decl.id)?
                }
                PluginComponent::LspServer(decl) => {
                    ensure_slot_free(&self.lsp_server_decls, &decl.id)?
                }
                PluginComponent::ModelProvider(decl) => {
                    ensure_slot_free(&self.model_provider_decls, &decl.id)?
                }
                PluginComponent::Channel(decl) => ensure_slot_free(&self.channel_decls, &decl.id)?,
                PluginComponent::ContextEngine(decl) => {
                    ensure_slot_free(&self.context_engine_decls, &decl.id)?
                }
                PluginComponent::MemoryBackend(decl) => {
                    ensure_slot_free(&self.memory_backend_decls, &decl.id)?
                }
            }
        }

        Ok(())
    }

    fn register_decl_from_manifest(
        &mut self,
        plugin_id: &str,
        component: &PluginComponent,
    ) -> Result<(), PluginError> {
        match component {
            PluginComponent::Tool(decl) => {
                upsert_decl(&mut self.tool_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "tool", &decl.id);
            }
            PluginComponent::Skill(decl) => {
                upsert_decl(&mut self.skill_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "skill", &decl.id);
            }
            PluginComponent::Command(decl) => {
                upsert_decl(&mut self.command_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "command", &decl.id);
            }
            PluginComponent::Agent(decl) => {
                upsert_decl(&mut self.agent_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "agent", &decl.id);
            }
            PluginComponent::OutputStyle(decl) => {
                upsert_decl(&mut self.output_style_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "output_style", &decl.id);
            }
            PluginComponent::Hook(decl) => {
                upsert_decl(&mut self.hook_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "hook", &decl.id);
            }
            PluginComponent::McpServer(decl) => {
                upsert_decl(&mut self.mcp_server_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "mcp_server", &decl.id);
            }
            PluginComponent::LspServer(decl) => {
                upsert_decl(&mut self.lsp_server_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "lsp_server", &decl.id);
            }
            PluginComponent::ModelProvider(decl) => {
                upsert_decl(&mut self.model_provider_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "model_provider", &decl.id);
            }
            PluginComponent::Channel(decl) => {
                upsert_decl(&mut self.channel_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "channel", &decl.id);
            }
            PluginComponent::ContextEngine(decl) => {
                upsert_decl(&mut self.context_engine_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "context_engine", &decl.id);
            }
            PluginComponent::MemoryBackend(decl) => {
                upsert_decl(&mut self.memory_backend_decls, &decl.id, decl.clone())?;
                self.track_component(plugin_id, "memory_backend", &decl.id);
            }
        }

        Ok(())
    }

    fn track_component(&mut self, plugin_id: &str, kind: &str, id: &str) {
        let Some(plugin) = self.plugins.get_mut(plugin_id) else {
            return;
        };
        plugin.components.insert(component_key(kind, id));
    }
}

fn ensure_slot_free<T>(map: &BTreeMap<String, T>, id: &str) -> Result<(), PluginError> {
    if map.contains_key(id) {
        Err(PluginError::DuplicateId { id: id.into() })
    } else {
        Ok(())
    }
}

fn upsert_decl<T>(map: &mut BTreeMap<String, T>, id: &str, value: T) -> Result<(), PluginError>
where
    T: PartialEq,
{
    match map.get(id) {
        Some(existing) if existing == &value => Ok(()),
        Some(_) => Err(PluginError::DuplicateId { id: id.into() }),
        None => {
            map.insert(id.into(), value);
            Ok(())
        }
    }
}

fn map_registry_error(error: RegistryError) -> PluginError {
    match error {
        RegistryError::DuplicateName(name) => PluginError::DuplicateId { id: name },
        RegistryError::InvalidSpec(message) => {
            PluginError::ManifestValidationError { cause: message }
        }
    }
}

fn component_key(kind: &str, id: &str) -> String {
    format!("{kind}:{id}")
}
