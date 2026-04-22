use super::*;

pub(crate) fn to_user_summary(paths: &WorkspacePaths, user: &StoredUser) -> UserRecordSummary {
    UserRecordSummary {
        id: user.record.id.clone(),
        username: user.record.username.clone(),
        display_name: user.record.display_name.clone(),
        avatar: avatar_data_url(paths, user),
        status: user.record.status.clone(),
        password_state: user.record.password_state.clone(),
    }
}

pub(crate) fn default_client_apps() -> Vec<ClientAppRecord> {
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

pub(crate) fn hash_password(password: &str) -> String {
    format!("plain::{password}")
}

pub(crate) fn verify_password(password: &str, hash: &str) -> bool {
    hash == hash_password(password)
}

pub(crate) fn append_json_line(path: &Path, value: &impl Serialize) -> Result<(), AppError> {
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
    pub(crate) fn now() -> u64 {
        timestamp_now()
    }

    pub(crate) fn ensure_project_exists(&self, project_id: &str) -> Result<(), AppError> {
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

    pub(crate) fn workspace_pet_snapshot(
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

    pub(crate) fn project_pet_snapshot(
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

    pub(crate) fn persist_pet_presence(
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

    pub(crate) fn persist_pet_binding(
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

    pub(crate) fn normalize_project_name(name: &str) -> Result<String, AppError> {
        let normalized = name.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input("project name is required"));
        }
        Ok(normalized.into())
    }

    pub(crate) fn normalize_project_description(description: &str) -> String {
        description.trim().into()
    }

    pub(crate) fn normalize_project_status(status: &str) -> Result<String, AppError> {
        match status.trim() {
            "active" => Ok("active".into()),
            "archived" => Ok("archived".into()),
            _ => Err(AppError::invalid_input(
                "project status must be active or archived",
            )),
        }
    }

    pub(crate) fn next_active_project_id(
        projects: &[ProjectRecord],
        current_project_id: &str,
    ) -> Option<String> {
        projects
            .iter()
            .find(|project| project.id != current_project_id && project.status == "active")
            .map(|project| project.id.clone())
    }

    pub(crate) fn replace_or_push<T, F>(items: &mut Vec<T>, value: T, matcher: F)
    where
        F: Fn(&T) -> bool,
    {
        if let Some(existing) = items.iter_mut().find(|item| matcher(item)) {
            *existing = value;
        } else {
            items.push(value);
        }
    }

    pub(crate) fn remove_avatar_file(&self, avatar_path: Option<&str>) -> Result<(), AppError> {
        let Some(avatar_path) = avatar_path else {
            return Ok(());
        };

        match fs::remove_file(self.state.paths.root.join(avatar_path)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    pub(crate) fn persist_workspace_avatar(
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

    pub(crate) fn persist_avatar(
        &self,
        user_id: &str,
        avatar: &AvatarUploadPayload,
    ) -> Result<(String, String, u64, String), AppError> {
        self.persist_workspace_avatar(user_id, avatar)
    }

    pub(crate) fn build_agent_record(
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

    pub(crate) fn build_team_record(
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
