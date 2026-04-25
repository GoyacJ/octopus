# M5 · L3 Single-Agent · engine + observability + plugin

> 状态：待启动 · 依赖：M3 完成（与 M4 可并行）· 阻塞：M6
> 关键交付：单 Agent 主循环（engine）+ Tracer / Replay / Redactor + Plugin 宿主
> 预计任务卡：17 张 · 累计工时：AI 21 小时 + 人类评审 10.5 小时
> 并行度：1（内部 3 步串行：observability → plugin → engine）

---

## 0. 里程碑级注意事项

1. **engine 必须最后做**：依赖 observability 的 Redactor 必经管道 + plugin 的 capability 注入
2. **observability 先行**：Redactor 必经管道契约（v1.8.1 P2-7）必须在 EventStore 装配时就位
3. **plugin 二分 Loader**：ManifestLoader（发现期不执行代码）+ RuntimeLoader（激活期），ADR-0015
4. **engine 主循环 5 态**：AwaitingModel / ProcessingToolUses / ApplyingHookResults / MergingContext / Ended(StopReason)
5. **EngineRunner trait**：放在 engine crate 内（v1.8.1 P2-1 已登记于 api-contracts §14.3）

---

## 1. 任务卡总览

| Crate | 任务卡 | 内容 |
|---|---|---|
| **observability** | M5-T01 ~ T04（含 T03.5） | Tracer + Usage + Replay + Redactor 必经管道 + Journal 装配集成测试 |
| **plugin** | M5-T05 ~ T09（含 T09.5） | ManifestLoader + RuntimeLoader + TrustedSignerStore + Capability handles + Skill plugin source 集成 |
| **engine** | M5-T10 ~ T15 | LoopState 主循环 + 中断 + iteration budget + grace call + EngineRunner trait |

---

## 2. 步骤 1 · `octopus-harness-observability`

### M5-T01 · Tracer trait + OTel 实现

**SPEC 锚点**：
- `harness-observability.md` §2-§3
- `api-contracts.md` §18

**预期产物**：
- `src/lib.rs`
- `src/tracer.rs`：Tracer trait + Span / SpanContext / TraceId
- `src/otel.rs`：OtelTracer（基于 `opentelemetry` + `tracing-opentelemetry`）

**Cargo feature**：`otel`

**预期 diff**：< 350 行

---

### M5-T02 · UsageAccumulator + CostCalculator 集成

**SPEC 锚点**：`harness-observability.md` §4

**预期产物**：
- `src/usage.rs`：UsageAccumulator + UsageProjection + UsageReport
- `tests/usage.rs`

**Cargo feature**：`prometheus`（输出 metrics）

**预期 diff**：< 300 行

---

### M5-T03 · DefaultRedactor 实现 + 6 挂钩点装配契约测试（v1.8.1 P2-7）

**SPEC 锚点**：
- `api-contracts.md` §18.2（Redactor trait 权威签名 `fn redact(&str, &RedactRules) -> String`）
- `harness-observability.md` §2.5.0（6 个**装配点（call site）**：events / messages / hooks / mcp / model in / model out）
- `harness-journal.md` §2.1（EventStore 装配点：写入前调 `redact()` 处理字符串字段）

**前置任务产物**（必读 PR）：
- M1-T07 PR：`octopus-harness-contracts` `Redactor` trait（单 method）+ `NoopRedactor` + `RedactRules`
- M2-T06 PR：`octopus-harness-journal` EventStore 实现已在写路径调用 `redactor.redact(field, &rules)`

**预期产物**：
- `src/redactor.rs`：
  - `pub struct DefaultRedactor { rules_default: RedactRules, /* compiled regex set */ }`
  - 实现 contracts 层 `Redactor` trait（**单 method `fn redact(&str, &RedactRules) -> String`**）
  - 内含 30+ 默认正则模式（API key / Bearer token / private IP / SSH key / OAuth code / database connection string / etc），由 `RedactRules` 控制启用范围
  - 不再重复定义 `Redactor` trait
- `src/contract.rs`：`RedactorContractTest` 套件验证：
  - `NoopRedactor` 与 `DefaultRedactor` 都通过同一组用例
  - 同一字符串多次 `redact()` 等幂（contract-test 验证幂等性）
  - 30+ 模式全部命中（用例数据集）

> Journal 装配点 contract test 由独立 chore 卡 **M5-T03.5** 处理（实施前评估 P2-2 修订），避免与 M2-T07/T08/T09 并行 PR 共改 `harness-journal` crate。

---

### M5-T03.5 · Journal 装配 Redactor 集成测试（chore，跨 crate 文件锁卡）

| 字段 | 值 |
|---|---|
| **依赖** | M5-T03 完成 + M2 全部完成 |
| **预期 diff** | < 200 行 |
| **文件锁声明** | 独占新增 `crates/octopus-harness-journal/tests/redactor_pipeline.rs`；冲突卡：无（仅新增） |

**预期产物**：
- `octopus-harness-journal/tests/redactor_pipeline.rs`：contract test 接入到 EventStore 三种实现（jsonl / sqlite / memory）
- 验证写路径**确实调用了 `redact()`**，且关键字符串字段（如 `error_message / tool_input.text / message_text`）已被替换

**关键不变量**：
- EventStore / Hook / MCP / Model in-out 6 个装配点：调用方自行把字符串过 `redact()`（**装配契约**而非 trait 形态）
- Redactor 自身**不应失败**（returns `String`，不是 `Result`）；正则不匹配则原样返回
- `DefaultRedactor::default()` 自带 30+ 模式

**禁止行为**：
- 不要把 trait 改成多 method（违反 SPEC §18.2）
- 不要让 redact() 返回 Result（业务上 redact 不应阻塞主流程）
- 不要把 `DefaultRedactor` 的具体规则写入 contracts crate

**Cargo feature**：`redactor`

**预期 diff**：< 350 行

---

### M5-T04 · ReplayEngine + Observability Contract Test

**SPEC 锚点**：`harness-observability.md` §5（Replay）

**预期产物**：
- `src/replay.rs`：ReplayEngine（从 EventStore 取事件 → 还原 SessionProjection）
- `tests/contract.rs`：Tracer / Redactor 一致性
- `tests/replay.rs`：跑一段 session → replay → 投影一致

**Cargo feature**：`replay`

**预期 diff**：< 350 行

---

## 3. 步骤 2 · `octopus-harness-plugin`

### M5-T05 · PluginManifest + TrustLevel + 二分 Loader trait

**SPEC 锚点**：
- `harness-plugin.md` §2-§3
- `api-contracts.md` §17.2 / §17.3
- ADR-006 / ADR-0015

**预期产物**：
- `src/lib.rs`
- `src/manifest.rs`：PluginManifest + ManifestRecord + TrustLevel
- `src/loader.rs`：PluginManifestLoader + PluginRuntimeLoader trait（**二分**）

**关键不变量**：
- ManifestLoader 返回 `Vec<ManifestRecord>`（**绝不**返回 `Arc<dyn Plugin>`，避免发现期执行代码）
- RuntimeLoader 仅由 PluginRegistry::activate 调用

**预期 diff**：< 350 行

---

### M5-T06 · PluginRegistry + Activation + Capability handles

**SPEC 锚点**：`harness-plugin.md` §4 / ADR-0015 §3.5

**预期产物**：
- `src/registry.rs`：PluginRegistry + activate(plugin_id) → ActivationContext
- `src/capability.rs`：PluginActivationContext（按 manifest 声明的 capability handle 集合）

**关键不变量**：
- ActivationContext 仅注入 manifest 声明的 capability handle（type-state + 运行期双重拦截越权注册）

**预期 diff**：< 400 行

---

### M5-T07 · TrustedSignerStore + ManifestSigner（ADR-0014）

**SPEC 锚点**：`harness-plugin.md` §5 / ADR-0014

**预期产物**：
- `src/signer.rs`：ManifestSigner + TrustedSignerStore + StaticTrustedSignerStore
- `tests/signer.rs`：启用窗口 + 撤销 + 与 IntegritySigner 完全独立

**Cargo feature**：`manifest-sign`

**关键不变量**：
- 与 `harness-permission/src/integrity_signer.rs` 的 KeyStore **完全独立**（ADR-0013 / ADR-0014）

**预期 diff**：< 350 行

---

### M5-T08 · 4 源 PluginSource Discovery + dynamic-load

**SPEC 锚点**：`harness-plugin.md` §6（4 源发现：admin / user / workspace / inline）

**预期产物**：
- `src/sources/admin.rs / user.rs / workspace.rs / inline.rs`
- `src/dynamic_load.rs`：动态库加载（仅 admin-trusted 可用）
- `tests/sources.rs`

**Cargo feature**：`dynamic-load`

**预期 diff**：< 400 行

---

### M5-T09 · Plugin Contract Test

**预期产物**：
- `tests/contract.rs`：ManifestLoader / RuntimeLoader 一致性

**预期 diff**：< 200 行

> Skill plugin source 集成由独立 chore 卡 **M5-T09.5** 处理（实施前评估 P2-2 修订），避免与 M4-T07 (Skill 三 source 卡) 并行 PR 共改 `harness-skill/src/sources/`。

---

### M5-T09.5 · Skill plugin source 集成（chore，跨 crate 文件锁卡）

| 字段 | 值 |
|---|---|
| **依赖** | M5-T09 + M4-T07 完成 |
| **预期 diff** | < 150 行 |
| **文件锁声明** | 独占新增 `crates/octopus-harness-skill/src/sources/plugin.rs` + 修改 `sources/mod.rs`；冲突卡：M4-T07（必须先合） |

**预期产物**：
- `octopus-harness-skill/src/sources/plugin.rs`：与 plugin crate 集成的 PluginSkillSource
- `octopus-harness-skill/src/sources/mod.rs`：增加 `pub mod plugin;`

---

## 4. 步骤 3 · `octopus-harness-engine`

### M5-T10 · EngineRunner trait + Engine 骨架

**SPEC 锚点**：
- `harness-engine.md` §2-§3
- `api-contracts.md` §14.3（EngineRunner trait，v1.8.1 P2-1 新增）

**预期产物**：
- `src/lib.rs`
- `src/runner.rs`：EngineRunner trait（防止 subagent ↔ engine 循环依赖）
- `src/engine.rs`：Engine struct + EngineBuilder
- `src/state.rs`：LoopState 5 态枚举

**预期 diff**：< 400 行

---

### M5-T11 · 主循环（turn 编排）

**SPEC 锚点**：
- `harness-engine.md` §4
- `overview.md` §7.1（turn 流程图）

**预期产物**：
- `src/turn.rs`：run_turn 主循环
- 完整流程：UserPromptSubmit → context.assemble → model.infer → tool.execute → context.after_turn → ... → RunEnded
- 集成 SteeringQueue.drain_and_merge（在主循环安全检查点）

**关键不变量**：
- iteration budget（默认 25）+ token budget
- grace call：剩余预算 -1 时给 LLM 一次收尾机会，发出 GraceCallTriggered 事件（v1.8.1 P2-5）

**预期 diff**：< 500 行

---

### M5-T12 · 中断 + EndReason 映射

**SPEC 锚点**：
- `harness-engine.md` §5（中断节末尾的触发源映射表，v1.8.1 P1-3）

**预期产物**：
- `src/interrupt.rs`：CancellationToken + 5 来源映射（User / System / Parent / Timeout / Budget）
- `src/end_reason.rs`：EndReason 决定逻辑（含 Cancelled { initiator }）
- `tests/interrupt.rs`

**关键不变量**：
- User cancel → EndReason::Cancelled { initiator: User }
- System interrupt → EndReason::Cancelled { initiator: System }
- Parent kill → EndReason::Cancelled { initiator: Parent }

**预期 diff**：< 300 行

---

### M5-T13 · ResultBudget 集成 + 工具结果注入

**SPEC 锚点**：`harness-engine.md` §6 / ADR-0010

**预期产物**：
- `src/result_inject.rs`
- `tests/result_budget.rs`

**预期 diff**：< 250 行

---

### M5-T14 · CapabilityRegistry 装配（ADR-0011）

**SPEC 锚点**：`harness-engine.md` §8

**预期产物**：
- `src/capability_assembly.rs`：把 PermissionCap / SandboxCap / ModelCap 等装配进 ToolContext
- `tests/capability.rs`

**Cargo feature**：`subagent-tool`（启用时引入 `harness-subagent`，D2 §10 例外破窗）

**预期 diff**：< 300 行

---

### M5-T15 · Engine Contract Test + E2E（替代 M3 临时 driver）

**预期产物**：
- `crates/octopus-harness-engine/tests/contract.rs`
- `crates/octopus-harness-engine/tests/e2e_engine.rs`：完整流程 E2E（替代 M3 的临时 driver）
- **`git rm crates/octopus-harness-session/tests/e2e_minimal.rs`**（实施前评估 P1-2 强制要求；M3-T20 留下的 `TODO(M5-T15)` 警示注释引用此处）
- 文档同步：在 `docs/plans/harness-sdk/audit/M3-mvp-gate.md` 末尾追加注解"已被 M5-T15 替换"

**Gate 通过判据（M5）**：
- ✅ 3 crate 各自 `cargo test --all-features` 全绿
- ✅ E2E 用例 "Engine.run(session) → AssistantDelta → ToolUseRequested → 完成" 跑通
- ✅ GraceCallTriggered Event 在剩余 -1 预算场景正确发出
- ✅ EndReason::Cancelled { initiator } 在 3 种场景正确路由
- ✅ Redactor 必经管道契约：所有 EventStore 测试装配 `DefaultRedactor`（M5-T03 产出，替代 M2 期 NoopRedactor）后通过
- ✅ M3-T20 临时 driver 已 `git rm`（执行 `! ls crates/octopus-harness-session/tests/e2e_minimal.rs 2>/dev/null` 应失败）

未全绿 → 不得开始 M6。

---

## 5. 索引

- **上一里程碑** → [`M3-l2-core.md`](./M3-l2-core.md)（M4 可并行）
- **下一里程碑** → [`M6-l3-agents.md`](./M6-l3-agents.md)
