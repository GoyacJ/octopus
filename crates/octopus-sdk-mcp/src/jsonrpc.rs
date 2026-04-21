use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    #[must_use]
    pub fn new(
        id: serde_json::Value,
        method: impl Into<String>,
        params: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        }
    }

    #[must_use]
    pub fn method(&self) -> &str {
        &self.method
    }

    #[must_use]
    pub fn params(&self) -> Option<&serde_json::Value> {
        self.params.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    #[must_use]
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    #[must_use]
    pub fn failure(id: serde_json::Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl JsonRpcNotification {
    #[must_use]
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

    #[test]
    fn jsonrpc_request_round_trips_with_params() {
        let request = JsonRpcRequest::new(
            json!(1),
            "tools/call",
            Some(json!({ "name": "grep", "arguments": { "pattern": "foo" } })),
        );

        let encoded = serde_json::to_value(&request).expect("request should serialize");
        let decoded: JsonRpcRequest =
            serde_json::from_value(encoded.clone()).expect("request should deserialize");

        assert_eq!(encoded["jsonrpc"], "2.0");
        assert_eq!(encoded["method"], "tools/call");
        assert_eq!(decoded.method(), "tools/call");
        assert_eq!(
            decoded.params(),
            Some(&json!({ "name": "grep", "arguments": { "pattern": "foo" } }))
        );
    }

    #[test]
    fn jsonrpc_response_round_trips_with_error() {
        let response = JsonRpcResponse::failure(
            json!("req-1"),
            JsonRpcError {
                code: -32_600,
                message: "invalid request".into(),
                data: Some(json!({ "field": "method" })),
            },
        );

        let encoded = serde_json::to_value(&response).expect("response should serialize");
        let decoded: JsonRpcResponse =
            serde_json::from_value(encoded.clone()).expect("response should deserialize");

        assert_eq!(encoded["error"]["code"], -32_600);
        assert_eq!(
            decoded.error.expect("error should exist").message,
            "invalid request"
        );
    }

    #[test]
    fn jsonrpc_notification_round_trips() {
        let notification = JsonRpcNotification::new(
            "notifications/tools/list_changed",
            Some(json!({ "server": "sdk" })),
        );

        let encoded = serde_json::to_value(&notification).expect("notification should serialize");
        let decoded: JsonRpcNotification =
            serde_json::from_value(encoded.clone()).expect("notification should deserialize");

        assert_eq!(encoded["jsonrpc"], "2.0");
        assert_eq!(decoded.method, "notifications/tools/list_changed");
        assert_eq!(decoded.params, Some(json!({ "server": "sdk" })));
    }
}
