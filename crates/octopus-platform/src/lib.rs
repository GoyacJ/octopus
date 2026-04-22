use std::sync::Arc;

pub mod access_control;
pub mod app_registry;
pub mod artifact;
pub mod auth;
pub mod authorization;
pub mod inbox;
pub mod knowledge;
pub mod observation;
pub mod project;
pub mod runtime;
pub mod runtime_sdk;
pub mod workspace;

pub use access_control::AccessControlService;
pub use app_registry::AppRegistryService;
pub use artifact::ArtifactService;
pub use auth::AuthService;
pub use authorization::AuthorizationService;
pub use inbox::InboxService;
pub use knowledge::KnowledgeService;
pub use observation::ObservationService;
pub use project::ProjectTaskService;
pub use runtime::{
    AutomationService, ModelRegistryService, RuntimeConfigService, RuntimeExecutionService,
    RuntimeProjectionService, RuntimeSessionService, ToolExecutionService,
};
pub use runtime_sdk::{RuntimeSdkBridge, RuntimeSdkDeps, RuntimeSdkFactory};
pub use workspace::WorkspaceService;

#[derive(Clone)]
pub struct PlatformServices {
    pub workspace: Arc<dyn WorkspaceService>,
    pub project_tasks: Arc<dyn ProjectTaskService>,
    pub access_control: Arc<dyn AccessControlService>,
    pub auth: Arc<dyn AuthService>,
    pub app_registry: Arc<dyn AppRegistryService>,
    pub authorization: Arc<dyn AuthorizationService>,
    pub runtime_session: Arc<dyn RuntimeSessionService>,
    pub runtime_execution: Arc<dyn RuntimeExecutionService>,
    pub runtime_config: Arc<dyn RuntimeConfigService>,
    pub runtime_registry: Arc<dyn ModelRegistryService>,
    pub artifact: Arc<dyn ArtifactService>,
    pub inbox: Arc<dyn InboxService>,
    pub knowledge: Arc<dyn KnowledgeService>,
    pub observation: Arc<dyn ObservationService>,
}
