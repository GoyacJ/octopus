use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetProfile {
    pub id: String,
    pub species: String,
    pub display_name: String,
    pub owner_user_id: String,
    pub avatar_label: String,
    pub summary: String,
    pub greeting: String,
    pub mood: String,
    pub favorite_snack: String,
    pub prompt_hints: Vec<String>,
    pub fallback_asset: String,
    pub rive_asset: Option<String>,
    pub state_machine: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetMessage {
    pub id: String,
    pub pet_id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetPosition {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetPresenceState {
    pub pet_id: String,
    pub is_visible: bool,
    pub chat_open: bool,
    pub motion_state: String,
    pub unread_count: u64,
    pub last_interaction_at: u64,
    pub position: PetPosition,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavePetPresenceInput {
    pub pet_id: String,
    pub is_visible: Option<bool>,
    pub chat_open: Option<bool>,
    pub motion_state: Option<String>,
    pub unread_count: Option<u64>,
    pub last_interaction_at: Option<u64>,
    pub position: Option<PetPosition>,
}

fn default_pet_context_scope() -> String {
    "home".into()
}

fn default_pet_owner_user_id() -> String {
    "user-owner".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetConversationBinding {
    pub pet_id: String,
    pub workspace_id: String,
    #[serde(default = "default_pet_owner_user_id")]
    pub owner_user_id: String,
    #[serde(default = "default_pet_context_scope")]
    pub context_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub conversation_id: String,
    pub session_id: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BindPetConversationInput {
    pub pet_id: String,
    pub conversation_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetWorkspaceSnapshot {
    pub workspace_id: String,
    pub owner_user_id: String,
    pub context_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub profile: PetProfile,
    pub presence: PetPresenceState,
    pub binding: Option<PetConversationBinding>,
    pub messages: Vec<PetMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PetDashboardSummary {
    pub pet_id: String,
    pub workspace_id: String,
    pub owner_user_id: String,
    pub species: String,
    pub mood: String,
    pub active_conversation_count: u64,
    pub knowledge_count: u64,
    pub memory_count: u64,
    pub reminder_count: u64,
    pub resource_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_interaction_at: Option<u64>,
}
