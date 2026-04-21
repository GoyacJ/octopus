use octopus_sdk_contracts::ContentBlock;
use octopus_sdk_mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();
    let mut writer = tokio::io::BufWriter::new(stdout);

    while let Ok(Some(line)) = reader.next_line().await {
        let request: JsonRpcRequest =
            serde_json::from_str(&line).expect("request should deserialize");

        let response = match request.method.as_str() {
            "initialize" => JsonRpcResponse::success(
                request.id,
                json!({
                    "protocolVersion": "2025-03-26"
                }),
            ),
            "tools/list" => JsonRpcResponse::success(
                request.id,
                json!({
                    "tools": [{
                        "name": "echo",
                        "description": "Echo input",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "message": { "type": "string" }
                            }
                        }
                    }]
                }),
            ),
            "tools/call" => {
                let message = request
                    .params
                    .as_ref()
                    .and_then(|params| params.get("arguments"))
                    .and_then(|arguments| arguments.get("message"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();

                JsonRpcResponse::success(
                    request.id,
                    serde_json::to_value(json!({
                        "content": [ContentBlock::Text { text: message.to_string() }],
                        "isError": false
                    }))
                    .expect("tool result should serialize"),
                )
            }
            other => JsonRpcResponse::failure(
                request.id,
                JsonRpcError {
                    code: -32_601,
                    message: format!("method not found: {other}"),
                    data: None,
                },
            ),
        };

        let encoded = serde_json::to_string(&response).expect("response should serialize");
        writer
            .write_all(encoded.as_bytes())
            .await
            .expect("response write should succeed");
        writer
            .write_all(b"\n")
            .await
            .expect("newline should succeed");
        writer.flush().await.expect("flush should succeed");
    }
}
