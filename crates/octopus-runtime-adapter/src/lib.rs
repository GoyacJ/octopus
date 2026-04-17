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
mod secret_store;
mod snapshot_store;
mod subrun_orchestrator;
mod team_runtime;
mod trace_context;
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
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use octopus_core::{
    timestamp_now, AgentRecord, AppError, ApprovalRequestRecord, ArtifactVersionReference,
    AuditRecord, CancelRuntimeSubrunInput, ConfiguredModelRecord, CostLedgerEntry,
    CreateDeliverableVersionInput, CreateRuntimeSessionInput, DeliverableDetail,
    DeliverableVersionContent, DeliverableVersionSummary, ModelCatalogSnapshot,
    ProjectWorkspaceAssignments, PromoteDeliverableInput, ResolveRuntimeApprovalInput,
    ResolveRuntimeAuthChallengeInput, ResolveRuntimeMemoryProposalInput, ResolvedExecutionTarget,
    RuntimeAuthChallengeSummary, RuntimeAuthStateSummary, RuntimeBackgroundRunSummary,
    RuntimeBootstrap, RuntimeCapabilityExecutionOutcome, RuntimeCapabilityPlanSummary,
    RuntimeCapabilityPolicyDecisions, RuntimeCapabilityProviderState,
    RuntimeCapabilityStateSnapshot, RuntimeConfigPatch, RuntimeConfigSnapshotSummary,
    RuntimeConfigSource, RuntimeConfigValidationResult, RuntimeConfiguredModelCredentialRecord,
    RuntimeConfiguredModelCredentialUpsertInput, RuntimeConfiguredModelProbeInput,
    RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig, RuntimeEventEnvelope,
    RuntimeHandoffSummary, RuntimeMailboxSummary, RuntimeMediationOutcome,
    RuntimeMemoryFreshnessSummary, RuntimeMemoryProposal, RuntimeMemoryProposalReview,
    RuntimeMemorySelectionSummary, RuntimeMemorySummary, RuntimeMessage,
    RuntimePendingMediationSummary, RuntimePolicyDecisionSummary, RuntimeRunCheckpoint,
    RuntimeRunSnapshot, RuntimeSecretReferenceStatus, RuntimeSelectedMemoryItem,
    RuntimeSessionDetail, RuntimeSessionPolicySnapshot, RuntimeSessionSummary,
    RuntimeSubrunSummary, RuntimeTraceContext, RuntimeTraceItem, RuntimeUsageSummary,
    RuntimeWorkerDispatchSummary, RuntimeWorkflowBlockingSummary, RuntimeWorkflowRunDetail,
    RuntimeWorkflowStepSummary, RuntimeWorkflowSummary, SubmitRuntimeTurnInput, TeamRecord,
    TraceEventRecord, RUNTIME_PERMISSION_READ_ONLY, RUNTIME_PERMISSION_WORKSPACE_WRITE,
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
    merge_project_assignments, optional_project_id, sync_runtime_session_detail,
    PendingRuntimeDeliverable, RuntimeAggregate, RuntimeAggregateMetadata, RuntimeState,
};
use executor::{ModelExecutionDeliverable, ModelExecutionResult};
pub use executor::{
    LiveRuntimeModelDriver, MockRuntimeModelDriver, RuntimeConversationExecution,
    RuntimeConversationRequest, RuntimeModelDriver,
};
use registry::EffectiveModelRegistry;
use runtime_config::{RuntimeConfigDocumentRecord, RuntimeConfigScopeKind};
#[cfg(not(test))]
use secret_store::KeyringRuntimeSecretStore;
#[cfg(test)]
use secret_store::MemoryRuntimeSecretStore;
use secret_store::RuntimeSecretStore;

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
            Arc::new(LiveRuntimeModelDriver::new()),
        )
    }

    pub fn new_with_executor(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
        authorization: Arc<dyn AuthorizationService>,
        executor: Arc<dyn RuntimeModelDriver>,
    ) -> Self {
        let workspace_id = workspace_id.into();
        #[cfg(test)]
        let secret_store: Arc<dyn RuntimeSecretStore> = Arc::new(MemoryRuntimeSecretStore::default());
        #[cfg(not(test))]
        let secret_store: Arc<dyn RuntimeSecretStore> =
            Arc::new(KeyringRuntimeSecretStore::new(&workspace_id));
        Self::new_with_executor_and_secret_store(
            workspace_id,
            paths,
            observation,
            authorization,
            executor,
            secret_store,
        )
    }

    pub(crate) fn new_with_executor_and_secret_store(
        workspace_id: impl Into<String>,
        paths: WorkspacePaths,
        observation: Arc<dyn ObservationService>,
        authorization: Arc<dyn AuthorizationService>,
        executor: Arc<dyn RuntimeModelDriver>,
        secret_store: Arc<dyn RuntimeSecretStore>,
    ) -> Self {
        let workspace_id = workspace_id.into();
        let config_loader = ConfigLoader::new(&paths.root, paths.runtime_config_dir.clone());
        let adapter = Self {
            state: Arc::new(RuntimeState {
                workspace_id,
                paths,
                observation,
                authorization,
                config_loader,
                executor,
                secret_store,
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
