use async_trait::async_trait;
use octopus_core::{
    AppError, LoginRequest, LoginResponse, RegisterBootstrapAdminRequest,
    RegisterBootstrapAdminResponse, SessionRecord,
};

#[async_trait]
pub trait AuthService: Send + Sync {
    async fn login(&self, request: LoginRequest) -> Result<LoginResponse, AppError>;
    async fn register_bootstrap_admin(
        &self,
        request: RegisterBootstrapAdminRequest,
    ) -> Result<RegisterBootstrapAdminResponse, AppError>;
    async fn session(&self, token: &str) -> Result<SessionRecord, AppError>;
    async fn lookup_session(&self, token: &str) -> Result<Option<SessionRecord>, AppError>;
    async fn list_sessions(&self) -> Result<Vec<SessionRecord>, AppError>;
    async fn revoke_session(&self, session_id: &str) -> Result<(), AppError>;
    async fn revoke_user_sessions(&self, user_id: &str) -> Result<(), AppError>;
}
