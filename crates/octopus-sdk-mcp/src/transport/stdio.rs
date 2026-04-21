use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::{
    process::{Child, Command},
    sync::{oneshot, Mutex as AsyncMutex},
};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

use crate::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpError, McpTransport};

type PendingMap =
    Arc<AsyncMutex<HashMap<String, oneshot::Sender<Result<JsonRpcResponse, McpError>>>>>;

pub struct StdioTransport {
    writer: Arc<AsyncMutex<FramedWrite<tokio::process::ChildStdin, LinesCodec>>>,
    pending: PendingMap,
    _process: Arc<StdioProcessGuard>,
    timeout: Duration,
}

impl StdioTransport {
    pub fn spawn(
        command: impl AsRef<str>,
        args: impl IntoIterator<Item = impl Into<String>>,
        env: HashMap<String, String>,
    ) -> Result<Self, McpError> {
        let mut child = Command::new(command.as_ref());
        child
            .args(args.into_iter().map(Into::into))
            .envs(&env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit());

        let mut child = child
            .spawn()
            .map_err(|error: std::io::Error| McpError::Transport {
                message: error.to_string(),
            })?;
        let stdin = child.stdin.take().ok_or_else(|| McpError::Transport {
            message: "stdio transport missing stdin".into(),
        })?;
        let stdout = child.stdout.take().ok_or_else(|| McpError::Transport {
            message: "stdio transport missing stdout".into(),
        })?;

        let pending = Arc::new(AsyncMutex::new(HashMap::new()));
        spawn_reader(stdout, Arc::clone(&pending), child.id().unwrap_or_default());

        Ok(Self {
            writer: Arc::new(AsyncMutex::new(FramedWrite::new(stdin, LinesCodec::new()))),
            pending,
            _process: Arc::new(StdioProcessGuard::new(child)),
            timeout: Duration::from_secs(30),
        })
    }
}

#[async_trait]
impl McpTransport for StdioTransport {
    async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        let key = response_key(&req.id);
        let (sender, receiver) = oneshot::channel();
        self.pending.lock().await.insert(key.clone(), sender);

        let payload = serde_json::to_string(&req).map_err(|error| McpError::InvalidResponse {
            body_preview: error.to_string(),
        })?;

        if let Err(error) = self.writer.lock().await.send(payload).await {
            self.pending.lock().await.remove(&key);
            return Err(McpError::Transport {
                message: error.to_string(),
            });
        }

        match tokio::time::timeout(self.timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(McpError::ServerCrashed {
                server_id: "stdio".into(),
                exit_code: None,
            }),
            Err(_) => {
                self.pending.lock().await.remove(&key);
                Err(McpError::Timeout {
                    message: format!("stdio transport timed out waiting for {}", req.method),
                })
            }
        }
    }

    async fn notify(&self, msg: JsonRpcNotification) -> Result<(), McpError> {
        let payload = serde_json::to_string(&msg).map_err(|error| McpError::InvalidResponse {
            body_preview: error.to_string(),
        })?;
        self.writer
            .lock()
            .await
            .send(payload)
            .await
            .map_err(|error| McpError::Transport {
                message: error.to_string(),
            })
    }
}

fn spawn_reader(stdout: tokio::process::ChildStdout, pending: PendingMap, server_pid: u32) {
    tokio::spawn(async move {
        let mut reader = FramedRead::new(stdout, LinesCodec::new());
        while let Some(line) = reader.next().await {
            match line {
                Ok(line) => match serde_json::from_str::<JsonRpcResponse>(&line) {
                    Ok(response) => {
                        let key = response_key(&response.id);
                        if let Some(sender) = pending.lock().await.remove(&key) {
                            let _ = sender.send(Ok(response));
                        }
                    }
                    Err(error) => {
                        notify_all(
                            &pending,
                            McpError::InvalidResponse {
                                body_preview: error.to_string(),
                            },
                        )
                        .await;
                        break;
                    }
                },
                Err(error) => {
                    notify_all(
                        &pending,
                        McpError::Transport {
                            message: error.to_string(),
                        },
                    )
                    .await;
                    break;
                }
            }
        }

        notify_all(
            &pending,
            McpError::ServerCrashed {
                server_id: format!("stdio:{server_pid}"),
                exit_code: None,
            },
        )
        .await;
    });
}

async fn notify_all(pending: &PendingMap, error: McpError) {
    let mut locked = pending.lock().await;
    let senders = locked.drain().map(|(_, sender)| sender).collect::<Vec<_>>();
    drop(locked);

    for sender in senders {
        let _ = sender.send(Err(error.clone()));
    }
}

fn response_key(id: &serde_json::Value) -> String {
    serde_json::to_string(id).expect("jsonrpc ids should serialize")
}

struct StdioProcessGuard {
    child: Mutex<Option<Child>>,
}

impl StdioProcessGuard {
    fn new(child: Child) -> Self {
        Self {
            child: Mutex::new(Some(child)),
        }
    }
}

impl Drop for StdioProcessGuard {
    fn drop(&mut self) {
        let Some(mut child) = self.child.lock().expect("child lock should work").take() else {
            return;
        };

        let join = std::thread::Builder::new()
            .name("octopus-sdk-mcp-stdio-drop".into())
            .spawn(move || {
                let _ = child.start_kill();
                if let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    let _ = runtime.block_on(async { child.wait().await });
                }
            });

        if let Ok(handle) = join {
            let _ = handle.join();
        }
    }
}
