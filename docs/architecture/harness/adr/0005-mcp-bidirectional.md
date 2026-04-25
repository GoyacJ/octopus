# ADR-005 · MCP 双向支持（Client 入站 + Server Adapter 出站）

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：`harness-mcp` / `harness-tool` / 业务层（特别是 `octopus-server`）

## 1. 背景与问题

Model Context Protocol（MCP）的集成有两个方向：

- **入站（Client）**：消费外部 MCP Server 提供的 tools / resources / prompts，把它们注入 Agent 的 ToolRegistry
- **出站（Server Adapter）**：把自身 Agent 能力暴露给外部 MCP 客户端（如 Claude Code / Cursor / Codex 等），让它们像调用本地工具一样调用我们的 Agent

参考项目现状：

| 项目 | Client | Server |
|---|---|---|
| Hermes | ✅（HER-042 `tools/mcp_tool.py`） | ✅（HER-042 `mcp_serve.py` 暴露 9 工具） |
| Claude Code | ✅（CC-19 七 Transport） | ❌（客户端形态） |
| OpenClaw | 未作专章 | 部分（通过 Gateway 协议暴露） |

Hermes 的 `mcp_serve.py` 对齐了 OpenClaw 的 9 个 Session-Tool，验证了"双向"的价值：一个 Harness 既是 MCP 消费者，也是 MCP 生产者。

## 2. 决策

**`harness-mcp` 同时提供 Client 与 Server Adapter**。

### 2.1 入站 Client

```rust
pub struct McpClient { /* ... */ }

#[async_trait]
pub trait McpTransport: Send + Sync + 'static {
    fn transport_id(&self) -> &str;
    async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>>;
}
```

内置 Transport：`stdio / http / websocket / sse / in-process`。

### 2.2 出站 Server Adapter

```rust
pub struct HarnessMcpServer {
    harness: Arc<Harness>,
    policy: McpServerPolicy,
}

impl HarnessMcpServer {
    pub async fn serve_stdio(self) -> Result<()>;
    pub async fn serve_http(self, addr: SocketAddr) -> Result<()>;
}

pub struct ExposedCapabilities {
    pub sessions_list: bool,
    pub session_get: bool,
    pub messages_read: bool,
    pub messages_send: bool,
    pub attachments_fetch: bool,
    pub events_poll: bool,
    pub events_wait: bool,
    pub permissions_list_open: bool,
    pub permissions_respond: bool,
    pub channels_list: bool,
}
```

对齐 Hermes `mcp_serve.py` 的 9+ 工具能力。

### 2.3 Agent-scoped 注入（对齐 CC-20）

子 Agent 声明 `mcpServers` 时：

- **`McpServerRef::Shared(id)`**：复用父连接
- **`McpServerRef::Inline(spec)`**：独立连接，Subagent 结束时 RAII 关闭；受 §6.5 trust 限制
- **`McpServerRef::Required(id)`**：复用但 connection 必须 Ready；用于强依赖某 server 的 agent

`SubagentSpec.required_mcp_servers` 声明 pattern 级依赖；装配期校验失败 fail-closed（详见 `crates/harness-mcp.md §5.3`）。

### 2.4 来源 / 信任 / 生命周期三个独立维度

`McpServerSpec` 使用三个**正交**字段表达 server 的属性：

| 字段 | 类型 | 含义 | 推导关系 |
|---|---|---|---|
| `source` | `McpServerSource` | 来源类别（Workspace/User/Project/Policy/Plugin/Dynamic/Managed） | 输入 |
| `trust` | `TrustLevel` | 信任级别 | 由 `source` 推导（见 `harness-mcp.md §2.2.4`） |
| `scope` | `McpServerScope` | 生命周期范围（Global/Session/Agent） | 由调用方按使用场景指定 |

`McpServerScope` 名字保持不变以兼容历史引用；语义即"生命周期范围"，**与"来源"和"信任"完全解耦**——避免 1.4 之前的"scope 字段同时承担三种语义"导致的歧义。

## 3. 替代方案

### 3.1 A：只提供 Client，Server 由业务层实现

- ❌ 每个业务层重复实现协议（HTTP + WS + stdio 解析）
- ❌ 共享能力矩阵难以保持一致

### 3.2 B：Client + Server 但分成两个 crate

- ❌ 协议版本同步成本（两个 crate 必须同时升级）
- ❌ 共享类型重复定义
- ❌ Feature flag 更复杂

### 3.3 C：同一 crate 双向（采纳）

- ✅ 协议版本单源
- ✅ 共享类型无缝
- ✅ Feature gate 控制：`mcp-client` / `mcp-server-adapter` 可独立开启

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| Crate 面积增大 | `harness-mcp` 包含两套逻辑 | Feature gate 隔离；业务层按需开启 |
| 授权模型复杂 | Client 与 Server 的 auth 模型不同 | 独立 `McpClientAuth` vs `McpServerAuth` 类型 |
| 生命周期管理 | Server 的 accept loop + Client 的 connection 协程 | 独立 `tokio::spawn`，统一 graceful shutdown |

## 5. 后果

### 5.1 正面

- `octopus-server` 可直接 `HarnessMcpServer::serve_http(...).await` 把 SDK 能力透明暴露
- Desktop App 可同时 consume 外部 MCP 且 expose 自身 MCP
- 测试时可用 `InProcessTransport` 端到端验证

### 5.2 负面

- `harness-mcp` crate 行数增多（协议细节多）
- Feature flag 组合增多

## 6. 实现指引

### 6.1 协议核心

- JSON-RPC 2.0 over various transports
- 支持 `tools/list` / `tools/call` / `tools/list_changed`
- 支持 `resources/*` / `prompts/*`
- 支持 `elicitation/*`（CC-21，对应 SEP 预留 -32042 错误码）
- 支持 OAuth（CC-21 XAA 跨应用访问）

### 6.2 Server Adapter 暴露的工具

对齐 HER-042：

```text
sessions_list        / 列出所有 Session
session_get          / 读取 Session 元数据
messages_read        / 读取消息（分页）
messages_send        / 发送消息（外部客户端触发 Agent）
attachments_fetch    / 拉取附件 blob
events_poll          / 拉取最新事件
events_wait          / 长轮询等待新事件
permissions_list_open / 列出待审批项
permissions_respond  / 回应审批
channels_list        / 列出可用 Channel
```

### 6.3 工具预过滤

注入 `ToolRegistry` 前由 `McpToolFilter`（`McpServerSpec.tool_filter`）做 allow / deny 过滤，与 `harness-permission` 的 `DenyRule.scope` 共用 canonical `mcp__<server>__<tool>` glob 语法。Filter 控制"模型是否可见 / 检索到"，Permission 控制"运行期是否可调用"，二者协同而非替代。详见 `crates/harness-mcp.md §2.6`。

### 6.4 Sampling 反向调用与 Cache 隔离

MCP Server 反向调用 `sampling/createMessage` 的所有路径必须经过 `SamplingPolicy`（`McpServerSpec.sampling`）治理：

- **默认 fail-closed**：`SamplingPolicy::denied()` 拒绝任何反向调用
- **七维 budget**：`per_request` / `aggregate` / `rate_limit` / `allowed_models` / `log_level` 等控制 token、轮数、超时、模型白名单与日志级别
- **Cache 硬隔离**：`SamplingCachePolicy::IsolatedNamespace { ttl }` 是默认；sampling 调用走独立的 prompt cache 命名空间，**不污染**主 Session cache key（与 ADR-003 §4 一致）
- **PermissionMode 联动**：`BypassPermissions` / `DontAsk` 强制把 `AllowWithApproval` 降级为 `Denied`（不能用绕过审批模式偷叫 LLM）；`Plan` 把 `AllowAuto` 强制为 `AllowWithApproval`
- **来源联动**：`UserControlled` server 不允许 `AllowAuto`，装配期 fail-closed

详细字段、流程、事件定义见 `crates/harness-mcp.md §6.5` 与 `event-schema.md §3.19.7`。

### 6.5 Inline MCP 受 trust 限制

`McpServerRef::Inline(spec)` 只在以下情况允许装配：

1. `spec.source ∈ {Workspace, Policy, Plugin{trust=Admin}, Managed}` —— 管控来源
2. 父 Subagent 自身已是 `AdminTrusted`
3. `PermissionMode = BypassPermissions` ∧ `HarnessBuilder.with_inline_user_mcp(true)` 显式开关 —— 必须落审计

否则 fail-closed 拒绝（`SubagentError::InlineMcpTrustViolation`）。该约束的目的是**禁止 user-controlled agent 自行 inline 引入用户级 MCP server**，与 ADR-006 "用户控制层不能升权"原则一致。

详细路径见 `crates/harness-mcp.md §5.2` 与 `crates/harness-subagent.md §3`。

### 6.6 Tool 命名冲突

外部 MCP Server 的工具名一律走 canonical 形态 `mcp__<server_id>__<tool_name>`，
避免与内建重名，同时兼容 OpenAI / Anthropic Function Calling 的字符集
（`^[a-zA-Z0-9_-]{1,64}$`）；命名 utility 与字符集校验定义于
`harness-contracts §3.4.2`，所有 SDK 内部 / 文档 / 规则文件均以双下划线分隔。

> **修订记录（2026-04-25）**：本节早期版本使用 `mcp:<server_id>:<tool_name>`
> （冒号分隔），与 LLM 工具命名字符集冲突，已废弃。`harness-mcp` 在
> `inject_tools_into` 时通过 `canonical_mcp_tool_name(server, tool)` 统一生成；
> 历史 Event / Decision Persistence 中的旧形态由业务层一次性迁移脚本改写，
> SDK 不做运行期兼容映射（避免影子命名空间）。

## 7. 相关

- `crates/harness-mcp.md`（§2.2 / §2.6 / §3.4 / §5.2 / §5.3 / §6.5）
- `crates/harness-subagent.md` §3（`required_mcp_servers` 校验）
- `event-schema.md` §3.19（MCP Events）
- ADR-003（Prompt Cache 锁定，与 §6.4 sampling cache 隔离一致）
- ADR-006（Plugin Trust Levels，与 §6.5 Inline trust 限制联动）
- ADR-007（Permission Events，与 §6.5 PermissionMode 联动）
- D7 · `extensibility.md` §6 MCP 扩展
- Evidence: HER-042, CC-19, CC-20, CC-21
