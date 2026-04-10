use super::*;

pub(super) fn to_user_summary(paths: &WorkspacePaths, user: &StoredUser) -> UserRecordSummary {
    UserRecordSummary {
        id: user.record.id.clone(),
        username: user.record.username.clone(),
        display_name: user.record.display_name.clone(),
        avatar: avatar_data_url(paths, user),
        status: user.record.status.clone(),
        password_state: user.record.password_state.clone(),
        role_ids: user.membership.role_ids.clone(),
        scope_project_ids: user.membership.scope_project_ids.clone(),
    }
}

pub(super) fn default_client_apps() -> Vec<ClientAppRecord> {
    vec![
        ClientAppRecord {
            id: "octopus-desktop".into(),
            name: "Octopus Desktop".into(),
            platform: "desktop".into(),
            status: "active".into(),
            first_party: true,
            allowed_origins: Vec::new(),
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into(), "runtime".into()],
        },
        ClientAppRecord {
            id: "octopus-web".into(),
            name: "Octopus Web".into(),
            platform: "web".into(),
            status: "active".into(),
            first_party: true,
            allowed_origins: vec!["http://127.0.0.1".into(), "http://localhost".into()],
            allowed_hosts: vec!["127.0.0.1".into(), "localhost".into()],
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into(), "runtime".into()],
        },
        ClientAppRecord {
            id: "octopus-mobile".into(),
            name: "Octopus Mobile".into(),
            platform: "mobile".into(),
            status: "disabled".into(),
            first_party: true,
            allowed_origins: Vec::new(),
            allowed_hosts: Vec::new(),
            session_policy: "session_token".into(),
            default_scopes: vec!["workspace".into()],
        },
    ]
}

pub(super) fn hash_password(password: &str) -> String {
    format!("plain::{password}")
}

pub(super) fn verify_password(password: &str, hash: &str) -> bool {
    hash == hash_password(password)
}

pub(super) fn append_json_line(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
    let mut raw = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };
    raw.push_str(&serde_json::to_string(value)?);
    raw.push('\n');
    fs::write(path, raw)?;
    Ok(())
}

impl InfraWorkspaceService {
    pub(super) fn now() -> u64 {
        timestamp_now()
    }

    pub(super) fn ensure_project_exists(&self, project_id: &str) -> Result<(), AppError> {
        let exists = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?
            .iter()
            .any(|project| project.id == project_id);
        if exists {
            Ok(())
        } else {
            Err(AppError::not_found("project not found"))
        }
    }

    pub(super) fn pet_scope_key(project_id: Option<&str>) -> String {
        project_id.unwrap_or("workspace").to_string()
    }

    pub(super) fn workspace_pet_snapshot(&self) -> Result<PetWorkspaceSnapshot, AppError> {
        let profile = default_pet_profile();
        let presence = self
            .state
            .workspace_pet_presence
            .lock()
            .map_err(|_| AppError::runtime("workspace pet presence mutex poisoned"))?
            .clone();
        let binding = self
            .state
            .workspace_pet_binding
            .lock()
            .map_err(|_| AppError::runtime("workspace pet binding mutex poisoned"))?
            .clone();
        let messages = if let Some(binding) = binding.as_ref() {
            load_runtime_messages_for_conversation(
                &self.state.open_db()?,
                &binding.conversation_id,
                &profile.id,
            )?
        } else {
            vec![]
        };
        Ok(PetWorkspaceSnapshot {
            profile,
            presence,
            binding,
            messages,
        })
    }

    pub(super) fn project_pet_snapshot(
        &self,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.ensure_project_exists(project_id)?;
        let profile = default_pet_profile();
        let presence = self
            .state
            .project_pet_presences
            .lock()
            .map_err(|_| AppError::runtime("project pet presences mutex poisoned"))?
            .iter()
            .find(|(id, _)| id == project_id)
            .map(|(_, presence)| presence.clone())
            .unwrap_or_else(default_workspace_pet_presence);
        let binding = self
            .state
            .project_pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("project pet bindings mutex poisoned"))?
            .iter()
            .find(|(id, _)| id == project_id)
            .map(|(_, binding)| binding.clone());
        let messages = if let Some(binding) = binding.as_ref() {
            load_runtime_messages_for_conversation(
                &self.state.open_db()?,
                &binding.conversation_id,
                &profile.id,
            )?
        } else {
            vec![]
        };
        Ok(PetWorkspaceSnapshot {
            profile,
            presence,
            binding,
            messages,
        })
    }

    pub(super) fn persist_pet_presence(
        &self,
        project_id: Option<&str>,
        presence: &PetPresenceState,
    ) -> Result<(), AppError> {
        let scope_key = Self::pet_scope_key(project_id);
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO pet_presence (scope_key, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                scope_key,
                project_id,
                presence.pet_id,
                if presence.is_visible { 1 } else { 0 },
                if presence.chat_open { 1 } else { 0 },
                presence.motion_state,
                presence.unread_count as i64,
                presence.last_interaction_at as i64,
                presence.position.x,
                presence.position.y,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) fn persist_pet_binding(
        &self,
        project_id: Option<&str>,
        binding: &PetConversationBinding,
    ) -> Result<(), AppError> {
        let scope_key = Self::pet_scope_key(project_id);
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO pet_conversation_bindings (scope_key, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                scope_key,
                project_id,
                binding.pet_id,
                binding.workspace_id,
                binding.conversation_id,
                binding.session_id,
                binding.updated_at as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) fn normalize_project_name(name: &str) -> Result<String, AppError> {
        let normalized = name.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input("project name is required"));
        }
        Ok(normalized.into())
    }

    pub(super) fn normalize_project_description(description: &str) -> String {
        description.trim().into()
    }

    pub(super) fn normalize_project_status(status: &str) -> Result<String, AppError> {
        match status.trim() {
            "active" => Ok("active".into()),
            "archived" => Ok("archived".into()),
            _ => Err(AppError::invalid_input(
                "project status must be active or archived",
            )),
        }
    }

    pub(super) fn next_active_project_id(
        projects: &[ProjectRecord],
        current_project_id: &str,
    ) -> Option<String> {
        projects
            .iter()
            .find(|project| project.id != current_project_id && project.status == "active")
            .map(|project| project.id.clone())
    }

    pub(super) fn replace_or_push<T, F>(items: &mut Vec<T>, value: T, matcher: F)
    where
        F: Fn(&T) -> bool,
    {
        if let Some(existing) = items.iter_mut().find(|item| matcher(item)) {
            *existing = value;
        } else {
            items.push(value);
        }
    }

    pub(super) fn remove_avatar_file(&self, avatar_path: Option<&str>) -> Result<(), AppError> {
        let Some(avatar_path) = avatar_path else {
            return Ok(());
        };

        match fs::remove_file(self.state.paths.root.join(avatar_path)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub(super) fn persist_workspace_avatar(
        &self,
        entity_id: &str,
        avatar: &AvatarUploadPayload,
    ) -> Result<(String, String, u64, String), AppError> {
        let content_type = avatar.content_type.trim().to_ascii_lowercase();
        if !matches!(
            content_type.as_str(),
            "image/png" | "image/jpeg" | "image/jpg" | "image/webp"
        ) {
            return Err(AppError::invalid_input("avatar must be png, jpeg, or webp"));
        }
        if avatar.byte_size == 0 || avatar.byte_size > 2 * 1024 * 1024 {
            return Err(AppError::invalid_input("avatar must be 2 MiB or smaller"));
        }

        let bytes = BASE64_STANDARD
            .decode(&avatar.data_base64)
            .map_err(|_| AppError::invalid_input("avatar payload is invalid"))?;
        if bytes.len() as u64 != avatar.byte_size {
            return Err(AppError::invalid_input("avatar byte size mismatch"));
        }

        let extension = match content_type.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "jpg",
        };
        let relative_path = format!("data/blobs/avatars/{entity_id}.{extension}");
        let absolute_path = self.state.paths.root.join(&relative_path);
        fs::write(&absolute_path, &bytes)?;

        Ok((
            relative_path,
            content_type,
            avatar.byte_size,
            content_hash(&bytes),
        ))
    }

    pub(super) fn persist_avatar(
        &self,
        user_id: &str,
        avatar: &AvatarUploadPayload,
    ) -> Result<(String, String, u64, String), AppError> {
        self.persist_workspace_avatar(user_id, avatar)
    }

    pub(super) fn build_agent_record(
        &self,
        agent_id: &str,
        input: UpsertAgentInput,
        current: Option<&AgentRecord>,
    ) -> Result<AgentRecord, AppError> {
        let next_avatar_path = if input.remove_avatar.unwrap_or(false) {
            None
        } else if let Some(avatar) = input.avatar.as_ref() {
            Some(self.persist_workspace_avatar(agent_id, avatar)?.0)
        } else {
            current.and_then(|record| record.avatar_path.clone())
        };
        let avatar = agent_avatar(&self.state.paths, next_avatar_path.as_deref());

        Ok(AgentRecord {
            id: agent_id.into(),
            workspace_id: if input.workspace_id.trim().is_empty() {
                self.state.workspace_id()?
            } else {
                input.workspace_id
            },
            project_id: input.project_id,
            scope: input.scope,
            name: input.name.trim().into(),
            avatar_path: next_avatar_path,
            avatar,
            personality: input.personality.trim().into(),
            tags: input.tags,
            prompt: input.prompt.trim().into(),
            builtin_tool_keys: input.builtin_tool_keys,
            skill_ids: input.skill_ids,
            mcp_server_names: input.mcp_server_names,
            integration_source: None,
            description: input.description.trim().into(),
            status: input.status.trim().into(),
            updated_at: Self::now(),
        })
    }

    pub(super) fn build_team_record(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
        current: Option<&TeamRecord>,
    ) -> Result<TeamRecord, AppError> {
        let next_avatar_path = if input.remove_avatar.unwrap_or(false) {
            None
        } else if let Some(avatar) = input.avatar.as_ref() {
            Some(self.persist_workspace_avatar(team_id, avatar)?.0)
        } else {
            current.and_then(|record| record.avatar_path.clone())
        };
        let avatar = agent_avatar(&self.state.paths, next_avatar_path.as_deref());

        Ok(TeamRecord {
            id: team_id.into(),
            workspace_id: if input.workspace_id.trim().is_empty() {
                self.state.workspace_id()?
            } else {
                input.workspace_id
            },
            project_id: input.project_id,
            scope: input.scope,
            name: input.name.trim().into(),
            avatar_path: next_avatar_path,
            avatar,
            personality: input.personality.trim().into(),
            tags: input.tags,
            prompt: input.prompt.trim().into(),
            builtin_tool_keys: input.builtin_tool_keys,
            skill_ids: input.skill_ids,
            mcp_server_names: input.mcp_server_names,
            leader_agent_id: input
                .leader_agent_id
                .filter(|value| !value.trim().is_empty()),
            member_agent_ids: input.member_agent_ids,
            integration_source: None,
            description: input.description.trim().into(),
            status: input.status.trim().into(),
            updated_at: Self::now(),
        })
    }

    pub(super) fn validate_workspace_user_identity(
        &self,
        username: &str,
        display_name: &str,
        exclude_user_id: Option<&str>,
    ) -> Result<(), AppError> {
        if username.trim().is_empty() || display_name.trim().is_empty() {
            return Err(AppError::invalid_input(
                "username and display name are required",
            ));
        }

        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        let username_exists = users.iter().any(|user| {
            if let Some(excluded_id) = exclude_user_id {
                if user.record.id == excluded_id {
                    return false;
                }
            }
            user.record.username == username.trim()
        });
        if username_exists {
            return Err(AppError::conflict("username already exists"));
        }
        Ok(())
    }

    pub(super) fn resolve_member_password(
        &self,
        password: Option<&str>,
        confirm_password: Option<&str>,
        use_default_password: bool,
    ) -> Result<(String, String), AppError> {
        if use_default_password || password.is_none() {
            return Ok((hash_password("changeme"), "reset-required".into()));
        }

        let password = password.unwrap_or_default();
        let confirm_password = confirm_password.unwrap_or_default();
        if password.len() < 8 {
            return Err(AppError::invalid_input(
                "password must be at least 8 characters",
            ));
        }
        if password != confirm_password {
            return Err(AppError::invalid_input(
                "password confirmation does not match",
            ));
        }

        Ok((hash_password(password), "set".into()))
    }
}

impl InfraAuthService {
    pub(super) fn now() -> u64 {
        timestamp_now()
    }

    pub(super) fn workspace_snapshot(&self) -> Result<WorkspaceSummary, AppError> {
        self.state.workspace_snapshot()
    }

    pub(super) fn ensure_active_client_app(
        &self,
        client_app_id: &str,
    ) -> Result<ClientAppRecord, AppError> {
        let app = self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .iter()
            .find(|app| app.id == client_app_id)
            .cloned()
            .ok_or_else(|| AppError::auth("client app is not registered"))?;
        if app.status != "active" {
            return Err(AppError::auth("client app is disabled"));
        }
        Ok(app)
    }

    pub(super) fn owner_exists(&self) -> Result<bool, AppError> {
        Ok(self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .iter()
            .any(|user| {
                user.membership
                    .role_ids
                    .iter()
                    .any(|role_id| role_id == "owner")
            }))
    }

    pub(super) fn persist_session(
        &self,
        user: &StoredUser,
        client_app_id: String,
    ) -> Result<SessionRecord, AppError> {
        let workspace = self.workspace_snapshot()?;
        let session = SessionRecord {
            id: format!("sess-{}", Uuid::new_v4()),
            workspace_id: workspace.id,
            user_id: user.record.id.clone(),
            client_app_id,
            token: Uuid::new_v4().to_string(),
            status: "active".into(),
            created_at: Self::now(),
            expires_at: None,
            role_ids: user.membership.role_ids.clone(),
            scope_project_ids: user.membership.scope_project_ids.clone(),
        };

        self.state
            .open_db()?
            .execute(
                "INSERT INTO sessions (id, workspace_id, user_id, client_app_id, token, status, created_at, expires_at, role_ids, scope_project_ids)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    session.id,
                    session.workspace_id,
                    session.user_id,
                    session.client_app_id,
                    session.token,
                    session.status,
                    session.created_at as i64,
                    session.expires_at.map(|value| value as i64),
                    serde_json::to_string(&session.role_ids)?,
                    serde_json::to_string(&session.scope_project_ids)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .push(session.clone());

        Ok(session)
    }

    pub(super) fn persist_avatar(
        &self,
        user_id: &str,
        avatar: &AvatarUploadPayload,
    ) -> Result<(String, String, u64, String), AppError> {
        let content_type = avatar.content_type.trim().to_ascii_lowercase();
        if !matches!(
            content_type.as_str(),
            "image/png" | "image/jpeg" | "image/jpg" | "image/webp"
        ) {
            return Err(AppError::invalid_input("avatar must be png, jpeg, or webp"));
        }
        if avatar.byte_size == 0 || avatar.byte_size > 2 * 1024 * 1024 {
            return Err(AppError::invalid_input("avatar must be 2 MiB or smaller"));
        }

        let bytes = BASE64_STANDARD
            .decode(&avatar.data_base64)
            .map_err(|_| AppError::invalid_input("avatar payload is invalid"))?;
        if bytes.len() as u64 != avatar.byte_size {
            return Err(AppError::invalid_input("avatar byte size mismatch"));
        }

        let extension = match content_type.as_str() {
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "jpg",
        };
        let relative_path = format!("data/blobs/avatars/{user_id}.{extension}");
        let absolute_path = self.state.paths.root.join(&relative_path);
        fs::write(&absolute_path, &bytes)?;

        Ok((
            relative_path,
            content_type,
            avatar.byte_size,
            content_hash(&bytes),
        ))
    }
}

#[async_trait]
impl AuthService for InfraAuthService {
    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AppError> {
        self.ensure_active_client_app(&request.client_app_id)?;

        let user = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .iter()
            .find(|user| user.record.username == request.username)
            .cloned()
            .ok_or_else(|| AppError::auth("invalid credentials"))?;
        if !verify_password(&request.password, &user.password_hash) {
            return Err(AppError::auth("invalid credentials"));
        }

        let session = self.persist_session(&user, request.client_app_id)?;

        Ok(LoginResponse {
            session,
            workspace: self.workspace_snapshot()?,
        })
    }

    async fn register_owner(
        &self,
        request: RegisterWorkspaceOwnerRequest,
    ) -> Result<RegisterWorkspaceOwnerResponse, AppError> {
        self.ensure_active_client_app(&request.client_app_id)?;

        if request.username.trim().is_empty() || request.display_name.trim().is_empty() {
            return Err(AppError::invalid_input(
                "username and display name are required",
            ));
        }
        if request.password.len() < 8 {
            return Err(AppError::invalid_input(
                "password must be at least 8 characters",
            ));
        }
        if request.password != request.confirm_password {
            return Err(AppError::invalid_input(
                "password confirmation does not match",
            ));
        }
        if self.owner_exists()? {
            return Err(AppError::conflict("workspace owner already exists"));
        }

        let workspace = self.workspace_snapshot()?;
        if workspace.bootstrap_status != "setup_required" && workspace.owner_user_id.is_some() {
            return Err(AppError::conflict("workspace owner already exists"));
        }

        {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            if users
                .iter()
                .any(|user| user.record.username == request.username.trim())
            {
                return Err(AppError::conflict("username already exists"));
            }
        }

        let now = Self::now();
        let user_id = format!("user-{}", Uuid::new_v4());
        let (avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash) =
            self.persist_avatar(&user_id, &request.avatar)?;
        let user_record = UserRecord {
            id: user_id.clone(),
            username: request.username.trim().to_string(),
            display_name: request.display_name.trim().to_string(),
            avatar_path: Some(avatar_path.clone()),
            avatar_content_type: Some(avatar_content_type.clone()),
            avatar_byte_size: Some(avatar_byte_size),
            avatar_content_hash: Some(avatar_content_hash.clone()),
            status: "active".into(),
            password_state: "set".into(),
            created_at: now,
            updated_at: now,
        };
        let membership = WorkspaceMembershipRecord {
            workspace_id: workspace.id.clone(),
            user_id: user_id.clone(),
            role_ids: vec!["owner".into()],
            scope_mode: "all-projects".into(),
            scope_project_ids: Vec::new(),
        };

        let db = self.state.open_db()?;
        db.execute(
            "INSERT INTO users (id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash, status, password_hash, password_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                user_record.id,
                user_record.username,
                user_record.display_name,
                user_record.avatar_path,
                user_record.avatar_content_type,
                user_record.avatar_byte_size.map(|value| value as i64),
                user_record.avatar_content_hash,
                user_record.status,
                hash_password(&request.password),
                user_record.password_state,
                user_record.created_at as i64,
                user_record.updated_at as i64,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        db.execute(
            "INSERT INTO memberships (workspace_id, user_id, role_ids, scope_mode, scope_project_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                membership.workspace_id,
                membership.user_id,
                serde_json::to_string(&membership.role_ids)?,
                membership.scope_mode,
                serde_json::to_string(&membership.scope_project_ids)?,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;

        {
            let mut workspace_state = self
                .state
                .workspace
                .lock()
                .map_err(|_| AppError::runtime("workspace mutex poisoned"))?;
            workspace_state.bootstrap_status = "ready".into();
            workspace_state.owner_user_id = Some(user_id.clone());
        }
        self.state.save_workspace_config()?;

        let stored_user = StoredUser {
            record: user_record,
            password_hash: hash_password(&request.password),
            membership,
        };
        self.state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .push(stored_user.clone());

        let session = self.persist_session(&stored_user, request.client_app_id)?;

        Ok(RegisterWorkspaceOwnerResponse {
            session,
            workspace: self.workspace_snapshot()?,
        })
    }

    async fn logout(&self, token: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE sessions SET status = 'revoked' WHERE token = ?1",
                params![token],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if let Some(session) = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter_mut()
            .find(|session| session.token == token)
        {
            session.status = "revoked".into();
        }

        Ok(())
    }

    async fn session(&self, token: &str) -> Result<SessionRecord, AppError> {
        self.lookup_session(token)
            .await?
            .ok_or_else(|| AppError::auth("session token is invalid"))
    }

    async fn lookup_session(&self, token: &str) -> Result<Option<SessionRecord>, AppError> {
        Ok(self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter()
            .find(|session| session.token == token && session.status == "active")
            .cloned())
    }
}

#[async_trait]
impl AppRegistryService for InfraAppRegistryService {
    async fn list_apps(&self) -> Result<Vec<ClientAppRecord>, AppError> {
        Ok(self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .clone())
    }

    async fn register_app(&self, record: ClientAppRecord) -> Result<ClientAppRecord, AppError> {
        if !record.first_party {
            return Err(AppError::invalid_input(
                "phase one only accepts first-party client apps",
            ));
        }

        self.state
            .open_db()?
            .execute(
                "INSERT OR REPLACE INTO client_apps
                 (id, name, platform, status, first_party, allowed_origins, allowed_hosts, session_policy, default_scopes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    record.id,
                    record.name,
                    record.platform,
                    record.status,
                    if record.first_party { 1 } else { 0 },
                    serde_json::to_string(&record.allowed_origins)?,
                    serde_json::to_string(&record.allowed_hosts)?,
                    record.session_policy,
                    serde_json::to_string(&record.default_scopes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        let mut apps = self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?;
        if let Some(existing) = apps.iter_mut().find(|app| app.id == record.id) {
            *existing = record.clone();
        } else {
            apps.push(record.clone());
        }
        let registry = AppRegistryFile { apps: apps.clone() };
        fs::write(
            &self.state.paths.app_registry_config,
            toml::to_string_pretty(&registry)?,
        )?;

        Ok(record)
    }

    async fn find_app(&self, app_id: &str) -> Result<Option<ClientAppRecord>, AppError> {
        Ok(self
            .state
            .apps
            .lock()
            .map_err(|_| AppError::runtime("app registry mutex poisoned"))?
            .iter()
            .find(|app| app.id == app_id)
            .cloned())
    }
}

#[async_trait]
impl RbacService for InfraRbacService {
    async fn authorize(
        &self,
        session: &SessionRecord,
        _capability: &str,
        project_id: Option<&str>,
    ) -> Result<AuthorizationDecision, AppError> {
        if session.role_ids.iter().any(|role| role == "owner") {
            return Ok(AuthorizationDecision {
                allowed: project_id
                    .map(|project| {
                        session.scope_project_ids.is_empty()
                            || session.scope_project_ids.iter().any(|item| item == project)
                    })
                    .unwrap_or(true),
                reason: None,
            });
        }

        Ok(AuthorizationDecision {
            allowed: false,
            reason: Some("no matching role permission".into()),
        })
    }
}
