# ADR-009 · Deferred Tool Loading 与 Tool Search 元工具

- **状态**：Accepted
- **日期**：2026-04-25
- **决策者**：架构组
- **影响范围**：`harness-contracts` / `harness-tool` / `harness-tool-search`（新增）/ `harness-mcp` / `harness-model` / `harness-session` / `harness-engine` / `harness-journal` / `harness-observability`

## 1. 背景与问题

### 1.1 症状

ADR-003 把 **Session 运行期 system prompt / toolset / memory 三件套冻结**立为硬约束。但工具供给侧事实上是单调增长的：

- 企业租户接入的 **MCP Server 数量 → 工具数**：在生产环境常见 5~20 台 Server，单台平均 10~50 个工具
- **Skill / Plugin** 生态扩张，每个 Skill 可带 N 个工具
- **Agent 级专属工具**（Team Member 的角色工具）需要按角色差异化加载

把全部工具 schema 一次性注入 system prompt 会同时触发两类冲突：

| 冲突 | 表象 |
|---|---|
| **Token 预算** | 少数活跃工具的 schema 就能吃掉数万 token，模型用于推理的上下文被挤压 |
| **Prompt Cache 稳定性** | 每次租户加一个 MCP / 装一个 Skill，整个 prefix cache 失效（违反 ADR-003 初衷） |

### 1.2 现有文档的缺口

目前架构文档存在三个**孤立**的半成品字段/机制，没有被串起来：

1. `harness-tool.md §2.2` 的 `ToolProperties.defer_load: bool` —— **字段存在，语义、触发路径、回收机制全部缺失**
2. `harness-mcp.md §6` 的"pending 队列" —— **只定义了"不立即 re-inject"，没定义如何让模型感知到这些 pending 工具**
3. `harness-model.md` 的 `ModelCapabilities` —— **缺 `supports_tool_reference` 能力位，无法驱动跨 provider 的降级**

### 1.3 行业先例

Claude Code 对同一问题给出的答案是 **Tool Search 机制**（见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/`）：

- 把工具分为 **AlwaysLoad**（含 schema 注入）与 **Deferred**（仅名字露出）
- 引入内建元工具 `ToolSearch`，模型主动 `query` → 服务端返回 `tool_reference` block → 服务端侧展开 schema，**不改 system prompt 前缀**
- 用 `DeferredToolsDelta` attachment 告诉模型工具集的增减，避免 "pending 队列" 对模型不可见

这是目前业界对 "MCP 工具爆炸 + Prompt Cache 必须稳定" 最成熟的工程解答。

## 2. 决策

### 2.1 总纲

引入 **Deferred Tool Loading** 作为 ADR-003 硬约束的自然延伸：工具按 `DeferPolicy` 分层，由新的 `harness-tool-search` crate 提供统一的发现/加载路径；按 provider 能力自动选择加载 backend。

### 2.2 `DeferPolicy`：三态枚举（Q1）

```rust
/// 单个 Tool 的加载策略。由工具注册方决定，Session 不能修改。
pub enum DeferPolicy {
    /// 总是在 Session 创建期进入 ToolPool 固定集，schema 立即可见。
    /// 用于：TodoTool / TaskStopTool / ToolSearchTool 自身 /
    ///       管控方通过 `with_always_load(name)` 强制覆盖的工具。
    AlwaysLoad,

    /// 由 Session 级 ToolSearchMode 决定。
    /// - Session.tool_search = Always        → 进入 Deferred 集
    /// - Session.tool_search = Auto { .. }   → 按阈值判断
    /// - Session.tool_search = Disabled      → 降级为 AlwaysLoad
    /// 用于：MCP 工具（默认）/ Plugin 工具（默认）/ 显式标记的 defer 工具。
    AutoDefer,

    /// 强制 Deferred。Session.tool_search = Disabled 时，
    /// 不降级为 AlwaysLoad，而是 **拒绝注册**（返回 ToolError::DeferralRequired）。
    /// 用于：管控级敏感/体积极大的工具，没 Tool Search 宁可不挂载。
    ForceDefer,
}
```

`ToolProperties` 用 `defer_policy: DeferPolicy` 字段**取代**现有 `defer_load: bool`。

### 2.3 `ToolSearchMode`：ratio + 绝对下限双重判据（Q2）

```rust
pub enum ToolSearchMode {
    /// 所有 AutoDefer 工具立即进入 Deferred 集
    Always,

    /// 阈值启用：deferred 集的 token 估算超过判据才启用
    Auto {
        /// 相对阈值：deferred_tokens / model.max_context_tokens
        ratio: f32,                 // 默认 0.10
        /// 绝对下限：deferred_tokens < min_absolute_tokens 时不启用
        min_absolute_tokens: u32,   // 默认 4_000
    },

    /// 全量注入，AutoDefer 降级为 AlwaysLoad；ForceDefer 注册失败
    Disabled,
}
```

**为何加绝对下限**：小上下文模型（如 200K）× 10% = 20K，但实际只有 2K deferred tokens 时启用 Tool Search 反而增加一次 round-trip。`min_absolute_tokens` 保证"值得启用"。

判据优先级：**token 计数优先，字符数回退**（字符→token 经验系数 2.5，CC-已验证）。

### 2.4 `ToolLoadingBackend`：多 provider 降级（Q3）

```rust
#[async_trait]
pub trait ToolLoadingBackend: Send + Sync + 'static {
    fn name(&self) -> &'static str;

    /// 把一批 deferred 工具"物化"为模型可调用的 schema。
    async fn materialize(
        &self,
        ctx: &ToolLoadingContext,
        requested: &[ToolName],
    ) -> Result<MaterializeOutcome, ToolLoadingError>;
}

pub enum MaterializeOutcome {
    /// Anthropic native 路径：返回 tool_reference blocks，服务端展开 schema。
    /// CacheImpact::NoInvalidation（前缀不变）。
    ToolReferenceEmitted {
        refs: Vec<ToolReference>,
    },

    /// Inline 降级路径：SDK 内部触发 reload_with(add_tools=..)
    /// 产生 CacheImpact::OneShotInvalidation。
    InlineReinjected {
        tools: Vec<ToolName>,
        cache_impact: CacheImpact,  // 必为 OneShotInvalidation
    },
}
```

**两个内置 backend**：

| Backend | 适用 | CacheImpact |
|---|---|---|
| `AnthropicToolReferenceBackend` | ModelCapabilities.supports_tool_reference = true 的模型 | `NoInvalidation` |
| `InlineReinjectionBackend` | 其他（OpenAI / Gemini / Bedrock / 不支持的 Anthropic 模型如 Haiku） | `OneShotInvalidation` |

**合并窗口（coalesce window）**：

`InlineReinjectionBackend` 维护一个 **N ms 的合并窗口**（默认 50ms），同一 turn 内多次 `materialize` 调用合并为一次 `reload_with`，把 N 次 OneShotInvalidation 压缩为 1 次。这是本 ADR 对 CC 的**显式增强**——CC 没有该机制，因为它只跑 Anthropic native 路径。

```rust
pub struct InlineReinjectionBackend {
    pub coalesce_window: Duration,  // 默认 50ms
    pub max_batch: usize,           // 默认 32（防暴涨）
}
```

### 2.5 `ToolSearchTool`：内建元工具（Q6）

`ToolSearchTool` 作为 **L2 `harness-tool-search` crate** 提供的内建工具，挂载在 `harness-tool::builtin` 的默认 toolset 里：

```rust
pub struct ToolSearchTool {
    scorer: Arc<dyn ToolSearchScorer>,
    backend: Arc<dyn ToolLoadingBackend>,
}

impl Tool for ToolSearchTool {
    fn descriptor(&self) -> &ToolDescriptor { /* name = "tool_search" */ }
    async fn execute(&self, input: Value, ctx: ToolContext)
        -> Result<ToolStream, ToolError>
    {
        /* ... */
    }
}
```

不选 Skill / 系统 RPC 的理由：
- 必须进入 tool_use 循环（有 `tool_use_id` / 进 permission broker / 落事件流）
- Skill 是 prompt 模板，不是执行单元
- 系统 RPC 绕过 tool_use ID 会破坏事件轨迹的完整性

### 2.6 查询语法（对齐 CC，但禁用环境变量口子）

```text
# 直接选择（逗号分隔支持批量）
select:FileRead,FileEdit,Grep

# 关键字搜索（按评分排序，截断到 max_results）
notebook jupyter

# 必选词（"+" 前缀，所有必选词都须命中）
+slack send message
```

**不**引入 `ENABLE_TOOL_SEARCH` 环境变量开关；改由 `SessionOptions.tool_search` 配置项表达。

### 2.7 评分规则（Q5）

`ToolSearchScorer` 作为 trait 暴露，**默认实现照搬 CC 权重**（经 A/B 验证的 baseline）：

```rust
pub struct ScoringWeights {
    pub name_part_exact_mcp: u32,       // 12
    pub name_part_exact_regular: u32,   // 10
    pub name_part_partial_mcp: u32,     // 6
    pub name_part_partial_regular: u32, // 5
    pub full_name_fallback: u32,        // 3（仅 score==0 时生效）
    pub search_hint: u32,               // 4
    pub description: u32,               // 2
}

#[async_trait]
pub trait ToolSearchScorer: Send + Sync {
    async fn score(
        &self,
        tool: &ToolDescriptor,
        terms: &[String],
        context: &ScoringContext,
    ) -> u32;
}
```

**可替换，但不公开为租户可配置**：权重是经验参数，暴露给租户会打坏 benchmark；只允许在 `HarnessBuilder` 级别替换（admin 级）。

**不做**中文分词：工具名按 MCP 协议惯例是 ASCII；中文工具名视作反模式。

### 2.8 `DiscoveredToolSet`：Projection（Q4）

已在会话中"解锁"过 schema 的工具集合作为 **Session Projection** 维护，不扫描消息历史（CC 因无 EventStore 才这么做，我们无需效仿）：

```rust
pub struct DiscoveredToolProjection {
    pub session_id: SessionId,
    pub tools: HashSet<ToolName>,
    pub last_event_id: EventId,
}

impl SessionProjection for DiscoveredToolProjection {
    fn apply(&mut self, event: &Event) -> Result<()> {
        match event {
            Event::ToolSchemaMaterialized(e) => {
                self.tools.extend(e.names.clone());
            }
            Event::SessionForked(e) => {
                // 父 session 的 discovered set 随 Fork 传递
                self.tools = e.inherited_discovered.clone();
            }
            Event::SessionCompacted(e) => {
                // Compact 边界把 discovered set 固化进事件
                self.tools = e.preserved_discovered.clone();
            }
            _ => {}
        }
        Ok(())
    }
}
```

### 2.9 `DeferredToolsDelta`：工具集增量通告

ADR-003 §7 升级 `harness-mcp.md §6` 的"pending 队列只记事件"为**显式通告**：

- Session 首次进入 deferred 模式时，发送一条 `system-reminder` 类 attachment，列出全部 deferred 工具名（不含 schema）
- 后续工具集变化（MCP 加一个 Server / 插件注册新工具），产生 `Event::ToolDeferredPoolChanged { added, removed }` → 下一轮 prompt 前追加一条 delta attachment
- **追加不影响前缀 cache**（因为 attachment 在尾部，不在 system prompt 内）
- `removed` 的语义：**只宣告"从 deferred 集中消失"**；若该工具本就已 materialize 到 AlwaysLoad 集则不宣告（防止模型误认为工具不可调用）

### 2.10 `ModelCapabilities` 扩展

`harness-model::ModelCapabilities` 新增：

```rust
pub struct ModelCapabilities {
    // ...（已有字段）
    pub supports_tool_reference: bool,
    pub supports_prompt_cache: bool,   // 顺便显式化
    pub tool_reference_beta_header: Option<&'static str>,
}
```

由 `ModelCatalog` 在 provider 注册时填写；`ToolSearchBackendSelector` 读取该位选择 backend。

## 3. 替代方案

### 3.1 A · 维持现状：全量注入 + 管控 MCP 数量

- ❌ 不可持续：企业用户一定会接 10+ 个 Server
- ❌ 违反 ADR-003 的立意：不是"尽量少破坏 cache"，而是"最多破坏 1 次 cache"

### 3.2 B · 只照搬 CC，Anthropic-only，其他 provider 不支持 Tool Search

- ❌ 丧失 Octopus "多 provider 中立"的核心卖点
- ❌ MCP 多了之后 OpenAI 用户被迫放弃 MCP 或承担 cache 成本

### 3.3 C · 多 backend + ratio/absolute 双阈值 + InlineReinjection 合并窗口（采纳）

- ✅ 对齐 ADR-003 的硬约束语义（`NoInvalidation` / `OneShotInvalidation` 已建模）
- ✅ 跨 provider 统一心智模型
- ✅ 合并窗口把多次 cache miss 压成一次，对"模型一轮连续多次查询"鲁棒

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| **两 backend 实现复杂度** | `AnthropicToolReferenceBackend` + `InlineReinjectionBackend` 两套路径 | 抽象为 `ToolLoadingBackend` trait；内置实现仅 ~400 LOC |
| **合并窗口引入延迟** | 每次 materialize 强制等 ≤50ms | 窗口值可配置；对 AnthropicToolReferenceBackend 不生效 |
| **评分规则隐式黑盒** | 租户无法调权重 | 故意的——提供 `ToolSearchScorer` trait 供 admin 替换 |
| **Projection 依赖 Event 种类扩展** | 必须新增 3 条事件 | 事件轻量，不影响现有 schema |
| **ForceDefer 增加注册失败路径** | Session.tool_search=Disabled 时部分工具不可注册 | 注册时提供明确错误码，业务层可选择降级 |

## 5. 后果

### 5.1 正面

- **ADR-003 的硬约束在工具爆炸场景下仍然成立**
- MCP 工具任意数量接入，不触发 prompt cache 全失效
- 跨 provider 统一心智：Anthropic native + Inline fallback 两条路径统一为同一 trait
- 评分规则可审计（事件流含 query / matched / score）

### 5.2 负面

- 新增 1 个 crate（`harness-tool-search`）、~3 条事件、~2 个内建 backend
- `InlineReinjectionBackend` 仍有 `OneShotInvalidation` 代价——但这是与 ADR-003 §2.3 一致的显式语义
- 依赖 Anthropic beta API 的 `tool_reference` —— 需要在 ModelCatalog 显式声明能力位，不依赖运行时探测

### 5.3 中立

- Skill / Plugin 的工具默认进入 `DeferPolicy::AutoDefer`，存量用户无感知
- 内建工具（Bash / FileRead / Grep 等）默认保持 `AlwaysLoad`，行为不变

## 6. 实现指引

### 6.1 最小可用路径

1. `harness-contracts`：扩 `ToolProperties` / 新增 3 条事件 / `ModelCapabilities.supports_tool_reference`
2. `harness-tool-search`（新 crate）：`ToolSearchTool` / `ToolLoadingBackend` / `InlineReinjectionBackend`(首版) / `AnthropicToolReferenceBackend`(首版) / 默认 `ToolSearchScorer`
3. `harness-tool`：Pool 从"固定集 + 追加集"扩展为"**固定集 + 延迟集 + 追加集**"三层；`ToolSearchTool` 加入默认 `BuiltinToolset::Default`
4. `harness-mcp`：§6 的 `on_list_changed` 追加 `Event::ToolDeferredPoolChanged` 触发路径
5. `harness-session`：`SessionOptions.tool_search: ToolSearchMode`；`DiscoveredToolProjection` 注册进 projection pipeline

### 6.2 Event 轨迹

```text
Session 创建期
    │
    ▼
[按 DeferPolicy + ToolSearchMode 分桶]
    │
    ├─ AlwaysLoad  → ToolPool 固定集（含 schema）
    └─ Deferred    → Event::ToolDeferredPoolChanged { added, removed }
                     → 作为 delta attachment 随下一轮 prompt 注入

运行期模型调 tool_search
    │
    ▼
Event::ToolSearchQueried { query, query_kind, scored: [{name, score}], matched: [...] }
    │
    ▼
ToolLoadingBackend::materialize
    │
    ├─ ToolReferenceEmitted → Event::ToolSchemaMaterialized {
    │                           names, backend: "anthropic_tool_reference",
    │                           cache_impact: NoInvalidation
    │                         }
    └─ InlineReinjected     → Event::ToolSchemaMaterialized {
                                names, backend: "inline_reinjection",
                                cache_impact: OneShotInvalidation
                              }
                              + Event::SessionReloadApplied { mode: AppliedInPlace, .. }
```

### 6.3 与 ADR-003 的关系锚点

| 操作 | ADR-003 语义 | 本 ADR 的 backend 决定 |
|---|---|---|
| MCP 加 Server（工具默认 AutoDefer） | 无（不走 `reload_with`，只改 deferred 池） | `ToolDeferredPoolChanged` 追加；cache 不动 |
| 模型 `tool_search` → Anthropic 模型 | 未触发 `reload_with` | `NoInvalidation` |
| 模型 `tool_search` → 非 Anthropic 模型 | 触发 `reload_with(add_tools)` | `OneShotInvalidation` |
| 删除 MCP Server | `ForkedNewSession` | `FullReset`，discovered set 不随 Fork 传递已删除项 |

### 6.4 Proxy / Kill Switch

- Session 级 `tool_search = Disabled` 是唯一关闭开关
- **不**提供环境变量 kill switch（不走 CC 的 `ENABLE_TOOL_SEARCH` 路径）
- `ModelCatalog` 可通过 admin 配置声明某个 provider 的 `supports_tool_reference = false`，强制走 `InlineReinjectionBackend`

## 7. 相关

- ADR-003 · Prompt Cache 硬约束
- ADR-005 · MCP 双向（tools/list_changed 源头）
- `crates/harness-tool-search.md`（新增规格书）
- `crates/harness-tool.md` §2.4 Pool 三分区
- `crates/harness-mcp.md` §6
- `crates/harness-model.md` `ModelCapabilities`
- `context-engineering.md` §9 注入顺序表
- Evidence: CC 源码 `ToolSearchTool/`（见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/ToolSearchTool/`）
