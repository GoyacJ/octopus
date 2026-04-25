# `octopus-harness-tool-search` · L2 复合能力 · Tool Search SPEC

> 层级：L2 · 状态：Accepted
> 依赖：`harness-contracts` · `harness-tool`（Tool trait / ToolContext）· `harness-model`（ModelCapabilities）
> 逆向依赖：`harness-sdk`（L4 门面负责把 `ToolSearchTool` 注入默认工具集）
>
> 注：`harness-tool-search` **不**被 `harness-tool` 反向依赖。L2 之间的 `ToolSearchTool` 注入走 L4 门面，保持 L2 层之间相互独立（ADR-008 分层原则）。

## 1. 职责

实现 ADR-009 所定义的 **Deferred Tool Loading** 运行时：

- `ToolSearchTool` 元工具：接受模型的 `select:` / 关键字 query，返回 `tool_reference` 或触发 inline 重注入
- `ToolLoadingBackend` trait + 两个内置实现（`AnthropicToolReferenceBackend` / `InlineReinjectionBackend`）
- `ToolSearchScorer` trait + `DefaultScorer`（照搬 Claude Code 经 A/B 验证的权重）
- `DeferredToolsDelta` 的拼装与 attachment 编码
- 阈值判据：`ToolSearchMode::Auto { ratio, min_absolute_tokens }` 的 token/char 估算
- `DiscoveredToolProjection`（Session 已材化工具集的投影器，供 `harness-session` 注册）

**不包含**：

- Tool trait 自身 / ToolRegistry / ToolPool → 属 `harness-tool`
- `SessionOptions.tool_search` 字段与 `reload_with` API → 属 `harness-session`
- MCP `tools/list_changed` 的连接层处理 → 属 `harness-mcp`
- UI 展示 → ADR-002 禁止

## 2. 对外 API

### 2.1 `ToolSearchMode`（来自 `harness-contracts`，此处给出 Session 层默认）

```rust
pub use harness_contracts::{DeferPolicy, ToolSearchQueryKind, ToolLoadingBackendName};

/// Session 级 Tool Search 模式。默认值见 `harness-session::SessionOptions`。
pub enum ToolSearchMode {
    /// 所有 `DeferPolicy::AutoDefer` 工具立即进入 Deferred 集
    Always,

    /// 阈值启用
    Auto {
        /// 相对阈值：`deferred_tokens / model.max_context_tokens`
        /// 默认 0.10
        ratio: f32,

        /// 绝对下限：`deferred_tokens < min_absolute_tokens` 时不启用
        /// 默认 4_000
        min_absolute_tokens: u32,
    },

    /// 全量注入：`AutoDefer` 降级为 `AlwaysLoad`；`ForceDefer` 注册失败
    Disabled,
}

impl Default for ToolSearchMode {
    fn default() -> Self {
        Self::Auto { ratio: 0.10, min_absolute_tokens: 4_000 }
    }
}
```

### 2.2 `ToolSearchTool`

```rust
pub struct ToolSearchTool {
    scorer: Arc<dyn ToolSearchScorer>,
    backend_selector: Arc<dyn ToolLoadingBackendSelector>,
    coalescer: Arc<MaterializationCoalescer>,
    /// 每次 query 的最大返回条数，默认 5
    default_max_results: usize,
}

impl ToolSearchTool {
    pub fn builder() -> ToolSearchToolBuilder;
}

pub struct ToolSearchToolBuilder { /* ... */ }

impl ToolSearchToolBuilder {
    pub fn with_scorer(self, scorer: Arc<dyn ToolSearchScorer>) -> Self;
    pub fn with_backend_selector(self, sel: Arc<dyn ToolLoadingBackendSelector>) -> Self;
    pub fn with_coalesce_window(self, window: Duration) -> Self;    // 默认 50ms
    pub fn with_max_coalesce_batch(self, max: usize) -> Self;       // 默认 32
    pub fn with_default_max_results(self, max: usize) -> Self;      // 默认 5
    pub fn build(self) -> Arc<ToolSearchTool>;
}

#[async_trait]
impl harness_tool::Tool for ToolSearchTool {
    fn descriptor(&self) -> &ToolDescriptor { &TOOL_SEARCH_DESC }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolStream, ToolError> {
        todo!("see §2.2.3")
    }
}

static TOOL_SEARCH_DESC: std::sync::LazyLock<ToolDescriptor> = std::sync::LazyLock::new(|| {
    ToolDescriptor {
        name: "tool_search".into(),
        display_name: "Tool Search".into(),
        description: TOOL_SEARCH_PROMPT.into(),              // §2.2.2
        category: "meta".into(),
        version: semver::Version::new(1, 0, 0),
        origin: ToolOrigin::Builtin,
        group: ToolGroup::Meta,                              // 元工具组
        trust_level: TrustLevel::AdminTrusted,
        required_capabilities: vec![],                       // 不借用任何高权限 capability
        properties: ToolProperties {
            is_concurrency_safe: true,
            is_read_only: true,
            is_destructive: false,
            long_running: None,
            defer_policy: DeferPolicy::AlwaysLoad,           // 自身必须永远可见
        },
        input_schema: TOOL_SEARCH_INPUT_SCHEMA.clone(),
        output_schema: Some(TOOL_SEARCH_OUTPUT_SCHEMA.clone()),
        dynamic_schema: false,
        provider_restriction: ProviderRestriction::default(),
        search_hint: None,
        budget: ResultBudget::bytes(32 * 1024),              // §2.5 与 ResultBudget 对接
    }
});
```

> `execute` 返回 `ToolStream = Pin<Box<dyn Stream<Item = ToolEvent>>>`，遵循 `harness-tool §2.1` 流式契约。本工具属于"小响应元工具"，正常情况下只 emit 一个 `ToolEvent::Final(ToolResult::Structured(...))`，因此对 `ResultBudget` 不敏感。

#### 2.2.1 `Input` / `Output` schema

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ToolSearchInput {
    /// `select:A,B,C` | `notebook jupyter` | `+slack send`
    pub query: String,

    /// 默认 5；范围 [1, 50]
    #[serde(default)]
    pub max_results: Option<usize>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct ToolSearchOutput {
    /// 命中的工具名（按分数降序）。
    pub matches: Vec<String>,

    pub query: String,

    /// 参与本次匹配的 deferred 集大小（不含 AlwaysLoad 工具）
    pub total_deferred_tools: usize,

    /// 连接中的 MCP Server（未就绪时用于提示模型"稍后再试"）
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pending_mcp_servers: Vec<String>,

    /// 由 backend 决定的物化结果编码（供上层映射为 API content block）
    pub materialization: MaterializationOutcome,
}

pub enum MaterializationOutcome {
    /// Anthropic native：返回 `tool_reference` block 的 tool_name 列表
    ToolReference { tool_names: Vec<String> },

    /// Inline 降级：matches 对应的工具已被追加到 ToolPool 分区 3
    InlineReinjected {
        tool_names: Vec<String>,
        cache_impact: CacheImpact,        // 必为 OneShotInvalidation
    },

    /// Deferred 集为空或全部查询未命中 → 不产生物化
    NoMatch,
}
```

#### 2.2.2 Prompt 常量

```rust
/// 稳定的 system-facing 描述；与 system prompt 一道进入 prompt cache。
/// 改动本常量属于 break-prompt-cache 级变更，需通过 ADR。
pub const TOOL_SEARCH_PROMPT: &str = r#"
Fetches full schema definitions for deferred tools so they can be called.

Deferred tools appear by name in <deferred-tools> messages. Until fetched,
only the name is known — there is no parameter schema, so the tool cannot
be invoked. This tool takes a query, matches it against the deferred tool
list, and returns the matched tools' complete JSONSchema definitions.

Query forms:
- "select:Read,Edit,Grep" — fetch these exact tools by name (comma separated)
- "notebook jupyter"      — keyword search, up to max_results best matches
- "+slack send"           — require "slack" in the name, rank by remaining terms
"#;
```

#### 2.2.3 `execute` 执行流

```text
1. 解析 query → ToolSearchQueryKind::{Select, Keyword}
2. 从 ctx.session_snapshot.tool_pool() 取 Deferred 集（分区 2）
3. 若 Select：按名查找（命中 AlwaysLoad / RuntimeAppended 时按 "已加载" 直接回填 matches，无 backend 物化）
4. 若 Keyword：调 scorer.score(tool, terms) → 排序 → 截断到 max_results
5. 对 matches 调 backend_selector.select(ctx.model_caps) → Backend::materialize 经 coalescer
6. emit Event::ToolSearchQueried + Event::ToolSchemaMaterialized
7. yield 单个 ToolEvent::Final(ToolResult::Structured(ToolSearchOutput))
```

> 整个流程**不**流式 yield 中间结果：ToolSearch 是"低延迟、小响应"的元工具，输出量恒定远低于 `budget = 32KiB`。若 coalescer 命中 batch（一次把 ≥ 32 个工具 schema 合并输出）则直接截断 + 记 `ToolEvent::Progress { fraction: None, message: "coalesced" }`，避免触发上层预算落盘。

#### 2.2.4 与 `ResultBudget` 的对接（ADR-010）

`ToolSearchTool` 本身遵守两条规则：

1. **自身产出受预算约束**：descriptor 声明 `budget = ResultBudget::bytes(32 * 1024)`。单次 query 返回的 `tool_reference` 列表 ≤ 5 条，每条 ≤ 2KB，命中预算的概率极低；若业务配置 `max_results > 50` 且选用 `InlineReinjectionBackend` 导致 schema 内联到结果中，才可能触发 overflow，走 Orchestrator 的 `OverflowAction::TruncateAndOffload` 流程（此时 `ToolSearchOutput.truncated = true` 也会被设置以保持模型语义一致）。
2. **不放大上游 schema**：当 backend 选择 `InlineReinjection` 时，每个候选工具的完整 JSON schema 可能达 10-50KB——这些 schema 应通过 `AttachmentDelta` 或 `DeferredToolsDelta` 经会话 reload 注入 **下一轮 prompt**，而不是把整块 schema 塞进本次 `ToolSearchOutput.matches[*]`。reload 路径由 `harness-session` 执行，与本工具的 `ToolResult` 解耦。

**示例：预算触发**

```text
ToolSearchTool.execute(query="notebook jupyter", max_results=100)
  → 25 个匹配 × 平均 8 KiB schema
  → ToolSearchOutput 序列化 ≈ 200 KiB
  → 超过 descriptor.budget（32 KiB）
  → ToolOrchestrator 按 OverflowAction::TruncateAndOffload：
      - 保留 head/tail 预览（matches 前 5 条 + 尾部 "…and N more"）
      - 原文落盘 → BlobRef
      - ToolResultEnvelope.overflow = Some(OverflowMetadata { ... })
      - emit Event::ToolResultOffloaded
```

Agent 看到 envelope 预览后可调 `read_blob` 拿完整结果，但一般场景直接信任前 5 条排序结果即可。

### 2.3 `ToolLoadingBackend`

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

pub struct ToolLoadingContext {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub model_caps: Arc<ModelCapabilities>,
    /// 供 InlineReinjectionBackend 触发 reload_with 的回调句柄
    pub reload_handle: Option<Arc<dyn ReloadHandle>>,
}

pub enum MaterializeOutcome {
    ToolReferenceEmitted { refs: Vec<ToolReference> },
    InlineReinjected {
        tools: Vec<ToolName>,
        cache_impact: CacheImpact,
    },
}

pub struct ToolReference {
    pub tool_name: String,
}

#[async_trait]
pub trait ReloadHandle: Send + Sync + 'static {
    async fn reload_with_add_tools(
        &self,
        tools: Vec<ToolName>,
    ) -> Result<CacheImpact, HarnessError>;
}
```

#### 2.3.1 `AnthropicToolReferenceBackend`

```rust
pub struct AnthropicToolReferenceBackend;

impl AnthropicToolReferenceBackend {
    pub const NAME: &'static str = "anthropic_tool_reference";
}

#[async_trait]
impl ToolLoadingBackend for AnthropicToolReferenceBackend {
    fn backend_name(&self) -> ToolLoadingBackendName { Self::NAME.into() }

    async fn materialize(
        &self,
        _ctx: &ToolLoadingContext,
        requested: &[ToolName],
    ) -> Result<MaterializeOutcome, ToolLoadingError> {
        Ok(MaterializeOutcome::ToolReferenceEmitted {
            refs: requested.iter().map(|n| ToolReference { tool_name: n.to_string() }).collect(),
        })
    }
}
```

**前置条件**：`ctx.model_caps.supports_tool_reference == true`；否则 `ToolLoadingBackendSelector` 不会选它。

**CacheImpact**：`NoInvalidation`（由 Anthropic 服务端展开 schema，客户端前缀不变）。

#### 2.3.2 `InlineReinjectionBackend`

```rust
pub struct InlineReinjectionBackend {
    coalescer: Arc<MaterializationCoalescer>,
}

impl InlineReinjectionBackend {
    pub const NAME: &'static str = "inline_reinjection";

    pub fn new(coalescer: Arc<MaterializationCoalescer>) -> Self {
        Self { coalescer }
    }
}

#[async_trait]
impl ToolLoadingBackend for InlineReinjectionBackend {
    fn backend_name(&self) -> ToolLoadingBackendName { Self::NAME.into() }

    async fn materialize(
        &self,
        ctx: &ToolLoadingContext,
        requested: &[ToolName],
    ) -> Result<MaterializeOutcome, ToolLoadingError> {
        let handle = ctx.reload_handle.as_ref()
            .ok_or(ToolLoadingError::ReloadHandleMissing)?
            .clone();

        let cache_impact = self.coalescer
            .submit(requested.to_vec(), handle)
            .await?;

        Ok(MaterializeOutcome::InlineReinjected {
            tools: requested.to_vec(),
            cache_impact,
        })
    }
}
```

**CacheImpact**：`OneShotInvalidation { reason: ToolsetAppended, .. }`（对齐 ADR-003 §2.3）。

### 2.4 `MaterializationCoalescer`（合并窗口）

同一 `Session` 上 N ms 内的多次物化请求合并为**一次** `session.reload_with(add_tools=..)`，把 N 次 `OneShotInvalidation` 压缩为 **1 次**。

```rust
pub struct MaterializationCoalescer {
    window: Duration,       // 默认 50ms
    max_batch: usize,       // 默认 32
    inner: Mutex<CoalescerState>,
}

struct CoalescerState {
    pending: HashMap<SessionId, PendingBatch>,
}

struct PendingBatch {
    tools: IndexSet<ToolName>,          // 去重 + 保留加入序
    waiters: Vec<oneshot::Sender<Result<CacheImpact, HarnessError>>>,
    deadline: Instant,
}

impl MaterializationCoalescer {
    pub fn new(window: Duration, max_batch: usize) -> Arc<Self> { /* ... */ }

    /// 调用方传入该次希望物化的工具；返回本次物化完成后的 CacheImpact。
    /// 若窗口内的批次达到 max_batch，立即 flush。
    pub async fn submit(
        &self,
        tools: Vec<ToolName>,
        handle: Arc<dyn ReloadHandle>,
    ) -> Result<CacheImpact, HarnessError> { /* ... */ }
}
```

**语义约束**：

1. 窗口以 "首个 submit 调用" 为起点；后续同 session 的 submit 在 deadline 前加入同一批次
2. `max_batch` 触顶或 deadline 到达，统一调用 `handle.reload_with_add_tools(batch)` 一次
3. 返回给所有 waiter **同一个** `CacheImpact`（即 "一次失效" 被全体共享）
4. 窗口为 `Duration::ZERO` 时退化为"每次直接 flush"（测试用）

**不对 `AnthropicToolReferenceBackend` 生效**（该 backend 不走 `reload_with`）。

### 2.5 `ToolLoadingBackendSelector`

```rust
#[async_trait]
pub trait ToolLoadingBackendSelector: Send + Sync + 'static {
    async fn select(
        &self,
        ctx: &ToolLoadingContext,
    ) -> Arc<dyn ToolLoadingBackend>;
}

pub struct DefaultBackendSelector {
    anthropic: Arc<AnthropicToolReferenceBackend>,
    inline: Arc<InlineReinjectionBackend>,
}

#[async_trait]
impl ToolLoadingBackendSelector for DefaultBackendSelector {
    async fn select(&self, ctx: &ToolLoadingContext) -> Arc<dyn ToolLoadingBackend> {
        if ctx.model_caps.supports_tool_reference {
            self.anthropic.clone()
        } else {
            self.inline.clone()
        }
    }
}
```

业务方可实现自定义 selector 以接入未来的 provider-specific 机制（如 OpenAI 若引入等价 beta 能力）。

### 2.6 `ToolSearchScorer`

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

pub struct ScoringTerms {
    /// 全部非 `+` 前缀的词
    pub optional: Vec<String>,
    /// 带 `+` 前缀的必选词（扣除前缀后）
    pub required: Vec<String>,
}

pub struct ScoringContext {
    /// 当前 Session 的 discovered set，用于降权（已加载的工具不应再占 top N）
    pub discovered: Arc<HashSet<ToolName>>,
}
```

#### 2.6.1 `DefaultScorer`

```rust
pub struct DefaultScorer {
    weights: ScoringWeights,
}

#[derive(Debug, Clone, Copy)]
pub struct ScoringWeights {
    pub name_part_exact_mcp: u32,       // 12
    pub name_part_exact_regular: u32,   // 10
    pub name_part_partial_mcp: u32,     // 6
    pub name_part_partial_regular: u32, // 5
    pub full_name_fallback: u32,        // 3 — 仅 score == 0 时生效
    pub search_hint: u32,               // 4
    pub description: u32,               // 2
    /// discovered 工具降权倍率；默认 0.3（保留可搜索性但排名靠后）
    pub discovered_penalty_ratio: f32,  // 0.3
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            name_part_exact_mcp: 12,
            name_part_exact_regular: 10,
            name_part_partial_mcp: 6,
            name_part_partial_regular: 5,
            full_name_fallback: 3,
            search_hint: 4,
            description: 2,
            discovered_penalty_ratio: 0.3,
        }
    }
}
```

**`name` 解析规则**（与 Claude Code 对齐）：

- MCP 工具 `mcp__server__action` → 去前缀、按 `__` / `_` 再拆，`parts = ["server", "action"]`
- 常规工具 `FileReadTool` → CamelCase 展开，`parts = ["file", "read", "tool"]`

**匹配流程**：

1. 查询按空格切词 → `required`（`+`）/ `optional`
2. 若 `required` 非空，先过滤到"所有 required 都命中 name/description/search_hint"的候选
3. 对候选按 `weights` 累计评分；`optional` 同样参与评分
4. 命中 discovered 集的工具最终分数 × `discovered_penalty_ratio`
5. 返回按分数降序前 `max_results` 条；score == 0 的条目被丢弃

**不做中文分词**（工具名按 MCP 约定 ASCII；中文工具名视作反模式）。

**可替换但不公开**：`ToolSearchScorer` 作为 `HarnessBuilder::with_tool_search_scorer(..)` 入口供 admin 替换；不作为租户 Session 级配置暴露。

### 2.7 `DeferredToolsDelta`

```rust
pub struct DeferredToolsDelta {
    pub added_names: Vec<ToolName>,
    pub removed_names: Vec<ToolName>,
    pub source: ToolPoolChangeSource,
    pub at: DateTime<Utc>,
}

/// 供 `Event::ToolDeferredPoolChanged.source` 复用；见 `harness-contracts`
#[non_exhaustive]
pub enum ToolPoolChangeSource {
    SessionCreated,
    McpListChanged { server_id: McpServerId },
    PluginLoaded  { plugin_id: PluginId },
    ReloadApplied,
}

impl DeferredToolsDelta {
    /// 编码为随下一轮 user message 注入的 attachment 文本。
    /// 格式刻意保持稳定（见 §3 提示契约）。
    pub fn to_attachment_text(&self) -> String { /* §2.7.1 */ }
}
```

#### 2.7.1 Attachment 文本契约

```text
<deferred-tools changed-at="2026-04-25T10:32:11Z">
  <added>
    mcp__slack__post_message
    mcp__slack__list_channels
    my_plugin__export_csv
  </added>
  <removed>
    legacy_tool
  </removed>
</deferred-tools>
```

- **追加在 user message 尾部**，不进 system prompt → 不破坏前缀 cache
- 首次 Session 用 `<deferred-tools initial="true">` 列全量，之后用增量
- `removed` 仅表示"从 deferred 集中消失"；若工具随后进入 AlwaysLoad/RuntimeAppended（即"已可用"），不出现在 `removed`
- 文本契约稳定性：格式变动视作 break-prompt-cache 级，需 ADR

### 2.8 `DiscoveredToolProjection`

```rust
pub struct DiscoveredToolProjection {
    pub session_id: SessionId,
    pub tools: HashSet<ToolName>,
    pub last_event_id: EventId,
}

impl harness_session::SessionProjection for DiscoveredToolProjection {
    fn projection_id(&self) -> &'static str { "discovered_tools" }

    fn apply(&mut self, event: &Event) -> Result<(), ProjectionError> {
        match event {
            Event::ToolSchemaMaterialized(e) => {
                self.tools.extend(e.names.iter().cloned());
                self.last_event_id = e.event_id;
            }
            Event::SessionForked(e) => {
                if let Some(inherit) = &e.inherited_discovered {
                    self.tools = inherit.iter().cloned().collect();
                }
            }
            Event::CompactionApplied(e) => {
                if let Some(preserved) = &e.preserved_discovered {
                    self.tools = preserved.iter().cloned().collect();
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

`harness-session` 在 `Session::from_journal` 时把本 projection 纳入 pipeline；`harness-engine` 在每轮 prompt 组装前读取。

### 2.9 阈值判据 · Token / Char 估算

```rust
pub struct DeferredThresholdEvaluator {
    token_counter: Option<Arc<dyn ModelTokenCounter>>,
}

impl DeferredThresholdEvaluator {
    /// 返回 `(enabled, metrics)` —— metrics 用于落 Event::ToolDeferredPoolChanged.metrics
    pub async fn evaluate(
        &self,
        mode: &ToolSearchMode,
        deferred: &[Arc<dyn Tool>],
        model_desc: &ModelDescriptor,
    ) -> (bool, ThresholdMetrics) { /* ... */ }
}

pub struct ThresholdMetrics {
    pub token_count: Option<u64>,        // None 表示 API 不可用时走了 char 回退
    pub char_count: u64,
    pub threshold_tokens: u64,
    pub absolute_floor: u64,             // = ToolSearchMode::Auto.min_absolute_tokens
}
```

- **优先**使用 provider 的 token 计数 API（`harness-model::ModelTokenCounter`）
- **回退**使用字符数估算：`chars / 2.5 ≈ tokens`（经验系数，Claude Code 同）
- `ToolSearchMode::Auto` 的判据：`token_count >= max(threshold_tokens, absolute_floor)`

## 3. 对外 Prompt 契约

本 crate 对外"向模型暴露的文本"仅三处，**均属于 prompt cache 稳定面**，改动视作破坏性：

1. `TOOL_SEARCH_PROMPT`（§2.2.2）
2. `DeferredToolsDelta::to_attachment_text()` 输出格式（§2.7.1）
3. `ToolSearchOutput.matches` 的排序契约（分数降序，同分按 `ToolName` 字母序稳定排）

其他文本（调试日志、事件字段等）不构成 prompt 契约。

## 4. 事件轨迹

```text
Session 创建期
   │
   ├─ 分桶：按 DeferPolicy + ToolSearchMode + ThresholdEvaluator
   │
   ▼
Event::ToolDeferredPoolChanged {
   session_id,
   source = ToolPoolChangeSource::SessionCreated,
   added: [..], removed: [],
   metrics: ThresholdMetrics { .. },
   at,
}
   │
   ▼  [下一轮 prompt 前拼 attachment]
   │
   ▼
模型调 tool_search
   │
   ▼
Event::ToolSearchQueried {
   session_id, run_id, tool_use_id,
   query, query_kind,
   scored: [{name, score}], matched: [..],
   max_results, total_deferred_tools,
   at,
}
   │
   ▼  [backend.materialize]
   │
   ├─ AnthropicToolReferenceBackend
   │     └→ Event::ToolSchemaMaterialized {
   │          backend = "anthropic_tool_reference",
   │          names: [..],
   │          cache_impact = NoInvalidation,
   │        }
   │
   └─ InlineReinjectionBackend (经 coalescer)
         ├→ Event::ToolSchemaMaterialized {
         │    backend = "inline_reinjection",
         │    names: [..],
         │    cache_impact = OneShotInvalidation,
         │    coalesced_from_tool_uses: [ToolUseId, ..],
         │  }
         └→ Event::SessionReloadApplied { mode: AppliedInPlace, .. }   // 来自 harness-session
```

## 5. 依赖

```toml
[dependencies]
octopus-harness-contracts = { path = "../octopus-harness-contracts" }
octopus-harness-tool      = { path = "../octopus-harness-tool" }
octopus-harness-model     = { path = "../octopus-harness-model" }

async-trait   = "0.1"
tokio         = { version = "1", features = ["sync", "time", "macros"] }
indexmap      = "2"
regex         = "1"
serde         = { version = "1", features = ["derive"] }
serde_json    = "1"
schemars      = "0.8"
thiserror     = "1"
tracing       = "0.1"
```

**禁止**新增其他 harness crate 依赖。注入 `ToolSearchTool` 到默认工具集的责任在 L4 `harness-sdk`。

## 6. Feature Flags

```toml
[features]
default = ["anthropic-native", "inline-fallback"]

# 启用 AnthropicToolReferenceBackend（tool_reference beta 路径）
anthropic-native = []

# 启用 InlineReinjectionBackend（多 provider 通用回退路径）
inline-fallback  = ["tokio/time"]
```

`harness-sdk` 的 `default` 集合开启 `tool-search = ["octopus-harness-tool-search"]`（见 `feature-flags.md §3.5`）。

## 7. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolLoadingError {
    #[error("reload handle missing: inline backend requires session.reload_with handle")]
    ReloadHandleMissing,

    #[error("requested tool not in deferred set: {0}")]
    NotInDeferredSet(String),

    #[error("backend internal: {0}")]
    Backend(String),

    #[error("coalescer closed")]
    CoalescerClosed,

    #[error("reload rejected: {0}")]
    ReloadRejected(String),
}

impl From<ToolLoadingError> for HarnessError {
    fn from(e: ToolLoadingError) -> Self { HarnessError::Internal(e.to_string()) }
}
```

## 8. 可观测性

| 指标 | 标签 | 说明 |
|---|---|---|
| `tool_search_queries_total` | `query_kind`, `backend`, `outcome` | Select/Keyword × backend × match/empty |
| `tool_search_match_count` | `query_kind` | 每次查询匹配数分布 |
| `tool_search_score_top1` | — | 首位命中分数，低于阈值可能提示 scorer 效果退化 |
| `tool_materialization_duration_ms` | `backend` | backend.materialize 耗时 |
| `tool_coalesce_batch_size` | — | 合并窗口内的批次尺寸 |
| `tool_coalesce_waiters` | — | 同批次 waiter 数（1 = 无合并；>1 = 合并生效） |
| `tool_deferred_pool_size` | `source` | 当前 deferred 集大小按来源分桶 |

## 9. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | `DefaultScorer` 权重：每条规则（精确/部分/full/hint/description）各写一组黄金测试 |
| 单元 | `parseToolName`：mcp / CamelCase / 纯 snake_case 三种命名往返稳定 |
| 并发 | `MaterializationCoalescer`：同 session × 32 并发 submit → 仅触发 1 次 reload；不同 session 不相互合并 |
| 超时 | coalescer 窗口内无新 submit，deadline 到达触发 flush |
| 契约 | `TOOL_SEARCH_PROMPT` / `DeferredToolsDelta::to_attachment_text` 文本 golden |
| Backend | `AnthropicToolReferenceBackend` 产出 `NoInvalidation`；`InlineReinjectionBackend` 产出 `OneShotInvalidation` |
| 事件 | 每次 `execute` 至少 emit 一条 `ToolSearchQueried` + 至多一条 `ToolSchemaMaterialized` |
| Projection | `DiscoveredToolProjection` 在 Fork / Compact 下 discovered 集的传递与收敛 |

## 10. 反模式

| 反模式 | 原因 |
|---|---|
| 在 Session 运行期修改 `ToolSearchMode` | 会产生不必要的 cache 失效（`reload_with` 将返回 `Rejected`） |
| `ToolSearchTool.defer_policy = AutoDefer` | 自身被 defer 则无法"解锁"其他工具；`AlwaysLoad` 是硬约束 |
| `InlineReinjectionBackend` 跳过 coalescer | 会把 N 次 query 放大为 N 次 cache miss |
| 把评分权重做成租户级配置 | 打坏 benchmark、让事件回放不稳定；仅 admin 替换 |
| 依赖 `process.env.ENABLE_TOOL_SEARCH` | 本 crate 不读环境变量；开关统一走 `SessionOptions.tool_search` |
| 中文分词 | 工具名按 MCP 协议惯例 ASCII；中文工具名是反模式 |

## 11. 相关

- ADR-009 Deferred Tool Loading 与 Tool Search 元工具
- ADR-003 Prompt Cache 硬约束（CacheImpact 定义源）
- ADR-005 MCP 双向（MCP 工具默认 AutoDefer 的来源）
- ADR-008 Crate 分层（本 crate 为 L2 · §2.4 清单）
- `crates/harness-tool.md` §2.4 Pool 三分区
- `crates/harness-mcp.md` §6 `tools/list_changed` 运行期处理
- `crates/harness-model.md` `ModelCapabilities`
- `crates/harness-session.md` `SessionOptions.tool_search` + `DiscoveredToolProjection` 注册
- `context-engineering.md` §9 注入顺序对缓存的影响
- Evidence: CC 源码 `tools/ToolSearchTool/`、`utils/toolSearch.ts`、`utils/toolPool.ts`
