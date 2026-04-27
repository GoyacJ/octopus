use std::{collections::BTreeMap, process::Stdio, sync::Arc};

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::{
    process::{Child, Command},
    sync::{oneshot, Mutex},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

use crate::{
    call_tool_request, decode_list_tools, decode_tool_result, initialize_request,
    initialized_notification, list_tools_request, response_key, JsonRpcNotification, JsonRpcPeer,
    JsonRpcRequest, JsonRpcResponse, McpConnection, McpError, McpServerSpec, McpToolDescriptor,
    McpToolResult, McpTransport, StdioEnv, TransportChoice,
};

type PendingMap = Arc<
    Mutex<std::collections::HashMap<String, oneshot::Sender<Result<JsonRpcResponse, McpError>>>>,
>;

#[derive(Default)]
pub struct StdioTransport;

impl StdioTransport {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_env(
        env: &StdioEnv,
        parent: &BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        match env {
            StdioEnv::Allowlist { inherit, extra } => {
                let mut resolved = parent
                    .iter()
                    .filter(|(key, _)| inherit.contains(*key))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<BTreeMap<_, _>>();
                resolved.extend(extra.clone());
                resolved
            }
            StdioEnv::InheritWithDeny { deny, extra } => {
                let mut resolved = parent
                    .iter()
                    .filter(|(key, _)| {
                        !deny.iter().any(|pattern| env_pattern_matches(pattern, key))
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect::<BTreeMap<_, _>>();
                resolved.extend(extra.clone());
                resolved
            }
            StdioEnv::Empty { extra } => extra.clone(),
        }
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    fn transport_id(&self) -> &'static str {
        "stdio"
    }

    async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>, McpError> {
        let TransportChoice::Stdio {
            command,
            args,
            env,
            policy,
        } = spec.transport.clone()
        else {
            return Err(McpError::Unsupported(
                "StdioTransport requires TransportChoice::Stdio".into(),
            ));
        };

        let parent = std::env::vars().collect::<BTreeMap<_, _>>();
        let resolved_env = Self::resolve_env(&env, &parent);
        let mut command_builder = Command::new(&command);
        command_builder
            .args(args)
            .env_clear()
            .envs(resolved_env)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(working_dir) = policy.working_dir {
            command_builder.current_dir(working_dir);
        }

        let mut child = command_builder
            .spawn()
            .map_err(|error| McpError::Transport(error.to_string()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Transport("stdio child missing stdin".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Transport("stdio child missing stdout".into()))?;
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(async move {
                let mut lines = FramedRead::new(stderr, LinesCodec::new());
                while lines.next().await.is_some() {}
            });
        }

        let pending = Arc::new(Mutex::new(std::collections::HashMap::new()));
        spawn_reader(stdout, Arc::clone(&pending));

        let connection = Arc::new(StdioConnection {
            connection_id: format!("stdio:{}", spec.server_id.0),
            writer: Arc::new(Mutex::new(FramedWrite::new(stdin, LinesCodec::new()))),
            pending,
            child: Arc::new(Mutex::new(Some(child))),
            timeout: spec.timeouts.call_default,
            peer: JsonRpcPeer::new(),
        });
        connection
            .send(initialize_request(&connection.peer))
            .await?;
        connection
            .send_notification(initialized_notification())
            .await?;
        Ok(connection)
    }
}

pub struct StdioConnection {
    connection_id: String,
    writer: Arc<Mutex<FramedWrite<tokio::process::ChildStdin, LinesCodec>>>,
    pending: PendingMap,
    child: Arc<Mutex<Option<Child>>>,
    timeout: std::time::Duration,
    peer: JsonRpcPeer,
}

impl StdioConnection {
    async fn send(&self, request: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let key = response_key(&request.id);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(key.clone(), sender);

        let payload = serde_json::to_string(&request)
            .map_err(|error| McpError::InvalidResponse(error.to_string()))?;
        if let Err(error) = self.writer.lock().await.send(payload).await {
            self.pending.lock().await.remove(&key);
            return Err(McpError::Transport(error.to_string()));
        }

        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(McpError::Connection("stdio response channel closed".into())),
            Err(_) => {
                self.pending.lock().await.remove(&key);
                Err(McpError::Connection(format!(
                    "stdio request timed out: {}",
                    request.method
                )))
            }
        }
    }

    async fn send_notification(&self, notification: JsonRpcNotification) -> Result<(), McpError> {
        let payload = serde_json::to_string(&notification)
            .map_err(|error| McpError::InvalidResponse(error.to_string()))?;
        self.writer
            .lock()
            .await
            .send(payload)
            .await
            .map_err(|error| McpError::Transport(error.to_string()))
    }
}

#[async_trait]
impl McpConnection for StdioConnection {
    fn connection_id(&self) -> &str {
        &self.connection_id
    }

    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError> {
        decode_list_tools(self.send(list_tools_request(&self.peer)).await?)
    }

    async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult, McpError> {
        decode_tool_result(self.send(call_tool_request(&self.peer, name, args)).await?)
    }

    async fn shutdown(&self) -> Result<(), McpError> {
        let _ = self
            .send_notification(JsonRpcNotification::new("shutdown", None))
            .await;
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        Ok(())
    }
}

fn spawn_reader(stdout: tokio::process::ChildStdout, pending: PendingMap) {
    tokio::spawn(async move {
        let mut reader = FramedRead::new(stdout, LinesCodec::new());
        while let Some(line) = reader.next().await {
            let response = match line {
                Ok(line) => serde_json::from_str::<JsonRpcResponse>(&line)
                    .map_err(|error| McpError::InvalidResponse(error.to_string())),
                Err(error) => Err(McpError::Transport(error.to_string())),
            };
            match response {
                Ok(response) => {
                    let key = response_key(&response.id);
                    if let Some(sender) = pending.lock().await.remove(&key) {
                        let _ = sender.send(Ok(response));
                    }
                }
                Err(error) => {
                    notify_all(&pending, error).await;
                    break;
                }
            }
        }
        notify_all(
            &pending,
            McpError::Connection("stdio child stdout closed".into()),
        )
        .await;
    });
}

async fn notify_all(pending: &PendingMap, error: McpError) {
    let senders = pending
        .lock()
        .await
        .drain()
        .map(|(_, sender)| sender)
        .collect::<Vec<_>>();
    for sender in senders {
        let _ = sender.send(Err(error.clone()));
    }
}

fn env_pattern_matches(pattern: &str, key: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        key.starts_with(prefix)
    } else {
        pattern == key
    }
}
