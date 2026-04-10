use std::future::Future;
use std::io;
use std::time::Duration;

use tokio::time::timeout;

use super::mcp_transport::{default_initialize_params, spawn_mcp_stdio_process};
use super::{McpServerManager, McpServerManagerError, MCP_INITIALIZE_TIMEOUT_MS};

impl McpServerManager {
    pub async fn shutdown(&mut self) -> Result<(), McpServerManagerError> {
        let server_names = self.servers.keys().cloned().collect::<Vec<_>>();
        for server_name in server_names {
            let server = self.server_mut(&server_name)?;
            if let Some(process) = server.process.as_mut() {
                process.shutdown().await?;
            }
            server.process = None;
            server.initialized = false;
        }
        Ok(())
    }

    pub(super) fn server_process_exited(
        &mut self,
        server_name: &str,
    ) -> Result<bool, McpServerManagerError> {
        let server = self.server_mut(server_name)?;
        match server.process.as_mut() {
            Some(process) => Ok(process.has_exited()?),
            None => Ok(false),
        }
    }

    pub(super) async fn reset_server(
        &mut self,
        server_name: &str,
    ) -> Result<(), McpServerManagerError> {
        let mut process = {
            let server = self.server_mut(server_name)?;
            server.initialized = false;
            server.process.take()
        };

        if let Some(process) = process.as_mut() {
            let _ = process.shutdown().await;
        }

        Ok(())
    }

    pub(super) fn is_retryable_error(error: &McpServerManagerError) -> bool {
        matches!(
            error,
            McpServerManagerError::Transport { .. } | McpServerManagerError::Timeout { .. }
        )
    }

    pub(super) fn should_reset_server(error: &McpServerManagerError) -> bool {
        matches!(
            error,
            McpServerManagerError::Transport { .. }
                | McpServerManagerError::Timeout { .. }
                | McpServerManagerError::InvalidResponse { .. }
        )
    }

    pub(super) async fn run_process_request<T, F>(
        server_name: &str,
        method: &'static str,
        timeout_ms: u64,
        future: F,
    ) -> Result<T, McpServerManagerError>
    where
        F: Future<Output = io::Result<T>>,
    {
        match timeout(Duration::from_millis(timeout_ms), future).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) if error.kind() == io::ErrorKind::InvalidData => {
                Err(McpServerManagerError::InvalidResponse {
                    server_name: server_name.to_string(),
                    method,
                    details: error.to_string(),
                })
            }
            Ok(Err(source)) => Err(McpServerManagerError::Transport {
                server_name: server_name.to_string(),
                method,
                source,
            }),
            Err(_) => Err(McpServerManagerError::Timeout {
                server_name: server_name.to_string(),
                method,
                timeout_ms,
            }),
        }
    }

    pub(super) async fn ensure_server_ready(
        &mut self,
        server_name: &str,
    ) -> Result<(), McpServerManagerError> {
        if self.server_process_exited(server_name)? {
            self.reset_server(server_name).await?;
        }

        let mut attempts = 0;
        loop {
            let needs_spawn = self
                .servers
                .get(server_name)
                .map(|server| server.process.is_none())
                .ok_or_else(|| McpServerManagerError::UnknownServer {
                    server_name: server_name.to_string(),
                })?;

            if needs_spawn {
                let server = self.server_mut(server_name)?;
                server.process = Some(spawn_mcp_stdio_process(&server.bootstrap)?);
                server.initialized = false;
            }

            let needs_initialize = self
                .servers
                .get(server_name)
                .map(|server| !server.initialized)
                .ok_or_else(|| McpServerManagerError::UnknownServer {
                    server_name: server_name.to_string(),
                })?;

            if !needs_initialize {
                return Ok(());
            }

            let request_id = self.take_request_id();
            let response = {
                let server = self.server_mut(server_name)?;
                let process = server.process.as_mut().ok_or_else(|| {
                    McpServerManagerError::InvalidResponse {
                        server_name: server_name.to_string(),
                        method: "initialize",
                        details: "server process missing before initialize".to_string(),
                    }
                })?;
                Self::run_process_request(
                    server_name,
                    "initialize",
                    MCP_INITIALIZE_TIMEOUT_MS,
                    process.initialize(request_id, default_initialize_params()),
                )
                .await
            };

            let response = match response {
                Ok(response) => response,
                Err(error) if attempts == 0 && Self::is_retryable_error(&error) => {
                    self.reset_server(server_name).await?;
                    attempts += 1;
                    continue;
                }
                Err(error) => {
                    if Self::should_reset_server(&error) {
                        self.reset_server(server_name).await?;
                    }
                    return Err(error);
                }
            };

            if let Some(error) = response.error {
                return Err(McpServerManagerError::JsonRpc {
                    server_name: server_name.to_string(),
                    method: "initialize",
                    error,
                });
            }

            if response.result.is_none() {
                let error = McpServerManagerError::InvalidResponse {
                    server_name: server_name.to_string(),
                    method: "initialize",
                    details: "missing result payload".to_string(),
                };
                self.reset_server(server_name).await?;
                return Err(error);
            }

            let server = self.server_mut(server_name)?;
            server.initialized = true;
            return Ok(());
        }
    }
}
