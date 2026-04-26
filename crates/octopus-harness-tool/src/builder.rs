use crate::{RegistrationError, Tool, ToolRegistry};

#[derive(Default)]
pub enum BuiltinToolset {
    #[default]
    Default,
    Empty,
    Custom(Vec<Box<dyn Tool>>),
}

#[derive(Default)]
pub struct ToolRegistryBuilder {
    builtin_toolset: BuiltinToolset,
    tools: Vec<Box<dyn Tool>>,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_builtin_toolset(mut self, builtin_toolset: BuiltinToolset) -> Self {
        self.builtin_toolset = builtin_toolset;
        self
    }

    #[must_use]
    pub fn with_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn build(self) -> Result<ToolRegistry, RegistrationError> {
        let registry = ToolRegistry::empty();

        match self.builtin_toolset {
            BuiltinToolset::Default | BuiltinToolset::Empty => {}
            BuiltinToolset::Custom(tools) => {
                for tool in tools {
                    registry.register(tool)?;
                }
            }
        }

        for tool in self.tools {
            registry.register(tool)?;
        }

        Ok(registry)
    }
}
