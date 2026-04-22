use super::*;

impl InfraAccessControlService {
    pub(super) async fn list_roles_impl(&self) -> Result<Vec<AccessRoleRecord>, AppError> {
        let connection = self.state.open_db()?;
        ensure_system_roles(&connection)?;
        load_access_roles(&connection)
    }

    pub(super) async fn create_role_impl(
        &self,
        request: RoleUpsertRequest,
    ) -> Result<AccessRoleRecord, AppError> {
        reject_reserved_system_role_code(&request.code)?;
        let record = AccessRoleRecord {
            id: format!("role-{}", Uuid::new_v4()),
            code: request.code,
            name: request.name,
            description: request.description,
            status: request.status,
            permission_codes: request.permission_codes,
            source: "custom".into(),
            editable: true,
        };
        self.state
            .open_db()?
            .execute(
                "INSERT INTO access_roles (id, code, name, description, status, permission_codes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    record.id,
                    record.code,
                    record.name,
                    record.description,
                    record.status,
                    serde_json::to_string(&record.permission_codes)?,
                ],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    pub(super) async fn update_role_impl(
        &self,
        role_id: &str,
        request: RoleUpsertRequest,
    ) -> Result<AccessRoleRecord, AppError> {
        let connection = self.state.open_db()?;
        let existing_role = load_access_roles(&connection)?
            .into_iter()
            .find(|role| role.id == role_id)
            .ok_or_else(|| AppError::not_found("role"))?;
        if !existing_role.editable {
            return Err(AppError::invalid_input(
                "system roles are managed by platform and cannot be edited",
            ));
        }
        reject_reserved_system_role_code(&request.code)?;
        connection.execute(
            "UPDATE access_roles SET code = ?2, name = ?3, description = ?4, status = ?5, permission_codes = ?6 WHERE id = ?1",
            params![
                role_id,
                request.code,
                request.name,
                request.description,
                request.status,
                serde_json::to_string(&request.permission_codes)?,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(AccessRoleRecord {
            id: role_id.into(),
            code: request.code,
            name: request.name,
            description: request.description,
            status: request.status,
            permission_codes: request.permission_codes,
            source: "custom".into(),
            editable: true,
        })
    }

    pub(super) async fn delete_role_impl(&self, role_id: &str) -> Result<(), AppError> {
        let connection = self.state.open_db()?;
        let existing_role = load_access_roles(&connection)?
            .into_iter()
            .find(|role| role.id == role_id)
            .ok_or_else(|| AppError::not_found("role"))?;
        if !existing_role.editable {
            return Err(AppError::invalid_input(
                "system roles are managed by platform and cannot be deleted",
            ));
        }
        connection
            .execute("DELETE FROM access_roles WHERE id = ?1", params![role_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM role_bindings WHERE role_id = ?1",
                params![role_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_role_bindings_impl(&self) -> Result<Vec<RoleBindingRecord>, AppError> {
        load_role_bindings(&self.state.open_db()?)
    }

    pub(super) async fn create_role_binding_impl(
        &self,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError> {
        let record = RoleBindingRecord {
            id: format!("binding-{}", Uuid::new_v4()),
            role_id: request.role_id,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            effect: request.effect,
        };
        self.state.open_db()?.execute(
            "INSERT INTO role_bindings (id, role_id, subject_type, subject_id, effect) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![record.id, record.role_id, record.subject_type, record.subject_id, record.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    pub(super) async fn update_role_binding_impl(
        &self,
        binding_id: &str,
        request: RoleBindingUpsertRequest,
    ) -> Result<RoleBindingRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE role_bindings SET role_id = ?2, subject_type = ?3, subject_id = ?4, effect = ?5 WHERE id = ?1",
            params![binding_id, request.role_id, request.subject_type, request.subject_id, request.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(RoleBindingRecord {
            id: binding_id.into(),
            role_id: request.role_id,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            effect: request.effect,
        })
    }

    pub(super) async fn delete_role_binding_impl(&self, binding_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM role_bindings WHERE id = ?1",
                params![binding_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_data_policies_impl(&self) -> Result<Vec<DataPolicyRecord>, AppError> {
        load_data_policies(&self.state.open_db()?)
    }

    pub(super) async fn create_data_policy_impl(
        &self,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError> {
        let record = DataPolicyRecord {
            id: format!("data-policy-{}", Uuid::new_v4()),
            name: request.name,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            scope_type: request.scope_type,
            project_ids: request.project_ids,
            tags: request.tags,
            classifications: request.classifications,
            effect: request.effect,
        };
        self.state.open_db()?.execute(
            "INSERT INTO data_policies (id, name, subject_type, subject_id, resource_type, scope_type, project_ids, tags, classifications, effect)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                record.id,
                record.name,
                record.subject_type,
                record.subject_id,
                record.resource_type,
                record.scope_type,
                serde_json::to_string(&record.project_ids)?,
                serde_json::to_string(&record.tags)?,
                serde_json::to_string(&record.classifications)?,
                record.effect,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    pub(super) async fn update_data_policy_impl(
        &self,
        policy_id: &str,
        request: DataPolicyUpsertRequest,
    ) -> Result<DataPolicyRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE data_policies
             SET name = ?2, subject_type = ?3, subject_id = ?4, resource_type = ?5, scope_type = ?6, project_ids = ?7, tags = ?8, classifications = ?9, effect = ?10
             WHERE id = ?1",
            params![
                policy_id,
                request.name,
                request.subject_type,
                request.subject_id,
                request.resource_type,
                request.scope_type,
                serde_json::to_string(&request.project_ids)?,
                serde_json::to_string(&request.tags)?,
                serde_json::to_string(&request.classifications)?,
                request.effect,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(DataPolicyRecord {
            id: policy_id.into(),
            name: request.name,
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            scope_type: request.scope_type,
            project_ids: request.project_ids,
            tags: request.tags,
            classifications: request.classifications,
            effect: request.effect,
        })
    }

    pub(super) async fn delete_data_policy_impl(&self, policy_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM data_policies WHERE id = ?1",
                params![policy_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_resource_policies_impl(
        &self,
    ) -> Result<Vec<ResourcePolicyRecord>, AppError> {
        load_resource_policies(&self.state.open_db()?)
    }

    pub(super) async fn create_resource_policy_impl(
        &self,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError> {
        let record = ResourcePolicyRecord {
            id: format!("resource-policy-{}", Uuid::new_v4()),
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            resource_id: request.resource_id,
            action: request.action,
            effect: request.effect,
        };
        self.state.open_db()?.execute(
            "INSERT INTO resource_policies (id, subject_type, subject_id, resource_type, resource_id, action_name, effect)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![record.id, record.subject_type, record.subject_id, record.resource_type, record.resource_id, record.action, record.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(record)
    }

    pub(super) async fn update_resource_policy_impl(
        &self,
        policy_id: &str,
        request: ResourcePolicyUpsertRequest,
    ) -> Result<ResourcePolicyRecord, AppError> {
        self.state.open_db()?.execute(
            "UPDATE resource_policies
             SET subject_type = ?2, subject_id = ?3, resource_type = ?4, resource_id = ?5, action_name = ?6, effect = ?7
             WHERE id = ?1",
            params![policy_id, request.subject_type, request.subject_id, request.resource_type, request.resource_id, request.action, request.effect],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(ResourcePolicyRecord {
            id: policy_id.into(),
            subject_type: request.subject_type,
            subject_id: request.subject_id,
            resource_type: request.resource_type,
            resource_id: request.resource_id,
            action: request.action,
            effect: request.effect,
        })
    }

    pub(super) async fn delete_resource_policy_impl(
        &self,
        policy_id: &str,
    ) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM resource_policies WHERE id = ?1",
                params![policy_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_menu_policies_impl(&self) -> Result<Vec<MenuPolicyRecord>, AppError> {
        load_menu_policies(&self.state.open_db()?)
    }

    pub(super) async fn upsert_menu_policy_impl(
        &self,
        menu_id: &str,
        request: MenuPolicyUpsertRequest,
    ) -> Result<MenuPolicyRecord, AppError> {
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO menu_policies (menu_id, enabled, order_value, group_key, visibility)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![menu_id, bool_to_sql(request.enabled), request.order, request.group, request.visibility],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(MenuPolicyRecord {
            menu_id: menu_id.into(),
            enabled: request.enabled,
            order: request.order,
            group: request.group,
            visibility: request.visibility,
        })
    }

    pub(super) async fn delete_menu_policy_impl(&self, menu_id: &str) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM menu_policies WHERE menu_id = ?1",
                params![menu_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        Ok(())
    }

    pub(super) async fn list_protected_resources_impl(
        &self,
    ) -> Result<Vec<ProtectedResourceDescriptor>, AppError> {
        load_protected_resource_metadata(&self.state.open_db()?)
    }

    pub(super) async fn upsert_protected_resource_impl(
        &self,
        resource_type: &str,
        resource_id: &str,
        request: ProtectedResourceMetadataUpsertRequest,
    ) -> Result<ProtectedResourceDescriptor, AppError> {
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO protected_resources
             (resource_type, resource_id, resource_subtype, project_id, tags, classification, owner_subject_type, owner_subject_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                resource_type,
                resource_id,
                request.resource_subtype,
                request.project_id,
                serde_json::to_string(&request.tags)?,
                request.classification,
                request.owner_subject_type,
                request.owner_subject_id,
            ],
        ).map_err(|error| AppError::database(error.to_string()))?;
        Ok(ProtectedResourceDescriptor {
            id: resource_id.into(),
            resource_type: resource_type.into(),
            resource_subtype: request.resource_subtype,
            name: resource_id.into(),
            project_id: request.project_id,
            tags: request.tags,
            classification: request.classification,
            owner_subject_type: request.owner_subject_type,
            owner_subject_id: request.owner_subject_id,
        })
    }
}
