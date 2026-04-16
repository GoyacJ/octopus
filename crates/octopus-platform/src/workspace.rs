use async_trait::async_trait;
use octopus_core::{
    AgentRecord, AppError, AutomationRecord, BindPetConversationInput, CapabilityAssetDisablePatch,
    CapabilityManagementProjection, ChangeCurrentUserPasswordRequest,
    ChangeCurrentUserPasswordResponse, CopyWorkspaceSkillToManagedInput,
    CreateProjectPromotionRequestInput, CreateProjectRequest, CreateWorkspaceResourceFolderInput,
    CreateWorkspaceResourceInput, CreateWorkspaceSkillInput, ExportWorkspaceAgentBundleInput,
    ExportWorkspaceAgentBundleResult, ImportWorkspaceAgentBundleInput,
    ImportWorkspaceAgentBundlePreview, ImportWorkspaceAgentBundlePreviewInput,
    ImportWorkspaceAgentBundleResult, ImportWorkspaceSkillArchiveInput,
    ImportWorkspaceSkillFolderInput, KnowledgeRecord, ModelCatalogRecord, PetConversationBinding,
    PetPresenceState, PetWorkspaceSnapshot, ProjectAgentLinkInput, ProjectAgentLinkRecord,
    ProjectPromotionRequest, ProjectRecord, ProjectTeamLinkInput, ProjectTeamLinkRecord,
    PromoteWorkspaceResourceInput, ProviderCredentialRecord, ReviewProjectPromotionRequestInput,
    SavePetPresenceInput, SystemBootstrapStatus, TeamRecord, ToolRecord,
    UpdateCurrentUserProfileRequest, UpdateProjectRequest, UpdateWorkspaceResourceInput,
    UpdateWorkspaceSkillFileInput, UpdateWorkspaceSkillInput, UpsertAgentInput, UpsertTeamInput,
    UpsertWorkspaceMcpServerInput, UserRecordSummary, WorkspaceDirectoryBrowserResponse,
    WorkspaceMcpServerDocument, WorkspaceResourceChildrenRecord, WorkspaceResourceContentDocument,
    WorkspaceResourceImportInput, WorkspaceResourceRecord, WorkspaceSkillDocument,
    WorkspaceSkillFileDocument, WorkspaceSkillTreeDocument, WorkspaceSummary,
};

#[async_trait]
pub trait WorkspaceService: Send + Sync {
    async fn system_bootstrap(&self) -> Result<SystemBootstrapStatus, AppError>;
    async fn workspace_summary(&self) -> Result<WorkspaceSummary, AppError>;
    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError>;
    async fn create_project(
        &self,
        request: CreateProjectRequest,
    ) -> Result<ProjectRecord, AppError>;
    async fn update_project(
        &self,
        project_id: &str,
        request: UpdateProjectRequest,
    ) -> Result<ProjectRecord, AppError>;
    async fn list_project_promotion_requests(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectPromotionRequest>, AppError>;
    async fn list_workspace_promotion_requests(
        &self,
    ) -> Result<Vec<ProjectPromotionRequest>, AppError>;
    async fn create_project_promotion_request(
        &self,
        project_id: &str,
        requested_by_user_id: &str,
        input: CreateProjectPromotionRequestInput,
    ) -> Result<ProjectPromotionRequest, AppError>;
    async fn review_project_promotion_request(
        &self,
        request_id: &str,
        reviewed_by_user_id: &str,
        input: ReviewProjectPromotionRequestInput,
    ) -> Result<ProjectPromotionRequest, AppError>;
    async fn list_workspace_resources(&self) -> Result<Vec<WorkspaceResourceRecord>, AppError>;
    async fn list_project_resources(
        &self,
        project_id: &str,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError>;
    async fn create_workspace_resource(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn create_project_resource(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn create_project_resource_folder(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: CreateWorkspaceResourceFolderInput,
    ) -> Result<Vec<WorkspaceResourceRecord>, AppError>;
    async fn import_workspace_resource(
        &self,
        workspace_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn import_project_resource(
        &self,
        project_id: &str,
        owner_user_id: &str,
        input: WorkspaceResourceImportInput,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn get_resource_detail(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn get_resource_content(
        &self,
        resource_id: &str,
    ) -> Result<WorkspaceResourceContentDocument, AppError>;
    async fn list_resource_children(
        &self,
        resource_id: &str,
    ) -> Result<Vec<WorkspaceResourceChildrenRecord>, AppError>;
    async fn promote_resource(
        &self,
        resource_id: &str,
        input: PromoteWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn list_directories(
        &self,
        path: Option<&str>,
    ) -> Result<WorkspaceDirectoryBrowserResponse, AppError>;
    async fn delete_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError>;
    async fn delete_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
    ) -> Result<(), AppError>;
    async fn update_workspace_resource(
        &self,
        workspace_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn update_project_resource(
        &self,
        project_id: &str,
        resource_id: &str,
        input: UpdateWorkspaceResourceInput,
    ) -> Result<WorkspaceResourceRecord, AppError>;
    async fn list_workspace_knowledge(&self) -> Result<Vec<KnowledgeRecord>, AppError>;
    async fn list_project_knowledge(
        &self,
        project_id: &str,
    ) -> Result<Vec<KnowledgeRecord>, AppError>;
    async fn get_workspace_pet_snapshot(
        &self,
        owner_user_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError>;
    async fn get_project_pet_snapshot(
        &self,
        owner_user_id: &str,
        project_id: &str,
    ) -> Result<PetWorkspaceSnapshot, AppError>;
    async fn save_workspace_pet_presence(
        &self,
        owner_user_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError>;
    async fn save_project_pet_presence(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: SavePetPresenceInput,
    ) -> Result<PetPresenceState, AppError>;
    async fn bind_workspace_pet_conversation(
        &self,
        owner_user_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError>;
    async fn bind_project_pet_conversation(
        &self,
        owner_user_id: &str,
        project_id: &str,
        input: BindPetConversationInput,
    ) -> Result<PetConversationBinding, AppError>;
    async fn list_agents(&self) -> Result<Vec<AgentRecord>, AppError>;
    async fn create_agent(&self, input: UpsertAgentInput) -> Result<AgentRecord, AppError>;
    async fn update_agent(
        &self,
        agent_id: &str,
        input: UpsertAgentInput,
    ) -> Result<AgentRecord, AppError>;
    async fn delete_agent(&self, agent_id: &str) -> Result<(), AppError>;
    async fn preview_import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError>;
    async fn import_agent_bundle(
        &self,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError>;
    async fn copy_workspace_agent_from_builtin(
        &self,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError>;
    async fn export_agent_bundle(
        &self,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError>;
    async fn preview_import_project_agent_bundle(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundlePreviewInput,
    ) -> Result<ImportWorkspaceAgentBundlePreview, AppError>;
    async fn import_project_agent_bundle(
        &self,
        project_id: &str,
        input: ImportWorkspaceAgentBundleInput,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError>;
    async fn copy_project_agent_from_builtin(
        &self,
        project_id: &str,
        agent_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError>;
    async fn export_project_agent_bundle(
        &self,
        project_id: &str,
        input: ExportWorkspaceAgentBundleInput,
    ) -> Result<ExportWorkspaceAgentBundleResult, AppError>;
    async fn list_project_agent_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectAgentLinkRecord>, AppError>;
    async fn link_project_agent(
        &self,
        input: ProjectAgentLinkInput,
    ) -> Result<ProjectAgentLinkRecord, AppError>;
    async fn unlink_project_agent(&self, project_id: &str, agent_id: &str) -> Result<(), AppError>;
    async fn list_teams(&self) -> Result<Vec<TeamRecord>, AppError>;
    async fn create_team(&self, input: UpsertTeamInput) -> Result<TeamRecord, AppError>;
    async fn update_team(
        &self,
        team_id: &str,
        input: UpsertTeamInput,
    ) -> Result<TeamRecord, AppError>;
    async fn delete_team(&self, team_id: &str) -> Result<(), AppError>;
    async fn copy_workspace_team_from_builtin(
        &self,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError>;
    async fn copy_project_team_from_builtin(
        &self,
        project_id: &str,
        team_id: &str,
    ) -> Result<ImportWorkspaceAgentBundleResult, AppError>;
    async fn list_project_team_links(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectTeamLinkRecord>, AppError>;
    async fn link_project_team(
        &self,
        input: ProjectTeamLinkInput,
    ) -> Result<ProjectTeamLinkRecord, AppError>;
    async fn unlink_project_team(&self, project_id: &str, team_id: &str) -> Result<(), AppError>;
    async fn list_models(&self) -> Result<Vec<ModelCatalogRecord>, AppError>;
    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialRecord>, AppError>;
    async fn get_capability_management_projection(
        &self,
    ) -> Result<CapabilityManagementProjection, AppError>;
    async fn set_capability_asset_disabled(
        &self,
        patch: CapabilityAssetDisablePatch,
    ) -> Result<CapabilityManagementProjection, AppError>;
    async fn get_workspace_skill(&self, skill_id: &str)
        -> Result<WorkspaceSkillDocument, AppError>;
    async fn create_workspace_skill(
        &self,
        input: CreateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError>;
    async fn update_workspace_skill(
        &self,
        skill_id: &str,
        input: UpdateWorkspaceSkillInput,
    ) -> Result<WorkspaceSkillDocument, AppError>;
    async fn get_workspace_skill_tree(
        &self,
        skill_id: &str,
    ) -> Result<WorkspaceSkillTreeDocument, AppError>;
    async fn get_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
    ) -> Result<WorkspaceSkillFileDocument, AppError>;
    async fn update_workspace_skill_file(
        &self,
        skill_id: &str,
        relative_path: &str,
        input: UpdateWorkspaceSkillFileInput,
    ) -> Result<WorkspaceSkillFileDocument, AppError>;
    async fn copy_workspace_skill_to_managed(
        &self,
        skill_id: &str,
        input: CopyWorkspaceSkillToManagedInput,
    ) -> Result<WorkspaceSkillDocument, AppError>;
    async fn import_workspace_skill_archive(
        &self,
        input: ImportWorkspaceSkillArchiveInput,
    ) -> Result<WorkspaceSkillDocument, AppError>;
    async fn import_workspace_skill_folder(
        &self,
        input: ImportWorkspaceSkillFolderInput,
    ) -> Result<WorkspaceSkillDocument, AppError>;
    async fn delete_workspace_skill(&self, skill_id: &str) -> Result<(), AppError>;
    async fn get_workspace_mcp_server(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError>;
    async fn create_workspace_mcp_server(
        &self,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError>;
    async fn copy_workspace_mcp_server_to_managed(
        &self,
        server_name: &str,
    ) -> Result<WorkspaceMcpServerDocument, AppError>;
    async fn update_workspace_mcp_server(
        &self,
        server_name: &str,
        input: UpsertWorkspaceMcpServerInput,
    ) -> Result<WorkspaceMcpServerDocument, AppError>;
    async fn delete_workspace_mcp_server(&self, server_name: &str) -> Result<(), AppError>;
    async fn list_tools(&self) -> Result<Vec<ToolRecord>, AppError>;
    async fn create_tool(&self, record: ToolRecord) -> Result<ToolRecord, AppError>;
    async fn update_tool(&self, tool_id: &str, record: ToolRecord) -> Result<ToolRecord, AppError>;
    async fn delete_tool(&self, tool_id: &str) -> Result<(), AppError>;
    async fn list_automations(&self) -> Result<Vec<AutomationRecord>, AppError>;
    async fn create_automation(
        &self,
        record: AutomationRecord,
    ) -> Result<AutomationRecord, AppError>;
    async fn update_automation(
        &self,
        automation_id: &str,
        record: AutomationRecord,
    ) -> Result<AutomationRecord, AppError>;
    async fn delete_automation(&self, automation_id: &str) -> Result<(), AppError>;
    async fn update_current_user_profile(
        &self,
        user_id: &str,
        request: UpdateCurrentUserProfileRequest,
    ) -> Result<UserRecordSummary, AppError>;
    async fn change_current_user_password(
        &self,
        user_id: &str,
        request: ChangeCurrentUserPasswordRequest,
    ) -> Result<ChangeCurrentUserPasswordResponse, AppError>;
}
