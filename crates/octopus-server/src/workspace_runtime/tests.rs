use super::*;
use std::{fs, path::Path};

use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
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

#[path = "tests/project_delete_tests_a.rs"]
mod project_delete_tests_a;
#[path = "tests/project_delete_tests_b.rs"]
mod project_delete_tests_b;
#[path = "tests/project_validation_runtime_tests.rs"]
mod project_validation_runtime_tests;
#[path = "tests/support_runtime.rs"]
mod support_runtime;
#[path = "tests/support_workspace.rs"]
mod support_workspace;
#[path = "tests/task_routes_tests_a.rs"]
mod task_routes_tests_a;
#[path = "tests/task_routes_tests_b.rs"]
mod task_routes_tests_b;
#[path = "tests/transport_visibility_tests.rs"]
mod transport_visibility_tests;
#[path = "tests/workspace_routes_tests.rs"]
mod workspace_routes_tests;

use support_runtime::*;
use support_workspace::*;
