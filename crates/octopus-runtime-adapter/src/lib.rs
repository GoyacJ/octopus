mod actor_context;
mod actor_manifest;
mod adapter_state;
#[cfg(test)]
mod adapter_tests;
mod agent_runtime_core;
mod approval_flow;
mod background_runtime;
mod capability_planner_bridge;
mod capability_state;
mod config_service;
mod event_bus;
mod execution_events;
mod execution_service;
mod execution_target;
mod executor;
mod model_usage;
mod persistence;
mod registry;
mod run_context;
mod subrun_orchestrator;
mod team_runtime;
mod runtime_config;
mod session_policy;
mod session_service;
mod snapshot_store;
mod trace_context;
mod turn_submit;
mod worker_runtime;
mod mailbox_runtime;
mod handoff_runtime;
mod workflow_runtime;

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
    timestamp_now, AgentRecord, AppError, ApprovalRequestRecord, AuditRecord,
    ConfiguredModelRecord, CostLedgerEntry, CreateRuntimeSessionInput, ModelCatalogSnapshot,
    ProjectWorkspaceAssignments, ResolveRuntimeApprovalInput, ResolvedExecutionTarget,
    RuntimeBackgroundRunSummary, RuntimeBootstrap, RuntimeCapabilityExecutionOutcome,
    RuntimeCapabilityPlanSummary, RuntimeCapabilityProviderState, RuntimeCapabilityStateSnapshot,
    RuntimeConfigPatch, RuntimeConfigSnapshotSummary, RuntimeConfigSource,
    RuntimeConfigValidationResult, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, RuntimeEventEnvelope,
    RuntimeHandoffSummary, RuntimeMailboxSummary, RuntimeMemorySummary, RuntimeMessage,
    RuntimePendingMediationSummary, RuntimeRunCheckpoint, RuntimeRunSnapshot,
    RuntimeSecretReferenceStatus, RuntimeSessionDetail, RuntimeSessionPolicySnapshot,
    RuntimeSessionSummary, RuntimeSubrunSummary, RuntimeTraceContext, RuntimeTraceItem,
    RuntimeUsageSummary, RuntimeWorkerDispatchSummary, RuntimeWorkflowRunDetail,
    RuntimeWorkflowSummary, SubmitRuntimeTurnInput, TeamRecord, TraceEventRecord,
    RUNTIME_PERMISSION_DANGER_FULL_ACCESS, RUNTIME_PERMISSION_READ_ONLY,
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
    merge_project_assignments, optional_project_id, sync_runtime_session_detail, RuntimeAggregate,
    RuntimeAggregateMetadata, RuntimeState,
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
