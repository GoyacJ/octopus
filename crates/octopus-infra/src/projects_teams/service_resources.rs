use super::*;

impl InfraWorkspaceService {
    pub(crate) async fn list_workspace_resources_impl(
        &self,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        Ok(self
            .state
            .resources
            .lock()
            .map_err(|_| AppError::runtime("resources mutex poisoned"))?
            .clone())
    }

    pub(crate) async fn list_project_resources_impl(
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

    pub(crate) async fn create_workspace_resource_impl(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let record = self.build_metadata_resource_record(workspace_id, owner_user_id, input)?;
        self.persist_resource_record(&record, false)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    pub(crate) async fn create_project_resource_impl(
        &self,
        project_id: &str,
        owner_user_id: &str,
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
        self.create_workspace_resource(&workspace_id, owner_user_id, input)
            .await
    }

    pub(crate) async fn create_project_resource_folder_impl(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceFolderInput,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        if input.files.is_empty() {
            return Ok(Vec::new());
        }

        let root_dir_name = self.infer_folder_root_name(&input.files);
        let files = self.trim_folder_root_prefix(root_dir_name.as_deref(), input.files)?;
        let record = self
            .import_project_resource(
                project_id,
                owner_user_id,
                WorkspaceResourceImportInput {
                    name: root_dir_name
                        .clone()
                        .or_else(|| files.first().map(|entry| entry.file_name.clone()))
                        .unwrap_or_else(|| "uploaded-folder".into()),
                    root_dir_name,
                    scope: "project".into(),
                    visibility: "public".into(),
                    tags: None,
                    files,
                },
            )
            .await?;
        Ok(vec![record])
    }

    pub(crate) async fn import_workspace_resource_impl(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.ensure_import_has_files(&input.files)?;
        let scope = self.normalize_resource_scope(None, &input.scope)?;
        let visibility = self.normalize_resource_visibility(&input.visibility)?;
        let imported = self.write_imported_resource(
            workspace_id,
            None,
            owner_user_id,
            scope,
            visibility,
            input,
            &self.state.paths.workspace_resources_dir,
        )?;
        self.persist_resource_record(&imported, false)?;
        self.upsert_resource_cache(imported.clone())?;
        Ok(imported)
    }

    pub(crate) async fn import_project_resource_impl(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.ensure_project_exists(project_id)?;
        self.ensure_import_has_files(&input.files)?;
        let project = self.project_record(project_id)?;
        let scope = self.normalize_resource_scope(Some(project_id), &input.scope)?;
        let visibility = self.normalize_resource_visibility(&input.visibility)?;
        let target_directory = self.resolve_storage_path(&project.resource_directory);
        fs::create_dir_all(&target_directory)?;
        let imported = self.write_imported_resource(
            &project.workspace_id,
            Some(project_id),
            owner_user_id,
            scope,
            visibility,
            input,
            &target_directory,
        )?;
        self.persist_resource_record(&imported, false)?;
        self.upsert_resource_cache(imported.clone())?;
        Ok(imported)
    }

    pub(crate) async fn get_resource_detail_impl(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.resource_record(resource_id)
    }

    pub(crate) async fn get_resource_content_impl(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceContentDocument, AppError> {
        let record = self.resource_record(resource_id)?;
        let content_type = record
            .content_type
            .clone()
            .or_else(|| Self::resource_content_type(&record.name, record.location.as_deref()));

        if record.preview_kind == "url" {
            return Ok(WorkspaceResourceContentDocument {
                resource_id: record.id,
                preview_kind: record.preview_kind,
                file_name: Some(record.name),
                content_type,
                external_url: record.location,
                text_content: None,
                data_base64: None,
                byte_size: record.byte_size,
            });
        }

        if record.preview_kind == "folder" {
            return Ok(WorkspaceResourceContentDocument {
                resource_id: record.id,
                preview_kind: record.preview_kind,
                file_name: Some(record.name),
                content_type,
                external_url: None,
                text_content: None,
                data_base64: None,
                byte_size: record.byte_size,
            });
        }

        let Some(storage_path) = record.storage_path.as_deref() else {
            return Ok(WorkspaceResourceContentDocument {
                resource_id: record.id,
                preview_kind: record.preview_kind,
                file_name: Some(record.name),
                content_type,
                external_url: None,
                text_content: None,
                data_base64: None,
                byte_size: record.byte_size,
            });
        };

        let absolute_path = self.resolve_storage_path(storage_path);
        let bytes = fs::read(&absolute_path)?;
        let byte_size = Some(bytes.len() as u64);
        let text_content = match record.preview_kind.as_str() {
            "text" | "code" | "markdown" => Some(String::from_utf8_lossy(&bytes).into_owned()),
            _ => None,
        };
        let data_base64 = match record.preview_kind.as_str() {
            "text" | "code" | "markdown" => None,
            _ => Some(BASE64_STANDARD.encode(bytes)),
        };

        Ok(WorkspaceResourceContentDocument {
            resource_id: record.id,
            preview_kind: record.preview_kind,
            file_name: Some(record.name),
            content_type,
            external_url: None,
            text_content,
            data_base64,
            byte_size,
        })
    }

    pub(crate) async fn list_resource_children_impl(
        &self,
        resource_id: &str,
    ) -> Result<Vec<WorkspaceResourceChildrenRecord>, AppError> {
        let record = self.resource_record(resource_id)?;
        if record.preview_kind != "folder" {
            return Ok(Vec::new());
        }
        let Some(storage_path) = record.storage_path.as_deref() else {
            return Ok(Vec::new());
        };
        let absolute_path = self.resolve_storage_path(storage_path);
        if !absolute_path.exists() || !absolute_path.is_dir() {
            return Ok(Vec::new());
        }
        let mut children = Vec::new();
        Self::collect_resource_children(&absolute_path, &absolute_path, &mut children)?;
        children.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
        Ok(children)
    }

    pub(crate) async fn promote_resource_impl(
        &self,
        resource_id: &str,
        input: PromoteWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut record = self.resource_record(resource_id)?;
        let next_scope = self.normalize_promoted_scope(&record, &input.scope)?;
        record.scope = next_scope;
        record.updated_at = timestamp_now();
        self.persist_resource_record(&record, true)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    pub(crate) async fn list_directories_impl(
        &self,
        path: Option<&str>,
    ) -> Result<WorkspaceDirectoryBrowserResponse, AppError> {
        let current_path = path
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| self.resolve_storage_path(value))
            .unwrap_or_else(|| self.state.paths.root.clone());
        if !current_path.exists() {
            return Err(AppError::not_found("directory not found"));
        }
        if !current_path.is_dir() {
            return Err(AppError::invalid_input("path is not a directory"));
        }

        let mut entries = fs::read_dir(&current_path)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let file_type = entry.file_type().ok()?;
                if !file_type.is_dir() {
                    return None;
                }
                let path = entry.path();
                Some(WorkspaceDirectoryBrowserEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    path: self.display_storage_path(&path),
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.name.cmp(&right.name).then(left.path.cmp(&right.path)));

        let parent_path = current_path.parent().and_then(|parent| {
            if parent.starts_with(&self.state.paths.root) {
                Some(self.display_storage_path(parent))
            } else {
                None
            }
        });

        Ok(WorkspaceDirectoryBrowserResponse {
            current_path: self.display_storage_path(&current_path),
            parent_path: if current_path == self.state.paths.root {
                None
            } else {
                parent_path
            },
            entries,
        })
    }

    pub(crate) async fn delete_workspace_resource_impl(
        &self,
        workspace_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let record = self.resource_record(resource_id)?;
        if record.workspace_id != workspace_id {
            return Err(AppError::not_found(format!(
                "resource {} not found in workspace {}",
                resource_id, workspace_id
            )));
        }
        self.delete_managed_resource_storage(&record)?;
        self.delete_resource_record(resource_id)?;
        Ok(())
    }

    pub(crate) async fn delete_project_resource_impl(
        &self,
        project_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        let record = self.resource_record(resource_id)?;
        if record.project_id.as_deref() != Some(project_id) {
            return Err(AppError::not_found(format!(
                "resource {} not found in project {}",
                resource_id, project_id
            )));
        }
        self.delete_managed_resource_storage(&record)?;
        self.delete_resource_record(resource_id)?;
        Ok(())
    }

    pub(crate) async fn update_workspace_resource_impl(
        &self,
        workspace_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut record = self.resource_record(resource_id)?;
        if record.workspace_id != workspace_id {
            return Err(AppError::not_found(format!(
                "resource {} not found in workspace {}",
                resource_id, workspace_id
            )));
        }
        self.apply_resource_update(&mut record, input)?;
        self.persist_resource_record(&record, true)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    pub(crate) async fn update_project_resource_impl(
        &self,
        project_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        let mut record = self.resource_record(resource_id)?;
        if record.project_id.as_deref() != Some(project_id) {
            return Err(AppError::not_found(format!(
                "resource {} not found in project {}",
                resource_id, project_id
            )));
        }
        self.apply_resource_update(&mut record, input)?;
        self.persist_resource_record(&record, true)?;
        self.upsert_resource_cache(record.clone())?;
        Ok(record)
    }

    pub(crate) async fn list_workspace_knowledge_impl(
        &self,
    ) -> Result<Vec<KnowledgeRecord>, AppError> {
        Ok(self
            .state
            .knowledge_records
            .lock()
            .map_err(|_| AppError::runtime("knowledge mutex poisoned"))?
            .clone())
    }

    pub(crate) async fn list_project_knowledge_impl(
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

    pub(crate) async fn get_workspace_pet_snapshot_impl(
        &self,
        owner_user_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.workspace_pet_snapshot(owner_user_id)
    }

    pub(crate) async fn get_project_pet_snapshot_impl(
        &self,
        owner_user_id: &str,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.project_pet_snapshot(owner_user_id, project_id)
    }

    pub(crate) async fn save_workspace_pet_presence_impl(
        &self,
        owner_user_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        let snapshot = self.workspace_pet_snapshot(owner_user_id)?;
        let mut presence = self
            .state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .get(&pet_context_key(owner_user_id, None))
            .cloned()
            .unwrap_or(snapshot.presence);
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
        self.persist_pet_presence(owner_user_id, None, &presence)?;
        self.state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .insert(pet_context_key(owner_user_id, None), presence.clone());
        Ok(presence)
    }

    pub(crate) async fn save_project_pet_presence_impl(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        self.ensure_project_exists(project_id)?;
        let snapshot = self.project_pet_snapshot(owner_user_id, project_id)?;
        let mut presence = self
            .state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .get(&pet_context_key(owner_user_id, Some(project_id)))
            .cloned()
            .unwrap_or(snapshot.presence);
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
        self.persist_pet_presence(owner_user_id, Some(project_id), &presence)?;
        self.state
            .pet_presences
            .lock()
            .map_err(|_| AppError::runtime("pet presences mutex poisoned"))?
            .insert(
                pet_context_key(owner_user_id, Some(project_id)),
                presence.clone(),
            );
        Ok(presence)
    }

    pub(crate) async fn bind_workspace_pet_conversation_impl(
        &self,
        owner_user_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        let snapshot = self.workspace_pet_snapshot(owner_user_id)?;
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                snapshot.profile.id
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            owner_user_id: owner_user_id.into(),
            context_scope: "home".into(),
            project_id: None,
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(owner_user_id, None, &binding)?;
        self.state
            .pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("pet bindings mutex poisoned"))?
            .insert(pet_context_key(owner_user_id, None), binding.clone());
        Ok(binding)
    }

    pub(crate) async fn bind_project_pet_conversation_impl(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        self.ensure_project_exists(project_id)?;
        let snapshot = self.project_pet_snapshot(owner_user_id, project_id)?;
        let binding = PetConversationBinding {
            pet_id: if input.pet_id.trim().is_empty() {
                snapshot.profile.id
            } else {
                input.pet_id
            },
            workspace_id: self.state.workspace_id()?,
            owner_user_id: owner_user_id.into(),
            context_scope: "project".into(),
            project_id: Some(project_id.into()),
            conversation_id: input.conversation_id,
            session_id: input.session_id,
            updated_at: Self::now(),
        };
        self.persist_pet_binding(owner_user_id, Some(project_id), &binding)?;
        self.state
            .pet_bindings
            .lock()
            .map_err(|_| AppError::runtime("pet bindings mutex poisoned"))?
            .insert(
                pet_context_key(owner_user_id, Some(project_id)),
                binding.clone(),
            );
        Ok(binding)
    }
}
