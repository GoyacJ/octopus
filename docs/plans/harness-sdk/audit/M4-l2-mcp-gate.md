# M4 L2 MCP Gate Audit

审计日期：2026-04-27  
范围：`octopus-harness-mcp`，对应 M4-T11 到 M4-T18。

## 结论

L2-MCP 的 M4 任务卡已收口，可以进入评审。

当前分支：`goya/l2-mcp-t11`。  
当前收口提交：`22ed240f feat(harness-mcp): add mcp contract and runtime governance`。

## 任务卡对照

| 任务卡 | 提交 | 结果 |
|---|---|---|
| M4-T11 Client core | `0d6f0e45` | `McpClient` / `McpTransport` / `McpConnection` / JSON-RPC / registry wrapper 已实现。 |
| M4-T12 stdio/http/websocket | `10b7e345` | 三种 transport 与对应测试已实现。 |
| M4-T13 sse/in-process | `a5d437da` | SSE 与 in-process transport 已实现。 |
| M4-T14 reconnect | `798aadb1` | 重连治理、退避、事件与 registry 接入已实现。 |
| M4-T15 OAuth + Elicitation | `fd988224` | PKCE、device flow、elicitation handler 与事件路径已实现。 |
| M4-T16 ServerAdapter | `bc1b5138` | 最小 JSON-RPC Server Adapter 已实现，能力源为 `ToolRegistry`。 |
| M4-T17 Sampling | `0c1946f5` | Sampling policy、budget、rate、cache namespace 与 permission mode 联动已实现。 |
| M4-T18 Contract / tenant / list_changed | `22ed240f` | 5 transport contract、多租户隔离、`tools/list_changed` 运行期策略已实现。 |

## Gate 证据

| 检查 | 结果 |
|---|---:|
| `cargo fmt --all -- --check` | PASS |
| `cargo check -p octopus-harness-mcp --all-features` | PASS |
| `cargo clippy -p octopus-harness-mcp --all-targets --all-features -- -D warnings` | PASS |
| `cargo test -p octopus-harness-mcp --all-features` | PASS |
| `cargo check -p octopus-harness-mcp` | PASS |
| `cargo check -p octopus-harness-mcp --features stdio,http,websocket` | PASS |
| `cargo check -p octopus-harness-mcp --features sse,in-process,server-adapter,oauth` | PASS |
| `cargo tree -p octopus-harness-mcp --all-features --depth 1` | PASS |
| `! rg -n 'octopus-sdk\|octopus_sdk\|octopus-sdk-mcp' crates/octopus-harness-mcp` | PASS |
| `git diff --check` | PASS |

当前 MCP integration/doc tests 数量：56 个测试，0 个 doctest。

## 边界

- 生产依赖保持在 `octopus-harness-contracts`、`octopus-harness-tool` 和基础协议库范围内。
- 未依赖旧 `octopus-sdk-mcp`。
- 未引入 `harness-session`、`harness-engine`、`harness-sdk`。
- 默认 feature 编译告警已修复：`pub use transports::*` 只在 transport feature 开启时导出。

## 后续范围

以下不属于 M4-T11 到 M4-T18 收口：

- 真实 `serve_http` / `serve_stdio` / `serve_websocket` 监听层。
- Server Adapter 的 9+1 Harness 业务工具。
- 完整 `resources/read`、`prompts/get`、resource subscription、cancel/progress 协议面。
- MCP observability metrics。
- Session reload、deferred projection、L4 SDK 注入集成。
