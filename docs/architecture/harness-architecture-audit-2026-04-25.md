# Harness Architecture Documentation Audit

> 日期：2026-04-25
> 范围：`docs/architecture/harness/**`、`docs/architecture/reference-analysis/**`
> 输出性质：架构文档审计结果。本文不包含开发计划、MVP、排期或实现任务拆分。

## 1. 审计结论

当前 `docs/architecture/harness` 文档集已经覆盖 Agent Harness 的主要架构面：模型、事件、权限、沙箱、工具、MCP、Hook、Skill、Memory、Session、Engine、Subagent、Team、Plugin、Observability、SDK 门面。

但它还不能作为可直接实现的稳定基线。

原因不是方向错误，而是**权威契约漂移**：

- D3 `api-contracts.md` 声称是 trait 签名单一事实源，但已落后于 v1.3、v1.4、v1.8 的核心变更。
- crate 数量、层级矩阵、总览、ADR-008 之间对 `harness-tool-search` 的归属不一致。
- hot reload 的 API、Event、ADR 表达不闭合，影响 replay 与 cache 审计。
- `TrustLevel`、`ToolOrigin`、`ToolDescriptor` 的信任建模存在不可编译的枚举用法。
- EventStore 与仓库既有持久化治理存在语义冲突。
- 证据 ID 存在断链，违反 harness README 自己规定的证据治理。

审计判定：

| 维度 | 判定 |
|---|---|
| 架构方向 | 可保留 |
| 文档作为实现基线 | 不通过 |
| 文档作为评审基线 | 有条件可用 |
| 主要风险 | 公开 API、事件 schema、存储真相源、信任模型 |
| 优先处理对象 | D3、D1/D2、D4、`harness-contracts`、`harness-tool`、`harness-journal` |

## 2. 检查范围

本次覆盖：

- `docs/architecture/harness/*.md`：11 个主文档，8521 行
- `docs/architecture/harness/crates/*.md`：19 个 crate SPEC，17137 行
- `docs/architecture/harness/adr/*.md`：18 个 ADR，3986 行
- `docs/architecture/reference-analysis/*.md`：5 个参考分析文档，1476 行

机器校验结果：

| 检查项 | 结果 |
|---|---|
| 本地 Markdown 链接 | 0 个断链 |
| `evidence-index.md` 已定义证据 ID | 129 个 |
| harness 文档实际引用证据 ID | 90 个 |
| harness 引用但未定义的证据 ID | 5 个 |
| 实际 crate SPEC 文件数 | 19 个 |
| 实际 ADR 文件数 | 18 个 |

## 3. P0 问题

### P0-1 D3 接口契约不是事实源

`api-contracts.md` 第 4 行声明自己是 trait 签名单一事实源，但核心接口已明显落后。

证据：

- `api-contracts.md:343-360` 仍定义 `Tool::invoke(...) -> ToolResult`。
- `harness-tool.md:72-82` 已改为 `Tool::execute(...) -> Result<ToolStream, ToolError>`。
- `README.md:246` 明确写出 `Tool::invoke -> execute`。
- `api-contracts.md:307-323` 仍把 `MemoryProvider` 写成单 trait。
- `harness-memory.md:32-122` 已拆成 `MemoryStore + MemoryLifecycle`，`MemoryProvider` 只是 blanket trait。
- `api-contracts.md:542-570` 的 Session 只补了 Steering API，缺 `create/run/fork/reload_with` 等公开面。
- D3 没有 `harness-tool-search` 独立章节。

影响：

- 任何按 D3 实现的下游都会写出旧版 Tool 和 Memory API。
- v1.3、v1.4、v1.8 的变更无法从“单一事实源”得到。
- `module-boundaries.md:148-150` 要求新增 trait / 公开类型同步 D3，这条治理已被破坏。

审计判定：

D3 必须重新成为公开 API 的权威入口，或者降级为索引文档。两者只能选一个。现在的状态最危险：名义上权威，内容上滞后。

### P0-2 crate 拓扑出现 18/19 分裂

文档集对 crate 数量和 L2 层成员给出两套答案。

证据：

- `README.md:22` 和 `README.md:67` 声明 19 个 crate。
- `adr/0008-crate-layout.md:51-69` 声明 19 个 crate，并把 `octopus-harness-tool-search` 放入 L2。
- `overview.md:174-197` 标题是 18 个 Crate，总表缺 `harness-tool-search`。
- `module-boundaries.md:8` 声明 18 个 crate。
- `module-boundaries.md:16` 把 L2 写成 `C1..C6`，缺 `harness-tool-search`。
- `module-boundaries.md:41-48` 的 L2 依赖白名单也缺 `harness-tool-search`。

影响：

- 依赖矩阵无法约束实际 19 crate。
- `harness-tool-search` 的依赖边界只能从 ADR-008 和自身 SPEC 推断，D2 没有给出 CI/评审可执行的规则。
- `overview.md` 作为新成员入口，会把架构规模讲错。

审计判定：

ADR-008 与实际文件系统已经指向 19 crate。D1/D2/D3 必须与 19 crate 对齐。

### P0-3 hot reload 事件契约不闭合

hot reload 同时涉及 Prompt Cache、Session、Event Replay。当前表达不一致。

证据：

- `harness-contracts.md:128-129` 有 `SessionReloadRequested` 和 `SessionReloadApplied` 两个 Event variant。
- `harness-session.md:450` 的生命周期图也写了 `SessionReloadRequested -> SessionReloadApplied`。
- `event-schema.md:20` 的 Session 生命周期总览只列 `SessionReloadApplied`。
- `event-schema.md` 没有 `SessionReloadRequestedEvent` 字段定义。
- `adr/0003-prompt-cache-locked.md:167` 要求 `SessionReloadApplied { session_id, mode, cache_impact, effective_from, at }`。
- `harness-session.md:156-160` 的 `ReloadOutcome` 也有独立 `cache_impact` 字段。
- `event-schema.md:1214-1220` 的 `SessionReloadAppliedEvent` 没有 `cache_impact`。
- `adr/0003-prompt-cache-locked.md:52-59` 把 `cache_impact` 放进 `ReloadMode` variant。
- `harness-session.md:163-167` 又把 `ReloadMode` 写成不含 `cache_impact` 的 enum。

影响：

- replay 时无法只看 Event 判断一次 reload 是否破坏 prompt cache。
- 审计无法区分 `NoInvalidation`、`OneShotInvalidation`、`FullReset`。
- API 返回值、ADR、Event schema 三者无法互相生成。

审计判定：

hot reload 的权威模型必须收敛到一处。`SessionReloadRequestedEvent` 不能只存在于 enum 和流程图里。`cache_impact` 必须进入可回放事件。

### P0-4 信任模型存在不可编译枚举用法

`TrustLevel` 只定义了插件信任域，但 Tool 描述子把 built-in 当成 TrustLevel variant 使用。

证据：

- `harness-contracts.md:310-314` 的 `TrustLevel` 只有 `AdminTrusted / UserControlled`。
- `harness-contracts.md:418-422` 的 `ToolOrigin` 才有 `Builtin`。
- `harness-tool.md:167` 的 `ToolDescriptor` 有 `trust_level: TrustLevel`。
- `harness-tool.md:796-798` 写 `origin: ToolOrigin::Builtin` 和 `trust_level: TrustLevel::Builtin`。
- `harness-tool-search.md:96-99` 同样使用 `TrustLevel::Builtin`。
- `security-trust.md:573` 和 `adr/0016-programmatic-tool-calling.md:257` 使用 `trust ∈ {Builtin, Plugin{AdminTrusted}}` 这种混合表达。

同一片段还有字段名漂移：

- `ToolDescriptor` 定义 `display_name`，但 `harness-tool.md:794` 和 `adr/0016-programmatic-tool-calling.md:97` 使用 `title`。
- `ToolProperties` 定义 `is_read_only`，但 `harness-tool.md:800` 和 `adr/0016-programmatic-tool-calling.md:104` 使用 `is_readonly`。
- `required_capabilities` 定义为 `Vec<ToolCapability>`，但 `harness-tool.md:804` 和 `adr/0016-programmatic-tool-calling.md:109` 使用 `bitflags![...]`。

影响：

- SPEC 里的关键示例不可编译。
- built-in 来源、admin trusted 插件、user controlled 插件被混在同一维度，后续权限矩阵会继续漂移。
- `execute_code` 的准入条件无法被类型系统准确表达。

审计判定：

`origin` 和 `trust` 必须分维度建模。`Builtin` 如果是来源，就不应出现在 `TrustLevel` 里；如果是信任级别，就必须进入 `TrustLevel` 并重写 ADR-006 的二分语义。

### P0-5 EventStore 与仓库持久化治理冲突

Octopus 根治理规定事件流是 `runtime/events/*.jsonl`，`data/main.db` 保存结构化状态和投影。harness 文档把 SQLite 同时写成 EventStore。

证据：

- 根 `AGENTS.md` 的 Persistence Governance 规定：`runtime/events/*.jsonl` 是 append-only event/audit streams；`data/main.db` 是结构化数据库，存 queryable state and projections。
- `overview.md:507-508` 映射为：JSONL 写 runtime events，SQLite 写 projection。
- `overview.md:429` 示例却使用 `.with_store(SqliteEventStore::open("data/main.db"))`。
- `harness-journal.md:570-610` 的 `SqliteEventStore` 创建 `events` 表，并把 event body 作为权威来源。
- `harness-journal.md:903-905` 示例同样打开 `data/main.db` 作为 event store。

影响：

- 同一个 session 的事件真相源可能同时存在 JSONL 和 SQLite 两套。
- `data/main.db` 的职责从 projection 扩张成 event journal，违反当前仓库持久化分层。
- 默认 feature 同时启用 `sqlite-store` 和 `jsonl-store`，但文档没有定义 Octopus 产品内的主从关系、双写一致性、故障恢复语义。

审计判定：

作为通用 SDK，`SqliteEventStore` 可以存在。作为 Octopus 产品集成基线，必须明确 JSONL 与 SQLite 的权威关系。当前文档把两种语义混在一起。

## 4. P1 问题

### P1-1 证据 ID 断链

`README.md:91-94` 要求所有设计主张引用 `evidence-index.md` 中的 ID。当前存在未定义 ID。

机器校验：

| 未定义 ID | 出现位置 |
|---|---|
| `CC-RES-1` | `crates/harness-tool.md:1162` |
| `OC-CAP-1` | `crates/harness-tool.md:1162` |
| `OC-ATT-1` | `crates/harness-tool.md:1162` |
| `HER-TRUNC-1` | `crates/harness-tool.md:1162` |
| `HER-AGENTTOOLS-1` | `crates/harness-tool.md:1162` |

另一个证据质量问题：

- `reference-analysis/evidence-index.md:21` 已有 `HER-015`，主题就是 PTC `execute_code`。
- `comparison-matrix.md` 的 R-18 使用 `HER-015` 作为 Programmatic Tool Calling 的主证据。
- `adr/0016-programmatic-tool-calling.md:413-416` 的 Evidence 表没有引用 `HER-015`，而是用 `HER-008 / HER-014 / CC-32 / OC-21` 支撑。

影响：

- 证据索引无法闭环。
- 评审者不能从 SPEC 反查参考实现事实。
- ADR-0016 没有引用最直接的 PTC 证据。

审计判定：

证据 ID 必须只来自 `evidence-index.md`。历史临时 ID 不应留在 Accepted SPEC 里。

### P1-2 Event 总览与实际事件集不同步

D4 的 Event 总览没有反映全部 Accepted 事件。

证据：

- `harness-contracts.md:124-148` 包含 `SessionReloadRequested`、`ToolUseHeartbeat`、`ToolResultOffloaded`、`ToolRegistrationShadowed`。
- `event-schema.md:20-23` 的总览没有列出这些新增事件。
- `README.md:270` 声明 v1.3.1 已补 `ToolUseHeartbeatEvent / ToolResultOffloadedEvent / ToolRegistrationShadowedEvent`。

影响：

- 总览不能用于审计覆盖检查。
- 新增事件容易遗漏 never-drop、projection、migration、redaction 规则。

审计判定：

D4 的总览表不能只停留在代表变体。Accepted Event enum 应有可机械对照的完整清单，至少要能和 `harness-contracts::Event` 一一校验。

### P1-3 `execute_code` 的审计语义与 EventStream 丢弃策略需要重新定界

ADR-0016 说内嵌工具调用仍走完整 Orchestrator 和 Permission 链，D4 又把 `ExecuteCodeStepInvoked` 放入默认可丢候选。

证据：

- `adr/0016-programmatic-tool-calling.md:52-54` 要求不绕过 Permission Broker，默认只读嵌入工具。
- `event-schema.md:387-391` 把 `ExecuteCodeStepInvokedEvent` 定义为一次嵌入式工具调用的审计事件，并说明各介入点会写 Hook / Permission 事件。
- `event-schema.md:2288-2289` 区分 Journal 不可丢、EventStream 可按策略丢。
- `event-schema.md:2309` 把 `ExecuteCodeStepInvoked` 放入默认可丢事件候选。

影响：

- 如果“审计”仅指 Journal，则文档应明说 EventStream 丢弃不影响审计完整性。
- 如果 SIEM / UI 依赖 EventStream 做实时审计，则默认可丢会漏掉 PTC 内嵌步骤。

审计判定：

需要明确 `ExecuteCodeStepInvoked` 的审计等级。至少要区分：Journal 必写、live stream 可采样、合规订阅不可丢。

### P1-4 D2 的依赖矩阵无法表达 v1.8 的实际依赖

`harness-tool-search` 需要依赖 `harness-tool` 和 `harness-model`，但 D2 没有这个 crate。D2 同时禁止 L2 同层具体实现依赖，ADR-008 又允许该依赖。

证据：

- `module-boundaries.md:16` L2 不含 `harness-tool-search`。
- `module-boundaries.md:81` 禁止 L2 crate 直接 use 另一个 L2 具体实现，除非 trait。
- `adr/0008-crate-layout.md:79-80` 明确 `harness-tool-search` 依赖 `harness-contracts + harness-tool + harness-model`。
- `crates/harness-tool-search.md:3-7` 重复该依赖关系，并说明通过 L4 注入避免反向依赖。

影响：

- CI 无法从 D2 推导合法依赖。
- `harness-tool-search -> harness-tool` 是合法例外还是 trait 依赖，D2 没有登记。

审计判定：

D2 应显式收录 `harness-tool-search`，并把这条 L2 内依赖写成白名单规则，而不是让读者从 ADR 推断。

## 5. P2 问题

### P2-1 版本叙事已经压过当前状态

`README.md` 长篇记录 v1.1 到 v1.8 的修订摘要。这对追溯有价值，但对“当前权威基线”不友好。

表现：

- README 前 800 行大部分是版本摘要。
- 入口读者需要穿过历史叙事才能理解当前状态。
- 部分摘要声称已同步，但实际 D1/D2/D3/D4 仍漂移。

审计判定：

README 应区分“当前基线索引”和“变更日志”。否则 Accepted 文档会被历史叙事掩盖。

### P2-2 `api-contracts.md` 的内置工具列表滞后

证据：

- `api-contracts.md:363-364` 的实现者列表仍是 `Bash / ReadFile / EditFile / Grep / Glob / WebFetch / Todo / AgentTool / TaskStop`。
- `harness-tool.md:649-664` 已列出 `FileRead / FileEdit / FileWrite / ListDir / WebSearch / Clarify / SendMessage / ReadBlob / ToolSearch` 等。
- `harness-tool.md:793-805` 又新增 `execute_code`。

影响：

- 接入方无法从 D3 得到 M0/M1 工具面的准确信息。
- Tool Search、ReadBlob、PTC 的公共面容易被遗漏。

### P2-3 `SessionReloadRequested` 缺少 never-drop 判定

证据：

- `harness-contracts.md:128` 定义 `SessionReloadRequested`。
- `event-schema.md:2291-2305` 的 `DEFAULT_NEVER_DROP_KINDS` 包含 `SessionReloadApplied`，不包含 `SessionReloadRequested`。

影响：

- live stream 中只能看到结果，看不到请求。
- reload 被拒绝或校验失败时，审计链可能只剩错误事件，缺少原始 delta hash。

### P2-4 安全文档中的检查项有未关闭项

证据：

- `security-trust.md:646` 仍有 `- [ ] 默认配置通过 security audit 指令`。

影响：

- Accepted 状态下仍保留未完成 checklist，容易让读者误判该安全基线是否已完成。

## 6. 通过项

以下部分方向是成立的：

- 文档集按 SAD、ADR、crate SPEC 分层，结构完整。
- `reference-analysis` 提供了 Hermes / OpenClaw / Claude Code 的证据索引和横向矩阵。
- 本地 Markdown 链接全部可解析。
- Prompt Cache、Tool 不含 UI、Permission Event、Sandbox 与 Permission 正交、Plugin Trust 二分，这些关键方向有 ADR 支撑。
- Memory v1.4 的栅栏、威胁扫描、外部 provider slot 与参考证据匹配度较高。
- MCP v1.5、Hook v1.6、Plugin v1.7、Steering/PTC v1.8 的设计面覆盖较全。

这些通过项不抵消 P0。它们说明方向可保留，但契约必须收敛。

## 7. 审计边界

本次没有评估：

- Rust 代码实现质量。
- Cargo workspace 是否已包含这些 crate。
- CI 配置是否实际执行依赖图校验。
- OpenAPI 生成链路。
- 产品发布节奏、MVP、里程碑或开发任务拆分。

本次结论只针对架构文档能否作为治理与实现基线。
