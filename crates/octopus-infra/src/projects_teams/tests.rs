use super::*;
use octopus_core::{
    AccessUserUpsertRequest, ApprovalPreference, ArtifactHandoffPolicy, CapabilityPolicy,
    CreateProjectDeletionRequestInput, DefaultModelStrategy, DelegationPolicy, LoginRequest,
    MailboxPolicy, MemoryPolicy, OutputContract, PermissionEnvelope, RegisterBootstrapAdminRequest,
    ReviewProjectDeletionRequestInput, SharedCapabilityPolicy, SharedMemoryPolicy, TeamTopology,
    WorkflowAffordance,
};
use octopus_platform::{AccessControlService, AuthService};

include!("tests_helpers.rs");
include!("tests_workspace_agents.rs");
include!("tests_projects.rs");
include!("tests_resources.rs");
