//! PostgreSQL adapter placeholders.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresAdapterConfig {
    pub connection_string: String,
}
