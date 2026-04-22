use thiserror::Error;

pub const DEFAULT_WORKSPACE_ID: &str = "ws-local";
pub const DEFAULT_PROJECT_ID: &str = "proj-redesign";
pub const RUNTIME_PERMISSION_READ_ONLY: &str = "read-only";
pub const RUNTIME_PERMISSION_WORKSPACE_WRITE: &str = "workspace-write";
pub const RUNTIME_PERMISSION_DANGER_FULL_ACCESS: &str = "danger-full-access";

#[derive(Debug, Error)]
pub enum AppError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("toml deserialize error: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
    #[error("toml serialize error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error: {0}")]
    Database(String),
    #[error("runtime error: {0}")]
    Runtime(String),
}

impl AppError {
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth(message.into())
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict(message.into())
    }

    pub fn database(message: impl Into<String>) -> Self {
        Self::Database(message.into())
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        Self::Runtime(message.into())
    }
}

#[must_use]
pub fn normalize_runtime_permission_mode_label(value: &str) -> Option<&'static str> {
    match value.trim() {
        "readonly" | "read-only" => Some(RUNTIME_PERMISSION_READ_ONLY),
        "auto" | "ask" | "workspace-write" => Some(RUNTIME_PERMISSION_WORKSPACE_WRITE),
        "danger-full-access" => Some(RUNTIME_PERMISSION_DANGER_FULL_ACCESS),
        _ => None,
    }
}

pub trait PreferencesPort: Send + Sync {
    fn load_preferences(&self) -> Result<crate::ShellPreferences, AppError>;
    fn save_preferences(
        &self,
        preferences: &crate::ShellPreferences,
    ) -> Result<crate::ShellPreferences, AppError>;
}
