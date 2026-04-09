mod actor_context;
mod adapter_state;
#[cfg(test)]
mod adapter_tests;
mod approval_flow;
mod config_service;
mod event_bus;
mod execution_events;
mod execution_service;
mod execution_target;
mod executor;
mod model_usage;
mod persistence;
mod registry;
mod runtime_config;
mod session_service;
mod snapshot_store;
mod turn_submit;

#[cfg(test)]
mod split_module_tests;

use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use octopus_core::{
    normalize_runtime_permission_mode_label, timestamp_now, AppError, ApprovalRequestRecord,
    AuditRecord, ConfiguredModelRecord, CostLedgerEntry, CreateRuntimeSessionInput,
    ModelCatalogSnapshot, ProjectWorkspaceAssignments, ResolveRuntimeApprovalInput,
    ResolvedExecutionTarget, RuntimeBootstrap, RuntimeConfigPatch, RuntimeConfigSnapshotSummary,
    RuntimeConfigSource, RuntimeConfigValidationResult, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, RuntimeEventEnvelope,
    RuntimeMessage, RuntimeRunSnapshot, RuntimeSecretReferenceStatus, RuntimeSessionDetail,
    RuntimeSessionSummary, RuntimeTraceItem, SubmitRuntimeTurnInput, TraceEventRecord,
    RUNTIME_PERMISSION_WORKSPACE_WRITE,
};
use octopus_infra::WorkspacePaths;
use octopus_platform::{
    ModelRegistryService, ObservationService, RuntimeConfigService, RuntimeExecutionService,
    RuntimeSessionService,
};
use plugins as _;
use runtime::{apply_config_patch, ConfigDocument, ConfigLoader, ConfigSource, JsonValue};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::broadcast;
use tools as _;
use uuid::Uuid;

use adapter_state::{
    merge_project_assignments, optional_project_id, RuntimeAggregate, RuntimeState,
};
use executor::ExecutionResponse;
pub use executor::{LiveRuntimeModelExecutor, MockRuntimeModelExecutor, RuntimeModelExecutor};
use registry::EffectiveModelRegistry;
use runtime_config::{RuntimeConfigDocumentRecord, RuntimeConfigScopeKind};

#[derive(Clone)]
pub struct RuntimeAdapter {
    state: Arc<RuntimeState>,
}

impl RuntimeAdapter {
    pub fn new(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
    ) -> Self {
        Self::new_with_executor(
            workspace_id,
            paths,
            observation,
            Arc::new(LiveRuntimeModelExecutor::new()),
        )
    }

    pub fn new_with_executor(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
        executor: Arc<dyn RuntimeModelExecutor>,
    ) -> Self {
        let config_loader = ConfigLoader::new(&paths.root, paths.runtime_config_dir.clone());
        let adapter = Self {
            state: Arc::new(RuntimeState {
                workspace_id: workspace_id.into(),
                paths,
                observation,
                config_loader,
                executor,
                sessions: Mutex::new(HashMap::new()),
                config_snapshots: Mutex::new(HashMap::new()),
                order: Mutex::new(Vec::new()),
                broadcasters: Mutex::new(HashMap::new()),
            }),
        };

        if let Err(error) = adapter.load_persisted_config_snapshots() {
            eprintln!("failed to load runtime config snapshots: {error}");
        }
        if let Err(error) = adapter.load_persisted_sessions() {
            eprintln!("failed to load runtime projections: {error}");
        }

        adapter
    }
}
