use super::*;

impl InfraWorkspaceService {
    pub(crate) fn normalize_resource_status(status: &str) -> Result<String, AppError> {
        let normalized = status.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input("resource status is required"));
        }
        Ok(normalized.into())
    }

    pub(crate) fn project_record(&self, project_id: &str) -> Result<ProjectRecord, AppError> {
        self.state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .iter()
            .find(|project| project.id == project_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("project not found"))
    }

    pub(crate) fn build_project_deletion_request_inbox_items(
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

    pub(crate) fn push_inbox_items(&self, items: Vec<InboxItemRecord>) -> Result<(), AppError> {
        self.state
            .inbox
            .lock()
            .map_err(|_| AppError::runtime("inbox mutex poisoned"))?
            .extend(items);
        Ok(())
    }

    pub(crate) fn resolve_project_deletion_request_inbox_items(
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

    pub(crate) fn resource_record(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .iter()
            .find(|record| record.id == resource_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("resource not found"))
    }

    pub(crate) fn linked_workspace_assets_from_assignments(
        _assignments: Option<&ProjectWorkspaceAssignments>,
    ) -> ProjectLinkedWorkspaceAssets {
        // Live-inheritance assignments now describe project-local deltas and project-owned assets.
        // Workspace links remain an explicit compatibility read model, so do not derive them here.
        empty_project_linked_workspace_assets()
    }

    pub(crate) fn required_workspace_capability_for_project_asset(
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

    pub(crate) fn upsert_resource_cache(
        &self,
        record: WorkspaceResourceRecord,
    ) -> Result<(), AppError> {
        let record_id = record.id.clone();
        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        Self::replace_or_push(&mut resources, record, |item| item.id == record_id);
        Ok(())
    }

    pub(crate) fn delete_resource_record(&self, resource_id: &str) -> Result<(), AppError> {
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

    pub(crate) fn persist_project_record(
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

    pub(crate) fn persist_project_promotion_request(
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

    pub(crate) fn persist_project_deletion_request(
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

    pub(crate) fn persist_resource_record(
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

    pub(crate) fn display_storage_path(&self, path: &Path) -> String {
        let display = display_path(path, &self.state.paths.root);
        if display.is_empty() {
            path.to_string_lossy().replace('\\', "/")
        } else {
            display
        }
    }

    pub(crate) fn resolve_storage_path(&self, stored_path: &str) -> PathBuf {
        let path = PathBuf::from(stored_path);
        if path.is_absolute() {
            path
        } else {
            self.state.paths.root.join(path)
        }
    }

    pub(crate) fn delete_stored_path_if_exists(&self, stored_path: &str) -> Result<(), AppError> {
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

    pub(crate) fn query_project_cleanup_paths(
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

    pub(crate) fn refresh_project_scoped_caches(
        &self,
        connection: &Connection,
    ) -> Result<(), AppError> {
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
}
