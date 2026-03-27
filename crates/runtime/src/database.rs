use std::{fs, path::Path};

use octopus_domain_context::SqliteContextStore;
use octopus_governance::SqliteGovernanceStore;
use octopus_interop_mcp::SqliteInteropStore;
use octopus_knowledge::SqliteKnowledgeStore;
use octopus_observe_artifact::SqliteObservationStore;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

use crate::RuntimeError;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone)]
pub struct Slice1Database {
    pool: SqlitePool,
    context_store: SqliteContextStore,
    governance_store: SqliteGovernanceStore,
    interop_store: SqliteInteropStore,
    knowledge_store: SqliteKnowledgeStore,
    observation_store: SqliteObservationStore,
}

impl Slice1Database {
    pub async fn open_at(path: &Path) -> Result<Self, RuntimeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        MIGRATOR.run(&pool).await?;
        let interop_store = SqliteInteropStore::new(pool.clone());
        interop_store.expire_stale_environment_leases().await?;

        Ok(Self {
            context_store: SqliteContextStore::new(pool.clone()),
            governance_store: SqliteGovernanceStore::new(pool.clone()),
            interop_store,
            knowledge_store: SqliteKnowledgeStore::new(pool.clone()),
            observation_store: SqliteObservationStore::new(pool.clone()),
            pool,
        })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn context_store(&self) -> &SqliteContextStore {
        &self.context_store
    }

    pub fn governance_store(&self) -> &SqliteGovernanceStore {
        &self.governance_store
    }

    pub fn interop_store(&self) -> &SqliteInteropStore {
        &self.interop_store
    }

    pub fn knowledge_store(&self) -> &SqliteKnowledgeStore {
        &self.knowledge_store
    }

    pub fn observation_store(&self) -> &SqliteObservationStore {
        &self.observation_store
    }
}
