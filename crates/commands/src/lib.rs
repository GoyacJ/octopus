mod automation_commands;
mod command_parser;
mod config_commands;
mod project_commands;
mod runtime_commands;
#[cfg(test)]
mod split_module_tests;
mod workspace_commands;

use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use plugins::{PluginError, PluginManager, PluginSummary};
use runtime::{
    compact_session, CompactionConfig, ConfigLoader, ConfigSource, McpOAuthConfig, McpServerConfig,
    ScopedMcpServerConfig, Session,
};
use serde_json::{json, Value};

pub use automation_commands::{CommandManifestEntry, CommandRegistry, CommandSource};
pub use command_parser::{
    handle_slash_command, render_slash_command_help, render_slash_command_help_detail,
    resume_supported_slash_commands, slash_command_specs, suggest_slash_commands,
    validate_slash_command_input, SlashCommand, SlashCommandParseError, SlashCommandResult,
};
pub use project_commands::{
    handle_plugins_slash_command, render_plugins_report, PluginsCommandResult,
};
pub use runtime_commands::{handle_mcp_slash_command, handle_mcp_slash_command_json};
pub use workspace_commands::{
    handle_agents_slash_command, handle_agents_slash_command_json, handle_skills_slash_command,
    handle_skills_slash_command_json,
};

#[allow(unused_imports)]
pub(crate) use automation_commands::*;
#[allow(unused_imports)]
pub(crate) use command_parser::*;
#[allow(unused_imports)]
pub(crate) use config_commands::*;
#[allow(unused_imports)]
pub(crate) use project_commands::*;
#[allow(unused_imports)]
pub(crate) use runtime_commands::*;
#[allow(unused_imports)]
pub(crate) use workspace_commands::*;
