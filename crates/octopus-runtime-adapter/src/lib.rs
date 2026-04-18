mod actor_context;
mod actor_manifest;
#[cfg(test)]
mod actor_runtime_tests;
mod adapter_state;
#[cfg(test)]
mod adapter_test_support;
mod agent_runtime_core;
mod approval_broker;
mod approval_flow;
#[cfg(test)]
mod approval_runtime_tests;
mod auth_mediation;
mod background_runtime;
mod capability_executor_bridge;
mod capability_planner_bridge;
#[cfg(test)]
mod capability_runtime_tests;
mod capability_state;
mod config_service;
#[cfg(test)]
mod deliverable_runtime_tests;
mod event_bus;
mod execution_events;
mod execution_service;
mod execution_target;
mod handoff_runtime;
mod mailbox_runtime;
#[cfg(test)]
mod mcp_runtime_tests;
mod memory_runtime;
#[cfg(test)]
mod memory_runtime_tests;
mod memory_selector;
mod memory_writer;
mod model_runtime;
mod model_usage;
mod persistence;
mod policy_compiler;
mod registry;
mod run_context;
#[cfg(test)]
mod runtime_compatibility_tests;
mod runtime_config;
#[cfg(test)]
mod runtime_config_tests;
#[cfg(test)]
mod runtime_contract_tests;
#[cfg(test)]
mod runtime_persistence_tests;
mod secret_store;
mod session_policy;
mod session_service;
mod snapshot_store;
mod subrun_orchestrator;
mod team_runtime;
#[cfg(test)]
mod token_usage_tests;
mod trace_context;
mod worker_runtime;
mod workflow_runtime;

use std::{
    collections::{BTreeMap, HashMap, HashSet},
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
    RuntimeConfigSource, RuntimeConfigValidationResult, RuntimeConfiguredModelCredentialInput,
    RuntimeConfiguredModelProbeInput, RuntimeConfiguredModelProbeResult, RuntimeEffectiveConfig,
    RuntimeEventEnvelope, RuntimeHandoffSummary, RuntimeMailboxSummary, RuntimeMediationOutcome,
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
pub use model_runtime::{
    resolve_model_auth_source, resolve_request_policy, CanonicalDefaultSelection,
    CanonicalModelAlias, CanonicalModelPolicy, LiveRuntimeModelDriver, MockRuntimeModelDriver,
    ModelDriverRegistry, ModelExecutionDeliverable, ModelExecutionResult, ProtocolDriver,
    ProtocolDriverCapability, ResolvedModelAuth, ResolvedModelAuthMode,
    RuntimeConversationExecution, RuntimeConversationRequest, RuntimeModelDriver,
};
use registry::EffectiveModelRegistry;
use runtime_config::{RuntimeConfigDocumentRecord, RuntimeConfigScopeKind};
const IN_MEMORY_SECRET_STORE_ENV: &str = "OCTOPUS_TEST_USE_IN_MEMORY_SECRET_STORE";

use secret_store::MemoryRuntimeSecretStore;
use secret_store::RuntimeSecretStore;
use secret_store::SqliteEncryptedRuntimeSecretStore;
use secret_store::UnavailableRuntimeSecretStore;

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
        let secret_store: Arc<dyn RuntimeSecretStore> =
            if std::env::var_os(IN_MEMORY_SECRET_STORE_ENV).is_some() {
                Arc::new(MemoryRuntimeSecretStore::default())
            } else {
                match SqliteEncryptedRuntimeSecretStore::new(&workspace_id, &paths) {
                    Ok(store) => Arc::new(store),
                    Err(error) => Arc::new(UnavailableRuntimeSecretStore::new(error.to_string())),
                }
            };
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
