# `octopus-harness-observability` · L3 · Tracer + Usage + Replay + Redactor SPEC

> 层级：L3 · 状态：Accepted
> 依赖：`harness-contracts` + `harness-journal`（只读）

## 1. 职责

提供 **观测性套件**：Tracer（OpenTelemetry 兼容）+ UsageAccumulator + Replay Engine + Redactor。对齐 ADR-001 Replay。

**核心能力**：

- Tracer：Span 采集，OTLP / Jaeger / Zipkin 导出
- Usage：token / 成本 / 工具次数聚合
- Replay：从 Event Journal 重建任意时点的 Projection
- Diff：对比两次 Run 的差异
- Redactor：敏感信息脱敏

## 2. 对外 API

### 2.1 Observer（聚合入口）

```rust
pub struct Observer {
    pub tracer: Arc<dyn Tracer>,
    pub usage: Arc<UsageAccumulator>,
    pub redactor: Arc<Redactor>,
    pub replay: Option<Arc<ReplayEngine>>,
}

impl Observer {
    pub fn builder() -> ObserverBuilder;
}

pub struct ObserverBuilder {
    // ...
}

impl ObserverBuilder {
    pub fn with_otel_endpoint(self, endpoint: impl AsRef<str>) -> Self;
    pub fn with_service_name(self, name: impl AsRef<str>) -> Self;
    pub fn with_prometheus(self, bind: SocketAddr) -> Self;
    pub fn with_replay_enabled(self, enabled: bool) -> Self;
    pub fn with_redactor(self, redactor: Redactor) -> Self;
    pub fn build(self) -> Result<Observer, ObservabilityError>;
}
```

### 2.2 Tracer Trait

```rust
pub trait Tracer: Send + Sync + 'static {
    fn start_span(&self, name: &str, attrs: SpanAttributes) -> Box<dyn Span>;
    fn inject_context(&self, carrier: &mut dyn TraceCarrier);
    fn extract_context(&self, carrier: &dyn TraceCarrier) -> Option<TraceContext>;
}

pub trait Span: Send {
    fn set_attribute(&mut self, key: &str, value: AttributeValue);
    fn add_event(&mut self, name: &str, attrs: SpanAttributes);
    fn set_status(&mut self, status: SpanStatus);
    fn end(self: Box<Self>);
}

pub struct SpanAttributes {
    pub attrs: HashMap<String, AttributeValue>,
}

pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
}
```

### 2.3 Usage Accumulator

```rust
pub struct UsageAccumulator {
    inner: Arc<RwLock<UsageState>>,
    cost_calculator: Option<Arc<dyn CostCalculator>>,
}

struct UsageState {
    by_tenant: HashMap<TenantId, TenantUsage>,
    by_session: HashMap<SessionId, SessionUsage>,
    by_model: HashMap<String, ModelUsage>,
}

pub struct UsageSnapshot {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cache_read_tokens: u64,
    /// 视觉输入 token（来自 `TokenCounter::count_image`；无视觉规则时不累计）
    pub image_tokens: u64,
    pub tool_calls: u64,
    pub cost: Option<Cost>,        // 由 CostCalculator 注入；None 表示 pricing 不可用
    /// 本次 usage 所基于的定价快照；`None` 表示 Pricing 不可用或本地 Provider
    pub pricing_snapshot_id: Option<PricingSnapshotId>,
    pub duration: Duration,
}

/// `Cost` / `Currency` 定义见 `crates/harness-model.md` §2.1.2
pub use octopus_harness_model::{
    Cost, Currency, ModelPricing, ModelRef,
    PricingId, PricingSnapshotId, BillingMode,
};

impl UsageAccumulator {
    pub fn builder() -> UsageAccumulatorBuilder;

    pub fn record(
        &self,
        scope: UsageScope,
        model_ref: Option<ModelRef>,
        delta: UsageSnapshot,
    );
    pub fn snapshot(&self, scope: UsageScope) -> UsageSnapshot;
    pub fn reset(&self, scope: UsageScope);
}

pub struct UsageAccumulatorBuilder {
    cost_calculator: Option<Arc<dyn CostCalculator>>,
}

impl UsageAccumulatorBuilder {
    pub fn with_cost_calculator(mut self, c: Arc<dyn CostCalculator>) -> Self;
    pub fn build(self) -> UsageAccumulator;
}

pub enum UsageScope {
    Global,
    Tenant(TenantId),
    Session(SessionId),
    Run(RunId),
    Model(String),
}
```

### 2.3.1 `CostCalculator` trait

```rust
#[async_trait]
pub trait CostCalculator: Send + Sync + 'static {
    fn calculator_id(&self) -> &str;

    /// 同步函数，必须快（热路径）。
    /// `pricing_snapshot_id = None` 表示无 pricing 数据；实现 **必须** 直接返回 `None`，
    /// 不允许默默回退到 `model_ref` 的"当前"定价（避免 Replay 时定价漂移）。
    fn compute(
        &self,
        model_ref: &ModelRef,
        pricing_snapshot_id: Option<&PricingSnapshotId>,
        usage: &UsageSnapshot,
    ) -> Option<Cost>;
}
```

**内置实现**：

- `PricingTableCostCalculator`：通过 `ModelCatalog::resolve_pricing_snapshot(snapshot_id)` 查表，按 `BillingMode` 分支计算（Standard / Cached / Batched / Tiered）
- `NoopCostCalculator`：始终返回 `None`；业务不关心成本时使用

**注入路径**：

1. 业务层 `HarnessBuilder` 未显式 `with_cost_calculator` → 默认 `PricingTableCostCalculator`
2. Engine 在每次主对话 / Aux 调用结束时通过 `ModelCatalog::lock_pricing_for_session` 锁定快照 ID，并把它写入 `AssistantMessageCompletedEvent.pricing_snapshot_id`、`UsageAccumulatedEvent.pricing_snapshot_id`
3. `UsageAccumulator::record` 接收带 snapshot 的 usage，调用 `compute(model_ref, snapshot_id, usage)`；命中失败 → `pricing_snapshot_miss_total += 1`
4. Prometheus / OTel 导出 `model_cost_by_currency`

**Replay 语义**：`ReplayEngine` 重放历史事件时，按事件携带的 `pricing_snapshot_id` 拿到当时的不可变定价（即便定价表在之后已被更新），保证成本可重算且与原次一致。

### 2.4 Replay Engine

```rust
pub struct ReplayEngine {
    store: Arc<dyn EventStore>,
}

impl ReplayEngine {
    pub fn new(store: Arc<dyn EventStore>) -> Self;

    pub async fn replay(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        cursor: ReplayCursor,
    ) -> Result<BoxStream<Event>, ObservabilityError>;

    pub async fn reconstruct_projection(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        at: ReplayCursor,
    ) -> Result<SessionProjection, ObservabilityError>;

    pub async fn diff(
        &self,
        tenant: TenantId,
        session_a: SessionId,
        session_b: SessionId,
    ) -> Result<SessionDiff, ObservabilityError>;

    pub async fn export_session(
        &self,
        tenant: TenantId,
        session_id: SessionId,
        format: ExportFormat,
        out: impl AsyncWrite + Unpin,
    ) -> Result<(), ObservabilityError>;
}

pub struct SessionDiff {
    pub added_messages: Vec<Message>,
    pub removed_messages: Vec<Message>,
    pub tool_divergence: Vec<ToolDivergence>,
    pub usage_delta: UsageSnapshot,
}

pub enum ExportFormat {
    Json,
    JsonLines,
    Markdown,
    Har,
}
```

### 2.5 Redactor

```rust
pub struct Redactor {
    patterns: Arc<RwLock<Vec<RedactPattern>>>,
    default_replacement: String,
}

pub struct RedactPattern {
    pub id: String,
    pub regex: Regex,
    pub replacement: String,
    pub scope: RedactScope,
}

pub enum RedactScope {
    All,
    TraceOnly,
    EventBody,
    LogOnly,
}

impl Redactor {
    pub fn default() -> Self;
    pub fn add_pattern(&self, pattern: RedactPattern);
    pub fn redact(&self, text: &str, scope: RedactScope) -> String;
    pub fn redact_value(&self, v: Value, scope: RedactScope) -> Value;
}

impl Default for Redactor {
    fn default() -> Self {
        // 默认内置：
        // - OpenAI API Key (sk-...)
        // - Anthropic API Key (sk-ant-...)
        // - JWT
        // - AWS Access Key (AKIA...)
        // - GCP service account JSON
    }
}
```

## 3. OpenTelemetry 集成

```rust
#[cfg(feature = "otel")]
pub struct OtelTracer {
    provider: Arc<opentelemetry_sdk::trace::TracerProvider>,
    tracer: Arc<opentelemetry::global::BoxedTracer>,
}

#[cfg(feature = "otel")]
impl OtelTracer {
    pub fn new(endpoint: &str, service_name: &str) -> Result<Self, ObservabilityError> {
        // 初始化 OTLP exporter
    }
}

#[cfg(feature = "otel")]
impl Tracer for OtelTracer { /* ... */ }
```

## 4. Prometheus 导出

```rust
#[cfg(feature = "prometheus")]
pub struct PrometheusExporter {
    bind_addr: SocketAddr,
    registry: prometheus::Registry,
}

#[cfg(feature = "prometheus")]
impl PrometheusExporter {
    pub async fn serve(&self) -> Result<(), ObservabilityError>;
}
```

## 5. Span 命名约定

| Span 名 | 属性（示例） |
|---|---|
| `harness.session.run` | `session.id`, `tenant.id`, `run.id`, `input.tokens`, `output.tokens` |
| `harness.model.infer` | `model.id`, `provider.id`, `api.mode`, `prompt.cache.hit_ratio` |
| `harness.tool.invoke` | `tool.name`, `tool.use.id`, `duration.ms`, `exit.code` |
| `harness.hook.dispatch` | `hook.event`, `handler.id`, `outcome` |
| `harness.permission.decide` | `request.id`, `decided.by`, `decision` |
| `harness.subagent.run` | `subagent.id`, `parent.session.id`, `depth` |
| `harness.team.message.route` | `team.id`, `from`, `to`, `routing.strategy` |
| `harness.compact.apply` | `stage`, `bytes.saved`, `ratio` |
| `harness.sandbox.exec` | `backend.id`, `command.hash`, `exit.code` |

## 6. Feature Flags

```toml
[features]
default = ["redactor"]
redactor = ["dep:regex"]
replay = []
otel = ["dep:opentelemetry", "dep:opentelemetry-otlp", "dep:opentelemetry_sdk"]
prometheus = ["dep:prometheus", "dep:axum"]
```

## 7. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum ObservabilityError {
    #[error("tracer init: {0}")]
    TracerInit(String),

    #[error("exporter: {0}")]
    Exporter(String),

    #[error("replay: {0}")]
    Replay(String),

    #[error("redact regex: {0}")]
    Regex(#[from] regex::Error),

    #[error("journal: {0}")]
    Journal(#[from] JournalError),
}
```

## 8. 使用示例

### 8.1 初始化

```rust
let observer = Observer::builder()
    .with_service_name("octopus-server")
    .with_otel_endpoint("http://otel-collector:4317")
    .with_prometheus("0.0.0.0:9100".parse()?)
    .with_replay_enabled(true)
    .with_redactor(Redactor::default())
    .build()?;

let harness = HarnessBuilder::new()
    .with_observability(Arc::new(observer))
    // ...
    .build()
    .await?;
```

### 8.2 Replay

```rust
let replay = harness.replay_engine();

let proj = replay.reconstruct_projection(
    TenantId::SINGLE,
    session_id,
    ReplayCursor::FromStart,
).await?;

println!("Session has {} messages", proj.messages.len());
```

### 8.3 Diff

```rust
let diff = replay.diff(
    TenantId::SINGLE,
    session_a,
    session_b,
).await?;

println!("Diff:");
println!("  Added {} messages", diff.added_messages.len());
println!("  Tool divergence: {:?}", diff.tool_divergence);
```

### 8.4 Redactor

```rust
let redactor = Redactor::default();
redactor.add_pattern(RedactPattern {
    id: "internal-user-id".into(),
    regex: Regex::new(r"user-\d{10}").unwrap(),
    replacement: "[USER_ID]".into(),
    scope: RedactScope::All,
});

let text = "User user-1234567890 did something";
let redacted = redactor.redact(text, RedactScope::All);
assert_eq!(redacted, "User [USER_ID] did something");
```

## 9. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | Redactor 内置模式；Usage 累加正确 |
| Replay | 给定 Event 序列，重建 Projection = 原状态 |
| Diff | 单点差异、连续差异 |
| OTel | 用 Mock Exporter 验证 span 字段 |
| 性能 | 1M Event 的 replay 性能（< 10s） |

## 10. 可观测性（观测自身）

| 指标 | 说明 |
|---|---|
| `observer_spans_exported_total` | 按 exporter 分桶 |
| `observer_export_errors_total` | |
| `observer_replay_duration_ms` | Replay 耗时 |
| `observer_redactor_hits_total` | 按 pattern 分桶 |

## 11. 反模式

- Tracer 在热路径里同步导出 Span（应 batch + async）
- 默认 Redactor 没启用就打印原始 Event body
- Replay 用于实时查询（应只用于审计/调试）
- Usage 用字符串拼接 key 而非强类型 scope

## 12. 相关

- ADR-001 Event Sourcing
- D4 · `event-schema.md` §Replay 语义
- D9 · `security-trust.md` §8 日志与审计
- `crates/harness-journal.md`
