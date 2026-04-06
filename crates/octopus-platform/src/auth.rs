use async_trait::async_trait;
use octopus_core::{
    AppError, LoginRequest, LoginResponse, RegisterWorkspaceOwnerRequest,
    RegisterWorkspaceOwnerResponse, SessionRecord,
};

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AppError>;
    async fn register_owner(
        &self,
        request: RegisterWorkspaceOwnerRequest,
    ) -> Result<RegisterWorkspaceOwnerResponse, AppError>;
    async fn logout(&self, token: &str) -> Result<(), AppError>;
    async fn session(&self, token: &str) -> Result<SessionRecord, AppError>;
    async fn lookup_session(&self, token: &str) -> Result<Option<SessionRecord>, AppError>;
}
