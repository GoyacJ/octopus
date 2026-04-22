use super::*;

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
        Ok(self.state.workspace_snapshot()?.owner_user_id.is_some())
    }

    pub(super) fn adopt_bootstrap_projects(
        &self,
        db: &Connection,
        user_id: &str,
    ) -> Result<(), AppError> {
        let mut projects = self
            .state
            .projects
            .lock()
            .map_err(|_| AppError::runtime("projects mutex poisoned"))?;

        for project in projects.iter_mut() {
            let replaces_owner = project.owner_user_id == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID;
            let had_placeholder_member = project
                .member_user_ids
                .iter()
                .any(|member_user_id| member_user_id == BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID);

            if !replaces_owner && !had_placeholder_member {
                continue;
            }

            if replaces_owner {
                project.owner_user_id = user_id.to_string();
            }

            let mut member_user_ids = project
                .member_user_ids
                .iter()
                .filter(|member_user_id| {
                    member_user_id.as_str() != BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            member_user_ids.insert(user_id.to_string());
            project.member_user_ids = member_user_ids.into_iter().collect();

            db.execute(
                "UPDATE projects SET owner_user_id = ?2, member_user_ids_json = ?3 WHERE id = ?1",
                params![
                    project.id,
                    project.owner_user_id,
                    serde_json::to_string(&project.member_user_ids)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        }

        Ok(())
    }

    pub(super) fn persist_session(
        &self,
        user: &StoredUser,
        client_app_id: String,
    ) -> Result<SessionRecord, AppError> {
        let workspace = self.workspace_snapshot()?;
        let connection = self.state.open_db()?;
        let session = SessionRecord {
            id: format!("sess-{}", Uuid::new_v4()),
            workspace_id: workspace.id,
            user_id: user.record.id.clone(),
            client_app_id,
            token: Uuid::new_v4().to_string(),
            status: "active".into(),
            created_at: Self::now(),
            expires_at: None,
        };

        connection
            .execute(
                "INSERT INTO sessions (id, workspace_id, user_id, client_app_id, token, status, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    session.id,
                    session.workspace_id,
                    session.user_id,
                    session.client_app_id,
                    session.token,
                    session.status,
                    session.created_at as i64,
                    session.expires_at.map(|value| value as i64),
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
