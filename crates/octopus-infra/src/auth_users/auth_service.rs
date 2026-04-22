use super::*;

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

    async fn register_bootstrap_admin(
        &self,
        request: RegisterBootstrapAdminRequest,
    ) -> Result<RegisterBootstrapAdminResponse, AppError> {
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
        let mapped_directory =
            normalize_mapped_directory_input(request.mapped_directory.as_deref())?;
        let current_workspace_root = self.state.paths.root.clone();
        let shell_root = PathBuf::from(workspace_shell_root_display_path(
            &workspace,
            &self.state.paths,
        ));

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
            "INSERT OR IGNORE INTO org_units (id, parent_id, code, name, status)
             VALUES ('org-root', NULL, ?1, ?2, 'active')",
            params![workspace.slug, workspace.name,],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        ensure_default_owner_role_permissions(&db)?;
        db.execute(
            "INSERT OR REPLACE INTO user_org_assignments (user_id, org_unit_id, is_primary, position_ids, user_group_ids)
             VALUES (?1, 'org-root', 1, '[]', '[]')",
            params![user_id],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        db.execute(
            "INSERT OR REPLACE INTO role_bindings (id, role_id, subject_type, subject_id, effect)
             VALUES (?1, ?2, 'user', ?3, 'allow')",
            params![
                format!("binding-user-{user_id}-system-owner"),
                SYSTEM_OWNER_ROLE_ID,
                user_id
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        self.adopt_bootstrap_projects(&db, &user_id)?;

        {
            let mut workspace_state = self
                .state
                .workspace
                .lock()
                .map_err(|_| AppError::runtime("workspace mutex poisoned"))?;
            workspace_state.bootstrap_status = "ready".into();
            workspace_state.owner_user_id = Some(user_id.clone());
            workspace_state.mapped_directory = mapped_directory.clone();
            workspace_state.mapped_directory_default =
                Some(shell_root.to_string_lossy().to_string());
        }
        self.state.save_workspace_config()?;

        let stored_user = StoredUser {
            record: user_record,
            password_hash: hash_password(&request.password),
        };
        self.state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .push(stored_user.clone());
        ensure_personal_pet_for_user(&db, &workspace.id, &user_id)?;
        *self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))? = load_agents(&db)?;
        *self
            .state
            .pet_extensions
            .lock()
            .map_err(|_| AppError::runtime("pet extensions mutex poisoned"))? =
            load_pet_agent_extensions(&db)?;

        let session = self.persist_session(&stored_user, request.client_app_id)?;
        if let Some(next_workspace_root) = mapped_directory
            .as_deref()
            .map(PathBuf::from)
            .filter(|path| path != &current_workspace_root)
        {
            let workspace = self.state.workspace_snapshot()?;
            let workspace_avatar_path = self
                .state
                .workspace_avatar_path
                .lock()
                .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?
                .clone();
            let workspace_avatar_content_type = self
                .state
                .workspace_avatar_content_type
                .lock()
                .map_err(|_| AppError::runtime("workspace avatar mutex poisoned"))?
                .clone();
            bootstrap::relocate_workspace_root(
                &current_workspace_root,
                &next_workspace_root,
                &shell_root,
                &workspace,
                workspace_avatar_path.as_deref(),
                workspace_avatar_content_type.as_deref(),
            )?;
        }

        Ok(RegisterBootstrapAdminResponse {
            session,
            workspace: self.workspace_snapshot()?,
        })
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

    async fn list_sessions(&self) -> Result<Vec<SessionRecord>, AppError> {
        Ok(self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .clone())
    }

    async fn revoke_session(&self, session_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE sessions SET status = 'revoked' WHERE id = ?1",
                params![session_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        if let Some(session) = self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter_mut()
            .find(|session| session.id == session_id)
        {
            session.status = "revoked".into();
        }

        Ok(())
    }

    async fn revoke_user_sessions(&self, user_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE sessions SET status = 'revoked' WHERE user_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;

        for session in self
            .state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .iter_mut()
        {
            if session.user_id == user_id {
                session.status = "revoked".into();
            }
        }

        Ok(())
    }
}
