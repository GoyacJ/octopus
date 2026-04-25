# 01 · 路线图（Roadmap）

> 状态：Active
> DAG 视图：M0 → M1 → M2(并行5路) → M3(串行) → {M4(并行3路) ‖ M5(串行)} → M6(并行2路) → M7(串行) → M8(并行3路) → M9(串行)

---

## 1. 总图

```text
                          ┌────────────────┐
                          │ M0 Bootstrap   │  workspace 清理 + 19 crate 空骨架
                          └───────┬────────┘
                                  ▼
                          ┌────────────────┐
                          │ M1 L0 contracts │  harness-contracts（串行）
                          └───────┬────────┘
                                  ▼
        ┌─────────────────────────┼─────────────────────────┐
        │ M2 L1 primitives（5 路并行）                        │
        ├─────┬─────────┬─────────┬───────────┬─────────────┤
        │model│ journal │ sandbox │ permission│ memory      │
        └─────┴─────────┴─────────┴───────────┴─────────────┘
                                  │
                                  ▼
        ┌────────────────────────────────────────────────────┐
        │ M3 L2 core（强制串行 4 步）                          │
        │  tool → hook → context → session                    │
        │  (M3 完成 = 最小可运行 SDK 闭环)                      │
        └────────────────────────────────────────────────────┘
                                  │
        ┌─────────────────────────┴─────────────────────────┐
        ▼                                                   ▼
┌──────────────────────┐                       ┌────────────────────────┐
│ M4 L2 extensions     │ ← 可与 M5 并行          │ M5 L3 single-agent    │
│ tool-search · skill · mcp                     │ engine + observability + plugin
└──────┬───────────────┘                       └──────────┬─────────────┘
       └──────────────────────┬──────────────────────────┘
                              ▼
              ┌────────────────────────────────┐
              │ M6 L3 multi-agent              │
              │ subagent ‖ team（并行 2 路）     │
              └─────────┬──────────────────────┘
                        ▼
              ┌────────────────────────────────┐
              │ M7 L4 facade · harness-sdk      │
              └─────────┬──────────────────────┘
                        ▼
        ┌───────────────┴───────────────────┐
        │ M8 Business cutover（并行 3 路）    │
        │ server ‖ desktop ‖ cli            │
        └───────────────┬───────────────────┘
                        ▼
              ┌────────────────────────────────┐
              │ M9 POC + Acceptance（串行）      │
              └────────────────────────────────┘
```

---

## 2. 里程碑卡片

### M0 · Bootstrap

- **目标**：清理旧工作空间、建立 19 个 crate 空骨架、CI 接入
- **入口任务卡**：`milestones/M0-bootstrap.md` T01-T08
- **关键交付**：
  - 14 个 `octopus-sdk*` crate 已 `git rm -rf`（保留 `octopus-core/persistence/platform/infra/server/desktop/cli`）
  - 19 个 `octopus-harness-*` crate 出现在 workspace（仅含 `lib.rs` 占位 + `Cargo.toml`）
  - `cargo check --workspace` 通过
  - GitHub Actions / `cargo deny` / `cargo clippy` CI workflow 就位
- **退出条件**：`cargo metadata` 列出 19 个新 crate；`cargo check` 通过；CI 首次绿
- **预估任务卡数**：8

### M1 · L0 Contracts

- **目标**：`octopus-harness-contracts` 全量公共类型 + JSON Schema 派生
- **入口任务卡**：`milestones/M1-l0-contracts.md` T01-T10
- **关键交付**：
  - 所有 ID（`TypedUlid<XxxScope>` 12 类）+ `TenantId::SINGLE/SHARED`
  - `Event` 枚举完整变体（含 v1.8 新增的 5 个 Steering、2 个 ExecuteCode、`GraceCallTriggered`、`Cancelled` 等）
  - `Decision / PermissionMode / Severity / EndReason / CancelInitiator` 等共享 enum
  - `BlobStore / ToolCapability / DecisionScope` 等基础 trait
  - `schemars` 派生 + JSON Schema 文件输出到 `schemas/`
- **退出条件**：`cargo doc --no-deps -p octopus-harness-contracts` 生成成功；schema 输出 ≥ 60 个文件
- **预估任务卡数**：10

### M2 · L1 Primitives（并行 5 路）

- **目标**：5 个 L1 原语 crate 同时落地
- **入口任务卡**：`milestones/M2-l1-primitives.md` T01-T25
- **并行单元**：
  - **L1-A**：`octopus-harness-model`（trait + Mock + Anthropic provider 默认实现）
  - **L1-B**：`octopus-harness-journal`（trait + JsonlEventStore + SqliteEventStore + InMemory + BlobStore 三实现 + RetentionEnforcer + VersionedEventStore）
  - **L1-C**：`octopus-harness-sandbox`（trait + LocalSandbox + NoopSandbox；Docker/SSH 暂占位）
  - **L1-D**：`octopus-harness-permission`（trait + DirectBroker + StreamBasedBroker + 4 RuleProvider + DangerousPatternLibrary）
  - **L1-E**：`octopus-harness-memory`（MemoryStore + MemoryLifecycle 二分；Memdir 默认实现 + ThreatScanner）
- **退出条件**：5 crate 各自 `cargo test` 通过；contract-test 完整；CI 矩阵覆盖各 feature 组合
- **预估任务卡数**：25（每 crate 5 卡）

### M3 · L2 Core（串行 4 步）

- **目标**：最小可运行 SDK 闭环（不含 multi-agent）
- **入口任务卡**：`milestones/M3-l2-core.md` T01-T20
- **强制串行子步**：
  1. **L2-T**：`octopus-harness-tool`（Registry + Pool + Orchestrator + ResultBudget + 内置工具集）
  2. **L2-H**：`octopus-harness-hook`（Dispatcher + Registry + transport（in-process/Exec/HTTP）+ FailureMode + 事务语义）
  3. **L2-C**：`octopus-harness-context`（5 阶段管线 + ContextProvider）
  4. **L2-S**：`octopus-harness-session`（生命周期 + Projection + Fork + HotReload + SteeringQueue）
- **退出条件**：4 crate 各自 `cargo test` 通过；用一个**临时 driver**（M5 之前的脚本）跑通"create_session → run_turn → 内置 ListDir 工具"E2E
- **预估任务卡数**：20

### M4 · L2 Extensions（可并行 3 路，需 M3 完成）

- **目标**：tool-search / skill / mcp 三个扩展能力
- **入口任务卡**：`milestones/M4-l2-extensions.md` T01-T18
- **并行单元**：
  - **L2-TS**：`octopus-harness-tool-search`（DeferPolicy + ToolSearchTool + AnthropicReferenceBackend + InlineReinjectionBackend + DefaultScorer）
  - **L2-SK**：`octopus-harness-skill`（Loader + 多源优先级 + frontmatter + SkillTool 三件套）
  - **L2-MCP**：`octopus-harness-mcp`（Client transport 5 种 + ServerAdapter + OAuth + Elicitation）
- **退出条件**：三 crate 各自 `cargo test` 通过；MCP feature 矩阵 CI 全绿
- **预估任务卡数**：18

### M5 · L3 Single-Agent（可与 M4 并行，需 M3 完成）

- **目标**：单 Agent 执行内核 + 观测性 + 插件宿主
- **入口任务卡**：`milestones/M5-l3-engine.md` T01-T15
- **顺序**：
  1. **L3-O**：`octopus-harness-observability`（Tracer + Usage + Replay + Redactor 必经管道）
  2. **L3-P**：`octopus-harness-plugin`（ManifestLoader + RuntimeLoader + TrustedSignerStore + Capability handles）
  3. **L3-E**：`octopus-harness-engine`（LoopState 主循环 + 中断 + iteration budget + grace call）
- **退出条件**：3 crate 各自 `cargo test` 通过；E2E 用例 "engine.run(session) → AssistantDelta → ToolUseRequested → 完成" 跑通
- **预估任务卡数**：15

### M6 · L3 Multi-Agent（并行 2 路，需 M5 完成）

- **目标**：subagent 委派 + team 协同
- **入口任务卡**：`milestones/M6-l3-agents.md` T01-T10
- **并行单元**：
  - **L3-SA**：`octopus-harness-subagent`（AgentTool + DelegationBlocklist + SubagentAnnouncement + ConcurrentSubagentPool）
  - **L3-TM**：`octopus-harness-team`（Topology 三种 + MessageBus + Coordinator）
- **退出条件**：跨进程禁令在文档与编译期都生效；E2E 用例验证 SubagentBlocklist 默认行为
- **预估任务卡数**：10

### M7 · L4 Facade（串行）

- **目标**：`octopus-harness-sdk` 门面 + Builder type-state + prelude/builtin/ext/testing
- **入口任务卡**：`milestones/M7-l4-facade.md` T01-T06
- **关键交付**：
  - `HarnessBuilder<Set<M>, Set<S>, Set<SB>>` type-state 完整
  - `Harness::create_session / resolve_permission / resolve_elicitation` 全 API
  - `prelude` / `builtin` / `ext` / `testing` 四个模块
  - feature flags 与 `feature-flags.md` 完全对齐
- **退出条件**：`cargo doc --no-deps -p octopus-harness-sdk` 干净生成；business 层试点编译通过
- **预估任务卡数**：6

### M8 · Business Cutover（并行 3 路，需 M7 完成）

- **目标**：业务层从 `octopus-sdk*` 切换到 `octopus-harness-sdk`
- **入口任务卡**：`milestones/M8-business-cutover.md` T01-T12
- **并行单元**：
  - **B-S**：`octopus-server`（HTTP API 适配 SDK 事件流）
  - **B-D**：`octopus-desktop` + `apps/desktop/src-tauri`（Tauri command 切换）
  - **B-C**：`octopus-cli`（CLI 启动 + interactive broker 接线）
- **退出条件**：`cargo build --workspace --release` 通过；3 个业务入口在本地能启动
- **预估任务卡数**：12

### M9 · POC + Acceptance（串行）

- **目标**：三个高风险 POC + 端到端验收报告
- **入口任务卡**：`milestones/M9-poc-and-acceptance.md` T01-T08
- **POC 项**：
  1. **POC-1**：Prompt Cache 实测命中率（Anthropic + 多轮对话 + reload_with）
  2. **POC-2**：Steering Queue 长 turn 场景下的语义正确性
  3. **POC-3**：Hook 多 transport（in-process / Exec / HTTP）失败模式与 replay 幂等
- **验收**：
  - `apps/desktop` 跑通 "用户提问 → 工具调用 → 流式输出 → 权限审批"完整闭环
  - 输出 `docs/architecture/harness/audit/2026-XX-implementation-acceptance.md`
- **预估任务卡数**：8

---

## 3. 并行矩阵

| 里程碑 | 并行度 | 协调点（多卡共修文件） | **评审者吞吐瓶颈** |
|---|---|---|---|
| M0 | 1（串行）| `Cargo.toml` workspace.members | 1 reviewer / 12 卡 ≈ 4 工作日 |
| M1 | 1（串行）| `harness-contracts/src/lib.rs` 是单一汇出口 | 1 reviewer / 12 卡 ≈ 4 工作日 |
| M2 | **5（理论）/ 2-3（实际）** | 各 crate 独立 + 无共享文件；末端合并各卡更新根 `Cargo.toml` | 5 路并行受 reviewer 吞吐限制；按 1 reviewer ≤ 3 PR/天 → 25 卡需 8-10 工作日 |
| M3 | 1（串行 4 步） | session 是聚合者，必须等 tool/hook/context 完成 | 1 reviewer / 21 卡（含 T21 chore）≈ 7 工作日 |
| M4 | **3** | 三 crate 独立；MCP feature 较复杂可拆 stdio/http 两卡 | 1 reviewer / 18 卡 ≈ 6 工作日 |
| M5 | 1（内部 3 步串行）| engine 必须最后做 | 1 reviewer / 15 卡 ≈ 5 工作日 |
| M6 | **2** | subagent / team 独立 | 1 reviewer / 10 卡 ≈ 3-4 工作日 |
| M7 | 1（串行）| sdk 门面是 single-writer | 1 reviewer / 6 卡 ≈ 2 工作日 |
| M8 | **3** | server / desktop / cli 三业务 crate 独立 | 1 reviewer / 12 卡 ≈ 4 工作日 |
| M9 | 1（串行）| POC 之间有依赖（POC-3 需 hook 完成；POC-1 需要 anthropic provider）| 1 reviewer / 8 卡 ≈ 3 工作日 |

**调度建议**（针对评审者瓶颈）：

- M2 五路并行的实际派发节奏：T01-T05（5 路核心 trait）一批 → 通过后才放出 T06-T10（5 路默认实现）
- M4 / M5 同时段进行时，必须分别使用不同 reviewer（避免单点阻塞）
- M8 三业务路并行需 3 名业务 TL 各自 review 自己路（不能共享 reviewer）
- 任何里程碑评审 backlog ≥ 5 个待审 PR 时暂停派发新任务卡，先消化 backlog

---

## 4. Review Gate（强制评审检查点）

部分里程碑后设置**人类强评审 gate**，gate 不通过则下一里程碑不得开始：

| Gate | 位置 | 检查内容 |
|---|---|---|
| **G-Bootstrap** | M0 完成后 | workspace 19 crate 拓扑符合 SPEC；旧 crate 全删；CI 矩阵就位 |
| **G-Contracts** | M1 完成后 | 全量类型与 D3 `api-contracts.md` 完全一致；`schemars` 输出 schema 与现有契约对齐 |
| **G-MVP** | M3 完成后 | 最小 SDK 闭环 E2E 跑通；可演示给业务方 |
| **G-Facade** | M7 完成后 | `prelude` 可作为业务方唯一 import 入口；feature flags 对齐 D10 |
| **G-Production** | M9 完成后 | POC 报告通过；端到端验收通过；可宣告 v1.0 |

> Gate 不通过的处理：开 retro 任务卡，分析根因；如根因是 SPEC 缺陷，回流到架构层修订；如是实施缺陷，重新派发任务卡。

---

## 5. 进度跟踪表（Maintainer 实时更新）

| 里程碑 | 任务卡总数 | 已完成 | 进行中 | AI 工时 | 评审工时 | 估算墙钟 | 状态 | 下一步 |
|---|---:|---:|---:|---:|---:|---:|---|---|
| M0 | 15 | 0 | 0 | 7h | 3.5h | 4-5d | 待启动 | 派发 M0-T01.5（含 T02 拆 a/b/c/d）|
| M1 | 12 | 0 | 0 | 8h | 4h | 4-5d | 待启动 | — |
| M2 | 28 | 0 | 0 | 26.5h | 13h | 8-10d | 待启动 | 含 M2-T02a/b、T04.5(provider stub)、T08a/b、S01 spike |
| M3 | 25 | 0 | 0 | 31h | 12h | 7-8d | 待启动 | 含 M3-T04 拆 a/b、M3-S01/S02 spike + T22 cli cutover |
| M4 | 19 | 0 | 0 | 22.5h | 8.5h | 5-6d（与 M5 并行）| 待启动 | 含 M4-T05.5 chore |
| M5 | 17 | 0 | 0 | 21h | 10.5h | 5d（与 M4 并行）| 待启动 | 含 M5-T03.5 / T09.5 chore |
| M6 | 10 | 0 | 0 | 14h | 5h | 3-4d | 待启动 | — |
| M7 | 6 | 0 | 0 | 8h | 6h | 2d | 待启动 | — |
| M8 | 12 | 0 | 0 | 16h | 8h | 4d | 待启动 | — |
| M9 | 8 | 0 | 0 | 12h | 16h | 3-5d | 待启动 | — |
| **合计** | **150** | 0 | 0 | **167h** | **86.5h** | **42-54d** | — | — |

> **总墙钟估算**：约 14-21 周（3.5-5 个月）。M4/M5 并行节省约 5-6 工作日。
> 如评审者从 1 名增至 2 名，M2/M4/M8 墙钟可压缩 30%-40%（总墙钟降至 11-15 周）。
> 任务卡总数 150 = 132（原计划）+ 18（修订增量，分布如下）：
> - M0：+7（T01.5/T01.6 + T01a-d + T02 拆 a/b/c/d，原 8 → 15）
> - M1：+2（T04 拆 a/b、T05 拆 a/b，原 10 → 12）
> - M2：+3（T02 拆 a/b、T04.5 provider stub、T08 拆 a/b、S01 spike，原 25 → 28）
> - M3：+5（T04 拆 a/b、T21 dep 预注入、T22 cli cutover、S01/S02 spike，原 20 → 25）
> - M4：+1（T05.5 chore）
> - M5：+2（T03.5、T09.5 chore）
>
> **Spike 前置策略**：把 3 个高风险 ADR 假设的失败检出从 M9 提前到 M2/M3 末，最大可省 2-4 周返工。
> **业务面渐进切换**：M3 末 cli 入口先行接入真 harness-sdk，让真集成风险从 M8 提前到 M3。

---

## 6. 风险登记

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| AI 在 trait 签名上"自由发挥" | 中 | 高 | 铁律 1（00-strategy）+ 闸门-5 SPEC grep 自检 |
| feature 矩阵爆炸（组合数 > 100）| 中 | 中 | CI 仅跑核心 8 组合 + nightly 跑全矩阵；feature-flags.md §6.1 已定 |
| Codex 上下文长度不够 | 高 | 中 | 任务卡严格 ≤ 500 行 diff；SPEC 锚点精确到行号片段 |
| 旧 SDK 删除后业务层短期崩溃 | 高 | 中 | M0 删除前先在 `apps/desktop` 关闭对旧 SDK 的依赖（M0-T05）|
| Anthropic API 真实命中率不达预期 | 中 | 高 | M9 POC-1 兜底；如果失败需要回到 ADR-003 重设计 |
| Codex 多会话并行造成 PR 冲突 | 中 | 低 | 并行任务卡声明"可能修改文件"清单（00-strategy §3.2）|
| 测试 mock 偷工 | 高 | 中 | 闸门-3 强制 contract-test 覆盖度（03-quality-gates）|

---

## 7. 索引

- **整体策略** → [`00-strategy.md`](./00-strategy.md)
- **任务卡模板** → [`02-task-template.md`](./02-task-template.md)
- **里程碑详细任务** → [`milestones/`](./milestones/)
