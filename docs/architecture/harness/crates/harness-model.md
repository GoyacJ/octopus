# `octopus-harness-model` · L1 原语 · Model Provider SPEC

> 层级：L1 · 状态：Accepted
> 依赖：`harness-contracts`

## 1. 职责

提供 **LLM Provider 抽象 + 凭证池 + Prompt-Cache 策略**。把不同厂商/模型的差异归一到一个 trait 下。

**核心能力**：

- 多 Provider 支持（OpenAI / Anthropic / Gemini / OpenRouter / Bedrock / Codex / Local Llama）
- 凭证池（多 key 轮转 + 冷却 + 策略选择，对齐 HER-048）
- Prompt Cache 策略适配（Anthropic system_and_3 / OpenAI auto / Gemini context caching）
- 消息归一化（把不同厂商的 tool_use 格式归一为 `harness-contracts::MessagePart`）
- 流式推理（返回 `BoxStream<ModelStreamEvent>`）

## 2. 对外 API

### 2.1 核心 Trait

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

    /// 健康探测默认为 `Healthy`；可选实现。见 §5.2 `ProviderHealthCheck`。
    async fn health(&self) -> HealthStatus { HealthStatus::Healthy }
}
```

> **能力声明规则**：`ModelProvider::supports_*` 是 **provider 级默认值**，仅在没有 `ModelDescriptor.capabilities` 的情况下兜底。正确的查询路径：
>
> 1. `ModelCatalog::get(model_ref).capabilities.supports_X`（model 级，精确）
> 2. fallback：`provider.supports_X()`（provider 级）
>
> `BackendSelector` / `ToolSearchBackendSelector` 按此顺序查询。

### 2.1.0 `InferContext`（推理调用上下文）

`InferContext` 是 `ModelProvider::infer` 的**唯一**可变控制通道：

```rust
pub struct InferContext {
    /// 唯一请求 ID；进入 Event Journal 后用于关联 LLM 调用
    pub request_id: RequestId,
    pub tenant_id: TenantId,
    pub session_id: Option<SessionId>,
    pub run_id: Option<RunId>,

    /// 取消信号。Provider 必须在 hot loop 里 `select!` / `poll` 它，
    /// 一旦触发需尽快关闭底层 HTTP / gRPC 连接（对齐 HER-031）
    pub cancel: CancellationToken,

    /// 软超时：达到后 Provider 必须尽力让流以 `ModelStreamEvent::StreamError
    /// { class: ErrorClass::Transient, .. }` 收束。
    /// 注意：deadline 不是硬超时；硬超时由业务层通过 `cancel` 强制。
    pub deadline: Option<Instant>,

    /// 重试策略（仅 Provider 自己的瞬态重试；跨 Provider fallback 由 Catalog 触发）
    pub retry_policy: RetryPolicy,

    /// OpenTelemetry 上下文（若启用 `otel`）
    pub tracing: Option<TraceContext>,

    /// Provider 之前需要跑的 Middleware 链（顺序 = Vec 顺序）
    /// 详见 §2.6 `InferMiddleware`
    pub middlewares: Vec<Arc<dyn InferMiddleware>>,
}

pub struct RetryPolicy {
    pub max_attempts: u32,          // 默认 3
    pub backoff: Backoff,           // 指数退避 + jitter
    pub retry_on: RetryClassifier,  // 一般只重试 ErrorClass::Transient / RateLimited
}

pub enum Backoff {
    Fixed(Duration),
    Exponential { initial: Duration, factor: f32, cap: Duration },
}

pub type RetryClassifier = Arc<dyn Fn(&ErrorClass) -> bool + Send + Sync>;
```

> **为什么必须显式**：原先把"是否支持取消/重试/tracing"留给各 Provider 自行实现，会造成不可观测的差异（Hermes 就因此踩过 429 重试风暴）。这里把取消、超时、重试、链路追踪、Middleware 全部挤到一个**必传**结构体里，Provider 实现没有理由再"绕开"。

> **注入**：`ModelCatalog::infer(req)` 会自动构造 `InferContext`（基于 `Session` / `Run` 信息）；SDK 直接调用 `provider.infer(req, ctx)` 时必须显式构造。

### 2.1.1 `ModelDescriptor` 与 `ModelPricing`

```rust
pub struct ModelDescriptor {
    pub provider_id: String,
    pub model_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub capabilities: ModelCapabilities,
    pub pricing: Option<ModelPricing>,
}

/// 模型能力矩阵。与 `ModelProvider` trait 的 `supports_*` 方法呼应，
/// 作为 ModelCatalog 对外的能力声明。
#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub supports_thinking: bool,
    pub supports_prompt_cache: bool,
    /// 是否支持 Anthropic `tool_reference` content block（ADR-009 §2.10）。
    /// 驱动 `BackendSelector` 在 `AnthropicToolReferenceBackend` 与
    /// `InlineReinjectionBackend` 之间选择。
    pub supports_tool_reference: bool,
    /// 若走 `tool_reference` 路径，额外需携带的 beta header。
    /// 形如 `Some("tool-reference-2025-04")`；None 表示无需特殊 header。
    pub tool_reference_beta_header: Option<&'static str>,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        // Fail-closed：未显式声明即保守假定不支持高级能力
        Self {
            supports_tools: true,
            supports_vision: false,
            supports_thinking: false,
            supports_prompt_cache: false,
            supports_tool_reference: false,
            tool_reference_beta_header: None,
        }
    }
}

pub struct ModelPricing {
    /// **版本化标识**。`(pricing_id, pricing_version)` 是一组不可变 snapshot 的外键。
    /// 每次定价变更必须 bump 版本；**运行中的 Session 锁定首轮快照**（参见 §2.1.1 规则 R-P1）。
    /// 对齐 HER-020（Hermes 在 SQLite 维护版本化 pricing 表）。
    pub pricing_id: PricingId,
    pub pricing_version: u32,

    pub currency: Currency,
    pub input_per_million: Decimal,
    pub output_per_million: Decimal,
    pub cache_creation_per_million: Option<Decimal>,
    pub cache_read_per_million: Option<Decimal>,
    pub image_per_image: Option<Decimal>,
    pub last_updated: DateTime<Utc>,
    pub source: PricingSource,

    /// 计费模式。用于区分标准计费、带缓存折扣、批量折扣等场景；
    /// CostCalculator 根据模式选择对应公式。
    pub billing_mode: BillingMode,
}

/// `PricingId` 是强类型字符串，用于唯一标识一条定价记录（跨 Provider / 跨版本都稳定）。
/// 建议格式：`"<provider_id>:<model_id>"`（如 `"anthropic:claude-sonnet-4.5"`）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PricingId(pub String);

/// `PricingSnapshotId` 绑定 `(pricing_id, pricing_version)`，写入事件/Usage/Cost 记录，
/// 用于回放时**精确**选对定价表。Journal 与 UsageAccumulator 以此作为外键。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PricingSnapshotId {
    pub pricing_id: PricingId,
    pub version: u32,
}

pub enum Currency {
    Usd,
    Cny,
    Eur,
    Custom(String),
}

pub enum PricingSource {
    Hardcoded,
    ProviderApi,
    ManualOverride,
    BusinessProvided,
}

pub enum BillingMode {
    /// 按 token × per_million 线性计算
    Standard,
    /// 缓存读写单独折扣（Anthropic prompt-cache：cache_read 通常 10% 原价）
    Cached { cache_read_discount: Ratio },
    /// 批处理折扣（OpenAI batch API：通常 50% 原价）
    Batched { discount: Ratio },
    /// 阶梯计费（Gemini 长上下文切档）
    Tiered { thresholds: Vec<(u64, Decimal)> },
}

pub struct Ratio(pub f32); // 0.0 ~ 1.0
```

- `pricing: Option<...>`：本地 Provider / Mock / 业务自建 Provider 可为 `None`
- `last_updated`：业务层可据此判断 pricing 是否过期需要刷新
- `PricingSource::BusinessProvided`：业务层通过 `ModelCatalog::set_pricing(model_ref, pricing)` 注入，覆盖内置值

**规则 R-P1 · Pricing Snapshot 锁定**：

- Session 首轮 `ModelProvider::infer` 之后，`ModelCatalog` 会在内部维护 `session_id → PricingSnapshotId` 映射，当轮及后续 usage 的计费一律用**首次**快照。
- 定价热更新（`ModelCatalog::set_pricing`）仅影响**之后**新建的 Session；不回灌到已在跑的 Session。
- 事件 `AssistantMessageCompleted` / `UsageAccumulated` 必须附带 `pricing_snapshot_id: Option<PricingSnapshotId>`（Pricing 缺失时为 `None`）；`ReplayEngine` 按此字段选择定价版本。

### 2.1.2 `CostCalculator` trait

定义在 `harness-observability`（L3，因需配合 `UsageAccumulator`），但 `ModelPricing` 是 L1 契约；两者解耦如下：

```rust
// in harness-observability
#[async_trait]
pub trait CostCalculator: Send + Sync + 'static {
    fn calculator_id(&self) -> &str;

    /// 标准入口：按 `pricing_snapshot_id` 查定价表，再与 usage 做线性/折扣运算。
    /// `pricing_snapshot_id = None` 表示 Pricing 不可用（本地 Provider / Mock），
    /// 实现必须返回 `None`（**不允许回退到"默认 model_ref 定价"**，避免跨 Session 漂移）。
    fn compute(
        &self,
        model_ref: &ModelRef,
        pricing_snapshot_id: Option<&PricingSnapshotId>,
        usage: &UsageSnapshot,
    ) -> Option<Cost>;
}

pub struct Cost {
    pub cents: u64,              // 精度：整分为 u64；亚分精度走 micro_cents
    pub micro_cents: u64,        // 次级精度（如 2.3 美分 = 230 micro_cents on top of 2 cents）
    pub currency: Currency,
    pub breakdown: CostBreakdown,
    /// 回放所需：本次 Cost 基于哪一条定价快照计算得到
    pub pricing_snapshot_id: Option<PricingSnapshotId>,
}

pub struct CostBreakdown {
    pub input: Option<u64>,
    pub output: Option<u64>,
    pub cache_creation: Option<u64>,
    pub cache_read: Option<u64>,
    pub image: Option<u64>,
}
```

**内置实现**：`PricingTableCostCalculator`（通过 `ModelCatalog::resolve_pricing_snapshot(pricing_snapshot_id)` 查表，按 `BillingMode` 分支计算；未知 snapshot 直接返回 `None` 并记录 `pricing_snapshot_miss_total` 指标）。

### 2.2 请求/响应

```rust
pub struct ModelRequest {
    pub model_id: String,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<ToolDescriptor>>,
    pub system: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub cache_breakpoints: Vec<CacheBreakpoint>,
    pub api_mode: ApiMode,
    pub extra: serde_json::Value,
}

pub enum ApiMode {
    ChatCompletions,
    Responses,       // OpenAI Responses API
    Messages,        // Anthropic Messages API
    GenerateContent, // Gemini
}

pub enum ModelStreamEvent {
    MessageStart { message_id: String, usage: UsageSnapshot },
    ContentBlockStart { index: u32, content_type: ContentType },
    ContentBlockDelta { index: u32, delta: ContentDelta },
    ContentBlockStop { index: u32 },
    MessageDelta { stop_reason: Option<StopReason>, usage_delta: UsageSnapshot },
    MessageStop,
    /// 流内错误。`class` 指示下游应如何反应（重试 / 降级 / 直接失败）。
    /// 替换旧 `Error(ModelError)` 变体，禁止再用 ad-hoc 字符串嗅探错误类型。
    StreamError {
        error: ModelError,
        class: ErrorClass,
        /// Provider 可以携带 `Retry-After` / `x-ratelimit-*` 等原始头，
        /// 供 Middleware（§2.6）观测；Engine 不直接解析。
        hints: ErrorHints,
    },
}

/// 错误语义分级。与 `ModelError` 正交：`ModelError` 说"出了什么"，`ErrorClass` 说"下游怎么办"。
pub enum ErrorClass {
    /// 网络抖动 / 5xx / stream 中断，允许按 `RetryPolicy` 重试
    Transient,
    /// 被限流；附带 `retry_after` 时 Middleware 可以据此冷却整个 CredentialKey
    RateLimited { retry_after: Option<Duration> },
    /// 请求 token 超过 context window；必须走 `harness-context` 的 Compact 管线
    ContextOverflow,
    /// OAuth / API Key 过期或无效；需要触发 CredentialSource::rotate
    AuthExpired,
    /// 不可恢复错误，停止重试（参数校验失败、模型下线等）
    Fatal,
}

pub struct ErrorHints {
    pub raw_headers: Option<http::HeaderMap>,
    pub provider_error_code: Option<String>,
    pub request_id: Option<String>,
}

pub enum ContentDelta {
    Text(String),
    /// 结构化 Thinking 块。与旧 `ThinkingText(String)` 变体的区别：
    /// - 保留 provider-native 原始结构（Anthropic `thinking` block 的 `signature`、
    ///   OpenAI Responses `reasoning` item 的 `encrypted_content` 等）
    /// - 业务层做 Replay 时可原样回灌，避免缓存错位与 "thinking not persisted" 问题
    /// - 对齐 HER-004（Hermes 将 provider-native reasoning 字段完整入库）
    Thinking(ThinkingDelta),
    ToolUseInputJson(String),  // 部分 JSON fragment
    ToolUseComplete { id: ToolUseId, name: String, input: Value },
}

/// Thinking / Reasoning 增量。`text` 用于展示与审计，`provider_native` 是
/// **唯一会回传给同一 provider** 的字段（用于下一轮 cache-safe 回灌）。
pub struct ThinkingDelta {
    /// 人类可读文本（可选：部分 provider 只给 encrypted 内容）
    pub text: Option<String>,
    /// Provider 原生结构（Anthropic `thinking` 块 / OpenAI reasoning item / Gemini thought 等）。
    /// 跨 provider 不可互换，必须和 `ModelDescriptor::provider_id` 一起持久化。
    pub provider_native: Option<serde_json::Value>,
    /// Anthropic `signature` 字段；OpenAI 为空。
    pub signature: Option<String>,
}
```

> **兼容迁移**：`AssistantDeltaProduced::DeltaChunk::Thought(String)`（`event-schema.md` §3.3）会升级为 `Thought(ThinkingDelta)`；历史事件通过 `From<String>` for ThinkingDelta 兜底反序列化。详见 `event-schema.md`。

### 2.2.1 Stream 聚合规则（`StreamAggregator`）

不同 Provider 对 tool_call JSON 的切分粒度差异巨大（OpenAI 按字符切；Anthropic 按整段 `input_json_delta`；Gemini 一次性给整块）。**Provider 不得直接把 partial JSON 提交给 Tool**；聚合由 SDK 侧的 `StreamAggregator` 负责：

```rust
pub struct StreamAggregator {
    /// 按 ContentBlock index 维护的聚合缓冲
    by_index: HashMap<u32, BlockBuffer>,
}

impl StreamAggregator {
    pub fn feed(&mut self, evt: ModelStreamEvent) -> Option<AggregatedEvent>;
    pub fn finish(&mut self) -> Vec<AggregatedEvent>;
}

pub enum AggregatedEvent {
    AssistantTextChunk(String),
    ThinkingChunk(ThinkingDelta),
    ToolCallReady { id: ToolUseId, name: String, input: serde_json::Value },
    MessageDone { stop_reason: StopReason, usage: UsageSnapshot },
    StreamError { error: ModelError, class: ErrorClass },
}
```

**契约**（对齐 HER-049）：

1. `ContentBlockStart { content_type: ToolUse }` → 打开一个 ToolCall 缓冲
2. `ContentBlockDelta { ToolUseInputJson(s) }` → 追加到缓冲，不立即触发
3. `ContentBlockStop { index }` → 尝试 `serde_json::from_str` 解析整块；失败则作为 `StreamError { class: Fatal }` 上抛
4. 只有成功解析后才产出 `AggregatedEvent::ToolCallReady`，供 Engine 走权限 / 执行流程

违反此契约（Provider 自己半成品 JSON 冒头）属于反模式（见 §13）。

### 2.3 Prompt Cache Style

```rust
pub enum PromptCacheStyle {
    Anthropic { mode: AnthropicCacheMode },
    OpenAi { auto: bool },
    /// Gemini Context Caching：与 Anthropic 的 inline `cache_control` 不同，
    /// Gemini 需要**提前创建** cached content 资源（REST: `cachedContents.create`）。
    /// `min_tokens` 低于此值不建议启用（官方建议 ≥ 32K tokens 才划算）。
    Gemini { mode: GeminiCacheMode },
    None,
}

pub enum AnthropicCacheMode {
    None,
    SystemAnd3,
    Custom(Vec<CacheBreakpoint>),
}

pub enum GeminiCacheMode {
    None,
    /// 显式创建 cached content，附带 TTL 与最小 tokens 门槛
    Explicit {
        ttl: Duration,       // 默认 5 分钟；Gemini 支持最长 1 小时
        min_tokens: u64,     // 低于此值直接不走 cache
    },
}
```

对齐 HER-027。

### 2.4 Credential Pool

```rust
pub struct CredentialPool {
    strategy: PoolStrategy,
    sources: Vec<Arc<dyn CredentialSource>>,
    cooldown: Duration,
    ban_list: parking_lot::RwLock<HashMap<CredentialKey, Instant>>,
}

pub enum PoolStrategy {
    FillFirst,
    RoundRobin,
    Random,
    LeastUsed,
}

#[async_trait]
pub trait CredentialSource: Send + Sync + 'static {
    async fn fetch(&self, key: CredentialKey) -> Result<CredentialValue, CredentialError>;
    async fn rotate(&self, key: CredentialKey) -> Result<(), CredentialError>;
}

/// 凭证键：**必须带 `tenant_id`**，避免多租户场景下同一 `provider_id`
/// 的不同租户共享冷却状态 / ban 表（Hermes 早期版本有此问题，HER-048 修复后迁移至此模型）。
///
/// **多租户安全契约**（强约束）：
/// - `tenant_id` 字段类型为 `TenantId`，**没有默认值**——业务方构造 `CredentialKey` 时必须显式提供。
/// - 单租户场景下使用 `TenantId::SINGLE`（`Default::default()`）。
/// - 想要**跨租户共享**同一把 key（如内部测试账号、企业级共享 quota）必须显式使用 `TenantId::SHARED`
///   哨兵，**不得**用任意 `TenantId::new()` 当作"全局"占位符。SDK 在凭证池首次解析到 `SHARED`
///   时必记一条 `Event::CredentialPoolSharedAcrossTenants`（写入 `security-trust §8.1`
///   必记事件清单），保证审计可见。
/// - `Default` impl 与所有 builder 默认值**不得**返回 `TenantId::SHARED`；CI 静态检查拦截。
/// - 任何缺失 `tenant_id` 的旧版 schema 在迁移层（`harness_contracts::migrate`）必须强制升版补值，
///   不得回退为 `SHARED`，必须由业务方在迁移钩子中显式选择 `SINGLE` 或具体租户。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CredentialKey {
    pub tenant_id: TenantId,
    pub provider_id: String,
    /// 同一租户下区分多把 key（如"主账号 / 备份账号"）的标签
    pub key_label: String,
}

pub struct CredentialValue {
    pub secret: secrecy::SecretString,
    pub metadata: CredentialMetadata,
}
```

**规则**（对齐 HER-048）：

- 遇 `429 / 402` 错误，默认 `cooldown = 1 hour`
- 命中 `ban_list` 时切到下一 key
- 所有 key 都 ban，则抛 `ModelError::AllCredentialsBanned`
- **ban_list / cooldown 全部按 `CredentialKey` 整键分桶**，跨租户不共享

### 2.5 Token Counter

```rust
pub trait TokenCounter: Send + Sync + 'static {
    fn count_tokens(&self, text: &str, model: &str) -> usize;
    fn count_messages(&self, messages: &[Message], model: &str) -> usize;

    /// 估算图像 token。未知 model / 未加载视觉规则 → `None`
    /// （调用方自行决定是否兜底为固定值，避免静默返回错误估算）。
    /// - Anthropic: `(width * height) / 750`（官方经验公式）
    /// - OpenAI 4o: 依 low/high 切分
    /// - Gemini: 按 tile 数量
    fn count_image(&self, image: &ImageMeta, model: &str) -> Option<usize> { None }
}

pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
    pub mime: String,
    /// 若 provider 支持 detail level（OpenAI），由 Context Stage 决定
    pub detail: ImageDetail,
}

pub enum ImageDetail { Low, High, Auto }
```

内置 `TiktokenCounter`（OpenAI 系）、`AnthropicCounter`、`ApproximateCounter`（兜底）。

### 2.6 `InferMiddleware`（横切扩展点）

Middleware 是 **Provider 与 Engine 之间**的横切通道，用于实现：

- OAuth token 过期自动刷新（观察 `ErrorClass::AuthExpired`）
- RateLimit 头观测（把 `x-ratelimit-remaining-requests` 暴露为 Prometheus metric）
- 请求/响应 header 级的审计落盘
- A/B / canary 路由（`before_request` 里改写 `model_id`）

```rust
#[async_trait]
pub trait InferMiddleware: Send + Sync + 'static {
    fn middleware_id(&self) -> &str;

    /// 请求即将发往 Provider 前；可改写 `ModelRequest` 或注入 `ctx.tracing`
    async fn before_request(
        &self,
        req: &mut ModelRequest,
        ctx: &mut InferContext,
    ) -> Result<(), ModelError> { Ok(()) }

    /// 收到 HTTP/gRPC 响应头之后、开始读 body 前（仅对 HTTP-based Provider 有意义）
    async fn on_response_headers(
        &self,
        headers: &http::HeaderMap,
        ctx: &InferContext,
    ) -> Result<(), ModelError> { Ok(()) }

    /// 包裹 stream；最常用于注入观测 metric 与 redact 敏感内容
    fn wrap_stream(
        &self,
        stream: BoxStream<ModelStreamEvent>,
        ctx: &InferContext,
    ) -> BoxStream<ModelStreamEvent> { stream }

    /// 最后一次 usage 聚合结束（包括重试在内的最终 usage）
    async fn on_request_end(
        &self,
        usage: &UsageSnapshot,
        ctx: &InferContext,
    ) -> Result<(), ModelError> { Ok(()) }
}
```

**内置 Middleware**（SDK 提供，业务按需启用）：

| `middleware_id` | 职责 | Feature flag |
|---|---|---|
| `rate-limit-observer` | 解析 `x-ratelimit-*` / `Retry-After`，推送到 Observer；观察到阈值触发 cooldown | `rate-limit-observer`（默认启） |
| `oauth-auto-refresh` | 检测 `ErrorClass::AuthExpired` → 调用 `CredentialSource::rotate` | `oauth`（默认关） |
| `redact-stream` | 在 `wrap_stream` 里对 `ContentDelta::Text` 跑 `Redactor` | `redactor` |
| `trace-span` | 自动创建 `harness.model.infer` span（对齐 `harness-observability` §5） | `otel` |

**执行顺序**：`before_request` 按 Vec 顺序、`wrap_stream` **反序**包裹（最后注册的最先感知 stream）。这个惯例与 Tower / Axum middleware 一致，避免业务层心智负担。

**反模式**：Middleware 不得改写 `messages` 内容（会破坏 Prompt Cache，违反 P5）；改写请用 `PreLlmCall` hook（在 Engine 层）。

## 3. 内置实现

```rust
#[cfg(feature = "openai")]
pub struct OpenAiProvider { /* ... */ }

#[cfg(feature = "anthropic")]
pub struct AnthropicProvider { /* ... */ }

#[cfg(feature = "gemini")]
pub struct GeminiProvider { /* ... */ }

#[cfg(feature = "openrouter")]
pub struct OpenRouterProvider { /* ... */ }

#[cfg(feature = "bedrock")]
pub struct BedrockProvider { /* ... */ }

#[cfg(feature = "codex")]
pub struct CodexResponsesProvider { /* ... */ }

#[cfg(feature = "local-llama")]
pub struct LocalLlamaProvider { /* ... */ }

#[cfg(feature = "mock")]
pub struct MockModelProvider { /* ... */ }

/// **Cassette**：包装一个真实 Provider，按 `mode` 决定录/播行为。
/// 典型用法：CI 跑 E2E 时启用 `Replay`，开发时 `Record`，失败 case 归档为
/// `tests/cassettes/<test-name>.json` 并锁定。对齐 HER-049（测试夹具）与 OC-22。
#[cfg(feature = "cassette")]
pub struct CassetteProvider {
    inner: Arc<dyn ModelProvider>,
    cassette: PathBuf,
    mode: CassetteMode,
}

#[cfg(feature = "cassette")]
pub enum CassetteMode {
    /// 调用真实 inner + 把 `(ModelRequest → Vec<ModelStreamEvent>)` 落盘
    Record,
    /// 只从 cassette 回放；未命中则 `ModelError::ProviderUnavailable("cassette miss")`
    Replay,
    /// 优先 replay，未命中则穿透到 inner 并记录（开发期友好，CI 禁用）
    Passthrough,
}
```

对齐 HER-048 / HER-049。

## 4. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("context too long: tokens={tokens}, max={max}")]
    ContextTooLong { tokens: usize, max: usize },

    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("all credentials banned")]
    AllCredentialsBanned,

    #[error("aux model not configured")]
    AuxModelNotConfigured,

    #[error("auth expired: {0}")]
    AuthExpired(String),

    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),

    #[error("cancelled by caller")]
    Cancelled,

    #[error("deadline exceeded after {0:?}")]
    DeadlineExceeded(Duration),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// `ModelError` → `ErrorClass` 的**唯一**映射表。Provider 实现一律调用 `classify`。
impl ModelError {
    pub fn classify(&self) -> ErrorClass {
        match self {
            ModelError::RateLimited(_) => ErrorClass::RateLimited { retry_after: None },
            ModelError::ContextTooLong { .. } => ErrorClass::ContextOverflow,
            ModelError::AuthExpired(_) => ErrorClass::AuthExpired,
            ModelError::Cancelled | ModelError::DeadlineExceeded(_) => ErrorClass::Fatal,
            ModelError::ProviderUnavailable(_) | ModelError::Io(_) => ErrorClass::Transient,
            ModelError::InvalidRequest(_)
            | ModelError::AllCredentialsBanned
            | ModelError::AuxModelNotConfigured
            | ModelError::UnexpectedResponse(_) => ErrorClass::Fatal,
        }
    }
}
```

## 5. Model Catalog

```rust
pub struct ModelCatalog {
    primary: Arc<dyn ModelProvider>,
    /// 同 primary `ModelRef` 对应的 fallback 链；调用 `primary` 失败（按 `ErrorClass` 判定）
    /// 时顺序尝试，直到成功或全部 fail。
    fallbacks: Vec<Arc<dyn ModelProvider>>,
    aux: Option<Arc<dyn AuxModelProvider>>,   // 类型升级，见 §5.1
    providers: Vec<Arc<dyn ModelProvider>>,
    alias_map: HashMap<String, ModelRef>,
    pricing_snapshots: Arc<RwLock<HashMap<PricingId, HashMap<u32, Arc<ModelPricing>>>>>,
    session_pricing_lock: Arc<RwLock<HashMap<SessionId, PricingSnapshotId>>>,
}

impl ModelCatalog {
    pub fn new(primary: impl ModelProvider) -> Self;
    pub fn with_aux_model(mut self, aux: impl AuxModelProvider) -> Self;
    pub fn with_provider(mut self, provider: impl ModelProvider) -> Self;
    pub fn with_alias(mut self, alias: &str, target: ModelRef) -> Self;
    pub fn resolve(&self, model_id: &str) -> Option<Arc<dyn ModelProvider>>;

    /// 返回辅助 LLM Provider（Microcompact / Autocompact / AuxLlmBroker 使用）
    pub fn aux(&self) -> Option<Arc<dyn AuxModelProvider>>;

    /// 返回主 Provider（Agent 主循环使用）
    pub fn primary(&self) -> Arc<dyn ModelProvider>;

    /// **B1 · Fallback 链**。仅在 `ErrorClass::{Transient, RateLimited}` 时切换到下一个
    /// Provider；`ContextOverflow` 不走 fallback（上游 Compact 管线负责）。
    pub fn with_fallback(
        mut self,
        primary: ModelRef,
        fallbacks: Vec<ModelRef>,
    ) -> Self;

    /// 按 (pricing_id, version) 查询一条不可变的定价快照；Replay / CostCalculator 使用
    pub fn resolve_pricing_snapshot(
        &self,
        id: &PricingSnapshotId,
    ) -> Option<Arc<ModelPricing>>;

    /// Session 首轮结束后调用，把当前 pricing 锁定到 session（对齐 R-P1）
    pub fn lock_pricing_for_session(
        &self,
        session: SessionId,
        snapshot: PricingSnapshotId,
    );
}

pub struct ModelRef {
    pub provider_id: String,
    pub model_id: String,
}
```

### 5.1 `AuxModelProvider`（Aux 独立 Trait）

Aux 调用的约束集合与主对话**不同**（更严格的超时、允许 fail-open、独立并发上限），用独立 trait 承载、避免把这些约束泄漏到 `ModelProvider`：

```rust
#[async_trait]
pub trait AuxModelProvider: Send + Sync + 'static {
    /// 底层走哪个 ModelProvider；Middleware / 凭证池复用主逻辑
    fn inner(&self) -> Arc<dyn ModelProvider>;

    fn aux_options(&self) -> AuxOptions;

    /// 高阶调用：调用方告诉 Aux "这是什么任务"，实现可以据此挑 system prompt / cache 断点
    async fn call_aux(
        &self,
        task: AuxTask,
        req: ModelRequest,
    ) -> Result<String, ModelError>;
}

pub enum AuxTask {
    /// 上下文压缩（`harness-context::MicrocompactProvider` / `AutocompactProvider`）
    Compact,
    /// 短摘要（Subagent 返回结果摘要、Team routing summary）
    Summarize,
    /// 分类（MessageClassifier 路由决策）
    Classify,
    /// 权限建议（`harness-permission::AuxLlmBroker`）
    PermissionAdvisory,
}

pub struct AuxOptions {
    /// 最大并发；超过即排队（默认 4）
    pub max_concurrency: usize,
    /// 单次调用超时（默认 30s，比主对话 short）
    pub per_task_timeout: Duration,
    /// Aux 调用失败是否允许上游继续：
    /// - Compact / Summarize / Classify → `true`（业务层应有 fallback）
    /// - PermissionAdvisory → `false`（安全相关，必须上抛）
    pub fail_open: bool,
}
```

- **定位**：Aux Model 是一个"更便宜 / 更快"的辅助 Provider，用于 **所有非主对话** 的推理：
  - `harness-context::MicrocompactProvider` / `AutocompactProvider` 做上下文摘要
  - `harness-permission::AuxLlmBroker`（feature `auto-mode`）做 `APPROVE/DENY/ESCALATE` 判定
  - `harness-team::MessageClassifier` 做语义路由
- **单例**：每个 `Harness` 至多一个 Aux Model（不需要 slot 约束，但只读取一个）。
- **回退**：若未显式 `with_aux_model`，上述消费者各自 fallback：
  - Compact Provider → 用 `primary` Model（成本稍高但功能可用；需业务显式同意，默认报错）
  - AuxLlmBroker → 报 `ModelError::AuxModelNotConfigured`（显式错误，不静默 fallback 到 primary，避免无意消耗高配额）
  - MessageClassifier → 退化为关键词/正则路由

> **设计取舍**：早期方案让 Aux 直接复用 `Arc<dyn ModelProvider>`，但 Hermes 的实战经验（HER-025）表明三类 Aux 任务的超时 / 失败策略差异显著，混用会出现 Compact 被正常重试消耗 30% 配额的事故。因此 Octopus 选择**独立 trait + `AuxOptions`**。

### 5.2 `ProviderHealthCheck`（可选 · 生产环境）

```rust
#[async_trait]
pub trait ProviderHealthCheck: Send + Sync + 'static {
    /// 轻量探活。实现可以调 `list_models` / `ping` 端点，也可以返回缓存的 last-seen 状态。
    async fn probe(&self) -> HealthStatus;
}

pub enum HealthStatus {
    Healthy,
    Degraded { reason: String },
    Unhealthy { reason: String },
}
```

- `ModelProvider::health()` 默认返回 `Healthy`；需要主动探活的 Provider 额外实现 `ProviderHealthCheck`。
- 业务层通过 `ModelCatalog::register_health_check(provider_id, Arc<dyn ProviderHealthCheck>)` 注入，由 Observer 周期轮询（默认 60s）并导出 `provider_health_status{provider}`。
- Degraded 状态下 `with_fallback` 会把该 Provider 排到链尾；Unhealthy 直接跳过。

## 6. API Mode 路由

不同 Provider 有不同的 API 端点：

```rust
impl AnthropicProvider {
    pub fn with_api_mode(mut self, mode: ApiMode) -> Self {
        assert!(matches!(mode, ApiMode::Messages));
        self
    }
}
```

对齐 HER-049：Anthropic / Bedrock / Codex 通过独立适配器模块把 provider API 归一到 OpenAI-style chat，但**对外保留原生 ApiMode** 供业务按需启用。

## 7. 认证支持（对齐 HER-049）

Anthropic 支持三种认证：

- `x-api-key`（普通 API）
- `Bearer OAuth`（Anthropic-beta）
- Claude Code credentials（读 `~/.claude/credentials.json`）

```rust
pub enum AnthropicAuth {
    ApiKey(SecretString),
    Bearer(SecretString),
    ClaudeCodeCredentials(PathBuf),
}
```

## 8. 与 Credential Source 的配合

```rust
// 业务层实现
struct VaultCredentialSource { /* ... */ }

#[async_trait]
impl CredentialSource for VaultCredentialSource {
    async fn fetch(&self, key: CredentialKey) -> Result<CredentialValue> {
        // 从 HashiCorp Vault / AWS Secrets Manager / 1Password 拉取
    }
    async fn rotate(&self, key: CredentialKey) -> Result<()> { /* ... */ }
}

// SDK 注入
let pool = CredentialPool::builder()
    .strategy(PoolStrategy::RoundRobin)
    .add_source(VaultCredentialSource::new())
    .cooldown(Duration::from_secs(3600))
    .build();

let provider = AnthropicProvider::with_credential_pool(pool);
```

## 9. Feature Flags

```toml
[features]
default = ["rate-limit-observer"]
openai = ["dep:async-openai"]
anthropic = ["dep:anthropic-sdk"]
gemini = ["dep:google-generative-ai"]
openrouter = ["openai"]   # 复用 OpenAI 协议
bedrock = ["dep:aws-sdk-bedrockruntime"]
codex = ["openai"]
local-llama = ["dep:llama-cpp-bindings"]
mock = []
cassette = ["dep:serde_json"]
rate-limit-observer = []
oauth = ["dep:oauth2"]
redactor = ["dep:octopus-harness-observability"]
otel = ["dep:opentelemetry"]
```

## 10. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | 消息归一化、credential pool cooldown/ban 逻辑、token counter、`ModelError::classify` 全分支 |
| 单元 | `StreamAggregator` 在三种 partial JSON 模式下行为正确（OpenAI char-level / Anthropic delta / Gemini whole） |
| 单元 | `ModelPricing` 同 `pricing_id` 不同 `version` 计算结果差异；`Cached` / `Tiered` / `Batched` 三种 `BillingMode` |
| 集成 | 每个 provider 的端到端 infer（使用 `CassetteProvider` recorded cassette） |
| 集成 | `AuxModelProvider` 三类 `AuxTask` 的超时 / 并发 / fail-open 行为 |
| 集成 | `B1 Fallback`：primary 抛 `ErrorClass::Transient` → 自动切换；抛 `Fatal` → 不切换 |
| Mock | `MockModelProvider` 支持预设流式序列（含 `StreamError + ErrorClass`） |
| 契约 | Response 必须能回转为 `harness-contracts::MessagePart`（含 `Thinking(ThinkingBlock)`） |
| 契约 | `Middleware::wrap_stream` 不得改写 `messages`（启动时静态校验 Vec 顺序） |

## 11. 使用示例

```rust
use octopus_harness_model::anthropic::AnthropicProvider;

let provider = AnthropicProvider::builder()
    .api_key(env::var("ANTHROPIC_API_KEY")?)
    .prompt_cache_mode(AnthropicCacheMode::SystemAnd3)
    .build()?;

let req = ModelRequest {
    model_id: "claude-sonnet-4.5".into(),
    system: Some("You are a helpful assistant.".into()),
    messages: vec![/* ... */],
    tools: None,
    cache_breakpoints: vec![/* ... */],
    api_mode: ApiMode::Messages,
    // ...
};

let ctx = InferContext {
    request_id: RequestId::new(),
    tenant_id: tenant,
    session_id: Some(session_id),
    run_id: Some(run_id),
    cancel: cancel_token.clone(),
    deadline: Some(Instant::now() + Duration::from_secs(120)),
    retry_policy: RetryPolicy::default(),
    tracing: tracer.current_context(),
    middlewares: vec![
        Arc::new(RateLimitObserverMiddleware::new(observer.clone())),
        Arc::new(TraceSpanMiddleware::new(tracer.clone())),
    ],
};

let mut stream = provider.infer(req, ctx).await?;
let mut agg = StreamAggregator::default();
while let Some(event) = stream.next().await {
    if let Some(out) = agg.feed(event?) {
        match out {
            AggregatedEvent::AssistantTextChunk(s) => { /* render */ }
            AggregatedEvent::ThinkingChunk(t) => { /* persist t.provider_native */ }
            AggregatedEvent::ToolCallReady { id, name, input } => { /* dispatch */ }
            AggregatedEvent::MessageDone { stop_reason, usage } => break,
            AggregatedEvent::StreamError { error, class } => {
                if matches!(class, ErrorClass::Transient | ErrorClass::RateLimited { .. }) {
                    /* let RetryPolicy / Catalog::with_fallback handle it */
                }
                return Err(error.into());
            }
        }
    }
}
```

## 12. 可观测性

| 指标 | 说明 |
|---|---|
| `model_infer_duration_ms` | 端到端推理耗时（含重试） |
| `model_tokens_input` / `tokens_output` | Token 使用 |
| `model_cache_creation_tokens` / `cache_read_tokens` | Anthropic 缓存指标 |
| `credential_pool_cooldowns_total` | cooldown 次数，按 `tenant_id / provider_id` 分桶 |
| `model_errors_total{class=...}` | 按 `ErrorClass` 分桶 |
| `model_stream_error_total{class=...}` | 流内错误；用于判定 fallback 触发率 |
| `model_fallback_invocations_total{from_provider, to_provider}` | B1 fallback 命中 |
| `provider_health_status{provider}` | `ProviderHealthCheck` 探活结果 |
| `pricing_snapshot_miss_total` | Cost 计算时 snapshot 不命中数 |
| `aux_model_queue_wait_ms{task}` | Aux 并发排队时延 |

## 13. 反模式

- 直接把 API Key 字符串存到 `provider.api_key = "sk-..."` 字段（应用 `SecretString` + `CredentialSource`）
- 在 `infer` 实现里缓存历史响应（违反 Provider 无状态原则）
- 硬编码 Retry 策略（应通过 `InferContext::retry_policy` 可配）
- Provider 直接把半成品 tool_call JSON 抛给 Engine（必须经 `StreamAggregator`，§2.2.1）
- 在 `Middleware::before_request` 改 `messages` 内容（破坏 Prompt Cache，违反 ADR-003 / P5）
- 用 `String` 在错误信息里 sniff 错误类型；应改 `ModelError::classify()`
- 直接复用 `ModelProvider` 跑 Compact 等 Aux 任务（应走 `AuxModelProvider`，避免 timeout / 并发策略错配）
- 跨租户共享 `CredentialKey`（必须含 `tenant_id`，否则 ban_list 会污染）
- 在事件里只记 `(model_id, tokens)` 而不带 `pricing_snapshot_id`（替换定价后无法重算成本）

## 14. 相关

- D3 · `api-contracts.md` §2 模型接入
- D8 · `context-engineering.md` §5 Prompt Cache
- D9 · `security-trust.md` §6 凭证管理
- ADR-003 Prompt Cache 硬约束
- ADR-009 Deferred Tool Loading（`ModelCapabilities.supports_tool_reference` 驱动 backend 选择）
- `crates/harness-tool-search.md` §3.5 `BackendSelector`
- `crates/harness-context.md` §2.1 / §3.2（`AuxModelProvider` 注入点）
- `crates/harness-observability.md` §2.3.1（`CostCalculator` 的 `pricing_snapshot_id` 契约）
- `event-schema.md` §3.3 / §3.4（`AssistantDeltaProduced.Thought` / `AssistantMessageCompleted.pricing_snapshot_id`）
- 参考分析：HER-004（Thinking 持久化）/ HER-020（Pricing 版本化）/ HER-025（Aux 独立约束）/ HER-048（CredentialPool）/ HER-049（Provider 适配器）/ CC-06（Compact 管线）/ OC-22（Cassette 测试）
