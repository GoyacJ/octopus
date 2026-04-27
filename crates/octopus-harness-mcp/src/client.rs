use std::sync::Arc;

use crate::{McpConnection, McpError, McpServerSpec, McpTransport};

#[derive(Clone)]
pub struct McpClient {
    transport: Arc<dyn McpTransport>,
}

impl McpClient {
    pub fn new(transport: Arc<dyn McpTransport>) -> Self {
        Self { transport }
    }

    pub fn transport_id(&self) -> &str {
        self.transport.transport_id()
    }

    pub async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>, McpError> {
        self.transport.connect(spec).await
    }
}
