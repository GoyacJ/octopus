# ADR-001 · 采用 Event-Sourcing 作为状态管理模型

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：`harness-contracts` / `harness-journal` / `harness-session` / `harness-observability` / 所有业务层消费者

## 1. 背景与问题

Octopus Harness SDK 承载多 Agent、多 Session、多租户的复杂运行时状态。需要一个能满足以下需求的状态模型：

- **审计**：能回答"这个决策何时做出、谁做的、基于什么输入？"
- **调试**：给定失败 session，能重放到任意节点定位问题
- **测试**：Golden replay 作为回归基线
- **持久化**：容忍进程崩溃、跨重启恢复
- **并发**：多消费者（业务 UI、审计服务、指标聚合）同时读而不互相干扰

参考项目对比（证据见 `reference-analysis/`）：

| 项目 | 状态模型 | 缺陷 |
|---|---|---|
| Claude Code | 内存 projection + `runtime/events/*.jsonl` append-only | Event 不是一等公民，场景专用 |
| Hermes | SQLite 投影 + 线程状态 | 状态修改无追溯链 |
| OpenClaw | Append-only JSONL + 无回放（OC-05） | **明确放弃**事件回放，调试困难 |

## 2. 决策

**采用 Event-Sourcing 作为 SDK 的唯一状态模型**：

1. 所有状态变化先以 `Event` 写入 Append-Only Journal
2. 当前状态由 `Projection::replay(events)` 派生
3. Event 结构使用 `#[non_exhaustive]`，允许未来扩展变体
4. 每个 Event 携带 `run_id / session_id / tenant_id / correlation_id / causation_id`
5. Journal 提供 `read(cursor) -> Stream<Event>` 与 `snapshot()` 以加速重建

## 3. 替代方案

### 3.1 A：仅 CRUD 模型（只存当前状态）

- ❌ 失去历史；审计要重建成本高
- ❌ 测试困难：无法 Golden replay
- ❌ 难以多消费者并发读

### 3.2 B：双写（CRUD + Event 日志）

- ❌ 两份真相；不一致风险
- ❌ 违反 Single Source of Truth

### 3.3 C：Event-Sourcing（采纳）

- ✅ 单一真相源
- ✅ 任意时点重建
- ✅ 多消费者自然支持（每消费者独立 cursor）
- ✅ 与 Octopus `runtime/events/*.jsonl` 规范天然兼容
- ⚠ Projection 重建成本；Snapshot 用于提速

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| 存储体积 | Event 比当前状态大 | Compact Event / Blob offload / 定期修整（`maybe_auto_prune_and_vacuum` 对齐 HER-024） |
| Replay 速度 | 长 session 重建慢 | Snapshot 快照 + 增量重放 |
| 迁移难度 | Event 结构变更需版本兼容 | `#[non_exhaustive]` + 版本字段 + 迁移器 |
| 非确定性 | 辅助 LLM 摘要不可复现 | 把摘要结果作为 Event 固化（`CompactionApplied.summary_ref`，schema 见 `event-schema.md §3.8`） |

## 5. 后果

### 5.1 正面

- 审计无死角
- 调试友好：`ReplayEngine::replay(session_id, cursor)`
- 多端一致性：UI 通过 EventStream 自渲染
- 对齐 Octopus 既有 `runtime/events/*.jsonl` 治理规则

### 5.2 负面

- 学习曲线：团队需接受"不直接改状态"的思维
- 存储：需定期 vacuum
- 查询复杂：当前状态查询需重建 projection 或走索引表

## 6. 实现指引

- **必读**：`event-schema.md`（D4）
- **核心 crate**：`harness-journal` / `harness-contracts::Event`
- **CI 校验**：`cargo test --features replay-compat` 必须跑 Golden replay 集
- **业务层**：订阅 `EventStream` 自渲染；不得直接写 `state.db`

## 7. 待办事项

- [ ] 对齐 `runtime/events/*.jsonl` 的字段命名（业务层治理文件）
- [ ] 提供 `Projection::from_events` 的辅助宏，减少样板代码
- [ ] 定义 Event schema 版本升级的迁移流程
