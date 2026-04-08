use std::sync::Arc;

pub mod app_registry;
pub mod artifact;
pub mod auth;
pub mod inbox;
pub mod knowledge;
pub mod observation;
pub mod project;
pub mod rbac;
pub mod runtime;
pub mod workspace;

pub use app_registry::AppRegistryService;
pub use artifact::ArtifactService;
pub use auth::AuthService;
pub use inbox::InboxService;
pub use knowledge::KnowledgeService;
pub use observation::ObservationService;
pub use rbac::RbacService;
pub use runtime::{
    AutomationService, ModelRegistryService, RuntimeConfigService, RuntimeExecutionService,
    RuntimeProjectionService, RuntimeSessionService, ToolExecutionService,
};
pub use workspace::WorkspaceService;

#[derive(Clone)]
pub struct PlatformServices {
    pub workspace: Arc<dyn WorkspaceService>,
    pub auth: Arc<dyn AuthService>,
    pub app_registry: Arc<dyn AppRegistryService>,
    pub rbac: Arc<dyn RbacService>,
    pub runtime_session: Arc<dyn RuntimeSessionService>,
    pub runtime_execution: Arc<dyn RuntimeExecutionService>,
    pub runtime_config: Arc<dyn RuntimeConfigService>,
    pub runtime_registry: Arc<dyn ModelRegistryService>,
    pub artifact: Arc<dyn ArtifactService>,
    pub inbox: Arc<dyn InboxService>,
    pub knowledge: Arc<dyn KnowledgeService>,
    pub observation: Arc<dyn ObservationService>,
}
