use super::*;

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

    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError> {
        Ok(self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .clone())
    }

    async fn create_project(
        &self,
        request: CreateProjectRequest,
    ) -> Result<ProjectRecord, AppError> {
        let record = ProjectRecord {
            id: format!("proj-{}", Uuid::new_v4()),
            workspace_id: self.state.workspace_id()?,
            name: Self::normalize_project_name(&request.name)?,
            status: "active".into(),
            description: Self::normalize_project_description(&request.description),
            assignments: request.assignments,
        };
        let assignments_json = record
            .assignments
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        self.state.open_db()?.execute(
            "INSERT INTO projects (id, workspace_id, name, status, description, assignments_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                record.id,
                record.workspace_id,
                record.name,
                record.status,
                record.description,
                assignments_json,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

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

        let assignments_json = updated
            .assignments
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?;

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO projects (id, workspace_id, name, status, description, assignments_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                updated.id,
                updated.workspace_id,
                updated.name,
                updated.status,
                updated.description,
                assignments_json,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

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
                bootstrap::save_workspace_config_file(
                    &self.state.paths.workspace_config,
                    &workspace,
                )?;
            }
        }

        Ok(updated)
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
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let record = WorkspaceResourceRecord {
            id: format!("res-{}", Uuid::new_v4()),
            workspace_id: workspace_id.to_string(),
            project_id: input.project_id,
            kind: input.kind,
            name: input.name,
            location: input.location,
            origin: "source".to_string(),
            status: "healthy".to_string(),
            updated_at: timestamp_now(),
            tags: input.tags,
            source_artifact_id: input.source_artifact_id,
        };

        let conn = self.state.open_db()?;
        conn.execute(
            "INSERT INTO resources (id, workspace_id, project_id, kind, name, location, origin, status, updated_at, tags, source_artifact_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.kind,
                record.name,
                record.location,
                record.origin,
                record.status,
                record.updated_at as i64,
                serde_json::to_string(&record.tags)?,
                record.source_artifact_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        resources.push(record.clone());
        Ok(record)
    }

    async fn create_project_resource(
        &self,
        project_id: &str,
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
        self.create_workspace_resource(&workspace_id, input).await
    }

    async fn create_project_resource_folder(
        &self,
        project_id: &str,
        input: CreateWorkspaceResourceFolderInput,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        let mut results = Vec::new();
        for entry in input.files {
            let folder_path = std::path::Path::new(&entry.relative_path);
            let name = folder_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&entry.file_name)
                .to_string();
            let location = folder_path
                .parent()
                .map(|p| p.to_string_lossy().to_string());

            let file_input = CreateWorkspaceResourceInput {
                project_id: Some(project_id.to_string()),
                kind: if entry.relative_path.ends_with('/') || entry.byte_size == 0 {
                    "folder".to_string()
                } else {
                    "file".to_string()
                },
                name,
                location,
                tags: vec![],
                source_artifact_id: None,
            };

            let record = self.create_project_resource(project_id, file_input).await?;
            results.push(record);
        }
        Ok(results)
    }

    async fn delete_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let conn = self.state.open_db()?;
        let affected = conn
            .execute(
                "DELETE FROM resources WHERE id = ?1 AND workspace_id = ?2",
                params![resource_id, workspace_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if affected == 0 {
            return Err(AppError::not_found(format!(
                "resource {} not found in workspace {}",
                resource_id, workspace_id
            )));
        }

        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        resources.retain(|r| !(r.id == resource_id && r.workspace_id == workspace_id));
        Ok(())
    }

    async fn delete_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let conn = self.state.open_db()?;
        let affected = conn
            .execute(
                "DELETE FROM resources WHERE id = ?1 AND project_id = ?2",
                params![resource_id, project_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if affected == 0 {
            return Err(AppError::not_found(format!(
                "resource {} not found in project {}",
                resource_id, project_id
            )));
        }

        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;
        resources.retain(|r| !(r.id == resource_id && r.project_id.as_deref() == Some(project_id)));
        Ok(())
    }

    async fn update_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;

        let record = resources
            .iter_mut()
            .find(|r| r.id == resource_id && r.workspace_id == workspace_id)
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "resource {} not found in workspace {}",
                    resource_id, workspace_id
                ))
            })?;

        if let Some(name) = input.name {
            record.name = name;
        }
        if let Some(location) = input.location {
            record.location = Some(location);
        }
        if let Some(status) = input.status {
            record.status = status;
        }
        if let Some(tags) = input.tags {
            record.tags = tags;
        }
        record.updated_at = timestamp_now();

        let conn = self.state.open_db()?;
        conn.execute(
            "UPDATE resources SET name = ?1, location = ?2, status = ?3, updated_at = ?4, tags = ?5 WHERE id = ?6 AND workspace_id = ?7",
            params![
                record.name,
                record.location,
                record.status,
                record.updated_at as i64,
                serde_json::to_string(&record.tags)?,
                resource_id,
                workspace_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        Ok(record.clone())
    }

    async fn update_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut resources = self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?;

        let record = resources
            .iter_mut()
            .find(|r| r.id == resource_id && r.project_id.as_deref() == Some(project_id))
            .ok_or_else(|| {
                AppError::not_found(format!(
                    "resource {} not found in project {}",
                    resource_id, project_id
                ))
            })?;

        if let Some(name) = input.name {
            record.name = name;
        }
        if let Some(location) = input.location {
            record.location = Some(location);
        }
        if let Some(status) = input.status {
            record.status = status;
        }
        if let Some(tags) = input.tags {
            record.tags = tags;
        }
        record.updated_at = timestamp_now();

        let conn = self.state.open_db()?;
        conn.execute(
            "UPDATE resources SET name = ?1, location = ?2, status = ?3, updated_at = ?4, tags = ?5 WHERE id = ?6 AND project_id = ?7",
            params![
                record.name,
                record.location,
                record.status,
                record.updated_at as i64,
                serde_json::to_string(&record.tags)?,
                resource_id,
                project_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        Ok(record.clone())
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

    async fn get_workspace_pet_snapshot(&self) -> Result<PetWorkspaceSnapshot, AppError> {
        self.workspace_pet_snapshot()
    }

    async fn get_project_pet_snapshot(
        &self,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.project_pet_snapshot(project_id)
    }

    async fn save_workspace_pet_presence(
        &self,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        let mut presence = self
            .state
            .workspace_pet_presence
            .lock()
            .map_err(|_| AppError::runtime("workspace pet presence mutex poisoned"))?
            .clone();
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
        self.persist_pet_presence(None, &presence)?;
        *self
            .state
            .workspace_pet_presence
            .lock()
            .map_err(|_| AppError::runtime("workspace pet presence mutex poisoned"))? =
            presence.clone();
        Ok(presence)
    }

    async fn save_project_pet_presence(
        &self,
        project_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        self.ensure_project_exists(project_id)?;
        let mut presences = self
            .state
            .project_pet_presences
            .lock()
            .map_err(|_| AppError::runtime("project pet presences mutex poisoned"))?;
        let mut presence = presences
            .iter()
            .find(|(id, _)| id == project_id)
            .map(|(_, presence)| presence.clone())
            .unwrap_or_else(default_workspace_pet_presence);
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
        self.persist_pet_presence(Some(project_id), &presence)?;
        Self::replace_or_push(
            &mut presences,
            (project_id.to_string(), presence.clone()),
            |item| item.0 == project_id,
        );
        Ok(presence)
    }

    async fn bind_workspace_pet_conversation(
        &self,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                "pet-octopus".into()
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            project_id: String::new(),
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(None, &binding)?;
        *self
            .state
            .workspace_pet_binding
            .lock()
            .map_err(|_| AppError::runtime("workspace pet binding mutex poisoned"))? =
            Some(binding.clone());
        Ok(binding)
    }

    async fn bind_project_pet_conversation(
        &self,
        project_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        self.ensure_project_exists(project_id)?;
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                "pet-octopus".into()
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            project_id: project_id.into(),
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(Some(project_id), &binding)?;
        let mut bindings = self
            .state
            .project_pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("project pet bindings mutex poisoned"))?;
        Self::replace_or_push(
            &mut bindings,
            (project_id.to_string(), binding.clone()),
            |item| item.0 == project_id,
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
            .clone();
        agents.extend(crate::agent_assets::list_builtin_agent_templates(
            &workspace_id,
        )?);
        agents.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(agents)
    }

    async fn create_agent(&self, input: UpsertAgentInput) -> Result<AgentRecord, AppError> {
        let agent_id = format!("agent-{}", Uuid::new_v4());
        let record = self.build_agent_record(&agent_id, input, None)?;

        self.state.open_db()?.execute(
            "INSERT INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

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

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO agents (id, workspace_id, project_id, scope, name, avatar_path, personality, tags, prompt, builtin_tool_keys, skill_ids, mcp_server_names, description, status, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.scope,
                record.name,
                record.avatar_path,
                record.personality,
                serde_json::to_string(&record.tags)?,
                record.prompt,
                serde_json::to_string(&record.builtin_tool_keys)?,
                serde_json::to_string(&record.skill_ids)?,
                serde_json::to_string(&record.mcp_server_names)?,
                record.description,
                record.status,
                record.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

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
        agent_assets::preview_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Workspace,
            input,
        )
    }

    async fn import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_assets::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Workspace,
            input,
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    async fn copy_workspace_agent_from_builtin(
        &self,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let files = crate::agent_assets::extract_builtin_agent_template_files(agent_id)?
            .ok_or_else(|| AppError::not_found("builtin agent template"))?;
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_assets::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Workspace,
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    async fn export_agent_bundle(
        &self,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_assets::export_assets(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Workspace,
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
        agent_assets::preview_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Project(project_id),
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
        let result = agent_assets::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Project(project_id),
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
        let files = crate::agent_assets::extract_builtin_agent_template_files(agent_id)?
            .ok_or_else(|| AppError::not_found("builtin agent template"))?;
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_assets::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Project(project_id),
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    async fn export_project_agent_bundle(
        &self,
        project_id: &str,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_assets::export_assets(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Project(project_id),
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
        teams.extend(crate::agent_assets::list_builtin_team_templates(
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
        let files = crate::agent_assets::extract_builtin_team_template_files(team_id)?
            .ok_or_else(|| AppError::not_found("builtin team template"))?;
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_assets::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Workspace,
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    async fn copy_project_team_from_builtin(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let files = crate::agent_assets::extract_builtin_team_template_files(team_id)?
            .ok_or_else(|| AppError::not_found("builtin team template"))?;
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_assets::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_assets::AssetTargetScope::Project(project_id),
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
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

    async fn get_tool_catalog(&self) -> Result<WorkspaceToolCatalogSnapshot, AppError> {
        self.build_tool_catalog().await
    }

    async fn set_tool_catalog_disabled(
        &self,
        patch: WorkspaceToolDisablePatch,
    ) -> Result<WorkspaceToolCatalogSnapshot, AppError> {
        let snapshot = self.build_tool_catalog().await?;
        if !snapshot
            .entries
            .iter()
            .any(|entry| entry.source_key == patch.source_key)
        {
            return Err(AppError::not_found("workspace tool catalog entry"));
        }

        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let mut disabled_keys = disabled_source_keys(&document);
        if patch.disabled {
            disabled_keys.insert(patch.source_key);
        } else {
            disabled_keys.remove(&patch.source_key);
        }
        set_disabled_source_keys(&mut document, &disabled_keys)?;
        self.save_workspace_runtime_document(document)?;
        self.build_tool_catalog().await
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
        skill_document_from_path(
            &self.state.paths.root,
            &skill_path,
            SkillSourceOrigin::SkillsDir,
        )
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
        if let Some(asset) = crate::agent_assets::find_builtin_skill_asset_by_id(skill_id)? {
            return self.import_skill_files_to_managed_root(&input.slug, asset.files);
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
        self.import_skill_files_to_managed_root(&input.slug, files)
    }

    async fn import_workspace_skill_archive(
        &self,
        input: ImportWorkspaceSkillArchiveInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let files = extract_archive_entries(&input)?;
        self.import_skill_files_to_managed_root(&input.slug, files)
    }

    async fn import_workspace_skill_folder(
        &self,
        input: ImportWorkspaceSkillFolderInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let files = normalize_uploaded_files(&input.files)?;
        self.import_skill_files_to_managed_root(&input.slug, files)
    }

    async fn delete_workspace_skill(&self, skill_id: &str) -> Result<(), AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        let skill_dir = entry
            .path
            .parent()
            .ok_or_else(|| AppError::invalid_input("workspace skill path is invalid"))?;
        fs::remove_dir_all(skill_dir)?;
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
        self.load_mcp_server_document(&input.server_name)
    }

    async fn copy_workspace_mcp_server_to_managed(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        let asset = crate::agent_assets::find_builtin_mcp_asset(server_name)?
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
        self.load_mcp_server_document(server_name)
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
        self.load_mcp_server_document(server_name)
    }

    async fn delete_workspace_mcp_server(&self, server_name: &str) -> Result<(), AppError> {
        let mut document = load_workspace_runtime_document(&self.state.paths)?;
        let servers = ensure_top_level_object(&mut document, "mcpServers")?;
        if servers.remove(server_name).is_none() {
            return Err(AppError::not_found("workspace mcp server"));
        }
        self.save_workspace_runtime_document(document)?;
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

    async fn list_automations(&self) -> Result<Vec<AutomationRecord>, AppError> {
        Ok(self
            .state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?
            .clone())
    }

    async fn create_automation(
        &self,
        mut record: AutomationRecord,
    ) -> Result<AutomationRecord, AppError> {
        if record.id.is_empty() {
            record.id = format!("automation-{}", Uuid::new_v4());
        }
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.title,
                record.description,
                record.cadence,
                record.owner_type,
                record.owner_id,
                record.status,
                record.next_run_at.map(|value| value as i64),
                record.last_run_at.map(|value| value as i64),
                record.output,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut automations = self
            .state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?;
        automations.push(record.clone());
        Ok(record)
    }

    async fn update_automation(
        &self,
        automation_id: &str,
        mut record: AutomationRecord,
    ) -> Result<AutomationRecord, AppError> {
        record.id = automation_id.into();
        if record.workspace_id.is_empty() {
            record.workspace_id = self.state.workspace_id()?;
        }

        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO automations (id, workspace_id, project_id, title, description, cadence, owner_type, owner_id, status, next_run_at, last_run_at, output)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.workspace_id,
                record.project_id,
                record.title,
                record.description,
                record.cadence,
                record.owner_type,
                record.owner_id,
                record.status,
                record.next_run_at.map(|value| value as i64),
                record.last_run_at.map(|value| value as i64),
                record.output,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut automations = self
            .state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?;
        Self::replace_or_push(&mut automations, record.clone(), |item| {
            item.id == automation_id
        });
        Ok(record)
    }

    async fn delete_automation(&self, automation_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM automations WHERE id = ?1",
                params![automation_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .automations
            .lock()
            .map_err(|_| AppError::runtime("automations mutex poisoned"))?
            .retain(|item| item.id != automation_id);
        Ok(())
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
}
