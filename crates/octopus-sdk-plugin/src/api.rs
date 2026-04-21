use std::sync::Arc;

use octopus_sdk_contracts::{DeclSource, HookDecl, ModelProviderDecl, SkillDecl, ToolDecl};
use octopus_sdk_hooks::{Hook, HookSource};
use octopus_sdk_tools::Tool;

use crate::{PluginError, PluginRegistry};

pub struct PluginToolRegistration {
    pub decl: ToolDecl,
    pub tool: Arc<dyn Tool>,
}

pub struct PluginHookRegistration {
    pub decl: HookDecl,
    pub hook: Arc<dyn Hook>,
    pub source: HookSource,
    pub priority: i32,
}

pub struct PluginApi<'a> {
    registry: &'a mut PluginRegistry,
    plugin_id: &'a str,
}

impl<'a> PluginApi<'a> {
    pub(crate) fn new(
        registry: &'a mut PluginRegistry,
        plugin_id: &'a str,
    ) -> Result<Self, PluginError> {
        registry.ensure_plugin_exists(plugin_id)?;
        Ok(Self {
            registry,
            plugin_id,
        })
    }

    pub fn register_tool(&mut self, reg: PluginToolRegistration) -> Result<(), PluginError> {
        validate_decl_source(self.plugin_id, &reg.decl.source)?;
        self.registry.register_tool_runtime(self.plugin_id, reg)
    }

    pub fn register_hook(&mut self, reg: PluginHookRegistration) -> Result<(), PluginError> {
        validate_decl_source(self.plugin_id, &reg.decl.source)?;
        validate_hook_source(self.plugin_id, &reg.source)?;
        self.registry.register_hook_runtime(self.plugin_id, reg)
    }

    pub fn register_skill_decl(&mut self, decl: SkillDecl) -> Result<(), PluginError> {
        self.registry.register_skill_decl(self.plugin_id, decl)
    }

    pub fn register_model_provider_decl(
        &mut self,
        decl: ModelProviderDecl,
    ) -> Result<(), PluginError> {
        self.registry
            .register_model_provider_decl(self.plugin_id, decl)
    }
}

fn validate_decl_source(plugin_id: &str, source: &DeclSource) -> Result<(), PluginError> {
    match source {
        DeclSource::Bundled => Ok(()),
        DeclSource::Plugin {
            plugin_id: source_plugin_id,
        } if source_plugin_id == plugin_id => Ok(()),
        DeclSource::Plugin {
            plugin_id: source_plugin_id,
        } => Err(PluginError::ManifestValidationError {
            cause: format!(
                "component source plugin_id `{source_plugin_id}` does not match `{plugin_id}`"
            ),
        }),
    }
}

fn validate_hook_source(plugin_id: &str, source: &HookSource) -> Result<(), PluginError> {
    match source {
        HookSource::Plugin {
            plugin_id: source_plugin_id,
        } if source_plugin_id == plugin_id => Ok(()),
        HookSource::Plugin {
            plugin_id: source_plugin_id,
        } => Err(PluginError::ManifestValidationError {
            cause: format!(
                "hook source plugin_id `{source_plugin_id}` does not match `{plugin_id}`"
            ),
        }),
        _ => Ok(()),
    }
}
