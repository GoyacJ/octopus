mod ask_user_question;
mod fs_edit;
mod fs_glob;
mod fs_grep;
mod fs_read;
mod fs_write;
mod shell_bash;
mod sleep;
mod todo_write;
mod w5_stubs;
mod web_fetch;
mod web_search;

use std::sync::Arc;

use crate::{RegistryError, Tool, ToolRegistry};

pub use ask_user_question::AskUserQuestionTool;
pub use fs_edit::FileEditTool;
pub use fs_glob::GlobTool;
pub use fs_grep::GrepTool;
pub use fs_read::FileReadTool;
pub use fs_write::FileWriteTool;
pub use shell_bash::BashTool;
pub use sleep::SleepTool;
pub use todo_write::TodoWriteTool;
pub use w5_stubs::{AgentTool, SkillTool, TaskGetTool, TaskListTool};
pub use web_fetch::WebFetchTool;
pub use web_search::WebSearchTool;

pub fn register_builtins(registry: &mut ToolRegistry) -> Result<(), RegistryError> {
    for tool in builtin_tools() {
        registry.register(tool)?;
    }
    Ok(())
}

fn builtin_tools() -> Vec<Arc<dyn Tool>> {
    vec![
        Arc::new(FileReadTool::new()),
        Arc::new(FileWriteTool::new()),
        Arc::new(FileEditTool::new()),
        Arc::new(GlobTool::new()),
        Arc::new(GrepTool::new()),
        Arc::new(BashTool::new()),
        Arc::new(WebSearchTool::new()),
        Arc::new(WebFetchTool::new()),
        Arc::new(AskUserQuestionTool::new()),
        Arc::new(TodoWriteTool::new()),
        Arc::new(SleepTool::new()),
        Arc::new(AgentTool::new()),
        Arc::new(SkillTool::new()),
        Arc::new(TaskListTool::new()),
        Arc::new(TaskGetTool::new()),
    ]
}
