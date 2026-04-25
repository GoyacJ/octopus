use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::*;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UsageSnapshot {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_micros: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EventPayload {
    #[serde(default)]
    pub fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CacheImpact {
    pub prompt_cache_invalidated: bool,
    pub reason: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ForkReason {
    UserRequested,
    Compaction,
    HotReload,
    Isolation,
    RetryFromCheckpoint(JournalOffset),
}

pub type DeltaHash = [u8; 32];
pub type HandlerId = String;
pub type SchemaId = String;
pub type CompactStrategyId = String;
pub type ManifestOriginRef = String;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ContentHash(pub [u8; 32]);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct MemoryActor {
    pub tenant_id: TenantId,
    pub user_id: Option<String>,
    pub team_id: Option<TeamId>,
    pub session_id: Option<SessionId>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct MemoryWriteTarget {
    pub kind: MemoryKind,
    pub visibility: MemoryVisibility,
    pub destination: WriteDestination,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WriteDestination {
    Memdir(MemdirFileTag),
    External { provider_id: String },
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MemdirFileTag {
    Memory,
    User,
    Dreams,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubagentTerminationReason {
    NaturalCompletion,
    ParentCancelled,
    AdminInterrupted { admin_id: String },
    Stalled { silent_for_ms: u64 },
    BridgeBroken,
    Failed { detail: String },
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubagentStatus {
    Completed,
    Cancelled,
    Failed,
    Stalled,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TopologyKind {
    Supervisor,
    Peer,
    Pipeline,
    Custom(String),
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TeamTerminationReason {
    Completed,
    Cancelled,
    Error(String),
    MemberFailed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompactionHandoff {
    pub summary_ref: BlobRef,
    pub metadata: Value,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompactTrigger {
    SoftBudget,
    HardBudget,
    ProviderReported,
    UserCommand,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CompactOutcome {
    Applied,
    Degraded,
    ReactiveFailed,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContextStage {
    Gather,
    Rank,
    Pack,
    Compact,
    Finalize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SandboxPolicySummary {
    pub mode: SandboxMode,
    pub scope: SandboxScope,
    pub network: NetworkAccess,
    pub resource_limits: ResourceLimits,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SandboxExitStatus {
    Code(i32),
    Signal(i32),
    Timeout,
    Cancelled,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookEventKind {
    PreToolUse,
    PostToolUse,
    TransformToolResult,
    PreModel,
    PostModel,
    SessionStart,
    SessionEnd,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HookOutcomeSummary {
    pub continued: bool,
    pub blocked_reason: Option<String>,
    pub rewrote_input: bool,
    pub overrode_permission: Option<Decision>,
    pub added_context_bytes: Option<u64>,
    pub transformed: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookFailureMode {
    FailOpen,
    FailClosed,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookFailureCauseKind {
    Unsupported,
    Inconsistent,
    Panicked,
    Timeout,
    Transport,
    Unauthorized,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HookOutcomeDiscriminant {
    Continue,
    Block,
    RewriteInput,
    OverridePermission,
    AddContext,
    TransformResult,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InconsistentReason {
    PreToolUseBlockExclusive,
    PromptCacheViolation,
    SchemaInvalid {
        schema_id: SchemaId,
        message: String,
    },
    ContextPatchTooLarge {
        limit_bytes: u64,
        actual_bytes: u64,
    },
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct HookPermissionConflictParticipant {
    pub handler_id: HandlerId,
    pub decision: Decision,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PersistenceTamperReason {
    SignatureMismatch,
    AlgorithmDowngrade,
    UnknownKeyId,
    MissingSignature,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuppressionReason {
    JoinedInFlight,
    RecentlyAllowed,
    RecentlyDenied,
    RecentlyTimedOut,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EmbeddedRefusedReason {
    NotWhitelisted,
    SelfReentrant,
    CapabilityDenied,
    PropertyViolation,
    PermissionDenied,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SteeringDropReason {
    Capacity,
    TtlExpired,
    DedupHit,
    RunEnded,
    SessionEnded,
    PluginDenied,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum McpConnectionLostReason {
    Network(String),
    AuthFailure(String),
    HandshakeMismatch(String),
    ServerExited,
    Other(String),
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginRejectedReason {
    TrustPolicy,
    ManifestInvalid,
    CapabilityDenied,
    Duplicate,
    Other(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct SchemaVersionRange {
    pub min: u32,
    pub max: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ManifestValidationFailure {
    SyntaxError {
        details: String,
    },
    SchemaViolation {
        json_pointer: String,
        details: String,
    },
    UnsupportedSchemaVersion {
        found: u32,
        supported: SchemaVersionRange,
    },
    CargoExtensionMetadataMalformed {
        details: String,
    },
    RemoteIntegrityMismatch {
        expected_etag: String,
        got_etag: Option<String>,
    },
}

pub fn now() -> DateTime<Utc> {
    Utc::now()
}
