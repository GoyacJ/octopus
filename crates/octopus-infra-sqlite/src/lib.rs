//! SQLite adapter placeholders.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqliteAdapterConfig {
    pub connection_string: String,
}
