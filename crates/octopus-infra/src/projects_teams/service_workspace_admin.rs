use super::*;

impl InfraWorkspaceService {
    pub(crate) async fn list_models_impl(&self) -> Result<Vec<ModelCatalogRecord>, AppError> {
        Ok(self
            .state
            .model_catalog
            .lock()
            .map_err(|_| AppError::runtime("model catalog mutex poisoned"))?
            .clone())
    }

    pub(crate) async fn list_provider_credentials_impl(
        &self,
    ) -> Result<Vec<ProviderCredentialRecord>, AppError> {
        Ok(self
            .state
            .provider_credentials
            .lock()
            .map_err(|_| AppError::runtime("provider credentials mutex poisoned"))?
            .clone())
    }

    pub(crate) async fn get_capability_management_projection_impl(
        &self,
    ) -> Result<octopus_core::CapabilityManagementProjection, AppError> {
        self.build_capability_management_projection().await
    }

    pub(crate) async fn set_capability_asset_disabled_impl(
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

    pub(crate) async fn get_workspace_skill_impl(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.get_workspace_skill_document(skill_id)
    }

    pub(crate) async fn create_workspace_skill_impl(
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

    pub(crate) async fn update_workspace_skill_impl(
        &self,
        skill_id: &str,
        input: UpdateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        let entry = self.ensure_workspace_owned_skill_entry(skill_id)?;
        fs::write(&entry.path, input.content)?;
        skill_document_from_path(&self.state.paths.root, &entry.path, entry.origin)
    }

    pub(crate) async fn get_workspace_skill_tree_impl(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError> {
        self.get_workspace_skill_tree_document(skill_id)
    }

    pub(crate) async fn get_workspace_skill_file_impl(
        &self,
        skill_id: &str,
        relative_path: &str,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        self.get_workspace_skill_file_document(skill_id, relative_path)
    }

    pub(crate) async fn update_workspace_skill_file_impl(
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

    pub(crate) async fn copy_workspace_skill_to_managed_impl(
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

    pub(crate) async fn import_workspace_skill_archive_impl(
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

    pub(crate) async fn import_workspace_skill_folder_impl(
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

    pub(crate) async fn delete_workspace_skill_impl(&self, skill_id: &str) -> Result<(), AppError> {
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

    pub(crate) async fn get_workspace_mcp_server_impl(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        self.load_mcp_server_document(server_name)
    }

    pub(crate) async fn create_workspace_mcp_server_impl(
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

    pub(crate) async fn copy_workspace_mcp_server_to_managed_impl(
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

    pub(crate) async fn update_workspace_mcp_server_impl(
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

    pub(crate) async fn delete_workspace_mcp_server_impl(
        &self,
        server_name: &str,
    ) -> Result<(), AppError> {
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

    pub(crate) async fn list_tools_impl(&self) -> Result<Vec<ToolRecord>, AppError> {
        Ok(self
            .state
            .tools
            .lock()
            .map_err(|_| AppError::runtime("tools mutex poisoned"))?
            .clone())
    }

    pub(crate) async fn create_tool_impl(
        &self,
        mut record: ToolRecord,
    ) -> Result<ToolRecord, AppError> {
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

    pub(crate) async fn update_tool_impl(
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

    pub(crate) async fn delete_tool_impl(&self, tool_id: &str) -> Result<(), AppError> {
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

    pub(crate) async fn current_user_profile_impl(
        &self,
        user_id: &str,
    ) -> Result<UserRecordSummary, AppError> {
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

    pub(crate) async fn update_current_user_profile_impl(
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

    pub(crate) async fn change_current_user_password_impl(
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
