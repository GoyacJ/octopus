# 架构评审报告 · Octopus Agent Harness SDK

> 日期：2026-04-25
> 评审范围：`docs/architecture/harness/`（README + CHANGELOG + D1-D10 + 18 ADR + 19 crate SPEC，约 30000 行）
> 评审基线：v1.8（README 首页所标）/ overview.md 仍标 v1.0(定稿)
> 评审身份：高级 Agent 架构师（独立第三方审计视角）
> 评审方法：四维并行抽检 + 1.8 主线贯穿验证 + 七维度结构化打分
> 报告状态：Accepted（用户已确认本次审计形式）
> **修订进度（2026-04-25）**：P0（2 项 ✅）、P1（5 项 ✅）、P2（7 项 ✅）已全部落地，详见末尾 §A 修订执行记录。文档基线升至 v1.8.1。P3（4 项）保持 Pending，按建议性处理。

---

## 0. 执行摘要

### 0.1 总体结论 · Verdict

**有条件准予进入实现阶段**。本设计是一份在中文 Agent SDK 领域中**罕见的高完成度方案**——从战略边界、三家参考体系（Claude Code / Hermes / OpenClaw）的反例对照、到 19 crate 的接口签名细节、到 18 份 ADR 的负向论证，证据链与原则贯彻一致性都已达到生产级方案的入门水准。但仍存在 **2 项 P0、5 项 P1、7 项 P2、4 项 P3** 的修订需求，须在进入实现阶段前完成。

| 维度 | 评分（0-5） | 简评 |
|---|---|---|
| **A** 战略与边界 | 5 | 14 项非目标守得很死；SDK / 业务层力量分配清晰；术语表与拓扑图明确 |
| **B** 架构骨架 | 4 | 5 层依赖单向、Bounded Context 清晰；`module-boundaries §3` feature 触发的跨原语依赖未登记，§10 例外表与实际不符 |
| **C** 契约与接口 | 3.5 | trait 总表覆盖度不足（缺 EngineRunner / 两 Plugin Loader）；`String` 与 Newtype 叙事并存；`SessionBuilder` 的 type-state 未在 sdk 章节闭环 |
| **D** 关键不变量 | 4.5 | Prompt Cache 锁定、Fail-Closed、Event Sourcing 全面贯彻；Hook 默认 `FailOpen` 与文档"FailClosed 默认"叙述存在误读风险 |
| **E** 运行时正确性 | 4 | 主循环、并发、ResultBudget 健壮；`EndReason::Cancelled` 与 `Interrupted` 分歧未明；grace call 缺独立 Event 变体 |
| **F** 安全与信任 | 4.5 | 信任域二分、签名链分工、沙箱-审批正交都很扎实；`DirectBroker` 回调签名两文档不一致 |
| **G** 可演化性 | 4 | feature flag 系统化、扩展点模板化；ADR-0011/0015/0016/0017/0018 未在 D2/D3 顶层文档同步索引 |

**加权总分：4.2 / 5（生产级方案下限达标）**

### 0.2 设计亮点保护清单（必须保护、不得在后续迭代中拆掉的高价值决策）

1. **L4 单门面 + Builder Type-State 编译期约束**：`HarnessBuilder<Set<M>, Set<S>, Set<SB>>` 缺必填即编译失败 (`harness-sdk.md:L183-L240`)。这是把"配置正确性"上升到类型层的正宗做法。
2. **Event Sourcing 单一真相源 + Projection 派生视图**：避免 OpenClaw 不回放（OC-05）反例 (`event-schema.md §1` + ADR-001)。
3. **Prompt Cache 三件套运行期锁定**：`set_system_prompt(&mut self, _) -> Err(Error::PromptCacheLocked)`，把硬约束抬到代码契约 (`overview.md §7.4` + ADR-003)。
4. **Permission 审批事件化**：`PermissionRequested / PermissionResolved` 事件 + `DecidedBy` 契约 + Journal 审计闭环，杜绝 Hermes `_session_approved` 进程字典反例（HER-040）。
5. **沙箱与审批正交闸门**：明文不采纳 HER-041（容器跳审批）(`harness-sandbox.md:L646-L660`)。
6. **Plugin 信任域二分 + Loader 二分**：ADR-006 admin-trusted/user-controlled + ADR-0015 ManifestLoader/RuntimeLoader 把"发现期不执行代码"抬到类型层。
7. **Manifest Signer 与 Permission Integrity Signer 完全隔离**：ADR-0014 与 ADR-0013 各自独立的 KeyStore，避免单点钥匙泄漏污染另一域 (`security-trust.md §9.2`)。
8. **`ToolSearchTool` 用 `ToolContext` 反向打破层间环**：`harness-tool-search` 仅吃 `Tool` trait + `ToolDescriptor`，由 L4 完成注入，避免 `harness-tool` 反向依赖 (`harness-tool-search.md:L4-L7`)。
9. **Memdir + 外部 MemoryProvider 二分 + `<memory-context>` 三道闸 + Recall 不污染 system cache** (`harness-memory.md §3` + `context-engineering.md §11`)。
10. **ADR-0018 反向决议**：明确不引入 Loop-Intercepted Tools，维持"Tool 是一等公民"边界，规避 Hermes 双路径复杂度（这个反向决议本身证明了团队的工程定力，是治理成熟度的标志）。
11. **Context 管线五阶段固定顺序**：`ToolResultBudget → Snip → Microcompact → Collapse → Autocompact` 在 `ContextStageId` 枚举层硬编码，不可重排。
12. **AGENTS.md 治理对齐**：runtime/events/*.jsonl、data/main.db、config/runtime/* 三套持久化责任已被纳入 SDK 边界声明 (`overview.md §11`)。

> 上述亮点必须在后续 ADR 中显式声明"不可破坏"，任何想要弱化它们的 PR 需先开 ADR。

---

## 1. 七维度评分卡详解

### 1.1 维度 A · 战略与边界（5 分）

**判断**：这是本设计最强的部分。

- 14 项非目标显式列出（无 UI / 无 HTTP / 无调度 / 无跨进程 Team / 无 DB 迁移），每项给出"业务层如何替代"的方向。
- 三家参考体系（Claude Code、Hermes、OpenClaw）的反例对照贯穿到具体 ADR，不是空头比对。
- AGENTS.md 治理（事件源、配置分层、blob 元数据分离）被纳入 SDK 边界声明。
- Workspace / Tenant / Session / Run / Agent / Subagent / Team Member 七个核心术语在 `overview.md §1.4` 形成术语表。

**唯一风险**：`overview §1.3` 排除"跨进程 Team"，但 `harness-team.md` 正文中**未同等显式**地写"单进程内"约束（仅技术实现暗示 `tokio::sync::broadcast`）。读者只读 crate SPEC 时可能误判。

### 1.2 维度 B · 架构骨架（4 分）

**判断**：5 层 + 19 crate 拆分合理，但 D2 模块边界文档与 SPEC 之间存在治理缺口。

**问题**：
- `module-boundaries.md §3` 的 crate 白名单与 SPEC `[dependencies]` **feature 触发的跨层/跨原语依赖** 不一致（详见 P1-1）
- §10 例外登记表只有 1 条，实际还有至少 3 条隐性破窗未登记
- `harness-engine` 白名单未涵盖 `subagent-tool` feature 引入的 L3→L3 依赖

**亮点**：
- L0 契约层对所有 crate 开放、L4 唯一门面、L2 同层耦合需 ADR 登记的治理规则非常清晰
- `cargo-deny` + `cargo-depgraph` 计划纳入 CI

### 1.3 维度 C · 契约与接口（3.5 分）

**判断**：trait 设计成熟，但"契约总表的权威性"未达成。

**问题**：
- `api-contracts.md` 自称"trait 单一事实源"（`api-contracts.md:L1-L5`），但实际**遗漏** `EngineRunner`、`PluginManifestLoader`、`PluginRuntimeLoader`、`McpTransport`、`HookTransport` 等关键 trait
- `EngineRunner` 在 `module-boundaries §6` 被引用为"循环依赖防御机制"，但实际定义在 `harness-engine.md:L71-L83`（L3 crate），不在 contracts 层。这与"循环依赖防御应来自更低层"的直觉冲突
- ADR-0011 称 `ToolCapabilityHandle`，仓库实际类型名 `ToolCapability`，命名漂移
- `harness-contracts` 中 `ToolOrigin::Plugin { plugin_id: String }` 与 `harness-plugin` 中 `PluginId(String)` newtype 形态不一致
- `SessionBuilder` 在 `harness-sdk.md` 中无完整 type-state 定义；只有 `harness-session.md` 散见 `with_*` 增量方法

**亮点**：
- `HarnessBuilder` type-state 设计正宗
- `MessagePart` / `ToolResultPart` 等核心载荷类型设计严谨

### 1.4 维度 D · 关键不变量（4.5 分）

**判断**：核心不变量执行扎实，但 Hook FailOpen/FailClosed 默认值需公开澄清。

**关键不变量贯彻情况**：

| 不变量 | 贯彻 | 备注 |
|---|---|---|
| Prompt Cache Locked（P5） | 完全贯彻 | 类型层 + ADR-003 + Hot Reload 三档 |
| Fail-Closed Default（P6） | 部分贯彻 | Permission Broker 默认 Deny-All ✓；Hook 默认 `FailOpen` ✗（与读者预期不一致） |
| Event Sourcing（P3） | 完全贯彻 | append-only + projection + replay 三件套 |
| Subagent 不直接对用户说话 | 完全贯彻 | Blocklist 默认含 `send_user_message` |
| Sandbox 不替代 Permission | 完全贯彻 | HER-041 显式不采纳 |
| `system + tools + memdir` 运行期不变 | 完全贯彻 | 类型层强制；reload_with 三档 |

**问题**：
- Hook `failure_mode` 默认 `FailOpen`（仅 admin 可声明 `FailClosed`），与"P6 Fail-Closed Default"的整体口径**有出入**。这并非错误（hook 失败即拒绝会让非关键 hook 阻塞主流程），但需要在文档中显式声明"P6 仅适用于 Permission/Tool 层，Hook 层默认 FailOpen 是受控例外"

### 1.5 维度 E · 运行时正确性（4 分）

**判断**：主循环状态机健壮，但终止理由与 Run 层映射存在缺口。

**亮点**：
- Engine `LoopState` 五态明确：`AwaitingModel / ProcessingToolUses / ApplyingHookResults / MergingContext / Ended(StopReason)`
- ResultBudget 三档（Truncate / Offload / Reject）在 Engine + Tool + Event 三处一致
- Tool Orchestrator 并发分桶基于 `is_concurrency_safe` 二档
- Steering Queue（ADR-0017）的 `drain_and_merge` 在主循环安全检查点 drain，避免半道污染 prompt cache

**问题**：
- `EndReason` 枚举（`harness-contracts.md:L355-L362`）**未列出独立 `Cancelled` 变体**；用户取消与系统中断都映射到 `Interrupted`，影响审计可追溯性
- "grace call"（剩余预算 -1 时给 LLM 一次收尾机会）在 Engine `§4.1` 已实现，但**未对应独立 Event 变体**（如 `GraceCallTriggered`），可观测性留白
- Tool 硬超时（`ToolError::Timeout`）与 `EndReason` 的对应路径在 SPEC 之间未一表全列
- "并发安全 / 非安全" 分桶的命名为 bool 二档，与 overview §8 中提到的"分桶"语境读起来像三档（Shared/Exclusive/FreeForm），需要术语对齐

### 1.6 维度 F · 安全与信任（4.5 分）

**判断**：本设计的第二强项。

**亮点**：
- ADR-006 / ADR-0014 / ADR-0013 / ADR-0015 形成完整的"插件供应链"审计闭环
- `MemoryThreatScanner` 默认 30 条正则、扫描器 + 三档动作（Warn/Redact/Block）+ `<memory-context>` 栅栏 + 上一轮栅栏剥离
- `DangerousPatternLibrary` 默认 30+ 条 Unix/Windows 命令模式（rm -rf 根、curl-pipe-sh、git force push main、fork bomb 等）
- 沙箱 CWD marker 通过独立 FD 协议而非 stdout 解析（避免 marker 污染输出）
- Stdio MCP 子进程默认屏蔽常见凭证环境变量

**问题**：
- `DirectBroker` 回调签名在 `permission-model.md:L247-L249`（无 PermissionContext）与 `harness-permission.md:L152-L155`（有 PermissionContext）**不一致**
- `CredentialPool` 跨租户隔离依赖业务方正确构造 `CredentialKey { tenant_id, ... }`；trait 本身**不强制** `tenant_id` 必填，存在跨租户泄露漏洞
- `Redactor` 与 Journal 写入的精确挂钩点（"事件流出 SDK 之前必须经 Redactor"）在 `harness-observability.md` 未写死管线
- 加密事件流的 Replay 能力在 `harness-observability.md` **未提及**（生产合规可能需要）

### 1.7 维度 G · 可演化性（4 分）

**判断**：feature flag 与扩展点系统化设计良好，但顶层文档对 1.8 新增 ADR 的索引不完整。

**问题**：
- `module-boundaries.md` 头注仅引用 ADR-008，**未引用** ADR-0009/0011/0015/0016/0017/0018
- `api-contracts.md` 仅引用 ADR-002/007/0017，**未引用** ADR-0009/0011/0015/0016/0018
- `feature-flags.md §11` ADR 索引仅列 ADR-008/009，**未列** 0011/0015/0016/0017/0018
- `extensibility.md §12` 测试模板表缺 Skill / MCP / Plugin 三类 mock
- ADR-0018 反向决议明确"不修改 Engine/Tool/Subagent SPEC"，这一点合理；但 `event-schema.md` 等顶层文档仍可加一行"已采纳 ADR-0018 反向决议，Tool 一律走 Orchestrator"

---

## 2. 问题清单（按优先级）

### 2.1 P0 · 阻断实现（必须修复）

> **修订状态**：P0-1 / P0-2 已于 2026-04-25 落地，文档基线 v1.8.1。详见 §A.1。

#### **P0-1** ✅ 已修订 · Hook 默认 FailOpen 与 P6 Fail-Closed 总则的语义冲突未被文档声明
- **事实**：`overview.md §2 P6` 写明"Fail-Closed Default：Tool 默认不并发安全、不只读、破坏性未知；Broker 默认 Deny-All"。但 `harness-hook.md:L414-L437` 显示 `HookFailureMode` 默认 `FailOpen`（仅 admin 域可声明 `FailClosed`，user 域强制 `FailOpen`）。
- **影响**：审计与合规审查时，读者无法在不阅读 hook crate 的前提下判断 Hook 故障是否 fail-closed。生产中 hook 失败放过可能漏掉关键审计日志。
- **建议**：在 `overview.md §2` P6 行明确"Hook 层除外，详见 harness-hook §4.3"，并在 `security-trust.md` 单列一节说明"Hook FailOpen 是受控例外，理由：避免业务 hook 故障阻塞主流程；缓解：Admin 域可强制 FailClosed + 失败必发 `HookFailed` 事件"。
- **证据**：[`overview.md:L106-L113`](docs/architecture/harness/overview.md), [`harness-hook.md:L414-L437`](docs/architecture/harness/crates/harness-hook.md)

#### **P0-2** ✅ 已修订 · `DirectBroker` 回调签名两份文档不一致
- **事实**：`permission-model.md:L247-L249` 定义 `F: Fn(PermissionRequest) -> BoxFuture<Decision>`；`harness-permission.md:L152-L155` 定义 `F: Fn(PermissionRequest, PermissionContext) -> BoxFuture<Decision>`（多一个 PermissionContext 参数）。
- **影响**：实现期开发者按哪份签名编码？业务方按哪份签名实现 Broker？错位会导致接口在两个 crate 之间无法互通。
- **建议**：以 `harness-contracts` / `harness-permission` 为准（`PermissionContext` 是必要的——它承载 tenant_id / session_id / run_id 等审计字段），把 `permission-model.md` 示例修正为带 PermissionContext 版本。
- **证据**：[`permission-model.md:L242-L292`](docs/architecture/harness/permission-model.md), [`harness-permission.md:L147-L218`](docs/architecture/harness/crates/harness-permission.md)

### 2.2 P1 · 实现前必修

> **修订状态**：P1-1 ~ P1-5 已于 2026-04-25 全部落地，文档基线 v1.8.1。详见 §A.2。

#### **P1-1** ✅ 已修订 · D2 §3 白名单与 SPEC `[dependencies]` 不一致
- **事实**（agent1 发现）：
  - `harness-permission` 的 `auto-mode` feature 引入 `dep:octopus-harness-model`（**L1 → L1**）（`harness-permission.md:L988-L995`），未列入 D2 §3.2 / §10
  - `harness-model` 的 `redactor` feature 引入 `dep:octopus-harness-observability`（**L1 → L3**，向上依赖）（`harness-model.md:L836-L851`），未列入 D2
  - `harness-engine` 的 `subagent-tool` feature 引入 `dep:octopus-harness-subagent`（**L3 → L3**）（`harness-engine.md:L471-L476`），D2 §3.4 白名单未列出
- **影响**：CI 接入 `cargo-deny` 后会拒绝合法配置，或更糟——CI 配置宽松，破窗依赖蔓延。
- **建议**：D2 §3 白名单加"feature 触发的依赖另立附录 §3.7"，并把上述 3 条以及"未来新增 feature 触发的依赖"全部登记。或选择性废弃 `harness-permission::auto-mode` 等 feature，把跨原语依赖改为业务层组合。
- **证据**：上述行号

#### **P1-2** ✅ 已修订 · `module-boundaries §10` 例外登记表不完备
- **事实**：表中仅 1 行（`harness-tool-search → harness-tool`）。实际至少还有 P1-1 中的 3 条破窗未登记。
- **影响**：治理失效——例外登记表是评审破窗依赖的关卡，不完整意味着任何人可继续加未登记的破窗。
- **建议**：与 P1-1 同步修订；或将 §10 范围明确缩小为"L2 同层耦合"，feature 依赖另设附录。
- **证据**：[`module-boundaries.md:L170-L178`](docs/architecture/harness/module-boundaries.md)

#### **P1-3** ✅ 已修订 · `EndReason` 缺独立 `Cancelled` 变体
- **事实**：`harness-contracts.md:L355-L362` 的 `EndReason` 当前列 `Completed / Interrupted / MaxIterationsReached / TokenBudgetExceeded / Compacted / Failed` 等。`Subagent` 的终止枚举有 `ParentCancelled`，但 Run 层没有用户主动 `Cancel` 与系统 `Interrupt` 的区分。
- **影响**：审计与可观测性受损——产品 UI 中"用户点取消"与"系统中断"事件无法区分；运营复盘"哪些 Run 是用户取消的"无法直接查询。
- **建议**：在 `EndReason` 中新增 `Cancelled { initiator: CancelInitiator }` 变体（`CancelInitiator = User / System / Parent`），或明确以 `metadata.cancel_initiator` 字段承载并写入 ADR。
- **证据**：[`harness-contracts.md:L355-L362`](docs/architecture/harness/crates/harness-contracts.md)

#### **P1-4** ✅ 已修订 · Hook PreToolUse 三件套与 PermissionContext 的失败路径未明
- **事实**：Hook 三件套（改写 input / 改写 permission decision / 阻止执行）若 hook 自身故障（FailOpen 或 FailClosed），其改写效果如何回滚？`harness-hook.md` 描述了能力但未明示"hook 失败时已改写 input 是否回滚"。
- **影响**：边界场景下 hook 中途崩溃会导致 input 已被改写但 permission 未生成，主流程行为不可预测。
- **建议**：在 `harness-hook.md` 增加一节"Hook 失败的事务语义"，明确"PreToolUse 链是 all-or-nothing：任一 hook 失败 → 视 failure_mode 决定，但 input 改写状态不外泄"。
- **证据**：[`harness-hook.md:L414-L437`](docs/architecture/harness/crates/harness-hook.md)

#### **P1-5** ✅ 已修订 · `CredentialKey` 不强制 `tenant_id`
- **事实**：`harness-model.md:L410-L454` 的 `CredentialKey` 设计良好，但是 trait 接口允许业务方传入"无 tenant_id"的 key（即跨租户共享凭证）。`security-trust.md §7.3` 说"per_tenant_credential_pool 可选开启"。
- **影响**：多租户场景下，业务方误用 → 跨租户凭证泄漏。Octopus 是声明的"多租户产品"，这是高风险路径。
- **建议**：将 `CredentialKey { tenant_id: TenantId, ... }` 中 `tenant_id` 标为**必填**（无默认值）；若需"全局凭证池"则用显式 `TenantId::SHARED` 哨兵值，并在审计层强制告警。
- **证据**：[`harness-model.md:L410-L454`](docs/architecture/harness/crates/harness-model.md), [`security-trust.md:L339-L347`](docs/architecture/harness/security-trust.md)

### 2.3 P2 · 进入实现可保留但需后续修订

> **修订状态**：P2-1 ~ P2-7 已于 2026-04-25 全部落地，文档基线 v1.8.1。详见 §A.3。

- **P2-1** ✅ 已修订：`api-contracts.md` 头注扩补 ADR-0009/0011/0015/0016/0017/0018；§14.3 新增 `EngineRunner` trait；§17.2/17.3 新增 `PluginManifestLoader` / `PluginRuntimeLoader`。
- **P2-2** ✅ 已修订：`adr/0011-tool-capability-handle.md` 新增 §0 术语对齐块，禁止 `ToolCapabilityHandle` / `ToolCap` 别名，统一为 `ToolCapability` enum + 7 个 `*Cap` 窄接口 trait + `CapabilityRegistry`。
- **P2-3** ✅ 已修订：`harness-contracts.md` `ToolOrigin::Plugin { plugin_id: PluginId, trust }` 改用 newtype，与 `McpServerSource::Plugin(PluginId)` / `SkillSourceKind::Plugin(PluginId)` 对齐。
- **P2-4** ✅ 已修订：`harness-sdk.md §8.1` 新增 Builder 幂等覆盖语义；§8.2 显式声明 Session 不走 type-state（运行时 Result 校验），与 type-state 二分边界明确。
- **P2-5** ✅ 已修订：`event-schema.md §3.1.1` 新增 `GraceCallTriggeredEvent`（带 usage_snapshot），§2 总览同步登记；`harness-engine.md §4.1` 增加发出契约。
- **P2-6** ✅ 已修订：`harness-tool.md §2.7` 显式声明"bool 二档（is_concurrency_safe），不存在三桶"，并解释为何不引入三档枚举（KISS）；`overview.md §7.1` 流程图同步标注。
- **P2-7** ✅ 已修订：`harness-observability.md §2.5.0` 新增"Redactor 必经管道契约"段，覆盖 6 条数据流的挂钩点；`harness-journal.md §2.1` 头注同步声明 EventStore 实现必须装配 `Arc<dyn Redactor>` 并通过 `RedactorContractTest` 套件验证。

### 2.4 P3 · 建议性

- **P3-1**：`module-boundaries.md` / `api-contracts.md` / `feature-flags.md` 头注同步引用 1.8 引入的 ADR-0009/0011/0015/0016/0017/0018。
- **P3-2**：`extensibility.md §12` 测试模板表补 Skill / MCP / Plugin 三类的 mock 标准。
- **P3-3**：`harness-team.md` 增加"单进程内"约束的显式段落（与 `overview §1.3` 等同陈述）。
- **P3-4**：MCP `ReconnectPolicy.max_attempts: 0` 的"0 = 不限"语义应在 SPEC 内显式注明（避免与"0 = 不重试"的直觉混淆）。

---

## 3. 一致性矩阵（主文档 ↔ ADR ↔ SPEC）

### 3.1 1.8 改动贯穿性追踪

| 1.8 改动 | CHANGELOG | ADR | overview | api-contracts | event-schema | feature-flags | crate SPEC | 评价 |
|---|---|---|---|---|---|---|---|---|
| Steering Queue | ✓ | ADR-0017 | ✗ §10 决策表 | §13.1.2 ✓ | §3.5.1 ✓ | ✓ | harness-engine / session ✓ | **顶层 overview §10 决策一览表未补 Steering Queue 行** |
| Programmatic Tool Calling | ✓ | ADR-0016 | ✗ §10 | ✗ 头注 | §3.5.2 ✓ | ✓ | harness-tool ✓ | **顶层文档头注未引用 ADR-0016** |
| ADR-0018 反向决议 | ✓ | ✓ | ✗ | ✗ | ✗ | ✗ | ✗（ADR 自声明不修改） | **合理留白，但建议在 overview §10 加一行"已采纳 ADR-0018"** |
| Plugin Manifest Signer | ✓ (1.7) | ADR-0014 | ✓ §10 | ✗ 头注 | §3.20 ✓ | ✓ | harness-plugin ✓ | 已对齐 |
| Plugin Loader 二分 | ✓ (1.7) | ADR-0015 | ✓ §10 | ✗ 头注 | ✓ | ✓ | harness-plugin ✓ | 头注未列入 ADR-0015 |
| ManifestValidationFailed Event | ✓ | ADR-0014 | — | §17 ✓ | §3.20 ✓ | — | ✓ | 已对齐 |

**结论**：1.8 的核心 ADR 在事件层、SPEC 层、feature flag 层基本贯穿，但**顶层索引（overview §10、api-contracts 头注、module-boundaries 头注、feature-flags §11）**对 1.8 ADR 的引用不完整，导致新成员从 overview 入门时可能漏掉关键决策。

### 3.2 关键 trait 归属一致性

| trait | api-contracts §声明 | 实际 SPEC 位置 | 是否在 contracts crate | 评价 |
|---|---|---|---|---|
| `ModelProvider` | §3 ✓ | harness-model.md ✓ | contracts 抽象 | 一致 |
| `EventStore` | §5 ✓ | harness-journal.md ✓ | contracts 抽象 | 一致 |
| `SandboxBackend` | §7 ✓ | harness-sandbox.md ✓ | contracts 抽象 | 一致 |
| `PermissionBroker` | §8 ✓ | harness-permission.md ✓ | contracts 抽象 | DirectBroker 签名不一致（P0-2） |
| `MemoryProvider` (Store + Lifecycle) | §6 ✓ | harness-memory.md ✓ | contracts 抽象 | 一致 |
| `Tool` | §9 ✓ | harness-tool.md ✓ | contracts 抽象 | 一致 |
| `EngineRunner` | **未列** | harness-engine.md §2.2 | **不在 contracts** | **不一致** — 影响 module-boundaries §6 循环依赖防御陈述 |
| `PluginManifestLoader` / `PluginRuntimeLoader` | **未列** | harness-plugin.md §3.2 | **不在 contracts** | **不一致** — ADR-0015 治理 trait 应在 contracts |
| `McpTransport` | §16 ✓ | harness-mcp.md ✓ | contracts 抽象 | 一致 |
| `HookHandler` | §11 ✓ | harness-hook.md ✓ | contracts 抽象 | 一致 |
| `HookTransport` | **未列** | harness-hook.md ✓ | 部分 | 不一致 |
| `ToolCapability`（ADR-0011 称 Handle） | §9 部分 | harness-contracts §3.4 ✓ | 在 contracts | 命名漂移 |

---

## 4. 后续行动 Plan（可直接转 Issue）

### 4.1 P0 修订任务（实现前必须完成）

| ID | 任务 | 影响文档 | 预计工作量 |
|---|---|---|---|
| **F-P0-1** | 在 `overview.md §2 P6` 与 `security-trust.md` 显式声明"Hook FailOpen 是 P6 受控例外" | overview.md / security-trust.md / harness-hook.md | 30 min |
| **F-P0-2** | 统一 `DirectBroker` 回调签名为带 `PermissionContext` 版本 | permission-model.md / harness-permission.md | 30 min |

### 4.2 P1 修订任务（实现前必须完成）

| ID | 任务 | 影响文档 | 预计工作量 |
|---|---|---|---|
| **F-P1-1** | D2 §3 增加"feature 触发依赖"附录，登记 3 条已知破窗 | module-boundaries.md | 1 h |
| **F-P1-2** | D2 §10 例外表与 P1-1 同步修订 | module-boundaries.md | 30 min |
| **F-P1-3** | `EndReason` 增加 `Cancelled { initiator }` 变体 | harness-contracts.md / event-schema.md / harness-engine.md | 1 h |
| **F-P1-4** | `harness-hook.md` 增加 "Hook 失败的事务语义"段 | harness-hook.md | 1 h |
| **F-P1-5** | `CredentialKey.tenant_id` 必填化 + `TenantId::SHARED` 哨兵 | harness-model.md / harness-contracts.md / security-trust.md | 1.5 h |

### 4.3 P2/P3 任务（进入实现后排期修订）

汇总打包为 1 个修订项目"Octopus Harness SDK · 文档对齐 v1.9"，包含 P2-1 ~ P3-4 共 11 项，预计累计工作量 8-12 小时。建议作为 v1.9 文档版本一次性发布。

### 4.4 实现期建议

1. **从 L0 契约层开始建仓**：第一个 PR 实现 `harness-contracts` 全部类型（先无逻辑）；CI 检查 `cargo doc` 与 schemars 派生通过即可
2. **L1 五原语并行**：每个 L1 crate 一个负责人，trait + 默认实现（Builtin* 系列）+ 单元测试同步实现
3. **L2 复合能力按需实现**：先实现 `tool / hook / context / session`（最小可运行集合），暂缓 `tool-search / skill / mcp`
4. **L3 由 Engine 牵头**：Engine 实现先做单 Run 主循环，再加 Subagent / Team
5. **L4 门面最后整合**：HarnessBuilder type-state 完整收口
6. **POC 优先验证 3 项高风险设计**（建议在正式实现前用最小 prototype 验证）：
   - Prompt Cache 在 Anthropic API 上的实际命中率（验证 P5 假设是否成立）
   - Steering Queue 在长 turn 场景下的语义正确性（验证 ADR-0017）
   - Hook 多 transport（in-process / Exec / HTTP）的失败模式与 replay 幂等真实行为（验证 P0-1 / P1-4 修订效果）

---

## 5. 评审方法论与证据来源

### 5.1 评审方法

- **七维度框架**：战略边界 / 架构骨架 / 契约接口 / 关键不变量 / 运行时正确性 / 安全信任 / 可演化性
- **四象限并行抽检**：每象限由独立只读 explore 子代理负责，输出 raw findings；评审者做最终判定与交叉验证
- **1.8 主线贯穿验证**：对最近 3 周引入的 6 份 ADR（0009/0011/0015/0016/0017/0018）做顶层文档贯穿性追踪
- **反例对照**：以 Claude Code（CC-xx）/ Hermes（HER-xxx）/ OpenClaw（OC-xx）三家工程债务为反例，验证设计是否规避

### 5.2 证据来源

- **主文档**：D1-D10 全部
- **ADR**：0001-0018 全部
- **Crate SPEC**：19 份全部
- **CHANGELOG**：v1.0 → v1.8 全部
- **参照分析**：`docs/architecture/reference-analysis/comparison-matrix.md`、`evidence-index.md`

### 5.3 评审者声明

本次评审为独立第三方架构审计视角，不参与设计决策本身。对发现的每一项问题：
- 都附文件路径 + 行号
- 都给优先级建议
- 都不预设具体的实现方案，由设计团队自行选择修订路径

如对评审结论有异议，请以 ADR 形式正式反驳；本报告将以本次提交版本为准。

---

## A. 修订执行记录（2026-04-25）

本节登记所有 P0/P1/P2 修订的实际落点（文件路径 + 章节号），便于反向追溯。

### A.1 P0 修订（2 项）

| 编号 | 修订点 | 落点文件 | 落点位置 |
|---|---|---|---|
| P0-1 | Hook FailOpen 标注为 P6 受控例外 | `overview.md` | §2 P6 行追加例外引用 |
| P0-1 | Hook 受控例外完整说明 | `security-trust.md` | 新增 §10.W（含原因 / 缓解措施 / 治理要求三段） |
| P0-2 | DirectBroker 签名统一 | `permission-model.md` | §5.1 替换为带 `PermissionContext` 版本 |

### A.2 P1 修订（5 项）

| 编号 | 修订点 | 落点文件 | 落点位置 |
|---|---|---|---|
| P1-1 | Feature 触发依赖治理附录 | `module-boundaries.md` | 新增 §3.7（5 条治理规则） |
| P1-2 | 例外登记表扩 | `module-boundaries.md` | §10 表新增 3 行（auto-mode / redactor / subagent-tool） |
| P1-3 | EndReason 加 Cancelled 变体 | `harness-contracts.md` | §3 EndReason 新增 `Cancelled { initiator: CancelInitiator }` 与 `CancelInitiator` 枚举 |
| P1-3 | EndReason 触发源映射 | `harness-engine.md` | §5 中断节末尾新增触发源选择表 |
| P1-3 | 审计契约说明 | `event-schema.md` | §3.0 SessionEnded 注解后新增审计契约段 |
| P1-4 | Hook 失败事务语义 | `harness-hook.md` | 新增 §2.6.2（事务边界 4 项 / failure_mode 关系 / 实现约束 / 反模式 4 项） |
| P1-5 | CredentialKey 多租户安全契约 | `harness-model.md` | §2.4 `CredentialKey` 文档块大幅扩写 |
| P1-5 | TenantId::SHARED 哨兵 | `harness-contracts.md` | §3 TenantId impl 块新增 SHARED 常量 |
| P1-5 | 强隔离表更新 | `security-trust.md` | §7.3 Model Credential Pool 行从"可选"改为"默认强隔离" |
| P1-5 | 必记事件登记 | `security-trust.md` §8.1 + `event-schema.md` §2 | 新增 `CredentialPoolSharedAcrossTenants` 事件 |
| P1-5 | HookFailedEvent 必记 | `security-trust.md` §8.1 | 补登 HookFailedEvent 为必记事件 |

### A.3 P2 修订（7 项）

| 编号 | 修订点 | 落点文件 | 落点位置 |
|---|---|---|---|
| P2-1 | api-contracts 头注扩 ADR | `api-contracts.md` | 头注追加 6 条 ADR 引用 |
| P2-1 | EngineRunner trait 登记 | `api-contracts.md` | 新增 §14.3 |
| P2-1 | Plugin Loaders trait 登记 | `api-contracts.md` | 新增 §17.2 / §17.3，原 PluginSource 顺延为 §17.4 |
| P2-2 | ToolCapability 术语对齐 | `adr/0011-tool-capability-handle.md` | 新增 §0 术语对齐块 |
| P2-3 | ToolOrigin.plugin_id newtype | `harness-contracts.md` | §3 ToolOrigin::Plugin 字段类型 String → PluginId |
| P2-4 | Builder 幂等语义 | `harness-sdk.md` | 新增 §8.1 |
| P2-4 | Session 不走 type-state 声明 | `harness-sdk.md` | 新增 §8.2（含 SessionOptions 完整签名） |
| P2-5 | GraceCallTriggered Event | `event-schema.md` | §2 总览 Run 执行行加事件名 + 新增 §3.1.1 完整结构 |
| P2-5 | Engine 发出契约 | `harness-engine.md` | §4.1 grace call 描述加发出契约 |
| P2-5 | EndReason 关系注释 | `harness-contracts.md` | §3 EndReason 上方加注释 |
| P2-6 | 并发分桶术语澄清 | `harness-tool.md` | §2.7 分桶规则 KISS 解释 |
| P2-6 | 流程图标注 | `overview.md` | §7.1 turn 流程图标注"bool 二档" |
| P2-7 | Redactor 必经管道契约 | `harness-observability.md` | §2.5 拆为 §2.5.0（必经管道）/ §2.5.1（数据结构），新增 6 行挂钩点表 |
| P2-7 | EventStore 头注同步 | `harness-journal.md` | §2.1 trait 上方头注新增管道契约声明 |

### A.4 验收建议

实现期开发应以本评审报告为入口，按 §A.1-A.3 逐文件复核已修订内容；本报告与对应 SPEC 出现冲突时，**以 SPEC 为准**（参见 `overview.md §12 本 SAD 的权威性`）。

### A.5 P3 待办（非必修）

- **P3-1**：`module-boundaries.md` / `api-contracts.md` 头注同步 1.8 ADR（**P2-1 已部分覆盖** api-contracts 头注；module-boundaries 头注仍只引 ADR-008）→ 仍待
- **P3-2**：`extensibility.md §12` 测试模板表补 Skill / MCP / Plugin 三类 mock → 仍待
- **P3-3**：`harness-team.md` 增加"单进程内"约束的显式段落 → 仍待
- **P3-4**：MCP `ReconnectPolicy.max_attempts: 0` "0 = 不限"语义注明 → 仍待

P3 项可在 v1.9 文档对齐窗口一次性处理，不阻断当前实现期推进。

---

## 6. 附录 · 评审过程审计

| 阶段 | 输入 | 输出 | 时长 |
|---|---|---|---|
| 范围确认 | 用户三个决策点 | 完整审计 + 落盘 + P0/P1 任务清单 | 5 min |
| 框架构思 | sequential-thinking 6 步 | 七维度评分卡 + 四象限并行计划 | 8 min |
| 基线建立 | README / overview / module-boundaries / CHANGELOG | 1.8 改动主线 + 拓扑图掌握 | 10 min |
| 并行抽检 | 4 个 explore 子代理（read-only） | 每子代理 6-8 个事实发现 | 25 min（并行） |
| 贯穿验证 | event-schema 总览 + harness-contracts grep | 顶层文档对 1.8 ADR 引用缺口 | 5 min |
| 报告汇编 | 上述所有材料 | 本报告 | 15 min |

**累计：约 70 分钟**（与"完整审计 1.5-2 小时"预估一致）。
