use serde::{Deserialize, Serialize};

use crate::WorkspaceSummary;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    pub client_app_id: String,
    pub username: String,
    pub password: String,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AvatarUploadPayload {
    pub file_name: String,
    pub content_type: String,
    pub data_base64: String,
    pub byte_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBootstrapAdminRequest {
    pub client_app_id: String,
    pub username: String,
    pub display_name: String,
    pub password: String,
    pub confirm_password: String,
    pub avatar: AvatarUploadPayload,
    pub workspace_id: Option<String>,
    pub mapped_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub session: SessionRecord,
    pub workspace: WorkspaceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegisterBootstrapAdminResponse {
    pub session: SessionRecord,
    pub workspace: WorkspaceSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientAppRecord {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub status: String,
    pub first_party: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_hosts: Vec<String>,
    pub session_policy: String,
    pub default_scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_path: Option<String>,
    pub avatar_content_type: Option<String>,
    pub avatar_byte_size: Option<u64>,
    pub avatar_content_hash: Option<String>,
    pub status: String,
    pub password_state: String,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub id: String,
    pub workspace_id: String,
    pub user_id: String,
    pub client_app_id: String,
    pub token: String,
    pub status: String,
    pub created_at: u64,
    pub expires_at: Option<u64>,
}
