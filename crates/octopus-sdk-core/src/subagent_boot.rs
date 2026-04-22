use std::sync::Arc;

use octopus_sdk_tools::{builtin::AgentTool, TaskFn, Tool};

pub(crate) fn agent_tool_with_task_fn(task_fn: Option<&Arc<dyn TaskFn>>) -> Arc<dyn Tool> {
    match task_fn {
        Some(task_fn) => Arc::new(AgentTool::new().with_task_fn(Arc::clone(task_fn))),
        None => Arc::new(AgentTool::new()),
    }
}
