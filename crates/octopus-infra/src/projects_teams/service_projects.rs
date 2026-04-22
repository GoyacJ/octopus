use super::*;

impl InfraWorkspaceService {
    pub(crate) async fn system_bootstrap_impl(&self) -> Result<SystemBootstrapStatus, AppError> {
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

    pub(crate) async fn workspace_summary_impl(&self) -> Result<WorkspaceSummary, AppError> {
        self.state.workspace_snapshot()
    }

    pub(crate) async fn update_workspace_impl(
        &self,
        request: UpdateWorkspaceRequest,
    ) -> Result<WorkspaceSummary, AppError> {
        let current_workspace = self.state.workspace_snapshot()?;
        let current_workspace_root = self.state.paths.root.clone();
        let shell_root = PathBuf::from(workspace_shell_root_display_path(
            &current_workspace,
            &self.state.paths,
        ));
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
            workspace.mapped_directory_default = Some(shell_root.to_string_lossy().to_string());
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
        self.state.workspace_snapshot()
    }

    pub(crate) async fn list_projects_impl(&self) -> Result<Vec<ProjectRecord>, AppError> {
        Ok(self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .clone())
    }

    pub(crate) async fn list_project_deliverables_impl(
        &self,
        project_id: &str,
    ) -> Result<Vec<ArtifactRecord>, AppError> {
        load_project_artifact_records(&self.state.open_db()?, project_id)
    }

    pub(crate) async fn create_project_impl(
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

    pub(crate) async fn update_project_impl(
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

    pub(crate) async fn list_project_promotion_requests_impl(
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

    pub(crate) async fn list_workspace_promotion_requests_impl(
        &self,
    ) -> Result<Vec<ProjectPromotionRequest>, AppError> {
        Ok(self
            .state
            .project_promotion_requests
            .lock()
            .map_err(|_| AppError::runtime("project promotion requests mutex poisoned"))?
            .clone())
    }

    pub(crate) async fn list_project_deletion_requests_impl(
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

    pub(crate) async fn create_project_promotion_request_impl(
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

    pub(crate) async fn create_project_deletion_request_impl(
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

    pub(crate) async fn review_project_promotion_request_impl(
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

    pub(crate) async fn review_project_deletion_request_impl(
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

    pub(crate) async fn delete_project_impl(&self, project_id: &str) -> Result<(), AppError> {
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
}
