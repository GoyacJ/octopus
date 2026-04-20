# 09 · 可观测性与评测（Observability & Evaluation）

> "Start evaluating immediately with small samples... LLM-as-judge evaluation scales..."
> — [Anthropic · Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)

非确定系统的质量保障必须建立在**系统的观测 + 评测**之上。

## 9.1 核心断言

- **Agent 是非确定的**：同一 prompt 多次运行可能走不同路径。
- **没有评测就没有改进**。
- **端到端评估优于逐步评估**：允许多条合理路径抵达同一目标。

## 9.2 最小可观测性（MVO：Minimum Viable Observability）

### 9.2.1 事件级 tracing

每个 tool_use 必须采集：

| 字段 | 含义 |
|---|---|
| `event_id` | 全局唯一（ULID） |
| `session_id` | 所属会话 |
| `trace_id` / `span_id` / `parent_span_id` | 跨父子代理串联 |
| `agent_role` | `lead` / `sub.research` / `sub.eval` / ... |
| `tool_name` | 工具名 |
| `input_hash` | 输入哈希（用于缓存命中统计） |
| `duration_ms` | 执行时长 |
| `tokens_in` / `tokens_out` | 大致字数 |
| `permission_decision` | allow / ask / deny |
| `error.type` | 若失败 |
| `model_id` + `model_version` | 方便回溯行为 |
| `config_snapshot_id` | 对应哪个配置 |

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Debugging benefits from new observability"；本仓 `AGENTS.md` §Persistence Governance（append-only 审计流）。

### 9.2.2 会话级摘要

每个 session 结束时生成：

- 总 turn 数 / 总 token
- 工具调用分布直方图
- 失败次数 / 类型分布
- 权限 ask 次数（用户打断频率）
- 是否触发 compaction / 次数
- 产生的 subagent 树
- final status: done / aborted / errored

### 9.2.3 交互决策树（Interaction map）

对**脱敏**的决策路径做统计：

```
{lead} → plan → spawn 3 research_sub → (parallel) → synthesize → end_turn
                                              ↓
                                   [1 failed, retried]
```

这样即便不看内容也能发现**结构性问题**（例：某类任务总是 retry 2 次）。

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Debugging"。

## 9.3 评测（Evaluation）

### 9.3.1 启动样本：宁小勿等

> "Do not wait for a big test set."

20 组真实任务 > 200 组合成任务。

- 人工标注"金标准答案 or 通过标准"
- 覆盖 happy path + 3 种失败路径

### 9.3.2 LLM-as-Judge

一个"评判"模型读 agent 的最终输出 + 金标准，给分：

- **单次调用**打出 `score: 0.0–1.0` + `pass/fail`
- 优于"把若干子指标分别调 LLM 打分"（成本 / 方差）
- 对主观任务（研究综合、UI 设计）尤其有用

示例评分 rubric：

```
Rate 0.0 to 1.0 + pass/fail:
- Factual accuracy
- Citation quality
- Completeness w.r.t. task
- No hallucinated tool calls
```

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Effective evaluation of agents"。

### 9.3.3 人工评估补位

LLM-as-judge 会漏：

- 刻意模式（如每次总选第一个结果）
- 安全相关的微妙偏差
- 与真实业务契合度

建议：**10–20% 流量做人工评估**。

### 9.3.4 端到端 vs 逐步

| 维度 | 端到端 | 逐步 |
|---|---|---|
| **适用** | 大多数业务 | 步步可验证的任务（数学、代码） |
| **优点** | 允许多路径、真实 | 精准定位失败 |
| **缺点** | 难归因 | 过度约束、模型不自由 |

**规范默认**：**端到端 + 关键节点断言**。详见 Anthropic "End-state evaluation for state-mutating agents"。

> 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"End-state evaluation"；[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) §"Evaluating tool usage"。

## 9.4 Replay / 回放

### 9.4.1 形式

- 全量事件 → 重建"假 Brain"（不真调模型）
- 时间轴 UI：每个 tool_use 展开 input/output
- 可分支：在某个事件点"what-if 换个决策"

### 9.4.2 用途

- 调试 emergent behavior
- 退步评测（新模型跑老 session → 观察分歧）
- 培训（给新人/新 agent 看范例）

## 9.5 Metrics Dashboard（推荐面板）

### 9.5.1 Cost / Usage

- 日/会话 token 消耗
- Prompt cache hit ratio（目标 ≥ 80%）
- 工具调用 / turn ratio

### 9.5.2 Quality

- 评测 pass rate（按任务类型切片）
- 平均 turn 数（过长 = 效率差）
- 用户中断率 / 重启率
- 评测分数分布

### 9.5.3 Reliability

- Tool 失败率（按工具名）
- Sandbox 重建率
- 模型 API 错误率 / fallback 触发率
- Prompt-too-long 次数

### 9.5.4 Safety

- Permission deny 次数
- Hook block 次数
- Strip dangerous 触发次数
- 超 budget 次数

## 9.6 Production-ready 基础设施

### 9.6.1 Retry 与幂等

- Session 事件写入幂等（event_id 去重）
- Tool 调用层面：由工具自己决定幂等性；`isConcurrencySafe` 为真的工具通常幂等

### 9.6.2 Rainbow Deployment

- 有状态 agent 不能 hot-swap（见 §4.2）
- 建议至少保留两代 Brain 镜像并行

### 9.6.3 Telemetry Backend

- 开放格式 OpenTelemetry：tracing + metrics + logs
- 事件 JSONL 可 ship 到 object storage（对象级归档）
- SQLite 投影 + FTS 保本地 UI 搜索

### 9.6.4 SLO 建议

| 指标 | SLO 建议 |
|---|---|
| 平均 turn latency | p50 < 10s, p95 < 45s |
| TTFT（首 token） | p50 < 2s |
| Tool 失败率 | < 3%（不含模型主动放弃） |
| Sandbox 崩溃率 | < 0.5% |
| Session resume 成功率 | > 99% |

## 9.7 对 Octopus 的落地约束

- 所有事件写 `runtime/events/*.jsonl` + SQLite 投影（符合本仓持久化治理）
- Telemetry 默认集成到 Octopus 现有 audit logs / trace 系统（`logs/`）
- 评测用例放 `docs/evals/` 或 `tests/evals/`，自动化跑 CI
- Replay UI 复用 Octopus 的 `UiTraceBlock` 组件（见本仓 AGENTS.md §Shared UI Component Catalog）

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | 评测 / 可观测性 / debugging 的官方经验 |
| [Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) | 工具评测方法 |
| [Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents) | Evaluator-Optimizer workflow |
| 本仓 `AGENTS.md` §Persistence Governance | 审计流 / 事件 JSONL 约束 |
| Claude Code restored src `services/analytics/*` | 事件埋点参考 |
| OpenTelemetry 官方规范 | 开放格式 tracing |
