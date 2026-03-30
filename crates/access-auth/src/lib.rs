use std::{fs, path::Path};

use argon2::{
    password_hash::{PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::ErrorKind as JwtErrorKind, DecodingKey, EncodingKey, Header,
    Validation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    Row, Sqlite, SqlitePool, Transaction,
};
use thiserror::Error;

pub const DEFAULT_BOOTSTRAP_EMAIL: &str = "admin@octopus.local";
pub const DEFAULT_BOOTSTRAP_PASSWORD: &str = "octopus-bootstrap-password";
pub const DEFAULT_BOOTSTRAP_ACTOR_REF: &str = "workspace_admin:bootstrap_admin";
pub const DEFAULT_JWT_SECRET: &str = "octopus-slice10-remote-hub-dev-secret";
pub const DEFAULT_SESSION_TTL_SECONDS: i64 = 60 * 60;
pub const DEFAULT_REFRESH_TOKEN_TTL_SECONDS: i64 = 60 * 60 * 24 * 7;

#[derive(Debug, Clone)]
pub struct RemoteAccessConfig {
    pub bootstrap_email: String,
    pub bootstrap_password: String,
    pub bootstrap_display_name: String,
    pub bootstrap_actor_ref: String,
    pub jwt_secret: String,
    pub session_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
}

impl Default for RemoteAccessConfig {
    fn default() -> Self {
        Self {
            bootstrap_email: DEFAULT_BOOTSTRAP_EMAIL.to_string(),
            bootstrap_password: DEFAULT_BOOTSTRAP_PASSWORD.to_string(),
            bootstrap_display_name: "Bootstrap Admin".to_string(),
            bootstrap_actor_ref: DEFAULT_BOOTSTRAP_ACTOR_REF.to_string(),
            jwt_secret: DEFAULT_JWT_SECRET.to_string(),
            session_ttl_seconds: DEFAULT_SESSION_TTL_SECONDS,
            refresh_token_ttl_seconds: DEFAULT_REFRESH_TOKEN_TTL_SECONDS,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HubSession {
    pub session_id: String,
    pub user_id: String,
    pub email: String,
    pub workspace_id: String,
    pub actor_ref: String,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HubLoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_expires_at: String,
    pub session: HubSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HubRefreshCommand {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HubRefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub refresh_expires_at: String,
    pub session: HubSession,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionTokenClaims {
    pub sub: String,
    pub sid: String,
    pub wid: String,
    pub actor_ref: String,
    pub email: String,
    pub iat: usize,
    pub exp: usize,
}

#[derive(Clone)]
pub struct RemoteAccessService {
    pool: SqlitePool,
    config: RemoteAccessConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

#[derive(Debug, Error)]
pub enum AccessAuthError {
    #[error("missing bearer token")]
    MissingBearerToken,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid session token")]
    InvalidToken,
    #[error("session token expired")]
    TokenExpired,
    #[error("workspace `{0}` is not available for this session")]
    WorkspaceForbidden(String),
    #[error("workspace `{0}` not found")]
    WorkspaceNotFound(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("password hashing failed: {0}")]
    PasswordHash(String),
    #[error("jwt encode failed: {0}")]
    JwtEncode(String),
}

#[derive(Debug, Clone)]
struct RemoteUserRecord {
    id: String,
    email: String,
    display_name: String,
    password_hash: String,
    is_bootstrap: bool,
}

#[derive(Debug, Clone)]
struct WorkspaceMembershipRecord {
    id: String,
    user_id: String,
    workspace_id: String,
    role: String,
    actor_ref: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct AuthSessionRecord {
    id: String,
    user_id: String,
    workspace_id: String,
    actor_ref: String,
    issued_at: String,
    expires_at: String,
    revoked_at: Option<String>,
    last_seen_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct AuthRefreshTokenRecord {
    id: String,
    family_id: String,
    session_id: String,
    user_id: String,
    workspace_id: String,
    token_hash: String,
    issued_at: String,
    expires_at: String,
    rotated_at: Option<String>,
    replaced_by_token_id: Option<String>,
    revoked_at: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct IssuedSessionTokens {
    access_token: String,
    refresh_token: String,
    refresh_expires_at: String,
    session: HubSession,
}

#[derive(Debug, Clone)]
struct ParsedRefreshToken {
    token_id: String,
    secret: String,
}

impl From<IssuedSessionTokens> for HubLoginResponse {
    fn from(value: IssuedSessionTokens) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            refresh_expires_at: value.refresh_expires_at,
            session: value.session,
        }
    }
}

impl From<IssuedSessionTokens> for HubRefreshResponse {
    fn from(value: IssuedSessionTokens) -> Self {
        Self {
            access_token: value.access_token,
            refresh_token: value.refresh_token,
            refresh_expires_at: value.refresh_expires_at,
            session: value.session,
        }
    }
}

impl RemoteAccessService {
    pub async fn open_at(path: &Path) -> Result<Self, AccessAuthError> {
        Self::open_at_with_config(path, RemoteAccessConfig::default()).await
    }

    pub async fn open_at_with_config(
        path: &Path,
        config: RemoteAccessConfig,
    ) -> Result<Self, AccessAuthError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::raw_sql(include_str!("../migrations/0010_remote_access_auth.sql"))
            .execute(&pool)
            .await?;
        sqlx::raw_sql(include_str!("../migrations/0011_remote_access_refresh_tokens.sql"))
            .execute(&pool)
            .await?;

        let service = Self {
            pool,
            encoding_key: EncodingKey::from_secret(config.jwt_secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            config,
        };
        service.seed_bootstrap_user().await?;
        Ok(service)
    }

    pub async fn login(
        &self,
        workspace_id: &str,
        email: &str,
        password: &str,
    ) -> Result<HubLoginResponse, AccessAuthError> {
        let user = self
            .fetch_user_by_email(email)
            .await?
            .ok_or(AccessAuthError::InvalidCredentials)?;
        verify_password(password, &user.password_hash)?;

        let membership = self
            .ensure_workspace_membership_for_login(&user, workspace_id)
            .await?;
        let mut tx = self.pool.begin().await?;
        let session = self.create_session_tx(&mut tx, &user, &membership).await?;
        let credentials = self
            .issue_session_tokens_tx(&mut tx, &session, &session.session_id)
            .await?;
        tx.commit().await?;

        Ok(credentials.into())
    }

    pub async fn authenticate_token(&self, raw_token: &str) -> Result<HubSession, AccessAuthError> {
        let token = extract_bearer_token(raw_token)?;
        let claims = decode::<SessionTokenClaims>(token, &self.decoding_key, &Validation::default())
            .map_err(map_jwt_decode_error)?
            .claims;
        let session = self
            .fetch_session(&claims.sid)
            .await?
            .ok_or(AccessAuthError::InvalidToken)?;

        if session.revoked_at.is_some() {
            return Err(AccessAuthError::InvalidToken);
        }
        if session.user_id != claims.sub
            || session.workspace_id != claims.wid
            || session.actor_ref != claims.actor_ref
        {
            return Err(AccessAuthError::InvalidToken);
        }
        if timestamp_expired(&session.expires_at) {
            return Err(AccessAuthError::TokenExpired);
        }

        self.touch_session(&session.id).await?;

        Ok(HubSession {
            session_id: session.id,
            user_id: session.user_id,
            email: claims.email,
            workspace_id: session.workspace_id,
            actor_ref: session.actor_ref,
            issued_at: session.issued_at,
            expires_at: session.expires_at,
        })
    }

    pub async fn current_session(
        &self,
        authorization: &str,
    ) -> Result<HubSession, AccessAuthError> {
        self.authenticate_token(authorization).await
    }

    pub async fn refresh_session(
        &self,
        refresh_token: &str,
    ) -> Result<HubRefreshResponse, AccessAuthError> {
        let parsed = parse_refresh_token(refresh_token)?;
        let mut tx = self.pool.begin().await?;
        let refresh = self
            .fetch_refresh_token_tx(&mut tx, &parsed.token_id)
            .await?
            .ok_or(AccessAuthError::InvalidToken)?;
        if hash_refresh_token_secret(&parsed.secret) != refresh.token_hash {
            return Err(AccessAuthError::InvalidToken);
        }
        if refresh.revoked_at.is_some() {
            return Err(AccessAuthError::InvalidToken);
        }
        if timestamp_expired(&refresh.expires_at) {
            return Err(AccessAuthError::TokenExpired);
        }
        if refresh.rotated_at.is_some() || refresh.replaced_by_token_id.is_some() {
            self.revoke_session_family_tx(&mut tx, &refresh.family_id)
                .await?;
            tx.commit().await?;
            return Err(AccessAuthError::InvalidToken);
        }

        let session = self
            .fetch_session_tx(&mut tx, &refresh.session_id)
            .await?
            .ok_or(AccessAuthError::InvalidToken)?;
        if session.revoked_at.is_some() {
            return Err(AccessAuthError::InvalidToken);
        }

        let user = self
            .fetch_user_by_id_tx(&mut tx, &session.user_id)
            .await?
            .ok_or(AccessAuthError::InvalidToken)?;
        let next_session = self
            .rotate_session_tx(&mut tx, &session, &user.email)
            .await?;
        let credentials = self
            .rotate_refresh_token_tx(&mut tx, &refresh, &next_session)
            .await?;
        tx.commit().await?;

        Ok(credentials.into())
    }

    pub async fn logout(&self, session_id: &str) -> Result<(), AccessAuthError> {
        let mut tx = self.pool.begin().await?;
        self.revoke_session_family_tx(&mut tx, session_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn ensure_workspace_access(
        &self,
        session: &HubSession,
        workspace_id: &str,
    ) -> Result<(), AccessAuthError> {
        if session.workspace_id != workspace_id {
            return Err(AccessAuthError::WorkspaceForbidden(workspace_id.to_string()));
        }

        let membership = self
            .fetch_membership(&session.user_id, workspace_id)
            .await?
            .ok_or_else(|| AccessAuthError::WorkspaceForbidden(workspace_id.to_string()))?;
        if membership.actor_ref != session.actor_ref {
            return Err(AccessAuthError::WorkspaceForbidden(workspace_id.to_string()));
        }

        Ok(())
    }

    async fn seed_bootstrap_user(&self) -> Result<(), AccessAuthError> {
        let user_id = "remote-user-bootstrap-admin".to_string();
        let now = current_timestamp();
        sqlx::query(
            r#"
            INSERT INTO remote_users (
                id, email, display_name, password_hash, is_bootstrap, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6)
            ON CONFLICT(email) DO UPDATE SET
                display_name = excluded.display_name,
                password_hash = excluded.password_hash,
                is_bootstrap = excluded.is_bootstrap,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&user_id)
        .bind(&self.config.bootstrap_email)
        .bind(&self.config.bootstrap_display_name)
        .bind(hash_password(&self.config.bootstrap_password)?)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn ensure_workspace_membership_for_login(
        &self,
        user: &RemoteUserRecord,
        workspace_id: &str,
    ) -> Result<WorkspaceMembershipRecord, AccessAuthError> {
        if let Some(existing) = self.fetch_membership(&user.id, workspace_id).await? {
            return Ok(existing);
        }

        if !user.is_bootstrap {
            return Err(AccessAuthError::WorkspaceForbidden(workspace_id.to_string()));
        }

        let workspace_exists = sqlx::query("SELECT id FROM workspaces WHERE id = ?1")
            .bind(workspace_id)
            .fetch_optional(&self.pool)
            .await?
            .is_some();
        if !workspace_exists {
            return Err(AccessAuthError::WorkspaceNotFound(workspace_id.to_string()));
        }

        let now = current_timestamp();
        let membership = WorkspaceMembershipRecord {
            id: format!("membership:{}:{workspace_id}", user.id),
            user_id: user.id.clone(),
            workspace_id: workspace_id.to_string(),
            role: "workspace_admin".to_string(),
            actor_ref: self.config.bootstrap_actor_ref.clone(),
            created_at: now.clone(),
            updated_at: now,
        };
        sqlx::query(
            r#"
            INSERT INTO workspace_memberships (
                id, user_id, workspace_id, role, actor_ref, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(user_id, workspace_id) DO UPDATE SET
                role = excluded.role,
                actor_ref = excluded.actor_ref,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&membership.id)
        .bind(&membership.user_id)
        .bind(&membership.workspace_id)
        .bind(&membership.role)
        .bind(&membership.actor_ref)
        .bind(&membership.created_at)
        .bind(&membership.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(membership)
    }

    async fn create_session_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        user: &RemoteUserRecord,
        membership: &WorkspaceMembershipRecord,
    ) -> Result<HubSession, AccessAuthError> {
        let issued_at = current_timestamp();
        let expires_at = (Utc::now() + Duration::seconds(self.config.session_ttl_seconds))
            .to_rfc3339();
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = current_timestamp();

        sqlx::query(
            r#"
            INSERT INTO auth_sessions (
                id, user_id, workspace_id, actor_ref, issued_at, expires_at, revoked_at,
                last_seen_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7, ?8, ?9)
            "#,
        )
        .bind(&session_id)
        .bind(&user.id)
        .bind(&membership.workspace_id)
        .bind(&membership.actor_ref)
        .bind(&issued_at)
        .bind(&expires_at)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .execute(&mut **tx)
        .await?;

        Ok(HubSession {
            session_id,
            user_id: user.id.clone(),
            email: user.email.clone(),
            workspace_id: membership.workspace_id.clone(),
            actor_ref: membership.actor_ref.clone(),
            issued_at,
            expires_at,
        })
    }

    async fn fetch_user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<RemoteUserRecord>, AccessAuthError> {
        sqlx::query(
            r#"
            SELECT id, email, display_name, password_hash, is_bootstrap
            FROM remote_users
            WHERE email = ?1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?
        .map(remote_user_from_row)
        .transpose()
        .map_err(AccessAuthError::from)
    }

    async fn fetch_user_by_id_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        user_id: &str,
    ) -> Result<Option<RemoteUserRecord>, AccessAuthError> {
        sqlx::query(
            r#"
            SELECT id, email, display_name, password_hash, is_bootstrap
            FROM remote_users
            WHERE id = ?1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(remote_user_from_row)
        .transpose()
        .map_err(AccessAuthError::from)
    }

    async fn fetch_membership(
        &self,
        user_id: &str,
        workspace_id: &str,
    ) -> Result<Option<WorkspaceMembershipRecord>, AccessAuthError> {
        sqlx::query(
            r#"
            SELECT id, user_id, workspace_id, role, actor_ref, created_at, updated_at
            FROM workspace_memberships
            WHERE user_id = ?1 AND workspace_id = ?2
            "#,
        )
        .bind(user_id)
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await?
        .map(workspace_membership_from_row)
        .transpose()
        .map_err(AccessAuthError::from)
    }

    async fn fetch_session(
        &self,
        session_id: &str,
    ) -> Result<Option<AuthSessionRecord>, AccessAuthError> {
        sqlx::query(
            r#"
            SELECT id, user_id, workspace_id, actor_ref, issued_at, expires_at, revoked_at,
                   last_seen_at, created_at, updated_at
            FROM auth_sessions
            WHERE id = ?1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?
        .map(auth_session_from_row)
        .transpose()
        .map_err(AccessAuthError::from)
    }

    async fn fetch_session_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        session_id: &str,
    ) -> Result<Option<AuthSessionRecord>, AccessAuthError> {
        sqlx::query(
            r#"
            SELECT id, user_id, workspace_id, actor_ref, issued_at, expires_at, revoked_at,
                   last_seen_at, created_at, updated_at
            FROM auth_sessions
            WHERE id = ?1
            "#,
        )
        .bind(session_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(auth_session_from_row)
        .transpose()
        .map_err(AccessAuthError::from)
    }

    async fn fetch_refresh_token_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        refresh_token_id: &str,
    ) -> Result<Option<AuthRefreshTokenRecord>, AccessAuthError> {
        sqlx::query(
            r#"
            SELECT id, family_id, session_id, user_id, workspace_id, token_hash, issued_at,
                   expires_at, rotated_at, replaced_by_token_id, revoked_at, created_at, updated_at
            FROM auth_refresh_tokens
            WHERE id = ?1
            "#,
        )
        .bind(refresh_token_id)
        .fetch_optional(&mut **tx)
        .await?
        .map(auth_refresh_token_from_row)
        .transpose()
        .map_err(AccessAuthError::from)
    }

    async fn issue_session_tokens_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        session: &HubSession,
        family_id: &str,
    ) -> Result<IssuedSessionTokens, AccessAuthError> {
        let access_token = self.encode_access_token(session)?;
        let refresh = self
            .create_refresh_token_record_tx(tx, family_id, session)
            .await?;

        Ok(IssuedSessionTokens {
            access_token,
            refresh_token: refresh.raw_token,
            refresh_expires_at: refresh.record.expires_at,
            session: session.clone(),
        })
    }

    async fn create_refresh_token_record_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        family_id: &str,
        session: &HubSession,
    ) -> Result<CreatedRefreshToken, AccessAuthError> {
        let issued_at = current_timestamp();
        let expires_at = (Utc::now() + Duration::seconds(self.config.refresh_token_ttl_seconds))
            .to_rfc3339();
        let token_id = uuid::Uuid::new_v4().to_string();
        let secret = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
        let now = current_timestamp();
        let record = AuthRefreshTokenRecord {
            id: token_id.clone(),
            family_id: family_id.to_string(),
            session_id: session.session_id.clone(),
            user_id: session.user_id.clone(),
            workspace_id: session.workspace_id.clone(),
            token_hash: hash_refresh_token_secret(&secret),
            issued_at,
            expires_at,
            rotated_at: None,
            replaced_by_token_id: None,
            revoked_at: None,
            created_at: now.clone(),
            updated_at: now,
        };

        sqlx::query(
            r#"
            INSERT INTO auth_refresh_tokens (
                id, family_id, session_id, user_id, workspace_id, token_hash, issued_at,
                expires_at, rotated_at, replaced_by_token_id, revoked_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, NULL, ?9, ?10)
            "#,
        )
        .bind(&record.id)
        .bind(&record.family_id)
        .bind(&record.session_id)
        .bind(&record.user_id)
        .bind(&record.workspace_id)
        .bind(&record.token_hash)
        .bind(&record.issued_at)
        .bind(&record.expires_at)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&mut **tx)
        .await?;

        Ok(CreatedRefreshToken {
            raw_token: format!("{}.{}", token_id, secret),
            record,
        })
    }

    async fn rotate_session_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        session: &AuthSessionRecord,
        email: &str,
    ) -> Result<HubSession, AccessAuthError> {
        let issued_at = current_timestamp();
        let refresh_session_ttl_seconds = self.config.session_ttl_seconds.max(1);
        let expires_at = (Utc::now() + Duration::seconds(refresh_session_ttl_seconds))
            .to_rfc3339();
        let now = current_timestamp();

        sqlx::query(
            r#"
            UPDATE auth_sessions
            SET issued_at = ?2, expires_at = ?3, last_seen_at = ?4, updated_at = ?4
            WHERE id = ?1
            "#,
        )
        .bind(&session.id)
        .bind(&issued_at)
        .bind(&expires_at)
        .bind(&now)
        .execute(&mut **tx)
        .await?;

        Ok(HubSession {
            session_id: session.id.clone(),
            user_id: session.user_id.clone(),
            email: email.to_string(),
            workspace_id: session.workspace_id.clone(),
            actor_ref: session.actor_ref.clone(),
            issued_at,
            expires_at,
        })
    }

    async fn rotate_refresh_token_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        current_refresh: &AuthRefreshTokenRecord,
        session: &HubSession,
    ) -> Result<IssuedSessionTokens, AccessAuthError> {
        let next_credentials = self
            .issue_session_tokens_tx(tx, session, &current_refresh.family_id)
            .await?;
        let now = current_timestamp();
        let next_refresh = parse_refresh_token(&next_credentials.refresh_token)?;

        sqlx::query(
            r#"
            UPDATE auth_refresh_tokens
            SET rotated_at = ?2, replaced_by_token_id = ?3, updated_at = ?2
            WHERE id = ?1
            "#,
        )
        .bind(&current_refresh.id)
        .bind(&now)
        .bind(&next_refresh.token_id)
        .execute(&mut **tx)
        .await?;

        Ok(next_credentials)
    }

    async fn revoke_session_family_tx(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        family_id: &str,
    ) -> Result<(), AccessAuthError> {
        let now = current_timestamp();
        sqlx::query(
            r#"
            UPDATE auth_sessions
            SET revoked_at = COALESCE(revoked_at, ?2), updated_at = ?2
            WHERE id = ?1
            "#,
        )
        .bind(family_id)
        .bind(&now)
        .execute(&mut **tx)
        .await?;
        sqlx::query(
            r#"
            UPDATE auth_refresh_tokens
            SET revoked_at = COALESCE(revoked_at, ?2), updated_at = ?2
            WHERE family_id = ?1
            "#,
        )
        .bind(family_id)
        .bind(&now)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    fn encode_access_token(&self, session: &HubSession) -> Result<String, AccessAuthError> {
        let claims = SessionTokenClaims::from_session(session);
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|error| AccessAuthError::JwtEncode(error.to_string()))
    }

    async fn touch_session(&self, session_id: &str) -> Result<(), AccessAuthError> {
        let now = current_timestamp();
        sqlx::query(
            r#"
            UPDATE auth_sessions
            SET last_seen_at = ?2, updated_at = ?2
            WHERE id = ?1
            "#,
        )
        .bind(session_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct CreatedRefreshToken {
    raw_token: String,
    record: AuthRefreshTokenRecord,
}

impl SessionTokenClaims {
    pub fn from_session(session: &HubSession) -> Self {
        Self {
            sub: session.user_id.clone(),
            sid: session.session_id.clone(),
            wid: session.workspace_id.clone(),
            actor_ref: session.actor_ref.clone(),
            email: session.email.clone(),
            iat: parse_timestamp_seconds(&session.issued_at),
            exp: parse_timestamp_seconds(&session.expires_at),
        }
    }
}

fn remote_user_from_row(row: sqlx::sqlite::SqliteRow) -> Result<RemoteUserRecord, sqlx::Error> {
    Ok(RemoteUserRecord {
        id: row.try_get("id")?,
        email: row.try_get("email")?,
        display_name: row.try_get("display_name")?,
        password_hash: row.try_get("password_hash")?,
        is_bootstrap: row.try_get::<i64, _>("is_bootstrap")? != 0,
    })
}

fn workspace_membership_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<WorkspaceMembershipRecord, sqlx::Error> {
    Ok(WorkspaceMembershipRecord {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        workspace_id: row.try_get("workspace_id")?,
        role: row.try_get("role")?,
        actor_ref: row.try_get("actor_ref")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn auth_session_from_row(row: sqlx::sqlite::SqliteRow) -> Result<AuthSessionRecord, sqlx::Error> {
    Ok(AuthSessionRecord {
        id: row.try_get("id")?,
        user_id: row.try_get("user_id")?,
        workspace_id: row.try_get("workspace_id")?,
        actor_ref: row.try_get("actor_ref")?,
        issued_at: row.try_get("issued_at")?,
        expires_at: row.try_get("expires_at")?,
        revoked_at: row.try_get("revoked_at")?,
        last_seen_at: row.try_get("last_seen_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn auth_refresh_token_from_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<AuthRefreshTokenRecord, sqlx::Error> {
    Ok(AuthRefreshTokenRecord {
        id: row.try_get("id")?,
        family_id: row.try_get("family_id")?,
        session_id: row.try_get("session_id")?,
        user_id: row.try_get("user_id")?,
        workspace_id: row.try_get("workspace_id")?,
        token_hash: row.try_get("token_hash")?,
        issued_at: row.try_get("issued_at")?,
        expires_at: row.try_get("expires_at")?,
        rotated_at: row.try_get("rotated_at")?,
        replaced_by_token_id: row.try_get("replaced_by_token_id")?,
        revoked_at: row.try_get("revoked_at")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn hash_password(password: &str) -> Result<String, AccessAuthError> {
    let salt =
        SaltString::encode_b64(b"octopus-slice10-bootstrap-salt").map_err(password_hash_error)?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(password_hash_error)
}

fn verify_password(password: &str, password_hash: &str) -> Result<(), AccessAuthError> {
    let parsed_hash = PasswordHash::new(password_hash).map_err(password_hash_error)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AccessAuthError::InvalidCredentials)
}

fn extract_bearer_token(header: &str) -> Result<&str, AccessAuthError> {
    header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .filter(|token| !token.trim().is_empty())
        .ok_or(AccessAuthError::MissingBearerToken)
}

fn parse_refresh_token(raw_token: &str) -> Result<ParsedRefreshToken, AccessAuthError> {
    let trimmed = raw_token.trim();
    let (token_id, secret) = trimmed
        .split_once('.')
        .ok_or(AccessAuthError::InvalidToken)?;
    if token_id.trim().is_empty() || secret.trim().is_empty() {
        return Err(AccessAuthError::InvalidToken);
    }

    Ok(ParsedRefreshToken {
        token_id: token_id.to_string(),
        secret: secret.to_string(),
    })
}

fn hash_refresh_token_secret(secret: &str) -> String {
    let digest = Sha256::digest(secret.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn password_hash_error(error: impl std::fmt::Display) -> AccessAuthError {
    AccessAuthError::PasswordHash(error.to_string())
}

fn map_jwt_decode_error(error: jsonwebtoken::errors::Error) -> AccessAuthError {
    if matches!(error.kind(), JwtErrorKind::ExpiredSignature) {
        AccessAuthError::TokenExpired
    } else {
        AccessAuthError::InvalidToken
    }
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn timestamp_expired(timestamp: &str) -> bool {
    DateTime::parse_from_rfc3339(timestamp)
        .map(|parsed| parsed.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(true)
}

fn parse_timestamp_seconds(timestamp: &str) -> usize {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|value| value.timestamp().max(0) as usize)
        .unwrap_or_default()
}
