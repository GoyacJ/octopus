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
        super::parse::validate_slash_command_input(input)
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
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for SlashCommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SlashCommandParseError {}
