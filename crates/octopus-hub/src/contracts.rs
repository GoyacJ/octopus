use serde::{Deserialize, Serialize};
use thiserror::Error;

const CORE_OBJECTS_JSON: &str = include_str!("../../../contracts/v1/core-objects.json");
const ENUMS_JSON: &str = include_str!("../../../contracts/v1/enums.json");
const EVENTS_JSON: &str = include_str!("../../../contracts/v1/events.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunType {
    Task,
    Discussion,
    Automation,
    Watch,
    Delegation,
    Review,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Planning,
    Running,
    WaitingInput,
    WaitingApproval,
    WaitingDependency,
    Paused,
    Recovering,
    Completed,
    Failed,
    Terminated,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalType {
    Execution,
    KnowledgePromotion,
    ExternalDelegation,
    ExportSharing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    Cron,
    Webhook,
    ManualEvent,
    McpEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxTier {
    LocalTrusted,
    TenantSandboxed,
    EphemeralRestricted,
    ExternalDelegated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeStatus {
    Candidate,
    VerifiedShared,
    PromotedOrg,
    RevokedOrTombstoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustLevel {
    Low,
    Medium,
    High,
    Verified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoreObjectContract {
    pub name: String,
    pub bounded_context: String,
    pub required_fields: Vec<String>,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSkeleton {
    pub name: String,
    pub category: String,
    pub required_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumCatalog {
    pub run_type: Vec<String>,
    pub run_status: Vec<String>,
    pub approval_type: Vec<String>,
    pub trigger_source: Vec<String>,
    pub sandbox_tier: Vec<String>,
    pub knowledge_status: Vec<String>,
    pub trust_level: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractCatalog {
    pub version: String,
    pub core_objects: Vec<CoreObjectContract>,
    pub enums: EnumCatalog,
    pub events: Vec<EventSkeleton>,
}

#[derive(Debug, Deserialize)]
struct CoreObjectFile {
    version: String,
    objects: Vec<CoreObjectContract>,
}

#[derive(Debug, Deserialize)]
struct EnumFile {
    version: String,
    enums: EnumCatalog,
}

#[derive(Debug, Deserialize)]
struct EventFile {
    version: String,
    events: Vec<EventSkeleton>,
}

#[derive(Debug, Error)]
pub enum ContractLoadError {
    #[error("failed to parse contract catalog: {0}")]
    InvalidJson(#[from] serde_json::Error),
}

pub fn run_type_values() -> Vec<String> {
    vec![
        RunType::Task,
        RunType::Discussion,
        RunType::Automation,
        RunType::Watch,
        RunType::Delegation,
        RunType::Review,
    ]
    .into_iter()
    .map(|value| value.as_str().to_string())
    .collect()
}

pub fn run_status_values() -> Vec<String> {
    vec![
        RunStatus::Queued,
        RunStatus::Planning,
        RunStatus::Running,
        RunStatus::WaitingInput,
        RunStatus::WaitingApproval,
        RunStatus::WaitingDependency,
        RunStatus::Paused,
        RunStatus::Recovering,
        RunStatus::Completed,
        RunStatus::Failed,
        RunStatus::Terminated,
        RunStatus::Cancelled,
    ]
    .into_iter()
    .map(|value| value.as_str().to_string())
    .collect()
}

pub fn approval_type_values() -> Vec<String> {
    vec![
        ApprovalType::Execution,
        ApprovalType::KnowledgePromotion,
        ApprovalType::ExternalDelegation,
        ApprovalType::ExportSharing,
    ]
    .into_iter()
    .map(|value| value.as_str().to_string())
    .collect()
}

pub fn trigger_source_values() -> Vec<String> {
    vec![
        TriggerSource::Cron,
        TriggerSource::Webhook,
        TriggerSource::ManualEvent,
        TriggerSource::McpEvent,
    ]
    .into_iter()
    .map(|value| value.as_str().to_string())
    .collect()
}

pub fn sandbox_tier_values() -> Vec<String> {
    vec![
        SandboxTier::LocalTrusted,
        SandboxTier::TenantSandboxed,
        SandboxTier::EphemeralRestricted,
        SandboxTier::ExternalDelegated,
    ]
    .into_iter()
    .map(|value| value.as_str().to_string())
    .collect()
}

pub fn knowledge_status_values() -> Vec<String> {
    vec![
        KnowledgeStatus::Candidate,
        KnowledgeStatus::VerifiedShared,
        KnowledgeStatus::PromotedOrg,
        KnowledgeStatus::RevokedOrTombstoned,
    ]
    .into_iter()
    .map(|value| value.as_str().to_string())
    .collect()
}

pub fn trust_level_values() -> Vec<String> {
    vec![
        TrustLevel::Low,
        TrustLevel::Medium,
        TrustLevel::High,
        TrustLevel::Verified,
    ]
    .into_iter()
    .map(|value| value.as_str().to_string())
    .collect()
}

pub fn contract_catalog() -> Result<ContractCatalog, ContractLoadError> {
    let core_object_file: CoreObjectFile = serde_json::from_str(CORE_OBJECTS_JSON)?;
    let enum_file: EnumFile = serde_json::from_str(ENUMS_JSON)?;
    let event_file: EventFile = serde_json::from_str(EVENTS_JSON)?;
    let version = core_object_file.version.clone();

    debug_assert_eq!(enum_file.version, version);
    debug_assert_eq!(event_file.version, core_object_file.version);

    Ok(ContractCatalog {
        version,
        core_objects: core_object_file.objects,
        enums: enum_file.enums,
        events: event_file.events,
    })
}

impl RunType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Discussion => "discussion",
            Self::Automation => "automation",
            Self::Watch => "watch",
            Self::Delegation => "delegation",
            Self::Review => "review",
        }
    }
}

impl RunStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Planning => "planning",
            Self::Running => "running",
            Self::WaitingInput => "waiting_input",
            Self::WaitingApproval => "waiting_approval",
            Self::WaitingDependency => "waiting_dependency",
            Self::Paused => "paused",
            Self::Recovering => "recovering",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Terminated => "terminated",
            Self::Cancelled => "cancelled",
        }
    }
}

impl ApprovalType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Execution => "execution",
            Self::KnowledgePromotion => "knowledge_promotion",
            Self::ExternalDelegation => "external_delegation",
            Self::ExportSharing => "export_sharing",
        }
    }
}

impl TriggerSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cron => "cron",
            Self::Webhook => "webhook",
            Self::ManualEvent => "manual_event",
            Self::McpEvent => "mcp_event",
        }
    }
}

impl SandboxTier {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalTrusted => "local_trusted",
            Self::TenantSandboxed => "tenant_sandboxed",
            Self::EphemeralRestricted => "ephemeral_restricted",
            Self::ExternalDelegated => "external_delegated",
        }
    }
}

impl KnowledgeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::VerifiedShared => "verified_shared",
            Self::PromotedOrg => "promoted_org",
            Self::RevokedOrTombstoned => "revoked_or_tombstoned",
        }
    }
}

impl TrustLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Verified => "verified",
        }
    }
}
