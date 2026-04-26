# D3 · 接口契约规范（Trait 总表）

> 依赖 ADR：ADR-002（Tool 不含 UI）, ADR-007（权限决策事件化）, ADR-009（Deferred Tool Loading）, ADR-0011（Tool Capability Handle）, ADR-0015（Plugin Loader 二分）, ADR-0016（Programmatic Tool Calling）, ADR-0017（Steering Queue）, ADR-0018（No-Loop Intercepted Tools）
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

- **实现者**：`both`（v1.0 内置 `OpenAI / Anthropic / Gemini / OpenRouter / Bedrock / Codex / LocalLlama / DeepSeek / Minimax / Qwen / Doubao / Zhipu / KM`；业务侧可实现私有 provider）
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
        tenant: TenantId,
        session_id: SessionId,
        events: &[Event],
    ) -> Result<JournalOffset, JournalError>;
    
    async fn read(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        cursor: ReplayCursor,
    ) -> Result<BoxStream<Event>, JournalError>;
    
    async fn snapshot(
        &self,
        tenant: TenantId,
        session_id: SessionId,
    ) -> Result<Option<SessionSnapshot>, JournalError>;

    async fn save_snapshot(
        &self,
        tenant: TenantId,
        snapshot: SessionSnapshot,
    ) -> Result<(), JournalError>;
    
    async fn compact_link(
        &self,
        parent: SessionId,
        child: SessionId,
        reason: ForkReason,
    ) -> Result<(), JournalError>;

    async fn list_sessions(
        &self,
        tenant: TenantId,
        filter: SessionFilter,
    ) -> Result<Vec<SessionSummary>, JournalError>;

    async fn prune(
        &self,
        tenant: TenantId,
        policy: PrunePolicy,
    ) -> Result<PruneReport, JournalError>;
}
```

- **实现者**：`both`（内置 `InMemory / Jsonl / Sqlite`；业务侧可实现 Postgres）
- **Octopus 产品基线**：`JsonlEventStore` 写 `runtime/events/*.jsonl` 是事件真相源；`SqliteEventStore` 仅是通用 SDK 后端，不得把 `data/main.db` 扩张为产品事件真相源。
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

    async fn before_execute(
        &self,
        spec: &ExecSpec,
        ctx: &ExecContext,
    ) -> Result<(), SandboxError>;

    async fn execute(
        &self,
        spec: ExecSpec,
        ctx: ExecContext,
    ) -> Result<ProcessHandle, SandboxError>;

    async fn after_execute(
        &self,
        outcome: &ExecOutcome,
        ctx: &ExecContext,
    ) -> Result<(), SandboxError>;

    async fn snapshot_session(
        &self,
        spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError>;

    async fn restore_session(
        &self,
        snapshot: &SessionSnapshotFile,
    ) -> Result<(), SandboxError>;

    async fn shutdown(&self) -> Result<(), SandboxError>;
}
```

- **实现者**：`both`（内置 `Local / Docker / Ssh / Noop`）
- **对象安全**：是

### 4.2 `ProcessHandle`

```rust
pub struct ProcessHandle {
    pub pid: Option<ProcessId>,
    pub stdout: Option<BoxStream<Bytes>>,
    pub stderr: Option<BoxStream<Bytes>>,
    pub stdin: Option<BoxStdin>,
    pub cwd_marker: Option<BoxStream<CwdMarkerLine>>,
    pub activity: Arc<dyn ActivityHandle>,
}
```

`ProcessHandle` 只承载进程句柄和流端点。等待、kill、心跳统一下沉到 `ActivityHandle`，让本地进程、远端命令、容器 exec、测试替身暴露同一活动状态面。

### 4.3 `ActivityHandle`

```rust
#[async_trait]
pub trait ActivityHandle: Send + Sync + 'static {
    async fn wait(&self) -> Result<ExecOutcome, SandboxError>;
    async fn kill(&self, signal: Signal, scope: KillScope) -> Result<(), SandboxError>;
    fn touch(&self);
    fn last_activity(&self) -> Instant;
}
```

- **实现者**：`both`
- **对象安全**：是

### 4.4 `EventSink`

```rust
pub trait EventSink: Send + Sync + 'static {
    fn emit(&self, event: Event) -> Result<(), SandboxError>;
}
```

`EventSink` 抽象“把事件投到 Journal”的能力，避免 `harness-sandbox` 反向依赖 `harness-journal`。

- **实现者**：`both`
- **对象安全**：是

### 4.5 `CodeSandbox`（feature `code-runtime`）

```rust
#[async_trait]
pub trait CodeSandbox: Send + Sync + 'static {
    fn capabilities(&self) -> CodeSandboxCapabilities;

    async fn run(
        &self,
        script: &CompiledScript,
        ctx: CodeSandboxRunContext,
    ) -> Result<CodeSandboxResult, SandboxError>;
}
```

- **实现者**：`both`（内置 mini-lua runtime；业务侧可实现受限解释器）
- **对象安全**：是

`harness-sandbox` crate 内由 `code-runtime` feature 门控；门面层 `programmatic-tool-calling` feature 组合启用 `harness-tool/programmatic-tool-calling` 与 `harness-sandbox/code-runtime`。

### 4.6 `UsageMeter`

```rust
pub trait UsageMeter: Send + Sync + 'static {
    fn record_instructions(&self, count: u64);
    fn record_event(&self, event: Event);
}
```

`UsageMeter` 仅服务 `CodeSandbox` 运行时记账。它不依赖 `harness-tool`。

- **实现者**：`both`
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

### 6.1 `MemoryStore`

> 与 `crates/harness-memory.md` §2.1 保持一致。`MemoryProvider` 不是独立大 trait，
> 而是 `MemoryStore + MemoryLifecycle` 的 blanket trait。

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync + 'static {
    fn provider_id(&self) -> &str;

    async fn recall(&self, query: MemoryQuery)
        -> Result<Vec<MemoryRecord>, MemoryError>;

    async fn upsert(&self, record: MemoryRecord)
        -> Result<MemoryId, MemoryError>;

    async fn forget(&self, id: MemoryId)
        -> Result<(), MemoryError>;

    async fn list(&self, scope: MemoryListScope)
        -> Result<Vec<MemorySummary>, MemoryError>;
}

#[async_trait]
pub trait MemoryLifecycle: Send + Sync + 'static {
    async fn initialize(&self, ctx: &MemorySessionCtx) -> Result<(), MemoryError> { Ok(()) }
    async fn on_turn_start(&self, turn: u32, message: &UserMessageView<'_>) -> Result<(), MemoryError> { Ok(()) }
    async fn on_pre_compress(&self, messages: &[MessageView<'_>]) -> Result<Option<String>, MemoryError> { Ok(None) }
    async fn on_memory_write(&self, action: MemoryWriteAction, target: &MemoryWriteTarget, content_hash: ContentHash) -> Result<(), MemoryError> { Ok(()) }
    async fn on_delegation(&self, task: &str, result: &str, child_session: SessionId) -> Result<(), MemoryError> { Ok(()) }
    async fn on_session_end(&self, ctx: &MemorySessionCtx, summary: &SessionSummaryView<'_>) -> Result<(), MemoryError> { Ok(()) }
    async fn shutdown(&self) -> Result<(), MemoryError> { Ok(()) }
}

pub trait MemoryProvider: MemoryStore + MemoryLifecycle {}
impl<T: MemoryStore + MemoryLifecycle> MemoryProvider for T {}
```

- **实现者**：`bus`（业务层接入向量库 / Graph DB）
- **约束**：每个 Session 最多 1 个外部 Provider（对齐 HER-016）
- **对象安全**：是

### 6.2 `MemoryThreatScanner`

```rust
pub struct MemoryThreatScanner { /* fields omitted */ }

impl MemoryThreatScanner {
    pub fn default() -> Self;
    pub fn from_patterns(patterns: Vec<ThreatPattern>) -> Self;
    pub fn scan(&self, content: &str) -> ThreatReport;
    pub fn redact(&self, content: &str) -> ThreatScanOutcome;
}
```

- **实现者**：`built`（正则模式库，对齐 HER-019）
- **复用方式**：以 `Arc<MemoryThreatScanner>` 注入 `BuiltinMemory`、`ExternalMemorySlot`

## 7. 工具 · `harness-tool`

### 7.1 `Tool`（核心）

```rust
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn descriptor(&self) -> &ToolDescriptor;

    fn input_schema(&self) -> &JsonSchema {
        &self.descriptor().input_schema
    }

    fn output_schema(&self) -> Option<&JsonSchema> {
        self.descriptor().output_schema.as_ref()
    }

    async fn resolve_schema(
        &self,
        ctx: &SchemaResolverContext,
    ) -> Result<JsonSchema, ToolError> {
        Ok(self.input_schema().clone())
    }
    
    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ValidationError>;
    async fn check_permission(
        &self,
        input: &Value,
        ctx: &ToolContext,
    ) -> PermissionCheck;

    async fn execute(
        &self,
        input: Value,
        ctx: ToolContext,
    ) -> Result<ToolStream, ToolError>;
}
```

- **严格禁止**：返回 `React.ReactNode` 或任何 UI 类型（ADR-002）
- **实现者**：`both`（内置 `Bash / FileRead / FileEdit / FileWrite / ListDir / Grep / Glob / WebFetch / WebSearch / Clarify / SendMessage / ReadBlob / ToolSearch / Todo / AgentTool / TaskStop / ExecuteCode`；业务侧扩展）
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

## 8. Tool Search · `harness-tool-search`

### 8.1 `ToolLoadingBackend`

```rust
#[async_trait]
pub trait ToolLoadingBackend: Send + Sync + 'static {
    fn backend_name(&self) -> ToolLoadingBackendName;

    async fn materialize(
        &self,
        ctx: &ToolLoadingContext,
        requested: &[ToolName],
    ) -> Result<MaterializeOutcome, ToolLoadingError>;
}
```

- **实现者**：`both`（内置 `AnthropicToolReferenceBackend / InlineReinjectionBackend`；业务可接入 provider-specific 物化能力）
- **对象安全**：是

### 8.2 `ToolLoadingBackendSelector`

```rust
#[async_trait]
pub trait ToolLoadingBackendSelector: Send + Sync + 'static {
    async fn select(
        &self,
        ctx: &ToolLoadingContext,
    ) -> Arc<dyn ToolLoadingBackend>;
}
```

- **实现者**：`both`
- **对象安全**：是

### 8.3 `ReloadHandle`

```rust
#[async_trait]
pub trait ReloadHandle: Send + Sync + 'static {
    async fn reload_with_add_tools(
        &self,
        tools: Vec<ToolName>,
    ) -> Result<CacheImpact, HarnessError>;
}
```

- **实现者**：`built`（由 `harness-session` / `harness-sdk` 装配）
- **对象安全**：是

### 8.4 `ToolSearchScorer`

```rust
#[async_trait]
pub trait ToolSearchScorer: Send + Sync + 'static {
    async fn score(
        &self,
        tool: &ToolDescriptor,
        properties: &ToolProperties,
        terms: &ScoringTerms,
        context: &ScoringContext,
    ) -> u32;
}
```

- **实现者**：`both`（内置 `DefaultScorer`；admin 可替换）
- **对象安全**：是

`ToolSearchTool` 是 `harness-tool-search` 的具体类型，实现 `harness-tool::Tool`。
它由 L4 `harness-sdk` 注入默认 toolset；`harness-tool` 不反向依赖 `harness-tool-search`。

## 9. 技能 · `harness-skill`

### 9.1 `SkillSource`

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

### 9.2 `SkillTemplateExpander`

```rust
pub trait SkillTemplateExpander: Send + Sync + 'static {
    fn expand(&self, template: &str, ctx: &TemplateContext) -> Result<String, TemplateError>;
}
```

- **实现者**：`built`
- **对象安全**：是

## 10. MCP · `harness-mcp`

### 10.1 入站 Client 端

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

### 10.2 Elicitation

```rust
#[async_trait]
pub trait ElicitationHandler: Send + Sync + 'static {
    async fn handle(&self, req: ElicitationRequest) -> ElicitationResponse;
}
```

- **实现者**：`bus`（业务层接管元素级询问，对齐 CC-21 的 `-32042`）

### 10.3 出站 Server Adapter

```rust
pub struct HarnessMcpServer { /* ... */ }

impl HarnessMcpServer {
    pub async fn serve_stdio(self) -> Result<(), McpError>;
    pub async fn serve_http(self, addr: SocketAddr) -> Result<(), McpError>;
}
```

- **实现者**：`built`（SDK 内置，对齐 HER-042）

## 11. Hook · `harness-hook`

### 11.1 `HookHandler`

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

### 11.2 `HookTransport`

```rust
pub struct HookPayload {
    pub event: HookEvent,
    pub ctx: HookContext,
}

pub type HookOutput = Result<HookOutcome, HookError>;

#[async_trait]
pub trait HookTransport: Send + Sync + 'static {
    async fn invoke(&self, payload: HookPayload) -> HookOutput;
}
```

- **内置实现**：`in-process` / `exec` / `http`（详见 `crates/harness-hook.md §3`）
  - `in-process` 通过 `InProcessHookTransport` 包装 `Arc<dyn HookHandler>`；该 wrapper 同时实现 `HookTransport` 与 `HookHandler`，可直接注册进 `HookRegistry`
  - `exec` 与 `http` 默认仅 `TrustLevel::AdminTrusted` 可安装；UserControlled HTTP hook 要求非空 `HookHttpSecurityPolicy.allowlist` 且 `ssrf_guard` 全部启用
  - 协议版本以 `HookProtocolVersion` 协商（详见 `harness-hook.md §3.4`）
- **扩展实现**：`HookTransport` 是开放扩展点，可由业务层按需自实现（例如以子 Agent 桥接外部 service 的 `agent-bridge` transport、或 WASM/V8 嵌入式执行器）；新增 transport 必须自行满足 §11 replay 幂等契约
- **实现者**：`both`（SDK 自带三种内置 transport，业务可叠加）
- **对象安全**：是

## 12. 上下文 · `harness-context`

### 12.1 `ContextStage`（管线节点）

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

### 12.2 `CompactStrategy`

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

## 13. Session · `harness-session`

### 13.1 `Session`（具体类型，非 trait）

> 详见 crates/harness-session.md。

#### 13.1.1 核心生命周期 API

```rust
impl Session {
    pub async fn run_turn(&self, input: TurnInput) -> Result<EventStream, SessionError>;
    pub async fn interrupt(&self) -> Result<(), SessionError>;
    pub async fn fork(&self, reason: ForkReason) -> Result<Session, SessionError>;
    pub async fn compact(&self, strategy: CompactStrategy) -> Result<Session, SessionError>;
    pub async fn reload_with(&self, delta: ConfigDelta) -> Result<ReloadOutcome, SessionError>;
    pub fn projection(&self) -> SessionProjection;
    pub fn snapshot_id(&self) -> SnapshotId;
    pub async fn end(&self, reason: EndReason) -> Result<(), SessionError>;
}
```

- **Hot Reload**：所有运行期配置修改必须走 `reload_with`，结果只能是 `AppliedInPlace / ForkedNewSession / Rejected`。
- **Fork**：新 session 写 `SessionForked`，父子血缘由 Journal `compact_link` / session projection 追踪。
- **运行**：`run_turn` 返回 `EventStream`，但事件审计真相仍以 `EventStore` 为准。

#### 13.1.2 软引导 API（ADR-0017）

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

### 13.2 `WorkspaceBootstrapLoader`

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

## 14. Engine · `harness-engine`

### 14.1 `Engine`（具体类型）

> 详见 crates/harness-engine.md

### 14.2 `InterruptSource`

```rust
#[async_trait]
pub trait InterruptSource: Send + Sync + 'static {
    async fn wait(&self) -> InterruptCause;
}
```

- **实现者**：`bus`（业务层提供用户取消信号）
- **对象安全**：是

### 14.3 `EngineRunner`

```rust
#[async_trait]
pub trait EngineRunner: Send + Sync + 'static {
    async fn run(
        &self,
        session: SessionHandle,
        input: TurnInput,
        ctx: RunContext,
    ) -> Result<EventStream, EngineError>;

    fn engine_id(&self) -> EngineId;
}
```

- **实现者**：`built`（`harness-engine::Engine` 实现），`both`（业务层可注入测试 mock）
- **对象安全**：是
- **作用**：把 `harness-engine` 的具体类型 `Engine` 抽象为 trait，供 `harness-subagent` / `harness-team` 通过 `Arc<dyn EngineRunner>` 注入而非直接 `use harness_engine::Engine`，避免 L3↔L3 的实现级耦合（详见 `module-boundaries.md §6` + ADR-008）
- **完整定义位置**：`crates/harness-engine.md §2.2`（trait 实际声明在 `harness-engine` crate 而非 `harness-contracts`，因签名引用 `RunContext` 等 engine 内部类型）

## 15. Subagent · `harness-subagent`

### 15.1 `SubagentRunner`

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

### 15.2 `DelegationPolicy`

```rust
pub trait DelegationPolicy: Send + Sync + 'static {
    fn apply(&self, parent: &ToolsetSnapshot) -> ToolsetSnapshot;
    fn max_depth(&self) -> u8;
    fn max_concurrent_children(&self) -> u16;
}
```

- **实现者**：`built`（默认 + 自定义组合）
- **对象安全**：是

## 16. Team · `harness-team`

### 16.1 `Coordinator`

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

### 16.2 `InterAgentBus`

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

## 17. Plugin · `harness-plugin`

### 17.1 `Plugin`

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

### 17.2 `PluginManifestLoader`（ADR-0015）

```rust
#[async_trait]
pub trait PluginManifestLoader: Send + Sync + 'static {
    /// 仅枚举 manifest（YAML 解析 + schema 校验 + 签名验证），
    /// **不得执行任何插件代码**。
    async fn enumerate(&self) -> Result<Vec<ManifestRecord>, PluginError>;
}
```

- **实现者**：`built`（`WorkspacePluginLoader / UserPluginLoader / ProjectPluginLoader / CargoExtensionLoader` 四源），`bus`（业务私有发现源）
- **对象安全**：是
- **类型约束**（ADR-0015 编译期保证）：返回类型固定为 `Vec<ManifestRecord>`；**禁止**返回 `Vec<Arc<dyn Plugin>>`——以类型层强制"发现期不实例化"
- **完整定义位置**：`crates/harness-plugin.md §3.2`

### 17.3 `PluginRuntimeLoader`（ADR-0015）

```rust
#[async_trait]
pub trait PluginRuntimeLoader: Send + Sync + 'static {
    /// 按 manifest 实例化 `Arc<dyn Plugin>`；仅在 `PluginRegistry::activate`
    /// 路径上调用，不得在 `enumerate` 期触发。
    async fn load(
        &self,
        record: &ManifestRecord,
        ctx: PluginActivationContext,
    ) -> Result<Arc<dyn Plugin>, PluginError>;
}
```

- **实现者**：`built`，`bus`
- **对象安全**：是
- **调用约束**：仅由 `PluginRegistry::activate` 调用；Loader 实现不得反向依赖 `PluginRegistry` 内部状态（详见 `module-boundaries.md §6`）
- **完整定义位置**：`crates/harness-plugin.md §3.2`

### 17.4 `PluginSource`

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

## 18. 观测性 · `harness-observability`

### 18.1 `Tracer`

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

### 18.2 `Redactor`

```rust
pub trait Redactor: Send + Sync + 'static {
    fn redact(&self, input: &str, rules: &RedactRules) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RedactRules {
    pub scope: RedactScope,
    pub replacement: String,
    pub pattern_set: RedactPatternSet,
}

impl Default for RedactRules {
    fn default() -> Self {
        Self {
            scope: RedactScope::EventBody,
            replacement: "[REDACTED]".into(),
            pattern_set: RedactPatternSet::Default,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RedactScope {
    All,
    TraceOnly,
    EventBody,
    LogOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RedactPatternSet {
    Default,
    AllBuiltins,
    Only(Vec<RedactPatternKind>),
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RedactPatternKind {
    ApiKey,
    BearerToken,
    PrivateKey,
    OAuthCode,
    DatabaseUrl,
    PrivateIp,
    Email,
    Custom(String),
}
```

- **实现者**：`built`（对齐 HER-051）
- **对象安全**：是
- **类型归属**：`Redactor / RedactRules / RedactScope / RedactPatternSet / RedactPatternKind` 均定义在 `octopus-harness-contracts`。`harness-observability` 只提供 `DefaultRedactor` 实现与默认正则集。

### 18.3 `ReplayEngine`（具体类型）

> 详见 crates/harness-observability.md

## 19. 错误模型

### 19.1 顶层错误

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

### 19.2 错误传递原则

- **不吞错**：除显式 `ignore_errors` 外，错误必须向上传播
- **不混淆**：业务错误与技术错误不合并（借鉴 Cursor Rules §4.3）
- **无布尔成功标志**：避免 `fn do_x() -> bool` 这种隐藏错误信息的模式

## 20. 不变式（Invariants）

| 编号 | 不变式 | 强制方式 |
|---|---|---|
| I-1 | 所有 Event 按 `session_id` 严格单增序写入 | EventStore 实现强制单调 `JournalOffset` |
| I-2 | 一个 Session 最多一个外部 MemoryProvider | `MemoryRegistry::attach` 在第二次 attach 时返回 `MemoryError::ExternalSlotBusy` |
| I-3 | Subagent 不能直接调用 `send_user_message` | `DelegationPolicy` 默认 blocklist 包含该工具 |
| I-4 | Session 中段不可改 system/toolset/memory 三件套 | 编译期 type-state + 运行期 `PromptCacheLocked` 错误 |
| I-5 | Tool 默认 fail-closed（`is_concurrency_safe=false, is_destructive=true, is_read_only=false`） | `ToolProperties::Default` 返回保守值 |
| I-6 | Sandbox 不能替代 Permission | `Engine::check_and_execute_tool` 在 sandbox 通过后仍进入 `PermissionBroker::decide` |

## 21. 版本策略

- **SDK 版本**：遵循 SemVer 2.0
- **破坏性变更**：Major 版本；需 ADR
- **新增 API**：Minor 版本
- **BugFix / 实现优化**：Patch 版本
- **Event Schema 变更**：视为破坏性，强 Major 升级，且**Schema 迁移工具**必须同时提供
