# ADR-010 · Tool 结果预算与溢出自动落盘 BlobStore

- **状态**：Accepted
- **日期**：2026-04-25
- **决策者**：架构组
- **影响范围**：`harness-contracts` / `harness-tool` / `harness-engine` / `harness-journal` / `harness-observability` / `harness-mcp` / `harness-skill`

## 1. 背景与问题

### 1.1 症状

`harness-tool.md §2.2` 现有 `ToolProperties.max_result_size_chars: Option<usize>`，但仅是**单字段未消费的占位**：

- 没有定义"超过 budget 时**截断 vs 落盘 vs 拒绝**"的处理矩阵；
- 没有规定 ToolOrchestrator 在 budget 命中时应产出什么 `Event` / `MessagePart`；
- 没有定义"模型上下文里看到什么"的提示契约（例如 tail/preview）；
- 与 `harness-contracts.md §3.7 BlobRef / BlobStore` 之间缺少调用粘合层。

后果：

| 风险 | 表象 |
|---|---|
| 上下文爆炸 | 一次 `BashTool` `find /` 就能产生数 MB 输出，整个 Session 上下文被挤爆，触发 ADR-003 的 prompt cache 失效与 ADR-009 deferred 工具被迫卸载 |
| 模型困惑 | 输出被静默截断，模型无法判断信息是否完整，会做出错误推理 |
| 审计缺口 | 截断后的内容**不可恢复**，事后无法重放（违反 ADR-001 事件可重放原则） |
| 配额不一致 | MCP 工具 / Bash / FileRead 各自有不同截断逻辑，UI 行为不可预测 |

### 1.2 行业先例

| 系统 | 做法 | 关键能力 |
|---|---|---|
| Claude Code | `Tool.tool_result_size_metric_name` + 命中阈值后强制 ToolResult 携带 `truncated_to`、TUI 显示 "tail/preview" | 全局阈值默认 30000 字符；BashTool 对 `head -200/tail -200` 提供 helper |
| OpenClaw | 工具协议层固定 `text` + `attachments[]`；超长文本统一走 attachment（落 BlobStore） | 单一通道：模型只看预览，详情走 attachment fetch |
| Hermes Agent | `ToolResult.truncated: bool` + 完整原文在 SQLite session blob | 截断元数据 + 完整可恢复 |

三者共识：**budget 是工具协议层一等公民**，截断必须**有元数据 + 可恢复**。

### 1.3 现状缺口

- `harness-tool.md` 没规定阈值默认值、计量单位、以及对二进制流的处理；
- `harness-contracts.md` 的 `ToolResult` 没有 `OffloadedRef` 这种"模型可见预览 + 完整内容外放"的载体；
- `harness-journal.md` / `harness-observability.md` 没明确落盘指标。

## 2. 决策

### 2.1 总纲

把 budget 处理上升到**工具协议层契约**：所有 Tool 实现走统一的 `ResultBudget` 检查；命中阈值后由 `ToolOrchestrator` 调用 `BlobStore::put` 落盘，回填 `ToolResult` 的 `OffloadedRef`，模型仅看到 head/tail 预览 + 摘要元数据。

设计目标（按优先级）：

1. **可重放**：完整字节必须落盘，事件层引用 `BlobRef`；
2. **模型可感知**：模型上下文里必须有 "已截断 + 可获取" 的明确声明；
3. **provider 中立**：head/tail 预览作为文本块，不依赖任何模型私有特性；
4. **零静默截断**：未配置 budget 的工具一律使用全局 default budget，不存在"无声裁剪"。

### 2.2 `ResultBudget`：协议层契约

```rust
/// 单个 Tool 实例的输出预算。来源优先级（高→低）：
/// 1. ToolDescriptor.budget（声明性，注册时给出）
/// 2. SessionPolicy.tool_budget_overrides[tool_name]（运营策略覆盖）
/// 3. HarnessConfig.default_tool_budget（兜底）
#[derive(Debug, Clone)]
pub struct ResultBudget {
    /// 计量方式
    pub metric: BudgetMetric,
    /// 上限值（与 metric 同单位）
    pub limit: u64,
    /// 命中后的处理策略
    pub on_overflow: OverflowAction,
    /// head / tail 预览长度（字符），仅 OverflowAction::Offload 使用
    pub preview_head_chars: u32,
    pub preview_tail_chars: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum BudgetMetric {
    /// UTF-8 字符数（默认；与 Claude Code 对齐）
    Chars,
    /// 字节数（用于二进制流，例如 ImageContent / 文件读取）
    Bytes,
    /// 行数（用于 BashTool / FileReadTool 的行级语义）
    Lines,
}

#[derive(Debug, Clone)]
pub enum OverflowAction {
    /// 截断到 limit，附 head + tail 预览（不落盘）。仅供"原本就只取尾部"的工具，例如 TailTool。
    Truncate,
    /// 落盘 BlobStore，模型上下文写入预览 + BlobRef + 元数据声明。【默认】
    Offload,
    /// 直接返回 ToolError::ResultTooLarge（用于安全敏感场景）
    Reject,
}
```

**默认值**（写入 `HarnessConfig`）：

```yaml
default_tool_budget:
  metric: Chars
  limit: 30000           # 与 Claude Code 默认对齐
  on_overflow: Offload
  preview_head_chars: 2000
  preview_tail_chars: 2000
```

### 2.3 `ToolResultEnvelope` 协议扩展

`harness-contracts.md §3.5` 现有 `ToolResult` 是 enum（`Text` / `Structured` / `Blob` / `Mixed`），用于承载"工具产出的具体内容"。本 ADR **不修改** 该 enum 的形态，而是新增"信封"承载预算元数据，避免破坏既有 Event payload：

```rust
pub struct ToolResultEnvelope {
    pub result: ToolResult,
    pub usage: Option<UsageSnapshot>,
    pub is_error: bool,
    /// 命中 budget 时填充；为 None 表示原文未触发溢出。
    pub overflow: Option<OverflowMetadata>,
}

#[derive(Debug, Clone)]
pub struct OverflowMetadata {
    /// 完整原文落盘后的引用；模型可通过 `read_blob` 这类工具按需取回
    pub blob_ref: BlobRef,
    /// 命中前/后预览（已计入 `content`，此处仅冗余便于审计/UI）
    pub head_chars: u32,
    pub tail_chars: u32,
    /// 原始大小（与 BudgetMetric 一致的单位）
    pub original_size: u64,
    pub original_metric: BudgetMetric,
    /// budget 上限值（命中时的瞬时配置）
    pub effective_limit: u64,
}
```

### 2.4 `ToolOrchestrator` 处理流水线

```text
Tool::execute(ctx, args) → ToolStream
  ↓
Orchestrator::collect_with_budget(stream, budget) → ToolResultEnvelope
  ├─ 在 limit 内：直接产出 ToolResultEnvelope { result, overflow: None, .. }
  └─ 超限：
       1. 同步收集 head + tail 预览（不需要 buffer 全文）
       2. 把流剩余字节交给 BlobStore::put_streaming() 落盘
       3. 拼接预览 + 引导文本 + BlobRef → ToolResult::Mixed
       4. 填充 envelope.overflow = Some(OverflowMetadata { ... })
       5. 发出 Event::ToolResultOffloaded { tool_use_id, blob_ref, metric, original_size, effective_limit }
       6. usage.bytes_offloaded += original_size  // 计费/观测
```

**关键约束**：

- **流式收集**：Orchestrator 必须支持 streaming budget check，避免 buffer 整段 GB 级输出；
- **不阻塞 Run**：BlobStore 落盘失败时降级为 `OverflowAction::Reject` + 触发 `ToolError::OffloadFailed`，不能静默丢数据；
- **预览拼装格式**（写入 `MessagePart::Text`，作为 prompt 契约）：

```text
<tool_result_head chars="2000">
... 头 2000 字符原文 ...
</tool_result_head>

<tool_result_offloaded
  blob_id="01H..."
  metric="chars"
  original_size="125234"
  effective_limit="30000"
  hint="output exceeds budget; head/tail attached, full content stored at the blob_ref above"
/>

<tool_result_tail chars="2000">
... 尾 2000 字符原文 ...
</tool_result_tail>
```

### 2.5 内置 `read_blob` 工具

为了让模型能"按需"取回 offloaded 内容，新增**内置 AlwaysLoad** 工具 `read_blob`：

```rust
// 工具骨架；详细 schema 见 harness-tool.md §6
struct ReadBlobTool;
// args: { blob_id, range?: { start: u64, end: u64 }, metric: BudgetMetric }
// 返回：MessagePart::Text 或 MessagePart::Binary
// 安全：仅允许读取本 Run 内 ToolResultOffloaded 产生的 blob_id（通过 DiscoveredToolProjection 校验）
```

这给了模型一个明确的"读全文"出口，避免反复重试相同命令导致的 token 浪费。

### 2.6 与 ADR-009 的关系

- ADR-009 解决"工具 schema 太多"，ADR-010 解决"工具结果太大"，二者**正交**；
- 但二者共享 BlobStore 与 Journal 投影：`DiscoveredToolProjection` 与本 ADR 的 `OffloadedBlobProjection` 一起构成 Run 级的"延迟资源池"；
- 当模型用 `read_blob` 读取 offloaded 内容时，结果**仍然受同一 ResultBudget 二次约束**（防止递归读取爆掉上下文）。

### 2.7 与 ADR-003 的关系

ADR-010 的 `ResultBudget` 是**运行期能力**，不影响 system prompt / toolset 冻结；落盘事件只走 Event 流，**不会**回写 system prompt。Prompt cache 不受影响。

## 3. 不在本 ADR 范围

- **二进制流 chunking 协议**（例如视频流）：保留为后续 ADR 题目；
- **BlobStore 跨租户共享**：参见 `harness-contracts.md §3.7` 的 `BlobRetention`，本 ADR 不重复定义；
- **Tool 输入 budget**（限制模型给工具的 args）：另议；当前由 `ToolDescriptor::input_size_limit` 单独承载。

## 4. 参考 / 证据

> 编号约定与 `docs/architecture/reference-analysis/evidence-index.md` 对齐（HER-* / OC-* / CC-*）。

| Evidence ID | 来源 | 引用片段 |
|---|---|---|
| CC-33 | `reference-analysis/evidence-index.md` L148 · `claude-code-sourcemap-main/restored-src/src/Tool.ts:58-62, 456-466` | `maxResultSizeChars` per tool；超限 preview + `ContentReplacementState` 落盘；`Infinity` 例外（Read 自限） |
| CC-32 | `reference-analysis/evidence-index.md` L147 · `restored-src/src/services/compact/*.ts`；`src/query.ts:365-426` | 上下文压缩流水线按 `tool-result-budget → snip → microcompact → collapse → autocompact` 序行，预算裁剪优先于压缩 |
| OC-21 | `reference-analysis/evidence-index.md` L89 · `openclaw-main/docs/plugins/architecture.md` §"Channel plugins and the shared message tool" / §"Message tool schemas" | `MessagePresentation` 统一抽象（text / context / divider / buttons / select），附件/溢出内容走独立通道而非结果正文 |
| HER-005 | `reference-analysis/evidence-index.md` L11 · `hermes-agent-main/tools/registry.py` | `ToolRegistry.register(..., max_result_size_chars)`：注册期即声明结果预算 |
| Internal | `harness-contracts.md §3.7` | `BlobRef` / `BlobStore` 已定义，本 ADR 仅叠加 `BlobRetention` 约束 |

## 5. 落地清单

| 项 | 责任 crate | 说明 |
|---|---|---|
| `ResultBudget` / `BudgetMetric` / `OverflowAction` / `OverflowMetadata` | `harness-contracts` §3.4 | 共享枚举/结构落 contracts |
| `ToolResultEnvelope` 新增 | `harness-contracts` §3.5 | 包裹现有 `ToolResult` enum，向后兼容（overflow 默认 None） |
| `Event::ToolResultOffloaded` | `harness-contracts` Event 枚举 + `event-schema.md §3.5` | 新增 variant |
| `Event::ToolUseHeartbeat` | `harness-contracts` Event 枚举 | 新增 variant，配合 long-running 心跳（harness-tool §2.7） |
| `Event::ToolRegistrationShadowed` | `harness-contracts` Event 枚举 | 新增 variant，记录 Registry 裁决结果（harness-tool §2.5.1） |
| `ToolOrchestrator::collect_with_budget` | `harness-tool` | 流式 budget 检查 + Blob 落盘粘合 |
| `read_blob` 内置工具 | `harness-tool` §6 | AlwaysLoad，受 `OffloadedBlobProjection` 安全校验 |
| `OffloadedBlobProjection` | `harness-journal` | Run 级投影：`Map<BlobId, OverflowMetadata>` |
| 观测指标 | `harness-observability` | `octopus.tool.bytes_offloaded` / `octopus.tool.budget_hit_rate` |
| 默认配置 | `HarnessConfig.default_tool_budget` | 写入默认配置文件模板 |
