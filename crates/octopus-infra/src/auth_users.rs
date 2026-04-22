use super::*;
use octopus_core::{default_agent_asset_role, AuthorizationRequest, DataPolicyRecord};
use std::collections::BTreeSet;

const BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID: &str = "user-owner";

#[path = "auth_users/app_registry_service.rs"]
mod app_registry_service;
#[path = "auth_users/auth_helpers.rs"]
mod auth_helpers;
#[path = "auth_users/auth_service.rs"]
mod auth_service;
#[path = "auth_users/authorization_service.rs"]
mod authorization_service;
#[path = "auth_users/workspace_helpers.rs"]
mod workspace_helpers;

pub(crate) use workspace_helpers::{
    append_json_line, default_client_apps, hash_password, to_user_summary, verify_password,
};

#[cfg(test)]
mod tests;
