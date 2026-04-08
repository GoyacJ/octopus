use async_trait::async_trait;
use octopus_core::{AppError, AuthorizationDecision, SessionRecord};

#[async_trait]
pub trait RbacService: Send + Sync {
    async fn authorize(
        &self,
        session: &SessionRecord,
        capability: &str,
        project_id: Option<&str>,
    ) -> Result<AuthorizationDecision, AppError>;
}
