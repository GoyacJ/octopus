use super::*;

const PROJECT_DELETION_REQUEST_INBOX_ITEM_TYPE: &str = "project-deletion-request";
const PROJECT_DELETION_REQUEST_ACTION_LABEL: &str = "Review approval";
use crate::project_tasks::{
    load_project_task_interventions, load_project_task_runs, load_project_task_scheduler_claims,
    load_project_tasks,
};

#[async_trait]
impl WorkspaceService for InfraWorkspaceService {
    async fn system_bootstrap(&self) -> Result<SystemBootstrapStatus, AppError> {
        let workspace = self.state.workspace_snapshot()?;
        let owner_ready = workspace.owner_user_id.is_some();
        Ok(SystemBootstrapStatus {
            workspace: workspace.clone(),
            setup_required: !owner_ready && workspace.bootstrap_status == "setup_required",
            owner_ready,
            registered_apps: self
                .state
                .apps
                .lock()
                .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
                .clone(),
            protocol_version: "2026-04-06".into(),
            api_base_path: "/api/v1".into(),
            transport_security: "loopback".into(),
            auth_mode: "session-token".into(),
            capabilities: octopus_core::WorkspaceCapabilitySet {
                polling: true,
                sse: true,
                idempotency: true,
                reconnect: true,
                event_replay: true,
            },
        })
    }

    async fn workspace_summary(&self) -> Result<WorkspaceSummary, AppError> {
        self.state.workspace_snapshot()
    }

    async fn update_workspace(
        &self,
        request: UpdateWorkspaceRequest,
    ) -> Result<WorkspaceSummary, AppError> {
        let current_workspace = self.state.workspace_snapshot()?;
        let current_workspace_root = self.state.paths.root.clone();
        let shell_root =
            PathBuf::from(workspace_shell_root_display_path(&current_workspace, &self.state.paths));
        let next_name = match request.name {
            Some(value) => {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(AppError::invalid_input("workspace name is required"));
                }
                trimmed.to_string()
            }
            None => current_workspace.name.clone(),
        };

        let next_avatar = if let Some(avatar) = request.avatar.as_ref() {
            let (avatar_path, avatar_content_type, _, _) =
                self.persist_workspace_avatar("workspace", avatar)?;
            (
                stored_avatar_data_url(
                    &self.state.paths,
                    Some(avatar_path.as_str()),
                    Some(avatar_content_type.as_str()),
                ),
                Some(avatar_path),
                Some(avatar_content_type),
            )
        } else if request.remove_avatar.unwrap_or(false) {
            (None, None, None)
        } else {
            (
                current_workspace.avatar.clone(),
                self.state
                    .workspace_avatar_path
                    .lock()
                    .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?
                    .clone(),
                self.state
                    .workspace_avatar_content_type
                    .lock()
                    .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?
                    .clone(),
            )
        };
        let next_mapped_directory =
            normalize_mapped_directory_input(request.mapped_directory.as_deref())?
                .or(current_workspace.mapped_directory.clone());

        {
            let mut workspace = self
                .state
                .workspace
                .lock()
                .map_err(|_| AppError::runtime("workspace mutex poisoned"))?;
            workspace.name = next_name;
            workspace.avatar = next_avatar.0.clone();
            workspace.mapped_directory = next_mapped_directory.clone();
            workspace.mapped_directory_default =
                Some(shell_root.to_string_lossy().to_string());
        }
        {
            let mut avatar_path = self
                .state
                .workspace_avatar_path
                .lock()
                .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?;
            *avatar_path = next_avatar.1.clone();
        }
        {
            let mut avatar_content_type = self
                .state
                .workspace_avatar_content_type
                .lock()
                .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?;
            *avatar_content_type = next_avatar.2.clone();
        }

        self.state.save_workspace_config()?;
        if let Some(next_workspace_root) = next_mapped_directory
            .as_deref()
            .map(PathBuf::from)
            .filter(|path| path != &current_workspace_root)
        {
            let workspace = self.state.workspace_snapshot()?;
            bootstrap::relocate_workspace_root(
                &current_workspace_root,
                &next_workspace_root,
                &shell_root,
                &workspace,
                next_avatar.1.as_deref(),
                next_avatar.2.as_deref(),
            )?;
        }
        Ok(self.state.workspace_snapshot()?)
    }

    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError> {
        Ok(self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .clone())
    }

    async fn list_project_deliverables(
        &self,
        project_id: &str,
    ) -> Result<Vec<ArtifactRecord>, AppError> {
        Ok(load_project_artifact_records(
            &self.state.open_db()?,
            project_id,
        )?)
    }

    async fn create_project(
        &self,
        request: CreateProjectRequest,
    ) -> Result<ProjectRecord, AppError> {
        let resource_directory = self.normalize_resource_directory(&request.resource_directory)?;
        let workspace = self.state.workspace_snapshot()?;
        let owner_user_id = request
            .owner_user_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| workspace.owner_user_id.clone())
            .unwrap_or_else(|| "user-owner".into());
        let assignments = request.assignments;
        let record = ProjectRecord {
            id: format!("proj-{}", Uuid::new_v4()),
            workspace_id: workspace.id,
            name: Self::normalize_project_name(&request.name)?,
            status: "active".into(),
            description: Self::normalize_project_description(&request.description),
            resource_directory,
            leader_agent_id: request.leader_agent_id.clone(),
            manager_user_id: request
                .manager_user_id
                .clone()
                .filter(|value| !value.trim().is_empty()),
            preset_code: request
                .preset_code
                .clone()
                .filter(|value| !value.trim().is_empty()),
            owner_user_id: owner_user_id.clone(),
            member_user_ids: normalized_project_member_user_ids(
                &owner_user_id,
                request.member_user_ids.unwrap_or_default(),
            ),
            permission_overrides: request
                .permission_overrides
                .unwrap_or_else(default_project_permission_overrides),
            linked_workspace_assets: request.linked_workspace_assets.unwrap_or_else(|| {
                Self::linked_workspace_assets_from_assignments(assignments.as_ref())
            }),
            assignments,
        };

        fs::create_dir_all(self.resolve_storage_path(&record.resource_directory))?;
        self.persist_project_record(&record, false)?;

        let mut projects = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?;
        projects.push(record.clone());
        Ok(record)
    }

    async fn update_project(
        &self,
        project_id: &str,
        request: UpdateProjectRequest,
    ) -> Result<ProjectRecord, AppError> {
        let mut projects = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?;
        let existing = projects
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("project not found"))?;

        let updated = ProjectRecord {
            id: project_id.into(),
            workspace_id: existing.workspace_id.clone(),
            name: Self::normalize_project_name(&request.name)?,
            status: Self::normalize_project_status(&request.status)?,
            description: Self::normalize_project_description(&request.description),
            resource_directory: self.normalize_resource_directory(&request.resource_directory)?,
            leader_agent_id: request
                .leader_agent_id
                .clone()
                .or(existing.leader_agent_id.clone()),
            manager_user_id: request
                .manager_user_id
                .clone()
                .or(existing.manager_user_id.clone()),
            preset_code: request.preset_code.clone().or(existing.preset_code.clone()),
            owner_user_id: request
                .owner_user_id
                .clone()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| existing.owner_user_id.clone()),
            member_user_ids: normalized_project_member_user_ids(
                request
                    .owner_user_id
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or(&existing.owner_user_id),
                request
                    .member_user_ids
                    .clone()
                    .unwrap_or_else(|| existing.member_user_ids.clone()),
            ),
            permission_overrides: request
                .permission_overrides
                .clone()
                .unwrap_or_else(|| existing.permission_overrides.clone()),
            linked_workspace_assets: request
                .linked_workspace_assets
                .clone()
                .unwrap_or_else(|| existing.linked_workspace_assets.clone()),
            assignments: request.assignments,
        };

        if existing.status != "archived" && updated.status == "archived" {
            let active_count = projects
                .iter()
                .filter(|project| project.status == "active")
                .count();
            if active_count <= 1 {
                return Err(AppError::invalid_input(
                    "cannot archive the last active project",
                ));
            }
        }

        fs::create_dir_all(self.resolve_storage_path(&updated.resource_directory))?;
        self.persist_project_record(&updated, true)?;

        Self::replace_or_push(&mut projects, updated.clone(), |item| item.id == project_id);

        if existing.status != updated.status && updated.status == "archived" {
            let mut workspace = self
                .state
                .workspace
                .lock()
                .map_err(|_| AppError::runtime("workspace mutex poisoned"))?;
            if workspace.default_project_id == project_id {
                workspace.default_project_id = Self::next_active_project_id(&projects, project_id)
                    .ok_or_else(|| {
                        AppError::invalid_input("cannot archive the last active project")
                    })?;
                drop(workspace);
                self.state.save_workspace_config()?;
            }
        }

        Ok(updated)
    }

    async fn list_project_promotion_requests(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectPromotionRequest>, AppError> {
        Ok(self
            .state
            .project_promotion_requests
            .lock()
            .map_err(|_| AppError::runtime("project promotion requests mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn list_workspace_promotion_requests(
        &self,
    ) -> Result<Vec<ProjectPromotionRequest>, AppError> {
        Ok(self
            .state
            .project_promotion_requests
            .lock()
            .map_err(|_| AppError::runtime("project promotion requests mutex poisoned"))?
            .clone())
    }

    async fn list_project_deletion_requests(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectDeletionRequest>, AppError> {
        Ok(self
            .state
            .project_deletion_requests
            .lock()
            .map_err(|_| AppError::runtime("project deletion requests mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn create_project_promotion_request(
        &self,
        project_id: &str,
        requested_by_user_id: &str,
        input: CreateProjectPromotionRequestInput,
    ) -> Result<ProjectPromotionRequest, AppError> {
        let project = self.project_record(project_id)?;
        let resource = match input.asset_type.as_str() {
            "resource" => Some(self.resource_record(&input.asset_id)?),
            _ => None,
        };
        if let Some(resource) = resource.as_ref() {
            if resource.project_id.as_deref() != Some(project_id) {
                return Err(AppError::invalid_input(
                    "project promotion assets must belong to the project",
                ));
            }
        }

        let now = timestamp_now();
        let record = ProjectPromotionRequest {
            id: format!("promotion-{}", Uuid::new_v4()),
            workspace_id: project.workspace_id.clone(),
            project_id: project.id.clone(),
            asset_type: input.asset_type.clone(),
            asset_id: input.asset_id.clone(),
            requested_by_user_id: requested_by_user_id.into(),
            submitted_by_owner_user_id: project.owner_user_id.clone(),
            required_workspace_capability: Self::required_workspace_capability_for_project_asset(
                &input.asset_type,
            )?,
            status: "pending".into(),
            reviewed_by_user_id: None,
            review_comment: None,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
        };
        self.persist_project_promotion_request(&record, false)?;
        self.state
            .project_promotion_requests
            .lock()
            .map_err(|_| AppError::runtime("project promotion requests mutex poisoned"))?
            .insert(0, record.clone());
        Ok(record)
    }

    async fn create_project_deletion_request(
        &self,
        project_id: &str,
        requested_by_user_id: &str,
        input: CreateProjectDeletionRequestInput,
    ) -> Result<ProjectDeletionRequest, AppError> {
        let project = self.project_record(project_id)?;
        if project.status != "archived" {
            return Err(AppError::conflict(
                "project must be archived before a deletion request can be created",
            ));
        }
        if self
            .state
            .project_deletion_requests
            .lock()
            .map_err(|_| AppError::runtime("project deletion requests mutex poisoned"))?
            .iter()
            .any(|record| record.project_id == project_id && record.status == "pending")
        {
            return Err(AppError::conflict(
                "project deletion request is already pending",
            ));
        }

        let now = timestamp_now();
        let record = ProjectDeletionRequest {
            id: format!("project-delete-{}", Uuid::new_v4()),
            workspace_id: project.workspace_id,
            project_id: project.id,
            requested_by_user_id: requested_by_user_id.into(),
            status: "pending".into(),
            reason: input
                .reason
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            reviewed_by_user_id: None,
            review_comment: None,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
        };
        let inbox_items = self.build_project_deletion_request_inbox_items(&record)?;
        self.persist_project_deletion_request(&record, false)?;
        self.state
            .project_deletion_requests
            .lock()
            .map_err(|_| AppError::runtime("project deletion requests mutex poisoned"))?
            .insert(0, record.clone());
        self.push_inbox_items(inbox_items)?;
        Ok(record)
    }

    async fn review_project_promotion_request(
        &self,
        request_id: &str,
        reviewed_by_user_id: &str,
        input: ReviewProjectPromotionRequestInput,
    ) -> Result<ProjectPromotionRequest, AppError> {
        let existing = self
            .state
            .project_promotion_requests
            .lock()
            .map_err(|_| AppError::runtime("project promotion requests mutex poisoned"))?
            .iter()
            .find(|record| record.id == request_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("project promotion request not found"))?;
        if existing.status != "pending" {
            return Err(AppError::conflict(
                "project promotion request has already been reviewed",
            ));
        }

        if input.approved {
            match existing.asset_type.as_str() {
                "resource" => {
                    self.promote_resource(
                        &existing.asset_id,
                        PromoteWorkspaceResourceInput {
                            scope: "workspace".into(),
                        },
                    )
                    .await?;
                }
                _ => {
                    return Err(AppError::invalid_input(
                        "asset promotion is not implemented for this asset type yet",
                    ));
                }
            }
        }

        let updated = ProjectPromotionRequest {
            status: if input.approved {
                "approved".into()
            } else {
                "rejected".into()
            },
            reviewed_by_user_id: Some(reviewed_by_user_id.into()),
            review_comment: input.review_comment.clone(),
            reviewed_at: Some(timestamp_now()),
            updated_at: timestamp_now(),
            ..existing
        };
        self.persist_project_promotion_request(&updated, true)?;
        let mut requests = self
            .state
            .project_promotion_requests
            .lock()
            .map_err(|_| AppError::runtime("project promotion requests mutex poisoned"))?;
        Self::replace_or_push(&mut requests, updated.clone(), |item| item.id == request_id);
        Ok(updated)
    }

    async fn review_project_deletion_request(
        &self,
        request_id: &str,
        reviewed_by_user_id: &str,
        approved: bool,
        input: ReviewProjectDeletionRequestInput,
    ) -> Result<ProjectDeletionRequest, AppError> {
        let existing = self
            .state
            .project_deletion_requests
            .lock()
            .map_err(|_| AppError::runtime("project deletion requests mutex poisoned"))?
            .iter()
            .find(|record| record.id == request_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("project deletion request not found"))?;
        if existing.status != "pending" {
            return Err(AppError::conflict(
                "project deletion request has already been reviewed",
            ));
        }

        let updated = ProjectDeletionRequest {
            status: if approved {
                "approved".into()
            } else {
                "rejected".into()
            },
            reviewed_by_user_id: Some(reviewed_by_user_id.into()),
            review_comment: input
                .review_comment
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            reviewed_at: Some(timestamp_now()),
            updated_at: timestamp_now(),
            ..existing
        };
        self.persist_project_deletion_request(&updated, true)?;
        let mut requests = self
            .state
            .project_deletion_requests
            .lock()
            .map_err(|_| AppError::runtime("project deletion requests mutex poisoned"))?;
        Self::replace_or_push(&mut requests, updated.clone(), |item| item.id == request_id);
        self.resolve_project_deletion_request_inbox_items(
            request_id,
            reviewed_by_user_id,
            approved,
        )?;
        Ok(updated)
    }

    async fn delete_project(&self, project_id: &str) -> Result<(), AppError> {
        let project = self.project_record(project_id)?;
        if project.status != "archived" {
            return Err(AppError::conflict(
                "project must be archived before deletion",
            ));
        }
        let has_approved_request = self
            .state
            .project_deletion_requests
            .lock()
            .map_err(|_| AppError::runtime("project deletion requests mutex poisoned"))?
            .iter()
            .any(|record| record.project_id == project_id && record.status == "approved");
        if !has_approved_request {
            return Err(AppError::conflict(
                "project deletion requires an approved deletion request",
            ));
        }

        let project_resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect::<Vec<_>>();
        let mut cleanup_paths = std::collections::BTreeSet::new();
        for path in Self::query_project_cleanup_paths(&self.state.open_db()?, project_id)? {
            cleanup_paths.insert(path);
        }

        let mut connection = self.state.open_db()?;
        let tx = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        for sql in [
            "DELETE FROM runtime_memory_proposals WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_approval_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_background_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_workflow_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_handoff_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_mailbox_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_subrun_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_run_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1)",
            "DELETE FROM runtime_session_projections WHERE project_id = ?1",
            "DELETE FROM runtime_memory_records WHERE project_id = ?1",
            "DELETE FROM artifact_versions WHERE project_id = ?1",
            "DELETE FROM artifact_records WHERE project_id = ?1",
            "DELETE FROM project_task_interventions WHERE project_id = ?1",
            "DELETE FROM project_task_runs WHERE project_id = ?1",
            "DELETE FROM project_task_scheduler_claims WHERE project_id = ?1",
            "DELETE FROM project_tasks WHERE project_id = ?1",
            "DELETE FROM resources WHERE project_id = ?1",
            "DELETE FROM knowledge_records WHERE project_id = ?1",
            "DELETE FROM bundle_asset_descriptors WHERE project_id = ?1",
            "DELETE FROM agents WHERE project_id = ?1",
            "DELETE FROM teams WHERE project_id = ?1",
            "DELETE FROM project_agent_links WHERE project_id = ?1",
            "DELETE FROM project_team_links WHERE project_id = ?1",
            "DELETE FROM project_promotion_requests WHERE project_id = ?1",
            "DELETE FROM project_deletion_requests WHERE project_id = ?1",
            "DELETE FROM protected_resources WHERE project_id = ?1",
            "DELETE FROM pet_presence WHERE project_id = ?1",
            "DELETE FROM pet_conversation_bindings WHERE project_id = ?1",
            "DELETE FROM trace_events WHERE project_id = ?1",
            "DELETE FROM audit_records WHERE project_id = ?1",
            "DELETE FROM cost_entries WHERE project_id = ?1",
            "DELETE FROM project_token_usage_projections WHERE project_id = ?1",
            "DELETE FROM projects WHERE id = ?1",
        ] {
            tx.execute(sql, params![project_id])
                .map_err(|error| AppError::database(error.to_string()))?;
        }
        tx.commit()
            .map_err(|error| AppError::database(error.to_string()))?;

        self.refresh_project_scoped_caches(&connection)?;

        let should_save_workspace = {
            let mut workspace = self
                .state
                .workspace
                .lock()
                .map_err(|_| AppError::runtime("workspace mutex poisoned"))?;
            if workspace.default_project_id == project_id {
                let projects = self
                    .state
                    .projects
                    .lock()
                    .map_err(|_| AppError::runtime("projects mutex poisoned"))?;
                workspace.default_project_id = Self::next_active_project_id(&projects, project_id)
                    .ok_or_else(|| AppError::conflict("cannot delete the last active project"))?;
                true
            } else {
                false
            }
        };
        if should_save_workspace {
            self.state.save_workspace_config()?;
        }

        for record in &project_resources {
            self.delete_managed_resource_storage(record)?;
        }
        for path in cleanup_paths {
            self.delete_stored_path_if_exists(&path)?;
        }
        let project_directory = self.resolve_storage_path(&project.resource_directory);
        if project_directory.exists() {
            fs::remove_dir_all(project_directory)?;
        }
        Ok(())
    }

    async fn list_workspace_resources(&self) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        Ok(self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .clone())
    }

    async fn list_project_resources(
        &self,
        project_id: &str,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        Ok(self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect())
    }

    async fn create_workspace_resource(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let record = self.build_metadata_resource_record(workspace_id, owner_user_id, input)?;
        self.persist_resource_record(&record, false)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    async fn create_project_resource(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut input = input;
        input.project_id = Some(project_id.to_string());
        let workspace_id = self
            .state
            .workspace
            .lock()
            .map_err(|_| AppError::runtime("workspace mutex poisoned"))?
            .id
            .clone();
        self.create_workspace_resource(&workspace_id, owner_user_id, input)
            .await
    }

    async fn create_project_resource_folder(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceFolderInput,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        if input.files.is_empty() {
            return Ok(Vec::new());
        }

        let root_dir_name = self.infer_folder_root_name(&input.files);
        let files = self.trim_folder_root_prefix(root_dir_name.as_deref(), input.files)?;
        let record = self
            .import_project_resource(
                project_id,
                owner_user_id,
                WorkspaceResourceImportInput {
                    name: root_dir_name
                        .clone()
                        .or_else(|| files.first().map(|entry| entry.file_name.clone()))
                        .unwrap_or_else(|| "uploaded-folder".into()),
                    root_dir_name,
                    scope: "project".into(),
                    visibility: "public".into(),
                    tags: None,
                    files,
                },
            )
            .await?;
        Ok(vec![record])
    }

    async fn import_workspace_resource(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.ensure_import_has_files(&input.files)?;
        let scope = self.normalize_resource_scope(None, &input.scope)?;
        let visibility = self.normalize_resource_visibility(&input.visibility)?;
        let imported = self.write_imported_resource(
            workspace_id,
            None,
            owner_user_id,
            scope,
            visibility,
            input,
            &self.state.paths.workspace_resources_dir,
        )?;
        self.persist_resource_record(&imported, false)?;
        self.upsert_resource_cache(imported.clone())?;
        Ok(imported)
    }

    async fn import_project_resource(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.ensure_project_exists(project_id)?;
        self.ensure_import_has_files(&input.files)?;
        let project = self.project_record(project_id)?;
        let scope = self.normalize_resource_scope(Some(project_id), &input.scope)?;
        let visibility = self.normalize_resource_visibility(&input.visibility)?;
        let target_directory = self.resolve_storage_path(&project.resource_directory);
        fs::create_dir_all(&target_directory)?;
        let imported = self.write_imported_resource(
            &project.workspace_id,
            Some(project_id),
            owner_user_id,
            scope,
            visibility,
            input,
            &target_directory,
        )?;
        self.persist_resource_record(&imported, false)?;
        self.upsert_resource_cache(imported.clone())?;
        Ok(imported)
    }

    async fn get_resource_detail(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.resource_record(resource_id)
    }

    async fn get_resource_content(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceContentDocument, AppError> {
        let record = self.resource_record(resource_id)?;
        let content_type = record
            .content_type
            .clone()
            .or_else(|| Self::resource_content_type(&record.name, record.location.as_deref()));

        if record.preview_kind == "url" {
            return Ok(WorkspaceResourceContentDocument {
                resource_id: record.id,
                preview_kind: record.preview_kind,
                file_name: Some(record.name),
                content_type,
                external_url: record.location,
                text_content: None,
                data_base64: None,
                byte_size: record.byte_size,
            });
        }

        if record.preview_kind == "folder" {
            return Ok(WorkspaceResourceContentDocument {
                resource_id: record.id,
                preview_kind: record.preview_kind,
                file_name: Some(record.name),
                content_type,
                external_url: None,
                text_content: None,
                data_base64: None,
                byte_size: record.byte_size,
            });
        }

        let Some(storage_path) = record.storage_path.as_deref() else {
            return Ok(WorkspaceResourceContentDocument {
                resource_id: record.id,
                preview_kind: record.preview_kind,
                file_name: Some(record.name),
                content_type,
                external_url: None,
                text_content: None,
                data_base64: None,
                byte_size: record.byte_size,
            });
        };

        let absolute_path = self.resolve_storage_path(storage_path);
        let bytes = fs::read(&absolute_path)?;
        let byte_size = Some(bytes.len() as u64);
        let text_content = match record.preview_kind.as_str() {
            "text" | "code" | "markdown" => Some(String::from_utf8_lossy(&bytes).into_owned()),
            _ => None,
        };
        let data_base64 = match record.preview_kind.as_str() {
            "text" | "code" | "markdown" => None,
            _ => Some(BASE64_STANDARD.encode(bytes)),
        };

        Ok(WorkspaceResourceContentDocument {
            resource_id: record.id,
            preview_kind: record.preview_kind,
            file_name: Some(record.name),
            content_type,
            external_url: None,
            text_content,
            data_base64,
            byte_size,
        })
    }

    async fn list_resource_children(
        &self,
        resource_id: &str,
    ) -> Result<Vec<WorkspaceResourceChildrenRecord>, AppError> {
        let record = self.resource_record(resource_id)?;
        if record.preview_kind != "folder" {
            return Ok(Vec::new());
        }
        let Some(storage_path) = record.storage_path.as_deref() else {
            return Ok(Vec::new());
        };
        let absolute_path = self.resolve_storage_path(storage_path);
        if !absolute_path.exists() || !absolute_path.is_dir() {
            return Ok(Vec::new());
        }
        let mut children = Vec::new();
        Self::collect_resource_children(&absolute_path, &absolute_path, &mut children)?;
        children.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(children)
    }

    async fn promote_resource(
        &self,
        resource_id: &str,
        input: PromoteWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut record = self.resource_record(resource_id)?;
        let next_scope = self.normalize_promoted_scope(&record, &input.scope)?;
        record.scope = next_scope;
        record.updated_at = timestamp_now();
        self.persist_resource_record(&record, true)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    async fn list_directories(
        &self,
        path: Option<&str>,
    ) -> Result<WorkspaceDirectoryBrowserResponse, AppError> {
        let current_path = path
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| self.resolve_storage_path(value))
            .unwrap_or_else(|| self.state.paths.root.clone());
        if !current_path.exists() {
            return Err(AppError::not_found("directory not found"));
        }
        if !current_path.is_dir() {
            return Err(AppError::invalid_input("path is not a directory"));
        }

        let mut entries = fs::read_dir(&current_path)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let file_type = entry.file_type().ok()?;
                if !file_type.is_dir() {
                    return None;
                }
                let path = entry.path();
                Some(WorkspaceDirectoryBrowserEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: self.display_storage_path(&path),
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));

        let parent_path = current_path.parent().and_then(|parent| {
            if parent.starts_with(&self.state.paths.root) {
                Some(self.display_storage_path(parent))
            } else {
                None
            }
        });

        Ok(WorkspaceDirectoryBrowserResponse {
            current_path: self.display_storage_path(&current_path),
            parent_path: if current_path == self.state.paths.root {
                None
            } else {
                parent_path
            },
            entries,
        })
    }

    async fn delete_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let record = self.resource_record(resource_id)?;
        if record.workspace_id != workspace_id {
            return Err(AppError::not_found(format!(
                "resource {} not found in workspace {}",
                resource_id, workspace_id
            )));
        }
        self.delete_managed_resource_storage(&record)?;
        self.delete_resource_record(resource_id)?;
        Ok(())
    }

    async fn delete_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let record = self.resource_record(resource_id)?;
        if record.project_id.as_deref() != Some(project_id) {
            return Err(AppError::not_found(format!(
                "resource {} not found in project {}",
                resource_id, project_id
            )));
        }
        self.delete_managed_resource_storage(&record)?;
        self.delete_resource_record(resource_id)?;
        Ok(())
    }

    async fn update_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut record = self.resource_record(resource_id)?;
        if record.workspace_id != workspace_id {
            return Err(AppError::not_found(format!(
                "resource {} not found in workspace {}",
                resource_id, workspace_id
            )));
        }
        self.apply_resource_update(&mut record, input)?;
        self.persist_resource_record(&record, true)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    async fn update_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut record = self.resource_record(resource_id)?;
        if record.project_id.as_deref() != Some(project_id) {
            return Err(AppError::not_found(format!(
                "resource {} not found in project {}",
                resource_id, project_id
            )));
        }
        self.apply_resource_update(&mut record, input)?;
        self.persist_resource_record(&record, true)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    async fn list_workspace_knowledge(&self) -> Result<Vec<KnowledgeRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .clone())
    }

    async fn list_project_knowledge(
        &self,
        project_id: &str,
    ) -> Result<Vec<KnowledgeRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id.as_deref() == Some(project_id))
            .cloned()
            .collect())
    }

    async fn get_workspace_pet_snapshot(
        &self,
        owner_user_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.workspace_pet_snapshot(owner_user_id)
    }

    async fn get_project_pet_snapshot(
        &self,
        owner_user_id: &str,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.project_pet_snapshot(owner_user_id, project_id)
    }

    async fn save_workspace_pet_presence(
        &self,
        owner_user_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        let snapshot = self.workspace_pet_snapshot(owner_user_id)?;
        let mut presence = self
            .state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .get(&pet_context_key(owner_user_id, None))
            .cloned()
            .unwrap_or(snapshot.presence);
        if !input.pet_id.trim().is_empty() {
            presence.pet_id = input.pet_id;
        }
        if let Some(value) = input.is_visible {
            presence.is_visible = value;
        }
        if let Some(value) = input.chat_open {
            presence.chat_open = value;
        }
        if let Some(value) = input.motion_state {
            presence.motion_state = value;
        }
        if let Some(value) = input.unread_count {
            presence.unread_count = value;
        }
        if let Some(value) = input.last_interaction_at {
            presence.last_interaction_at = value;
        }
        if let Some(value) = input.position {
            presence.position = value;
        }
        self.persist_pet_presence(owner_user_id, None, &presence)?;
        self.state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .insert(pet_context_key(owner_user_id, None), presence.clone());
        Ok(presence)
    }

    async fn save_project_pet_presence(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        self.ensure_project_exists(project_id)?;
        let snapshot = self.project_pet_snapshot(owner_user_id, project_id)?;
        let mut presence = self
            .state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .get(&pet_context_key(owner_user_id, Some(project_id)))
            .cloned()
            .unwrap_or(snapshot.presence);
        if !input.pet_id.trim().is_empty() {
            presence.pet_id = input.pet_id;
        }
        if let Some(value) = input.is_visible {
            presence.is_visible = value;
        }
        if let Some(value) = input.chat_open {
            presence.chat_open = value;
        }
        if let Some(value) = input.motion_state {
            presence.motion_state = value;
        }
        if let Some(value) = input.unread_count {
            presence.unread_count = value;
        }
        if let Some(value) = input.last_interaction_at {
            presence.last_interaction_at = value;
        }
        if let Some(value) = input.position {
            presence.position = value;
        }
        self.persist_pet_presence(owner_user_id, Some(project_id), &presence)?;
        self.state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .insert(
                pet_context_key(owner_user_id, Some(project_id)),
                presence.clone(),
            );
        Ok(presence)
    }

    async fn bind_workspace_pet_conversation(
        &self,
        owner_user_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        let snapshot = self.workspace_pet_snapshot(owner_user_id)?;
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                snapshot.profile.id
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            owner_user_id: owner_user_id.into(),
            context_scope: "home".into(),
            project_id: None,
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(owner_user_id, None, &binding)?;
        self.state
            .pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("pet bindings mutex poisoned"))?
            .insert(pet_context_key(owner_user_id, None), binding.clone());
        Ok(binding)
    }

    async fn bind_project_pet_conversation(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        self.ensure_project_exists(project_id)?;
        let snapshot = self.project_pet_snapshot(owner_user_id, project_id)?;
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                snapshot.profile.id
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            owner_user_id: owner_user_id.into(),
            context_scope: "project".into(),
            project_id: Some(project_id.into()),
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(owner_user_id, Some(project_id), &binding)?;
        self.state
            .pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("pet bindings mutex poisoned"))?
            .insert(
                pet_context_key(owner_user_id, Some(project_id)),
                binding.clone(),
            );
        Ok(binding)
    }

    async fn list_agents(&self) -> Result<Vec<AgentRecord>, AppError> {
        let workspace_id = self.state.workspace_id()?;
        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?
            .iter()
            .filter(|record| record.asset_role != "pet")
            .cloned()
            .collect::<Vec<_>>();
        agents.extend(crate::agent_bundle::list_builtin_agent_templates(
            &workspace_id,
        )?);
        agents.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(agents)
    }

    async fn create_agent(&self, input: UpsertAgentInput) -> Result<AgentRecord, AppError> {
        let agent_id = format!("agent-{}", Uuid::new_v4());
        let record = self.build_agent_record(&agent_id, input, None)?;

        write_agent_record(&self.state.open_db()?, &record, false)?;

        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        agents.push(record.clone());
        Ok(record)
    }

    async fn update_agent(
        &self,
        agent_id: &str,
        input: UpsertAgentInput,
    ) -> Result<AgentRecord, AppError> {
        let current = {
            self.state
                .agents
                .lock()
                .map_err(|_| AppError::runtime("agents mutex poisoned"))?
                .iter()
                .find(|item| item.id == agent_id)
                .cloned()
        };
        let record = self.build_agent_record(agent_id, input, current.as_ref())?;

        write_agent_record(&self.state.open_db()?, &record, true)?;

        if let Some(previous) = current.as_ref() {
            if previous.avatar_path != record.avatar_path {
                self.remove_avatar_file(previous.avatar_path.as_deref())?;
            }
        }

        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        Self::replace_or_push(&mut agents, record.clone(), |item| item.id == agent_id);
        Ok(record)
    }

    async fn delete_agent(&self, agent_id: &str) -> Result<(), AppError> {
        let removed = {
            let mut agents = self
                .state
                .agents
                .lock()
                .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
            let existing = agents.iter().find(|item| item.id == agent_id).cloned();
            agents.retain(|item| item.id != agent_id);
            existing
        };

        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM agents WHERE id = ?1", params![agent_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM project_agent_links WHERE agent_id = ?1",
                params![agent_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .retain(|item| item.agent_id != agent_id);

        if let Some(record) = removed {
            self.remove_avatar_file(record.avatar_path.as_deref())?;
        }
        Ok(())
    }

    async fn preview_import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::preview_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Workspace,
            input,
        )
    }

    async fn import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Workspace,
            input,
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    async fn copy_workspace_agent_from_builtin(
        &self,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_agent_asset(agent_assets::AssetTargetScope::Workspace, agent_id)
    }

    async fn export_agent_bundle(
        &self,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::export_assets(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Workspace,
            input,
        )
    }

    async fn preview_import_project_agent_bundle(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::preview_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Project(project_id),
            input,
        )
    }

    async fn import_project_agent_bundle(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Project(project_id),
            input,
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    async fn copy_project_agent_from_builtin(
        &self,
        project_id: &str,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_agent_asset(
            agent_assets::AssetTargetScope::Project(project_id),
            agent_id,
        )
    }

    async fn export_project_agent_bundle(
        &self,
        project_id: &str,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::export_assets(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Project(project_id),
            input,
        )
    }

    async fn list_project_agent_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAgentLinkRecord>, AppError> {
        Ok(self
            .state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn link_project_agent(
        &self,
        input: ProjectAgentLinkInput,
    ) -> Result<ProjectAgentLinkRecord, AppError> {
        let record = ProjectAgentLinkRecord {
            workspace_id: self.state.workspace_id()?,
            project_id: input.project_id,
            agent_id: input.agent_id,
            linked_at: Self::now(),
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO project_agent_links (workspace_id, project_id, agent_id, linked_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.workspace_id, record.project_id, record.agent_id, record.linked_at as i64],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut links = self
            .state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?;
        Self::replace_or_push(&mut links, record.clone(), |item| {
            item.project_id == record.project_id && item.agent_id == record.agent_id
        });
        Ok(record)
    }

    async fn unlink_project_agent(&self, project_id: &str, agent_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM project_agent_links WHERE project_id = ?1 AND agent_id = ?2",
                params![project_id, agent_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .retain(|item| !(item.project_id == project_id && item.agent_id == agent_id));
        Ok(())
    }

    async fn list_teams(&self) -> Result<Vec<TeamRecord>, AppError> {
        let workspace_id = self.state.workspace_id()?;
        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?
            .clone();
        teams.extend(crate::agent_bundle::list_builtin_team_templates(
            &workspace_id,
        )?);
        teams.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(teams)
    }

    async fn create_team(&self, input: UpsertTeamInput) -> Result<TeamRecord, AppError> {
        let team_id = format!("team-{}", Uuid::new_v4());
        let record = self.build_team_record(&team_id, input, None)?;

        write_team_record(&self.state.open_db()?, &record, false)?;

        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        teams.push(record.clone());
        Ok(record)
    }

    async fn update_team(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
    ) -> Result<TeamRecord, AppError> {
        let current = {
            self.state
                .teams
                .lock()
                .map_err(|_| AppError::runtime("teams mutex poisoned"))?
                .iter()
                .find(|item| item.id == team_id)
                .cloned()
        };
        let record = self.build_team_record(team_id, input, current.as_ref())?;

        write_team_record(&self.state.open_db()?, &record, true)?;

        if let Some(previous) = current.as_ref() {
            if previous.avatar_path != record.avatar_path {
                self.remove_avatar_file(previous.avatar_path.as_deref())?;
            }
        }

        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        Self::replace_or_push(&mut teams, record.clone(), |item| item.id == team_id);
        Ok(record)
    }

    async fn delete_team(&self, team_id: &str) -> Result<(), AppError> {
        let removed = {
            let mut teams = self
                .state
                .teams
                .lock()
                .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
            let existing = teams.iter().find(|item| item.id == team_id).cloned();
            teams.retain(|item| item.id != team_id);
            existing
        };

        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM teams WHERE id = ?1", params![team_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM project_team_links WHERE team_id = ?1",
                params![team_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .retain(|item| item.team_id != team_id);

        if let Some(record) = removed {
            self.remove_avatar_file(record.avatar_path.as_deref())?;
        }
        Ok(())
    }

    async fn copy_workspace_team_from_builtin(
        &self,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_team_asset(agent_assets::AssetTargetScope::Workspace, team_id)
    }

    async fn copy_project_team_from_builtin(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_team_asset(agent_assets::AssetTargetScope::Project(project_id), team_id)
    }

    async fn list_project_team_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectTeamLinkRecord>, AppError> {
        Ok(self
            .state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    async fn link_project_team(
        &self,
        input: ProjectTeamLinkInput,
    ) -> Result<ProjectTeamLinkRecord, AppError> {
        let record = ProjectTeamLinkRecord {
            workspace_id: self.state.workspace_id()?,
            project_id: input.project_id,
            team_id: input.team_id,
            linked_at: Self::now(),
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO project_team_links (workspace_id, project_id, team_id, linked_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.workspace_id, record.project_id, record.team_id, record.linked_at as i64],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut links = self
            .state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?;
        Self::replace_or_push(&mut links, record.clone(), |item| {
            item.project_id == record.project_id && item.team_id == record.team_id
        });
        Ok(record)
    }

    async fn unlink_project_team(&self, project_id: &str, team_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM project_team_links WHERE project_id = ?1 AND team_id = ?2",
                params![project_id, team_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .retain(|item| !(item.project_id == project_id && item.team_id == team_id));
        Ok(())
    }

    async fn list_models(&self) -> Result<Vec<ModelCatalogRecord>, AppError> {
        Ok(self
            .state
            .model_catalog
            .lock()
            .map_err(|_| AppError::runtime("model catalog mutex poisoned"))?
            .clone())
    }

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialRecord>, AppError> {
        Ok(self
            .state
            .provider_credentials
            .lock()
            .map_err(|_| AppError::runtime("provider credentials mutex poisoned"))?
            .clone())
    }

    async fn get_capability_management_projection(
        &self,
    ) -> Result<octopus_core::CapabilityManagementProjection, AppError> {
        self.build_capability_management_projection().await
    }

    async fn set_capability_asset_disabled(
        &self,
        patch: CapabilityAssetDisablePatch,
    ) -> Result<octopus_core::CapabilityManagementProjection, AppError> {
        let entries = self.build_tool_catalog_entries().await?;
        if !entries
            .iter()
            .any(|entry| entry.source_key == patch.source_key)
        {
            return Err(AppError::not_found("workspace capability asset"));
        }

        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_enabled(&mut asset_state, &patch.source_key, !patch.disabled);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        self.build_capability_management_projection().await
    }

    async fn get_workspace_skill(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.get_workspace_skill_document(skill_id)
    }

    async fn create_workspace_skill(
        &self,
        input: CreateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let slug = validate_skill_slug(&input.slug)?;
        let skill_dir = workspace_owned_skill_root(&self.state.paths).join(&slug);
        if skill_dir.exists() {
            return Err(AppError::conflict(format!(
                "workspace skill '{slug}' already exists"
            )));
        }
        fs::create_dir_all(&skill_dir)?;
        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, input.content)?;
        let document = skill_document_from_path(
            &self.state.paths.root,
            &skill_path,
            SkillSourceOrigin::SkillsDir,
        )?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(document)
    }

    async fn update_workspace_skill(
        &self,
        skill_id: &str,
        input: UpdateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        fs::write(&entry.path, input.content)?;
        skill_document_from_path(&self.state.paths.root, &entry.path, entry.origin)
    }

    async fn get_workspace_skill_tree(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError> {
        self.get_workspace_skill_tree_document(skill_id)
    }

    async fn get_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        self.get_workspace_skill_file_document(skill_id, relative_path)
    }

    async fn update_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
        input: UpdateWorkspaceSkillFileInput,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        let skill_root = skill_root_path(&entry.path, entry.origin)?;
        let path = resolve_skill_file_path(&skill_root, entry.origin, relative_path)?;
        if !path.exists() {
            return Err(AppError::not_found("workspace skill file"));
        }
        let existing = self.get_workspace_skill_file_document(skill_id, relative_path)?;
        if !existing.is_text {
            return Err(AppError::invalid_input(
                "binary skill files cannot be edited in the workspace tool page",
            ));
        }
        fs::write(&path, input.content)?;
        skill_file_document_from_path(
            &self.state.paths.root,
            skill_id,
            &skill_source_key(&entry.path, &self.state.paths.root),
            &skill_root,
            entry.origin,
            &path,
            false,
        )
    }

    async fn copy_workspace_skill_to_managed(
        &self,
        skill_id: &str,
        input: CopyWorkspaceSkillToManagedInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        if let Some(asset) = crate::agent_bundle::find_builtin_skill_asset_by_id(skill_id)? {
            let document = self.import_skill_files_to_managed_root(&input.slug, asset.files)?;
            let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
            set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
            save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
            return Ok(document);
        }
        let entry = self.find_skill_catalog_entry(skill_id)?;
        let source_root = skill_root_path(&entry.path, entry.origin)?;
        let files = match entry.origin {
            SkillSourceOrigin::SkillsDir => {
                let mut collected = Vec::new();
                for node in build_skill_tree(&source_root, entry.origin)? {
                    collect_tree_files(&source_root, &node, &mut collected)?;
                }
                collected
            }
            SkillSourceOrigin::LegacyCommandsDir => vec![(
                source_root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                fs::read(&source_root)?,
            )],
        };
        let document = self.import_skill_files_to_managed_root(&input.slug, files)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(document)
    }

    async fn import_workspace_skill_archive(
        &self,
        input: ImportWorkspaceSkillArchiveInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let files = extract_archive_entries(&input)?;
        let document = self.import_skill_files_to_managed_root(&input.slug, files)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(document)
    }

    async fn import_workspace_skill_folder(
        &self,
        input: ImportWorkspaceSkillFolderInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let files = normalize_uploaded_files(&input.files)?;
        let document = self.import_skill_files_to_managed_root(&input.slug, files)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(document)
    }

    async fn delete_workspace_skill(&self, skill_id: &str) -> Result<(), AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        let source_key = skill_source_key(&entry.path, &self.state.paths.root);
        let skill_dir = entry
            .path
            .parent()
            .ok_or_else(|| AppError::invalid_input("workspace skill path is invalid"))?;
        fs::remove_dir_all(skill_dir)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        remove_workspace_asset_metadata(&mut asset_state, &source_key);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(())
    }

    async fn get_workspace_mcp_server(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        self.load_mcp_server_document(server_name)
    }

    async fn create_workspace_mcp_server(
        &self,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        if input.server_name.trim().is_empty() {
            return Err(AppError::invalid_input("serverName is required"));
        }
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if servers.contains_key(&input.server_name) {
            return Err(AppError::conflict(format!(
                "mcp server '{}' already exists",
                input.server_name
            )));
        }
        let config =
            input.config.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("mcp server config must be a JSON object")
            })?;
        servers.insert(input.server_name.clone(), serde_json::Value::Object(config));
        self.save_workspace_runtime_document(document)?;
        let document = self.load_mcp_server_document(&input.server_name)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(document)
    }

    async fn copy_workspace_mcp_server_to_managed(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        let asset = crate::agent_bundle::find_builtin_mcp_asset(server_name)?
            .ok_or_else(|| AppError::not_found("builtin mcp server"))?;
        let config =
            asset.config.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("mcp server config must be a JSON object")
            })?;
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if servers.contains_key(server_name) {
            return Err(AppError::conflict(format!(
                "mcp server '{}' already exists",
                server_name
            )));
        }
        servers.insert(server_name.into(), serde_json::Value::Object(config));
        self.save_workspace_runtime_document(document)?;
        let document = self.load_mcp_server_document(server_name)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(document)
    }

    async fn update_workspace_mcp_server(
        &self,
        server_name: &str,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        if input.server_name != server_name {
            return Err(AppError::invalid_input(
                "serverName in body must match the route parameter",
            ));
        }
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if !servers.contains_key(server_name) {
            return Err(AppError::not_found("workspace mcp server"));
        }
        let config =
            input.config.as_object().cloned().ok_or_else(|| {
                AppError::invalid_input("mcp server config must be a JSON object")
            })?;
        servers.insert(server_name.into(), serde_json::Value::Object(config));
        self.save_workspace_runtime_document(document)?;
        let document = self.load_mcp_server_document(server_name)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        set_workspace_asset_trusted(&mut asset_state, &document.source_key, true);
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(document)
    }

    async fn delete_workspace_mcp_server(&self, server_name: &str) -> Result<(), AppError> {
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if servers.remove(server_name).is_none() {
            return Err(AppError::not_found("workspace mcp server"));
        }
        self.save_workspace_runtime_document(document)?;
        let mut asset_state = load_workspace_asset_state_document(&self.state.paths)?;
        remove_workspace_asset_metadata(&mut asset_state, &format!("mcp:{server_name}"));
        save_workspace_asset_state_document(&self.state.paths, &asset_state)?;
        Ok(())
    }

    async fn list_tools(&self) -> Result<Vec<ToolRecord>, AppError> {
        Ok(self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?
            .clone())
    }

    async fn create_tool(&self, mut record: ToolRecord) -> Result<ToolRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("tool-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT INTO tools (id, workspace_id, kind, name, description, status, permission_mode, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.workspace_id,
                record.kind,
                record.name,
                record.description,
                record.status,
                record.permission_mode,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut tools = self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?;
        tools.push(record.clone());
        Ok(record)
    }

    async fn update_tool(
        &self,
        tool_id: &str,
        mut record: ToolRecord,
    ) -> Result<ToolRecord, AppError> {
        record.id = tool_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }
        record.updated_at = Self::now();

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO tools (id, workspace_id, kind, name, description, status, permission_mode, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.id,
                record.workspace_id,
                record.kind,
                record.name,
                record.description,
                record.status,
                record.permission_mode,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut tools = self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?;
        Self::replace_or_push(&mut tools, record.clone(), |item| item.id == tool_id);
        Ok(record)
    }

    async fn delete_tool(&self, tool_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM tools WHERE id = ?1", params![tool_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?
            .retain(|item| item.id != tool_id);
        Ok(())
    }

    async fn current_user_profile(&self, user_id: &str) -> Result<UserRecordSummary, AppError> {
        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let user = users
            .iter()
            .find(|item| item.record.id == user_id)
            .ok_or_else(|| AppError::not_found("workspace user"))?;
        Ok(to_user_summary(&self.state.paths, user))
    }

    async fn update_current_user_profile(
        &self,
        user_id: &str,
        request: UpdateCurrentUserProfileRequest,
    ) -> Result<UserRecordSummary, AppError> {
        let username = request.username.trim();
        let display_name = request.display_name.trim();
        if username.is_empty() || display_name.is_empty() {
            return Err(AppError::invalid_input(
                "username and display name are required",
            ));
        }

        let current_user = {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            if users
                .iter()
                .any(|user| user.record.id != user_id && user.record.username == username)
            {
                return Err(AppError::conflict("username already exists"));
            }
            users
                .iter()
                .find(|user| user.record.id == user_id)
                .cloned()
                .ok_or_else(|| AppError::not_found("workspace user"))?
        };

        let next_avatar = if let Some(avatar) = request.avatar.as_ref() {
            let (avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash) =
                self.persist_avatar(user_id, avatar)?;
            (
                Some(avatar_path),
                Some(avatar_content_type),
                Some(avatar_byte_size),
                Some(avatar_content_hash),
            )
        } else if request.remove_avatar.unwrap_or(false) {
            (None, None, None, None)
        } else {
            (
                current_user.record.avatar_path.clone(),
                current_user.record.avatar_content_type.clone(),
                current_user.record.avatar_byte_size,
                current_user.record.avatar_content_hash.clone(),
            )
        };

        let now = Self::now();
        self.state
            .open_db()?
            .execute(
                "UPDATE users
                 SET username = ?2,
                     display_name = ?3,
                     avatar_path = ?4,
                     avatar_content_type = ?5,
                     avatar_byte_size = ?6,
                     avatar_content_hash = ?7,
                     updated_at = ?8
                 WHERE id = ?1",
                params![
                    user_id,
                    username,
                    display_name,
                    next_avatar.0.clone(),
                    next_avatar.1.clone(),
                    next_avatar.2.map(|value| value as i64),
                    next_avatar.3.clone(),
                    now as i64
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let user = users
            .iter_mut()
            .find(|item| item.record.id == user_id)
            .ok_or_else(|| AppError::not_found("workspace user"))?;
        user.record.username = username.to_string();
        user.record.display_name = display_name.to_string();
        user.record.avatar_path = next_avatar.0.clone();
        user.record.avatar_content_type = next_avatar.1.clone();
        user.record.avatar_byte_size = next_avatar.2;
        user.record.avatar_content_hash = next_avatar.3.clone();
        user.record.updated_at = now;

        if current_user.record.avatar_path != next_avatar.0 {
            self.remove_avatar_file(current_user.record.avatar_path.as_deref())?;
        }

        Ok(to_user_summary(&self.state.paths, user))
    }

    async fn change_current_user_password(
        &self,
        user_id: &str,
        request: ChangeCurrentUserPasswordRequest,
    ) -> Result<ChangeCurrentUserPasswordResponse, AppError> {
        if request.new_password.len() < 8 {
            return Err(AppError::invalid_input(
                "new password must be at least 8 characters",
            ));
        }
        if request.new_password != request.confirm_password {
            return Err(AppError::invalid_input(
                "password confirmation does not match",
            ));
        }
        if request.new_password == request.current_password {
            return Err(AppError::invalid_input(
                "new password must be different from current password",
            ));
        }

        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let user = users
            .iter_mut()
            .find(|item| item.record.id == user_id)
            .ok_or_else(|| AppError::not_found("workspace user"))?;
        if !verify_password(&request.current_password, &user.password_hash) {
            return Err(AppError::invalid_input("current password is incorrect"));
        }
        user.password_hash = hash_password(&request.new_password);
        user.record.password_state = "set".into();
        user.record.updated_at = Self::now();
        self.state
            .open_db()?
            .execute(
                "UPDATE users SET password_hash = ?2, password_state = ?3, updated_at = ?4 WHERE id = ?1",
                params![
                    user_id,
                    user.password_hash,
                    user.record.password_state,
                    user.record.updated_at as i64,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(ChangeCurrentUserPasswordResponse {
            password_state: user.record.password_state.clone(),
        })
    }
}

impl InfraWorkspaceService {
    fn refresh_agent_and_team_caches(&self, connection: &Connection) -> Result<(), AppError> {
        let next_agents = load_agents(connection)?;
        let next_teams = load_teams(connection)?;
        *self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))? = next_agents;
        *self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))? = next_teams;
        Ok(())
    }

    fn normalize_resource_directory(&self, value: &str) -> Result<String, AppError> {
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input(
                "project resource directory is required",
            ));
        }
        let path = PathBuf::from(normalized);
        if path.is_absolute() {
            return Ok(self.display_storage_path(&path));
        }
        Ok(normalized.trim_end_matches('/').replace('\\', "/"))
    }

    fn normalize_resource_name(name: &str) -> Result<String, AppError> {
        let normalized = name.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input("resource name is required"));
        }
        Ok(normalized.into())
    }

    fn normalize_resource_location(location: Option<String>) -> Option<String> {
        location.and_then(|value| {
            let normalized = value.trim().replace('\\', "/");
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
    }

    fn normalize_resource_scope(
        &self,
        project_id: Option<&str>,
        value: &str,
    ) -> Result<String, AppError> {
        let normalized = value.trim();
        let normalized = if normalized.is_empty() {
            if project_id.is_some() {
                "project"
            } else {
                "workspace"
            }
        } else {
            normalized
        };

        match normalized {
            "personal" => Ok("personal".into()),
            "project" if project_id.is_some() => Ok("project".into()),
            "workspace" => Ok("workspace".into()),
            "project" => Err(AppError::invalid_input(
                "workspace resources cannot use project scope",
            )),
            _ => Err(AppError::invalid_input("resource scope is invalid")),
        }
    }

    fn normalize_resource_visibility(&self, value: &str) -> Result<String, AppError> {
        match value.trim() {
            "" | "public" => Ok("public".into()),
            "private" => Ok("private".into()),
            _ => Err(AppError::invalid_input("resource visibility is invalid")),
        }
    }

    fn normalize_promoted_scope(
        &self,
        record: &WorkspaceResourceRecord,
        requested_scope: &str,
    ) -> Result<String, AppError> {
        let requested = requested_scope.trim();
        if requested.is_empty() || requested == record.scope {
            return Ok(record.scope.clone());
        }

        match (
            record.scope.as_str(),
            requested,
            record.project_id.is_some(),
        ) {
            ("personal", "project", true) => Ok("project".into()),
            ("personal", "workspace", false) | ("project", "workspace", _) => {
                Ok("workspace".into())
            }
            ("workspace", _, _) => Err(AppError::invalid_input(
                "workspace resources cannot be promoted further",
            )),
            _ => Err(AppError::invalid_input(
                "resource promotion scope is invalid",
            )),
        }
    }

    fn copy_agent_asset(
        &self,
        target: agent_assets::AssetTargetScope<'_>,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let files = if let Some(files) =
            crate::agent_bundle::extract_builtin_agent_template_files(agent_id)?
        {
            files
        } else {
            let source = load_agents(&connection)?
                .into_iter()
                .find(|record| record.id == agent_id)
                .ok_or_else(|| AppError::not_found("agent not found"))?;
            let source_scope = match source.project_id.as_deref() {
                Some(project_id) => agent_assets::AssetTargetScope::Project(project_id),
                None => agent_assets::AssetTargetScope::Workspace,
            };
            crate::agent_bundle::export_assets(
                &connection,
                &self.state.paths,
                &workspace_id,
                match source_scope {
                    agent_assets::AssetTargetScope::Workspace => {
                        crate::agent_bundle::BundleTarget::Workspace
                    }
                    agent_assets::AssetTargetScope::Project(project_id) => {
                        crate::agent_bundle::BundleTarget::Project(project_id)
                    }
                },
                ExportWorkspaceAgentBundleInput {
                    mode: "single".into(),
                    agent_ids: vec![agent_id.into()],
                    team_ids: Vec::new(),
                },
            )?
            .files
        };

        let result = crate::agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            match target {
                agent_assets::AssetTargetScope::Workspace => {
                    crate::agent_bundle::BundleTarget::Workspace
                }
                agent_assets::AssetTargetScope::Project(project_id) => {
                    crate::agent_bundle::BundleTarget::Project(project_id)
                }
            },
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    fn copy_team_asset(
        &self,
        target: agent_assets::AssetTargetScope<'_>,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let files = if let Some(files) =
            crate::agent_bundle::extract_builtin_team_template_files(team_id)?
        {
            files
        } else {
            let source = load_teams(&connection)?
                .into_iter()
                .find(|record| record.id == team_id)
                .ok_or_else(|| AppError::not_found("team not found"))?;
            let source_scope = match source.project_id.as_deref() {
                Some(project_id) => agent_assets::AssetTargetScope::Project(project_id),
                None => agent_assets::AssetTargetScope::Workspace,
            };
            crate::agent_bundle::export_assets(
                &connection,
                &self.state.paths,
                &workspace_id,
                match source_scope {
                    agent_assets::AssetTargetScope::Workspace => {
                        crate::agent_bundle::BundleTarget::Workspace
                    }
                    agent_assets::AssetTargetScope::Project(project_id) => {
                        crate::agent_bundle::BundleTarget::Project(project_id)
                    }
                },
                ExportWorkspaceAgentBundleInput {
                    mode: "single".into(),
                    agent_ids: Vec::new(),
                    team_ids: vec![team_id.into()],
                },
            )?
            .files
        };

        let result = crate::agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            match target {
                agent_assets::AssetTargetScope::Workspace => {
                    crate::agent_bundle::BundleTarget::Workspace
                }
                agent_assets::AssetTargetScope::Project(project_id) => {
                    crate::agent_bundle::BundleTarget::Project(project_id)
                }
            },
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    fn normalize_resource_status(status: &str) -> Result<String, AppError> {
        let normalized = status.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input("resource status is required"));
        }
        Ok(normalized.into())
    }

    fn project_record(&self, project_id: &str) -> Result<ProjectRecord, AppError> {
        self.state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("project not found"))
    }

    fn build_project_deletion_request_inbox_items(
        &self,
        request: &ProjectDeletionRequest,
    ) -> Result<Vec<InboxItemRecord>, AppError> {
        let project = self.project_record(&request.project_id)?;
        let approver_user_ids = resolve_project_deletion_approver_user_ids(
            &self.state.open_db()?,
            &request.project_id,
        )?;
        if approver_user_ids.is_empty() {
            return Err(AppError::conflict(
                "project deletion request requires at least one eligible approver",
            ));
        }

        let description = request
            .reason
            .as_deref()
            .map(|reason| format!("Archived project deletion requested: {reason}"))
            .unwrap_or_else(|| "Archived project deletion requested.".into());
        let project_settings_route = format!(
            "/workspaces/{}/projects/{}/settings",
            request.workspace_id, request.project_id
        );

        Ok(approver_user_ids
            .into_iter()
            .map(|user_id| InboxItemRecord {
                id: format!("project-deletion-request-{}-{user_id}", request.id),
                workspace_id: request.workspace_id.clone(),
                project_id: Some(request.project_id.clone()),
                target_user_id: user_id,
                item_type: PROJECT_DELETION_REQUEST_INBOX_ITEM_TYPE.into(),
                title: format!("Review deletion for {}", project.name),
                description: description.clone(),
                status: "pending".into(),
                priority: "high".into(),
                actionable: true,
                route_to: Some(project_settings_route.clone()),
                action_label: Some(PROJECT_DELETION_REQUEST_ACTION_LABEL.into()),
                created_at: request.created_at,
            })
            .collect())
    }

    fn push_inbox_items(&self, items: Vec<InboxItemRecord>) -> Result<(), AppError> {
        self.state
            .inbox
            .lock()
            .map_err(|_| AppError::runtime("inbox mutex poisoned"))?
            .extend(items);
        Ok(())
    }

    fn resolve_project_deletion_request_inbox_items(
        &self,
        request_id: &str,
        reviewed_by_user_id: &str,
        approved: bool,
    ) -> Result<(), AppError> {
        let item_prefix = format!("project-deletion-request-{request_id}-");
        let reviewed_status = if approved { "approved" } else { "rejected" };
        let mut inbox = self
            .state
            .inbox
            .lock()
            .map_err(|_| AppError::runtime("inbox mutex poisoned"))?;

        for item in inbox.iter_mut().filter(|item| {
            item.item_type == PROJECT_DELETION_REQUEST_INBOX_ITEM_TYPE
                && item.id.starts_with(&item_prefix)
        }) {
            item.actionable = false;
            if item.target_user_id == reviewed_by_user_id {
                item.status = reviewed_status.into();
            } else if item.status == "pending" {
                item.status = "closed".into();
            }
        }

        Ok(())
    }

    fn resource_record(&self, resource_id: &str) -> Result<WorkspaceResourceRecord, AppError> {
        self.state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .iter()
            .find(|record| record.id == resource_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("resource not found"))
    }

    fn linked_workspace_assets_from_assignments(
        _assignments: Option<&ProjectWorkspaceAssignments>,
    ) -> ProjectLinkedWorkspaceAssets {
        // Live-inheritance assignments now describe project-local deltas and project-owned assets.
        // Workspace links remain an explicit compatibility read model, so do not derive them here.
        empty_project_linked_workspace_assets()
    }

    fn required_workspace_capability_for_project_asset(
        asset_type: &str,
    ) -> Result<String, AppError> {
        match asset_type.trim() {
            "agent" => Ok("agent.publish".into()),
            "resource" => Ok("resource.publish".into()),
            "knowledge" => Ok("knowledge.publish".into()),
            "tool.skill" => Ok("tool.skill.publish".into()),
            "tool.mcp" => Ok("tool.mcp.publish".into()),
            _ => Err(AppError::invalid_input(format!(
                "unsupported project promotion asset type: {asset_type}"
            ))),
        }
    }

    fn upsert_resource_cache(&self, record: WorkspaceResourceRecord) -> Result<(), AppError> {
        let record_id = record.id.clone();
        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        Self::replace_or_push(&mut resources, record, |item| item.id == record_id);
        Ok(())
    }

    fn delete_resource_record(&self, resource_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM resources WHERE id = ?1", params![resource_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .retain(|record| record.id != resource_id);
        Ok(())
    }

    fn persist_project_record(
        &self,
        record: &ProjectRecord,
        replace: bool,
    ) -> Result<(), AppError> {
        let verb = if replace {
            "INSERT OR REPLACE"
        } else {
            "INSERT"
        };
        let sql = format!(
            "{verb} INTO projects (id, workspace_id, name, status, description, resource_directory, leader_agent_id, manager_user_id, preset_code, assignments_json, owner_user_id, member_user_ids_json, permission_overrides_json, linked_workspace_assets_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
        );
        let assignments_json = record
            .assignments
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;
        let linked_workspace_assets_json =
            if record.linked_workspace_assets == empty_project_linked_workspace_assets() {
                None
            } else {
                Some(serde_json::to_string(&record.linked_workspace_assets)?)
            };

        self.state
            .open_db()?
            .execute(
                &sql,
                params![
                    record.id,
                    record.workspace_id,
                    record.name,
                    record.status,
                    record.description,
                    record.resource_directory,
                    record.leader_agent_id,
                    record.manager_user_id,
                    record.preset_code,
                    assignments_json,
                    record.owner_user_id,
                    serde_json::to_string(&record.member_user_ids)?,
                    serde_json::to_string(&record.permission_overrides)?,
                    linked_workspace_assets_json,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn persist_project_promotion_request(
        &self,
        record: &ProjectPromotionRequest,
        replace: bool,
    ) -> Result<(), AppError> {
        let verb = if replace {
            "INSERT OR REPLACE"
        } else {
            "INSERT"
        };
        let sql = format!(
            "{verb} INTO project_promotion_requests (id, workspace_id, project_id, asset_type, asset_id, requested_by_user_id, submitted_by_owner_user_id, required_workspace_capability, status, reviewed_by_user_id, review_comment, created_at, updated_at, reviewed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)"
        );

        self.state
            .open_db()?
            .execute(
                &sql,
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.asset_type,
                    record.asset_id,
                    record.requested_by_user_id,
                    record.submitted_by_owner_user_id,
                    record.required_workspace_capability,
                    record.status,
                    record.reviewed_by_user_id,
                    record.review_comment,
                    record.created_at as i64,
                    record.updated_at as i64,
                    record.reviewed_at.map(|value| value as i64),
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn persist_project_deletion_request(
        &self,
        record: &ProjectDeletionRequest,
        replace: bool,
    ) -> Result<(), AppError> {
        let verb = if replace {
            "INSERT OR REPLACE"
        } else {
            "INSERT"
        };
        let sql = format!(
            "{verb} INTO project_deletion_requests (id, workspace_id, project_id, requested_by_user_id, status, reason, reviewed_by_user_id, review_comment, created_at, updated_at, reviewed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        );

        self.state
            .open_db()?
            .execute(
                &sql,
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.requested_by_user_id,
                    record.status,
                    record.reason,
                    record.reviewed_by_user_id,
                    record.review_comment,
                    record.created_at as i64,
                    record.updated_at as i64,
                    record.reviewed_at.map(|value| value as i64),
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn persist_resource_record(
        &self,
        record: &WorkspaceResourceRecord,
        replace: bool,
    ) -> Result<(), AppError> {
        let verb = if replace {
            "INSERT OR REPLACE"
        } else {
            "INSERT"
        };
        let sql = format!(
            "{verb} INTO resources (id, workspace_id, project_id, kind, name, location, origin, scope, visibility, owner_user_id, storage_path, content_type, byte_size, preview_kind, status, updated_at, tags, source_artifact_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)"
        );

        self.state
            .open_db()?
            .execute(
                &sql,
                params![
                    record.id,
                    record.workspace_id,
                    record.project_id,
                    record.kind,
                    record.name,
                    record.location,
                    record.origin,
                    record.scope,
                    record.visibility,
                    record.owner_user_id,
                    record.storage_path,
                    record.content_type,
                    record.byte_size.map(|value| value as i64),
                    record.preview_kind,
                    record.status,
                    record.updated_at as i64,
                    serde_json::to_string(&record.tags)?,
                    record.source_artifact_id,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    fn display_storage_path(&self, path: &Path) -> String {
        let display = display_path(path, &self.state.paths.root);
        if display.is_empty() {
            path.to_string_lossy().replace('\\', "/")
        } else {
            display
        }
    }

    fn resolve_storage_path(&self, stored_path: &str) -> PathBuf {
        let path = PathBuf::from(stored_path);
        if path.is_absolute() {
            path
        } else {
            self.state.paths.root.join(path)
        }
    }

    fn delete_stored_path_if_exists(&self, stored_path: &str) -> Result<(), AppError> {
        let absolute_path = self.resolve_storage_path(stored_path);
        if !absolute_path.exists() {
            return Ok(());
        }
        if absolute_path.is_dir() {
            fs::remove_dir_all(absolute_path)?;
        } else {
            fs::remove_file(absolute_path)?;
        }
        Ok(())
    }

    fn query_project_cleanup_paths(
        connection: &Connection,
        project_id: &str,
    ) -> Result<Vec<String>, AppError> {
        let mut stmt = connection
            .prepare(
                "SELECT path FROM (
                   SELECT storage_path AS path FROM artifact_records WHERE project_id = ?1 AND storage_path IS NOT NULL
                   UNION
                   SELECT storage_path AS path FROM artifact_versions WHERE project_id = ?1 AND storage_path IS NOT NULL
                   UNION
                   SELECT storage_path AS path FROM bundle_asset_descriptors WHERE project_id = ?1 AND storage_path IS NOT NULL
                   UNION
                   SELECT avatar_path AS path FROM agents WHERE project_id = ?1 AND avatar_path IS NOT NULL
                   UNION
                   SELECT avatar_path AS path FROM teams WHERE project_id = ?1 AND avatar_path IS NOT NULL
                   UNION
                   SELECT body_storage_path AS path FROM runtime_mailbox_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1) AND body_storage_path IS NOT NULL
                   UNION
                   SELECT envelope_storage_path AS path FROM runtime_handoff_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1) AND envelope_storage_path IS NOT NULL
                   UNION
                   SELECT detail_storage_path AS path FROM runtime_workflow_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1) AND detail_storage_path IS NOT NULL
                   UNION
                   SELECT state_storage_path AS path FROM runtime_background_projections WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1) AND state_storage_path IS NOT NULL
                   UNION
                   SELECT storage_path AS path FROM runtime_memory_records WHERE project_id = ?1 AND storage_path IS NOT NULL
                   UNION
                   SELECT artifact_storage_path AS path FROM runtime_memory_proposals WHERE session_id IN (SELECT id FROM runtime_session_projections WHERE project_id = ?1) AND artifact_storage_path IS NOT NULL
                 ) ORDER BY path ASC",
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        let rows = stmt
            .query_map(params![project_id], |row| row.get::<_, String>(0))
            .map_err(|error| AppError::database(error.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| AppError::database(error.to_string()))
    }

    fn refresh_project_scoped_caches(&self, connection: &Connection) -> Result<(), AppError> {
        *self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))? =
            load_projects(connection)?;
        *self
            .state
            .project_promotion_requests
            .lock()
            .map_err(|_| AppError::runtime("project promotion requests mutex poisoned"))? =
            load_project_promotion_requests(connection)?;
        *self
            .state
            .project_deletion_requests
            .lock()
            .map_err(|_| AppError::runtime("project deletion requests mutex poisoned"))? =
            load_project_deletion_requests(connection)?;
        *self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))? =
            load_resources(connection)?;
        *self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge records mutex poisoned"))? =
            load_knowledge_records(connection)?;
        *self
            .state
            .project_tasks
            .lock()
            .map_err(|_| AppError::runtime("project tasks mutex poisoned"))? =
            load_project_tasks(connection)?;
        *self
            .state
            .project_task_runs
            .lock()
            .map_err(|_| AppError::runtime("project task runs mutex poisoned"))? =
            load_project_task_runs(connection)?;
        *self
            .state
            .project_task_interventions
            .lock()
            .map_err(|_| AppError::runtime("project task interventions mutex poisoned"))? =
            load_project_task_interventions(connection)?;
        *self
            .state
            .project_task_scheduler_claims
            .lock()
            .map_err(|_| AppError::runtime("project task scheduler claims mutex poisoned"))? =
            load_project_task_scheduler_claims(connection)?;
        *self
            .state
            .artifacts
            .lock()
            .map_err(|_| AppError::runtime("artifacts mutex poisoned"))? =
            load_artifact_records(connection)?;
        *self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))? = load_agents(connection)?;
        *self
            .state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))? =
            load_project_agent_links(connection)?;
        *self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))? = load_teams(connection)?;
        *self
            .state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))? =
            load_project_team_links(connection)?;
        *self
            .state
            .trace_events
            .lock()
            .map_err(|_| AppError::runtime("trace events mutex poisoned"))? =
            load_trace_events(connection)?;
        *self
            .state
            .audit_records
            .lock()
            .map_err(|_| AppError::runtime("audit records mutex poisoned"))? =
            load_audit_records(connection)?;
        *self
            .state
            .cost_entries
            .lock()
            .map_err(|_| AppError::runtime("cost entries mutex poisoned"))? =
            load_cost_entries(connection)?;
        *self
            .state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))? =
            load_pet_presences(connection)?;
        *self
            .state
            .pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("pet bindings mutex poisoned"))? =
            load_pet_bindings(connection)?;
        Ok(())
    }

    fn resource_content_type(name: &str, location: Option<&str>) -> Option<String> {
        let extension = Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .or_else(|| {
                location.and_then(|value| {
                    Path::new(value)
                        .extension()
                        .and_then(|extension| extension.to_str())
                })
            })?
            .to_ascii_lowercase();

        let content_type = match extension.as_str() {
            "md" => "text/markdown",
            "txt" | "csv" | "rs" | "ts" | "tsx" | "js" | "jsx" | "vue" | "py" | "go" | "java"
            | "kt" | "swift" | "c" | "cc" | "cpp" | "h" | "hpp" | "html" | "css" | "yaml"
            | "yml" | "toml" | "sql" | "sh" => "text/plain",
            "json" => "application/json",
            "pdf" => "application/pdf",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "svg" => "image/svg+xml",
            "mp3" => "audio/mpeg",
            "wav" => "audio/wav",
            "ogg" => "audio/ogg",
            "m4a" => "audio/mp4",
            "mp4" => "video/mp4",
            "mov" => "video/quicktime",
            "webm" => "video/webm",
            _ => "application/octet-stream",
        };

        Some(content_type.into())
    }

    fn resource_preview_kind(
        kind: &str,
        name: &str,
        location: Option<&str>,
        content_type: Option<&str>,
    ) -> String {
        if kind == "folder" {
            return "folder".into();
        }
        if kind == "url" {
            return "url".into();
        }

        let content_type = content_type.unwrap_or_default().to_ascii_lowercase();
        if content_type == "text/markdown" {
            return "markdown".into();
        }
        if content_type.starts_with("image/") {
            return "image".into();
        }
        if content_type == "application/pdf" {
            return "pdf".into();
        }
        if content_type.starts_with("audio/") {
            return "audio".into();
        }
        if content_type.starts_with("video/") {
            return "video".into();
        }
        if content_type.starts_with("text/") || content_type == "application/json" {
            let extension = Path::new(name)
                .extension()
                .and_then(|extension| extension.to_str())
                .or_else(|| {
                    location.and_then(|value| {
                        Path::new(value)
                            .extension()
                            .and_then(|extension| extension.to_str())
                    })
                })
                .map(|extension| extension.to_ascii_lowercase());
            if extension.as_deref() == Some("md") {
                return "markdown".into();
            }
            if matches!(
                extension.as_deref(),
                Some(
                    "rs" | "ts"
                        | "tsx"
                        | "js"
                        | "jsx"
                        | "vue"
                        | "py"
                        | "go"
                        | "java"
                        | "kt"
                        | "swift"
                        | "c"
                        | "cc"
                        | "cpp"
                        | "h"
                        | "hpp"
                        | "html"
                        | "css"
                        | "json"
                        | "yaml"
                        | "yml"
                        | "toml"
                        | "sql"
                        | "sh"
                )
            ) {
                return "code".into();
            }
            return "text".into();
        }

        let lower = name.to_ascii_lowercase();
        if lower.ends_with(".md") {
            return "markdown".into();
        }
        if lower.ends_with(".pdf") {
            return "pdf".into();
        }
        if matches!(
            lower.rsplit('.').next(),
            Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg")
        ) {
            return "image".into();
        }
        if matches!(
            lower.rsplit('.').next(),
            Some("mp3" | "wav" | "ogg" | "m4a")
        ) {
            return "audio".into();
        }
        if matches!(lower.rsplit('.').next(), Some("mp4" | "mov" | "webm")) {
            return "video".into();
        }
        if matches!(
            lower.rsplit('.').next(),
            Some(
                "rs" | "ts"
                    | "tsx"
                    | "js"
                    | "jsx"
                    | "vue"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "swift"
                    | "c"
                    | "cc"
                    | "cpp"
                    | "h"
                    | "hpp"
                    | "html"
                    | "css"
                    | "json"
                    | "yaml"
                    | "yml"
                    | "toml"
                    | "sql"
                    | "sh"
            )
        ) {
            return "code".into();
        }

        "binary".into()
    }

    fn build_metadata_resource_record(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let kind = input.kind.trim().to_string();
        let name = Self::normalize_resource_name(&input.name)?;
        let location = Self::normalize_resource_location(input.location);
        let scope = self.normalize_resource_scope(
            input.project_id.as_deref(),
            input.scope.as_deref().unwrap_or_default(),
        )?;
        let visibility =
            self.normalize_resource_visibility(input.visibility.as_deref().unwrap_or("public"))?;
        let content_type = if kind == "url" {
            None
        } else {
            Self::resource_content_type(&name, location.as_deref())
        };
        let preview_kind =
            Self::resource_preview_kind(&kind, &name, location.as_deref(), content_type.as_deref());

        Ok(WorkspaceResourceRecord {
            id: format!("res-{}", Uuid::new_v4()),
            workspace_id: workspace_id.into(),
            project_id: input.project_id,
            kind: kind.clone(),
            name,
            location,
            origin: if kind == "url" {
                "generated".into()
            } else {
                "source".into()
            },
            scope,
            visibility,
            owner_user_id: owner_user_id.into(),
            storage_path: None,
            content_type,
            byte_size: None,
            preview_kind,
            status: "healthy".into(),
            updated_at: timestamp_now(),
            tags: input.tags,
            source_artifact_id: input.source_artifact_id,
        })
    }

    fn ensure_import_has_files(
        &self,
        files: &[WorkspaceResourceFolderUploadEntry],
    ) -> Result<(), AppError> {
        if files.is_empty() {
            Err(AppError::invalid_input(
                "resource import requires at least one file",
            ))
        } else {
            Ok(())
        }
    }

    fn infer_folder_root_name(
        &self,
        files: &[WorkspaceResourceFolderUploadEntry],
    ) -> Option<String> {
        let mut names = files
            .iter()
            .filter_map(|entry| entry.relative_path.split('/').next())
            .map(str::trim)
            .filter(|value: &&str| !value.is_empty())
            .map(|value| (*value).to_string())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        if names.len() == 1 {
            names.into_iter().next()
        } else {
            None
        }
    }

    fn trim_folder_root_prefix(
        &self,
        root_dir_name: Option<&str>,
        files: Vec<WorkspaceResourceFolderUploadEntry>,
    ) -> Result<Vec<WorkspaceResourceFolderUploadEntry>, AppError> {
        let Some(root_dir_name) = root_dir_name.filter(|value: &&str| !value.trim().is_empty())
        else {
            return Ok(files);
        };

        files
            .into_iter()
            .map(|entry| {
                let relative_path = entry.relative_path.replace('\\', "/");
                let trimmed = relative_path
                    .strip_prefix(&format!("{root_dir_name}/"))
                    .unwrap_or(&relative_path)
                    .to_string();
                Ok(WorkspaceResourceFolderUploadEntry {
                    relative_path: trimmed,
                    ..entry
                })
            })
            .collect()
    }

    fn normalize_uploaded_relative_path(&self, raw: &str) -> Result<PathBuf, AppError> {
        let normalized = raw.trim().replace('\\', "/");
        if normalized.is_empty() {
            return Err(AppError::invalid_input("resource file path is required"));
        }
        let path = Path::new(&normalized);
        if path.is_absolute() {
            return Err(AppError::invalid_input(
                "resource file path must be relative",
            ));
        }

        let mut safe = PathBuf::new();
        for component in path.components() {
            match component {
                Component::Normal(part) => safe.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(AppError::invalid_input("resource file path is invalid"));
                }
            }
        }

        if safe.as_os_str().is_empty() {
            return Err(AppError::invalid_input("resource file path is invalid"));
        }
        Ok(safe)
    }

    fn leaf_name(raw: &str) -> Result<String, AppError> {
        let normalized = raw.trim();
        let file_name = Path::new(normalized)
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::invalid_input("resource name is invalid"))?;
        Ok(file_name.to_string())
    }

    fn unique_target_path(&self, candidate: PathBuf, is_dir: bool) -> PathBuf {
        if !candidate.exists() {
            return candidate;
        }

        let suffix = &Uuid::new_v4().simple().to_string()[..8];
        if is_dir {
            let file_name = candidate
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("resource");
            return candidate.with_file_name(format!("{file_name}-{suffix}"));
        }

        let stem = candidate
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("resource");
        match candidate
            .extension()
            .and_then(|extension| extension.to_str())
        {
            Some(extension) => candidate.with_file_name(format!("{stem}-{suffix}.{extension}")),
            None => candidate.with_file_name(format!("{stem}-{suffix}")),
        }
    }

    fn write_single_imported_file(
        &self,
        target_path: &Path,
        entry: &WorkspaceResourceFolderUploadEntry,
    ) -> Result<(), AppError> {
        let bytes = BASE64_STANDARD
            .decode(entry.data_base64.as_bytes())
            .map_err(|error| AppError::invalid_input(error.to_string()))?;
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, bytes)?;
        Ok(())
    }

    fn write_imported_resource(
        &self,
        workspace_id: &str,
        project_id: Option<&str>,
        owner_user_id: &str,
        scope: String,
        visibility: String,
        input: WorkspaceResourceImportInput,
        target_root: &Path,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let name = Self::normalize_resource_name(&input.name)?;
        let files = self.trim_folder_root_prefix(input.root_dir_name.as_deref(), input.files)?;
        let is_folder = input
            .root_dir_name
            .as_deref()
            .is_some_and(|value: &str| !value.trim().is_empty())
            || files.len() > 1
            || files
                .iter()
                .any(|entry| entry.relative_path.replace('\\', "/").contains('/'));

        if is_folder {
            let folder_name = Self::leaf_name(input.root_dir_name.as_deref().unwrap_or(&name))?;
            let absolute_root = self.unique_target_path(target_root.join(folder_name), true);
            fs::create_dir_all(&absolute_root)?;
            for entry in &files {
                let relative_path = self.normalize_uploaded_relative_path(&entry.relative_path)?;
                self.write_single_imported_file(&absolute_root.join(relative_path), entry)?;
            }
            let storage_path = self.display_storage_path(&absolute_root);
            return Ok(WorkspaceResourceRecord {
                id: format!("res-{}", Uuid::new_v4()),
                workspace_id: workspace_id.into(),
                project_id: project_id.map(str::to_string),
                kind: "folder".into(),
                name,
                location: Some(storage_path.clone()),
                origin: "source".into(),
                scope,
                visibility,
                owner_user_id: owner_user_id.into(),
                storage_path: Some(storage_path),
                content_type: None,
                byte_size: None,
                preview_kind: "folder".into(),
                status: "healthy".into(),
                updated_at: timestamp_now(),
                tags: input.tags.unwrap_or_default(),
                source_artifact_id: None,
            });
        }

        let entry = files
            .into_iter()
            .next()
            .ok_or_else(|| AppError::invalid_input("resource import requires at least one file"))?;
        let file_name = Self::leaf_name(&entry.file_name)?;
        let absolute_path = self.unique_target_path(target_root.join(&file_name), false);
        self.write_single_imported_file(&absolute_path, &entry)?;
        let storage_path = self.display_storage_path(&absolute_path);
        let content_type = if entry.content_type.trim().is_empty() {
            Self::resource_content_type(&file_name, Some(&storage_path))
        } else {
            Some(entry.content_type.trim().into())
        };
        Ok(WorkspaceResourceRecord {
            id: format!("res-{}", Uuid::new_v4()),
            workspace_id: workspace_id.into(),
            project_id: project_id.map(str::to_string),
            kind: "file".into(),
            name,
            location: Some(storage_path.clone()),
            origin: "source".into(),
            scope,
            visibility,
            owner_user_id: owner_user_id.into(),
            storage_path: Some(storage_path.clone()),
            content_type: content_type.clone(),
            byte_size: Some(entry.byte_size),
            preview_kind: Self::resource_preview_kind(
                "file",
                &file_name,
                Some(&storage_path),
                content_type.as_deref(),
            ),
            status: "healthy".into(),
            updated_at: timestamp_now(),
            tags: input.tags.unwrap_or_default(),
            source_artifact_id: None,
        })
    }

    fn collect_resource_children(
        root: &Path,
        current: &Path,
        children: &mut Vec<WorkspaceResourceChildrenRecord>,
    ) -> Result<(), AppError> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                Self::collect_resource_children(root, &path, children)?;
                continue;
            }
            if !file_type.is_file() {
                continue;
            }

            let relative_path = path
                .strip_prefix(root)
                .map_err(|_| AppError::runtime("resource child path is invalid"))?
                .to_string_lossy()
                .replace('\\', "/");
            let file_name = entry.file_name().to_string_lossy().to_string();
            let metadata = fs::metadata(&path)?;
            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs())
                .unwrap_or_else(timestamp_now);
            let content_type = Self::resource_content_type(&file_name, Some(&relative_path));
            children.push(WorkspaceResourceChildrenRecord {
                name: file_name.clone(),
                relative_path,
                kind: "file".into(),
                preview_kind: Self::resource_preview_kind(
                    "file",
                    &file_name,
                    Some(&file_name),
                    content_type.as_deref(),
                ),
                content_type,
                byte_size: Some(metadata.len()),
                updated_at: modified_at,
            });
        }
        Ok(())
    }

    fn delete_managed_resource_storage(
        &self,
        record: &WorkspaceResourceRecord,
    ) -> Result<(), AppError> {
        let Some(storage_path) = record.storage_path.as_deref() else {
            return Ok(());
        };
        let absolute_path = self.resolve_storage_path(storage_path);
        if !absolute_path.exists() {
            return Ok(());
        }
        if absolute_path.is_dir() {
            fs::remove_dir_all(absolute_path)?;
        } else {
            fs::remove_file(absolute_path)?;
        }
        Ok(())
    }

    fn apply_resource_update(
        &self,
        record: &mut WorkspaceResourceRecord,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<(), AppError> {
        if let Some(name) = input.name {
            record.name = Self::normalize_resource_name(&name)?;
        }
        if input.location.is_some() {
            record.location = Self::normalize_resource_location(input.location);
        }
        if let Some(visibility) = input.visibility {
            record.visibility = self.normalize_resource_visibility(&visibility)?;
        }
        if let Some(status) = input.status {
            record.status = Self::normalize_resource_status(&status)?;
        }
        if let Some(tags) = input.tags {
            record.tags = tags;
        }
        record.updated_at = timestamp_now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use octopus_core::{
        AccessUserUpsertRequest, ApprovalPreference, ArtifactHandoffPolicy, CapabilityPolicy,
        CreateProjectDeletionRequestInput, DefaultModelStrategy, DelegationPolicy, LoginRequest,
        MailboxPolicy, MemoryPolicy, OutputContract, PermissionEnvelope,
        RegisterBootstrapAdminRequest, ReviewProjectDeletionRequestInput, SharedCapabilityPolicy,
        SharedMemoryPolicy, TeamTopology, WorkflowAffordance,
    };
    use octopus_platform::{AccessControlService, AuthService};

    fn runtime() -> tokio::runtime::Runtime {
        tokio::runtime::Runtime::new().expect("runtime")
    }

    fn encoded_file(
        relative_path: &str,
        content_type: &str,
        content: &str,
    ) -> octopus_core::WorkspaceResourceFolderUploadEntry {
        octopus_core::WorkspaceResourceFolderUploadEntry {
            relative_path: relative_path.into(),
            file_name: Path::new(relative_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(relative_path)
                .into(),
            content_type: content_type.into(),
            data_base64: BASE64_STANDARD.encode(content.as_bytes()),
            byte_size: content.len() as u64,
        }
    }

    fn avatar_payload() -> octopus_core::AvatarUploadPayload {
        octopus_core::AvatarUploadPayload {
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            file_name: "avatar.png".into(),
            byte_size: 8,
        }
    }

    fn bootstrap_admin_session(bundle: &crate::InfraBundle) -> SessionRecord {
        runtime()
            .block_on(
                bundle
                    .auth
                    .register_bootstrap_admin(RegisterBootstrapAdminRequest {
                        client_app_id: "octopus-desktop".into(),
                        username: "owner".into(),
                        display_name: "Owner".into(),
                        password: "password123".into(),
                        confirm_password: "password123".into(),
                        avatar: avatar_payload(),
                        workspace_id: Some("ws-local".into()),
                        mapped_directory: None,
                    }),
            )
            .expect("bootstrap admin")
            .session
    }

    fn create_user_session(
        bundle: &crate::InfraBundle,
        username: &str,
        display_name: &str,
    ) -> SessionRecord {
        runtime().block_on(async {
            bundle
                .access_control
                .create_user(AccessUserUpsertRequest {
                    username: username.into(),
                    display_name: display_name.into(),
                    status: "active".into(),
                    password: Some("password123".into()),
                    confirm_password: Some("password123".into()),
                    reset_password: Some(false),
                })
                .await
                .expect("create user");

            bundle
                .auth
                .login(LoginRequest {
                    client_app_id: "octopus-desktop".into(),
                    username: username.into(),
                    password: "password123".into(),
                    workspace_id: Some("ws-local".into()),
                })
                .await
                .expect("login user")
                .session
        })
    }

    #[test]
    fn update_workspace_moves_the_real_workspace_root_and_preserves_shell_root_pointer() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        bootstrap_admin_session(&bundle);
        let mapped_root = temp
            .path()
            .parent()
            .expect("temp parent")
            .join(format!("octopus-mapped-root-{}", uuid::Uuid::new_v4()));

        let updated = runtime()
            .block_on(bundle.workspace.update_workspace(UpdateWorkspaceRequest {
                name: Some("Workspace Moved".into()),
                avatar: None,
                remove_avatar: Some(false),
                mapped_directory: Some(mapped_root.to_string_lossy().to_string()),
            }))
            .expect("updated workspace");

        assert_eq!(updated.name, "Workspace Moved");
        assert_eq!(
            updated.mapped_directory.as_deref(),
            Some(mapped_root.to_string_lossy().as_ref())
        );
        assert_eq!(
            updated.mapped_directory_default.as_deref(),
            Some(temp.path().to_string_lossy().as_ref())
        );
        assert!(mapped_root.join("data").join("main.db").exists());
        assert!(mapped_root.join("config").join("workspace.toml").exists());
        assert!(!temp.path().join("data").join("main.db").exists());

        let shell_pointer = fs::read_to_string(temp.path().join("config").join("workspace.toml"))
            .expect("shell root workspace config");
        assert!(shell_pointer.contains(mapped_root.to_string_lossy().as_ref()));

        let reloaded = build_infra_bundle(&mapped_root).expect("reloaded bundle");
        let workspace = runtime()
            .block_on(reloaded.workspace.workspace_summary())
            .expect("reloaded workspace summary");
        assert_eq!(workspace.name, "Workspace Moved");
        assert_eq!(
            workspace.mapped_directory.as_deref(),
            Some(mapped_root.to_string_lossy().as_ref())
        );
        assert_eq!(
            workspace.mapped_directory_default.as_deref(),
            Some(temp.path().to_string_lossy().as_ref())
        );

        let login = runtime()
            .block_on(reloaded.auth.login(LoginRequest {
                client_app_id: "octopus-desktop".into(),
                username: "owner".into(),
                password: "password123".into(),
                workspace_id: Some("ws-local".into()),
            }))
            .expect("login after move");
        assert_eq!(login.session.user_id, workspace.owner_user_id.expect("owner user id"));
    }

    fn insert_artifact_record(
        connection: &Connection,
        id: &str,
        project_id: &str,
        title: &str,
        updated_at: u64,
    ) {
        connection
            .execute(
                "INSERT INTO artifact_records (
                    id, workspace_id, project_id, conversation_id, session_id, run_id,
                    source_message_id, parent_artifact_id, title, status, preview_kind,
                    latest_version, promotion_state, promotion_knowledge_id, updated_at,
                    storage_path, content_hash, byte_size, content_type
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6,
                    NULL, NULL, ?7, 'ready', 'markdown',
                    1, 'not-promoted', NULL, ?8,
                    NULL, NULL, NULL, 'text/markdown'
                )",
                rusqlite::params![
                    id,
                    DEFAULT_WORKSPACE_ID,
                    project_id,
                    format!("conv-{id}"),
                    format!("session-{id}"),
                    format!("run-{id}"),
                    title,
                    updated_at as i64,
                ],
            )
            .expect("insert artifact record");
    }

    #[test]
    fn create_agent_persists_runtime_policy_fields_across_db_reload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = runtime()
            .block_on(bundle.workspace.create_agent(UpsertAgentInput {
                workspace_id: DEFAULT_WORKSPACE_ID.into(),
                project_id: None,
                scope: "workspace".into(),
                name: "Research Analyst".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Structured and evidence-driven".into(),
                tags: vec!["research".into(), "docs".into()],
                prompt: "Investigate sources and produce a concise brief.".into(),
                builtin_tool_keys: vec!["bash".into(), "read_file".into()],
                skill_ids: vec!["skill-research".into()],
                mcp_server_names: vec!["browser".into()],
                description: "Produces research briefs and source syntheses.".into(),
                status: "active".into(),
                task_domains: vec!["research".into(), "docs".into()],
                default_model_strategy: Some(DefaultModelStrategy {
                    selection_mode: "actor-default".into(),
                    preferred_model_ref: Some("claude-sonnet-4-5".into()),
                    fallback_model_refs: vec!["gpt-4o".into()],
                    allow_turn_override: false,
                }),
                capability_policy: Some(CapabilityPolicy {
                    mode: "allow-list".into(),
                    deny_by_default: true,
                    builtin_tool_keys: vec!["bash".into(), "read_file".into()],
                    skill_ids: vec!["skill-research".into()],
                    mcp_server_names: vec!["browser".into()],
                    plugin_capability_refs: vec!["plugin.browser.capture".into()],
                }),
                permission_envelope: Some(PermissionEnvelope {
                    default_mode: "readonly".into(),
                    max_mode: "workspace-write".into(),
                    escalation_allowed: true,
                    allowed_resource_scopes: vec!["project-shared".into(), "team-shared".into()],
                }),
                memory_policy: Some(MemoryPolicy {
                    durable_scopes: vec!["user-private".into(), "project-shared".into()],
                    write_requires_approval: true,
                    allow_workspace_shared_write: false,
                    max_selections: 4,
                    freshness_required: true,
                }),
                delegation_policy: Some(DelegationPolicy {
                    mode: "single-worker".into(),
                    allow_background_runs: true,
                    allow_parallel_workers: false,
                    max_worker_count: 1,
                }),
                approval_preference: Some(ApprovalPreference {
                    tool_execution: "require-approval".into(),
                    memory_write: "require-approval".into(),
                    mcp_auth: "require-approval".into(),
                    team_spawn: "deny".into(),
                    workflow_escalation: "require-approval".into(),
                }),
                output_contract: Some(OutputContract {
                    primary_format: "markdown".into(),
                    artifact_kinds: vec!["report".into(), "trace".into()],
                    require_structured_summary: true,
                    preserve_lineage: true,
                }),
                shared_capability_policy: Some(SharedCapabilityPolicy {
                    allow_team_inherited_capabilities: false,
                    deny_direct_member_escalation: true,
                    shared_capability_refs: vec!["skill://docs/review".into()],
                }),
            }))
            .expect("create agent");

        let connection = bundle.workspace.state.open_db().expect("open db");
        let reloaded = load_agents(&connection)
            .expect("load agents")
            .into_iter()
            .find(|agent| agent.id == created.id)
            .expect("reloaded agent");

        assert_eq!(reloaded.task_domains, vec!["research", "docs"]);
        assert_eq!(
            reloaded.default_model_strategy,
            DefaultModelStrategy {
                selection_mode: "actor-default".into(),
                preferred_model_ref: Some("claude-sonnet-4-5".into()),
                fallback_model_refs: vec!["gpt-4o".into()],
                allow_turn_override: false,
            }
        );
        assert_eq!(
            reloaded.capability_policy.plugin_capability_refs,
            vec!["plugin.browser.capture"]
        );
        assert_eq!(reloaded.permission_envelope.default_mode, "readonly");
        assert_eq!(reloaded.memory_policy.max_selections, 4);
        assert_eq!(reloaded.delegation_policy.mode, "single-worker");
        assert_eq!(reloaded.approval_preference.team_spawn, "deny");
        assert_eq!(
            reloaded.output_contract.artifact_kinds,
            vec!["report", "trace"]
        );
        assert_eq!(
            reloaded.shared_capability_policy.shared_capability_refs,
            vec!["skill://docs/review"]
        );
        assert_eq!(reloaded.manifest_revision, "asset-manifest/v2");
        assert_eq!(reloaded.import_metadata.origin_kind, "native");
        assert_eq!(reloaded.import_metadata.translation_status, "native");
        assert_eq!(reloaded.trust_metadata.trust_level, "trusted");
        assert!(reloaded.dependency_resolution.is_empty());
    }

    #[test]
    fn list_project_deliverables_returns_only_requested_project_in_updated_order() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let connection = bundle.workspace.state.open_db().expect("open db");

        insert_artifact_record(&connection, "artifact-a-older", "proj-a", "A older", 100);
        insert_artifact_record(&connection, "artifact-b-newest", "proj-b", "B newest", 500);
        insert_artifact_record(&connection, "artifact-a-newest", "proj-a", "A newest", 400);
        insert_artifact_record(&connection, "artifact-a-middle", "proj-a", "A middle", 200);

        let records = runtime()
            .block_on(bundle.workspace.list_project_deliverables("proj-a"))
            .expect("list project deliverables");

        let ids = records
            .iter()
            .map(|record| record.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            ids,
            vec!["artifact-a-newest", "artifact-a-middle", "artifact-a-older"]
        );
        assert!(records.iter().all(|record| record.project_id == "proj-a"));
    }

    #[test]
    fn personal_pet_snapshots_and_bindings_are_scoped_by_owner_user() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let owner_session = bootstrap_admin_session(&bundle);
        let analyst_session = create_user_session(&bundle, "analyst", "Analyst");

        let owner_snapshot = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&owner_session.user_id),
            )
            .expect("owner pet snapshot");
        let analyst_snapshot = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&analyst_session.user_id),
            )
            .expect("analyst pet snapshot");

        assert_eq!(owner_snapshot.owner_user_id, owner_session.user_id);
        assert_eq!(owner_snapshot.context_scope, "home");
        assert_eq!(analyst_snapshot.owner_user_id, analyst_session.user_id);
        assert_ne!(owner_snapshot.profile.id, analyst_snapshot.profile.id);

        let owner_binding = runtime()
            .block_on(bundle.workspace.bind_workspace_pet_conversation(
                &owner_session.user_id,
                BindPetConversationInput {
                    pet_id: owner_snapshot.profile.id.clone(),
                    conversation_id: "conversation-owner".into(),
                    session_id: Some("session-owner".into()),
                },
            ))
            .expect("bind owner pet");

        let refreshed_owner = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&owner_session.user_id),
            )
            .expect("refreshed owner pet");
        let refreshed_analyst = runtime()
            .block_on(
                bundle
                    .workspace
                    .get_workspace_pet_snapshot(&analyst_session.user_id),
            )
            .expect("refreshed analyst pet");

        assert_eq!(
            refreshed_owner
                .binding
                .as_ref()
                .map(|binding| binding.conversation_id.as_str()),
            Some("conversation-owner")
        );
        assert_eq!(
            refreshed_owner
                .binding
                .as_ref()
                .map(|binding| binding.owner_user_id.as_str()),
            Some(owner_session.user_id.as_str())
        );
        assert_eq!(owner_binding.context_scope, "home");
        assert!(refreshed_analyst.binding.is_none());
    }

    #[test]
    fn generic_agent_listing_excludes_personal_pet_agents() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let owner_session = bootstrap_admin_session(&bundle);
        let analyst_session = create_user_session(&bundle, "analyst", "Analyst");

        let agents = runtime()
            .block_on(bundle.workspace.list_agents())
            .expect("list agents");

        assert!(
            agents.iter().all(|record| record.asset_role != "pet"),
            "pet agents must be hidden from the generic catalog"
        );
        assert!(bundle
            .workspace
            .state
            .agents
            .lock()
            .expect("agents")
            .iter()
            .any(|record| {
                record.asset_role == "pet"
                    && record.owner_user_id.as_deref() == Some(owner_session.user_id.as_str())
            }));
        assert!(bundle
            .workspace
            .state
            .agents
            .lock()
            .expect("agents")
            .iter()
            .any(|record| {
                record.asset_role == "pet"
                    && record.owner_user_id.as_deref() == Some(analyst_session.user_id.as_str())
            }));
    }

    #[test]
    fn create_team_persists_topology_and_workflow_policy_fields_across_db_reload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = runtime()
            .block_on(bundle.workspace.create_team(UpsertTeamInput {
                workspace_id: DEFAULT_WORKSPACE_ID.into(),
                project_id: None,
                scope: "workspace".into(),
                name: "Research Ops Team".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Leader-coordinated specialists".into(),
                tags: vec!["research".into(), "browser".into()],
                prompt: "Break research work into specialist subruns.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: vec!["skill-research".into(), "skill-synthesis".into()],
                mcp_server_names: vec!["browser".into(), "notion".into()],
                description: "Coordinates research and browsing specialists.".into(),
                status: "active".into(),
                task_domains: vec!["research".into(), "browser".into()],
                default_model_strategy: Some(DefaultModelStrategy {
                    selection_mode: "session-selected".into(),
                    preferred_model_ref: Some("claude-sonnet-4-5".into()),
                    fallback_model_refs: vec!["gpt-4o".into()],
                    allow_turn_override: true,
                }),
                capability_policy: Some(CapabilityPolicy {
                    mode: "allow-list".into(),
                    deny_by_default: true,
                    builtin_tool_keys: vec!["bash".into()],
                    skill_ids: vec!["skill-research".into(), "skill-synthesis".into()],
                    mcp_server_names: vec!["browser".into(), "notion".into()],
                    plugin_capability_refs: vec!["plugin.browser.capture".into()],
                }),
                permission_envelope: Some(PermissionEnvelope {
                    default_mode: "workspace-write".into(),
                    max_mode: "danger-full-access".into(),
                    escalation_allowed: true,
                    allowed_resource_scopes: vec!["team-shared".into(), "project-shared".into()],
                }),
                memory_policy: Some(MemoryPolicy {
                    durable_scopes: vec!["team-shared".into(), "project-shared".into()],
                    write_requires_approval: true,
                    allow_workspace_shared_write: false,
                    max_selections: 6,
                    freshness_required: true,
                }),
                delegation_policy: Some(DelegationPolicy {
                    mode: "leader-orchestrated".into(),
                    allow_background_runs: true,
                    allow_parallel_workers: true,
                    max_worker_count: 3,
                }),
                approval_preference: Some(ApprovalPreference {
                    tool_execution: "require-approval".into(),
                    memory_write: "require-approval".into(),
                    mcp_auth: "require-approval".into(),
                    team_spawn: "require-approval".into(),
                    workflow_escalation: "require-approval".into(),
                }),
                output_contract: Some(OutputContract {
                    primary_format: "markdown".into(),
                    artifact_kinds: vec!["brief".into(), "artifact".into()],
                    require_structured_summary: true,
                    preserve_lineage: true,
                }),
                shared_capability_policy: Some(SharedCapabilityPolicy {
                    allow_team_inherited_capabilities: true,
                    deny_direct_member_escalation: true,
                    shared_capability_refs: vec!["skill://research/common".into()],
                }),
                leader_ref: "agent://workspace/lead".into(),
                member_refs: vec![
                    "agent://workspace/research".into(),
                    "agent://workspace/browser".into(),
                ],
                team_topology: Some(TeamTopology {
                    mode: "leader-orchestrated".into(),
                    leader_ref: "agent://workspace/lead".into(),
                    member_refs: vec![
                        "agent://workspace/research".into(),
                        "agent://workspace/browser".into(),
                    ],
                }),
                shared_memory_policy: Some(SharedMemoryPolicy {
                    share_mode: "team-shared".into(),
                    writable_by_workers: true,
                    require_review_before_persist: true,
                }),
                mailbox_policy: Some(MailboxPolicy {
                    mode: "leader-hub".into(),
                    allow_worker_to_worker: false,
                    retain_messages: true,
                }),
                artifact_handoff_policy: Some(ArtifactHandoffPolicy {
                    mode: "leader-reviewed".into(),
                    require_lineage: true,
                    retain_artifacts: true,
                }),
                workflow_affordance: Some(WorkflowAffordance {
                    supported_task_kinds: vec!["research".into(), "browser".into()],
                    background_capable: true,
                    automation_capable: true,
                }),
                worker_concurrency_limit: Some(3),
            }))
            .expect("create team");

        let connection = bundle.workspace.state.open_db().expect("open db");
        let reloaded = load_teams(&connection)
            .expect("load teams")
            .into_iter()
            .find(|team| team.id == created.id)
            .expect("reloaded team");

        assert_eq!(reloaded.leader_ref, "agent://workspace/lead");
        assert_eq!(
            reloaded.member_refs,
            vec!["agent://workspace/research", "agent://workspace/browser"]
        );
        assert_eq!(reloaded.team_topology.mode, "leader-orchestrated");
        assert_eq!(reloaded.shared_memory_policy.share_mode, "team-shared");
        assert_eq!(reloaded.mailbox_policy.mode, "leader-hub");
        assert_eq!(reloaded.artifact_handoff_policy.mode, "leader-reviewed");
        assert_eq!(
            reloaded.workflow_affordance.supported_task_kinds,
            vec!["research", "browser"]
        );
        assert_eq!(reloaded.worker_concurrency_limit, 3);
        assert_eq!(reloaded.delegation_policy.max_worker_count, 3);
        assert_eq!(reloaded.trust_metadata.trust_level, "trusted");
        assert_eq!(reloaded.import_metadata.origin_kind, "native");
    }

    #[test]
    fn create_team_rejects_missing_leader_ref() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let error = runtime()
            .block_on(bundle.workspace.create_team(UpsertTeamInput {
                workspace_id: DEFAULT_WORKSPACE_ID.into(),
                project_id: None,
                scope: "workspace".into(),
                name: "Invalid Team".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Missing leader ref".into(),
                tags: vec!["research".into()],
                prompt: "This should fail closed.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: vec!["research".into()],
                default_model_strategy: None,
                capability_policy: None,
                permission_envelope: None,
                memory_policy: None,
                delegation_policy: None,
                approval_preference: None,
                output_contract: None,
                shared_capability_policy: None,
                leader_ref: String::new(),
                member_refs: Vec::new(),
                team_topology: None,
                shared_memory_policy: None,
                mailbox_policy: None,
                artifact_handoff_policy: None,
                workflow_affordance: None,
                worker_concurrency_limit: None,
                description: "Legacy-only team input".into(),
                status: "active".into(),
            }))
            .expect_err("missing leader_ref should fail");

        assert!(error.to_string().contains("leader_ref"));
    }

    #[test]
    fn project_resource_directory_persists_on_create() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Resource Project".into(),
                description: "Resource directory persistence.".into(),
                resource_directory: "data/projects/resource-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        assert_eq!(
            created.resource_directory,
            "data/projects/resource-project/resources"
        );
        assert!(bundle
            .paths
            .root
            .join("data/projects/resource-project/resources")
            .exists());
    }

    #[test]
    fn create_project_persists_manager_and_preset_fields_without_legacy_grant_snapshots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Governed Project".into(),
                description: "Project metadata persistence coverage.".into(),
                resource_directory: "data/projects/governed-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: Some("agent-leader".into()),
                manager_user_id: Some("user-manager".into()),
                preset_code: Some("preset-governed".into()),
                assignments: None,
            }))
            .expect("created project");

        assert_eq!(created.leader_agent_id.as_deref(), Some("agent-leader"));
        assert_eq!(created.manager_user_id.as_deref(), Some("user-manager"));
        assert_eq!(created.preset_code.as_deref(), Some("preset-governed"));
        assert_eq!(
            created.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(created.assignments.is_none());

        let listed = runtime
            .block_on(bundle.workspace.list_projects())
            .expect("listed projects");
        let persisted = listed
            .iter()
            .find(|project| project.id == created.id)
            .expect("persisted project");

        assert_eq!(persisted.leader_agent_id.as_deref(), Some("agent-leader"));
        assert_eq!(persisted.manager_user_id.as_deref(), Some("user-manager"));
        assert_eq!(persisted.preset_code.as_deref(), Some("preset-governed"));
        assert_eq!(
            persisted.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(persisted.assignments.is_none());
    }

    #[test]
    fn update_project_rewrites_manager_and_preset_fields_without_legacy_grant_snapshots() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Rewrite Governed Project".into(),
                description: "Rewrite project metadata persistence coverage.".into(),
                resource_directory: "data/projects/rewrite-governed-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: Some("agent-alpha".into()),
                manager_user_id: Some("user-manager-alpha".into()),
                preset_code: Some("preset-alpha".into()),
                assignments: None,
            }))
            .expect("created project");

        let updated = runtime
            .block_on(bundle.workspace.update_project(
                &created.id,
                UpdateProjectRequest {
                    name: "Rewrite Governed Project".into(),
                    description: "Rewrite project metadata persistence updated.".into(),
                    status: "active".into(),
                    resource_directory: created.resource_directory.clone(),
                    owner_user_id: None,
                    member_user_ids: None,
                    permission_overrides: None,
                    linked_workspace_assets: None,
                    leader_agent_id: Some("agent-beta".into()),
                    manager_user_id: Some("user-manager-beta".into()),
                    preset_code: Some("preset-beta".into()),
                    assignments: None,
                },
            ))
            .expect("updated project");

        assert_eq!(updated.leader_agent_id.as_deref(), Some("agent-beta"));
        assert_eq!(
            updated.manager_user_id.as_deref(),
            Some("user-manager-beta")
        );
        assert_eq!(updated.preset_code.as_deref(), Some("preset-beta"));
        assert_eq!(
            updated.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(updated.assignments.is_none());

        let persisted = runtime
            .block_on(bundle.workspace.list_projects())
            .expect("listed projects")
            .into_iter()
            .find(|project| project.id == created.id)
            .expect("persisted project");
        assert_eq!(persisted.leader_agent_id.as_deref(), Some("agent-beta"));
        assert_eq!(
            persisted.manager_user_id.as_deref(),
            Some("user-manager-beta")
        );
        assert_eq!(persisted.preset_code.as_deref(), Some("preset-beta"));
        assert_eq!(
            persisted.linked_workspace_assets,
            empty_project_linked_workspace_assets()
        );
        assert!(persisted.assignments.is_none());

        let connection = bundle.workspace.state.open_db().expect("open db");
        let (
            project_count,
            stored_leader_agent_id,
            stored_manager_user_id,
            stored_preset_code,
            assignments_json,
            linked_workspace_assets_json,
        ): (
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "SELECT COUNT(*), leader_agent_id, manager_user_id, preset_code, assignments_json, linked_workspace_assets_json
                 FROM projects
                 WHERE id = ?1
                 GROUP BY leader_agent_id, manager_user_id, preset_code, assignments_json, linked_workspace_assets_json",
                params![created.id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                    ))
                },
            )
            .expect("load persisted project row");
        assert_eq!(project_count, 1);
        assert_eq!(stored_leader_agent_id.as_deref(), Some("agent-beta"));
        assert_eq!(stored_manager_user_id.as_deref(), Some("user-manager-beta"));
        assert_eq!(stored_preset_code.as_deref(), Some("preset-beta"));
        assert!(assignments_json.is_none());
        assert!(linked_workspace_assets_json.is_none());
    }

    #[test]
    fn project_deletion_requests_round_trip_and_gate_delete_until_approved() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let owner_session = bootstrap_admin_session(&bundle);

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Delete Gate Project".into(),
                description: "Project deletion request coverage.".into(),
                resource_directory: "data/projects/delete-gate-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        let error = runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect_err("active project delete should fail");
        assert!(error.to_string().contains("archived"));

        let archived = runtime
            .block_on(bundle.workspace.update_project(
                &created.id,
                UpdateProjectRequest {
                    name: created.name.clone(),
                    description: created.description.clone(),
                    status: "archived".into(),
                    resource_directory: created.resource_directory.clone(),
                    owner_user_id: Some(created.owner_user_id.clone()),
                    member_user_ids: Some(created.member_user_ids.clone()),
                    permission_overrides: Some(created.permission_overrides.clone()),
                    linked_workspace_assets: Some(created.linked_workspace_assets.clone()),
                    leader_agent_id: created.leader_agent_id.clone(),
                    manager_user_id: created.manager_user_id.clone(),
                    preset_code: created.preset_code.clone(),
                    assignments: created.assignments.clone(),
                },
            ))
            .expect("archived project");
        assert_eq!(archived.status, "archived");

        let error = runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect_err("archived project without request should fail");
        assert!(error.to_string().contains("approved"));

        let pending = runtime
            .block_on(bundle.workspace.create_project_deletion_request(
                &created.id,
                &owner_session.user_id,
                CreateProjectDeletionRequestInput {
                    reason: Some("Archive complete".into()),
                },
            ))
            .expect("created deletion request");
        assert_eq!(pending.status, "pending");
        assert_eq!(pending.reason.as_deref(), Some("Archive complete"));

        let listed = runtime
            .block_on(bundle.workspace.list_project_deletion_requests(&created.id))
            .expect("listed deletion requests");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, pending.id);

        let error = runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect_err("pending request should not unlock delete");
        assert!(error.to_string().contains("approved"));

        let approved = runtime
            .block_on(bundle.workspace.review_project_deletion_request(
                &pending.id,
                &owner_session.user_id,
                true,
                ReviewProjectDeletionRequestInput {
                    review_comment: Some("Approved for deletion".into()),
                },
            ))
            .expect("approved deletion request");
        assert_eq!(approved.status, "approved");
        assert_eq!(
            approved.reviewed_by_user_id.as_deref(),
            Some(owner_session.user_id.as_str())
        );
        assert_eq!(
            approved.review_comment.as_deref(),
            Some("Approved for deletion")
        );
        assert!(approved.reviewed_at.is_some());

        runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect("deleted project");

        let projects = runtime
            .block_on(bundle.workspace.list_projects())
            .expect("listed projects");
        assert!(!projects.iter().any(|project| project.id == created.id));

        let connection = bundle.workspace.state.open_db().expect("open db");
        let remaining_requests: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM project_deletion_requests WHERE project_id = ?1",
                params![created.id],
                |row| row.get(0),
            )
            .expect("count project deletion requests");
        assert_eq!(remaining_requests, 0);
    }

    #[test]
    fn project_delete_removes_managed_resource_storage_and_project_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let owner_session = bootstrap_admin_session(&bundle);

        let created = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Delete Storage Project".into(),
                description: "Project delete cleanup coverage.".into(),
                resource_directory: "data/projects/delete-storage-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        let imported = runtime
            .block_on(bundle.workspace.import_project_resource(
                &created.id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "cleanup-folder".into(),
                    root_dir_name: Some("cleanup-folder".into()),
                    scope: "project".into(),
                    visibility: "private".into(),
                    tags: Some(vec!["cleanup".into()]),
                    files: vec![
                        encoded_file("notes/todo.md", "text/markdown", "# Cleanup"),
                        encoded_file("payload.json", "application/json", "{\"ok\":true}"),
                    ],
                },
            ))
            .expect("imported project resource");

        let project_root = bundle.paths.root.join(&created.resource_directory);
        let storage_path = imported.storage_path.clone().expect("storage path");
        let absolute_storage_path = bundle.paths.root.join(&storage_path);
        assert!(project_root.exists());
        assert!(absolute_storage_path.exists());

        runtime
            .block_on(bundle.workspace.update_project(
                &created.id,
                UpdateProjectRequest {
                    name: created.name.clone(),
                    description: created.description.clone(),
                    status: "archived".into(),
                    resource_directory: created.resource_directory.clone(),
                    owner_user_id: Some(created.owner_user_id.clone()),
                    member_user_ids: Some(created.member_user_ids.clone()),
                    permission_overrides: Some(created.permission_overrides.clone()),
                    linked_workspace_assets: Some(created.linked_workspace_assets.clone()),
                    leader_agent_id: created.leader_agent_id.clone(),
                    manager_user_id: created.manager_user_id.clone(),
                    preset_code: created.preset_code.clone(),
                    assignments: created.assignments.clone(),
                },
            ))
            .expect("archived project");
        let deletion_request = runtime
            .block_on(bundle.workspace.create_project_deletion_request(
                &created.id,
                &owner_session.user_id,
                CreateProjectDeletionRequestInput {
                    reason: Some("Cleanup all files".into()),
                },
            ))
            .expect("created deletion request");
        runtime
            .block_on(bundle.workspace.review_project_deletion_request(
                &deletion_request.id,
                &owner_session.user_id,
                true,
                ReviewProjectDeletionRequestInput {
                    review_comment: Some("Approved cleanup".into()),
                },
            ))
            .expect("approved deletion request");

        runtime
            .block_on(bundle.workspace.delete_project(&created.id))
            .expect("deleted project");

        assert!(!project_root.exists());
        assert!(!absolute_storage_path.exists());

        let connection = bundle.workspace.state.open_db().expect("open db");
        let remaining_resources: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM resources WHERE project_id = ?1",
                params![created.id],
                |row| row.get(0),
            )
            .expect("count project resources");
        let remaining_projects: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM projects WHERE id = ?1",
                params![created.id],
                |row| row.get(0),
            )
            .expect("count projects");
        assert_eq!(remaining_resources, 0);
        assert_eq!(remaining_projects, 0);
    }

    #[test]
    fn project_owned_agents_and_teams_can_be_promoted_to_workspace() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");

        let project = runtime
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Promotion Agents".into(),
                description: "Agent/team promotion coverage.".into(),
                resource_directory: "data/projects/promotion-agents/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        let project_agent = runtime
            .block_on(bundle.workspace.create_agent(UpsertAgentInput {
                workspace_id: project.workspace_id.clone(),
                project_id: Some(project.id.clone()),
                scope: "project".into(),
                name: "Promotion Analyst".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Project-only analyst".into(),
                tags: vec!["promotion".into()],
                prompt: "Promote this agent into the workspace.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: vec!["promotion".into()],
                default_model_strategy: None,
                capability_policy: None,
                permission_envelope: None,
                memory_policy: None,
                delegation_policy: None,
                approval_preference: None,
                output_contract: None,
                shared_capability_policy: None,
                description: "Project-owned agent".into(),
                status: "active".into(),
            }))
            .expect("created project agent");

        let project_team = runtime
            .block_on(bundle.workspace.create_team(UpsertTeamInput {
                workspace_id: project.workspace_id.clone(),
                project_id: Some(project.id.clone()),
                scope: "project".into(),
                name: "Promotion Strike Team".into(),
                avatar: None,
                remove_avatar: None,
                personality: "Project-only team".into(),
                tags: vec!["promotion".into()],
                prompt: "Promote this team into the workspace.".into(),
                builtin_tool_keys: vec!["bash".into()],
                skill_ids: Vec::new(),
                mcp_server_names: Vec::new(),
                task_domains: vec!["promotion".into()],
                default_model_strategy: None,
                capability_policy: None,
                permission_envelope: None,
                memory_policy: None,
                delegation_policy: None,
                approval_preference: None,
                output_contract: None,
                shared_capability_policy: None,
                leader_ref: crate::canonical_agent_ref(&project_agent.id),
                member_refs: vec![crate::canonical_agent_ref(&project_agent.id)],
                team_topology: None,
                shared_memory_policy: None,
                mailbox_policy: None,
                artifact_handoff_policy: None,
                workflow_affordance: None,
                worker_concurrency_limit: None,
                description: "Project-owned team".into(),
                status: "active".into(),
            }))
            .expect("created project team");

        let promoted_agent = runtime
            .block_on(
                bundle
                    .workspace
                    .copy_workspace_agent_from_builtin(&project_agent.id),
            )
            .expect("promoted project agent");
        assert_eq!(promoted_agent.failure_count, 0);
        assert_eq!(promoted_agent.agent_count, 1);

        let promoted_team = runtime
            .block_on(
                bundle
                    .workspace
                    .copy_workspace_team_from_builtin(&project_team.id),
            )
            .expect("promoted project team");
        assert_eq!(promoted_team.failure_count, 0);
        assert_eq!(promoted_team.team_count, 1);
        assert_eq!(promoted_team.agent_count, 1);

        let agents = runtime
            .block_on(bundle.workspace.list_agents())
            .expect("list agents");
        assert!(agents.iter().any(|agent| agent.id == project_agent.id
            && agent.project_id.as_deref() == Some(project.id.as_str())));
        assert!(agents
            .iter()
            .any(|agent| agent.name == "Promotion Analyst" && agent.project_id.is_none()));

        let teams = runtime
            .block_on(bundle.workspace.list_teams())
            .expect("list teams");
        assert!(teams.iter().any(|team| team.id == project_team.id
            && team.project_id.as_deref() == Some(project.id.as_str())));
        assert!(teams
            .iter()
            .any(|team| team.name == "Promotion Strike Team" && team.project_id.is_none()));
    }

    #[test]
    fn import_folder_creates_single_record_and_delete_removes_managed_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Import Project".into(),
                description: "Resource import coverage.".into(),
                resource_directory: "data/projects/import-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        let imported = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.import_project_resource(
                &created.id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "design-assets".into(),
                    root_dir_name: Some("design-assets".into()),
                    scope: "project".into(),
                    visibility: "public".into(),
                    tags: Some(vec!["assets".into()]),
                    files: vec![
                        encoded_file("brief.md", "text/markdown", "# Brief"),
                        encoded_file("nested/spec.json", "application/json", "{\"ok\":true}"),
                    ],
                },
            ))
            .expect("imported folder");

        assert_eq!(imported.kind, "folder");
        assert_eq!(imported.scope, "project");
        assert_eq!(imported.visibility, "public");

        let listed = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.list_project_resources(&created.id))
            .expect("listed resources");
        assert_eq!(listed.len(), 1);

        let children = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.list_resource_children(&imported.id))
            .expect("children");
        assert_eq!(children.len(), 2);
        assert!(children
            .iter()
            .any(|entry| entry.relative_path == "nested/spec.json"));

        let promoted = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "workspace".into(),
                },
            ))
            .expect("promoted");
        assert_eq!(promoted.scope, "workspace");

        let storage_path = imported.storage_path.expect("storage path");
        let absolute_storage_path = bundle.paths.root.join(&storage_path);
        assert!(absolute_storage_path.exists());

        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(
                bundle
                    .workspace
                    .delete_project_resource(&created.id, &imported.id),
            )
            .expect("deleted");

        assert!(!absolute_storage_path.exists());
    }

    #[test]
    fn workspace_import_writes_into_workspace_resources_and_supports_content_and_directory_browsing(
    ) {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let workspace_id = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.workspace_summary())
            .expect("workspace summary")
            .id;

        let imported = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.import_workspace_resource(
                &workspace_id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "workspace-handbook.md".into(),
                    root_dir_name: None,
                    scope: "workspace".into(),
                    visibility: "public".into(),
                    tags: Some(vec!["docs".into()]),
                    files: vec![encoded_file(
                        "workspace-handbook.md",
                        "text/markdown",
                        "# Workspace Handbook",
                    )],
                },
            ))
            .expect("imported workspace resource");

        let storage_path = imported.storage_path.clone().expect("storage path");
        assert!(storage_path.starts_with("data/resources/workspace/workspace-handbook"));
        assert!(storage_path.ends_with(".md"));
        assert!(bundle.paths.root.join(&storage_path).exists());

        let content = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.get_resource_content(&imported.id))
            .expect("resource content");
        assert_eq!(content.preview_kind, "markdown");
        assert_eq!(
            content.text_content.as_deref(),
            Some("# Workspace Handbook")
        );
        assert!(content.data_base64.is_none());

        let directories = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.list_directories(Some("data/resources")))
            .expect("directories");
        assert_eq!(directories.current_path, "data/resources");
        assert!(directories
            .entries
            .iter()
            .any(|entry| entry.path == "data/resources/workspace"));
    }

    #[test]
    fn project_personal_resources_follow_the_promotion_chain() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("infra bundle");

        let created = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.create_project(CreateProjectRequest {
                name: "Promotion Project".into(),
                description: "Promotion coverage.".into(),
                resource_directory: "data/projects/promotion-project/resources".into(),
                owner_user_id: None,
                member_user_ids: None,
                permission_overrides: None,
                linked_workspace_assets: None,
                leader_agent_id: None,
                manager_user_id: None,
                preset_code: None,
                assignments: None,
            }))
            .expect("created project");

        let imported = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.import_project_resource(
                &created.id,
                "user-owner",
                octopus_core::WorkspaceResourceImportInput {
                    name: "private-notes.md".into(),
                    root_dir_name: None,
                    scope: "personal".into(),
                    visibility: "private".into(),
                    tags: Some(vec!["notes".into()]),
                    files: vec![encoded_file(
                        "private-notes.md",
                        "text/markdown",
                        "# Private Notes",
                    )],
                },
            ))
            .expect("imported personal resource");

        assert_eq!(imported.scope, "personal");
        assert_eq!(imported.visibility, "private");

        let invalid_direct_promotion = tokio::runtime::Runtime::new().expect("runtime").block_on(
            bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "workspace".into(),
                },
            ),
        );
        assert!(invalid_direct_promotion.is_err());

        let promoted_to_project = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "project".into(),
                },
            ))
            .expect("promoted to project");
        assert_eq!(promoted_to_project.scope, "project");
        assert_eq!(promoted_to_project.visibility, "private");

        let promoted_to_workspace = tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(bundle.workspace.promote_resource(
                &imported.id,
                octopus_core::PromoteWorkspaceResourceInput {
                    scope: "workspace".into(),
                },
            ))
            .expect("promoted to workspace");
        assert_eq!(promoted_to_workspace.scope, "workspace");
        assert_eq!(promoted_to_workspace.visibility, "private");
    }
}
