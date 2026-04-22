use super::*;

impl InfraWorkspaceService {
    pub(crate) fn refresh_agent_and_team_caches(
        &self,
        connection: &Connection,
    ) -> Result<(), AppError> {
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

    pub(crate) fn normalize_resource_directory(&self, value: &str) -> Result<String, AppError> {
        let normalized = value.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input(
                "project resource directory is required",
            ));
        }
        let path = PathBuf::from(normalized);
        if path.is_absolute() {
            return Ok(self.display_storage_path(&path));
        }
        Ok(normalized.trim_end_matches('/').replace('\\', "/"))
    }

    pub(crate) fn normalize_resource_name(name: &str) -> Result<String, AppError> {
        let normalized = name.trim();
        if normalized.is_empty() {
            return Err(AppError::invalid_input("resource name is required"));
        }
        Ok(normalized.into())
    }

    pub(crate) fn normalize_resource_location(location: Option<String>) -> Option<String> {
        location.and_then(|value| {
            let normalized = value.trim().replace('\\', "/");
            if normalized.is_empty() {
                None
            } else {
                Some(normalized)
            }
        })
    }

    pub(crate) fn normalize_resource_scope(
        &self,
        project_id: Option<&str>,
        value: &str,
    ) -> Result<String, AppError> {
        let normalized = value.trim();
        let normalized = if normalized.is_empty() {
            if project_id.is_some() {
                "project"
            } else {
                "workspace"
            }
        } else {
            normalized
        };

        match normalized {
            "personal" => Ok("personal".into()),
            "project" if project_id.is_some() => Ok("project".into()),
            "workspace" => Ok("workspace".into()),
            "project" => Err(AppError::invalid_input(
                "workspace resources cannot use project scope",
            )),
            _ => Err(AppError::invalid_input("resource scope is invalid")),
        }
    }

    pub(crate) fn normalize_resource_visibility(&self, value: &str) -> Result<String, AppError> {
        match value.trim() {
            "" | "public" => Ok("public".into()),
            "private" => Ok("private".into()),
            _ => Err(AppError::invalid_input("resource visibility is invalid")),
        }
    }

    pub(crate) fn normalize_promoted_scope(
        &self,
        record: &WorkspaceResourceRecord,
        requested_scope: &str,
    ) -> Result<String, AppError> {
        let requested = requested_scope.trim();
        if requested.is_empty() || requested == record.scope {
            return Ok(record.scope.clone());
        }

        match (
            record.scope.as_str(),
            requested,
            record.project_id.is_some(),
        ) {
            ("personal", "project", true) => Ok("project".into()),
            ("personal", "workspace", false) | ("project", "workspace", _) => {
                Ok("workspace".into())
            }
            ("workspace", _, _) => Err(AppError::invalid_input(
                "workspace resources cannot be promoted further",
            )),
            _ => Err(AppError::invalid_input(
                "resource promotion scope is invalid",
            )),
        }
    }

    pub(crate) fn copy_agent_asset(
        &self,
        target: agent_assets::AssetTargetScope<'_>,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let files = if let Some(files) =
            crate::agent_bundle::extract_builtin_agent_template_files(agent_id)?
        {
            files
        } else {
            let source = load_agents(&connection)?
                .into_iter()
                .find(|record| record.id == agent_id)
                .ok_or_else(|| AppError::not_found("agent not found"))?;
            let source_scope = match source.project_id.as_deref() {
                Some(project_id) => agent_assets::AssetTargetScope::Project(project_id),
                None => agent_assets::AssetTargetScope::Workspace,
            };
            crate::agent_bundle::export_assets(
                &connection,
                &self.state.paths,
                &workspace_id,
                match source_scope {
                    agent_assets::AssetTargetScope::Workspace => {
                        crate::agent_bundle::BundleTarget::Workspace
                    }
                    agent_assets::AssetTargetScope::Project(project_id) => {
                        crate::agent_bundle::BundleTarget::Project(project_id)
                    }
                },
                ExportWorkspaceAgentBundleInput {
                    mode: "single".into(),
                    agent_ids: vec![agent_id.into()],
                    team_ids: Vec::new(),
                },
            )?
            .files
        };

        let result = crate::agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            match target {
                agent_assets::AssetTargetScope::Workspace => {
                    crate::agent_bundle::BundleTarget::Workspace
                }
                agent_assets::AssetTargetScope::Project(project_id) => {
                    crate::agent_bundle::BundleTarget::Project(project_id)
                }
            },
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    pub(crate) fn copy_team_asset(
        &self,
        target: agent_assets::AssetTargetScope<'_>,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let files = if let Some(files) =
            crate::agent_bundle::extract_builtin_team_template_files(team_id)?
        {
            files
        } else {
            let source = load_teams(&connection)?
                .into_iter()
                .find(|record| record.id == team_id)
                .ok_or_else(|| AppError::not_found("team not found"))?;
            let source_scope = match source.project_id.as_deref() {
                Some(project_id) => agent_assets::AssetTargetScope::Project(project_id),
                None => agent_assets::AssetTargetScope::Workspace,
            };
            crate::agent_bundle::export_assets(
                &connection,
                &self.state.paths,
                &workspace_id,
                match source_scope {
                    agent_assets::AssetTargetScope::Workspace => {
                        crate::agent_bundle::BundleTarget::Workspace
                    }
                    agent_assets::AssetTargetScope::Project(project_id) => {
                        crate::agent_bundle::BundleTarget::Project(project_id)
                    }
                },
                ExportWorkspaceAgentBundleInput {
                    mode: "single".into(),
                    agent_ids: Vec::new(),
                    team_ids: vec![team_id.into()],
                },
            )?
            .files
        };

        let result = crate::agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            match target {
                agent_assets::AssetTargetScope::Workspace => {
                    crate::agent_bundle::BundleTarget::Workspace
                }
                agent_assets::AssetTargetScope::Project(project_id) => {
                    crate::agent_bundle::BundleTarget::Project(project_id)
                }
            },
            ImportWorkspaceAgentBundleInput { files },
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }
}
