use super::*;

fn default_user_password_state(
    password: Option<&str>,
    confirm_password: Option<&str>,
    reset_password: bool,
) -> Result<(String, String), AppError> {
    if reset_password || password.is_none() {
        return Ok((hash_password("changeme"), "reset-required".into()));
    }
    let password = password.unwrap_or_default();
    if password.len() < 8 {
        return Err(AppError::invalid_input(
            "password must be at least 8 characters",
        ));
    }
    if password != confirm_password.unwrap_or_default() {
        return Err(AppError::invalid_input(
            "password confirmation does not match",
        ));
    }
    Ok((hash_password(password), "set".into()))
}

fn validate_username_unique(
    users: &[StoredUser],
    username: &str,
    exclude_user_id: Option<&str>,
) -> Result<(), AppError> {
    let normalized = username.trim();
    if normalized.is_empty() {
        return Err(AppError::invalid_input("username is required"));
    }
    let exists = users.iter().any(|user| {
        if let Some(excluded_id) = exclude_user_id {
            if user.record.id == excluded_id {
                return false;
            }
        }
        user.record.username == normalized
    });
    if exists {
        return Err(AppError::conflict("username already exists"));
    }
    Ok(())
}

impl InfraAccessControlService {
    pub(super) async fn list_users_impl(&self) -> Result<Vec<AccessUserRecord>, AppError> {
        let connection = self.state.open_db()?;
        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .clone();
        users
            .iter()
            .map(|user| map_access_user_record_from_parts(&connection, user))
            .collect()
    }

    pub(super) async fn get_experience_snapshot_impl(
        &self,
    ) -> Result<AccessExperienceSnapshot, AppError> {
        let connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        build_access_experience_snapshot(&connection)
    }

    pub(super) async fn list_member_summaries_impl(
        &self,
    ) -> Result<Vec<AccessMemberSummary>, AppError> {
        let connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .clone();
        build_access_member_summaries(&connection, &users)
    }

    pub(super) async fn assign_user_preset_impl(
        &self,
        user_id: &str,
        request: AccessUserPresetUpdateRequest,
    ) -> Result<AccessMemberSummary, AppError> {
        let role_id = preset_code_to_system_role_code(&request.preset_code).ok_or_else(|| {
            AppError::invalid_input(format!("unknown access preset: {}", request.preset_code))
        })?;

        {
            let users = self
                .state
                .users
                .lock()
                .map_err(|_| AppError::runtime("users mutex poisoned"))?;
            if !users.iter().any(|user| user.record.id == user_id) {
                return Err(AppError::not_found("access user"));
            }
        }

        let mut connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        let tx = connection
            .transaction()
            .map_err(|error| AppError::database(error.to_string()))?;
        tx.execute(
            "DELETE FROM role_bindings
             WHERE subject_type = 'user'
               AND subject_id = ?1
               AND role_id LIKE 'system.%'",
            params![user_id],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        tx.execute(
            "INSERT INTO role_bindings (id, role_id, subject_type, subject_id, effect)
             VALUES (?1, ?2, 'user', ?3, 'allow')",
            params![
                format!("binding-user-{user_id}-{}", role_id.replace('.', "-")),
                role_id,
                user_id,
            ],
        )
        .map_err(|error| AppError::database(error.to_string()))?;
        tx.commit()
            .map_err(|error| AppError::database(error.to_string()))?;

        let users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .clone();
        let summaries = build_access_member_summaries(&connection, &users)?;
        summaries
            .into_iter()
            .find(|summary| summary.user.id == user_id)
            .ok_or_else(|| AppError::not_found("access user"))
    }

    pub(super) async fn create_user_impl(
        &self,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError> {
        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        validate_username_unique(&users, &request.username, None)?;
        let user_id = format!("user-{}", Uuid::new_v4());
        let now = timestamp_now();
        let (password_hash, password_state) = default_user_password_state(
            request.password.as_deref(),
            request.confirm_password.as_deref(),
            false,
        )?;
        let stored_user = StoredUser {
            record: UserRecord {
                id: user_id.clone(),
                username: request.username.trim().into(),
                display_name: request.display_name.trim().into(),
                avatar_path: None,
                avatar_content_type: None,
                avatar_byte_size: None,
                avatar_content_hash: None,
                status: request.status.clone(),
                password_state: password_state.clone(),
                created_at: now,
                updated_at: now,
            },
            password_hash: password_hash.clone(),
        };

        let connection = self.state.open_db()?;
        connection.execute(
            "INSERT INTO users (id, username, display_name, avatar_path, avatar_content_type, avatar_byte_size, avatar_content_hash, status, password_hash, password_state, created_at, updated_at)
             VALUES (?1, ?2, ?3, NULL, NULL, NULL, NULL, ?4, ?5, ?6, ?7, ?8)",
            params![
                stored_user.record.id,
                stored_user.record.username,
                stored_user.record.display_name,
                stored_user.record.status,
                password_hash,
                password_state,
                now as i64,
                now as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        ensure_personal_pet_for_user(&connection, &self.state.workspace_id()?, &user_id)?;
        *self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))? = load_agents(&connection)?;
        *self
            .state
            .pet_extensions
            .lock()
            .map_err(|_| AppError::runtime("pet extensions mutex poisoned"))? =
            load_pet_agent_extensions(&connection)?;

        users.push(stored_user);
        let created = users
            .iter()
            .find(|user| user.record.id == user_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("created user"))?;
        map_access_user_record_from_parts(&connection, &created)
    }

    pub(super) async fn update_user_impl(
        &self,
        user_id: &str,
        request: AccessUserUpsertRequest,
    ) -> Result<AccessUserRecord, AppError> {
        let mut users = self
            .state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?;
        validate_username_unique(&users, &request.username, Some(user_id))?;
        let user = users
            .iter_mut()
            .find(|user| user.record.id == user_id)
            .ok_or_else(|| AppError::not_found("access user"))?;

        let (password_hash, password_state) = if request.reset_password.unwrap_or(false)
            || request.password.is_some()
            || request.confirm_password.is_some()
        {
            default_user_password_state(
                request.password.as_deref(),
                request.confirm_password.as_deref(),
                request.reset_password.unwrap_or(false),
            )?
        } else {
            (
                user.password_hash.clone(),
                user.record.password_state.clone(),
            )
        };

        let now = timestamp_now();
        let connection = self.state.open_db()?;
        connection.execute(
            "UPDATE users
             SET username = ?2, display_name = ?3, status = ?4, password_hash = ?5, password_state = ?6, updated_at = ?7
             WHERE id = ?1",
            params![
                user_id,
                request.username.trim(),
                request.display_name.trim(),
                request.status,
                password_hash,
                password_state,
                now as i64,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;

        user.record.username = request.username.trim().into();
        user.record.display_name = request.display_name.trim().into();
        user.record.status = request.status;
        user.record.password_state = password_state;
        user.record.updated_at = now;
        user.password_hash = password_hash;

        map_access_user_record_from_parts(&connection, user)
    }

    pub(super) async fn delete_user_impl(&self, user_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM users WHERE id = ?1", params![user_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        let connection = self.state.open_db()?;
        connection
            .execute(
                "DELETE FROM user_org_assignments WHERE user_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM role_bindings WHERE subject_type = 'user' AND subject_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM data_policies WHERE subject_type = 'user' AND subject_id = ?1",
                params![user_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute("DELETE FROM sessions WHERE user_id = ?1", params![user_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .users
            .lock()
            .map_err(|_| AppError::runtime("users mutex poisoned"))?
            .retain(|user| user.record.id != user_id);
        self.state
            .sessions
            .lock()
            .map_err(|_| AppError::runtime("sessions mutex poisoned"))?
            .retain(|session| session.user_id != user_id);
        Ok(())
    }

    pub(super) async fn list_org_units_impl(&self) -> Result<Vec<OrgUnitRecord>, AppError> {
        load_org_units(&self.state.open_db()?)
    }

    pub(super) async fn create_org_unit_impl(
        &self,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError> {
        let record = OrgUnitRecord {
            id: format!("org-{}", Uuid::new_v4()),
            parent_id: request.parent_id,
            code: request.code,
            name: request.name,
            status: request.status,
        };
        self.state.open_db()?.execute(
            "INSERT INTO org_units (id, parent_id, code, name, status) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.id, record.parent_id, record.code, record.name, record.status],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    pub(super) async fn update_org_unit_impl(
        &self,
        org_unit_id: &str,
        request: OrgUnitUpsertRequest,
    ) -> Result<OrgUnitRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE org_units SET parent_id = ?2, code = ?3, name = ?4, status = ?5 WHERE id = ?1",
            params![org_unit_id, request.parent_id, request.code, request.name, request.status],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(OrgUnitRecord {
            id: org_unit_id.into(),
            parent_id: request.parent_id,
            code: request.code,
            name: request.name,
            status: request.status,
        })
    }

    pub(super) async fn delete_org_unit_impl(&self, org_unit_id: &str) -> Result<(), AppError> {
        if org_unit_id == "org-root" {
            return Err(AppError::invalid_input("org-root cannot be deleted"));
        }
        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM org_units WHERE id = ?1", params![org_unit_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM user_org_assignments WHERE org_unit_id = ?1",
                params![org_unit_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_positions_impl(&self) -> Result<Vec<PositionRecord>, AppError> {
        load_positions(&self.state.open_db()?)
    }

    pub(super) async fn create_position_impl(
        &self,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError> {
        let record = PositionRecord {
            id: format!("position-{}", Uuid::new_v4()),
            code: request.code,
            name: request.name,
            status: request.status,
        };
        self.state
            .open_db()?
            .execute(
                "INSERT INTO positions (id, code, name, status) VALUES (?1, ?2, ?3, ?4)",
                params![record.id, record.code, record.name, record.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    pub(super) async fn update_position_impl(
        &self,
        position_id: &str,
        request: PositionUpsertRequest,
    ) -> Result<PositionRecord, AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE positions SET code = ?2, name = ?3, status = ?4 WHERE id = ?1",
                params![position_id, request.code, request.name, request.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(PositionRecord {
            id: position_id.into(),
            code: request.code,
            name: request.name,
            status: request.status,
        })
    }

    pub(super) async fn delete_position_impl(&self, position_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM positions WHERE id = ?1", params![position_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_user_groups_impl(&self) -> Result<Vec<UserGroupRecord>, AppError> {
        load_user_groups(&self.state.open_db()?)
    }

    pub(super) async fn create_user_group_impl(
        &self,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError> {
        let record = UserGroupRecord {
            id: format!("group-{}", Uuid::new_v4()),
            code: request.code,
            name: request.name,
            status: request.status,
        };
        self.state
            .open_db()?
            .execute(
                "INSERT INTO user_groups (id, code, name, status) VALUES (?1, ?2, ?3, ?4)",
                params![record.id, record.code, record.name, record.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    pub(super) async fn update_user_group_impl(
        &self,
        group_id: &str,
        request: UserGroupUpsertRequest,
    ) -> Result<UserGroupRecord, AppError> {
        self.state
            .open_db()?
            .execute(
                "UPDATE user_groups SET code = ?2, name = ?3, status = ?4 WHERE id = ?1",
                params![group_id, request.code, request.name, request.status],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(UserGroupRecord {
            id: group_id.into(),
            code: request.code,
            name: request.name,
            status: request.status,
        })
    }

    pub(super) async fn delete_user_group_impl(&self, group_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute("DELETE FROM user_groups WHERE id = ?1", params![group_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_user_org_assignments_impl(
        &self,
    ) -> Result<Vec<UserOrgAssignmentRecord>, AppError> {
        load_user_org_assignments(&self.state.open_db()?)
    }

    pub(super) async fn upsert_user_org_assignment_impl(
        &self,
        request: UserOrgAssignmentUpsertRequest,
    ) -> Result<UserOrgAssignmentRecord, AppError> {
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO user_org_assignments (user_id, org_unit_id, is_primary, position_ids, user_group_ids)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                request.user_id,
                request.org_unit_id,
                bool_to_sql(request.is_primary),
                serde_json::to_string(&request.position_ids)?,
                serde_json::to_string(&request.user_group_ids)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(UserOrgAssignmentRecord {
            user_id: request.user_id,
            org_unit_id: request.org_unit_id,
            is_primary: request.is_primary,
            position_ids: request.position_ids,
            user_group_ids: request.user_group_ids,
        })
    }

    pub(super) async fn delete_user_org_assignment_impl(
        &self,
        user_id: &str,
        org_unit_id: &str,
    ) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM user_org_assignments WHERE user_id = ?1 AND org_unit_id = ?2",
                params![user_id, org_unit_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }
}
