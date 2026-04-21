use std::{
    collections::HashMap,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

use crate::{
    InitializeResult, McpClient, McpError, McpLifecyclePhase, McpTransport, TransportKind,
};

pub struct McpServerManager {
    servers: Arc<Mutex<HashMap<String, ServerHandle>>>,
}

pub struct ServerHandle {
    pub client: Arc<McpClient>,
    pub kind: TransportKind,
    pub phase: McpLifecyclePhase,
    stdio_child: Option<Child>,
}

pub struct McpServerSpec {
    pub server_id: String,
    pub transport: McpServerTransport,
}

pub enum McpServerTransport {
    Stdio {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
        transport: Arc<dyn McpTransport>,
    },
    Http {
        transport: Arc<dyn McpTransport>,
    },
    Sdk {
        transport: Arc<dyn McpTransport>,
    },
}

impl McpServerManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            servers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn spawn(&self, spec: McpServerSpec) -> Result<String, McpError> {
        let server_id = spec.server_id;
        let (kind, transport, stdio_child) = match spec.transport {
            McpServerTransport::Stdio {
                command,
                args,
                env,
                transport,
            } => (
                TransportKind::Stdio,
                transport,
                Some(spawn_stdio_child(&command, &args, &env)?),
            ),
            McpServerTransport::Http { transport } => (TransportKind::Http, transport, None),
            McpServerTransport::Sdk { transport } => (TransportKind::Sdk, transport, None),
        };

        let client = Arc::new(McpClient::new(server_id.clone(), transport));
        let InitializeResult { .. } = client.initialize().await?;

        let mut servers = self.servers.lock().expect("servers lock should work");
        if servers.contains_key(&server_id) {
            return Err(McpError::Protocol {
                message: format!("mcp server `{server_id}` already exists"),
            });
        }

        servers.insert(
            server_id.clone(),
            ServerHandle {
                client,
                kind,
                phase: McpLifecyclePhase::Ready,
                stdio_child,
            },
        );

        Ok(server_id)
    }

    pub async fn shutdown(&self, server_id: &str) -> Result<(), McpError> {
        let mut servers = self.servers.lock().expect("servers lock should work");
        let Some(mut handle) = servers.remove(server_id) else {
            return Ok(());
        };

        handle.phase = McpLifecyclePhase::Stopped;
        if let Some(child) = handle.stdio_child.as_mut() {
            terminate_child(child)?;
        }

        Ok(())
    }

    #[must_use]
    pub fn get_client(&self, server_id: &str) -> Option<Arc<McpClient>> {
        self.servers
            .lock()
            .expect("servers lock should work")
            .get(server_id)
            .map(|handle| Arc::clone(&handle.client))
    }

    #[must_use]
    pub fn list_servers(&self) -> Vec<String> {
        let mut servers = self
            .servers
            .lock()
            .expect("servers lock should work")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        servers.sort();
        servers
    }
}

impl Default for McpServerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for McpServerManager {
    fn drop(&mut self) {
        if let Ok(mut servers) = self.servers.lock() {
            for handle in servers.values_mut() {
                handle.phase = McpLifecyclePhase::Stopped;
                if let Some(child) = handle.stdio_child.as_mut() {
                    let _ = terminate_child(child);
                }
            }
        }
    }
}

fn spawn_stdio_child(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<Child, McpError> {
    let mut process = Command::new(command);
    process
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    for (key, value) in env {
        process.env(key, value);
    }

    process.spawn().map_err(|error| McpError::Transport {
        message: error.to_string(),
    })
}

fn terminate_child(child: &mut Child) -> Result<(), McpError> {
    match child.try_wait() {
        Ok(Some(_)) => Ok(()),
        Ok(None) => {
            child.kill().map_err(|error| McpError::Transport {
                message: error.to_string(),
            })?;
            child.wait().map_err(|error| McpError::Transport {
                message: error.to_string(),
            })?;
            Ok(())
        }
        Err(error) => Err(McpError::Transport {
            message: error.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashMap, VecDeque},
        process::Command,
        sync::{Arc, Mutex},
    };

    use async_trait::async_trait;
    use serde_json::json;

    use super::{McpServerManager, McpServerSpec, McpServerTransport};
    use crate::{JsonRpcRequest, JsonRpcResponse, McpError, McpTransport};

    struct MockTransport {
        responses: Arc<Mutex<VecDeque<JsonRpcResponse>>>,
    }

    impl MockTransport {
        fn initialized() -> Arc<Self> {
            Arc::new(Self {
                responses: Arc::new(Mutex::new(VecDeque::from(vec![JsonRpcResponse::success(
                    json!(1),
                    json!({ "protocolVersion": "2025-03-26" }),
                )]))),
            })
        }
    }

    #[async_trait]
    impl McpTransport for MockTransport {
        async fn call(&self, _req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
            self.responses
                .lock()
                .expect("responses lock should work")
                .pop_front()
                .ok_or_else(|| McpError::InvalidResponse {
                    body_preview: "missing mock response".into(),
                })
        }
    }

    #[tokio::test]
    async fn manager_spawns_stdio_client_and_shuts_down() {
        let manager = McpServerManager::new();
        let server_id = manager
            .spawn(McpServerSpec {
                server_id: "server-1".into(),
                transport: McpServerTransport::Stdio {
                    command: "/bin/sh".into(),
                    args: vec!["-c".into(), "sleep 60".into()],
                    env: HashMap::new(),
                    transport: MockTransport::initialized(),
                },
            })
            .await
            .expect("spawn should succeed");

        assert_eq!(server_id, "server-1");
        assert!(manager.get_client("server-1").is_some());
        manager
            .shutdown("server-1")
            .await
            .expect("shutdown should succeed");
        assert!(manager.get_client("server-1").is_none());
    }

    #[tokio::test]
    async fn manager_drop_reaps_stdio_child() {
        let manager = McpServerManager::new();
        manager
            .spawn(McpServerSpec {
                server_id: "server-1".into(),
                transport: McpServerTransport::Stdio {
                    command: "/bin/sh".into(),
                    args: vec!["-c".into(), "sleep 60".into()],
                    env: HashMap::new(),
                    transport: MockTransport::initialized(),
                },
            })
            .await
            .expect("spawn should succeed");

        let pid = manager
            .servers
            .lock()
            .expect("servers lock should work")
            .get("server-1")
            .and_then(|handle| handle.stdio_child.as_ref())
            .map(std::process::Child::id)
            .expect("stdio child pid should exist");

        drop(manager);

        let status = Command::new("ps")
            .args(["-p", &pid.to_string()])
            .status()
            .expect("ps should run");

        assert!(!status.success());
    }

    #[tokio::test]
    async fn manager_shutdown_is_idempotent() {
        let manager = McpServerManager::new();
        manager
            .shutdown("missing")
            .await
            .expect("shutdown on missing server should be ok");

        manager
            .spawn(McpServerSpec {
                server_id: "server-1".into(),
                transport: McpServerTransport::Http {
                    transport: MockTransport::initialized(),
                },
            })
            .await
            .expect("spawn should succeed");

        manager
            .shutdown("server-1")
            .await
            .expect("first shutdown should succeed");
        manager
            .shutdown("server-1")
            .await
            .expect("second shutdown should also succeed");
    }
}
