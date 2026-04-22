use super::{
    build_infra_bundle, initialize_workspace, CopyWorkspaceSkillToManagedInput, WorkspacePaths,
};
use octopus_core::{
    AccessUserUpsertRequest, AvatarUploadPayload, CapabilityAssetDisablePatch, CostLedgerEntry,
    CreateProjectDeletionRequestInput, CreateProjectRequest, DataPolicyUpsertRequest,
    RegisterBootstrapAdminRequest, ReviewProjectDeletionRequestInput, RoleBindingUpsertRequest,
    RoleUpsertRequest, UpdateProjectRequest,
};
use octopus_platform::{
    AccessControlService, AuthService, InboxService, ObservationService, WorkspaceService,
};
use serde_json::Value as JsonValue;

fn read_json_file(path: &std::path::Path) -> JsonValue {
    let raw = std::fs::read_to_string(path).expect("json file");
    serde_json::from_str(&raw).expect("json document")
}

fn avatar_payload() -> AvatarUploadPayload {
    AvatarUploadPayload {
        content_type: "image/png".into(),
        data_base64: "iVBORw0KGgo=".into(),
        file_name: "avatar.png".into(),
        byte_size: 8,
    }
}

include!("tests_core.rs");
include!("tests_assets_inbox.rs");
