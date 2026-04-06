use async_trait::async_trait;
use octopus_core::{
    AgentRecord, AppError, AutomationRecord, KnowledgeRecord, MenuRecord, ModelCatalogRecord,
    PermissionRecord, ProjectRecord, ProviderCredentialRecord, RoleRecord,
    SystemBootstrapStatus, TeamRecord, ToolRecord, UserRecordSummary,
    WorkspaceResourceRecord, WorkspaceSummary, WorkspaceToolCatalogSnapshot,
};

#[async_trait]
pub trait WorkspaceService: Send + Sync {
    async fn system_bootstrap(&self) -> Result<SystemBootstrapStatus, AppError>;
    async fn workspace_summary(&self) -> Result<WorkspaceSummary, AppError>;
    async fn list_projects(&self) -> Result<Vec<ProjectRecord>, AppError>;
    async fn list_workspace_resources(&self) -> Result<Vec<WorkspaceResourceRecord>, AppError>;
    async fn list_project_resources(&self, project_id: &str) -> Result<Vec<WorkspaceResourceRecord>, AppError>;
    async fn list_workspace_knowledge(&self) -> Result<Vec<KnowledgeRecord>, AppError>;
    async fn list_project_knowledge(&self, project_id: &str) -> Result<Vec<KnowledgeRecord>, AppError>;
    async fn list_agents(&self) -> Result<Vec<AgentRecord>, AppError>;
    async fn create_agent(&self, record: AgentRecord) -> Result<AgentRecord, AppError>;
    async fn update_agent(&self, agent_id: &str, record: AgentRecord) -> Result<AgentRecord, AppError>;
    async fn delete_agent(&self, agent_id: &str) -> Result<(), AppError>;
    async fn list_teams(&self) -> Result<Vec<TeamRecord>, AppError>;
    async fn create_team(&self, record: TeamRecord) -> Result<TeamRecord, AppError>;
    async fn update_team(&self, team_id: &str, record: TeamRecord) -> Result<TeamRecord, AppError>;
    async fn delete_team(&self, team_id: &str) -> Result<(), AppError>;
    async fn list_models(&self) -> Result<Vec<ModelCatalogRecord>, AppError>;
    async fn list_provider_credentials(&self) -> Result<Vec<ProviderCredentialRecord>, AppError>;
    async fn get_tool_catalog(&self) -> Result<WorkspaceToolCatalogSnapshot, AppError>;
    async fn list_tools(&self) -> Result<Vec<ToolRecord>, AppError>;
    async fn create_tool(&self, record: ToolRecord) -> Result<ToolRecord, AppError>;
    async fn update_tool(&self, tool_id: &str, record: ToolRecord) -> Result<ToolRecord, AppError>;
    async fn delete_tool(&self, tool_id: &str) -> Result<(), AppError>;
    async fn list_automations(&self) -> Result<Vec<AutomationRecord>, AppError>;
    async fn create_automation(&self, record: AutomationRecord) -> Result<AutomationRecord, AppError>;
    async fn update_automation(
        &self,
        automation_id: &str,
        record: AutomationRecord,
    ) -> Result<AutomationRecord, AppError>;
    async fn delete_automation(&self, automation_id: &str) -> Result<(), AppError>;
    async fn list_users(&self) -> Result<Vec<UserRecordSummary>, AppError>;
    async fn create_user(&self, record: UserRecordSummary) -> Result<UserRecordSummary, AppError>;
    async fn update_user(&self, user_id: &str, record: UserRecordSummary) -> Result<UserRecordSummary, AppError>;
    async fn list_roles(&self) -> Result<Vec<RoleRecord>, AppError>;
    async fn create_role(&self, record: RoleRecord) -> Result<RoleRecord, AppError>;
    async fn update_role(&self, role_id: &str, record: RoleRecord) -> Result<RoleRecord, AppError>;
    async fn list_permissions(&self) -> Result<Vec<PermissionRecord>, AppError>;
    async fn create_permission(&self, record: PermissionRecord) -> Result<PermissionRecord, AppError>;
    async fn update_permission(
        &self,
        permission_id: &str,
        record: PermissionRecord,
    ) -> Result<PermissionRecord, AppError>;
    async fn list_menus(&self) -> Result<Vec<MenuRecord>, AppError>;
    async fn create_menu(&self, record: MenuRecord) -> Result<MenuRecord, AppError>;
    async fn update_menu(&self, menu_id: &str, record: MenuRecord) -> Result<MenuRecord, AppError>;
}
