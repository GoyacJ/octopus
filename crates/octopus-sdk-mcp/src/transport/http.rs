use std::{collections::HashMap, time::Duration};

use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};

use crate::{JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, McpError, McpTransport};

pub struct HttpTransport {
    client: Client,
    base_url: String,
}

impl HttpTransport {
    pub fn new(
        base_url: impl Into<String>,
        headers: HashMap<String, String>,
    ) -> Result<Self, McpError> {
        let mut default_headers = HeaderMap::new();
        for (key, value) in headers {
            let name = HeaderName::try_from(key.as_str()).map_err(|error| McpError::Transport {
                message: error.to_string(),
            })?;
            let value =
                HeaderValue::try_from(value.as_str()).map_err(|error| McpError::Transport {
                    message: error.to_string(),
                })?;
            default_headers.insert(name, value);
        }

        let client = Client::builder()
            .default_headers(default_headers)
            .pool_max_idle_per_host(0)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(McpError::from)?;

        Ok(Self {
            client,
            base_url: base_url.into(),
        })
    }

    fn endpoint(&self) -> String {
        format!("{}/mcp", self.base_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl McpTransport for HttpTransport {
    async fn call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, McpError> {
        self.client
            .post(self.endpoint())
            .json(&req)
            .send()
            .await
            .map_err(McpError::from)?
            .json::<JsonRpcResponse>()
            .await
            .map_err(McpError::from)
    }

    async fn notify(&self, msg: JsonRpcNotification) -> Result<(), McpError> {
        self.client
            .post(self.endpoint())
            .json(&msg)
            .send()
            .await
            .map_err(McpError::from)?;
        Ok(())
    }
}
