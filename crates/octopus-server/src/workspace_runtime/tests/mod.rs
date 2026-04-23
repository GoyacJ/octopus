use super::tasks_support::{
    build_task_run_record, task_prompt_from_record, update_task_record_from_run,
};
use super::*;
use std::{fs, path::Path};

use axum::{
    body::{to_bytes, Body},
    http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode},
};
use octopus_core::{
    CreateProjectDeletionRequestInput, CreateProjectRequest, CreateRuntimeSessionInput,
    CreateTaskInterventionRequest, CreateTaskRequest, DataPolicyUpsertRequest,
    LaunchTaskRequest, ProjectDeletionRequest, ProjectPermissionOverrides,
    RegisterBootstrapAdminRequest, RerunTaskRequest, ReviewProjectDeletionRequestInput,
    RoleBindingUpsertRequest, RoleUpsertRequest, SubmitRuntimeTurnInput, TaskContextBundle,
    TaskContextRef, UpdateWorkspaceRequest, WorkspaceSummary, DEFAULT_PROJECT_ID,
    DEFAULT_WORKSPACE_ID,
};
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use tower::ServiceExt;

use crate::test_runtime_sdk::test_server_state;

mod inbox;
mod project_deletion;
mod project_scope;
mod runtime_generation;
mod support;
mod support_runtime;
mod support_workspace;
mod task_mutations;
mod task_routes;
mod task_runtime_approval;
mod transport;
mod validation;
mod workspace;
