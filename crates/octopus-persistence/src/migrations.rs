use octopus_core::AppError;
use rusqlite::Connection;

pub type MigrationFn = fn(&Connection) -> Result<(), AppError>;

#[derive(Clone, Copy)]
pub struct Migration {
    pub key: &'static str,
    pub apply: MigrationFn,
}
