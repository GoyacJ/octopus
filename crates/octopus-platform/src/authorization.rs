use async_trait::async_trait;
use octopus_core::{AppError, AuthorizationDecision, AuthorizationRequest, SessionRecord};

#[async_trait]
pub trait AuthorizationService: Send + Sync {
    async fn authorize_request(
        &self,
        session: &SessionRecord,
        request: &AuthorizationRequest,
    ) -> Result<AuthorizationDecision, AppError>;

    async fn authorize(
        &self,
        session: &SessionRecord,
        capability: &str,
        project_id: Option<&str>,
    ) -> Result<AuthorizationDecision, AppError> {
        self.authorize_request(
            session,
            &AuthorizationRequest {
                subject_id: session.user_id.clone(),
                capability: capability.into(),
                project_id: project_id.map(str::to_string),
                resource_type: None,
                resource_id: None,
                resource_subtype: None,
                tags: Vec::new(),
                classification: None,
                owner_subject_type: None,
                owner_subject_id: None,
            },
        )
        .await
    }
}
