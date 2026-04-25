# D3 · 接口契约规范（Trait 总表）

> 依赖 ADR：ADR-002（Tool 不含 UI）, ADR-007（权限决策事件化）
> 状态：Accepted · 本文是 **trait 签名单一事实源**，crate SPEC 细化实现细节

## 1. 总览

本文档列出 SDK 对外所有 `pub trait`。每个 trait：

- **签名**：Rust 代码（省略 doc comment，详见对应 crate SPEC）
- **实现者**：业务层（`bus`） / SDK 内置（`built`） / 两者都可能（`both`）
- **对象安全**：是否可作为 `dyn Trait`

所有 trait 默认 `Send + Sync + 'static`（除非明确标注）。

## 2. 模型接入 · `harness-model`

### 2.1 `ModelProvider`（核心）

```rust
#[async_trait]
pub trait ModelProvider: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    fn supported_models(&self) -> Vec<ModelDescriptor>;
    async fn infer(
        &self,
        req: ModelRequest,
        ctx: InferContext,
    ) -> Result<BoxStream<ModelStreamEvent>, ModelError>;
    fn prompt_cache_style(&self) -> PromptCacheStyle;
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn supports_thinking(&self) -> bool { false }
    async fn health(&self) -> HealthStatus { HealthStatus::Healthy }
}
```

- **实现者**：`both`（内置 `OpenAi / Anthropic / Gemini / OpenRouter / Bedrock / Codex / LocalLlama`；业务侧可实现私有 provider）
- **对象安全**：是
- **`InferContext`**：详细字段见 `crates/harness-model.md` §2.1.0；包含 `tenant_id / cancel / retry_policy / tracing / middlewares` 等

### 2.2 `CredentialSource`

```rust
#[async_trait]
pub trait CredentialSource: Send + Sync + 'static {
    async fn fetch(&self, key: CredentialKey) -> Result<CredentialValue, CredentialError>;
    async fn rotate(&self, key: CredentialKey) -> Result<(), CredentialError>;
}
```

- **实现者**：`bus`（业务层读取密钥 / OAuth / Vault）
- **对象安全**：是

### 2.3 `TokenCounter`

```rust
pub trait TokenCounter: Send + Sync + 'static {
    fn count_tokens(&self, text: &str, model: &str) -> usize;
    fn count_messages(&self, messages: &[Message], model: &str) -> usize;
    fn count_image(&self, image: &ImageMeta, model: &str) -> Option<usize> { None }
}
```

- **实现者**：`built`（基于 `tiktoken` / `anthropic-tokenizer`）
- **对象安全**：是
- **`count_image` 返回 `None`**：表示模型无视觉规则；调用方负责回退（不得静默按 0 计入预算）

### 2.4 `AuxModelProvider`

```rust
#[async_trait]
pub trait AuxModelProvider: Send + Sync + 'static {
    fn inner(&self) -> Arc<dyn ModelProvider>;
    fn aux_options(&self) -> AuxOptions;
    async fn call_aux(&self, task: AuxTask, req: ModelRequest)
        -> Result<String, ModelError>;
}
```

- **实现者**：`both`（内置 `BasicAuxProvider`：直接包一个 `ModelProvider`；业务可实现专用路由）
- **对象安全**：是
- **必须独立**：与 `ModelProvider` 共享 trait 会让 Compact / PermissionAdvisory / Classify 等任务的超时与并发约束相互污染（HER-025）。详见 `crates/harness-model.md` §5.1

### 2.5 `InferMiddleware`

```rust
#[async_trait]
pub trait InferMiddleware: Send + Sync + 'static {
    fn middleware_id(&self) -> &str;

    async fn before_request(
        &self,
        req: &mut ModelRequest,
        ctx: &mut InferContext,
    ) -> Result<(), ModelError> { Ok(()) }

    async fn on_response_headers(
        &self,
        headers: &http::HeaderMap,
        ctx: &InferContext,
    ) -> Result<(), ModelError> { Ok(()) }

    fn wrap_stream(
        &self,
        stream: BoxStream<ModelStreamEvent>,
        ctx: &InferContext,
    ) -> BoxStream<ModelStreamEvent> { stream }

    async fn on_request_end(
        &self,
        usage: &UsageSnapshot,
        ctx: &InferContext,
    ) -> Result<(), ModelError> { Ok(()) }
}
```

- **实现者**：`both`（内置 `RateLimitObserver / OAuthAutoRefresh / RedactStream / TraceSpan`，详见 `crates/harness-model.md` §2.6）
- **对象安全**：是
- **顺序语义**：`before_request` 按 Vec 顺序、`wrap_stream` 反序包裹（与 Tower / Axum 一致）
- **禁止改写 `messages`**（违反 P5 / ADR-003），此类需求请使用 `PreLlmCall` Hook

### 2.6 `ProviderHealthCheck`（可选）

```rust
#[async_trait]
pub trait ProviderHealthCheck: Send + Sync + 'static {
    async fn probe(&self) -> HealthStatus;
}
```

- **实现者**：`bus`（生产侧需要主动探活的 Provider 才注入；缺省由 `ModelProvider::health()` 默认返回 `Healthy`）
- **对象安全**：是

## 3. 事件持久化 · `harness-journal`

### 3.1 `EventStore`（核心）

```rust
#[async_trait]
pub trait EventStore: Send + Sync + 'static {
    async fn append(
        &self,
        session_id: SessionId,
        events: &[Event],
    ) -> Result<JournalOffset, JournalError>;
    
    async fn read(
        &self,
        session_id: SessionId,
        cursor: ReplayCursor,
    ) -> Result<BoxStream<Event>, JournalError>;
    
    async fn snapshot(
        &self,
        session_id: SessionId,
    ) -> Result<Option<SessionSnapshot>, JournalError>;
    
    async fn link_compaction(
        &self,
        parent: SessionId,
        child: SessionId,
        reason: CompactReason,
    ) -> Result<(), JournalError>;
}
```

- **实现者**：`both`（内置 `InMemory / Jsonl / Sqlite`；业务侧可实现 Postgres）
- **对象安全**：是

### 3.2 `Projection`

```rust
pub trait Projection: Send + Sync + 'static {
    type Output;
    fn apply(&mut self, event: &Event);
    fn snapshot(&self) -> Self::Output;
}
```

- **实现者**：`both`
- **对象安全**：否（含关联类型）；应通过具体类型暴露

### 3.3 `BlobStore`（大块数据存取）

```rust
#[async_trait]
pub trait BlobStore: Send + Sync + 'static {
    fn store_id(&self) -> &str;
    async fn put(&self, tenant: TenantId, bytes: Bytes, meta: BlobMeta)
        -> Result<BlobRef, BlobError>;
    async fn get(&self, tenant: TenantId, blob: &BlobRef)
        -> Result<BoxStream<Bytes>, BlobError>;
    async fn head(&self, tenant: TenantId, blob: &BlobRef)
        -> Result<Option<BlobMeta>, BlobError>;
    async fn delete(&self, tenant: TenantId, blob: &BlobRef)
        -> Result<(), BlobError>;
}
```

- **类型定义**：见 `crates/harness-contracts.md` §3.7
- **实现者**：`both`（内置 `FileBlobStore / SqliteBlobStore / InMemoryBlobStore`；业务侧可实现 `S3BlobStore` 等）
- **对象安全**：是
- **与 Event 的关系**：Event 里的 `BlobRef` 是轻量句柄；实际字节流经 `BlobStore` 存取。`Event::ToolUseCompleted.result` / `MessagePart::Image` / `CompactionApplied.summary_ref` 等场景均以 `BlobRef` 形式出现
- **与 Journal 的关系**：`BlobStore` 是 Journal 的**同级基础设施**（都在 L1），生命周期由 `BlobRetention` + Journal prune 联动

## 4. 沙箱 · `harness-sandbox`

### 4.1 `SandboxBackend`（核心）

```rust
#[async_trait]
pub trait SandboxBackend: Send + Sync + 'static {
    fn backend_id(&self) -> &str;
    fn capabilities(&self) -> SandboxCapabilities;
    
    async fn execute(
        &self,
        spec: ExecSpec,
        ctx: ExecContext,
    ) -> Result<ProcessHandle, SandboxError>;
    
    async fn snapshot_session(
        &self,
        spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError>;
}
```

- **实现者**：`both`（内置 `Local / Docker / Ssh / Noop`）
- **对象安全**：是

### 4.2 `ProcessHandle`

```rust
#[async_trait]
pub trait ProcessHandle: Send + Sync + 'static {
    fn id(&self) -> ProcessId;
    async fn stdout(&self) -> BoxStream<Bytes>;
    async fn stderr(&self) -> BoxStream<Bytes>;
    async fn wait(&self) -> Result<ExitStatus, SandboxError>;
    async fn kill(&self, signal: Signal) -> Result<(), SandboxError>;
    fn heartbeat(&self) -> ActivityHeartbeat;
}
```

- **对象安全**：是

## 5. 权限 · `harness-permission`

> 本节所有引用的 `Decision` / `DecisionScope` / `DecidedBy` / `PermissionMode` / `Severity` 均定义在 L0 契约层 `harness-contracts`（详见 `crates/harness-contracts.md` §3.4）；本节只声明 trait 方法签名。

### 5.1 `PermissionBroker`（核心）

```rust
#[async_trait]
pub trait PermissionBroker: Send + Sync + 'static {
    async fn decide(
        &self,
        request: PermissionRequest,
        ctx: PermissionContext,
    ) -> Decision;
    
    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<(), PermissionError>;
}
```

- **实现者**：`both`（内置 `DenyAll / AllowAll / RuleEngine / StreamBased / Direct(fn)`；业务侧可实现企业 SSO 审批）
- **对象安全**：是

### 5.2 `RuleProvider`

```rust
#[async_trait]
pub trait RuleProvider: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    fn source(&self) -> RuleSource;
    async fn resolve_rules(&self, tenant: TenantId)
        -> Result<Vec<PermissionRule>, PermissionError>;
    fn watch(&self) -> Option<BoxStream<RulesUpdated>>;
}
```

- **实现者**：`both`（内置 `FileRuleProvider / SettingsRuleProvider / CliArgRuleProvider / SessionRuleProvider`；业务层可实现企业 IAM 推送等）
- **对象安全**：是
- **watch 语义**：返回 `None` 代表静态 provider（从不更新）；返回 `Some(stream)` 代表 provider 会推送热更新，`RuleEngineBroker` 订阅后原子替换规则 snapshot（详见 `crates/harness-permission.md` §3.4.2）

### 5.3 `DecisionPersistence`

```rust
#[async_trait]
pub trait DecisionPersistence: Send + Sync + 'static {
    async fn save(&self, decision: PersistedDecision) -> Result<()>;
    async fn load(&self, scope: DecisionScope) -> Vec<PersistedDecision>;
}
```

- **实现者**：`both`
- **对象安全**：是

## 6. 记忆 · `harness-memory`

### 6.1 `MemoryProvider`

> 与 `crates/harness-memory.md` §2.1 保持一致。

```rust
#[async_trait]
pub trait MemoryProvider: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    async fn recall(&self, query: MemoryQuery)
        -> Result<Vec<MemoryRecord>, MemoryError>;
    async fn upsert(&self, record: MemoryRecord)
        -> Result<(), MemoryError>;
    async fn forget(&self, id: MemoryId)
        -> Result<(), MemoryError>;
    async fn list(&self, scope: MemoryScope)
        -> Result<Vec<MemorySummary>, MemoryError>;
}
```

- **实现者**：`bus`（业务层接入向量库 / Graph DB）
- **约束**：每个 Session 最多 1 个外部 Provider（对齐 HER-016）
- **对象安全**：是

### 6.2 `MemoryThreatScanner`

```rust
pub trait MemoryThreatScanner: Send + Sync + 'static {
    fn scan(&self, content: &str) -> ThreatReport;
}
```

- **实现者**：`built`（正则模式库，对齐 HER-019）
- **对象安全**：是

## 7. 工具 · `harness-tool`

### 7.1 `Tool`（核心）

```rust
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn descriptor(&self) -> ToolDescriptor;
    fn input_schema(&self) -> &JsonSchema;
    fn output_schema(&self) -> Option<&JsonSchema>;
    fn properties(&self) -> ToolProperties;
    
    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError>;
    async fn check_permission(
        &self,
        input: &Value,
        ctx: &ToolContext,
    ) -> PermissionCheck;
    async fn invoke(&self, input: Value, ctx: ToolContext) -> ToolResult;
}
```

- **严格禁止**：返回 `React.ReactNode` 或任何 UI 类型（ADR-002）
- **实现者**：`both`（内置 `Bash / ReadFile / EditFile / Grep / Glob / WebFetch / Todo / AgentTool / TaskStop`；业务侧扩展）
- **对象安全**：是

### 7.2 `ToolRegistry`（非 trait，具体类型）

> 参见 crates/harness-tool.md §核心结构

### 7.3 `DangerousCommandDetector`

```rust
pub trait DangerousCommandDetector: Send + Sync + 'static {
    fn is_dangerous(&self, command: &str) -> Option<DangerReport>;
}
```

- **实现者**：`built`（对齐 HER-039）
- **对象安全**：是

## 8. 技能 · `harness-skill`

### 8.1 `SkillSource`

```rust
#[async_trait]
pub trait SkillSource: Send + Sync + 'static {
    fn source_id(&self) -> &str;
    fn priority(&self) -> u32;
    async fn discover(&self) -> Result<Vec<SkillRecord>, SkillError>;
    async fn load(&self, skill_id: &SkillId) -> Result<SkillContent, SkillError>;
    fn watch(&self) -> Option<BoxStream<SkillUpdated>>;
}
```

- **实现者**：`both`（内置 `Workspace / User / Plugin / Managed / Bundled / Mcp`，对齐 CC-26）
- **对象安全**：是

### 8.2 `SkillTemplateExpander`

```rust
pub trait SkillTemplateExpander: Send + Sync + 'static {
    fn expand(&self, template: &str, ctx: &TemplateContext) -> Result<String, TemplateError>;
}
```

- **实现者**：`built`
- **对象安全**：是

## 9. MCP · `harness-mcp`

### 9.1 入站 Client 端

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
    fn connection_id(&self) -> &str;
    async fn list_tools(&self) -> Result<Vec<McpToolDef>, McpError>;
    async fn call_tool(&self, name: &str, input: Value) -> Result<McpToolResult, McpError>;
    async fn subscribe_changes(&self) -> Result<BoxStream<McpChange>, McpError>;
    async fn close(&self) -> Result<(), McpError>;
}
```

- **实现者**：`both`（内置 `stdio / http / websocket / sse / in-process`）

### 9.2 Elicitation

```rust
#[async_trait]
pub trait ElicitationHandler: Send + Sync + 'static {
    async fn handle(&self, req: ElicitationRequest) -> ElicitationResponse;
}
```

- **实现者**：`bus`（业务层接管元素级询问，对齐 CC-21 的 `-32042`）

### 9.3 出站 Server Adapter

```rust
pub struct HarnessMcpServer { /* ... */ }

impl HarnessMcpServer {
    pub async fn serve_stdio(self) -> Result<(), McpError>;
    pub async fn serve_http(self, addr: SocketAddr) -> Result<(), McpError>;
}
```

- **实现者**：`built`（SDK 内置，对齐 HER-042）

## 10. Hook · `harness-hook`

### 10.1 `HookHandler`

```rust
#[async_trait]
pub trait HookHandler: Send + Sync + 'static {
    fn handler_id(&self) -> &str;
    fn interested_events(&self) -> &[HookEventKind];
    fn priority(&self) -> i32 { 0 }

    async fn handle(
        &self,
        event: HookEvent,
        ctx: HookContext,
    ) -> Result<HookOutcome, HookError>;
}
```

- **输出能力**：详细矩阵见 `crates/harness-hook.md §2.4`，关键约束：
  - `PreToolUse` **唯一支持复合三件套**：返回 `HookOutcome::PreToolUse(PreToolUseOutcome { rewrite_input, override_permission, additional_context, block })`，可一次同时改写输入、覆盖审批、注入上下文；不允许在该事件下返回单一 `RewriteInput / OverridePermission / AddContext`（dispatcher 视为 `HookReturnedUnsupported`）。
  - `TransformToolResult` / `TransformTerminalOutput` 是改写工具结果与原始终端字节的**唯一通道**。
  - `PostToolUse` 仅允许 `Continue` / `AddContext`，**不能**改写 result。
  - 其他事件按 §2.4 矩阵决定，不在矩阵内的 outcome 一律按 `HookReturnedUnsupported` 处理。
  - handler 必须满足 `harness-hook.md §11` 的 **replay 幂等契约**——纯函数化、不持久化外部副作用、不读易变全局态。
- **实现者**：`bus`
- **对象安全**：是
- **`HookContext`**：结构定义见 `crates/harness-hook.md §2.2.1`；本契约层只承诺该结构是只读快照、字段全部 `Clone + Send`。

### 10.2 `HookTransport`

```rust
#[async_trait]
pub trait HookTransport: Send + Sync + 'static {
    async fn invoke(&self, payload: HookPayload) -> HookOutput;
}
```

- **内置实现**：`in-process` / `exec` / `http`（详见 `crates/harness-hook.md §3`）
  - `exec` 与 `http` 默认仅 `TrustLevel::AdminTrusted` 可安装；UserControlled HTTP hook 要求非空 `HookHttpSecurityPolicy.allowlist` 且 `ssrf_guard` 全部启用
  - 协议版本以 `HookProtocolVersion` 协商（详见 `harness-hook.md §3.4`）
- **扩展实现**：`HookTransport` 是开放扩展点，可由业务层按需自实现（例如以子 Agent 桥接外部 service 的 `agent-bridge` transport、或 WASM/V8 嵌入式执行器）；新增 transport 必须自行满足 §11 replay 幂等契约
- **实现者**：`both`（SDK 自带三种内置 transport，业务可叠加）
- **对象安全**：是

## 11. 上下文 · `harness-context`

### 11.1 `ContextStage`（管线节点）

```rust
#[async_trait]
pub trait ContextStage: Send + Sync + 'static {
    fn stage_id(&self) -> ContextStageId;
    async fn apply(
        &self,
        context: &mut ContextBuffer,
        ctx: &ContextEngineContext,
    ) -> Result<(), ContextError>;
}
```

- **固定管线顺序**：`ToolResultBudget → Snip → Microcompact → Collapse → Autocompact`（ADR-003）
- **实现者**：`built`（顺序不可改；每阶段内部可插拔 Strategy）
- **对象安全**：是

### 11.2 `CompactStrategy`

```rust
#[async_trait]
pub trait CompactStrategy: Send + Sync + 'static {
    async fn summarize(
        &self,
        messages: &[Message],
        budget: TokenBudget,
    ) -> Result<CompactedSegment, ContextError>;
}
```

- **实现者**：`built`
- **对象安全**：是

## 12. Session · `harness-session`

### 12.1 `Session`（具体类型，非 trait）

> 详见 crates/harness-session.md。

#### 12.1.1 软引导 API（ADR-0017）

```rust
impl Session {
    /// 任意运行期可调用；与 `interrupt()` 互补，永远不终止 Run。
    /// 队列规则、合并语义、容量与 TTL 见 `crates/harness-session.md §2.7`。
    pub async fn push_steering(
        &self,
        msg: SteeringRequest,
    ) -> Result<SteeringId, SessionError>;

    /// 取出当前 Steering 队列的快照（仅用于 UI 展示，不修改队列）。
    pub fn steering_snapshot(&self) -> SteeringSnapshot;
}
```

- **可观测性**：`Event::SteeringMessageQueued / Applied / Dropped`（`event-schema.md §3.5.1`）
- **能力闸门**：`SteeringSource::Plugin` 必须配合 `harness-plugin` capability handle
- **失败模式**：feature flag off → `SessionError::FeatureDisabled`；
  source 非法 → `SessionError::SteeringSourceDenied`

### 12.2 `WorkspaceBootstrapLoader`

```rust
#[async_trait]
pub trait WorkspaceBootstrapLoader: Send + Sync + 'static {
    async fn load(
        &self,
        workspace_path: &Path,
    ) -> Result<WorkspaceBootstrap, SessionError>;
}
```

- **作用**：加载 `AGENTS.md / CLAUDE.md / USER.md / SOUL.md / IDENTITY.md / TOOLS.md / BOOTSTRAP.md`（对齐 OC-07）
- **实现者**：`built`

## 13. Engine · `harness-engine`

### 13.1 `Engine`（具体类型）

> 详见 crates/harness-engine.md

### 13.2 `InterruptSource`

```rust
#[async_trait]
pub trait InterruptSource: Send + Sync + 'static {
    async fn wait(&self) -> InterruptCause;
}
```

- **实现者**：`bus`（业务层提供用户取消信号）
- **对象安全**：是

## 14. Subagent · `harness-subagent`

### 14.1 `SubagentRunner`

```rust
#[async_trait]
pub trait SubagentRunner: Send + Sync + 'static {
    async fn spawn(
        &self,
        spec: SubagentSpec,
        input: TurnInput,
    ) -> Result<SubagentHandle, SubagentError>;
}
```

- **实现者**：`built`
- **对象安全**：是

### 14.2 `DelegationPolicy`

```rust
pub trait DelegationPolicy: Send + Sync + 'static {
    fn apply(&self, parent: &ToolsetSnapshot) -> ToolsetSnapshot;
    fn max_depth(&self) -> u8;
    fn max_concurrent_children(&self) -> u16;
}
```

- **实现者**：`built`（默认 + 自定义组合）
- **对象安全**：是

## 15. Team · `harness-team`

### 15.1 `Coordinator`

```rust
#[async_trait]
pub trait Coordinator: Send + Sync + 'static {
    async fn dispatch(
        &self,
        team: &Team,
        input: TeamInput,
    ) -> Result<EventStream, TeamError>;
    
    async fn route(
        &self,
        message: AgentMessage,
    ) -> Result<Vec<AgentId>, TeamError>;
    
    async fn terminate(
        &self,
        team: &Team,
        reason: TerminationReason,
    ) -> Result<(), TeamError>;
}
```

- **实现者**：`both`
- **对象安全**：是

### 15.2 `InterAgentBus`

```rust
#[async_trait]
pub trait InterAgentBus: Send + Sync + 'static {
    async fn publish(
        &self,
        from: AgentId,
        to: Recipient,
        payload: MessagePayload,
    ) -> Result<(), TeamError>;
    
    async fn subscribe(
        &self,
        agent: AgentId,
    ) -> BoxStream<AgentMessage>;
    
    async fn broadcast(
        &self,
        from: AgentId,
        payload: MessagePayload,
    ) -> Result<(), TeamError>;
}
```

- **实现者**：`built`（基于 `tokio::sync::broadcast` + 持久化到 Journal）
- **对象安全**：是

## 16. Plugin · `harness-plugin`

### 16.1 `Plugin`

```rust
#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    fn manifest(&self) -> &PluginManifest;
    fn trust_level(&self) -> TrustLevel;
    async fn activate(&self, slots: CapabilitySlots) -> Result<(), PluginError>;
    async fn deactivate(&self) -> Result<(), PluginError>;
}
```

- **实现者**：`bus`
- **对象安全**：是

### 16.2 `PluginSource`

```rust
#[async_trait]
pub trait PluginSource: Send + Sync + 'static {
    fn source_id(&self) -> &str;
    async fn discover(&self) -> Result<Vec<PluginDescriptor>, PluginError>;
    async fn load(&self, id: &PluginId) -> Result<Box<dyn Plugin>, PluginError>;
}
```

- **实现者**：`both`（四源：workspace / user / project / entry-point，对齐 HER-033）
- **对象安全**：是

## 17. 观测性 · `harness-observability`

### 17.1 `Tracer`

```rust
#[async_trait]
pub trait Tracer: Send + Sync + 'static {
    async fn span(
        &self,
        name: &str,
        attrs: SpanAttributes,
    ) -> SpanHandle;
    
    fn flush(&self) -> BoxFuture<'static, Result<(), TraceError>>;
}
```

- **实现者**：`built`（内置 `OtelTracer / ConsoleTracer / NoopTracer`）
- **对象安全**：是

### 17.2 `Redactor`

```rust
pub trait Redactor: Send + Sync + 'static {
    fn redact(&self, input: &str, rules: &RedactRules) -> String;
}
```

- **实现者**：`built`（对齐 HER-051）
- **对象安全**：是

### 17.3 `ReplayEngine`（具体类型）

> 详见 crates/harness-observability.md

## 18. 错误模型

### 18.1 顶层错误

```rust
#[derive(Debug, thiserror::Error)]
pub enum HarnessError {
    #[error("model: {0}")]
    Model(#[from] ModelError),
    #[error("journal: {0}")]
    Journal(#[from] JournalError),
    #[error("sandbox: {0}")]
    Sandbox(#[from] SandboxError),
    #[error("permission: {0}")]
    Permission(#[from] PermissionError),
    #[error("memory: {0}")]
    Memory(#[from] MemoryError),
    #[error("tool: {0}")]
    Tool(#[from] ToolError),
    #[error("session: {0}")]
    Session(#[from] SessionError),
    #[error("engine: {0}")]
    Engine(#[from] EngineError),
    #[error("plugin: {0}")]
    Plugin(#[from] PluginError),
    #[error("mcp: {0}")]
    Mcp(#[from] McpError),
    #[error("hook: {0}")]
    Hook(#[from] HookError),
    #[error("context: {0}")]
    Context(#[from] ContextError),
    #[error("prompt cache locked for running session")]
    PromptCacheLocked,
    #[error("tenant mismatch")]
    TenantMismatch,
    #[error("other: {0}")]
    Other(String),
}

pub type Result<T, E = HarnessError> = std::result::Result<T, E>;
```

### 18.2 错误传递原则

- **不吞错**：除显式 `ignore_errors` 外，错误必须向上传播
- **不混淆**：业务错误与技术错误不合并（借鉴 Cursor Rules §4.3）
- **无布尔成功标志**：避免 `fn do_x() -> bool` 这种隐藏错误信息的模式

## 19. 不变式（Invariants）

| 编号 | 不变式 | 强制方式 |
|---|---|---|
| I-1 | 所有 Event 按 `session_id` 严格单增序写入 | EventStore 实现强制单调 `JournalOffset` |
| I-2 | 一个 Session 最多一个外部 MemoryProvider | `MemoryRegistry::attach` 在第二次 attach 时返回 `MemoryError::ExternalSlotBusy` |
| I-3 | Subagent 不能直接调用 `send_user_message` | `DelegationPolicy` 默认 blocklist 包含该工具 |
| I-4 | Session 中段不可改 system/toolset/memory 三件套 | 编译期 type-state + 运行期 `PromptCacheLocked` 错误 |
| I-5 | Tool 默认 fail-closed（`is_concurrency_safe=false, is_destructive=true, is_read_only=false`） | `ToolProperties::Default` 返回保守值 |
| I-6 | Sandbox 不能替代 Permission | `Engine::check_and_execute_tool` 在 sandbox 通过后仍进入 `PermissionBroker::decide` |

## 20. 版本策略

- **SDK 版本**：遵循 SemVer 2.0
- **破坏性变更**：Major 版本；需 ADR
- **新增 API**：Minor 版本
- **BugFix / 实现优化**：Patch 版本
- **Event Schema 变更**：视为破坏性，强 Major 升级，且**Schema 迁移工具**必须同时提供
