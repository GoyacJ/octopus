use async_trait::async_trait;

use crate::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpError};

mod http;
mod sdk;
mod stdio;

pub use http::*;
pub use sdk::*;
pub use stdio::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportKind {
    Stdio,
    Http,
    Sdk,
}

#[async_trait]
pub trait McpTransport: Send + Sync {
    async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError>;

    async fn notify(&self, msg: JsonRpcNotification) -> Result<(), McpError> {
        let _ = self
            .call(JsonRpcRequest::new(
                serde_json::Value::Null,
                msg.method,
                msg.params,
            ))
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;

    use super::{McpTransport, TransportKind};
    use crate::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpError};

    struct DummyTransport;

    #[async_trait]
    impl McpTransport for DummyTransport {
        async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
            Ok(JsonRpcResponse::success(req.id, json!({ "ok": true })))
        }
    }

    #[test]
    fn transport_trait_is_object_safe() {
        let transport: Arc<dyn McpTransport> = Arc::new(DummyTransport);
        let _ = transport;
        assert_eq!(TransportKind::Sdk, TransportKind::Sdk);
    }

    #[tokio::test]
    async fn notify_defaults_to_call() {
        let transport = DummyTransport;

        transport
            .notify(JsonRpcNotification::new(
                "notifications/test",
                Some(json!({ "ok": true })),
            ))
            .await
            .expect("notify should succeed");
    }
}
