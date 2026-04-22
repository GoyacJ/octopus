use super::*;

impl InfraWorkspaceService {
    pub(crate) async fn list_agents_impl(&self) -> Result<Vec<AgentRecord>, AppError> {
        let workspace_id = self.state.workspace_id()?;
        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?
            .iter()
            .filter(|record| record.asset_role != "pet")
            .cloned()
            .collect::<Vec<_>>();
        agents.extend(crate::agent_bundle::list_builtin_agent_templates(
            &workspace_id,
        )?);
        agents.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(agents)
    }

    pub(crate) async fn create_agent_impl(
        &self,
        input: UpsertAgentInput,
    ) -> Result<AgentRecord, AppError> {
        let agent_id = format!("agent-{}", Uuid::new_v4());
        let record = self.build_agent_record(&agent_id, input, None)?;

        write_agent_record(&self.state.open_db()?, &record, false)?;

        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        agents.push(record.clone());
        Ok(record)
    }

    pub(crate) async fn update_agent_impl(
        &self,
        agent_id: &str,
        input: UpsertAgentInput,
    ) -> Result<AgentRecord, AppError> {
        let current = {
            self.state
                .agents
                .lock()
                .map_err(|_| AppError::runtime("agents mutex poisoned"))?
                .iter()
                .find(|item| item.id == agent_id)
                .cloned()
        };
        let record = self.build_agent_record(agent_id, input, current.as_ref())?;

        write_agent_record(&self.state.open_db()?, &record, true)?;

        if let Some(previous) = current.as_ref() {
            if previous.avatar_path != record.avatar_path {
                self.remove_avatar_file(previous.avatar_path.as_deref())?;
            }
        }

        let mut agents = self
            .state
            .agents
            .lock()
            .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
        Self::replace_or_push(&mut agents, record.clone(), |item| item.id == agent_id);
        Ok(record)
    }

    pub(crate) async fn delete_agent_impl(&self, agent_id: &str) -> Result<(), AppError> {
        let removed = {
            let mut agents = self
                .state
                .agents
                .lock()
                .map_err(|_| AppError::runtime("agents mutex poisoned"))?;
            let existing = agents.iter().find(|item| item.id == agent_id).cloned();
            agents.retain(|item| item.id != agent_id);
            existing
        };

        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM agents WHERE id = ?1", params![agent_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM project_agent_links WHERE agent_id = ?1",
                params![agent_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .retain(|item| item.agent_id != agent_id);

        if let Some(record) = removed {
            self.remove_avatar_file(record.avatar_path.as_deref())?;
        }
        Ok(())
    }

    pub(crate) async fn preview_import_agent_bundle_impl(
        &self,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::preview_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Workspace,
            input,
        )
    }

    pub(crate) async fn import_agent_bundle_impl(
        &self,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Workspace,
            input,
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    pub(crate) async fn copy_workspace_agent_from_builtin_impl(
        &self,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_agent_asset(agent_assets::AssetTargetScope::Workspace, agent_id)
    }

    pub(crate) async fn export_agent_bundle_impl(
        &self,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::export_assets(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Workspace,
            input,
        )
    }

    pub(crate) async fn preview_import_project_agent_bundle_impl(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::preview_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Project(project_id),
            input,
        )
    }

    pub(crate) async fn import_project_agent_bundle_impl(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        let result = agent_bundle::execute_import(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Project(project_id),
            input,
        )?;
        self.refresh_agent_and_team_caches(&connection)?;
        Ok(result)
    }

    pub(crate) async fn copy_project_agent_from_builtin_impl(
        &self,
        project_id: &str,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_agent_asset(
            agent_assets::AssetTargetScope::Project(project_id),
            agent_id,
        )
    }

    pub(crate) async fn export_project_agent_bundle_impl(
        &self,
        project_id: &str,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        let connection = self.state.open_db()?;
        let workspace_id = self.state.workspace_id()?;
        agent_bundle::export_assets(
            &connection,
            &self.state.paths,
            &workspace_id,
            agent_bundle::BundleTarget::Project(project_id),
            input,
        )
    }

    pub(crate) async fn list_project_agent_links_impl(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAgentLinkRecord>, AppError> {
        Ok(self
            .state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    pub(crate) async fn link_project_agent_impl(
        &self,
        input: ProjectAgentLinkInput,
    ) -> Result<ProjectAgentLinkRecord, AppError> {
        let record = ProjectAgentLinkRecord {
            workspace_id: self.state.workspace_id()?,
            project_id: input.project_id,
            agent_id: input.agent_id,
            linked_at: Self::now(),
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO project_agent_links (workspace_id, project_id, agent_id, linked_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.workspace_id, record.project_id, record.agent_id, record.linked_at as i64],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut links = self
            .state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?;
        Self::replace_or_push(&mut links, record.clone(), |item| {
            item.project_id == record.project_id && item.agent_id == record.agent_id
        });
        Ok(record)
    }

    pub(crate) async fn unlink_project_agent_impl(
        &self,
        project_id: &str,
        agent_id: &str,
    ) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM project_agent_links WHERE project_id = ?1 AND agent_id = ?2",
                params![project_id, agent_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_agent_links
            .lock()
            .map_err(|_| AppError::runtime("project agent links mutex poisoned"))?
            .retain(|item| !(item.project_id == project_id && item.agent_id == agent_id));
        Ok(())
    }

    pub(crate) async fn list_teams_impl(&self) -> Result<Vec<TeamRecord>, AppError> {
        let workspace_id = self.state.workspace_id()?;
        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?
            .clone();
        teams.extend(crate::agent_bundle::list_builtin_team_templates(
            &workspace_id,
        )?);
        teams.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(teams)
    }

    pub(crate) async fn create_team_impl(
        &self,
        input: UpsertTeamInput,
    ) -> Result<TeamRecord, AppError> {
        let team_id = format!("team-{}", Uuid::new_v4());
        let record = self.build_team_record(&team_id, input, None)?;

        write_team_record(&self.state.open_db()?, &record, false)?;

        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        teams.push(record.clone());
        Ok(record)
    }

    pub(crate) async fn update_team_impl(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
    ) -> Result<TeamRecord, AppError> {
        let current = {
            self.state
                .teams
                .lock()
                .map_err(|_| AppError::runtime("teams mutex poisoned"))?
                .iter()
                .find(|item| item.id == team_id)
                .cloned()
        };
        let record = self.build_team_record(team_id, input, current.as_ref())?;

        write_team_record(&self.state.open_db()?, &record, true)?;

        if let Some(previous) = current.as_ref() {
            if previous.avatar_path != record.avatar_path {
                self.remove_avatar_file(previous.avatar_path.as_deref())?;
            }
        }

        let mut teams = self
            .state
            .teams
            .lock()
            .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
        Self::replace_or_push(&mut teams, record.clone(), |item| item.id == team_id);
        Ok(record)
    }

    pub(crate) async fn delete_team_impl(&self, team_id: &str) -> Result<(), AppError> {
        let removed = {
            let mut teams = self
                .state
                .teams
                .lock()
                .map_err(|_| AppError::runtime("teams mutex poisoned"))?;
            let existing = teams.iter().find(|item| item.id == team_id).cloned();
            teams.retain(|item| item.id != team_id);
            existing
        };

        let connection = self.state.open_db()?;
        connection
            .execute("DELETE FROM teams WHERE id = ?1", params![team_id])
            .map_err(|error| AppError::database(error.to_string()))?;
        connection
            .execute(
                "DELETE FROM project_team_links WHERE team_id = ?1",
                params![team_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .retain(|item| item.team_id != team_id);

        if let Some(record) = removed {
            self.remove_avatar_file(record.avatar_path.as_deref())?;
        }
        Ok(())
    }

    pub(crate) async fn copy_workspace_team_from_builtin_impl(
        &self,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_team_asset(agent_assets::AssetTargetScope::Workspace, team_id)
    }

    pub(crate) async fn copy_project_team_from_builtin_impl(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_team_asset(agent_assets::AssetTargetScope::Project(project_id), team_id)
    }

    pub(crate) async fn list_project_team_links_impl(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectTeamLinkRecord>, AppError> {
        Ok(self
            .state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .iter()
            .filter(|record| record.project_id == project_id)
            .cloned()
            .collect())
    }

    pub(crate) async fn link_project_team_impl(
        &self,
        input: ProjectTeamLinkInput,
    ) -> Result<ProjectTeamLinkRecord, AppError> {
        let record = ProjectTeamLinkRecord {
            workspace_id: self.state.workspace_id()?,
            project_id: input.project_id,
            team_id: input.team_id,
            linked_at: Self::now(),
        };
        self.state.open_db()?.execute(
            "INSERT OR REPLACE INTO project_team_links (workspace_id, project_id, team_id, linked_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![record.workspace_id, record.project_id, record.team_id, record.linked_at as i64],
        ).map_err(|error| AppError::database(error.to_string()))?;

        let mut links = self
            .state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?;
        Self::replace_or_push(&mut links, record.clone(), |item| {
            item.project_id == record.project_id && item.team_id == record.team_id
        });
        Ok(record)
    }

    pub(crate) async fn unlink_project_team_impl(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<(), AppError> {
        self.state
            .open_db()?
            .execute(
                "DELETE FROM project_team_links WHERE project_id = ?1 AND team_id = ?2",
                params![project_id, team_id],
            )
            .map_err(|error| AppError::database(error.to_string()))?;
        self.state
            .project_team_links
            .lock()
            .map_err(|_| AppError::runtime("project team links mutex poisoned"))?
            .retain(|item| !(item.project_id == project_id && item.team_id == team_id));
        Ok(())
    }
}
