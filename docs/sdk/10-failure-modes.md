# 10 · 生产级失败模式与缓解策略（Failure Modes）

本章把前 9 章的警示汇成一份**可在代码评审中对照**的 checklist。每条都标注出处。

## 10.1 Harness 层

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| H1 | **Kitchen-sink session** | 一次会话被塞进十几个不相干的任务 | 主动 `/clear` 或开新 session；单会话单目标 | [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Kitchen sink sessions" |
| H2 | **Over-specified CLAUDE.md** | CLAUDE.md 超过 500 行、模型表现退化 | Goldilocks Zone；保留"为什么"、删掉"怎么做" | 同上 §"Avoid failure patterns" |
| H3 | **"Kind of works" first pass** | 模型第一次写的代码"看起来对"但边界错 | 评估 loop / Evaluator subagent / 测试 | [Harness design](https://www.anthropic.com/engineering/harness-design-long-running-apps) |
| H4 | **过度重构** | 改了大量无关代码 | `NOTES.md` 列禁区；Permission 禁用特定路径 | [Effective harnesses](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) |
| H5 | **Harness staleness** | 新模型不需要的旧补偿仍在运行 | Feature flag 化每个补偿；周期性删除 | [Managed Agents](https://www.anthropic.com/engineering/managed-agents) |
| H6 | **硬编码模型专属 workaround** | 新模型表现更差 | 可开关；标注失效的模型版本 | 同上 |

## 10.2 Context / 记忆

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| C1 | **Prompt cache miss** | 成本飙升、延迟变差 | 工具排序确定；不改历史 turn；compaction 在尾部 | [Prompt caching docs](https://docs.claude.com/en/docs/build-with-claude/prompt-caching) ; Hermes AGENTS.md §Prompt Caching; OpenClaw CLAUDE.md |
| C2 | **Context rot** | 模型越长表现越差 | 积极 compact、JIT 加载、subagent 隔离 | [Chroma · Context Rot](https://research.trychroma.com/context-rot) |
| C3 | **Context anxiety（早停）** | 模型接近窗口上限就 "end_turn" | 升级模型（Opus 4.5+）或强制 context reset | [Harness design](https://www.anthropic.com/engineering/harness-design-long-running-apps) |
| C4 | **Compaction 丢决策** | 摘要后关键设计决策没了 | `PreCompact` hook 把决策冻入 `NOTES.md` | [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) |
| C5 | **Upfront RAG overfill** | 预检索塞满窗口但精度下降 | JIT 工具检索为主，RAG 为辅 | 同上 |

## 10.3 Tools / ACI

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| T1 | **工具太多** | 50+ 工具 → 模型迷失、token 被挤 | 合并、`tool_search` meta-tool、code execution with MCP | [Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents); [Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) |
| T2 | **错误信息无法复用** | 模型在错误后瞎试 | `remediation` 字段 + 例子 | [Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) §"Prompt-engineer error messages" |
| T3 | **大输出撑爆窗口** | 一次 `cat` 文件就 OOM | 默认截断 + pagination + `response_format` | 同上 §"Truncate and paginate" |
| T4 | **并发写冲突** | 两个写工具同时改同一文件 | `isConcurrencySafe=false` 串行 | Claude Code `toolOrchestration.ts` |
| T5 | **隐藏参数** | description 没讲清 side-effect | description 明示 + examples | [Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) |
| T6 | **工具名冲突** | `search` 来自多个 server | 命名空间 `mcp__<server>__search` | 同上 §"Namespace your tools" |

## 10.4 Permissions / Sandbox

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| S1 | **Approval fatigue** | 用户点 YES 点到麻木 | Autonomy dial + Auto Mode + Allowlist | [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) |
| S2 | **只隔离 FS 不限制网络** | 数据外泄 | 双隔离；egress allowlist | [Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) |
| S3 | **凭据进入沙箱** | 模型读到 .env / token | 凭据注入代理；env 清空；vault | [Managed Agents](https://www.anthropic.com/engineering/managed-agents) |
| S4 | **Sandbox 崩 → Brain 崩** | 一次工具故障 session 挂 | sandbox crash = tool error | 同上 |
| S5 | **Hook 能绕过 deny** | 审批失效 | hooks 只能更严 | Claude Code `hooks/toolPermission` |

## 10.5 Multi-agent

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| M1 | **过度委派** | 简单问题也 spawn 10 个 subagent | Effort scaling；lead 判断后再派 | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) |
| M2 | **Generator 自评** | 自我安慰式通过 | 独立 Evaluator agent + 外部验证 | [Harness design](https://www.anthropic.com/engineering/harness-design-long-running-apps) |
| M3 | **Subagent 输出过大** | 父窗口被塞爆 | 大输出走 file ref | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) Appendix |
| M4 | **Subagent 互相污染** | 多个子代理同步一个 bad 结论 | 隔离上下文；不允许兄弟直连 | 同上 |
| M5 | **无 trace_id** | emergent bug 无法复现 | 强制 trace_id + 完整事件流 | 同上 §"Debugging" |

## 10.6 Long-horizon

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| L1 | **过早收工** | 说完成但 feature 缺失 | Feature list + Stop hook | [Effective harnesses](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) |
| L2 | **Coding Agent 找不着北** | 跨 session 后不知上下文 | 强制读 `NOTES.md` + `runtime/notes/<session>.md`（会话级进度快照，08 §8.4.1 / §8.9） | 同上 |
| L3 | **循环自检** | 一直在 lint fix 不前进 | Budget + "本轮是否有实质进展" hook | 综合 |
| L4 | **过度 compaction 后丢身份** | 模型"失忆" | 关键决策冻入 NOTES；保留尾部 5–10 turns | [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) |
| L5 | **无 incremental commit** | 回滚必须 redo 全部 | 每步 commit；失败尝试 stash | [Effective harnesses](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) |

## 10.7 Observability / Ops

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| O1 | **有状态 agent 热升级** | 升级时会话崩坏 | Rainbow deployment | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) |
| O2 | **全量生产数据进日志** | 泄露敏感内容 | 结构化决策日志（去隐私） | 同上 |
| O3 | **只看成功率不看成本** | 成本悄悄飙升 | 面板同时看 pass rate + token/session | 综合 |
| O4 | **没有 replay** | 失败无法复现 | 全量事件 + replay UI | §9.4 |
| O5 | **评测集合成数据** | 真实场景分歧大 | 20 个真实任务起步 | [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) §"Effective evaluation" |

## 10.8 Octopus 特有

| # | 失败 | 症状 | 缓解 | 来源 |
|---|---|---|---|---|
| X1 | **Runtime config 放进 main.db** | 成本不透明、违反治理 | 配置以文件为主 | 本仓 `AGENTS.md` §Runtime Config |
| X2 | **Session 在活跃时响应 config 热改** | 行为漂移 | Session 绑定 config snapshot | 同上 §Live session behavior |
| X3 | **Host 之间接口不一致** | Tauri 和 Browser 行为差 | 公共 adapter contract | 同上 §Host consistency rule |
| X4 | **Business 页直 fetch 绕 adapter** | 契约失真 | 强制走 `tauri/shell.ts` / `tauri/workspace-client.ts` | 同上 §Request Contract Governance |
| X5 | **UI 绕过 design system** | 视觉割裂 | 走 `@octopus/ui`；设计语言在 `docs/design/DESIGN.md` | 同上 §Frontend Governance |

---

## 10.9 代码评审清单（Code Review Checklist）

每次 PR 请对照：

- [ ] 没引入 H1–H6（harness staleness 检查）
- [ ] 没破坏 Prompt cache 稳定性（工具顺序、历史 turn、compaction 位置）
- [ ] 新工具有 description、examples、错误 remediation、响应截断
- [ ] 新工具声明 `isConcurrencySafe`
- [ ] 新工具落事件流；非只读工具走权限
- [ ] 新 hook 有超时；不改已发生的 turn
- [ ] Subagent 有工具白名单、budget、输出格式约束
- [ ] Long-task 更新了 `runtime/notes/<session>.md` 与（必要时）`NOTES.md`
- [ ] 新 config 走 `config/runtime/*` 分层而非 DB
- [ ] 新 API 走 `contracts/openapi/`，不绕过 adapter

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| 全部前 9 章所引 | 本章为汇总性质；参见各章末尾 "参考来源汇总" |
| 本仓 `AGENTS.md` | Octopus 特有约束来源 |
