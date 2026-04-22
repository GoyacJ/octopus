use super::*;
use octopus_core::CapabilityManagementProjection;

const PROJECT_DELETION_REQUEST_INBOX_ITEM_TYPE: &str = "project-deletion-request";
const PROJECT_DELETION_REQUEST_ACTION_LABEL: &str = "Review approval";
use crate::project_tasks::{
    load_project_task_interventions, load_project_task_runs, load_project_task_scheduler_claims,
    load_project_tasks,
};

#[path = "projects_teams/helpers_asset_copy.rs"]
mod helpers_asset_copy;
#[path = "projects_teams/helpers_project_persistence.rs"]
mod helpers_project_persistence;
#[path = "projects_teams/helpers_resource_io.rs"]
mod helpers_resource_io;
#[path = "projects_teams/service_agents_teams.rs"]
mod service_agents_teams;
#[path = "projects_teams/service_projects.rs"]
mod service_projects;
#[path = "projects_teams/service_resources.rs"]
mod service_resources;
#[path = "projects_teams/service_workspace_admin.rs"]
mod service_workspace_admin;

#[cfg(test)]
mod tests;

#[async_trait]
impl WorkspaceService for InfraWorkspaceService {
    async fn system_bootstrap(&self) -> Result<SystemBootstrapStatus, AppError> {
        self.system_bootstrap_impl().await
    }

    async fn workspace_summary(&self) -> Result<WorkspaceSummary, AppError> {
        self.workspace_summary_impl().await
    }

    async fn update_workspace(
        &self,
        request: UpdateWorkspaceRequest,
    ) -> Result<WorkspaceSummary, AppError> {
        self.update_workspace_impl(request).await
    }

    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError> {
        self.list_projects_impl().await
    }

    async fn list_project_deliverables(
        &self,
        project_id: &str,
    ) -> Result<Vec<ArtifactRecord>, AppError> {
        self.list_project_deliverables_impl(project_id).await
    }

    async fn create_project(
        &self,
        request: CreateProjectRequest,
    ) -> Result<ProjectRecord, AppError> {
        self.create_project_impl(request).await
    }

    async fn update_project(
        &self,
        project_id: &str,
        request: UpdateProjectRequest,
    ) -> Result<ProjectRecord, AppError> {
        self.update_project_impl(project_id, request).await
    }

    async fn list_project_promotion_requests(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectPromotionRequest>, AppError> {
        self.list_project_promotion_requests_impl(project_id).await
    }

    async fn list_workspace_promotion_requests(
        &self,
    ) -> Result<Vec<ProjectPromotionRequest>, AppError> {
        self.list_workspace_promotion_requests_impl().await
    }

    async fn list_project_deletion_requests(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectDeletionRequest>, AppError> {
        self.list_project_deletion_requests_impl(project_id).await
    }

    async fn create_project_promotion_request(
        &self,
        project_id: &str,
        requested_by_user_id: &str,
        input: CreateProjectPromotionRequestInput,
    ) -> Result<ProjectPromotionRequest, AppError> {
        self.create_project_promotion_request_impl(project_id, requested_by_user_id, input)
            .await
    }

    async fn create_project_deletion_request(
        &self,
        project_id: &str,
        requested_by_user_id: &str,
        input: CreateProjectDeletionRequestInput,
    ) -> Result<ProjectDeletionRequest, AppError> {
        self.create_project_deletion_request_impl(project_id, requested_by_user_id, input)
            .await
    }

    async fn review_project_promotion_request(
        &self,
        request_id: &str,
        reviewed_by_user_id: &str,
        input: ReviewProjectPromotionRequestInput,
    ) -> Result<ProjectPromotionRequest, AppError> {
        self.review_project_promotion_request_impl(request_id, reviewed_by_user_id, input)
            .await
    }

    async fn review_project_deletion_request(
        &self,
        request_id: &str,
        reviewed_by_user_id: &str,
        approved: bool,
        input: ReviewProjectDeletionRequestInput,
    ) -> Result<ProjectDeletionRequest, AppError> {
        self.review_project_deletion_request_impl(request_id, reviewed_by_user_id, approved, input)
            .await
    }

    async fn delete_project(&self, project_id: &str) -> Result<(), AppError> {
        self.delete_project_impl(project_id).await
    }

    async fn list_workspace_resources(&self) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        self.list_workspace_resources_impl().await
    }

    async fn list_project_resources(
        &self,
        project_id: &str,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        self.list_project_resources_impl(project_id).await
    }

    async fn create_workspace_resource(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.create_workspace_resource_impl(workspace_id, owner_user_id, input)
            .await
    }

    async fn create_project_resource(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.create_project_resource_impl(project_id, owner_user_id, input)
            .await
    }

    async fn create_project_resource_folder(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceFolderInput,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError> {
        self.create_project_resource_folder_impl(project_id, owner_user_id, input)
            .await
    }

    async fn import_workspace_resource(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.import_workspace_resource_impl(workspace_id, owner_user_id, input)
            .await
    }

    async fn import_project_resource(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.import_project_resource_impl(project_id, owner_user_id, input)
            .await
    }

    async fn get_resource_detail(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.get_resource_detail_impl(resource_id).await
    }

    async fn get_resource_content(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceContentDocument, AppError> {
        self.get_resource_content_impl(resource_id).await
    }

    async fn list_resource_children(
        &self,
        resource_id: &str,
    ) -> Result<Vec<WorkspaceResourceChildrenRecord>, AppError> {
        self.list_resource_children_impl(resource_id).await
    }

    async fn promote_resource(
        &self,
        resource_id: &str,
        input: PromoteWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.promote_resource_impl(resource_id, input).await
    }

    async fn list_directories(
        &self,
        path: Option<&str>,
    ) -> Result<WorkspaceDirectoryBrowserResponse, AppError> {
        self.list_directories_impl(path).await
    }

    async fn delete_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        self.delete_workspace_resource_impl(workspace_id, resource_id)
            .await
    }

    async fn delete_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError> {
        self.delete_project_resource_impl(project_id, resource_id)
            .await
    }

    async fn update_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.update_workspace_resource_impl(workspace_id, resource_id, input)
            .await
    }

    async fn update_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError> {
        self.update_project_resource_impl(project_id, resource_id, input)
            .await
    }

    async fn list_workspace_knowledge(&self) -> Result<Vec<KnowledgeRecord>, AppError> {
        self.list_workspace_knowledge_impl().await
    }

    async fn list_project_knowledge(
        &self,
        project_id: &str,
    ) -> Result<Vec<KnowledgeRecord>, AppError> {
        self.list_project_knowledge_impl(project_id).await
    }

    async fn get_workspace_pet_snapshot(
        &self,
        owner_user_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.get_workspace_pet_snapshot_impl(owner_user_id).await
    }

    async fn get_project_pet_snapshot(
        &self,
        owner_user_id: &str,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError> {
        self.get_project_pet_snapshot_impl(owner_user_id, project_id)
            .await
    }

    async fn save_workspace_pet_presence(
        &self,
        owner_user_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        self.save_workspace_pet_presence_impl(owner_user_id, input)
            .await
    }

    async fn save_project_pet_presence(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError> {
        self.save_project_pet_presence_impl(owner_user_id, project_id, input)
            .await
    }

    async fn bind_workspace_pet_conversation(
        &self,
        owner_user_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        self.bind_workspace_pet_conversation_impl(owner_user_id, input)
            .await
    }

    async fn bind_project_pet_conversation(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError> {
        self.bind_project_pet_conversation_impl(owner_user_id, project_id, input)
            .await
    }

    async fn list_agents(&self) -> Result<Vec<AgentRecord>, AppError> {
        self.list_agents_impl().await
    }

    async fn create_agent(&self, input: UpsertAgentInput) -> Result<AgentRecord, AppError> {
        self.create_agent_impl(input).await
    }

    async fn update_agent(
        &self,
        agent_id: &str,
        input: UpsertAgentInput,
    ) -> Result<AgentRecord, AppError> {
        self.update_agent_impl(agent_id, input).await
    }

    async fn delete_agent(&self, agent_id: &str) -> Result<(), AppError> {
        self.delete_agent_impl(agent_id).await
    }

    async fn preview_import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
        self.preview_import_agent_bundle_impl(input).await
    }

    async fn import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.import_agent_bundle_impl(input).await
    }

    async fn copy_workspace_agent_from_builtin(
        &self,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_workspace_agent_from_builtin_impl(agent_id).await
    }

    async fn export_agent_bundle(
        &self,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        self.export_agent_bundle_impl(input).await
    }

    async fn preview_import_project_agent_bundle(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError> {
        self.preview_import_project_agent_bundle_impl(project_id, input)
            .await
    }

    async fn import_project_agent_bundle(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.import_project_agent_bundle_impl(project_id, input)
            .await
    }

    async fn copy_project_agent_from_builtin(
        &self,
        project_id: &str,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_project_agent_from_builtin_impl(project_id, agent_id)
            .await
    }

    async fn export_project_agent_bundle(
        &self,
        project_id: &str,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError> {
        self.export_project_agent_bundle_impl(project_id, input)
            .await
    }

    async fn list_project_agent_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAgentLinkRecord>, AppError> {
        self.list_project_agent_links_impl(project_id).await
    }

    async fn link_project_agent(
        &self,
        input: ProjectAgentLinkInput,
    ) -> Result<ProjectAgentLinkRecord, AppError> {
        self.link_project_agent_impl(input).await
    }

    async fn unlink_project_agent(&self, project_id: &str, agent_id: &str) -> Result<(), AppError> {
        self.unlink_project_agent_impl(project_id, agent_id).await
    }

    async fn list_teams(&self) -> Result<Vec<TeamRecord>, AppError> {
        self.list_teams_impl().await
    }

    async fn create_team(&self, input: UpsertTeamInput) -> Result<TeamRecord, AppError> {
        self.create_team_impl(input).await
    }

    async fn update_team(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
    ) -> Result<TeamRecord, AppError> {
        self.update_team_impl(team_id, input).await
    }

    async fn delete_team(&self, team_id: &str) -> Result<(), AppError> {
        self.delete_team_impl(team_id).await
    }

    async fn copy_workspace_team_from_builtin(
        &self,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_workspace_team_from_builtin_impl(team_id).await
    }

    async fn copy_project_team_from_builtin(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError> {
        self.copy_project_team_from_builtin_impl(project_id, team_id)
            .await
    }

    async fn list_project_team_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectTeamLinkRecord>, AppError> {
        self.list_project_team_links_impl(project_id).await
    }

    async fn link_project_team(
        &self,
        input: ProjectTeamLinkInput,
    ) -> Result<ProjectTeamLinkRecord, AppError> {
        self.link_project_team_impl(input).await
    }

    async fn unlink_project_team(&self, project_id: &str, team_id: &str) -> Result<(), AppError> {
        self.unlink_project_team_impl(project_id, team_id).await
    }

    async fn list_models(&self) -> Result<Vec<ModelCatalogRecord>, AppError> {
        self.list_models_impl().await
    }

    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialRecord>, AppError> {
        self.list_provider_credentials_impl().await
    }

    async fn get_capability_management_projection(
        &self,
    ) -> Result<CapabilityManagementProjection, AppError> {
        self.get_capability_management_projection_impl().await
    }

    async fn set_capability_asset_disabled(
        &self,
        patch: CapabilityAssetDisablePatch,
    ) -> Result<CapabilityManagementProjection, AppError> {
        self.set_capability_asset_disabled_impl(patch).await
    }

    async fn get_workspace_skill(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.get_workspace_skill_impl(skill_id).await
    }

    async fn create_workspace_skill(
        &self,
        input: CreateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.create_workspace_skill_impl(input).await
    }

    async fn update_workspace_skill(
        &self,
        skill_id: &str,
        input: UpdateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.update_workspace_skill_impl(skill_id, input).await
    }

    async fn get_workspace_skill_tree(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError> {
        self.get_workspace_skill_tree_impl(skill_id).await
    }

    async fn get_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        self.get_workspace_skill_file_impl(skill_id, relative_path)
            .await
    }

    async fn update_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
        input: UpdateWorkspaceSkillFileInput,
    ) -> Result<WorkspaceSkillFileDocument, AppError> {
        self.update_workspace_skill_file_impl(skill_id, relative_path, input)
            .await
    }

    async fn copy_workspace_skill_to_managed(
        &self,
        skill_id: &str,
        input: CopyWorkspaceSkillToManagedInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.copy_workspace_skill_to_managed_impl(skill_id, input)
            .await
    }

    async fn import_workspace_skill_archive(
        &self,
        input: ImportWorkspaceSkillArchiveInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.import_workspace_skill_archive_impl(input).await
    }

    async fn import_workspace_skill_folder(
        &self,
        input: ImportWorkspaceSkillFolderInput,
    ) -> Result<WorkspaceSkillDocument, AppError> {
        self.import_workspace_skill_folder_impl(input).await
    }

    async fn delete_workspace_skill(&self, skill_id: &str) -> Result<(), AppError> {
        self.delete_workspace_skill_impl(skill_id).await
    }

    async fn get_workspace_mcp_server(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        self.get_workspace_mcp_server_impl(server_name).await
    }

    async fn create_workspace_mcp_server(
        &self,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        self.create_workspace_mcp_server_impl(input).await
    }

    async fn copy_workspace_mcp_server_to_managed(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        self.copy_workspace_mcp_server_to_managed_impl(server_name)
            .await
    }

    async fn update_workspace_mcp_server(
        &self,
        server_name: &str,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError> {
        self.update_workspace_mcp_server_impl(server_name, input)
            .await
    }

    async fn delete_workspace_mcp_server(&self, server_name: &str) -> Result<(), AppError> {
        self.delete_workspace_mcp_server_impl(server_name).await
    }

    async fn list_tools(&self) -> Result<Vec<ToolRecord>, AppError> {
        self.list_tools_impl().await
    }

    async fn create_tool(&self, record: ToolRecord) -> Result<ToolRecord, AppError> {
        self.create_tool_impl(record).await
    }

    async fn update_tool(&self, tool_id: &str, record: ToolRecord) -> Result<ToolRecord, AppError> {
        self.update_tool_impl(tool_id, record).await
    }

    async fn delete_tool(&self, tool_id: &str) -> Result<(), AppError> {
        self.delete_tool_impl(tool_id).await
    }

    async fn current_user_profile(&self, user_id: &str) -> Result<UserRecordSummary, AppError> {
        self.current_user_profile_impl(user_id).await
    }

    async fn update_current_user_profile(
        &self,
        user_id: &str,
        request: UpdateCurrentUserProfileRequest,
    ) -> Result<UserRecordSummary, AppError> {
        self.update_current_user_profile_impl(user_id, request)
            .await
    }

    async fn change_current_user_password(
        &self,
        user_id: &str,
        request: ChangeCurrentUserPasswordRequest,
    ) -> Result<ChangeCurrentUserPasswordResponse, AppError> {
        self.change_current_user_password_impl(user_id, request)
            .await
    }
}
