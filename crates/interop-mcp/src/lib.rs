use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

pub const TRUST_LEVEL_TRUSTED: &str = "trusted";
pub const TRUST_LEVEL_EXTERNAL_UNTRUSTED: &str = "external_untrusted";
pub const PROVENANCE_SOURCE_MCP_CONNECTOR: &str = "mcp_connector";
pub const KNOWLEDGE_GATE_ELIGIBLE: &str = "eligible";
pub const KNOWLEDGE_GATE_BLOCKED_LOW_TRUST: &str = "blocked_low_trust";

pub const TRANSPORT_KIND_SIMULATED: &str = "simulated";
pub const TRANSPORT_KIND_HTTP_JSONRPC: &str = "http_jsonrpc";

pub const HEALTH_STATUS_UNKNOWN: &str = "unknown";
pub const HEALTH_STATUS_HEALTHY: &str = "healthy";
pub const HEALTH_STATUS_DEGRADED: &str = "degraded";
pub const HEALTH_STATUS_DISABLED: &str = "disabled";

pub const CREDENTIAL_KIND_BEARER_TOKEN: &str = "bearer_token";
pub const CREDENTIAL_KIND_STATIC_HEADER: &str = "static_header";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerRecord {
    pub id: String,
    pub capability_id: String,
    pub namespace: String,
    pub platform: String,
    pub transport_kind: String,
    pub endpoint: Option<String>,
    pub request_timeout_ms: i64,
    pub credential_ref: Option<String>,
    pub trust_level: String,
    pub health_status: String,
    pub lease_ttl_seconds: i64,
    pub created_at: String,
    pub updated_at: String,
    pub last_checked_at: String,
}

impl McpServerRecord {
    pub fn new_fake(
        id: impl Into<String>,
        capability_id: impl Into<String>,
        namespace: impl Into<String>,
        platform: impl Into<String>,
        trust_level: impl Into<String>,
        lease_ttl_seconds: i64,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            capability_id: capability_id.into(),
            namespace: namespace.into(),
            platform: platform.into(),
            transport_kind: TRANSPORT_KIND_SIMULATED.to_string(),
            endpoint: None,
            request_timeout_ms: 1_000,
            credential_ref: None,
            trust_level: trust_level.into(),
            health_status: HEALTH_STATUS_HEALTHY.to_string(),
            lease_ttl_seconds,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_checked_at: now,
        }
    }

    pub fn new_http(
        id: impl Into<String>,
        capability_id: impl Into<String>,
        namespace: impl Into<String>,
        platform: impl Into<String>,
        trust_level: impl Into<String>,
        lease_ttl_seconds: i64,
        endpoint: impl Into<String>,
        request_timeout_ms: i64,
        credential_ref: Option<impl Into<String>>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            capability_id: capability_id.into(),
            namespace: namespace.into(),
            platform: platform.into(),
            transport_kind: TRANSPORT_KIND_HTTP_JSONRPC.to_string(),
            endpoint: Some(endpoint.into()),
            request_timeout_ms: request_timeout_ms.max(1),
            credential_ref: credential_ref.map(|value| value.into()),
            trust_level: trust_level.into(),
            health_status: HEALTH_STATUS_UNKNOWN.to_string(),
            lease_ttl_seconds,
            created_at: now.clone(),
            updated_at: now.clone(),
            last_checked_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCredentialRecord {
    pub id: String,
    pub namespace: String,
    pub credential_kind: String,
    pub header_name: String,
    pub auth_scheme: Option<String>,
    pub secret_present: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl McpCredentialRecord {
    pub fn bearer_token(id: impl Into<String>, namespace: impl Into<String>) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            namespace: namespace.into(),
            credential_kind: CREDENTIAL_KIND_BEARER_TOKEN.to_string(),
            header_name: "Authorization".to_string(),
            auth_scheme: Some("Bearer".to_string()),
            secret_present: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn static_header(
        id: impl Into<String>,
        namespace: impl Into<String>,
        header_name: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: id.into(),
            namespace: namespace.into(),
            credential_kind: CREDENTIAL_KIND_STATIC_HEADER.to_string(),
            header_name: header_name.into(),
            auth_scheme: None,
            secret_present: false,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone)]
struct StoredMcpCredentialRecord {
    record: McpCredentialRecord,
    secret_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentLeaseRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub capability_id: String,
    pub environment_type: String,
    pub sandbox_tier: String,
    pub status: String,
    pub heartbeat_at: String,
    pub expires_at: String,
    pub resume_token: String,
    pub created_at: String,
    pub updated_at: String,
}

impl EnvironmentLeaseRecord {
    pub fn new_requested(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        capability_id: impl Into<String>,
        environment_type: impl Into<String>,
        sandbox_tier: impl Into<String>,
        ttl_seconds: i64,
    ) -> Self {
        let now = current_timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id: id.clone(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            capability_id: capability_id.into(),
            environment_type: environment_type.into(),
            sandbox_tier: sandbox_tier.into(),
            status: "requested".to_string(),
            heartbeat_at: now.clone(),
            expires_at: expiry_from(&now, ttl_seconds),
            resume_token: format!("lease:{id}"),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpInvocationRecord {
    pub id: String,
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub capability_id: String,
    pub server_id: String,
    pub lease_id: String,
    pub namespace: String,
    pub tool_name: String,
    pub request_json: Value,
    pub response_json: Option<Value>,
    pub status: String,
    pub error_message: Option<String>,
    pub retryable: bool,
    pub trust_level: String,
    pub gate_status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl McpInvocationRecord {
    fn completed(
        workspace_id: impl Into<String>,
        project_id: impl Into<String>,
        run_id: impl Into<String>,
        task_id: impl Into<String>,
        capability_id: impl Into<String>,
        server_id: impl Into<String>,
        lease_id: impl Into<String>,
        namespace: impl Into<String>,
        tool_name: impl Into<String>,
        request_json: Value,
        response_json: Option<Value>,
        status: impl Into<String>,
        error_message: Option<String>,
        retryable: bool,
        trust_level: impl Into<String>,
        gate_status: impl Into<String>,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_id: workspace_id.into(),
            project_id: project_id.into(),
            run_id: run_id.into(),
            task_id: task_id.into(),
            capability_id: capability_id.into(),
            server_id: server_id.into(),
            lease_id: lease_id.into(),
            namespace: namespace.into(),
            tool_name: tool_name.into(),
            request_json,
            response_json,
            status: status.into(),
            error_message,
            retryable,
            trust_level: trust_level.into(),
            gate_status: gate_status.into(),
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayArtifactMetadata {
    pub provenance_source: String,
    pub source_descriptor_id: String,
    pub source_invocation_id: String,
    pub trust_level: String,
    pub knowledge_gate_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayExecutionSuccess {
    pub content: String,
    pub invocation: McpInvocationRecord,
    pub lease: EnvironmentLeaseRecord,
    pub artifact_metadata: GatewayArtifactMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayExecutionFailure {
    pub message: String,
    pub retryable: bool,
    pub invocation: McpInvocationRecord,
    pub lease: EnvironmentLeaseRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayExecutionOutcome {
    Succeeded(GatewayExecutionSuccess),
    Failed(GatewayExecutionFailure),
}

#[derive(Debug, Clone)]
pub struct GatewayRequest {
    pub workspace_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub capability_id: String,
    pub tool_name: String,
    pub arguments: Value,
    pub attempt: i64,
}

#[derive(Debug, Clone)]
struct TransportExecutionResult {
    content: Option<String>,
    response_json: Option<Value>,
    error_message: Option<String>,
    retryable: bool,
    health_status: String,
}

#[derive(Debug)]
enum CredentialResolutionError {
    Store(InteropStoreError),
    Normalized(TransportExecutionResult),
}

impl TransportExecutionResult {
    fn succeeded(content: String, response_json: Option<Value>) -> Self {
        Self {
            content: Some(content),
            response_json,
            error_message: None,
            retryable: false,
            health_status: HEALTH_STATUS_HEALTHY.to_string(),
        }
    }

    fn failed(
        message: impl Into<String>,
        retryable: bool,
        response_json: Option<Value>,
        health_status: impl Into<String>,
    ) -> Self {
        Self {
            content: None,
            response_json,
            error_message: Some(message.into()),
            retryable,
            health_status: health_status.into(),
        }
    }
}

#[derive(Debug, Clone)]
struct SimulatedTransport;

impl SimulatedTransport {
    async fn execute(&self, request: &GatewayRequest) -> TransportExecutionResult {
        match request.tool_name.as_str() {
            "emit_text" => match required_string(&request.arguments, "content", &request.tool_name)
            {
                Ok(content) => TransportExecutionResult::succeeded(
                    content.clone(),
                    Some(json!({ "content": content })),
                ),
                Err(message) => {
                    TransportExecutionResult::failed(message, false, None, HEALTH_STATUS_HEALTHY)
                }
            },
            "fail_once_then_emit_text" => {
                let failure_message = match required_string(
                    &request.arguments,
                    "failure_message",
                    &request.tool_name,
                ) {
                    Ok(value) => value,
                    Err(message) => {
                        return TransportExecutionResult::failed(
                            message,
                            false,
                            None,
                            HEALTH_STATUS_HEALTHY,
                        );
                    }
                };
                let content =
                    match required_string(&request.arguments, "content", &request.tool_name) {
                        Ok(value) => value,
                        Err(message) => {
                            return TransportExecutionResult::failed(
                                message,
                                false,
                                None,
                                HEALTH_STATUS_HEALTHY,
                            );
                        }
                    };

                if request.attempt <= 1 {
                    TransportExecutionResult::failed(
                        failure_message,
                        true,
                        None,
                        HEALTH_STATUS_HEALTHY,
                    )
                } else {
                    TransportExecutionResult::succeeded(
                        content.clone(),
                        Some(json!({ "content": content })),
                    )
                }
            }
            "always_fail" => {
                match required_string(&request.arguments, "message", &request.tool_name) {
                    Ok(message) => TransportExecutionResult::failed(
                        message,
                        false,
                        None,
                        HEALTH_STATUS_HEALTHY,
                    ),
                    Err(message) => TransportExecutionResult::failed(
                        message,
                        false,
                        None,
                        HEALTH_STATUS_HEALTHY,
                    ),
                }
            }
            _ => TransportExecutionResult::failed(
                format!("unsupported fake tool `{}`", request.tool_name),
                false,
                None,
                HEALTH_STATUS_HEALTHY,
            ),
        }
    }
}

#[derive(Debug, Clone)]
struct HttpMcpTransport {
    client: Client,
}

impl HttpMcpTransport {
    fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    async fn execute(
        &self,
        store: &SqliteInteropStore,
        server: &McpServerRecord,
        request: &GatewayRequest,
    ) -> Result<TransportExecutionResult, InteropStoreError> {
        if server.health_status == HEALTH_STATUS_DISABLED {
            return Ok(TransportExecutionResult::failed(
                format!("mcp server `{}` is disabled", server.id),
                false,
                None,
                HEALTH_STATUS_DISABLED,
            ));
        }

        let Some(endpoint) = server.endpoint.as_deref() else {
            return Ok(TransportExecutionResult::failed(
                format!("mcp server `{}` missing endpoint", server.id),
                false,
                None,
                HEALTH_STATUS_DEGRADED,
            ));
        };
        if endpoint.trim().is_empty() {
            return Ok(TransportExecutionResult::failed(
                format!("mcp server `{}` missing endpoint", server.id),
                false,
                None,
                HEALTH_STATUS_DEGRADED,
            ));
        }

        let headers = match self.resolve_headers(store, server).await {
            Ok(headers) => headers,
            Err(CredentialResolutionError::Store(error)) => return Err(error),
            Err(CredentialResolutionError::Normalized(result)) => return Ok(result),
        };
        let payload = json!({
            "jsonrpc": "2.0",
            "id": format!("{}:{}", request.run_id, request.attempt),
            "method": "tools/call",
            "params": {
                "name": request.tool_name,
                "arguments": request.arguments,
                "namespace": server.namespace,
            }
        });

        let response = match self
            .client
            .post(endpoint)
            .headers(headers)
            .timeout(StdDuration::from_millis(
                server.request_timeout_ms.max(1) as u64
            ))
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => response,
            Err(error) => return Ok(normalize_reqwest_error(error)),
        };

        let status = response.status();
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                return Ok(TransportExecutionResult::failed(
                    format!("mcp transport response read failed: {error}"),
                    true,
                    None,
                    HEALTH_STATUS_DEGRADED,
                ));
            }
        };

        if !status.is_success() {
            return Ok(normalize_http_status(status, body));
        }

        let response_json = match serde_json::from_str::<Value>(&body) {
            Ok(value) => value,
            Err(_) => {
                return Ok(TransportExecutionResult::failed(
                    "invalid JSON-RPC response from MCP server",
                    false,
                    Some(json!({ "raw_body": body })),
                    HEALTH_STATUS_DEGRADED,
                ));
            }
        };

        Ok(parse_jsonrpc_response(response_json))
    }

    async fn resolve_headers(
        &self,
        store: &SqliteInteropStore,
        server: &McpServerRecord,
    ) -> Result<HeaderMap, CredentialResolutionError> {
        let mut headers = HeaderMap::new();

        let Some(credential_ref) = server.credential_ref.as_deref() else {
            return Ok(headers);
        };

        let Some(credential) = store
            .fetch_stored_mcp_credential(credential_ref)
            .await
            .map_err(CredentialResolutionError::Store)?
        else {
            headers.insert(
                "x-octopus-missing-credential",
                HeaderValue::from_static("true"),
            );
            return Err(CredentialResolutionError::Normalized(
                TransportExecutionResult::failed(
                    format!("credential reference `{credential_ref}` not found"),
                    false,
                    None,
                    HEALTH_STATUS_DEGRADED,
                ),
            ));
        };

        if credential.secret_value.trim().is_empty() {
            return Err(CredentialResolutionError::Normalized(
                TransportExecutionResult::failed(
                    format!("credential reference `{credential_ref}` has no secret material"),
                    false,
                    None,
                    HEALTH_STATUS_DEGRADED,
                ),
            ));
        }

        let header_name = match HeaderName::from_bytes(credential.record.header_name.as_bytes()) {
            Ok(name) => name,
            Err(_) => {
                return Err(CredentialResolutionError::Normalized(
                    TransportExecutionResult::failed(
                        format!("credential reference `{credential_ref}` has invalid header name"),
                        false,
                        None,
                        HEALTH_STATUS_DEGRADED,
                    ),
                ));
            }
        };
        let header_value_raw = match credential.record.credential_kind.as_str() {
            CREDENTIAL_KIND_BEARER_TOKEN => format!(
                "{} {}",
                credential
                    .record
                    .auth_scheme
                    .as_deref()
                    .unwrap_or("Bearer")
                    .trim(),
                credential.secret_value
            ),
            CREDENTIAL_KIND_STATIC_HEADER => credential.secret_value.clone(),
            other => {
                return Err(CredentialResolutionError::Normalized(
                    TransportExecutionResult::failed(
                        format!(
                            "credential reference `{credential_ref}` uses unsupported kind `{other}`"
                        ),
                        false,
                        None,
                        HEALTH_STATUS_DEGRADED,
                    ),
                ));
            }
        };
        let header_value = match HeaderValue::from_str(header_value_raw.trim()) {
            Ok(value) => value,
            Err(_) => {
                return Err(CredentialResolutionError::Normalized(
                    TransportExecutionResult::failed(
                        format!(
                            "credential reference `{credential_ref}` produced invalid header value"
                        ),
                        false,
                        None,
                        HEALTH_STATUS_DEGRADED,
                    ),
                ));
            }
        };

        headers.insert(header_name, header_value);
        store
            .touch_mcp_credential(&credential.record.id)
            .await
            .map_err(CredentialResolutionError::Store)?;
        Ok(headers)
    }
}

#[derive(Debug, Error)]
pub enum InteropStoreError {
    #[error("mcp server for capability `{0}` not found")]
    McpServerNotFound(String),
    #[error("environment lease `{0}` not found")]
    EnvironmentLeaseNotFound(String),
    #[error("invalid environment lease transition for `{lease_id}`: `{from}` -> `{to}`")]
    InvalidEnvironmentLeaseTransition {
        lease_id: String,
        from: String,
        to: String,
    },
    #[error("invalid MCP tool arguments for `{tool_name}`: {message}")]
    InvalidToolArguments { tool_name: String, message: String },
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Clone)]
pub struct SqliteInteropStore {
    pool: SqlitePool,
}

impl SqliteInteropStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn upsert_mcp_server(
        &self,
        record: &McpServerRecord,
    ) -> Result<(), InteropStoreError> {
        sqlx::query(
            r#"
            INSERT INTO mcp_servers (
                id, capability_id, namespace, platform, transport_kind, endpoint,
                request_timeout_ms, credential_ref, trust_level, health_status,
                lease_ttl_seconds, created_at, updated_at, last_checked_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            ON CONFLICT(id) DO UPDATE SET
                capability_id = excluded.capability_id,
                namespace = excluded.namespace,
                platform = excluded.platform,
                transport_kind = excluded.transport_kind,
                endpoint = excluded.endpoint,
                request_timeout_ms = excluded.request_timeout_ms,
                credential_ref = excluded.credential_ref,
                trust_level = excluded.trust_level,
                health_status = excluded.health_status,
                lease_ttl_seconds = excluded.lease_ttl_seconds,
                updated_at = excluded.updated_at,
                last_checked_at = excluded.last_checked_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.capability_id)
        .bind(&record.namespace)
        .bind(&record.platform)
        .bind(&record.transport_kind)
        .bind(&record.endpoint)
        .bind(record.request_timeout_ms)
        .bind(&record.credential_ref)
        .bind(&record.trust_level)
        .bind(&record.health_status)
        .bind(record.lease_ttl_seconds)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .bind(&record.last_checked_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerRecord>, InteropStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, capability_id, namespace, platform, transport_kind, endpoint,
                   request_timeout_ms, credential_ref, trust_level, health_status,
                   lease_ttl_seconds, created_at, updated_at, last_checked_at
            FROM mcp_servers
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| mcp_server_from_row(&row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(InteropStoreError::from)
    }

    pub async fn fetch_mcp_server_by_capability_id(
        &self,
        capability_id: &str,
    ) -> Result<Option<McpServerRecord>, InteropStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, capability_id, namespace, platform, transport_kind, endpoint,
                   request_timeout_ms, credential_ref, trust_level, health_status,
                   lease_ttl_seconds, created_at, updated_at, last_checked_at
            FROM mcp_servers
            WHERE capability_id = ?1
            ORDER BY created_at ASC
            LIMIT 1
            "#,
        )
        .bind(capability_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| mcp_server_from_row(&row))
            .transpose()
            .map_err(InteropStoreError::from)
    }

    pub async fn upsert_mcp_credential(
        &self,
        record: &McpCredentialRecord,
        secret: &str,
    ) -> Result<(), InteropStoreError> {
        let secret_present = !secret.trim().is_empty();
        let updated_at = current_timestamp();
        let created_at = if record.created_at.is_empty() {
            updated_at.clone()
        } else {
            record.created_at.clone()
        };

        sqlx::query(
            r#"
            INSERT INTO mcp_credentials (
                id, namespace, credential_kind, header_name, auth_scheme, secret_value,
                secret_present, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                namespace = excluded.namespace,
                credential_kind = excluded.credential_kind,
                header_name = excluded.header_name,
                auth_scheme = excluded.auth_scheme,
                secret_value = excluded.secret_value,
                secret_present = excluded.secret_present,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.id)
        .bind(&record.namespace)
        .bind(&record.credential_kind)
        .bind(&record.header_name)
        .bind(&record.auth_scheme)
        .bind(secret)
        .bind(secret_present)
        .bind(created_at)
        .bind(updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_mcp_credentials(
        &self,
    ) -> Result<Vec<McpCredentialRecord>, InteropStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, namespace, credential_kind, header_name, auth_scheme,
                   secret_present, created_at, updated_at
            FROM mcp_credentials
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| mcp_credential_from_row(&row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(InteropStoreError::from)
    }

    pub async fn request_environment_lease(
        &self,
        workspace_id: &str,
        project_id: &str,
        run_id: &str,
        task_id: &str,
        capability_id: &str,
        environment_type: &str,
        sandbox_tier: &str,
        ttl_seconds: i64,
    ) -> Result<EnvironmentLeaseRecord, InteropStoreError> {
        let mut lease = EnvironmentLeaseRecord::new_requested(
            workspace_id,
            project_id,
            run_id,
            task_id,
            capability_id,
            environment_type,
            sandbox_tier,
            ttl_seconds,
        );
        self.insert_environment_lease(&lease).await?;

        lease.status = "granted".to_string();
        lease.updated_at = current_timestamp();
        self.update_environment_lease(&lease).await?;

        lease.status = "active".to_string();
        lease.heartbeat_at = current_timestamp();
        lease.expires_at = expiry_from(&lease.heartbeat_at, ttl_seconds);
        lease.updated_at = lease.heartbeat_at.clone();
        self.update_environment_lease(&lease).await?;

        Ok(lease)
    }

    pub async fn heartbeat_environment_lease(
        &self,
        lease_id: &str,
        ttl_seconds: i64,
    ) -> Result<EnvironmentLeaseRecord, InteropStoreError> {
        let mut lease = self
            .fetch_environment_lease(lease_id)
            .await?
            .ok_or_else(|| InteropStoreError::EnvironmentLeaseNotFound(lease_id.to_string()))?;

        if lease.status != "active" {
            return Err(InteropStoreError::InvalidEnvironmentLeaseTransition {
                lease_id: lease.id,
                from: lease.status,
                to: "active".to_string(),
            });
        }

        lease.heartbeat_at = current_timestamp();
        lease.expires_at = expiry_from(&lease.heartbeat_at, ttl_seconds);
        lease.updated_at = lease.heartbeat_at.clone();
        self.update_environment_lease(&lease).await?;
        Ok(lease)
    }

    pub async fn release_environment_lease(
        &self,
        lease_id: &str,
    ) -> Result<EnvironmentLeaseRecord, InteropStoreError> {
        let mut lease = self
            .fetch_environment_lease(lease_id)
            .await?
            .ok_or_else(|| InteropStoreError::EnvironmentLeaseNotFound(lease_id.to_string()))?;

        if !matches!(lease.status.as_str(), "requested" | "granted" | "active") {
            return Err(InteropStoreError::InvalidEnvironmentLeaseTransition {
                lease_id: lease.id,
                from: lease.status,
                to: "released".to_string(),
            });
        }

        lease.status = "released".to_string();
        lease.updated_at = current_timestamp();
        self.update_environment_lease(&lease).await?;
        Ok(lease)
    }

    pub async fn list_environment_leases_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<EnvironmentLeaseRecord>, InteropStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, capability_id, environment_type,
                   sandbox_tier, status, heartbeat_at, expires_at, resume_token, created_at, updated_at
            FROM environment_leases
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| environment_lease_from_row(&row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(InteropStoreError::from)
    }

    pub async fn expire_stale_environment_leases(&self) -> Result<u64, InteropStoreError> {
        let now = current_timestamp();
        let result = sqlx::query(
            r#"
            UPDATE environment_leases
            SET status = 'expired', updated_at = ?1
            WHERE status = 'active' AND expires_at < ?1
            "#,
        )
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn insert_mcp_invocation(
        &self,
        record: &McpInvocationRecord,
    ) -> Result<(), InteropStoreError> {
        sqlx::query(
            r#"
            INSERT INTO mcp_invocations (
                id, workspace_id, project_id, run_id, task_id, capability_id, server_id, lease_id,
                namespace, tool_name, request_json, response_json, status, error_message, retryable,
                trust_level, gate_status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
            "#,
        )
        .bind(&record.id)
        .bind(&record.workspace_id)
        .bind(&record.project_id)
        .bind(&record.run_id)
        .bind(&record.task_id)
        .bind(&record.capability_id)
        .bind(&record.server_id)
        .bind(&record.lease_id)
        .bind(&record.namespace)
        .bind(&record.tool_name)
        .bind(serde_json::to_string(&record.request_json)?)
        .bind(record.response_json.as_ref().map(serde_json::to_string).transpose()?)
        .bind(&record.status)
        .bind(&record.error_message)
        .bind(record.retryable)
        .bind(&record.trust_level)
        .bind(&record.gate_status)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn list_mcp_invocations_by_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<McpInvocationRecord>, InteropStoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, capability_id, server_id, lease_id,
                   namespace, tool_name, request_json, response_json, status, error_message, retryable,
                   trust_level, gate_status, created_at, updated_at
            FROM mcp_invocations
            WHERE run_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| mcp_invocation_from_row(&row))
            .collect::<Result<Vec<_>, _>>()
            .map_err(InteropStoreError::from)
    }

    async fn fetch_stored_mcp_credential(
        &self,
        credential_id: &str,
    ) -> Result<Option<StoredMcpCredentialRecord>, InteropStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, namespace, credential_kind, header_name, auth_scheme,
                   secret_value, secret_present, created_at, updated_at
            FROM mcp_credentials
            WHERE id = ?1
            LIMIT 1
            "#,
        )
        .bind(credential_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| stored_mcp_credential_from_row(&row))
            .transpose()
            .map_err(InteropStoreError::from)
    }

    async fn touch_mcp_credential(&self, credential_id: &str) -> Result<(), InteropStoreError> {
        sqlx::query(
            r#"
            UPDATE mcp_credentials
            SET updated_at = ?2
            WHERE id = ?1
            "#,
        )
        .bind(credential_id)
        .bind(current_timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_mcp_server_health(
        &self,
        server_id: &str,
        health_status: &str,
    ) -> Result<(), InteropStoreError> {
        let now = current_timestamp();
        sqlx::query(
            r#"
            UPDATE mcp_servers
            SET health_status = ?2,
                last_checked_at = ?3,
                updated_at = ?3
            WHERE id = ?1
            "#,
        )
        .bind(server_id)
        .bind(health_status)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn fetch_environment_lease(
        &self,
        lease_id: &str,
    ) -> Result<Option<EnvironmentLeaseRecord>, InteropStoreError> {
        let row = sqlx::query(
            r#"
            SELECT id, workspace_id, project_id, run_id, task_id, capability_id, environment_type,
                   sandbox_tier, status, heartbeat_at, expires_at, resume_token, created_at, updated_at
            FROM environment_leases
            WHERE id = ?1
            "#,
        )
        .bind(lease_id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| environment_lease_from_row(&row))
            .transpose()
            .map_err(InteropStoreError::from)
    }

    async fn insert_environment_lease(
        &self,
        lease: &EnvironmentLeaseRecord,
    ) -> Result<(), InteropStoreError> {
        sqlx::query(
            r#"
            INSERT INTO environment_leases (
                id, workspace_id, project_id, run_id, task_id, capability_id, environment_type,
                sandbox_tier, status, heartbeat_at, expires_at, resume_token, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            "#,
        )
        .bind(&lease.id)
        .bind(&lease.workspace_id)
        .bind(&lease.project_id)
        .bind(&lease.run_id)
        .bind(&lease.task_id)
        .bind(&lease.capability_id)
        .bind(&lease.environment_type)
        .bind(&lease.sandbox_tier)
        .bind(&lease.status)
        .bind(&lease.heartbeat_at)
        .bind(&lease.expires_at)
        .bind(&lease.resume_token)
        .bind(&lease.created_at)
        .bind(&lease.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn update_environment_lease(
        &self,
        lease: &EnvironmentLeaseRecord,
    ) -> Result<(), InteropStoreError> {
        sqlx::query(
            r#"
            UPDATE environment_leases
            SET status = ?2,
                heartbeat_at = ?3,
                expires_at = ?4,
                updated_at = ?5
            WHERE id = ?1
            "#,
        )
        .bind(&lease.id)
        .bind(&lease.status)
        .bind(&lease.heartbeat_at)
        .bind(&lease.expires_at)
        .bind(&lease.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct McpGateway {
    store: SqliteInteropStore,
    simulated_transport: SimulatedTransport,
    http_transport: HttpMcpTransport,
}

impl McpGateway {
    pub fn new(store: SqliteInteropStore) -> Self {
        Self {
            store,
            simulated_transport: SimulatedTransport,
            http_transport: HttpMcpTransport::new(),
        }
    }

    pub async fn execute(
        &self,
        request: GatewayRequest,
    ) -> Result<GatewayExecutionOutcome, InteropStoreError> {
        let server = self
            .store
            .fetch_mcp_server_by_capability_id(&request.capability_id)
            .await?
            .ok_or_else(|| InteropStoreError::McpServerNotFound(request.capability_id.clone()))?;

        let lease = self
            .store
            .request_environment_lease(
                &request.workspace_id,
                &request.project_id,
                &request.run_id,
                &request.task_id,
                &request.capability_id,
                "mcp_tool_call",
                "ephemeral_restricted",
                server.lease_ttl_seconds,
            )
            .await?;

        let execution = self.execute_transport(&server, &request).await?;
        self.store
            .update_mcp_server_health(&server.id, &execution.health_status)
            .await?;

        let gate_status = gate_status_for_trust(server.trust_level.as_str()).to_string();
        let invocation = McpInvocationRecord::completed(
            &request.workspace_id,
            &request.project_id,
            &request.run_id,
            &request.task_id,
            &request.capability_id,
            &server.id,
            &lease.id,
            &server.namespace,
            &request.tool_name,
            request.arguments.clone(),
            execution.response_json.clone(),
            if execution.error_message.is_some() {
                "failed"
            } else {
                "succeeded"
            },
            execution.error_message.clone(),
            execution.retryable,
            &server.trust_level,
            &gate_status,
        );
        self.store.insert_mcp_invocation(&invocation).await?;
        let released_lease = self.store.release_environment_lease(&lease.id).await?;

        if let Some(message) = execution.error_message {
            return Ok(GatewayExecutionOutcome::Failed(GatewayExecutionFailure {
                message,
                retryable: execution.retryable,
                invocation,
                lease: released_lease,
            }));
        }

        Ok(GatewayExecutionOutcome::Succeeded(
            GatewayExecutionSuccess {
                content: execution.content.unwrap_or_default(),
                invocation: invocation.clone(),
                lease: released_lease,
                artifact_metadata: GatewayArtifactMetadata {
                    provenance_source: PROVENANCE_SOURCE_MCP_CONNECTOR.to_string(),
                    source_descriptor_id: request.capability_id,
                    source_invocation_id: invocation.id.clone(),
                    trust_level: invocation.trust_level.clone(),
                    knowledge_gate_status: invocation.gate_status.clone(),
                },
            },
        ))
    }

    async fn execute_transport(
        &self,
        server: &McpServerRecord,
        request: &GatewayRequest,
    ) -> Result<TransportExecutionResult, InteropStoreError> {
        match server.transport_kind.as_str() {
            TRANSPORT_KIND_SIMULATED => Ok(self.simulated_transport.execute(request).await),
            TRANSPORT_KIND_HTTP_JSONRPC => {
                self.http_transport
                    .execute(&self.store, server, request)
                    .await
            }
            other => Ok(TransportExecutionResult::failed(
                format!("unsupported MCP transport `{other}`"),
                false,
                None,
                HEALTH_STATUS_DEGRADED,
            )),
        }
    }
}

fn required_string(arguments: &Value, field: &str, tool_name: &str) -> Result<String, String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            format!("invalid MCP tool arguments for `{tool_name}`: missing `{field}` string")
        })
}

fn normalize_reqwest_error(error: reqwest::Error) -> TransportExecutionResult {
    if error.is_timeout() {
        return TransportExecutionResult::failed(
            "mcp transport timeout",
            true,
            Some(json!({ "error": error.to_string() })),
            HEALTH_STATUS_DEGRADED,
        );
    }

    if error.is_connect() {
        return TransportExecutionResult::failed(
            "mcp transport connection failed",
            true,
            Some(json!({ "error": error.to_string() })),
            HEALTH_STATUS_DEGRADED,
        );
    }

    TransportExecutionResult::failed(
        format!("mcp transport request failed: {error}"),
        true,
        Some(json!({ "error": error.to_string() })),
        HEALTH_STATUS_DEGRADED,
    )
}

fn normalize_http_status(status: StatusCode, body: String) -> TransportExecutionResult {
    let retryable = matches!(
        status,
        StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    ) || status.is_server_error();
    let message = match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
            "mcp transport unauthorized".to_string()
        }
        _ => format!("mcp transport returned HTTP {}", status.as_u16()),
    };

    TransportExecutionResult::failed(
        message,
        retryable,
        Some(json!({
            "http_status": status.as_u16(),
            "body": body,
        })),
        HEALTH_STATUS_DEGRADED,
    )
}

fn parse_jsonrpc_response(response_json: Value) -> TransportExecutionResult {
    if let Some(error) = response_json.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("mcp JSON-RPC error")
            .to_string();
        let retryable = error
            .get("data")
            .and_then(|data| data.get("retryable"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        return TransportExecutionResult::failed(
            message,
            retryable,
            Some(response_json),
            HEALTH_STATUS_DEGRADED,
        );
    }

    let Some(result) = response_json.get("result") else {
        return TransportExecutionResult::failed(
            "invalid JSON-RPC response from MCP server",
            false,
            Some(response_json),
            HEALTH_STATUS_DEGRADED,
        );
    };

    let Some(content) = extract_result_content(result) else {
        return TransportExecutionResult::failed(
            "invalid JSON-RPC response from MCP server",
            false,
            Some(response_json),
            HEALTH_STATUS_DEGRADED,
        );
    };

    TransportExecutionResult::succeeded(content, Some(response_json))
}

fn extract_result_content(result: &Value) -> Option<String> {
    if let Some(content) = result.get("content").and_then(Value::as_str) {
        return Some(content.to_string());
    }

    if let Some(text) = result.get("text").and_then(Value::as_str) {
        return Some(text.to_string());
    }

    let items = result.get("content")?.as_array()?;
    let texts: Vec<&str> = items
        .iter()
        .filter_map(|item| match item {
            Value::String(text) => Some(text.as_str()),
            Value::Object(_) => item.get("text").and_then(Value::as_str),
            _ => None,
        })
        .collect();

    if texts.is_empty() {
        None
    } else {
        Some(texts.join("\n"))
    }
}

fn gate_status_for_trust(trust_level: &str) -> &'static str {
    if trust_level == TRUST_LEVEL_TRUSTED {
        KNOWLEDGE_GATE_ELIGIBLE
    } else {
        KNOWLEDGE_GATE_BLOCKED_LOW_TRUST
    }
}

fn mcp_server_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<McpServerRecord, sqlx::Error> {
    Ok(McpServerRecord {
        id: row.try_get("id")?,
        capability_id: row.try_get("capability_id")?,
        namespace: row.try_get("namespace")?,
        platform: row.try_get("platform")?,
        transport_kind: row.try_get("transport_kind")?,
        endpoint: row.try_get("endpoint")?,
        request_timeout_ms: row.try_get("request_timeout_ms")?,
        credential_ref: row.try_get("credential_ref")?,
        trust_level: row.try_get("trust_level")?,
        health_status: row.try_get("health_status")?,
        lease_ttl_seconds: row.try_get("lease_ttl_seconds")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
        last_checked_at: row.try_get("last_checked_at")?,
    })
}

fn mcp_credential_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<McpCredentialRecord, sqlx::Error> {
    Ok(McpCredentialRecord {
        id: row.try_get("id")?,
        namespace: row.try_get("namespace")?,
        credential_kind: row.try_get("credential_kind")?,
        header_name: row.try_get("header_name")?,
        auth_scheme: row.try_get("auth_scheme")?,
        secret_present: row.try_get("secret_present")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn stored_mcp_credential_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<StoredMcpCredentialRecord, sqlx::Error> {
    Ok(StoredMcpCredentialRecord {
        record: McpCredentialRecord {
            id: row.try_get("id")?,
            namespace: row.try_get("namespace")?,
            credential_kind: row.try_get("credential_kind")?,
            header_name: row.try_get("header_name")?,
            auth_scheme: row.try_get("auth_scheme")?,
            secret_present: row.try_get("secret_present")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        },
        secret_value: row.try_get("secret_value")?,
    })
}

fn environment_lease_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<EnvironmentLeaseRecord, sqlx::Error> {
    Ok(EnvironmentLeaseRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        run_id: row.try_get("run_id")?,
        task_id: row.try_get("task_id")?,
        capability_id: row.try_get("capability_id")?,
        environment_type: row.try_get("environment_type")?,
        sandbox_tier: row.try_get("sandbox_tier")?,
        status: row.try_get("status")?,
        heartbeat_at: row.try_get("heartbeat_at")?,
        expires_at: row.try_get("expires_at")?,
        resume_token: row.try_get("resume_token")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn mcp_invocation_from_row(
    row: &sqlx::sqlite::SqliteRow,
) -> Result<McpInvocationRecord, sqlx::Error> {
    let request_json_raw: String = row.try_get("request_json")?;
    let response_json_raw: Option<String> = row.try_get("response_json")?;
    Ok(McpInvocationRecord {
        id: row.try_get("id")?,
        workspace_id: row.try_get("workspace_id")?,
        project_id: row.try_get("project_id")?,
        run_id: row.try_get("run_id")?,
        task_id: row.try_get("task_id")?,
        capability_id: row.try_get("capability_id")?,
        server_id: row.try_get("server_id")?,
        lease_id: row.try_get("lease_id")?,
        namespace: row.try_get("namespace")?,
        tool_name: row.try_get("tool_name")?,
        request_json: parse_json_value(&request_json_raw)?,
        response_json: response_json_raw
            .as_deref()
            .map(parse_json_value)
            .transpose()?,
        status: row.try_get("status")?,
        error_message: row.try_get("error_message")?,
        retryable: row.try_get("retryable")?,
        trust_level: row.try_get("trust_level")?,
        gate_status: row.try_get("gate_status")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn expiry_from(now: &str, ttl_seconds: i64) -> String {
    chrono::DateTime::parse_from_rfc3339(now)
        .map(|value| value + Duration::seconds(ttl_seconds.max(1)))
        .unwrap_or_else(|_| Utc::now().fixed_offset() + Duration::seconds(ttl_seconds.max(1)))
        .to_rfc3339()
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn parse_json_value(raw: &str) -> Result<Value, sqlx::Error> {
    serde_json::from_str(raw).map_err(|error| sqlx::Error::Decode(Box::new(error)))
}
