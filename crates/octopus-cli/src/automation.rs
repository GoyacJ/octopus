use std::fmt;

use crate::config::normalize_optional_args;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandManifestEntry {
    pub name: String,
    pub source: CommandSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandSource {
    Builtin,
    InternalOnly,
    FeatureGated,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandRegistry {
    entries: Vec<CommandManifestEntry>,
}

impl CommandRegistry {
    #[must_use]
    pub fn new(entries: Vec<CommandManifestEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn entries(&self) -> &[CommandManifestEntry] {
        &self.entries
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlashCommandSpec {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub summary: &'static str,
    pub argument_hint: Option<&'static str>,
    pub resume_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommand {
    Help,
    Status,
    Sandbox,
    Compact,
    Bughunter {
        scope: Option<String>,
    },
    Commit,
    Pr {
        context: Option<String>,
    },
    Issue {
        context: Option<String>,
    },
    Ultraplan {
        task: Option<String>,
    },
    Teleport {
        target: Option<String>,
    },
    DebugToolCall,
    Model {
        model: Option<String>,
    },
    Permissions {
        mode: Option<String>,
    },
    Clear {
        confirm: bool,
    },
    Cost,
    Resume {
        session_path: Option<String>,
    },
    Config {
        section: Option<String>,
    },
    Mcp {
        action: Option<String>,
        target: Option<String>,
    },
    Memory,
    Init,
    Diff,
    Version,
    Export {
        path: Option<String>,
    },
    Session {
        action: Option<String>,
        target: Option<String>,
    },
    Plugins {
        action: Option<String>,
        target: Option<String>,
    },
    Agents {
        args: Option<String>,
    },
    Skills {
        args: Option<String>,
    },
    Doctor,
    Login,
    Logout,
    Vim,
    Upgrade,
    Stats,
    Share,
    Feedback,
    Files,
    Fast,
    Exit,
    Summary,
    Desktop,
    Brief,
    Advisor,
    Stickers,
    Insights,
    Thinkback,
    ReleaseNotes,
    SecurityReview,
    Keybindings,
    PrivacySettings,
    Plan {
        mode: Option<String>,
    },
    Review {
        scope: Option<String>,
    },
    Tasks {
        args: Option<String>,
    },
    Theme {
        name: Option<String>,
    },
    Voice {
        mode: Option<String>,
    },
    Usage {
        scope: Option<String>,
    },
    Rename {
        name: Option<String>,
    },
    Copy {
        target: Option<String>,
    },
    Hooks {
        args: Option<String>,
    },
    Context {
        action: Option<String>,
    },
    Color {
        scheme: Option<String>,
    },
    Effort {
        level: Option<String>,
    },
    Branch {
        name: Option<String>,
    },
    Rewind {
        steps: Option<String>,
    },
    Ide {
        target: Option<String>,
    },
    Tag {
        label: Option<String>,
    },
    OutputStyle {
        style: Option<String>,
    },
    AddDir {
        path: Option<String>,
    },
    Unknown(String),
}

impl SlashCommand {
    pub fn parse(input: &str) -> Result<Option<Self>, SlashCommandParseError> {
        validate_slash_command_input(input)
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Help => "help",
            Self::Status => "status",
            Self::Sandbox => "sandbox",
            Self::Compact => "compact",
            Self::Bughunter { .. } => "bughunter",
            Self::Commit => "commit",
            Self::Pr { .. } => "pr",
            Self::Issue { .. } => "issue",
            Self::Ultraplan { .. } => "ultraplan",
            Self::Teleport { .. } => "teleport",
            Self::DebugToolCall => "debug-tool-call",
            Self::Model { .. } => "model",
            Self::Permissions { .. } => "permissions",
            Self::Clear { .. } => "clear",
            Self::Cost => "cost",
            Self::Resume { .. } => "resume",
            Self::Config { .. } => "config",
            Self::Mcp { .. } => "mcp",
            Self::Memory => "memory",
            Self::Init => "init",
            Self::Diff => "diff",
            Self::Version => "version",
            Self::Export { .. } => "export",
            Self::Session { .. } => "session",
            Self::Plugins { .. } => "plugin",
            Self::Agents { .. } => "agents",
            Self::Skills { .. } => "skills",
            Self::Doctor => "doctor",
            Self::Login => "login",
            Self::Logout => "logout",
            Self::Vim => "vim",
            Self::Upgrade => "upgrade",
            Self::Stats => "stats",
            Self::Share => "share",
            Self::Feedback => "feedback",
            Self::Files => "files",
            Self::Fast => "fast",
            Self::Exit => "exit",
            Self::Summary => "summary",
            Self::Desktop => "desktop",
            Self::Brief => "brief",
            Self::Advisor => "advisor",
            Self::Stickers => "stickers",
            Self::Insights => "insights",
            Self::Thinkback => "thinkback",
            Self::ReleaseNotes => "release-notes",
            Self::SecurityReview => "security-review",
            Self::Keybindings => "keybindings",
            Self::PrivacySettings => "privacy-settings",
            Self::Plan { .. } => "plan",
            Self::Review { .. } => "review",
            Self::Tasks { .. } => "tasks",
            Self::Theme { .. } => "theme",
            Self::Voice { .. } => "voice",
            Self::Usage { .. } => "usage",
            Self::Rename { .. } => "rename",
            Self::Copy { .. } => "copy",
            Self::Hooks { .. } => "hooks",
            Self::Context { .. } => "context",
            Self::Color { .. } => "color",
            Self::Effort { .. } => "effort",
            Self::Branch { .. } => "branch",
            Self::Rewind { .. } => "rewind",
            Self::Ide { .. } => "ide",
            Self::Tag { .. } => "tag",
            Self::OutputStyle { .. } => "output-style",
            Self::AddDir { .. } => "add-dir",
            Self::Unknown(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlashCommandParseError {
    message: String,
}

impl SlashCommandParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SlashCommandParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SlashCommandParseError {}

pub const SLASH_COMMAND_SPECS: &[SlashCommandSpec] = &[
    SlashCommandSpec {
        name: "help",
        aliases: &[],
        summary: "Show available slash commands",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "status",
        aliases: &[],
        summary: "Show current session status",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "sandbox",
        aliases: &[],
        summary: "Show sandbox isolation status",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "compact",
        aliases: &[],
        summary: "Compact local session history",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "model",
        aliases: &[],
        summary: "Show or switch the active model",
        argument_hint: Some("[model]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "permissions",
        aliases: &[],
        summary: "Show or switch the active permission mode",
        argument_hint: Some("[read-only|workspace-write|danger-full-access]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "clear",
        aliases: &[],
        summary: "Start a fresh local session",
        argument_hint: Some("[--confirm]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "cost",
        aliases: &[],
        summary: "Show cumulative token usage for this session",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "resume",
        aliases: &[],
        summary: "Load a saved session into the REPL",
        argument_hint: Some("<session-path>"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "config",
        aliases: &[],
        summary: "Inspect Claude config files or merged sections",
        argument_hint: Some("[env|hooks|model|plugins]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "mcp",
        aliases: &[],
        summary: "Inspect configured MCP servers",
        argument_hint: Some("[list|show <server>|help]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "memory",
        aliases: &[],
        summary: "Inspect loaded Claude instruction memory files",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "init",
        aliases: &[],
        summary: "Create a starter CLAUDE.md for this repo",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "diff",
        aliases: &[],
        summary: "Show git diff for current workspace changes",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "version",
        aliases: &[],
        summary: "Show CLI version and build information",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "bughunter",
        aliases: &[],
        summary: "Inspect the codebase for likely bugs",
        argument_hint: Some("[scope]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "commit",
        aliases: &[],
        summary: "Generate a commit message and create a git commit",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "pr",
        aliases: &[],
        summary: "Draft or create a pull request from the conversation",
        argument_hint: Some("[context]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "issue",
        aliases: &[],
        summary: "Draft or create a GitHub issue from the conversation",
        argument_hint: Some("[context]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "ultraplan",
        aliases: &[],
        summary: "Run a deep planning prompt with multi-step reasoning",
        argument_hint: Some("[task]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "teleport",
        aliases: &[],
        summary: "Jump to a file or symbol by searching the workspace",
        argument_hint: Some("<symbol-or-path>"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "debug-tool-call",
        aliases: &[],
        summary: "Replay the last tool call with debug details",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "export",
        aliases: &[],
        summary: "Export the current conversation to a file",
        argument_hint: Some("[file]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "session",
        aliases: &[],
        summary: "List, switch, or fork managed local sessions",
        argument_hint: Some("[list|switch <session-id>|fork [branch-name]]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "plugin",
        aliases: &["plugins", "marketplace"],
        summary: "Manage Claw Code plugins",
        argument_hint: Some(
            "[list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]",
        ),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "agents",
        aliases: &[],
        summary: "List configured agents",
        argument_hint: Some("[list|help]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "skills",
        aliases: &[],
        summary: "List or install available skills",
        argument_hint: Some("[list|install <path>|help]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "doctor",
        aliases: &[],
        summary: "Diagnose setup issues and environment health",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "login",
        aliases: &[],
        summary: "Log in to the service",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "logout",
        aliases: &[],
        summary: "Log out of the current session",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "plan",
        aliases: &[],
        summary: "Toggle or inspect planning mode",
        argument_hint: Some("[on|off]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "review",
        aliases: &[],
        summary: "Run a code review on current changes",
        argument_hint: Some("[scope]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "tasks",
        aliases: &[],
        summary: "List and manage background tasks",
        argument_hint: Some("[list|get <id>|stop <id>]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "theme",
        aliases: &[],
        summary: "Switch the terminal color theme",
        argument_hint: Some("[theme-name]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "vim",
        aliases: &[],
        summary: "Toggle vim keybinding mode",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "voice",
        aliases: &[],
        summary: "Toggle voice input mode",
        argument_hint: Some("[on|off]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "upgrade",
        aliases: &[],
        summary: "Check for and install CLI updates",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "usage",
        aliases: &[],
        summary: "Show detailed API usage statistics",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "stats",
        aliases: &[],
        summary: "Show workspace and session statistics",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "rename",
        aliases: &[],
        summary: "Rename the current session",
        argument_hint: Some("<name>"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "copy",
        aliases: &[],
        summary: "Copy conversation or output to clipboard",
        argument_hint: Some("[last|all]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "share",
        aliases: &[],
        summary: "Share the current conversation",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "feedback",
        aliases: &[],
        summary: "Submit feedback about the current session",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "hooks",
        aliases: &[],
        summary: "List and manage lifecycle hooks",
        argument_hint: Some("[list|run <hook>]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "files",
        aliases: &[],
        summary: "List files in the current context window",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "context",
        aliases: &[],
        summary: "Inspect or manage the conversation context",
        argument_hint: Some("[show|clear]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "color",
        aliases: &[],
        summary: "Configure terminal color settings",
        argument_hint: Some("[scheme]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "effort",
        aliases: &[],
        summary: "Set the effort level for responses",
        argument_hint: Some("[low|medium|high]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "fast",
        aliases: &[],
        summary: "Toggle fast/concise response mode",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "exit",
        aliases: &[],
        summary: "Exit the REPL session",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "branch",
        aliases: &[],
        summary: "Create or switch git branches",
        argument_hint: Some("[name]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "rewind",
        aliases: &[],
        summary: "Rewind the conversation to a previous state",
        argument_hint: Some("[steps]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "summary",
        aliases: &[],
        summary: "Generate a summary of the conversation",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "desktop",
        aliases: &[],
        summary: "Open or manage the desktop app integration",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "ide",
        aliases: &[],
        summary: "Open or configure IDE integration",
        argument_hint: Some("[vscode|cursor]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "tag",
        aliases: &[],
        summary: "Tag the current conversation point",
        argument_hint: Some("[label]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "brief",
        aliases: &[],
        summary: "Toggle brief output mode",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "advisor",
        aliases: &[],
        summary: "Toggle advisor mode for guidance-only responses",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "stickers",
        aliases: &[],
        summary: "Browse and manage sticker packs",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "insights",
        aliases: &[],
        summary: "Show AI-generated insights about the session",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "thinkback",
        aliases: &[],
        summary: "Replay the thinking process of the last response",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "release-notes",
        aliases: &[],
        summary: "Generate release notes from recent changes",
        argument_hint: None,
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "security-review",
        aliases: &[],
        summary: "Run a security review on the codebase",
        argument_hint: Some("[scope]"),
        resume_supported: false,
    },
    SlashCommandSpec {
        name: "keybindings",
        aliases: &[],
        summary: "Show or configure keyboard shortcuts",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "privacy-settings",
        aliases: &[],
        summary: "View or modify privacy settings",
        argument_hint: None,
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "output-style",
        aliases: &[],
        summary: "Switch output formatting style",
        argument_hint: Some("[style]"),
        resume_supported: true,
    },
    SlashCommandSpec {
        name: "add-dir",
        aliases: &[],
        summary: "Add an additional directory to the context",
        argument_hint: Some("<path>"),
        resume_supported: false,
    },
];

#[allow(clippy::too_many_lines)]
pub fn validate_slash_command_input(
    input: &str,
) -> Result<Option<SlashCommand>, SlashCommandParseError> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return Ok(None);
    }

    let mut parts = trimmed.trim_start_matches('/').split_whitespace();
    let command = parts.next().unwrap_or_default();
    if command.is_empty() {
        return Err(SlashCommandParseError::new(
            "Slash command name is missing. Use /help to list available slash commands.",
        ));
    }

    let args = parts.collect::<Vec<_>>();
    let remainder = remainder_after_command(trimmed, command);

    Ok(Some(match command {
        "help" => {
            validate_no_args(command, &args)?;
            SlashCommand::Help
        }
        "status" => {
            validate_no_args(command, &args)?;
            SlashCommand::Status
        }
        "sandbox" => {
            validate_no_args(command, &args)?;
            SlashCommand::Sandbox
        }
        "compact" => {
            validate_no_args(command, &args)?;
            SlashCommand::Compact
        }
        "bughunter" => SlashCommand::Bughunter { scope: remainder },
        "commit" => {
            validate_no_args(command, &args)?;
            SlashCommand::Commit
        }
        "pr" => SlashCommand::Pr { context: remainder },
        "issue" => SlashCommand::Issue { context: remainder },
        "ultraplan" => SlashCommand::Ultraplan { task: remainder },
        "teleport" => SlashCommand::Teleport {
            target: Some(require_remainder(command, remainder, "<symbol-or-path>")?),
        },
        "debug-tool-call" => {
            validate_no_args(command, &args)?;
            SlashCommand::DebugToolCall
        }
        "model" => SlashCommand::Model {
            model: optional_single_arg(command, &args, "[model]")?,
        },
        "permissions" => SlashCommand::Permissions {
            mode: parse_permissions_mode(&args)?,
        },
        "clear" => SlashCommand::Clear {
            confirm: parse_clear_args(&args)?,
        },
        "cost" => {
            validate_no_args(command, &args)?;
            SlashCommand::Cost
        }
        "resume" => SlashCommand::Resume {
            session_path: Some(require_remainder(command, remainder, "<session-path>")?),
        },
        "config" => SlashCommand::Config {
            section: parse_config_section(&args)?,
        },
        "mcp" => parse_mcp_command(&args)?,
        "memory" => {
            validate_no_args(command, &args)?;
            SlashCommand::Memory
        }
        "init" => {
            validate_no_args(command, &args)?;
            SlashCommand::Init
        }
        "diff" => {
            validate_no_args(command, &args)?;
            SlashCommand::Diff
        }
        "version" => {
            validate_no_args(command, &args)?;
            SlashCommand::Version
        }
        "export" => SlashCommand::Export { path: remainder },
        "session" => parse_session_command(&args)?,
        "plugin" | "plugins" | "marketplace" => parse_plugin_command(&args)?,
        "agents" => SlashCommand::Agents {
            args: parse_list_or_help_args(command, remainder)?,
        },
        "skills" => SlashCommand::Skills {
            args: parse_skills_args(remainder.as_deref())?,
        },
        "doctor" => {
            validate_no_args(command, &args)?;
            SlashCommand::Doctor
        }
        "login" => {
            validate_no_args(command, &args)?;
            SlashCommand::Login
        }
        "logout" => {
            validate_no_args(command, &args)?;
            SlashCommand::Logout
        }
        "vim" => {
            validate_no_args(command, &args)?;
            SlashCommand::Vim
        }
        "upgrade" => {
            validate_no_args(command, &args)?;
            SlashCommand::Upgrade
        }
        "stats" => {
            validate_no_args(command, &args)?;
            SlashCommand::Stats
        }
        "share" => {
            validate_no_args(command, &args)?;
            SlashCommand::Share
        }
        "feedback" => {
            validate_no_args(command, &args)?;
            SlashCommand::Feedback
        }
        "files" => {
            validate_no_args(command, &args)?;
            SlashCommand::Files
        }
        "fast" => {
            validate_no_args(command, &args)?;
            SlashCommand::Fast
        }
        "exit" => {
            validate_no_args(command, &args)?;
            SlashCommand::Exit
        }
        "summary" => {
            validate_no_args(command, &args)?;
            SlashCommand::Summary
        }
        "desktop" => {
            validate_no_args(command, &args)?;
            SlashCommand::Desktop
        }
        "brief" => {
            validate_no_args(command, &args)?;
            SlashCommand::Brief
        }
        "advisor" => {
            validate_no_args(command, &args)?;
            SlashCommand::Advisor
        }
        "stickers" => {
            validate_no_args(command, &args)?;
            SlashCommand::Stickers
        }
        "insights" => {
            validate_no_args(command, &args)?;
            SlashCommand::Insights
        }
        "thinkback" => {
            validate_no_args(command, &args)?;
            SlashCommand::Thinkback
        }
        "release-notes" => {
            validate_no_args(command, &args)?;
            SlashCommand::ReleaseNotes
        }
        "security-review" => {
            validate_no_args(command, &args)?;
            SlashCommand::SecurityReview
        }
        "keybindings" => {
            validate_no_args(command, &args)?;
            SlashCommand::Keybindings
        }
        "privacy-settings" => {
            validate_no_args(command, &args)?;
            SlashCommand::PrivacySettings
        }
        "plan" => SlashCommand::Plan { mode: remainder },
        "review" => SlashCommand::Review { scope: remainder },
        "tasks" => SlashCommand::Tasks { args: remainder },
        "theme" => SlashCommand::Theme { name: remainder },
        "voice" => SlashCommand::Voice { mode: remainder },
        "usage" => SlashCommand::Usage { scope: remainder },
        "rename" => SlashCommand::Rename { name: remainder },
        "copy" => SlashCommand::Copy { target: remainder },
        "hooks" => SlashCommand::Hooks { args: remainder },
        "context" => SlashCommand::Context { action: remainder },
        "color" => SlashCommand::Color { scheme: remainder },
        "effort" => SlashCommand::Effort { level: remainder },
        "branch" => SlashCommand::Branch { name: remainder },
        "rewind" => SlashCommand::Rewind { steps: remainder },
        "ide" => SlashCommand::Ide { target: remainder },
        "tag" => SlashCommand::Tag { label: remainder },
        "output-style" => SlashCommand::OutputStyle { style: remainder },
        "add-dir" => SlashCommand::AddDir { path: remainder },
        other => SlashCommand::Unknown(other.to_string()),
    }))
}

fn validate_no_args(command: &str, args: &[&str]) -> Result<(), SlashCommandParseError> {
    if args.is_empty() {
        return Ok(());
    }
    Err(command_error(
        &format!("Unexpected arguments for /{command}."),
        command,
        &format!("/{command}"),
    ))
}

fn optional_single_arg(
    command: &str,
    args: &[&str],
    argument_hint: &str,
) -> Result<Option<String>, SlashCommandParseError> {
    match args {
        [] => Ok(None),
        [value] => Ok(Some((*value).to_string())),
        _ => Err(usage_error(command, argument_hint)),
    }
}

fn require_remainder(
    command: &str,
    remainder: Option<String>,
    argument_hint: &str,
) -> Result<String, SlashCommandParseError> {
    remainder.ok_or_else(|| usage_error(command, argument_hint))
}

fn parse_permissions_mode(args: &[&str]) -> Result<Option<String>, SlashCommandParseError> {
    let mode = optional_single_arg(
        "permissions",
        args,
        "[read-only|workspace-write|danger-full-access]",
    )?;
    if let Some(mode) = mode {
        if matches!(
            mode.as_str(),
            "read-only" | "workspace-write" | "danger-full-access"
        ) {
            return Ok(Some(mode));
        }
        return Err(command_error(
            &format!("Unsupported /permissions mode '{mode}'. Use read-only, workspace-write, or danger-full-access."),
            "permissions",
            "/permissions [read-only|workspace-write|danger-full-access]",
        ));
    }
    Ok(None)
}

fn parse_clear_args(args: &[&str]) -> Result<bool, SlashCommandParseError> {
    match args {
        [] => Ok(false),
        ["--confirm"] => Ok(true),
        [unexpected] => Err(command_error(
            &format!("Unsupported /clear argument '{unexpected}'. Use /clear or /clear --confirm."),
            "clear",
            "/clear [--confirm]",
        )),
        _ => Err(usage_error("clear", "[--confirm]")),
    }
}

fn parse_config_section(args: &[&str]) -> Result<Option<String>, SlashCommandParseError> {
    let section = optional_single_arg("config", args, "[env|hooks|model|plugins]")?;
    if let Some(section) = section {
        if matches!(section.as_str(), "env" | "hooks" | "model" | "plugins") {
            return Ok(Some(section));
        }
        return Err(command_error(
            &format!("Unsupported /config section '{section}'. Use env, hooks, model, or plugins."),
            "config",
            "/config [env|hooks|model|plugins]",
        ));
    }
    Ok(None)
}

fn parse_session_command(args: &[&str]) -> Result<SlashCommand, SlashCommandParseError> {
    match args {
        [] => Ok(SlashCommand::Session { action: None, target: None }),
        ["list"] => Ok(SlashCommand::Session { action: Some("list".to_string()), target: None }),
        ["list", ..] => Err(usage_error("session", "[list|switch <session-id>|fork [branch-name]]")),
        ["switch"] => Err(usage_error("session switch", "<session-id>")),
        ["switch", target] => Ok(SlashCommand::Session { action: Some("switch".to_string()), target: Some((*target).to_string()) }),
        ["switch", ..] => Err(command_error("Unexpected arguments for /session switch.", "session", "/session switch <session-id>")),
        ["fork"] => Ok(SlashCommand::Session { action: Some("fork".to_string()), target: None }),
        ["fork", target] => Ok(SlashCommand::Session { action: Some("fork".to_string()), target: Some((*target).to_string()) }),
        ["fork", ..] => Err(command_error("Unexpected arguments for /session fork.", "session", "/session fork [branch-name]")),
        [action, ..] => Err(command_error(&format!("Unknown /session action '{action}'. Use list, switch <session-id>, or fork [branch-name]."), "session", "/session [list|switch <session-id>|fork [branch-name]]")),
    }
}

fn parse_mcp_command(args: &[&str]) -> Result<SlashCommand, SlashCommandParseError> {
    match args {
        [] => Ok(SlashCommand::Mcp {
            action: None,
            target: None,
        }),
        ["list"] => Ok(SlashCommand::Mcp {
            action: Some("list".to_string()),
            target: None,
        }),
        ["list", ..] => Err(usage_error("mcp list", "")),
        ["show"] => Err(usage_error("mcp show", "<server>")),
        ["show", target] => Ok(SlashCommand::Mcp {
            action: Some("show".to_string()),
            target: Some((*target).to_string()),
        }),
        ["show", ..] => Err(command_error(
            "Unexpected arguments for /mcp show.",
            "mcp",
            "/mcp show <server>",
        )),
        ["help" | "-h" | "--help"] => Ok(SlashCommand::Mcp {
            action: Some("help".to_string()),
            target: None,
        }),
        [action, ..] => Err(command_error(
            &format!("Unknown /mcp action '{action}'. Use list, show <server>, or help."),
            "mcp",
            "/mcp [list|show <server>|help]",
        )),
    }
}

fn parse_plugin_command(args: &[&str]) -> Result<SlashCommand, SlashCommandParseError> {
    match args {
        [] => Ok(SlashCommand::Plugins { action: None, target: None }),
        ["list"] => Ok(SlashCommand::Plugins { action: Some("list".to_string()), target: None }),
        ["list", ..] => Err(usage_error("plugin list", "")),
        ["install"] => Err(usage_error("plugin install", "<path>")),
        ["install", target @ ..] => Ok(SlashCommand::Plugins { action: Some("install".to_string()), target: Some(target.join(" ")) }),
        ["enable"] => Err(usage_error("plugin enable", "<name>")),
        ["enable", target] => Ok(SlashCommand::Plugins { action: Some("enable".to_string()), target: Some((*target).to_string()) }),
        ["enable", ..] => Err(command_error("Unexpected arguments for /plugin enable.", "plugin", "/plugin enable <name>")),
        ["disable"] => Err(usage_error("plugin disable", "<name>")),
        ["disable", target] => Ok(SlashCommand::Plugins { action: Some("disable".to_string()), target: Some((*target).to_string()) }),
        ["disable", ..] => Err(command_error("Unexpected arguments for /plugin disable.", "plugin", "/plugin disable <name>")),
        ["uninstall"] => Err(usage_error("plugin uninstall", "<id>")),
        ["uninstall", target] => Ok(SlashCommand::Plugins { action: Some("uninstall".to_string()), target: Some((*target).to_string()) }),
        ["uninstall", ..] => Err(command_error("Unexpected arguments for /plugin uninstall.", "plugin", "/plugin uninstall <id>")),
        ["update"] => Err(usage_error("plugin update", "<id>")),
        ["update", target] => Ok(SlashCommand::Plugins { action: Some("update".to_string()), target: Some((*target).to_string()) }),
        ["update", ..] => Err(command_error("Unexpected arguments for /plugin update.", "plugin", "/plugin update <id>")),
        [action, ..] => Err(command_error(&format!("Unknown /plugin action '{action}'. Use list, install <path>, enable <name>, disable <name>, uninstall <id>, or update <id>."), "plugin", "/plugin [list|install <path>|enable <name>|disable <name>|uninstall <id>|update <id>]")),
    }
}

fn parse_list_or_help_args(
    command: &str,
    args: Option<String>,
) -> Result<Option<String>, SlashCommandParseError> {
    match normalize_optional_args(args.as_deref()) {
        None | Some("list" | "help" | "-h" | "--help") => Ok(args),
        Some(unexpected) => Err(command_error(&format!("Unexpected arguments for /{command}: {unexpected}. Use /{command}, /{command} list, or /{command} help."), command, &format!("/{command} [list|help]"))),
    }
}

fn parse_skills_args(args: Option<&str>) -> Result<Option<String>, SlashCommandParseError> {
    let Some(args) = normalize_optional_args(args) else {
        return Ok(None);
    };
    if matches!(args, "list" | "help" | "-h" | "--help") {
        return Ok(Some(args.to_string()));
    }
    if args == "install" {
        return Err(command_error(
            "Usage: /skills install <path>",
            "skills",
            "/skills install <path>",
        ));
    }
    if let Some(target) = args.strip_prefix("install").map(str::trim) {
        if !target.is_empty() {
            return Ok(Some(format!("install {target}")));
        }
    }
    Err(command_error(&format!("Unexpected arguments for /skills: {args}. Use /skills, /skills list, /skills install <path>, or /skills help."), "skills", "/skills [list|install <path>|help]"))
}

fn usage_error(command: &str, argument_hint: &str) -> SlashCommandParseError {
    let usage = format!("/{command} {argument_hint}");
    let usage = usage.trim_end().to_string();
    command_error(
        &format!("Usage: {usage}"),
        command_root_name(command),
        &usage,
    )
}

fn command_error(message: &str, command: &str, usage: &str) -> SlashCommandParseError {
    let detail = render_slash_command_help_detail(command)
        .map(|detail| format!("\n\n{detail}"))
        .unwrap_or_default();
    SlashCommandParseError::new(format!("{message}\n  Usage            {usage}{detail}"))
}

fn remainder_after_command(input: &str, command: &str) -> Option<String> {
    input
        .trim()
        .strip_prefix(&format!("/{command}"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn find_slash_command_spec(name: &str) -> Option<&'static SlashCommandSpec> {
    slash_command_specs().iter().find(|spec| {
        spec.name.eq_ignore_ascii_case(name)
            || spec
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(name))
    })
}

fn command_root_name(command: &str) -> &str {
    command.split_whitespace().next().unwrap_or(command)
}

fn slash_command_usage(spec: &SlashCommandSpec) -> String {
    match spec.argument_hint {
        Some(argument_hint) => format!("/{} {argument_hint}", spec.name),
        None => format!("/{}", spec.name),
    }
}

fn slash_command_detail_lines(spec: &SlashCommandSpec) -> Vec<String> {
    let mut lines = vec![format!("/{}", spec.name)];
    lines.push(format!("  Summary          {}", spec.summary));
    lines.push(format!("  Usage            {}", slash_command_usage(spec)));
    lines.push(format!(
        "  Category         {}",
        slash_command_category(spec.name)
    ));
    if !spec.aliases.is_empty() {
        lines.push(format!(
            "  Aliases          {}",
            spec.aliases
                .iter()
                .map(|alias| format!("/{alias}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if spec.resume_supported {
        lines.push("  Resume           Supported with --resume SESSION.jsonl".to_string());
    }
    lines
}

#[must_use]
pub fn render_slash_command_help_detail(name: &str) -> Option<String> {
    find_slash_command_spec(name).map(|spec| slash_command_detail_lines(spec).join("\n"))
}

#[must_use]
pub fn slash_command_specs() -> &'static [SlashCommandSpec] {
    SLASH_COMMAND_SPECS
}

#[must_use]
pub fn resume_supported_slash_commands() -> Vec<&'static SlashCommandSpec> {
    slash_command_specs()
        .iter()
        .filter(|spec| spec.resume_supported)
        .collect()
}

fn slash_command_category(name: &str) -> &'static str {
    match name {
        "help" | "status" | "sandbox" | "model" | "permissions" | "cost" | "resume" | "session"
        | "version" | "login" | "logout" | "usage" | "stats" | "rename" | "privacy-settings" => {
            "Session & visibility"
        }
        "compact" | "clear" | "config" | "memory" | "init" | "diff" | "commit" | "pr" | "issue"
        | "export" | "plugin" | "branch" | "add-dir" | "files" | "hooks" | "release-notes" => {
            "Workspace & git"
        }
        "agents" | "skills" | "teleport" | "debug-tool-call" | "mcp" | "context" | "tasks"
        | "doctor" | "ide" | "desktop" => "Discovery & debugging",
        "bughunter" | "ultraplan" | "review" | "security-review" | "advisor" | "insights" => {
            "Analysis & automation"
        }
        "theme" | "vim" | "voice" | "color" | "effort" | "fast" | "brief" | "output-style"
        | "keybindings" | "stickers" => "Appearance & input",
        "copy" | "share" | "feedback" | "summary" | "tag" | "thinkback" | "plan" | "exit"
        | "upgrade" | "rewind" => "Communication & control",
        _ => "Other",
    }
}

fn format_slash_command_help_line(spec: &SlashCommandSpec) -> String {
    let name = slash_command_usage(spec);
    let alias_suffix = if spec.aliases.is_empty() {
        String::new()
    } else {
        format!(
            " (aliases: {})",
            spec.aliases
                .iter()
                .map(|alias| format!("/{alias}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let resume = if spec.resume_supported {
        " [resume]"
    } else {
        ""
    };
    format!("  {name:<66} {}{alias_suffix}{resume}", spec.summary)
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    if left == right {
        return 0;
    }
    if left.is_empty() {
        return right.chars().count();
    }
    if right.is_empty() {
        return left.chars().count();
    }

    let right_chars = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left.chars().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution_cost = usize::from(left_char != *right_char);
            current[right_index + 1] = (current[right_index] + 1)
                .min(previous[right_index + 1] + 1)
                .min(previous[right_index] + substitution_cost);
        }
        previous.clone_from(&current);
    }

    previous[right_chars.len()]
}

#[must_use]
pub fn suggest_slash_commands(input: &str, limit: usize) -> Vec<String> {
    let query = input.trim().trim_start_matches('/').to_ascii_lowercase();
    if query.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut suggestions = slash_command_specs()
        .iter()
        .filter_map(|spec| {
            let best = std::iter::once(spec.name)
                .chain(spec.aliases.iter().copied())
                .map(str::to_ascii_lowercase)
                .map(|candidate| {
                    let prefix_rank =
                        if candidate.starts_with(&query) || query.starts_with(&candidate) {
                            0
                        } else if candidate.contains(&query) || query.contains(&candidate) {
                            1
                        } else {
                            2
                        };
                    let distance = levenshtein_distance(&candidate, &query);
                    (prefix_rank, distance)
                })
                .min();

            best.and_then(|(prefix_rank, distance)| {
                if prefix_rank <= 1 || distance <= 2 {
                    Some((prefix_rank, distance, spec.name.len(), spec.name))
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    suggestions.sort_unstable();
    suggestions
        .into_iter()
        .map(|(_, _, _, name)| format!("/{name}"))
        .take(limit)
        .collect()
}

#[must_use]
pub fn render_slash_command_help() -> String {
    let mut lines = vec![
        "Slash commands".to_string(),
        "  Start here        /status, /diff, /agents, /skills, /commit".to_string(),
        "  [resume]          also works with --resume SESSION.jsonl".to_string(),
        String::new(),
    ];

    let categories = [
        "Session & visibility",
        "Workspace & git",
        "Discovery & debugging",
        "Analysis & automation",
    ];

    for category in categories {
        lines.push(category.to_string());
        for spec in slash_command_specs()
            .iter()
            .filter(|spec| slash_command_category(spec.name) == category)
        {
            lines.push(format_slash_command_help_line(spec));
        }
        lines.push(String::new());
    }

    lines
        .into_iter()
        .rev()
        .skip_while(String::is_empty)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        render_slash_command_help, suggest_slash_commands, validate_slash_command_input,
        SlashCommand,
    };

    #[test]
    fn parses_plugins_and_skills_commands() {
        assert_eq!(
            SlashCommand::parse("/plugins install demo"),
            Ok(Some(SlashCommand::Plugins {
                action: Some("install".into()),
                target: Some("demo".into()),
            }))
        );
        assert_eq!(
            SlashCommand::parse("/skills install ./fixtures/help-skill"),
            Ok(Some(SlashCommand::Skills {
                args: Some("install ./fixtures/help-skill".into()),
            }))
        );
    }

    #[test]
    fn renders_help_and_suggestions() {
        let help = render_slash_command_help();
        assert!(help.contains("/agents"));
        assert_eq!(
            suggest_slash_commands("/hep", 2).first(),
            Some(&"/help".to_string())
        );
    }

    #[test]
    fn rejects_invalid_clear_args() {
        let error =
            validate_slash_command_input("/clear nope").expect_err("invalid args should fail");
        assert!(error.to_string().contains("/clear [--confirm]"));
    }
}
