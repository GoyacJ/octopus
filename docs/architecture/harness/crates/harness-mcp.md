# `octopus-harness-mcp` · L2 复合能力 · MCP Client + Server Adapter SPEC

> 层级：L2 · 状态：Accepted
> 依赖：`harness-contracts` + `harness-tool`（注册）

## 1. 职责

提供 **Model Context Protocol 双向支持**：入站 Client（消费外部 MCP Server）+ 出站 Server Adapter（把 Harness 暴露为 MCP Server）。对齐 ADR-005。

**核心能力**：

- 多 Transport：stdio / http / websocket / sse / in-process（自定义 transport 走 §6 扩展点）
- OAuth + XAA（跨应用访问，CC-21）
- 每 server 一个后台 tokio task 维护连接，带 `ReconnectPolicy` 指数退避（§2.2.1）
- stdio 凭证沙化：默认 `InheritWithDeny + default_deny_envs()`（§2.2.2 / §2.2.3）
- 动态 `tools/list_changed` / `resources/updated` 刷新（§6）
- Agent-scoped 注入（Shared / Inline / Required）+ Inline 受 trust 限制（§5）
- Elicitation 处理（`-32042` 错误码 + 与 PermissionMode 联动，§2.5）
- 工具预过滤 `McpToolFilter`（注入前 allow/deny；§2.6）
- Sampling 反向调用治理（七维 budget + Cache 隔离；§6.5）
- Server Adapter 暴露 9+1 工具（对齐 HER-042），多租户隔离 + 速率限制（§3.4）

## 2. Client 端 API

### 2.1 核心 Trait

```rust
#[async_trait]
pub trait McpTransport: Send + Sync + 'static {
    fn transport_id(&self) -> &str;

    async fn connect(
        &self,
        spec: McpServerSpec,
    ) -> Result<Arc<dyn McpConnection>, McpError>;
}

#[async_trait]
pub trait McpConnection: Send + Sync + 'static {
    async fn list_tools(&self) -> Result<Vec<McpToolDescriptor>, McpError>;
    async fn call_tool(&self, name: &str, args: Value) -> Result<McpToolResult, McpError>;
    async fn list_resources(&self) -> Result<Vec<McpResource>, McpError>;
    async fn read_resource(&self, uri: &str) -> Result<McpResourceContents, McpError>;
    async fn list_prompts(&self) -> Result<Vec<McpPrompt>, McpError>;
    async fn get_prompt(&self, name: &str, args: Value) -> Result<McpPromptMessages, McpError>;
    async fn subscribe_list_changed(&self) -> Result<BoxStream<ListChangedEvent>, McpError>;
    async fn shutdown(&self) -> Result<(), McpError>;
}
```

### 2.2 核心类型

```rust
pub struct McpServerSpec {
    pub server_id: McpServerId,
    pub display_name: String,
    pub transport: TransportChoice,
    pub auth: McpClientAuth,
    pub capabilities_expected: McpExpectedCapabilities,

    /// 来源类别决定 trust 推导（见 §2.2.4 与 ADR-005 §6.5）。
    pub source: McpServerSource,
    /// 由 `source` 推导的信任级别；业务层不能直接覆盖（fail-closed）。
    /// `Inline` Subagent 引用 + `UserControlled` server 的组合受 §5.2 / `harness-subagent.md §3` 限制。
    pub trust: TrustLevel,

    /// 客户端拉取 / 调用 / 反向 sampling 的超时控制。
    pub timeouts: McpTimeouts,
    /// 网络断连重连策略；默认 `ReconnectPolicy::default()`（指数退避 + 上限）。
    pub reconnect: ReconnectPolicy,

    /// 注入 ToolRegistry 前的工具过滤（allow / deny），与 `harness-permission` DenyRule 共用 glob 语法（见 §2.6）。
    pub tool_filter: McpToolFilter,
    /// MCP Server 反向调用本端 LLM（`sampling/createMessage`）时的限额与隔离策略；见 §6.5。
    /// 默认 `SamplingPolicy::denied()` —— 任何反向调用都拒绝。
    pub sampling: SamplingPolicy,
}

pub enum TransportChoice {
    Stdio {
        command: String,
        args: Vec<String>,
        /// stdio 进程的环境变量沙化策略（见 §2.2.2）。
        env: StdioEnv,
        /// 错误日志脱敏与 stdio 子进程治理。
        policy: StdioPolicy,
    },
    Http { url: Url, headers: HashMap<String, String> },
    WebSocket { url: Url, headers: HashMap<String, String> },
    Sse { url: Url, headers: HashMap<String, String> },
    InProcess { factory: Arc<dyn InProcessFactory> },
}

pub enum McpClientAuth {
    None,
    Bearer(SecretString),
    OAuth {
        authorize_url: Url,
        token_url: Url,
        client_id: String,
        client_secret: SecretString,
        scopes: Vec<String>,
    },
    Xaa {
        parent_session: SessionId,
        scopes: Vec<String>,
    },
    Custom(Arc<dyn McpAuthProvider>),
}

pub struct McpServerId(pub String);
```

#### 2.2.1 `McpTimeouts` / `ReconnectPolicy`

```rust
pub struct McpTimeouts {
    /// `initialize` / `tools/list` / `resources/list` 等元数据握手超时。
    pub handshake: Duration,
    /// 单个 `tools/call` / `resources/read` 请求的默认超时（工具可在 `_meta` 覆盖）。
    pub call_default: Duration,
    /// `sampling/createMessage` 反向调用的超时（与 `SamplingPolicy.timeout` 取小者）。
    pub sampling: Duration,
    /// `subscribe_*` 流空闲超时（无心跳即重连）。
    pub idle: Duration,
}

impl Default for McpTimeouts {
    /// 5s / 30s / 60s / 300s（保守上限，业务层按需收紧）。
    fn default() -> Self { /* ... */ }
}

pub struct ReconnectPolicy {
    pub max_attempts: u32,           // 0 = 不限
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_jitter: f32,         // [0.0, 1.0]，给退避加抖动
    /// 连接成功保持 `success_reset_after` 之后，重连计数器清零（避免雪崩后无限退避）。
    pub success_reset_after: Duration,
    /// 在重连过程中是否保留已注入的 deferred 工具描述（`true` = 保留，UI/Schema 仍可见但调用会以 `McpError::Connection` 失败）。
    pub keep_deferred_during_reconnect: bool,
}

impl Default for ReconnectPolicy {
    /// 0(不限) / 500ms / 30s / 0.2 / 5min / true。
    fn default() -> Self { /* ... */ }
}
```

每次重连成功发 `Event::McpConnectionRecovered`（详见 §2.7、`event-schema.md §3.19`）。

#### 2.2.2 `StdioEnv` / `StdioPolicy`

stdio Transport 启动子进程时直接 fork 父进程环境会泄露 `OPENAI_API_KEY` / `AWS_*` / `KUBE_*` 等敏感凭证给第三方 MCP Server。本节强制声明继承策略：

```rust
pub enum StdioEnv {
    /// 显式白名单：只继承指定的环境变量，并合并 `extra`（默认 `extra` 为空）。
    Allowlist {
        inherit: BTreeSet<String>,
        extra: BTreeMap<String, SecretString>,
    },
    /// 父进程环境减去 `deny`，并合并 `extra`。`deny` 默认包含 §2.2.3 的全部条目。
    InheritWithDeny {
        deny: BTreeSet<String>,
        extra: BTreeMap<String, SecretString>,
    },
    /// 完全隔离：仅使用 `extra`。
    Empty {
        extra: BTreeMap<String, SecretString>,
    },
}

impl StdioEnv {
    /// SDK 默认 = `InheritWithDeny { deny: default_deny_envs(), extra: {} }`，
    /// fail-closed 屏蔽常见凭证类变量。
    pub fn default() -> Self;
}

pub struct StdioPolicy {
    /// stderr 行最大长度（超长截断），避免恶意 server 把长 ANSI / 凭证 echo 灌进日志。
    pub stderr_line_max_bytes: u32,
    /// 把 stderr 行写入 Journal 之前是否走 `harness-permission::Redactor`（默认 `true`）。
    pub redact_stderr: bool,
    /// 子进程退出后等待资源回收的最长时间；超过则强制 SIGKILL。
    pub graceful_kill_after: Duration,
    /// 子进程 cwd（默认继承父进程 cwd；测试 / 多租户场景应显式声明）。
    pub working_dir: Option<PathBuf>,
}

impl Default for StdioPolicy {
    /// 4096 / true / 5s / None。
    fn default() -> Self { /* ... */ }
}
```

#### 2.2.3 默认 deny envs

`StdioEnv::default_deny_envs()` 至少包含：

```text
OPENAI_API_KEY / OPENAI_ORG / ANTHROPIC_API_KEY / GOOGLE_API_KEY
AWS_ACCESS_KEY_ID / AWS_SECRET_ACCESS_KEY / AWS_SESSION_TOKEN
AZURE_OPENAI_KEY / AZURE_CLIENT_SECRET
GOOGLE_APPLICATION_CREDENTIALS
KUBECONFIG / KUBE_TOKEN
GITHUB_TOKEN / GITLAB_TOKEN
DOCKER_AUTH_CONFIG
NPM_TOKEN / CARGO_REGISTRY_TOKEN
HARNESS_*  (SDK 自身命名空间)
```

完整列表与版本策略由 `harness-mcp/data/stdio-deny-envs.toml` 维护，遵循 ADR-006 同款的"开闸只能放宽，不能默认 inherit 全部"。

#### 2.2.4 `McpServerSource → TrustLevel` 推导

`McpServerSource` 定义见 `harness-contracts.md §3.4`。SDK 在 `McpRegistry::add_server` 时按下表推导 `trust`，业务层显式传入 `trust` 与推导值不一致时 fail-closed 拒绝注册：

| `McpServerSource` | 推导 `TrustLevel` | 备注 |
|---|---|---|
| `Workspace` / `Policy` / `Managed{..}` | `AdminTrusted` | 部署期/企业策略下发，不可被 user-config 覆盖 |
| `Plugin(_)` | 跟随插件本身的 trust（见 `harness-plugin.md §2`） | 复用 ADR-006 的二分模型 |
| `User` / `Project` | `UserControlled` | 仅供 admin 自身 / 当前项目使用 |
| `Dynamic { .. }` | `UserControlled` | 运行期注册，不允许进入 admin agent / managed 流程 |

### 2.3 Registry

```rust
pub struct McpRegistry {
    inner: Arc<RwLock<McpRegistryInner>>,
}

struct McpRegistryInner {
    servers: HashMap<McpServerId, ManagedMcpServer>,
    shared_connections: HashMap<McpServerId, Arc<dyn McpConnection>>,
}

pub struct ManagedMcpServer {
    pub spec: McpServerSpec,
    pub connection: Arc<dyn McpConnection>,
    pub injected_tools: Vec<ToolDescriptor>,
    /// 生命周期范围：决定连接何时被 `harness-mcp` 强制 shutdown。
    /// 命名沿用 `McpServerScope`（兼容历史引用），语义即"生命周期范围"。
    pub scope: McpServerScope,
    pub last_list_changed: Option<DateTime<Utc>>,
    /// 当前连接状态（见 §2.7）。
    pub connection_state: McpConnectionState,
    /// 自上次 `tools/list` 计算的 schema 指纹，用于幂等检测、重连后比对。
    pub schema_fingerprint: ContentHash,
}

/// MCP Server 的**生命周期范围**（≠ `McpServerSource`）。
///
/// - `McpServerSource` 表达「来源类别」 → 决定 `trust`
/// - `McpServerScope` 表达「生命周期」 → 决定连接何时回收
pub enum McpServerScope {
    Global,                  // 父 Harness 生命周期
    Session(SessionId),
    Agent(AgentId),          // Agent-scoped（CC-20）
}

#[non_exhaustive]
pub enum McpConnectionState {
    Connecting,
    Ready,
    Reconnecting { attempt: u32, last_error: String },
    Failed { reason: String },
    ShuttingDown,
    Closed,
}
```

### 2.4 注入到 ToolRegistry

```rust
use harness_contracts::{canonical_mcp_tool_name, ToolNameError};

impl McpRegistry {
    pub async fn inject_tools_into(
        &self,
        tool_registry: &ToolRegistry,
        server_id: &McpServerId,
    ) -> Result<Vec<String>, McpError> {
        let connection = self.get_connection(server_id)?;
        let mcp_tools = connection.list_tools().await?;
        let mut registered = Vec::new();
        for mcp_tool in mcp_tools {
            let defer_policy = Self::resolve_defer_policy(&mcp_tool);
            let upstream_name = mcp_tool.name.clone();
            let canonical = match canonical_mcp_tool_name(server_id.as_ref(), &upstream_name) {
                Ok(n) => n,
                Err(ToolNameError::ReservedSeparator(_)) => {
                    Self::collapse_reserved_separator(server_id, &upstream_name)?
                }
                Err(e) => return Err(McpError::ToolNamingViolation(e.to_string())),
            };
            let tool = McpToolWrapper::new(
                server_id.clone(),
                mcp_tool,
                connection.clone(),
                defer_policy,
                canonical.clone(),
            );
            tool_registry.register(Box::new(tool))?;
            registered.push(canonical);
        }
        Ok(registered)
    }

    /// MCP 工具的 DeferPolicy 规则（ADR-009 §2.2）：
    /// - `_meta["anthropic/alwaysLoad"] = true` → `AlwaysLoad`（反向覆盖）
    /// - 否则 → `AutoDefer`（MCP 工具的默认）
    fn resolve_defer_policy(mcp_tool: &McpToolDescriptor) -> DeferPolicy {
        match mcp_tool.meta.get("anthropic/alwaysLoad") {
            Some(Value::Bool(true)) => DeferPolicy::AlwaysLoad,
            _                       => DeferPolicy::AutoDefer,
        }
    }

    /// 上游工具名包含 `__` 时的折叠策略：
    /// 1. 默认按单下划线折叠（如 `bulk__import` → `bulk_import`），保证 canonical 仍可解析；
    /// 2. 折叠后再次 `validate_tool_name`，若仍冲突则发 `Event::McpToolNameRejected` 拒绝注册；
    /// 3. 业务层若希望保留原始名以便审计，应在 `McpToolWrapper.upstream_name` 字段读取。
    fn collapse_reserved_separator(
        server_id: &McpServerId,
        upstream: &str,
    ) -> Result<String, McpError>;
}
```

**设计说明**：

- MCP 工具天然"工作流特定、数量不可控"，因此默认 `AutoDefer` 让它们进入 Deferred 集，由 `ToolSearchTool` 按需 materialize；管控方可通过 MCP Server 声明的 `_meta["anthropic/alwaysLoad"]` 标记把关键工具钉为 `AlwaysLoad`（详见 ADR-009 §6.3）。
- **命名 canonical 形态**：所有注入后的工具名形如 `mcp__<server_id>__<tool>`，与 `harness-contracts §3.4.2` 一致；规则配置（`Rule.scope` / glob）、Permission 审计、Event Journal、Display 全部使用同一形态。
- **Event 一致性**：`McpToolWrapper` 在 `Event::ToolUseRequested` / `PermissionRequested` 中以 canonical name 出现，`upstream_name` 仅作 metadata 落 `ToolDescriptor::extras["mcp_upstream_name"]`，便于 server-side 调试但不参与匹配。

### 2.5 Elicitation（CC-21）

```rust
pub struct ElicitationRequest {
    pub request_id: RequestId,
    pub server_id: McpServerId,
    pub schema: JsonSchema,
    pub subject: String,
    pub detail: Option<String>,
    pub timeout: Option<Duration>,
}

#[async_trait]
pub trait ElicitationHandler: Send + Sync + 'static {
    fn handler_id(&self) -> &str;
    async fn handle(&self, request: ElicitationRequest)
        -> Result<Value, ElicitationError>;
}

pub enum ElicitationError {
    UserDeclined,
    Timeout,
    Invalid(String),
    NoHandlerRegistered,
}
```

`-32042` 错误码：MCP Server 要求客户端填写额外信息。

#### 2.5.1 内置实现

| 实现 | 语义 | 适用场景 |
|---|---|---|
| `StreamElicitationHandler`（**默认**） | 把 `ElicitationRequest` 以 `Event::McpElicitationRequested` 发到 Session EventStream；业务层 `harness.resolve_elicitation(request_id, value).await` 回传 | Desktop / Web UI 异步响应 |
| `DirectElicitationHandler<F>` | 构造时传闭包 `F: Fn(ElicitationRequest) -> BoxFuture<Value>`，SDK 直接 await | CLI / 测试 |
| `RejectAllElicitationHandler` | 一律返回 `ElicitationError::UserDeclined` | 批处理 / CI；禁止交互模式的 fail-safe 默认 |

#### 2.5.2 默认解析顺序（`HarnessBuilder`）

```rust
// 未显式 with_elicitation_handler
// 若启用 mcp client feature → 默认注入 StreamElicitationHandler
// 否则 → RejectAllElicitationHandler（拒绝所有 -32042）
```

#### 2.5.3 Resolve API

```rust
impl Harness {
    pub async fn resolve_elicitation(
        &self,
        request_id: RequestId,
        value: Value,
    ) -> Result<(), ElicitationError>;

    pub async fn reject_elicitation(
        &self,
        request_id: RequestId,
        reason: String,
    ) -> Result<(), ElicitationError>;
}
```

#### 2.5.4 Event 轨迹

```text
[MCP Server 返回 -32042]
    │
    ▼
Event::McpElicitationRequested { request_id, server_id, schema, subject, ... }
    │
    ▼  (业务层 UI 渲染、用户填写)
harness.resolve_elicitation(request_id, value).await
    │
    ▼
Event::McpElicitationResolved { request_id, outcome, at }
    │
    ▼
[MCP 请求继续]
```

`Event::McpElicitationResolved` 已在 `harness-contracts::Event` 中存在（详见 `event-schema.md §3.19`）。

#### 2.5.5 与 PermissionMode 联动

| `PermissionMode` | 默认 ElicitationHandler 选择 |
|---|---|
| `Default` / `Auto` | `StreamElicitationHandler` |
| `Plan` | `StreamElicitationHandler`（仅展示，不持久化决策） |
| `AcceptEdits` | 跟随上层（不影响 elicitation） |
| `BypassPermissions` | **强制降级** `RejectAllElicitationHandler` —— `BypassPermissions` 的语义是"自动接受所有写操作而不交互"，elicitation 任意 prompt 用户都不应触达 |
| `DontAsk` | 强制降级 `RejectAllElicitationHandler` |

降级由 `HarnessBuilder` 在装配时计算，业务层显式 `with_elicitation_handler(...)` 仍以业务设置为准（fail-loud：装配期日志告警）。

### 2.6 工具预过滤（McpToolFilter）

注入到 `ToolRegistry` 之前，按 server / tool 维度做 allow / deny 过滤，避免不必要的工具进入模型可见面（即便 `AutoDefer` 也会在 `tool_search` 中被检索到）。允许 / 拒绝的 glob 形态与 `harness-permission` 的 `DenyRule.scope` 共享 canonical `mcp__<server>__<tool>` 语法（见 `harness-contracts.md §3.4.2`）。

```rust
pub struct McpToolFilter {
    /// 白名单（空集 = 不限制）。
    pub allow: Vec<McpToolGlob>,
    /// 黑名单（默认空）；命中即丢弃。
    pub deny: Vec<McpToolGlob>,
    /// 当一条 mcp 工具同时命中 allow 与 deny，**deny 优先**。
    pub on_conflict: FilterConflict,
}

pub struct McpToolGlob(pub String);  // 形如 `mcp__slack__*` / `mcp__*__delete_*`

#[non_exhaustive]
pub enum FilterConflict {
    DenyWins,           // 默认
    AllowWins,
    Reject,             // 拒绝注册整条 server（fail-closed）
}

impl McpToolFilter {
    /// 返回 None 表示不应注入；Some(reason) 由调用方落 Event::McpToolInjected{ filtered_out: true, reason }
    pub fn evaluate(&self, canonical_name: &str) -> FilterDecision;
}

#[non_exhaustive]
pub enum FilterDecision {
    Inject,
    Skip { reason: String },
}
```

**决策约束**：

1. 过滤发生在 `inject_tools_into` 中、`ToolRegistry::register` 之前。被 `Skip` 的工具**不会**出现在 `discovered_tools` 投影中，也不进 deferred 池。
2. `McpToolFilter` 与 `harness-permission` 的 `DenyRule` 是**协同而非替代**：filter 决定模型是否能看见 / 检索到，permission 决定运行时是否能调用。
3. Filter 命中 `Reject` 冲突会导致整个 server 注册失败，`Event::McpToolInjected` **不发**，转而落 `Event::McpConnectionLost { reason: "filter conflict rejected" }`。
4. Glob 字符集与 `harness-permission` 的 `Rule.scope` 完全一致；不引入"MCP 专属语法"。

### 2.7 连接生命周期与重连

`ReconnectPolicy`（§2.2.1）驱动每个 `ManagedMcpServer` 在断连后的恢复尝试。生命周期与事件对应如下：

```text
Connecting ──成功──▶ Ready ──断连──▶ Reconnecting{attempt=N}
                                          │
                                  ┌───成功◀┘
                                  ▼
                                Ready
                                  │
                                  └─attempt 超过 max_attempts──▶ Failed
```

| 转换 | 触发事件 |
|---|---|
| `Connecting → Ready` | `Event::McpConnectionRecovered { server_id, was_first: true, .. }` |
| `Ready → Reconnecting` | `Event::McpConnectionLost { reason, attempts_so_far: 0 }` |
| `Reconnecting → Ready` | `Event::McpConnectionRecovered { was_first: false, total_downtime: D }` |
| `Reconnecting → Failed` | `Event::McpConnectionLost { reason, attempts_so_far: N, terminal: true }` |
| `Ready → ShuttingDown → Closed` | 不发额外事件（由 Subagent / Session 关闭引起） |

业务侧约束：

- 在 `Reconnecting` 期间，所有 `tools/call` 失败为 `McpError::Connection`，**不阻塞**主循环；模型可走 fail-soft。
- `keep_deferred_during_reconnect = true` 时，`discovered_tools` 投影保留对应 server 的工具描述，UI 显示为 `pending_mcp_servers`（见 `harness-tool-search.md §2.3`）。
- `Failed` 是终态；只有显式 `McpRegistry::reapply_server(id)` 才能再触发 `Connecting`，重连过程会**重新走 §2.2.4 trust 推导**（防止业务在过程中悄悄改写 source）。

## 3. Server Adapter 端 API

### 3.1 `HarnessMcpServer`

```rust
pub struct HarnessMcpServer {
    harness: Arc<Harness>,
    policy: McpServerPolicy,
}

impl HarnessMcpServer {
    pub fn new(harness: Arc<Harness>) -> HarnessMcpServerBuilder;

    pub async fn serve_stdio(self) -> Result<(), McpServerError>;
    pub async fn serve_http(self, addr: SocketAddr) -> Result<(), McpServerError>;
    pub async fn serve_websocket(self, addr: SocketAddr) -> Result<(), McpServerError>;
}

pub struct McpServerPolicy {
    pub exposed_capabilities: ExposedCapabilities,
    pub auth: McpServerAuth,
    pub rate_limit: RateLimit,
    pub tenant_mapping: TenantMapping,
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

对齐 HER-042 的 9 个工具 + `channels_list`。

### 3.2 Auth（Server 端）

```rust
pub enum McpServerAuth {
    None,
    StaticBearer(SecretString),
    OAuthValidator {
        issuer: Url,
        audience: String,
        jwks_url: Url,
    },
    MutualTls {
        ca_cert: Vec<u8>,
    },
    Custom(Arc<dyn McpServerAuthValidator>),
}
```

### 3.3 Tenant Mapping

```rust
pub enum TenantMapping {
    Single(TenantId),
    Claim(String),  // 从 JWT claim 里读 tenant_id
    Header(String),
    Custom(Arc<dyn TenantResolver>),
}
```

### 3.4 Tenant 隔离与速率限制

`McpServerPolicy` 在 §3.1 中已声明 `rate_limit: RateLimit` 与 `tenant_mapping: TenantMapping`；本节给出完整结构与默认值。

#### 3.4.1 `TenantIsolationPolicy`

```rust
pub struct TenantIsolationPolicy {
    /// 默认 `StrictTenant`：解析出的 tenant 必须严格等于被访问对象（Session/Permission/Memory）
    /// 的 tenant；不允许跨 tenant 读取。
    pub mode: IsolationMode,

    /// 允许哪些 `ExposedCapabilities` 在跨 tenant 时返回**摘要**（不含 payload）。
    /// 例如：`sessions_list` 可能跨 tenant 列基本元数据，但 `messages_read` 永远不可。
    pub cross_tenant_summary_caps: BTreeSet<ExposedCapability>,

    /// 跨 tenant 调用的审计级别（`Severity::High` 默认必记）。
    pub audit_severity: Severity,
}

#[non_exhaustive]
pub enum IsolationMode {
    /// 严格模式（默认）：tenant_mapping 解析失败或 mismatch 一律 401/403
    StrictTenant,
    /// 单租户部署：跳过 tenant 校验（`tenant_mapping = Single(SINGLE_TENANT)` 的简化路径）
    SingleTenant,
    /// 自定义：业务方自己实现 `TenantResolver` 后用此变体禁用 SDK 默认校验，**风险自负**
    Delegated,
}

impl Default for TenantIsolationPolicy {
    /// `mode = StrictTenant; cross_tenant_summary_caps = {}; audit_severity = High`
    fn default() -> Self { /* ... */ }
}
```

**约束**：

1. SDK 在 `serve_*` 入口验证 tenant 时，先按 `TenantMapping` 解析 → 再走 `TenantIsolationPolicy` 校验 → 最后才把请求交给业务能力。
2. 凡是 `cross_tenant_summary_caps` 未声明的 capability，跨 tenant 一律返回 `McpServerError::TenantIsolation`。
3. `audit_severity = High` 的越权尝试落 `Event::McpServerRequestRejected { reason: TenantIsolation }`（属于 `harness-server` 业务事件，不在本 SPEC 范畴）。

#### 3.4.2 `RateLimit`

```rust
pub struct RateLimit {
    /// 全局每秒峰值（0 = 不限）
    pub global_rps: u32,
    /// 单租户每秒峰值
    pub per_tenant_rps: u32,
    /// 单 capability 每秒峰值（`messages_send` 通常需要更严格）
    pub per_capability_rps: BTreeMap<ExposedCapability, u32>,
    /// 突发桶大小（单位：请求数）
    pub burst: u32,
    /// 命中限流时是否记 `Event::McpServerRequestThrottled`
    pub audit_throttle: bool,
}

impl Default for RateLimit {
    /// `global_rps = 0; per_tenant_rps = 60; per_capability_rps = {messages_send: 6};
    ///  burst = 30; audit_throttle = true`
    fn default() -> Self { /* ... */ }
}

#[non_exhaustive]
pub enum ExposedCapability {
    SessionsList, SessionGet, MessagesRead, MessagesSend,
    AttachmentsFetch, EventsPoll, EventsWait,
    PermissionsListOpen, PermissionsRespond, ChannelsList,
}
```

**约束**：

1. 限流计数在 token-bucket 实现，跨进程重启后重置（不持久化），与 OAuth refresh 计数器同样语义。
2. 命中限流返回 JSON-RPC `error.code = -32029`（自定义业务码），带 `retry_after_ms` 元数据。
3. 限流计数 / 超限事件不进 contracts `Event` 枚举（属于业务侧 `octopus-server`）；本 SPEC 仅约束 SDK 暴露的钩子。

## 4. 协议实现

### 4.1 JSON-RPC 2.0 over various transports

- Request / Response 类型在 contracts 定义
- 错误码对齐 MCP 规范（`-32000..-32100`）
- 预留 `-32042` 给 Elicitation

### 4.2 支持的方法

```text
initialize / shutdown / ping
tools/list / tools/call / tools/list_changed
resources/list / resources/read / resources/list_changed
resources/subscribe / resources/unsubscribe / resources/updated
prompts/list / prompts/get / prompts/list_changed
sampling/createMessage              (Server 反向 → Client；策略见 §6.5)
elicitation/create                  (-32042; 见 §2.5)
notifications/cancelled             (双向取消)
notifications/progress              (长操作进度)
```

`McpConnection` trait 在 `subscribe_list_changed` 之外补足 `subscribe_resource(uri)` / `subscribe_resources_list_changed()`，由 SDK 内部统一映射到 JSON-RPC `resources/subscribe + resources/updated`。每条 update 落 `Event::McpResourceUpdated`（`event-schema.md §3.19`）。

**约束**：

- `resources/updated` 推送频率受 `McpTimeouts.idle` 治理；空闲超过 idle 时间且无心跳则触发 `ReconnectPolicy`。
- `notifications/cancelled` 必须在 60s 内对齐 `tools/call` 的取消 promise，否则 SDK 视为 server 行为异常并断连。

## 5. Agent-scoped MCP 注入（CC-20）

### 5.1 引用形态

子 Agent 声明 `mcpServers`：

```rust
pub enum McpServerRef {
    Shared(McpServerId),             // 引用父连接
    Inline(McpServerSpec),           // 独立连接
    /// 声明依赖某个父级 server 必须可用；不可用时 Agent 装配失败而非 fail-soft。
    /// 与 `Shared` 的差别：`Shared` 仍允许在 server 未连接时 fail-soft，`Required` 必须成功。
    Required(McpServerId),
}
```

- **Shared**：从父 `McpRegistry` 获取连接，子 Subagent 结束时**不关闭**父连接
- **Inline**：新建连接，子 Subagent 结束时 RAII 关闭
- **Required**：复用父连接但要求 `connection_state = Ready`；不就绪则装配失败，避免 agent 在缺关键工具时盲跑

### 5.2 Inline 受 trust 限制

`McpServerRef::Inline(spec)` 的 `spec.source` **必须**满足以下任一条件，否则 SDK 在 `SubagentRunner._run` 阶段 fail-closed 拒绝装配：

1. `spec.source ∈ {Workspace, Policy, Plugin(p) where p.trust = AdminTrusted}` — 管控来源；
2. 父 Subagent 自身已是 `AdminTrusted` agent；
3. 当前 `PermissionMode = BypassPermissions` **且** 业务层在 `HarnessBuilder` 显式 `with_inline_user_mcp(true)`（fail-loud 配置开关，写入 Journal）。

否则视为 user-controlled agent 试图引入用户级 MCP server，统一拒绝（与 ADR-006 的"用户控制层不能升权"原则一致）。

约束的具体生效路径见 `crates/harness-subagent.md §3` 与 `extensibility.md §6.3`。

### 5.3 `required_mcp_servers` 校验

`SubagentSpec.required_mcp_servers`（`harness-subagent.md §2.2`）在装配期由 `McpRegistry::evaluate_required` 计算可用性：

```rust
pub struct McpServerPattern {
    /// 形如 `slack` / `mcp__slack__*` / `mcp__*__post_message`
    pub pattern: String,
    /// 是否要求 server 当前 connection_state = Ready
    pub require_ready: bool,
    /// 是否允许 Inline 形态满足该 pattern（对 Shared 总是允许）
    pub allow_inline: bool,
}

pub enum RequiredEvaluation {
    Satisfied,
    Missing { pattern: String },
    NotReady { server_id: McpServerId, state: McpConnectionState },
    InlineDisallowed { pattern: String, server_id: McpServerId },
}
```

任意 `Missing / NotReady / InlineDisallowed` 都会让 Subagent 装配返回 `SubagentError::McpRequirementUnsatisfied`，由调用方处理（典型动作：UI 提示用户连接、batch 任务直接降级为父 agent 自处理）。

## 6. 运行期 Server → Client 通知 / 反向调用

本节覆盖所有由 MCP Server 主动发起、需要 SDK 处理的运行期路径：

- §6.1 ~ §6.4：`tools/list_changed` 与 Prompt Cache 联动（ADR-003 / ADR-009）
- §6.5：`sampling/createMessage` 反向调用（SamplingPolicy / Cache 隔离）
- §6.6：`resources/updated` 与 prompts 的 `list_changed`（精简版，复用 §6.1 模型）

MCP Server 主动推送 `tools/list_changed` 会与 Prompt Cache 硬约束（ADR-003）发生冲突。ADR-009 引入 Deferred Pool 后，SDK 处理分两档：

### 6.1 Deferred 集内的增减 —— 零 cache 成本

若 `list_changed` 带来的**新增/删除工具**在当前 Session 的 `ToolSearchMode` 下会进入 **Deferred 集**（MCP 工具默认 `AutoDefer`），SDK 直接处理：

```rust
impl McpRegistry {
    async fn on_list_changed(&self, server_id: &McpServerId) -> Result<(), McpError> {
        // 1. 先打 McpToolsListChanged 事件（原始记录）
        self.emit_event(Event::McpToolsListChanged {
            server_id: server_id.clone(),
            received_at: Utc::now(),
            pending_since: self.pending_since_or_now(server_id),
        });

        // 2. 拉最新 tools 列表，与旧列表 diff
        let latest = self.connection(server_id).list_tools().await?;
        let delta = self.diff_against_snapshot(server_id, &latest);  // { added, removed }

        // 3. 若所有变更都进 Deferred 集：直接更新 ToolRegistry + 发 ToolDeferredPoolChanged
        if delta.all_deferred_in_session() {
            self.apply_to_registry(delta.clone());
            self.emit_event(Event::ToolDeferredPoolChanged {
                added: delta.added,
                removed: delta.removed,
                source: ToolPoolChangeSource::McpListChanged { server_id: server_id.clone() },
                deferred_total: self.deferred_pool_size(),
                at: Utc::now(),
            });
            // 下一轮 prompt 前，context 组装器会把 DeferredToolsDelta attachment
            // 拼到消息尾部（不碰 system prompt 前缀 → 不破坏 cache）
            return Ok(());
        }

        // 4. 含 AlwaysLoad 工具的变更 → 进 pending 队列，等业务 reload
        self.pending.write().await.insert(server_id.clone());
        Ok(())
    }
}
```

**关键性质**（对齐 ADR-003 和 ADR-009）：
- MCP 工具默认 `AutoDefer` → 绝大多数 `list_changed` 走本路径 → **零 cache miss**
- `DeferredToolsDelta` attachment 不进 system prompt 前缀
- 模型感知路径：下一轮 prompt 看到 delta 后，通过 `ToolSearchTool` 按需材化

### 6.2 AlwaysLoad 工具的增减 —— 走 reload_with

若变更包含 `DeferPolicy::AlwaysLoad` 工具（`_meta["anthropic/alwaysLoad"] = true` 的工具），该工具进 system prompt 前缀，SDK **不自动** re-inject：

- SDK 把变更堆入 `pending` 队列，等业务层主动调用
  ```rust
  session.reload_with(ConfigDelta { reapply_mcp_servers: vec![server_id] })
  ```
- Reload 按 ADR-003 §2.3 分类：
  - 只**新增** `AlwaysLoad` 工具 → `AppliedInPlace + CacheImpact::OneShotInvalidation`
  - 存在**删除** → `ForkedNewSession + CacheImpact::FullReset`
- 业务层可选择延迟 reload（例如"用户空闲 > 30s"）以最小化对进行中对话的影响

### 6.3 新 Session 自动获取最新列表

```rust
impl Harness {
    pub async fn create_session(&self, opts: SessionOptions) -> Result<Session, HarnessError> {
        // 创建期：query 所有 MCP server 的 tools/list（不使用 cache）
        // 新 Session 自然拿到最新的 tool list，并按 DeferPolicy 分桶
    }
}
```

### 6.4 Event 轨迹

```text
[MCP Server 推 tools/list_changed]
    │
    ▼
Event::McpToolsListChanged { server_id, received_at, pending_since }
    │
    ▼
[SDK diff 新旧工具列表]
    │
    ├─ delta 全在 Deferred 集（MCP 默认路径）
    │       │
    │       ▼
    │   Event::ToolDeferredPoolChanged { added, removed, source, ... }
    │       │
    │       ▼
    │   下一轮 prompt 拼 DeferredToolsDelta attachment（不破坏 cache）
    │
    └─ delta 含 AlwaysLoad 工具
            │
            ▼
        pending queue（等业务层）
            │
            ▼  (业务层显式触发)
        session.reload_with(ConfigDelta { reapply_mcp_servers: [id] })
            │
            ▼
        Event::SessionReloadApplied { mode, cache_impact, ... }
```

### 6.5 `sampling/createMessage` 反向调用与 SamplingPolicy

MCP 协议允许 Server 反向调用 Client 的 LLM（`sampling/createMessage`），让 server 端的工作流（如 RAG 重排、子任务生成）借用客户侧已经认证的模型能力。这是放大攻击面与成本失控的高风险路径，对齐 HER-040 的"七维"控制 + 与 ADR-003 Prompt Cache 的硬隔离。

#### 6.5.1 `SamplingPolicy`

```rust
pub struct SamplingPolicy {
    /// 最低准入：是否允许该 server 反向调用本端 LLM。
    pub allow: SamplingAllow,
    /// 模型选择白名单（与 `harness-model.md` ModelCatalog 对齐）。
    pub allowed_models: ModelAllowlist,
    /// 单次调用预算。
    pub per_request: SamplingBudget,
    /// 单 server / 单 session 累积预算。
    pub aggregate: AggregateBudget,
    /// 速率限制（最小粒度 = 秒）。
    pub rate_limit: SamplingRateLimit,
    /// 调用日志级别。
    pub log_level: SamplingLogLevel,
    /// **缓存隔离**：sampling 调用使用的 prompt cache 与主 Session 隔离（见 §6.5.4）。
    /// 默认 `IsolatedNamespace { ttl: 5min }`，**禁止**关闭。
    pub cache: SamplingCachePolicy,
}

#[non_exhaustive]
pub enum SamplingAllow {
    Denied,                                  // 默认；任何反向调用直接 -32601 拒绝
    AllowWithApproval { mode: ApprovalMode }, // 走 harness-permission 审批
    AllowAuto,                               // 仅 AdminTrusted server 可选；其他 source 装配期 fail-closed
}

pub struct SamplingBudget {
    pub max_input_tokens: u32,
    pub max_output_tokens: u32,
    pub max_tool_rounds: u8,                 // sampling 中允许的 tool 调用轮数（默认 0；非 0 强制走 elicitation 通报）
    pub timeout: Duration,
}

pub struct AggregateBudget {
    pub per_server_session_input_tokens: u64,
    pub per_server_session_output_tokens: u64,
    pub per_session_input_tokens: u64,
    pub per_session_output_tokens: u64,
    /// 命中 aggregate cap 后是否锁定（直到 Session 结束）；默认 `true`，避免 burst 后还能继续耗尽下一个窗口。
    pub lock_after_exceeded: bool,
}

pub struct SamplingRateLimit {
    pub per_server_rps: f32,
    pub per_session_rps: f32,
    pub burst: u32,
}

#[non_exhaustive]
pub enum SamplingLogLevel {
    None,                  // 仅指标
    Summary,               // 默认；记 metadata（model / token / latency / outcome）
    FullPrompt,            // 仅 AdminTrusted server 可选；payload 走 BlobStore
}

#[non_exhaustive]
pub enum SamplingCachePolicy {
    /// 默认：sampling 用独立 cache 命名空间（与主 Session 完全隔离）；TTL 控制反向调用 cache 寿命。
    IsolatedNamespace { ttl: Duration },
    /// 仅 AdminTrusted server + Inline Subagent 组合可选；与主 Session 共享 cache key
    /// （会潜在影响主 Session 的 cache 命中曲线，需要业务方审慎评估）。
    SharedWithMainSession,
}

pub enum ModelAllowlist {
    InheritSession,                              // 与主 Session 同一 ModelDescriptor
    Restricted(BTreeSet<String /* model_id */>), // 显式白名单
}

impl SamplingPolicy {
    /// 系统默认；任何反向调用一律拒绝（fail-closed）。
    pub fn denied() -> Self;
}
```

#### 6.5.2 `McpServerSource → SamplingAllow` 默认推导

| `McpServerSource` | `SamplingPolicy.allow` 默认 |
|---|---|
| `Workspace` / `Policy` / `Managed` / `Plugin{trust=Admin}` | `Denied`（业务必须显式打开），可选 `AllowAuto` |
| `Plugin{trust=User}` / `User` / `Project` / `Dynamic` | `Denied` 强制；显式声明 `AllowAuto` 装配期 fail-closed；`AllowWithApproval` 仅当 `PermissionMode != BypassPermissions` |

降权路径（避免 user-controlled MCP server 偷偷叫主 LLM 烧钱）。

#### 6.5.3 调用流程（高层）

```text
[Server 发 sampling/createMessage]
    │
    ▼
SamplingPolicy.allow ──Denied────────▶ 立即返回 JSON-RPC error -32601 + Event::McpSamplingRequested{outcome: Denied}
    │
    ├─AllowWithApproval ─▶ 经 harness-permission Broker 审批 (PermissionRequested/Resolved 事件)
    │
    ▼
SamplingBudget / AggregateBudget / RateLimit 校验
    │
    └─任一超限 ─▶ 返回 -32029 + Event::McpSamplingRequested{outcome: BudgetExceeded}
    │
    ▼
SamplingCachePolicy.namespace ◀─ 新建独立 ModelInvocation
    │
    ▼
harness-model::invoke (model_id 限定为 ModelAllowlist 内)
    │
    ▼
Event::McpSamplingRequested {server_id, model_id, input_tokens, output_tokens, latency_ms, outcome: Completed}
    │
    ▼
返回 sampling 结果给 Server
```

#### 6.5.4 Cache 隔离硬约束

`SamplingCachePolicy::IsolatedNamespace` 是默认值；其作用：

1. sampling 调用的 prompt 走独立的 cache 命名空间 `mcp::sampling::<server_id>::<session_id>`；
2. 此命名空间 TTL 默认 5 分钟，过期回收；
3. **不进入** `Session.message_history` 与 `SessionProjection`，避免污染主 Session cache key（与 ADR-003 §4 一致）；
4. `Event::McpSamplingRequested` 中的 `prompt_cache_namespace` 字段记录命中的 namespace，便于 audit。

`SharedWithMainSession` 仅供 admin agent + inline subagent 这种**完整可控**链路使用：业务方需要在 `HarnessBuilder` 显式 `with_sampling_share_cache(true)` 并将日志升至 `FullPrompt`，否则装配期 fail-closed。

#### 6.5.5 与 PermissionMode 的硬联动

| `PermissionMode` | sampling 行为补丁 |
|---|---|
| `BypassPermissions` / `DontAsk` | `SamplingAllow::AllowWithApproval` **降级** `Denied`（不能用绕过审批模式偷叫 LLM） |
| `Plan` | `AllowAuto` 强制 `AllowWithApproval`（方案模式下任何外部 LLM 调用都需 dry-run 审计） |
| 其他 | 按 `SamplingPolicy.allow` 原样执行 |

### 6.6 `resources/updated` 与 `prompts/list_changed`

`resources/list_changed` 与 `prompts/list_changed` 与工具不同——它们不进 system prompt 前缀，因此**不存在 cache 冲突**。SDK 行为：

1. 收到 `resources/list_changed` → 拉最新 list → diff → 落 `Event::McpResourceUpdated { server_id, kind: ListChanged, .. }`
2. 收到 `resources/updated { uri }` → 直接落 `Event::McpResourceUpdated { server_id, kind: ResourceUpdated { uri }, .. }`，不主动 read（避免拉取风暴）；业务层按需 `connection.read_resource(uri)`
3. 收到 `prompts/list_changed` → 拉最新 list → diff → 落 `Event::McpResourceUpdated { kind: PromptsListChanged }`（事件名复用，避免新增 event 维度）

资源/Prompt 推送速率受 `McpTimeouts.idle` 与 `RateLimit` 治理，过速由 SDK 主动 `subscribe_unsubscribe` 触发降级（向上游 server 退订）。

## 7. Feature Flags

```toml
[features]
default = ["stdio", "http"]
stdio = ["dep:tokio"]
http = ["dep:reqwest"]
websocket = ["dep:tokio-tungstenite"]
sse = ["dep:reqwest-eventsource"]
in-process = []
server-adapter = ["dep:axum"]
oauth = ["dep:oauth2"]
```

## 8. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("connection: {0}")]
    Connection(String),

    #[error("rpc: code={code} message={message}")]
    Rpc { code: i32, message: String },

    #[error("timeout")]
    Timeout,

    #[error("auth: {0}")]
    Auth(String),

    #[error("capability missing: {0}")]
    CapabilityMissing(String),

    #[error("unsupported transport: {0}")]
    UnsupportedTransport(String),

    /// 上游工具名违反 `harness-contracts §3.4.2` Tool Name 字符集
    /// 或经折叠后仍冲突；fail-closed 拒绝注册。
    #[error("tool naming violation: {0}")]
    ToolNamingViolation(String),
}
```

## 9. 使用示例

### 9.1 Client 入站

```rust
let registry = McpRegistry::new();
registry.add_server(McpServerSpec {
    server_id: McpServerId("filesystem".into()),
    transport: TransportChoice::Stdio {
        command: "mcp-filesystem-server".into(),
        args: vec!["--root".into(), "/workspace".into()],
        env: HashMap::new(),
    },
    auth: McpClientAuth::None,
    // ...
}).await?;

// 把 MCP 工具注入到 ToolRegistry
registry.inject_tools_into(&tool_registry, &McpServerId("filesystem".into())).await?;
```

### 9.2 Server Adapter 出站

```rust
let mcp_server = HarnessMcpServer::new(harness.clone())
    .with_exposed_capabilities(ExposedCapabilities {
        sessions_list: true,
        session_get: true,
        messages_send: true,
        events_wait: true,
        permissions_respond: true,
        ..Default::default()
    })
    .with_auth(McpServerAuth::StaticBearer(SecretString::new(token)))
    .build()?;

tokio::spawn(async move {
    mcp_server.serve_http("0.0.0.0:7820".parse().unwrap()).await
});
```

Claude Code / Cursor / Codex 可连接此 server 直接访问 Harness 能力。

## 10. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | JSON-RPC 编解码、错误码 |
| 集成 | Stdio / HTTP / WebSocket 端到端 |
| Agent-scoped | Shared 不关连接；Inline 关连接 |
| Elicitation | `-32042` 流程 + 业务 UI 回调 |
| Server Adapter | 模拟 Claude Code 客户端连接；调用 `sessions_list` 等 |

## 11. 可观测性

| 指标 | 说明 |
|---|---|
| `mcp_connection_total` | 按 transport × outcome 分桶 |
| `mcp_connection_state` | 当前连接数按 `McpConnectionState` × `McpServerSource` 分桶 |
| `mcp_reconnect_attempts_total` | 按 server_id × outcome 分桶；命中 `success_reset_after` 后清零 |
| `mcp_tool_invocations_total` | 按 server_id × tool × outcome 分桶 |
| `mcp_tool_filter_skipped_total` | `McpToolFilter` 命中 deny 的次数（按 server_id × pattern 分桶） |
| `mcp_list_changed_total` | 动态刷新次数 |
| `mcp_resource_updated_total` | `resources/updated` 推送次数 |
| `mcp_sampling_requested_total` | 按 server_id × outcome（Completed/Denied/BudgetExceeded/RateLimited）分桶 |
| `mcp_sampling_input_tokens_sum` / `mcp_sampling_output_tokens_sum` | 按 server_id × session_id 累积 |
| `mcp_server_requests_total` | Server Adapter 收到请求数 |
| `mcp_server_throttled_total` | Server Adapter `RateLimit` 命中次数 |
| `mcp_server_tenant_isolation_rejected_total` | `TenantIsolationPolicy` 拒绝次数 |
| `mcp_oauth_refresh_total` | OAuth 刷新次数 |

## 12. 反模式

- 直接把外部 MCP 工具的 schema 原封输入 LLM（应包一层验证）
- Server Adapter 不加 auth 就暴露在公网
- Inline MCP server 在 Session 中段新增（破坏 Prompt Cache）
- 忽略 `tools/list_changed`（工具名不稳定，影响 Replay）
- 自行 `format!("mcp:{}:{}", ...)` 拼接工具名（旧冒号形态在 LLM 端会被字符集校验拒绝；必须走 `harness_contracts::canonical_mcp_tool_name`）
- 把上游 MCP 工具的原始名直接送入 `tool_registry.register`（绕过 canonical 校验，可能触发 `RegistrationError::InvalidToolName`）
- **stdio Transport 直接 inherit 整个父进程环境**（凭证泄漏；必须使用 §2.2.2 的 `StdioEnv` 规则）
- **把 `McpToolFilter` 当成 `harness-permission` 的替身**（filter 控制可见性，permission 控制运行；二者不互斥）
- **`SamplingPolicy::AllowAuto` 配 `UserControlled` server**（fail-closed 拒绝；user-controlled MCP server 不应能自动反向调用主 LLM 烧钱）
- **`SamplingCachePolicy::SharedWithMainSession` 在非 admin agent 链路上启用**（污染主 Session cache key + 信息泄露）
- **跳过 `TenantIsolationPolicy`**（`Delegated` 模式仅在业务 `TenantResolver` 自身严密验证时使用，否则同 SQL 注入级别风险）
- **直接 push 自定义 transport 类型**（IDE 桥接、QUIC、Unix Socket 等扩展点应实现 `McpTransport` trait 并通过 `InProcess { factory }` 或自定义 `transport_id`，不要硬编码到 `TransportChoice` 枚举）

## 13. 相关

- D7 · `extensibility.md` §6 MCP 扩展
- ADR-003 Prompt Cache 硬约束（`list_changed` 冲突源、本文档 §6.5 Sampling cache 隔离）
- ADR-005 MCP 双向（§6.4 Sampling、§6.5 Inline trust 联动）
- ADR-006 插件信任级别（`McpServerSource → TrustLevel` 推导对齐）
- ADR-009 Deferred Tool Loading（`DeferPolicy` + `ToolDeferredPoolChanged` 事件）
- `crates/harness-contracts.md` §3.4（`McpServerSource` / `Event::McpConnectionRecovered` / `Event::McpResourceUpdated` / `Event::McpSamplingRequested`）
- `crates/harness-permission.md`（`DenyRule` 与 `McpToolFilter` 的 glob 共用）
- `crates/harness-subagent.md` §2.2 / §3（`required_mcp_servers` / Inline trust 校验）
- `crates/harness-tool-search.md` §3.10 `DeferredToolsDelta`
- Evidence: HER-040, HER-042, CC-19, CC-20, CC-21
