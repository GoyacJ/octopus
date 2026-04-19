use super::*;
use octopus_core::{default_agent_asset_role, AuthorizationRequest, DataPolicyRecord};
use std::collections::BTreeSet;

const BOOTSTRAP_OWNER_PLACEHOLDER_USER_ID: &str = "user-owner";

pub(super) fn to_user_summary(paths: &WorkspacePaths, user: &StoredUser) -> UserRecordSummary {
    UserRecordSummary {
        id: user.record.id.clone(),
        username: user.record.username.clone(),
        display_name: user.record.display_name.clone(),
        avatar: avatar_data_url(paths, user),
        status: user.record.status.clone(),
        password_state: user.record.password_state.clone(),
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

    fn ensure_personal_pet_loaded(
        &self,
        owner_user_id: &str,
    ) -> Result<PetAgentExtensionRecord, AppError> {
        if let Some(extension) = self
            .state
            .pet_extensions
            .lock()
            .map_err(|_| AppError::runtime("pet extensions mutex poisoned"))?
            .get(owner_user_id)
            .cloned()
        {
            return Ok(extension);
        }

        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        ensure_personal_pet_for_user(&connection, &workspace_id, owner_user_id)?;
        let agents = load_agents(&connection)?;
        let pet_extensions = load_pet_agent_extensions(&connection)?;
        let extension = pet_extensions
            .get(owner_user_id)
            .cloned()
            .ok_or_else(|| AppError::not_found("personal pet"))?;

        *self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))? = agents;
        *self
            .state
            .pet_extensions
            .lock()
            .map_err(|_| AppError::runtime("pet extensions mutex poisoned"))? = pet_extensions;
        Ok(extension)
    }

    pub(super) fn workspace_pet_snapshot(
        &self,
        owner_user_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        let extension = self.ensure_personal_pet_loaded(owner_user_id)?;
        let profile = default_pet_profile(&extension.pet_id, owner_user_id, &extension);
        let context_key = pet_context_key(owner_user_id, None);
        let presence = self
            .state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .get(&context_key)
            .cloned()
            .unwrap_or_else(|| default_workspace_pet_presence_for(&profile.id));
        let binding = self
            .state
            .pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("pet bindings mutex poisoned"))?
            .get(&context_key)
            .cloned();
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
            workspace_id: self.state.workspace_id()?,
            owner_user_id: owner_user_id.into(),
            context_scope: "home".into(),
            project_id: None,
            profile,
            presence,
            binding,
            messages,
        })
    }

    pub(super) fn project_pet_snapshot(
        &self,
        owner_user_id: &str,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.ensure_project_exists(project_id)?;
        let extension = self.ensure_personal_pet_loaded(owner_user_id)?;
        let profile = default_pet_profile(&extension.pet_id, owner_user_id, &extension);
        let context_key = pet_context_key(owner_user_id, Some(project_id));
        let presence = self
            .state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .get(&context_key)
            .cloned()
            .unwrap_or_else(|| default_workspace_pet_presence_for(&profile.id));
        let binding = self
            .state
            .pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("pet bindings mutex poisoned"))?
            .get(&context_key)
            .cloned();
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
            workspace_id: self.state.workspace_id()?,
            owner_user_id: owner_user_id.into(),
            context_scope: "project".into(),
            project_id: Some(project_id.into()),
            profile,
            presence,
            binding,
            messages,
        })
    }

    pub(super) fn persist_pet_presence(
        &self,
        owner_user_id: &str,
        project_id: Option<&str>,
        presence: &PetPresenceState,
    ) -> Result<(), AppError> {
        let scope_key = pet_context_key(owner_user_id, project_id);
        let context_scope = if project_id.is_some() {
            "project"
        } else {
            "home"
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO pet_presence (scope_key, owner_user_id, context_scope, project_id, pet_id, is_visible, chat_open, motion_state, unread_count, last_interaction_at, position_x, position_y)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                scope_key,
                owner_user_id,
                context_scope,
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
        owner_user_id: &str,
        project_id: Option<&str>,
        binding: &PetConversationBinding,
    ) -> Result<(), AppError> {
        let scope_key = pet_context_key(owner_user_id, project_id);
        let context_scope = if project_id.is_some() {
            "project"
        } else {
            "home"
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO pet_conversation_bindings (scope_key, owner_user_id, context_scope, project_id, pet_id, workspace_id, conversation_id, session_id, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                scope_key,
                owner_user_id,
                context_scope,
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
        let UpsertAgentInput {
            workspace_id,
            project_id,
            scope,
            name,
            avatar,
            remove_avatar,
            personality,
            tags,
            prompt,
            builtin_tool_keys,
            skill_ids,
            mcp_server_names,
            task_domains,
            default_model_strategy: input_default_model_strategy,
            capability_policy: input_capability_policy,
            permission_envelope: input_permission_envelope,
            memory_policy: input_memory_policy,
            delegation_policy: input_delegation_policy,
            approval_preference: input_approval_preference,
            output_contract: input_output_contract,
            shared_capability_policy: input_shared_capability_policy,
            description,
            status,
        } = input;

        let next_avatar_path = if remove_avatar.unwrap_or(false) {
            None
        } else if let Some(avatar) = avatar.as_ref() {
            Some(self.persist_workspace_avatar(agent_id, avatar)?.0)
        } else {
            current.and_then(|record| record.avatar_path.clone())
        };
        let avatar = agent_avatar(&self.state.paths, next_avatar_path.as_deref());
        let task_domains = normalize_task_domains(task_domains);

        Ok(AgentRecord {
            id: agent_id.into(),
            workspace_id: if workspace_id.trim().is_empty() {
                self.state.workspace_id()?
            } else {
                workspace_id
            },
            project_id,
            scope,
            owner_user_id: current.and_then(|record| record.owner_user_id.clone()),
            asset_role: current
                .map(|record| record.asset_role.clone())
                .unwrap_or_else(default_agent_asset_role),
            name: name.trim().into(),
            avatar_path: next_avatar_path,
            avatar,
            personality: personality.trim().into(),
            tags,
            prompt: prompt.trim().into(),
            builtin_tool_keys: builtin_tool_keys.clone(),
            skill_ids: skill_ids.clone(),
            mcp_server_names: mcp_server_names.clone(),
            task_domains,
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: input_default_model_strategy
                .unwrap_or_else(default_model_strategy),
            capability_policy: input_capability_policy.unwrap_or_else(|| {
                capability_policy_from_sources(&builtin_tool_keys, &skill_ids, &mcp_server_names)
            }),
            permission_envelope: input_permission_envelope
                .unwrap_or_else(default_permission_envelope),
            memory_policy: input_memory_policy.unwrap_or_else(default_agent_memory_policy),
            delegation_policy: input_delegation_policy
                .unwrap_or_else(default_agent_delegation_policy),
            approval_preference: input_approval_preference
                .unwrap_or_else(default_approval_preference),
            output_contract: input_output_contract.unwrap_or_else(default_output_contract),
            shared_capability_policy: input_shared_capability_policy
                .unwrap_or_else(default_agent_shared_capability_policy),
            integration_source: None,
            trust_metadata: default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: default_asset_import_metadata(),
            description: description.trim().into(),
            status: status.trim().into(),
            updated_at: Self::now(),
        })
    }

    pub(super) fn build_team_record(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
        current: Option<&TeamRecord>,
    ) -> Result<TeamRecord, AppError> {
        let UpsertTeamInput {
            workspace_id,
            project_id,
            scope,
            name,
            avatar,
            remove_avatar,
            personality,
            tags,
            prompt,
            builtin_tool_keys,
            skill_ids,
            mcp_server_names,
            task_domains,
            default_model_strategy: input_default_model_strategy,
            capability_policy: input_capability_policy,
            permission_envelope: input_permission_envelope,
            memory_policy: input_memory_policy,
            delegation_policy: input_delegation_policy,
            approval_preference: input_approval_preference,
            output_contract: input_output_contract,
            shared_capability_policy: input_shared_capability_policy,
            leader_ref: input_leader_ref,
            member_refs: input_member_refs,
            team_topology: input_team_topology,
            shared_memory_policy: input_shared_memory_policy,
            mailbox_policy: input_mailbox_policy,
            artifact_handoff_policy: input_artifact_handoff_policy,
            workflow_affordance: input_workflow_affordance,
            worker_concurrency_limit: input_worker_concurrency_limit,
            description,
            status,
        } = input;

        let next_avatar_path = if remove_avatar.unwrap_or(false) {
            None
        } else if let Some(avatar) = avatar.as_ref() {
            Some(self.persist_workspace_avatar(team_id, avatar)?.0)
        } else {
            current.and_then(|record| record.avatar_path.clone())
        };
        let avatar = agent_avatar(&self.state.paths, next_avatar_path.as_deref());

        let member_refs = crate::canonical_agent_refs(&input_member_refs);
        let leader_ref = crate::canonical_agent_ref(&input_leader_ref);
        if leader_ref.is_empty() {
            return Err(AppError::invalid_input("team leader_ref is required"));
        }
        let task_domains = normalize_task_domains(task_domains);
        let delegation_policy =
            input_delegation_policy.unwrap_or_else(default_team_delegation_policy);
        let worker_concurrency_limit =
            input_worker_concurrency_limit.unwrap_or(delegation_policy.max_worker_count);
        let team_topology = input_team_topology.unwrap_or_else(|| {
            team_topology_from_refs(Some(leader_ref.clone()), member_refs.clone())
        });
        let workflow_affordance = input_workflow_affordance.unwrap_or_else(|| {
            workflow_affordance_from_task_domains(
                &task_domains,
                delegation_policy.allow_background_runs,
                true,
            )
        });

        Ok(TeamRecord {
            id: team_id.into(),
            workspace_id: if workspace_id.trim().is_empty() {
                self.state.workspace_id()?
            } else {
                workspace_id
            },
            project_id,
            scope,
            name: name.trim().into(),
            avatar_path: next_avatar_path,
            avatar,
            personality: personality.trim().into(),
            tags,
            prompt: prompt.trim().into(),
            builtin_tool_keys: builtin_tool_keys.clone(),
            skill_ids: skill_ids.clone(),
            mcp_server_names: mcp_server_names.clone(),
            task_domains,
            manifest_revision: ASSET_MANIFEST_REVISION_V2.into(),
            default_model_strategy: input_default_model_strategy
                .unwrap_or_else(default_model_strategy),
            capability_policy: input_capability_policy.unwrap_or_else(|| {
                capability_policy_from_sources(&builtin_tool_keys, &skill_ids, &mcp_server_names)
            }),
            permission_envelope: input_permission_envelope
                .unwrap_or_else(default_permission_envelope),
            memory_policy: input_memory_policy.unwrap_or_else(default_team_memory_policy),
            delegation_policy: delegation_policy.clone(),
            approval_preference: input_approval_preference
                .unwrap_or_else(default_approval_preference),
            output_contract: input_output_contract.unwrap_or_else(default_output_contract),
            shared_capability_policy: input_shared_capability_policy
                .unwrap_or_else(default_team_shared_capability_policy),
            leader_ref: leader_ref.clone(),
            member_refs: member_refs.clone(),
            team_topology,
            shared_memory_policy: input_shared_memory_policy
                .unwrap_or_else(default_shared_memory_policy),
            mailbox_policy: input_mailbox_policy.unwrap_or_else(default_mailbox_policy),
            artifact_handoff_policy: input_artifact_handoff_policy
                .unwrap_or_else(default_artifact_handoff_policy),
            workflow_affordance,
            worker_concurrency_limit,
            integration_source: None,
            trust_metadata: default_asset_trust_metadata(),
            dependency_resolution: Vec::new(),
            import_metadata: default_asset_import_metadata(),
            description: description.trim().into(),
            status: status.trim().into(),
            updated_at: Self::now(),
        })
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
        let mapped_directory = normalize_mapped_directory_input(
            &self.state.paths,
            request.mapped_directory.as_deref(),
        )?;

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
            workspace_state.mapped_directory = mapped_directory;
            workspace_state.mapped_directory_default =
                Some(workspace_root_display_path(&self.state.paths));
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
impl AuthorizationService for InfraAuthorizationService {
    async fn authorize_request(
        &self,
        session: &SessionRecord,
        request: &AuthorizationRequest,
    ) -> Result<AuthorizationDecision, AppError> {
        fn requested_action(capability: &str) -> &str {
            capability.rsplit('.').next().unwrap_or(capability)
        }

        fn resource_type_matches(policy_type: &str, request_type: Option<&str>) -> bool {
            let Some(request_type) = request_type else {
                return policy_type == "project";
            };
            policy_type == request_type
                || (policy_type == "tool" && request_type.starts_with("tool."))
        }

        fn resource_policy_action_matches(
            policy_action: &str,
            requested_action: &str,
            capability: &str,
        ) -> bool {
            policy_action == "*" || policy_action == requested_action || policy_action == capability
        }

        let connection = self._state.open_db()?;
        let org_units = load_org_units(&connection)?;
        let _assignments =
            assignments_for_user(&load_user_org_assignments(&connection)?, &session.user_id);
        let (permission_codes, bindings) =
            resolve_effective_permission_codes(&connection, &session.user_id)?;
        let data_policies = resolve_subject_data_policies(&connection, &session.user_id)?;
        let all_resource_policies = load_resource_policies(&connection)?;
        let resource_policies = resolve_subject_resource_policies(&connection, &session.user_id)?;

        if !permission_codes
            .iter()
            .any(|code| code == &request.capability)
        {
            return Ok(AuthorizationDecision {
                allowed: false,
                reason: Some("no matching role permission".into()),
                matched_role_binding_ids: bindings.into_iter().map(|binding| binding.id).collect(),
                matched_policy_ids: data_policies
                    .into_iter()
                    .map(|policy| policy.id)
                    .chain(resource_policies.into_iter().map(|policy| policy.id))
                    .collect(),
            });
        }

        let matched_role_binding_ids = bindings
            .iter()
            .map(|binding| binding.id.clone())
            .collect::<Vec<_>>();
        let requested_action = requested_action(&request.capability);
        let request_resource_type = request.resource_type.as_deref();
        let request_project_id = request.project_id.as_deref();
        let request_classification = request.classification.as_deref();
        let request_owner_type = request.owner_subject_type.as_deref();
        let request_owner_id = request.owner_subject_id.as_deref();

        let owner_org_ancestor_ids = match (request_owner_type, request_owner_id) {
            (Some("org-unit"), Some(owner_org_unit_id))
            | (Some("org_unit"), Some(owner_org_unit_id)) => {
                org_unit_ancestor_ids(&org_units, owner_org_unit_id)
            }
            _ => BTreeSet::new(),
        };

        let data_policy_matches_scope = |policy: &DataPolicyRecord| match policy.scope_type.as_str()
        {
            "all" | "all-projects" => true,
            "selected-projects" => request_project_id
                .map(|project_id| {
                    policy
                        .project_ids
                        .iter()
                        .any(|candidate| candidate == project_id)
                })
                .unwrap_or(false),
            "org-unit-self" => matches!(
                (request_owner_type, request_owner_id),
                (Some("org-unit"), Some(owner_id)) | (Some("org_unit"), Some(owner_id))
                    if owner_id == policy.subject_id
            ),
            "org-unit-subtree" => {
                matches!(request_owner_type, Some("org-unit") | Some("org_unit"))
                    && owner_org_ancestor_ids.contains(&policy.subject_id)
            }
            "tag-match" => {
                !policy.tags.is_empty()
                    && policy
                        .tags
                        .iter()
                        .all(|tag| request.tags.iter().any(|candidate| candidate == tag))
            }
            _ => false,
        };

        let data_policy_matches = |policy: &DataPolicyRecord| {
            resource_type_matches(&policy.resource_type, request_resource_type)
                && (policy.classifications.is_empty()
                    || request_classification
                        .map(|classification| {
                            policy
                                .classifications
                                .iter()
                                .any(|candidate| candidate == classification)
                        })
                        .unwrap_or(false))
                && data_policy_matches_scope(policy)
        };

        let relevant_data_policies = data_policies
            .iter()
            .filter(|policy| resource_type_matches(&policy.resource_type, request_resource_type))
            .collect::<Vec<_>>();
        let matched_data_policies = relevant_data_policies
            .iter()
            .filter(|policy| data_policy_matches(policy))
            .collect::<Vec<_>>();
        let mut matched_policy_ids = matched_data_policies
            .iter()
            .map(|policy| policy.id.clone())
            .collect::<Vec<_>>();

        if matched_data_policies
            .iter()
            .any(|policy| policy.effect == "deny")
        {
            return Ok(AuthorizationDecision {
                allowed: false,
                reason: Some("data policy denied".into()),
                matched_role_binding_ids,
                matched_policy_ids,
            });
        }

        let has_domain_constraints = !relevant_data_policies.is_empty();
        let has_data_allow = matched_data_policies
            .iter()
            .any(|policy| policy.effect == "allow");
        if has_domain_constraints && !has_data_allow {
            return Ok(AuthorizationDecision {
                allowed: false,
                reason: Some("data policy allow missing".into()),
                matched_role_binding_ids,
                matched_policy_ids,
            });
        }

        if let (Some(resource_type), Some(resource_id)) = (
            request.resource_type.as_deref(),
            request.resource_id.as_deref(),
        ) {
            let relevant_resource_policies = all_resource_policies
                .iter()
                .filter(|policy| {
                    policy.resource_type == resource_type
                        && policy.resource_id == resource_id
                        && resource_policy_action_matches(
                            &policy.action,
                            requested_action,
                            &request.capability,
                        )
                })
                .collect::<Vec<_>>();
            let matching_resource_policies = resource_policies
                .iter()
                .filter(|policy| {
                    policy.resource_type == resource_type
                        && policy.resource_id == resource_id
                        && resource_policy_action_matches(
                            &policy.action,
                            requested_action,
                            &request.capability,
                        )
                })
                .collect::<Vec<_>>();

            matched_policy_ids.extend(
                relevant_resource_policies
                    .iter()
                    .map(|policy| policy.id.clone()),
            );

            if matching_resource_policies
                .iter()
                .any(|policy| policy.effect == "deny")
            {
                return Ok(AuthorizationDecision {
                    allowed: false,
                    reason: Some("resource access denied".into()),
                    matched_role_binding_ids,
                    matched_policy_ids,
                });
            }

            if !relevant_resource_policies.is_empty()
                && !matching_resource_policies
                    .iter()
                    .any(|policy| policy.effect == "allow")
            {
                return Ok(AuthorizationDecision {
                    allowed: false,
                    reason: Some("resource allow missing".into()),
                    matched_role_binding_ids,
                    matched_policy_ids,
                });
            }
        }

        Ok(AuthorizationDecision {
            allowed: true,
            reason: None,
            matched_role_binding_ids,
            matched_policy_ids,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_infra_bundle;
    use octopus_core::{
        AccessUserUpsertRequest, AuthorizationRequest, AvatarUploadPayload,
        DataPolicyUpsertRequest, LoginRequest, RegisterBootstrapAdminRequest,
        ResourcePolicyUpsertRequest, RoleBindingUpsertRequest, RoleUpsertRequest,
        DEFAULT_PROJECT_ID,
    };
    use octopus_platform::{
        AccessControlService, AuthService, AuthorizationService, WorkspaceService,
    };

    fn avatar_payload() -> AvatarUploadPayload {
        AvatarUploadPayload {
            content_type: "image/png".into(),
            data_base64: "iVBORw0KGgo=".into(),
            file_name: "avatar.png".into(),
            byte_size: 8,
        }
    }

    fn bootstrap_admin(bundle: &crate::InfraBundle) -> SessionRecord {
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime
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
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
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
    fn bootstrap_admin_adopts_seeded_default_project_membership() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let session = bootstrap_admin(&bundle);

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let project = bundle
                .workspace
                .list_projects()
                .await
                .expect("projects")
                .into_iter()
                .find(|record| record.id == DEFAULT_PROJECT_ID)
                .expect("default project");

            assert_eq!(project.owner_user_id, session.user_id);
            assert!(project
                .member_user_ids
                .iter()
                .any(|user_id| user_id == &session.user_id));
            assert!(!project
                .member_user_ids
                .iter()
                .any(|user_id| user_id == "user-owner"));
        });
    }

    #[test]
    fn default_project_seeds_model_assignments() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let project = bundle
                .workspace
                .list_projects()
                .await
                .expect("projects")
                .into_iter()
                .find(|record| record.id == DEFAULT_PROJECT_ID)
                .expect("default project");

            let models = project
                .assignments
                .as_ref()
                .and_then(|assignments| assignments.models.as_ref())
                .expect("default project model assignments");

            assert_eq!(models.default_configured_model_id, "claude-sonnet-4-5");
            assert!(models
                .configured_model_ids
                .iter()
                .any(|configured_model_id| configured_model_id == "claude-sonnet-4-5"));
        });
    }

    #[test]
    fn loading_existing_workspace_backfills_missing_default_project_model_assignments() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");

        let db = bundle.workspace.state.open_db().expect("open db");
        db.execute(
            "UPDATE projects SET assignments_json = NULL WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
        )
        .expect("clear default project assignments");

        let reloaded = build_infra_bundle(temp.path()).expect("reloaded bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let project = reloaded
                .workspace
                .list_projects()
                .await
                .expect("projects")
                .into_iter()
                .find(|record| record.id == DEFAULT_PROJECT_ID)
                .expect("default project");

            let models = project
                .assignments
                .as_ref()
                .and_then(|assignments| assignments.models.as_ref())
                .expect("backfilled default project model assignments");

            assert_eq!(models.default_configured_model_id, "claude-sonnet-4-5");
            assert!(models
                .configured_model_ids
                .iter()
                .any(|configured_model_id| configured_model_id == "claude-sonnet-4-5"));
        });
    }

    #[test]
    fn loading_existing_workspace_backfills_placeholder_project_membership_to_owner() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let session = bootstrap_admin(&bundle);

        let db = bundle.workspace.state.open_db().expect("open db");
        db.execute(
            "UPDATE projects SET owner_user_id = 'user-owner', member_user_ids_json = '[\"user-owner\"]' WHERE id = ?1",
            params![DEFAULT_PROJECT_ID],
        )
        .expect("reset project placeholder owner");

        let reloaded = build_infra_bundle(temp.path()).expect("reloaded bundle");
        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let project = reloaded
                .workspace
                .list_projects()
                .await
                .expect("projects")
                .into_iter()
                .find(|record| record.id == DEFAULT_PROJECT_ID)
                .expect("default project");

            assert_eq!(project.owner_user_id, session.user_id);
            assert_eq!(project.member_user_ids, vec![session.user_id.clone()]);
        });
    }

    #[test]
    fn bootstrap_admin_persists_requested_mapped_directory_in_workspace_summary() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let mapped_root = temp.path().to_string_lossy().to_string();

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let response = runtime
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
                        mapped_directory: Some(mapped_root.clone()),
                    }),
            )
            .expect("bootstrap admin");

        assert_eq!(
            response.workspace.mapped_directory.as_deref(),
            Some(mapped_root.as_str())
        );
        assert_eq!(
            response.workspace.mapped_directory_default.as_deref(),
            Some(mapped_root.as_str())
        );

        let saved = fs::read_to_string(temp.path().join("config").join("workspace.toml"))
            .expect("workspace config");
        assert!(saved.contains("mapped_directory"));
        assert!(saved.contains(mapped_root.as_str()));
    }

    #[test]
    fn current_user_profile_returns_stored_avatar_summary() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let session = bootstrap_admin(&bundle);

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        let profile = runtime
            .block_on(bundle.workspace.current_user_profile(&session.user_id))
            .expect("current profile");

        assert_eq!(profile.id, session.user_id);
        assert_eq!(
            profile.avatar.as_deref(),
            Some("data:image/png;base64,iVBORw0KGgo=")
        );
    }

    #[test]
    fn authorization_denies_when_object_has_policies_but_subject_has_no_allow_match() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let _owner_session = bootstrap_admin(&bundle);
        let analyst_session = create_user_session(&bundle, "analyst", "Analyst");
        let reviewer_session = create_user_session(&bundle, "reviewer", "Reviewer");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let role = bundle
                .access_control
                .create_role(RoleUpsertRequest {
                    code: "tool-mcp-operator".into(),
                    name: "Tool MCP Operator".into(),
                    description: "Can invoke MCP tools.".into(),
                    status: "active".into(),
                    permission_codes: vec!["tool.mcp.invoke".into()],
                })
                .await
                .expect("create role");

            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: role.id.clone(),
                    subject_type: "user".into(),
                    subject_id: analyst_session.user_id.clone(),
                    effect: "allow".into(),
                })
                .await
                .expect("bind analyst");

            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: role.id,
                    subject_type: "user".into(),
                    subject_id: reviewer_session.user_id.clone(),
                    effect: "allow".into(),
                })
                .await
                .expect("bind reviewer");

            bundle
                .access_control
                .create_resource_policy(ResourcePolicyUpsertRequest {
                    subject_type: "user".into(),
                    subject_id: reviewer_session.user_id.clone(),
                    resource_type: "tool.mcp".into(),
                    resource_id: "mcp-prod".into(),
                    action: "invoke".into(),
                    effect: "allow".into(),
                })
                .await
                .expect("resource policy");

            let decision = bundle
                .authorization
                .authorize_request(
                    &analyst_session,
                    &AuthorizationRequest {
                        subject_id: analyst_session.user_id.clone(),
                        capability: "tool.mcp.invoke".into(),
                        project_id: None,
                        resource_type: Some("tool.mcp".into()),
                        resource_id: Some("mcp-prod".into()),
                        resource_subtype: None,
                        tags: Vec::new(),
                        classification: Some("internal".into()),
                        owner_subject_type: None,
                        owner_subject_id: None,
                    },
                )
                .await
                .expect("decision");

            assert!(
                !decision.allowed,
                "object-scoped allow list should deny unmatched subject"
            );
        });
    }

    #[test]
    fn authorization_denies_when_tag_scoped_policy_exists_but_request_misses_allow_match() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let _owner_session = bootstrap_admin(&bundle);
        let analyst_session = create_user_session(&bundle, "tag-user", "Tag User");

        let runtime = tokio::runtime::Runtime::new().expect("runtime");
        runtime.block_on(async {
            let role = bundle
                .access_control
                .create_role(RoleUpsertRequest {
                    code: "resource-reader".into(),
                    name: "Resource Reader".into(),
                    description: "Can view protected resources.".into(),
                    status: "active".into(),
                    permission_codes: vec!["resource.view".into()],
                })
                .await
                .expect("create role");

            bundle
                .access_control
                .create_role_binding(RoleBindingUpsertRequest {
                    role_id: role.id,
                    subject_type: "user".into(),
                    subject_id: analyst_session.user_id.clone(),
                    effect: "allow".into(),
                })
                .await
                .expect("bind role");

            bundle
                .access_control
                .create_data_policy(DataPolicyUpsertRequest {
                    name: "confidential resources".into(),
                    subject_type: "user".into(),
                    subject_id: analyst_session.user_id.clone(),
                    resource_type: "resource".into(),
                    scope_type: "tag-match".into(),
                    project_ids: Vec::new(),
                    tags: vec!["confidential".into()],
                    classifications: Vec::new(),
                    effect: "allow".into(),
                })
                .await
                .expect("create data policy");

            let decision = bundle
                .authorization
                .authorize_request(
                    &analyst_session,
                    &AuthorizationRequest {
                        subject_id: analyst_session.user_id.clone(),
                        capability: "resource.view".into(),
                        project_id: Some("proj-alpha".into()),
                        resource_type: Some("resource".into()),
                        resource_id: Some("res-1".into()),
                        resource_subtype: None,
                        tags: vec!["public".into()],
                        classification: Some("internal".into()),
                        owner_subject_type: None,
                        owner_subject_id: None,
                    },
                )
                .await
                .expect("decision");

            assert!(
                !decision.allowed,
                "resource-scoped data policies should require an allow match when the domain is policy-controlled"
            );
        });
    }

    #[test]
    fn bootstrap_admin_backfills_missing_default_owner_permissions() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let db = bundle.auth.state.open_db().expect("db");
        db.execute(
            "UPDATE access_roles SET permission_codes = ?1 WHERE id = ?2",
            params![
                serde_json::to_string(&vec!["runtime.session.read", "custom.permission",])
                    .expect("owner permissions json"),
                SYSTEM_OWNER_ROLE_ID,
            ],
        )
        .expect("downgrade owner role");

        let _owner_session = bootstrap_admin(&bundle);

        let permission_codes_raw: String = db
            .query_row(
                "SELECT permission_codes FROM access_roles WHERE id = ?1",
                params![SYSTEM_OWNER_ROLE_ID],
                |row| row.get::<_, String>(0),
            )
            .expect("load owner role permissions");
        let permission_codes: Vec<String> =
            serde_json::from_str(&permission_codes_raw).expect("parse owner role permissions");

        assert!(
            permission_codes
                .iter()
                .any(|code| code == "runtime.submit_turn"),
            "bootstrap should backfill missing runtime submit permission"
        );
        assert!(
            permission_codes
                .iter()
                .any(|code| code == "custom.permission"),
            "bootstrap should preserve existing custom owner permissions"
        );
    }

    #[test]
    fn loading_existing_workspace_backfills_missing_default_owner_permissions() {
        let temp = tempfile::tempdir().expect("tempdir");
        let bundle = build_infra_bundle(temp.path()).expect("bundle");
        let _owner_session = bootstrap_admin(&bundle);
        let db = bundle.auth.state.open_db().expect("db");
        db.execute(
            "UPDATE access_roles SET permission_codes = ?1 WHERE id = ?2",
            params![
                serde_json::to_string(&vec!["runtime.session.read"])
                    .expect("legacy owner permissions json"),
                SYSTEM_OWNER_ROLE_ID
            ],
        )
        .expect("downgrade owner role");
        drop(db);
        drop(bundle);

        let reloaded_bundle = build_infra_bundle(temp.path()).expect("reloaded bundle");
        let reloaded_db = reloaded_bundle.auth.state.open_db().expect("reloaded db");
        let permission_codes_raw: String = reloaded_db
            .query_row(
                "SELECT permission_codes FROM access_roles WHERE id = ?1",
                params![SYSTEM_OWNER_ROLE_ID],
                |row| row.get::<_, String>(0),
            )
            .expect("load reloaded owner role permissions");
        let permission_codes: Vec<String> =
            serde_json::from_str(&permission_codes_raw).expect("parse reloaded owner permissions");

        assert!(
            permission_codes
                .iter()
                .any(|code| code == "runtime.submit_turn"),
            "loading an existing workspace should backfill missing runtime submit permission"
        );
    }
}
