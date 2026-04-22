use std::sync::Arc;

use octopus_sdk_contracts::PluginsSnapshot;
use octopus_sdk_plugin::PluginRegistry;
use octopus_sdk_tools::{TaskFn, ToolRegistry};

use crate::{subagent_boot::agent_tool_with_task_fn, RuntimeError};

pub(crate) fn materialize_tool_registry(
    base: &ToolRegistry,
    plugins: &PluginRegistry,
    task_fn: Option<&Arc<dyn TaskFn>>,
) -> Result<ToolRegistry, RuntimeError> {
    let mut merged = ToolRegistry::new();
    let mut inserted_task_tool = false;

    for (name, tool) in base.iter() {
        if name == "task" {
            merged.register(agent_tool_with_task_fn(task_fn))?;
            inserted_task_tool = true;
            continue;
        }

        merged.register(Arc::clone(tool))?;
    }

    for (name, tool) in plugins.tools().iter() {
        if name == "task" {
            if !inserted_task_tool {
                merged.register(agent_tool_with_task_fn(task_fn))?;
                inserted_task_tool = true;
            }
            continue;
        }

        merged.register(Arc::clone(tool))?;
    }

    if !inserted_task_tool && task_fn.is_some() {
        merged.register(agent_tool_with_task_fn(task_fn))?;
    }

    Ok(merged)
}

pub(crate) fn resolve_plugins_snapshot(
    plugins: &PluginRegistry,
    supplied: Option<PluginsSnapshot>,
) -> Result<PluginsSnapshot, RuntimeError> {
    let discovered = plugins.get_snapshot();

    match supplied {
        Some(snapshot) if snapshot == discovered => Ok(snapshot),
        Some(_) => Err(RuntimeError::PluginSnapshotMismatch),
        None => Ok(discovered),
    }
}
