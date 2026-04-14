mod actor_context;
mod actor_manifest;
mod adapter_state;
#[cfg(test)]
mod adapter_tests;
mod agent_runtime_core;
mod approval_broker;
mod approval_flow;
mod auth_mediation;
mod background_runtime;
mod capability_executor_bridge;
mod capability_planner_bridge;
mod capability_state;
mod config_service;
mod event_bus;
mod execution_events;
mod execution_service;
mod execution_target;
mod executor;
mod handoff_runtime;
mod mailbox_runtime;
mod memory_runtime;
mod memory_selector;
mod memory_writer;
mod model_usage;
mod persistence;
mod policy_compiler;
mod registry;
mod run_context;
mod runtime_config;
mod session_policy;
mod session_service;
mod snapshot_store;
mod subrun_orchestrator;
mod team_runtime;
mod trace_context;
mod turn_submit;
mod worker_runtime;
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
    ProjectWorkspaceAssignments, ResolveRuntimeApprovalInput, ResolveRuntimeAuthChallengeInput,
    ResolveRuntimeMemoryProposalInput, ResolvedExecutionTarget, RuntimeAuthChallengeSummary,
    RuntimeAuthStateSummary, RuntimeBackgroundRunSummary, RuntimeBootstrap,
    RuntimeCapabilityExecutionOutcome, RuntimeCapabilityPlanSummary,
    RuntimeCapabilityPolicyDecisions, RuntimeCapabilityProviderState,
    RuntimeCapabilityStateSnapshot, RuntimeConfigPatch, RuntimeConfigSnapshotSummary,
    RuntimeConfigSource, RuntimeConfigValidationResult, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, RuntimeEventEnvelope,
    RuntimeHandoffSummary, RuntimeMailboxSummary, RuntimeMediationOutcome,
    RuntimeMemoryFreshnessSummary, RuntimeMemoryProposal, RuntimeMemoryProposalReview,
    RuntimeMemorySelectionSummary, RuntimeMemorySummary, RuntimeMessage,
    RuntimePendingMediationSummary, RuntimePolicyDecisionSummary, RuntimeRunCheckpoint,
    RuntimeRunSnapshot, RuntimeSecretReferenceStatus, RuntimeSelectedMemoryItem,
    RuntimeSessionDetail, RuntimeSessionPolicySnapshot, RuntimeSessionSummary,
    RuntimeSubrunSummary, RuntimeTraceContext, RuntimeTraceItem, RuntimeUsageSummary,
    RuntimeWorkerDispatchSummary, RuntimeWorkflowRunDetail, RuntimeWorkflowSummary,
    SubmitRuntimeTurnInput, TeamRecord, TraceEventRecord, RUNTIME_PERMISSION_DANGER_FULL_ACCESS,
    RUNTIME_PERMISSION_READ_ONLY, RUNTIME_PERMISSION_WORKSPACE_WRITE,
};
use octopus_infra::WorkspacePaths;
use octopus_platform::{
    AuthorizationService, ModelRegistryService, ObservationService, RuntimeConfigService,
    RuntimeExecutionService, RuntimeSessionService,
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
pub use executor::{
    LiveRuntimeModelExecutor, MockRuntimeModelExecutor, RuntimeConversationRequest,
    RuntimeModelExecutor,
};
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
        authorization: Arc<dyn AuthorizationService>,
    ) -> Self {
        Self::new_with_executor(
            workspace_id,
            paths,
            observation,
            authorization,
            Arc::new(LiveRuntimeModelExecutor::new()),
        )
    }

    pub fn new_with_executor(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
        authorization: Arc<dyn AuthorizationService>,
        executor: Arc<dyn RuntimeModelExecutor>,
    ) -> Self {
        let config_loader = ConfigLoader::new(&paths.root, paths.runtime_config_dir.clone());
        let adapter = Self {
            state: Arc::new(RuntimeState {
                workspace_id: workspace_id.into(),
                paths,
                observation,
                authorization,
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
