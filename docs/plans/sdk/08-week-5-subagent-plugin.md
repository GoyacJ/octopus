# W5 · `octopus-sdk-subagent` + `octopus-sdk-plugin`

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `docs/sdk/05-sub-agents.md` → `docs/sdk/12-plugin-system.md` → `docs/sdk/07-hooks-lifecycle.md §7.11`（插件源 hook）→ `02-crate-topology.md §2.10 / §2.11 / §5 / §6 / §8` → `03-legacy-retirement.md §3（tools::subagent_runtime / lsp_runtime）/ §4（plugins/*）/ §2.1（runtime::plugin_lifecycle / worker_boot）`。

## Status

状态：`draft`（本周第一行生产代码提交前须切 `in_progress`）。

## Active Work

当前 Task：`none`（Plan 起稿阶段；Task 1 启动需先完成 Pre-Task Checklist）

当前 Step：`none`

### Pre-Task Checklist（起稿阶段留空模板，Task 1 启动前逐条勾选）

- [ ] 已阅读本子 Plan 的 `Goal` / `Architecture` / `Scope`。
- [ ] 已阅读 `00-overview.md §1 10 项取舍`，且当前任务未违反。
- [ ] 已阅读 `docs/sdk/05 / 12 / 07 §7.11` 与本 Task 对应的规范章节。
- [ ] 已阅读 Task 段落的 `Files` / `Preconditions` / `Step*` 且无歧义。
- [ ] 已识别本 Task 涉及的 **SDK 对外公共面** 变更（是）。
  - 若"是"：已确认变更在 `02-crate-topology.md §对外公共面` 有登记项（或计划在本批次内新增登记）。
- [ ] 已识别是否涉及 `contracts/openapi/src/**` 或 `packages/schema/src/**`（预期否；若 `plugin_snapshot` / `subagent_summary` 需要出现在 OpenAPI 会话事件，则转是）。
- [ ] 已识别是否涉及 `docs/sdk/14` UI Intent IR 变更（否；本周不新增 `RenderBlock.kind`）。
- [ ] Preconditions 已全部满足；未满足项已在 `Open Questions` 中登记。
- [ ] 当前 git 工作树干净或有明确切分；本批次计划 diff ≤ 800 行（不含 generated）。
- [ ] 已识别所有 `Stop if:` 条款；遇到任一条件 → 立即停止并汇报。

### 已确认的审核决策（2026-04-21）

下列 4 项由 owner 在 Plan 起稿审核时**明确接受**，不再回翻。执行期若需变更，必须在本文件 §变更日志追加专项决策条目，或走 `docs/sdk/README.md ## Fact-Fix 勘误`。

| # | 决策点 | 确认结论 | 关联章节 |
|---|---|---|---|
| D1 | Generator-Evaluator 的 Evaluator 首版落地形态 | **仅 `MockEvaluator`**：契约先立，`trait Evaluator` + `MockEvaluator` 闭环；不引入 Playwright / sidecar 依赖。真实 Evaluator（Playwright / Snapshot / 外部 judge）通过 `PluginComponent::Evaluator` 扩展点在 Phase 2 注入。`GeneratorEvaluator::run` 的 sprint contract 契约保持可落；Evaluator 输出结构固化为 `{design_quality, originality, craft, functionality, overall_pass, next_actions}`（见 `docs/sdk/05 §5.3.3`）。 | Architecture / Task 5 |
| D2 | W5 阶段 Plugin 分发格式 | **仅本地目录**：`PluginSource::LocalDirectory` 一种。`config` 驱动的 `plugins.allow / plugins.enabled / plugins.slots.*` 三个字段最小实现。`git / npm / github / url / .mcpb / marketplace` 全部延至 Phase 2（W8 后），不纳入 W5 Task Ledger；manifest schema 中预留 `source` 字段但 Deserialize 侧拒绝非 `local` 以外的变体（返回 `PluginError::UnsupportedSource`）。 | Architecture / Task 7 |
| D3 | `worker_boot.rs` / `subagent_runtime.rs` 的 W5 边界 | **W5 不再把这两处当成 subagent SDK 的实现来源**：当前 `crates/runtime/src/worker_boot.rs` 是 trust gate / ready-for-prompt / prompt-misdelivery 控制面；`crates/tools/src/subagent_runtime.rs` 只剩 TODO stub。两者都不承载本 Plan 所需的 fan-out → fan-in / condensed summary 抽象。W5 `octopus-sdk-subagent` 按 `docs/sdk/05-sub-agents.md` + 当前 SDK crate 边界**绿色实现**；`worker_boot` 的业务控制面与团队态映射留到 W7 再处理。 | Architecture / Task 2 / Task 3 / §退役登记 |
| D4 | Agent Definition 发现路径 | **统一走 `<project>/.agents/**/*.md` + `<workspace>/.agents/**/*.md`**（workspace 覆盖 project，project 覆盖 bundled；与 `docs/sdk/05 §5.6` 一致）。SDK 侧 `AgentRegistry::discover(roots: &[PathBuf])` 只接受外部传入的 `roots`，不硬编码路径；`octopus-platform` 负责在业务层拼出 `roots`。YAML frontmatter 解析走 `serde_yaml` + snake_case → camelCase 映射（`task_budget → taskBudget.total`）。 | Architecture / Task 6 |

## Goal

产出 **2 个零业务语义的新 SDK crate** —— `crates/octopus-sdk-subagent`（Level 3）/ `crates/octopus-sdk-plugin`（Level 2），落地 `docs/sdk/05 / 12` 的：

1. **子代理层** · `SubagentSpec / SubagentContext / SubagentOutput / SubagentError` + `OrchestratorWorkers`（fan-out → fan-in，默认 5 并发，condensed summary / file ref 回传，深度 ≤ 2）+ `GeneratorEvaluator`（`Planner → Generator → Evaluator` 三段，Evaluator 独立上下文）+ `AgentRegistry`（`.agents/**/*.md` frontmatter 解析）。
2. **插件层** · `PluginManifest`（Zod 风格 + `compat.pluginApi` 范围校验）+ `PluginRegistry`（12 类扩展点的登记表 + 单向流）+ `PluginLifecycle`（`discover → enablement → load → register → expose` 五段 + 三道安全门）+ `PluginError`（22 型 discriminated union 子集）+ **声明层 / 运行时层拆分**（`ToolDecl` / `HookDecl` 只做静态元数据；真正进入执行路径的必须是可执行 runtime registration）。
3. **合流收口** · `crates/plugins/*` + `runtime::plugin_lifecycle` + `runtime::hooks` 的"四源"在本周 **逻辑合一**（新 SDK 实现成为唯一真相源，legacy 仍保留引用但不再被新代码消费）；真正的 legacy 文件删除归 W7。
4. **Session 快照契约** · `plugins_snapshot: PluginsSnapshot` 是 W5 的**前置合同硬门禁**，不是尾部 session 小补丁：必须先扩 `SessionEvent::SessionStarted`、`SessionSnapshot`、`SessionStore` 承载面；默认内嵌在 `SessionStarted`，若 W1 首事件无法扩面，则退回紧随其后的 `session.plugins_snapshot` 次事件，再做 store/replay/OpenAPI 对齐。

本周 **不** 实现 Brain Loop dispatch 流水线（归 W6），也 **不** 从 legacy crate 删除文件（归 W7）。W5 只保证"新 SDK 实现到位 + legacy 停止被 SDK 新代码 `use`"。

## Architecture

### 1. Level / 依赖方向

- **`octopus-sdk-subagent` = Level 3**（编排层），可依赖 Level 0–2 全部：
  - Level 0：`octopus-sdk-contracts::{SubagentSpec, SubagentOutput, SubagentError, PermissionMode, AskPrompt, RenderBlock, SessionId, EventId, Message}`。
  - Level 1：`octopus-sdk-model::{ModelProvider, RoleRouter, FallbackPolicy}`（子代理路由；`SubagentSpec.model_role` 在 Level 0 只保存 opaque key，运行时再由 `sdk-model` 解析）；`octopus-sdk-session::{SessionStore}`（子代理独立 session 快照）。
  - Level 2：`octopus-sdk-tools::{ToolRegistry, ToolContext}`（白名单过滤）；`octopus-sdk-permissions::{PermissionGate, PermissionPolicy}`（最小权限）；`octopus-sdk-context::{SystemPromptBuilder, DurableScratchpad}`（scratchpad hint）；`octopus-sdk-hooks::{HookRunner}`（子代理自己跑自己的 Hook 链）。
  - **禁止**：`subagent → plugin` 直接 `use`（同 Level 3 不允许引用 Level 2 `plugin`；子代理与插件通过 Brain Loop 协作，不跨 crate 直接持有 handle）。

- **`octopus-sdk-plugin` = Level 2**，与 `tools / mcp / hooks / sandbox / permissions / ui-intent` 同层：
  - 依赖 Level 0 `octopus-sdk-contracts`（数据契约；`ModelProviderDecl` 仅保存 opaque metadata）+ **同层 runtime surface** `octopus-sdk-tools::{Tool, ToolRegistry}` / `octopus-sdk-hooks::{Hook, HookRunner, HookSource}`。
  - **禁止**：`plugin → sandbox / permissions / subagent / core` 直接 `use`。W5 允许 `plugin → tools / hooks` 的同层依赖，因为当前执行路径的真实注册表就位于这两个 crate 内；前提是不得引入反向循环。

### 2. 声明层 / 运行时层拆分（W5 的关键契约）

W5 不再把 `ToolDecl` / `HookDecl` 当成"注册后即可执行"的对象。插件层拆成两层：**Level 0 declaration** 只描述静态元数据；**Level 2 runtime registration** 才把工具和 hook 真正接入执行路径。

```rust
// Level 0: static declaration
pub struct ToolDecl { /* id / name / description / schema / source */ }
pub struct HookDecl { /* id / point: HookPoint / source */ }

// Level 2: executable runtime registration
pub struct PluginToolRegistration {
    pub decl: ToolDecl,
    pub tool: Arc<dyn Tool>,
}

pub struct PluginHookRegistration {
    pub decl: HookDecl,
    pub hook: Arc<dyn Hook>,
    pub source: HookSource,
    pub priority: i32,
}

pub struct PluginApi<'a> {
    pub tools: &'a mut ToolRegistry,
    pub hooks: &'a HookRunner,
    // W5: skills / model providers 仍先走 metadata + builder handoff
}
impl PluginApi<'_> {
    pub fn register_tool(&mut self, reg: PluginToolRegistration) -> Result<(), PluginError>;
    pub fn register_hook(&mut self, reg: PluginHookRegistration) -> Result<(), PluginError>;
}
```

- `ToolDecl / HookDecl / SkillDecl / ModelProviderDecl` **定义在 `octopus-sdk-contracts`**；它们只用于 manifest、诊断、snapshot、兼容性检查，不承担执行。
- 插件工具和 hook 只有在 `PluginApi` 收到 `PluginToolRegistration` / `PluginHookRegistration` 这类**可执行记录**后，才算进入 runtime 执行路径；只写 declaration 不算 load/register 完成。
- `SkillDecl` / `ModelProviderDecl` 在 W5 先保留为 metadata + builder slot handoff，不伪装成已经接上 runtime 执行面。
- W6 `octopus-sdk-core::AgentRuntimeBuilder` 负责把 concrete `ToolRegistry` / `HookRunner` 组装进 `PluginLifecycle`；不再要求 `sdk-tools` / `sdk-hooks` 反向实现 `octopus-sdk-plugin` 自定义 registration trait。

### 3. 子代理独立上下文窗口（不变量 #1 延续）

- **每个子代理一次 `OrchestratorWorkers::run_worker(spec, input)` 启动一个"短周期 session"**，对应 `SessionStore::new_child_session(parent_id, spec)`；子 session 独立写入 JSONL + SQLite（延续 W1 双通道），父 session 只收到 `SubagentOutput` 并把其 `summary` 以 `RenderKind::Text / Markdown` 写入父事件流。
- **Token Budget**：`SubagentSpec.task_budget.total: u32`（`quick=10_000 / medium=40_000 / very_thorough=150_000` 的外部映射由业务层决定），W5 只负责在 `SubagentContext.on_turn_end` 累加 token，超过 `COMPLETION_THRESHOLD = 0.9 * budget` 时写入 `subagent_budget_exceeded` 事件并由子代理 orchestrator 自行收尾（不强杀）。
- **并发**：`OrchestratorWorkers::new(max_concurrency: usize)`，默认 5；超出时 `tokio::sync::Semaphore` 排队。父代理自己的工具并发不共享 semaphore。
- **深度**：`SubagentContext.depth: u8`，深度 `> 2` 时 `run_worker` 返回 `SubagentError::DepthExceeded { depth }`。
- **大输出走 file ref**：`SubagentOutput::FileRef { path: PathBuf, bytes: u64 }`（`> 4_096 bytes` 自动转 file ref，写入 `runtime/notes/subagent-<id>.md` via `DurableScratchpad`）。

### 4. Generator-Evaluator（`docs/sdk/05 §5.3`）

- **三段显式**：`Planner::expand(user_prompt) -> SprintContract` → `Generator::run(contract) -> Draft` → `Evaluator::judge(draft) -> Verdict`。`Verdict::fail { reasons, next_actions }` 时回到 `Generator::run(contract.with_feedback(verdict))`；最多 `max_rounds` 轮（默认 3）；均失败返回 `SubagentError::EvaluatorExhausted`。
- **Evaluator 独立上下文**：`Evaluator::judge` 参数只含 `Draft`（不含 Generator 的推理过程），在 `SubagentContext::for_evaluator(parent_spec, draft)` 产出的独立子 session 内执行。
- **Mock 闭环（D1）**：W5 Task 5 交付 `MockEvaluator { rubric: fn(&Draft) -> Verdict }`，契约测试用固定脚本校验 "fail → feedback → pass" 路径。
- **Sprint contract 字段**：`{ scope: String, done_definition: String, out_of_scope: Vec<String>, invariants: Vec<String> }`；Generator / Evaluator 双方持有同一契约的序列化副本，契约变更视为新 sprint。

### 5. Plugin Lifecycle 五段 + 三道安全门

- **discover**：输入 `PluginDiscoveryConfig { roots: Vec<PathBuf>, allow: Vec<PluginId>, deny: Vec<PluginId> }`；递归扫描每个 root（`<root>/*/plugin.json`），排除 `deny` 命中；**W5 只支持本地目录**（D2），`source: local` 以外一律 `PluginError::UnsupportedSource`。
- **enablement**：
  1. `plugin.json` zod 风格 schema 校验（`octopus-sdk-plugin::schema`）；
  2. `compat.pluginApi` semver range 对比 `SDK_PLUGIN_API_VERSION: &str = "1.0.0"`；major 不匹配 → `PluginError::IncompatibleApi`；
  3. **三道安全门**：
     - 路径逃逸（所有 manifest 引用路径 symlink resolve 后必须留在 plugin root 内）；
     - 世界可写（非 bundled 插件的文件 mode `& 0o002 == 0` 必须成立）；
     - 保留名 / 非 ASCII 名（`name` 必须 `[a-z0-9-]+`，长度 ≤ 64）；
  4. `allow` 白名单（若配置 `plugins.allow` 非空，只加载命中者）。
- **load**：Native plugin 本周采用 **静态注册**（`bundled_plugins()` 返回 `Vec<Box<dyn Plugin>>`）；不引入动态 `libloading`（延至 Phase 2）。`register(api)` 被调用一次。
- **register**：插件调用 `api.register_tool(PluginToolRegistration)` / `register_hook(PluginHookRegistration)`；`PluginRegistry` 同步写 declaration index 并把 executable record 注入 `ToolRegistry` / `HookRunner`。`SkillDecl` / `ModelProviderDecl` 在 W5 只登记元数据与 builder slot。
- **expose**：`PluginRegistry::get_snapshot() -> PluginsSnapshot` 提供给 `SessionStore` 作为 session start 持久化输入；默认内嵌在 `SessionStarted`，若首事件无法扩面则作为紧随其后的 `session.plugins_snapshot` 次事件载荷。

### 6. Session 快照契约（硬门禁）

- 新契约类型（Level 0）：
  ```rust
  pub struct PluginsSnapshot {
      pub api_version: String,
      pub plugins: Vec<PluginSummary>,
  }
  pub struct PluginSummary {
      pub id: String,
      pub version: String,
      pub git_sha: Option<String>,
      pub source: PluginSourceTag,   // Local / Bundled
      pub enabled: bool,
      pub components_count: u16,
  }
  pub enum PluginSourceTag { Local, Bundled }
  ```
- `SessionStore::append_session_started(..., plugins_snapshot: Option<PluginsSnapshot>)` 在 W1 双通道写入；默认分支把快照内嵌进首事件，fallback 分支写 `SessionStarted { plugins_snapshot: None }` 后紧随追加 `session.plugins_snapshot` 次事件；replay 契约测试：构造 2 个 plugin + 固定 config，快照 JSON 字节稳定。
- **不变量**：`PluginsSnapshot` 不序列化 `setup.providerAuthEnvVars` 值（只记 key 名），延续 W4 凭据零暴露合同。

### 7. 四源合一

| 源 | 本周动作 |
|---|---|
| `crates/plugins/src/{manifest.rs, discovery.rs, lifecycle.rs}` | **逻辑移位**：新 SDK 成为唯一入口；legacy `PluginManager` 内部改为包装 `octopus-sdk-plugin::PluginRegistry`（W7 前 adapter 薄壳）。 |
| `crates/plugins/src/{hooks.rs, hook_dispatch.rs}` | **停止被引用**：legacy `HookEvent` / `HookRunner` 已在 W4 归 `octopus-sdk-hooks`；W5 只清 `PluginTool` 类型对 `sdk-hooks::Hook` 的直接映射，确认 `crates/plugins` 内无新代码 `use runtime::hooks`。 |
| `crates/runtime/src/plugin_lifecycle.rs` | **停止被引用**：legacy `PluginLifecycle` 仅保留 adapter 过渡；W5 检查 `rg "use runtime::plugin_lifecycle" crates/` 仅 adapter 命中。 |
| `crates/runtime/src/hooks.rs` | 由 W4 归并；W5 不再碰（守护扫描复核）。 |

### 8. 承 W4 / 启 W6-W7 的契约链

- **承 W4**：
  - `SubagentSpec.permission_mode: PermissionMode`（复用 W4 四态枚举）；
  - `SubagentContext.tools: Arc<ToolRegistry>`（从父 runtime `ToolRegistry` 派生 filtered child registry，兼顾 prompt 构建与实际 dispatch）；
  - `SubagentContext.permissions: Arc<dyn PermissionGate>`（父 gate 的过滤子集；`allowed_tools` 命中 `alwaysAllowRules`，其余 `alwaysAskRules` 或 `Deny`）；
  - `SubagentContext.hooks: Arc<HookRunner>`（子代理的 Hook 链独立注册，不继承父 Hook）；
  - 插件注册的 Hook 默认 `source: Plugin`，priority 由 manifest 指定，与 W4 `session > project > workspace > defaults` 叠加为 `session > project > plugin > workspace > defaults`（Plugin 夹在 project 与 workspace 之间；在 `02 §5` 登记）。
- **启 W6**：
  - `AgentRuntimeBuilder::with_plugin_registry(registry)` / `.with_subagent_orchestrator(orch)` 两个槽位本周只暴露 trait 签名，W6 Brain Loop 消费；
  - `OrchestratorWorkers` 与 `AgentTool`（W3 builtin）通过 `TaskFn: Fn(&SubagentSpec, &str) -> Result<SubagentOutput, SubagentError>` 契约契合；本周在 `sdk-tools::AgentTool` 内补一个 `Arc<dyn TaskFn>` 注入点（不修改 W3 签名，只新增 setter）。
- **启 W7**：
  - `crates/plugins` / `crates/runtime::plugin_lifecycle / worker_boot` 的"业务面"退役在 W7 业务切换时完成；W5 只做 SDK 面。

## Scope

- In scope：
  - 新建 2 个 crate 骨架（`Cargo.toml` / `src/lib.rs` / `tests/`），同批更新顶层 `Cargo.toml` `members`。
  - `02 §2.1` 追加 Level 0 contracts 补丁（`SubagentSpec / SubagentOutput / SubagentError / SubagentSummary / SprintContract / Verdict / PluginsSnapshot / PluginSummary / PluginSourceTag / ToolDecl / HookDecl / HookPoint / SkillDecl / ModelProviderDecl / PluginError 薄形状`）。
  - `02 §2.10 / §2.11` 全部数据符号落地 + 本周签名回填。
  - `OrchestratorWorkers::run / run_worker / fan_in`（默认 5 并发 + 深度 ≤ 2 + file ref 分叉）。
  - `GeneratorEvaluator::run`（`max_rounds=3` 循环 + MockEvaluator 闭环）。
  - `AgentRegistry::discover`（`serde_yaml` frontmatter + snake_case/camelCase 映射 + id 冲突拒绝）。
  - `PluginManifest` zod 风格 schema + 三道安全门（路径逃逸 / 世界可写 / 保留名）+ `compat.pluginApi` semver 校验。
  - `PluginRegistry` 12 类扩展点登记表最小实现（`tools / hooks / skills / agents / commands / mcp_servers / lsp_servers / model_providers / channels / context_engines / memory_backends / output_styles`；W5 只有 `tools / hooks` 真正接上 runtime 执行路径，`skills / model_providers` 保留 metadata + builder handoff，其余落 `Map<String, Decl>` 元信息）。
  - `PluginLifecycle::run(discover → enablement → load → register → expose)` 端到端契约测试。
  - **前置合同任务**：先扩 `SessionEvent::SessionStarted`、`SessionSnapshot`、`SessionStore` 承载 `plugins_snapshot`；默认走首事件内嵌，若 W1 首事件不能扩面则登记 `session.plugins_snapshot` 次事件；随后再做 store 实现、replay 契约测试、OpenAPI/schema 对齐。
  - `bundled/` 目录新增 **1 个最小 native plugin 示例**（`bundled/example-noop-tool/` 注册 1 个 `noop` tool），验证 `PluginLifecycle` 端到端。
  - **合同测试**（硬门禁）：
    - 父子独立上下文 + condensed summary（Task 3）；
    - Generator / Evaluator 独立上下文（Task 5）；
    - Plugin manifest 零执行校验（Task 7）；
    - 四源合一守护扫描（Task 11）；
    - Plugin session 快照 replay（Task 10）。
  - `02 §5 契约差异清单` 追加 W5 新增条目：
    - Hook source 优先级新增 `Plugin` 段（与 W4 叠加语义）；
    - `plugins_snapshot` 事件载荷与 `contracts/openapi/src/**` 的 `RuntimeSessionEvent` 对齐（若业务侧暴露；默认首事件内嵌，fallback 为紧随 `session.started` 的 `session.plugins_snapshot` 次事件）。
  - `README.md §文档索引` 同批次切 `W5: pending → draft / in_progress`；Weekly Gate 通过后切 `done`。

- Out of scope：
  - **Brain Loop dispatch 流水线**（`docs/sdk/03 §3.5.1` 8 阶段）→ 归 W6 `octopus-sdk-core`。
  - **W5 不删除** `crates/plugins/*` / `crates/runtime/src/plugin_lifecycle.rs` / `crates/runtime/src/worker_boot.rs` / `crates/tools/src/subagent_runtime.rs / lsp_runtime.rs / skill_runtime.rs` 的原文件。它们 **归 W7** 整体下线；W5 只停止 SDK 新代码的 `use`。
  - **Plugin 分发格式**：`git / npm / github / url / .mcpb / marketplace` 全部 Phase 2（D2）。
  - **Evaluator 真实实现**：Playwright / Snapshot / 外部 judge 全部 Phase 2（D1）。
  - **动态 plugin load**（`libloading` / `wasmtime`）→ Phase 2。
  - **业务域符号**：`WorkerPromptTarget / WorkerFailureKind / WorkerTrustResolution / classify_lane_failure / maybe_commit_provenance / lane / team / green-contract / trust-resolver` 等业务域治理语义 → 归 `octopus-platform::{team, governance}`（D3）。
  - **Slot 机制实际仲裁**：`plugins.slots.contextEngine / memoryBackend / primaryProvider` 只落字段，仲裁留 W6 Brain Loop。
  - **热重载 `/reload-plugins`**：留 W7（CLI 接线时一起）。
  - **MCPB / OAuth 签名校验**：留 Phase 2。
  - **Plugin 诊断 CLI（`octopus plugins inspect / doctor`）**：留 W6 / W7（CLI 入口就绪后）。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | **`sdk-plugin` Level 2 与 `sdk-subagent` Level 3 跨层契约**：`PluginComponent::Agent(AgentDecl)` 需要引用 `SubagentSpec`。 | `AgentDecl` **不**直接持有 `SubagentSpec`，只持有 `manifest_path: PathBuf` 与 `name: String`；真实 `SubagentSpec` 由 `AgentRegistry::load(&AgentDecl)` 按需解析（Level 3 消费 Level 2 的 declaration，方向合法）。 | 若 `AgentDecl` 需要强类型 `SubagentSpec` 字段 → Stop #2（跨层耦合） |
| R2 | **Plugin Hook 注册与 W4 `source` 优先级冲突**：W4 已定义 `session > project > workspace > defaults` 四段；W5 插件 Hook 插在哪段？ | 插在 `project > PLUGIN > workspace` 之间（plugin manifest 可声明 `source: "project"` 覆盖到更高段，但默认 `plugin` 独立段）。`02 §5` 登记为对 W4 语义的延伸，不是破坏性变更。 | 若 owner 认为插件应低于 workspace → 改为 `workspace > PLUGIN > defaults`，走 Fact-Fix |
| R3 | **子代理的父 Hook 继承**：子代理是否执行父代理注册的 Hook？ | **默认不继承**。子代理独立上下文 → 独立 `HookRunner`（只含 bundled + plugin source）。若父代理 prompt/工具链必须经审计，审计 Hook 应注册为 `source: plugin` 或 `workspace` 以便对子代理生效。文档上明确这是"独立上下文的代价"。 | 若业务方要求默认继承 → 走 Fact-Fix，加 `SubagentSpec.inherit_parent_hooks: bool` 字段（默认 false） |
| R4 | **Agent Definition frontmatter 解析的 `max_turns / task_budget` 单位**：`task_budget: 40000` 是 token 还是 cost？ | token（input+output 合计）。`docs/sdk/05 §5.5.2` 未明确，本 Plan 固化为 token。cost-based budget 由业务侧 `octopus-platform` 在 `PlatformServices` 层包一层换算。 | 若 W6 E2E 发现与 `UsageLedger` 单位不一致 → 走 Fact-Fix |
| R5 | **`crates/plugins` 内 `split_module_tests.rs`（1160 行）迁移路径**：测试集合庞大，本周要不要全拆？ | **W5 不拆**：`split_module_tests.rs` 归 W7 整体下线；W5 在 `octopus-sdk-plugin/tests/` 下**独立重写** `discover.rs / lifecycle.rs / registry.rs / manifest.rs / snapshot.rs` 五个测试文件（每个 ≤ 300 行），不从 legacy 迁移。legacy 测试随 W7 crate 一并删除。 | 若 Task 11 合同测试与 legacy 测试覆盖率出现回退 → 登记到 `§变更日志` 但不阻断 Weekly Gate |
| R6 | **`OrchestratorWorkers` 的 file ref 分叉阈值 4_096 bytes 来源** | Anthropic `docs/sdk/05 §5.4.2` 表格提到"大输出必须走 file ref"，未给具体阈值。本 Plan 定 `4_096`（约等于 1K tokens，保守），写入 `octopus-sdk-subagent::config::FILE_REF_THRESHOLD`。后续根据 W6 E2E cache 命中率再调。 | 若 W6 cache 命中率下降 > 20% → 调为 `2_048` 或走 Fact-Fix |
| R7 | **`PluginsSnapshot` 序列化稳定性**：插件 `components_count` 依赖注册顺序。 | `PluginRegistry::get_snapshot()` 内部按 `id` 字典序排序；`components_count` 取 `components.len()`；序列化 3 次字节一致作为契约测试。`SessionEvent::SessionStarted` / `SessionSnapshot` / store fixture 同批次一起更新，不拆尾巴。 | 若顺序不稳定 → Stop #10 |
| R8 | **`sdk-subagent` 引用 `sdk-session` 创建子 session 是否违反"不反向依赖"** | Level 3 `subagent` → Level 1 `session`：合法（下层被上层引用）。实现上 `SubagentContext` 持有 `Arc<dyn SessionStore>`，业务层在 Builder 阶段注入；不在 SDK 内部直接打开 SQLite。 | 若内部出现 `use rusqlite` → Stop #2 |
| R9 | **`PluginError` 22 型是否本周全落？** | **否**。本周只落 10 型子集：`path_not_found / manifest_parse_error / manifest_validation_error / incompatible_api / plugin_not_found / dependency_unsatisfied / duplicate_id / path_escape / world_writable / unsupported_source`；其余 12 型延 W7/Phase 2，登记在 `02 §10 变更日志`。 | 若 W6 E2E 必须返回未落变体 → 本文件 §变更日志 append 延迟条目 |
| R10 | **`AgentTool` 与 `OrchestratorWorkers` 的 `TaskFn` 注入**：W3 `AgentTool::new` 签名不含 `TaskFn`。 | 本周新增 `AgentTool::with_task_fn(self, f: Arc<dyn TaskFn>) -> Self` builder 方法，不修改原构造签名；`02 §2.4` 追加 `AgentTool::with_task_fn` 登记；默认 `TaskFn` 在 W3 测试下走 `ErrorTaskFn { reason: "TaskFn not injected" }`，避免破坏 W3 既有测试。 | 若 `AgentTool::execute` 必须强制依赖 `TaskFn` → 本文件 §变更日志 append 专项决策 |
| R11 | **`bundled/example-noop-tool/` 目录落在哪里** | 放在 `crates/octopus-sdk-plugin/bundled/example-noop-tool/`（SDK crate 自带示例），而不是 `crates/plugins/bundled/`（legacy）。`PluginDiscoveryConfig::default_roots()` 在 test 场景下指向此路径；生产配置由业务侧注入。 | 若 Clippy 抱怨 `crates/octopus-sdk-plugin/bundled/**` 不是 Rust 源码 → `include_str!` 或 build.rs 处理 |
| R12 | **`SubagentOutput::FileRef` 的 path 相对/绝对性** | 相对 `DurableScratchpad::base` 的相对路径（`runtime/notes/subagent-<id>.md`）；业务侧 replay 时需 join base。契约测试 assert 路径不以 `/` 开头。 | 若 W6 Brain Loop 需要绝对路径 → `SubagentOutput::FileRefAbsolute` 延后追加变体 |
| R13 | **`HookPoint` 与 `HookEvent` 的语义分野被后续实现混用** | 在 Level 0 明确 `HookPoint` 是声明层枚举，`HookEvent` 是 runtime payload；补 `impl HookPoint { pub fn of(event: &HookEvent) -> Self; pub fn variants() -> &'static [HookPoint]; }` 之类 helper，并加合同测试断言两侧枚举数同步。 | 若 W6 新增 `HookEvent` 变体但未同步 `HookPoint`/映射 helper → Stop #8 |

## 承 W4 / 启 W6-W7 的契约链

> 见 §Architecture §8。Task 1–11 合入批次必须逐条在 `02-crate-topology.md` 回填；Task 12 Weekly Gate 时复核。

## 本周 `02 §2.1 / §2.4 / §2.10 / §2.11` 公共面修订清单（同批次回填）

> 以下修订必须在 Task 1 / Task 2 / Task 5 / Task 6 / Task 7 / Task 10 合入批次内**同 PR** 回填到 `02-crate-topology.md`，否则视为 `Stop Condition #1`（公共面裸增）。

### `02 §2.1 octopus-sdk-contracts`（W5 补丁 · Level 0 下沉）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 1 | `§2.1` 新增类型 | 类型新增 | `SubagentSpec { id: String, system_prompt: String, allowed_tools: Vec<String>, model_role: String, permission_mode: PermissionMode, task_budget: TaskBudget, max_turns: u16, depth: u8 }`（`model_role` 在 Level 0 只保存 opaque key，运行时再由 `sdk-model` 解析） |
| 2 | `§2.1` 新增类型 | 类型新增 | `TaskBudget { total: u32, completion_threshold: f32 }`（`completion_threshold` 默认 `0.9`） |
| 3 | `§2.1` 新增类型 | 类型新增 | `SubagentOutput { Summary { text: String, meta: SubagentSummary }, FileRef { path: PathBuf, bytes: u64, meta: SubagentSummary }, Json { value: serde_json::Value, meta: SubagentSummary } }` |
| 4 | `§2.1` 新增类型 | 类型新增 | `SubagentSummary { session_id: SessionId, turns: u16, tokens_used: u32, duration_ms: u64, trace_id: String }` |
| 5 | `§2.1` 新增类型 | 类型新增 | `SubagentError { DepthExceeded { depth: u8 }, BudgetExceeded { used: u32, total: u32 }, EvaluatorExhausted { rounds: u16 }, Permission { reason: String }, Provider { reason: String }, Storage { reason: String } }` |
| 6 | `§2.1` 新增类型 | 类型新增 | `SprintContract { scope: String, done_definition: String, out_of_scope: Vec<String>, invariants: Vec<String> }` |
| 7 | `§2.1` 新增类型 | 类型新增 | `Verdict { Pass { notes: Vec<String> }, Fail { reasons: Vec<String>, next_actions: Vec<String> } }` |
| 8 | `§2.1` 新增类型 | 类型新增 | `PluginsSnapshot { api_version: String, plugins: Vec<PluginSummary> }` |
| 9 | `§2.1` 新增类型 | 类型新增 | `PluginSummary { id: String, version: String, git_sha: Option<String>, source: PluginSourceTag, enabled: bool, components_count: u16 }` |
| 10 | `§2.1` 新增类型 | 类型新增 | `PluginSourceTag { Local, Bundled }`（W5 仅两值；后续扩展） |
| 11 | `§2.1` 新增类型 | 类型新增 | `ToolDecl { id: String, name: String, description: String, schema: serde_json::Value, source: DeclSource }`、`HookDecl { id: String, point: HookPoint, source: DeclSource }`、`HookPoint { PreToolUse, PostToolUse, Stop, SessionStart, SessionEnd, UserPromptSubmit, PreCompact, PostCompact }`、`SkillDecl { id: String, manifest_path: PathBuf }`、`ModelProviderDecl { id: String, provider_ref: String }`、`DeclSource { Bundled, Plugin { plugin_id: String } }`（`provider_ref` 在 Level 0 只保存 opaque key） |
| 12 | `§2.1` 新增类型 | 类型新增 | `PluginErrorKind { PathNotFound, ManifestParseError, ManifestValidationError, IncompatibleApi, PluginNotFound, DependencyUnsatisfied, DuplicateId, PathEscape, WorldWritable, UnsupportedSource }`（W5 10 型子集） |

### `02 §2.4 octopus-sdk-tools`（W5 补丁）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 13 | `§2.4 AgentTool` | 方法新增 | `pub fn with_task_fn(self, f: Arc<dyn TaskFn>) -> Self`（builder setter；不改 W3 原签名） |
| 14 | `§2.4 TaskFn` | trait 新增 | `pub trait TaskFn: Send + Sync { async fn run(&self, spec: &SubagentSpec, input: &str) -> Result<SubagentOutput, SubagentError>; }` |

### `02 §2.10 octopus-sdk-subagent`（Level 3 · 本周落地）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 15 | `§2.10 OrchestratorWorkers` | 方法补齐 | `pub fn new(max_concurrency: usize) -> Self / pub async fn run(&self, specs: Vec<SubagentSpec>, inputs: Vec<String>) -> Vec<Result<SubagentOutput, SubagentError>> / async fn run_worker(&self, spec: SubagentSpec, input: String) -> Result<SubagentOutput, SubagentError> / fn fan_in(outputs: Vec<SubagentOutput>) -> SubagentOutput` |
| 16 | `§2.10 GeneratorEvaluator` | 方法补齐 | `pub fn new(planner: Arc<dyn Planner>, generator: Arc<dyn Generator>, evaluator: Arc<dyn Evaluator>, max_rounds: u16) -> Self / pub async fn run(&self, user_prompt: &str) -> Result<SubagentOutput, SubagentError>` |
| 17 | `§2.10 trait` | trait 新增 | `pub trait Planner: Send + Sync { async fn expand(&self, prompt: &str) -> Result<SprintContract, SubagentError>; }`、`pub trait Generator: Send + Sync { async fn run(&self, contract: &SprintContract, feedback: Option<&Verdict>) -> Result<Draft, SubagentError>; }`、`pub trait Evaluator: Send + Sync { async fn judge(&self, draft: &Draft) -> Result<Verdict, SubagentError>; }` |
| 18 | `§2.10 Draft` | 类型新增 | `pub struct Draft { pub content: SubagentOutput, pub metadata: serde_json::Value }` |
| 19 | `§2.10 SubagentContext` | 结构新增 | `pub struct SubagentContext { pub session_store: Arc<dyn SessionStore>, pub model: Arc<dyn ModelProvider>, pub tools: Arc<ToolRegistry>, pub permissions: Arc<dyn PermissionGate>, pub hooks: Arc<HookRunner>, pub scratchpad: Arc<DurableScratchpad>, pub parent_session: Option<SessionId>, pub depth: u8 }` |
| 20 | `§2.10 AgentRegistry` | 结构新增 | `pub struct AgentRegistry { /* 私有字段 */ } impl AgentRegistry { pub fn discover(roots: &[PathBuf]) -> Result<Self, SubagentError>; pub fn get(&self, name: &str) -> Option<&SubagentSpec>; pub fn list(&self) -> Vec<&SubagentSpec>; }` |
| 21 | `§2.10 MockEvaluator` | 类型新增（tests 专用） | `#[cfg(any(test, feature = "test-utils"))] pub struct MockEvaluator { /* ... */ }` |
| 22 | `§2.10 FILE_REF_THRESHOLD` | 常量新增 | `pub const FILE_REF_THRESHOLD: usize = 4_096;` |

### `02 §2.11 octopus-sdk-plugin`（Level 2 · 本周落地）

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 23 | `§2.11 PluginManifest` | 结构补齐 | `pub struct PluginManifest { pub name: String, pub version: semver::Version, pub description: Option<String>, pub author: Option<Author>, pub compat: PluginCompat, pub components: Vec<PluginComponent>, pub dependencies: Vec<DependencyRef>, pub source: PluginSourceTag }` |
| 24 | `§2.11 PluginCompat` | 结构补齐 | `pub struct PluginCompat { pub plugin_api: semver::VersionReq, pub min_host_version: Option<semver::Version> }` |
| 25 | `§2.11 PluginComponent` | 枚举补齐 | `Tool(ToolDecl) / Hook(HookDecl) / Skill(SkillDecl) / Agent(AgentDecl) / Command(CommandDecl) / McpServer(McpServerDecl) / LspServer(LspServerDecl) / ModelProvider(ModelProviderDecl) / Channel(ChannelDecl) / ContextEngine(ContextEngineDecl) / MemoryBackend(MemoryBackendDecl) / OutputStyle(OutputStyleDecl)`（W5 只有 `Tool / Hook` 接入真实执行路径；`Skill / ModelProvider` 保留 metadata + builder handoff；其余落元信息） |
| 26 | `§2.11 PluginRegistry` | 方法补齐 | `pub fn new(tools: ToolRegistry, hooks: HookRunner, ...) -> Self / pub fn register_plugin(&mut self, manifest: PluginManifest) -> Result<(), PluginError> / pub fn get_snapshot(&self) -> PluginsSnapshot / pub fn tools(&self) -> &ToolRegistry / pub fn hooks(&self) -> &HookRunner` |
| 27 | `§2.11 PluginLifecycle` | 结构补齐 | `pub struct PluginLifecycle { /* ... */ } impl PluginLifecycle { pub fn new(config: PluginDiscoveryConfig) -> Self; pub async fn run(&self, registry: &mut PluginRegistry, plugins: &[Box<dyn Plugin>]) -> Result<(), PluginError>; }` |
| 28 | `§2.11 Plugin trait` | trait 新增 | `pub trait Plugin: Send + Sync { fn manifest(&self) -> &PluginManifest; fn register(&self, api: &mut PluginApi) -> Result<(), PluginError>; }` |
| 29 | `§2.11 PluginApi` | 结构新增 | `pub struct PluginToolRegistration { pub decl: ToolDecl, pub tool: Arc<dyn Tool> }`、`pub struct PluginHookRegistration { pub decl: HookDecl, pub hook: Arc<dyn Hook>, pub source: HookSource, pub priority: i32 }`、`pub struct PluginApi<'a> { /* 持有 ToolRegistry / HookRunner + metadata stores */ } impl PluginApi<'_> { pub fn register_tool(&mut self, reg: PluginToolRegistration) -> Result<(), PluginError>; pub fn register_hook(&mut self, reg: PluginHookRegistration) -> Result<(), PluginError>; pub fn register_skill_decl(&mut self, decl: SkillDecl) -> Result<(), PluginError>; pub fn register_model_provider_decl(&mut self, decl: ModelProviderDecl) -> Result<(), PluginError>; }` |
| 30 | `§2.11 PluginDiscoveryConfig` | 结构新增 | `pub struct PluginDiscoveryConfig { pub roots: Vec<PathBuf>, pub allow: Vec<String>, pub deny: Vec<String> } impl PluginDiscoveryConfig { pub fn default_roots() -> Vec<PathBuf>; }` |
| 31 | `§2.11 SDK_PLUGIN_API_VERSION` | 常量新增 | `pub const SDK_PLUGIN_API_VERSION: &str = "1.0.0";` |
| 32 | `§2.11 PluginError` | 枚举补齐 | 对应 `PluginErrorKind` 10 型；`#[error(transparent)]` 变体承载详情 |

## Task Ledger

> 本周共 12 Task。每个 Task 对应 ≤ 300 行批次；Task 7 与 Task 8（Plugin 核心）必要时可拆分为 7a / 7b / 8a / 8b。

### Task 1: `octopus-sdk-subagent` crate 骨架 + W5 前置合同基线（序号 1–12）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-subagent/Cargo.toml`
- Create: `crates/octopus-sdk-subagent/src/lib.rs`
- Modify: `crates/octopus-sdk-contracts/src/lib.rs`（追加 `subagent.rs` 子模块）
- Create: `crates/octopus-sdk-contracts/src/subagent.rs`
- Create: `crates/octopus-sdk-contracts/src/plugin.rs`
- Modify: `crates/octopus-sdk-contracts/src/event.rs`
- Modify: `crates/octopus-sdk-session/src/snapshot.rs`
- Modify: `crates/octopus-sdk-session/src/store.rs`
- Modify: `Cargo.toml`（顶层，`members += "crates/octopus-sdk-subagent"`）
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.1 1–12 + `plugins_snapshot` 承载面 + §2.10 crate 落地登记）

Preconditions:
- W4 已完成（`PermissionMode / HookEvent` 可引用）。
- `sdk-contracts` lib.rs 无冲突工作区。

Step 1:
- Action: 起 `octopus-sdk-subagent` crate 骨架（`Cargo.toml` 依赖 `octopus-sdk-contracts / -model / -session / -tools / -permissions / -context / -hooks` + `tokio / tracing / thiserror / async-trait`）；`src/lib.rs` 只 `pub use` 模块。
- Done when: `cargo build -p octopus-sdk-subagent` 空 crate 编译通过。
- Verify: `cargo build -p octopus-sdk-subagent`。
- Stop if: workspace 成员顺序引起 `Cargo.lock` 重写失败 → Stop #10。

Step 2:
- Action: 在 `sdk-contracts/src/subagent.rs` 落地 `SubagentSpec / TaskBudget / SubagentOutput / SubagentSummary / SubagentError / SprintContract / Verdict`（序号 1–7）；lib.rs 追加 `pub mod subagent; pub use subagent::*;`。
- Done when: `cargo test -p octopus-sdk-contracts --test subagent_contract` 覆盖 `SubagentSpec` 序列化/反序列化字节稳定（3 次一致）；`SubagentError` 实现 `std::error::Error`。
- Verify: `cargo test -p octopus-sdk-contracts --test subagent_contract`。
- Stop if: 评审有人要求把 `ModelError` 下沉进 Level 0 → 停止并维持 `SubagentError::Provider { reason: String }`；W5 不允许把 `sdk-model` 错误类型塞进 contracts。

Step 3:
- Action: 在 `sdk-contracts/src/plugin.rs` 落地 `PluginsSnapshot / PluginSummary / PluginSourceTag / ToolDecl / HookDecl / HookPoint / SkillDecl / ModelProviderDecl / DeclSource / PluginErrorKind`（序号 8–12）；`HookDecl` 明确只持有静态 `HookPoint`，不持有 runtime `HookEvent`。
- Done when: `cargo build -p octopus-sdk-contracts` 通过，且 `HookDecl` / `ToolDecl` 可独立序列化。
- Verify: `cargo build -p octopus-sdk-contracts`。

Step 4:
- Action: 前置扩 `SessionEvent::SessionStarted`、`SessionSnapshot`、`SessionStore` 的公共面，使其显式承载 `plugins_snapshot`；默认把快照内嵌在 `SessionStarted`，若 W1 首事件无法扩面则同批登记紧随其后的 `session.plugins_snapshot` 次事件；若业务侧要透出 runtime session schema，同批次登记 OpenAPI/schema 影响边界。
- Done when: W5 不再把 `plugins_snapshot` 描述成 session 小补丁；`event.rs` / `snapshot.rs` / `store.rs` 的目标态已在本 Task 明确。
- Verify: `cargo test -p octopus-sdk-session`（或最小契约测试 target）。
- Stop if: 现有 W1 首事件契约无法扩展 → 先切换为紧随 `session.started` 的次事件 `session.plugins_snapshot`，并先回写 `02 §5` / `docs/sdk/README.md`；只有试图直接编辑 `contracts/openapi/octopus.openapi.yaml` 或 `packages/schema/src/generated.ts` 时才触发 Stop #3。

Step 5:
- Action: 落地 `FILE_REF_THRESHOLD = 4_096`（序号 22，放在 `octopus-sdk-subagent::config`）；`02 §2.10` 第 22 条回填。
- Done when: 常量对外可见；内部测试可引用。
- Verify: `cargo build -p octopus-sdk-subagent`。

Step 6:
- Action: 回填 `02-crate-topology.md §2.1 / §2.2 / §2.10`（序号 1–12 / 22 + `plugins_snapshot` 承载面）。
- Done when: diff 审读符合 §2 公共面修订清单格式。
- Verify: `rg '^\| 1 \| `§2.1` 新增类型' docs/plans/sdk/02-crate-topology.md`（示意，实际跑守护扫描）。

### Task 2: `SubagentContext` + Session 子会话 + `OrchestratorWorkers::run_worker`

Status: `pending`

Files:
- Create: `crates/octopus-sdk-subagent/src/context.rs`
- Create: `crates/octopus-sdk-subagent/src/orchestrator.rs`
- Create: `crates/octopus-sdk-subagent/tests/context_isolation.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.10 序号 15 / 19）

Preconditions:
- Task 1 完成。
- `sdk-session::SessionStore::new_child_session(parent_id, spec)` 若 W1 未暴露 → 先在 `sdk-session` 补 `new_child_session` 方法（追加 `02 §2.2` 登记）。

Step 1:
- Action: `SubagentContext::new(parent_session, spec)`：从父 `PermissionGate` 派生子 gate（白名单 ∩ `spec.allowed_tools`）；从父 `ToolRegistry` 派生 filtered child registry；初始化空 `HookRunner`（不继承父 Hook，见 R3）；`scratchpad` 复用父实例（共享 `runtime/notes` 目录）。
- Done when: `SubagentContext::from_parent(parent, spec)` 返回的 `allowed_tools` 严格包含于父的 `allowed_tools`。
- Verify: `cargo test -p octopus-sdk-subagent --test context_isolation test_allowed_tools_is_subset`。
- Stop if: 需修改 W4 `PermissionGate` trait 签名 → Stop #2。

Step 2:
- Action: `OrchestratorWorkers::run_worker(spec, input)`：打开子 session（通过 `SessionStore`），跑 `model.complete` 直到 `end_turn / max_turns / tokens_used >= threshold * budget`；累加 tokens；若超阈值写入 `subagent_budget_exceeded` 事件后收尾（不强杀）。返回 `SubagentOutput::Summary` 或 `FileRef`（`> FILE_REF_THRESHOLD` 时自动分叉写 `scratchpad`）。
- Done when: `test_subagent_file_ref_switch` 断言 5KB 输出进入 `FileRef`；1KB 输出进入 `Summary`。
- Verify: `cargo test -p octopus-sdk-subagent --test context_isolation test_subagent_file_ref_switch`。
- Stop if: `DurableScratchpad::write` 在 Windows 丢失更新 → 复用 W4 R5 的 `std::sync::Mutex` fallback。

Step 3:
- Action: 深度守护：`spec.depth > 2` 返回 `SubagentError::DepthExceeded`；`run_worker` 内若需 spawn 孙子代理，`new_context.depth = parent.depth + 1`。
- Done when: `test_depth_limit` 断言 depth=3 返回 `DepthExceeded`。
- Verify: `cargo test -p octopus-sdk-subagent --test context_isolation test_depth_limit`。

Step 4:
- Action: 回填 `02 §2.10` 序号 15 / 19。
- Done when: diff 审读。
- Verify: `rg 'SubagentContext' docs/plans/sdk/02-crate-topology.md`。

### Task 3: `OrchestratorWorkers::run`（fan-out → fan-in + 5 并发）+ 父子独立上下文合同测试

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-subagent/src/orchestrator.rs`
- Create: `crates/octopus-sdk-subagent/tests/fan_in_fan_out.rs`
- Create: `crates/octopus-sdk-subagent/tests/condensed_summary.rs`

Preconditions:
- Task 2 完成。

Step 1:
- Action: `run(specs, inputs)` 用 `tokio::sync::Semaphore(max_concurrency=5)` + `join_all` 并行；每个 worker 在各自 `SubagentContext` 内运行；收集 `Vec<Result<SubagentOutput, SubagentError>>` 按 spec 顺序返回。
- Done when: `test_fan_out_concurrency` 用 `MockModelProvider` 延迟 100ms × 10 个 spec，实际耗时 ≤ 300ms（证明 5 并发生效）。
- Verify: `cargo test -p octopus-sdk-subagent --test fan_in_fan_out test_fan_out_concurrency -- --nocapture`。
- Stop if: `join_all` 在某 worker panic 时吞掉其他 → 改为 `futures::future::join_all` 收集 `Result`，不 unwrap。

Step 2:
- Action: `fan_in(outputs)`：把多个 `SubagentOutput::Summary` 按"- <text>"列表形式合并为父代理可见的单一 summary；大 output 统一引用 file ref；最终返回 `SubagentOutput::Summary { text: 合并文本, meta: 合成 SubagentSummary（tokens 累加） }`。
- Done when: `test_fan_in_merge` 断言 3 个 summary 合并后包含 3 条 bullet + 合并 meta。
- Verify: `cargo test -p octopus-sdk-subagent --test fan_in_fan_out test_fan_in_merge`。

Step 3:
- Action: 父子独立上下文合同测试：父 session 工具链注册 `ToolA / ToolB / ToolC`；子代理 `allowed_tools: ["ToolA"]`；子代理 session 事件流扫描 **不含** ToolB/ToolC 的 `tool.executed` 事件；父 session 事件流**只收到** `subagent.summary` 事件。
- Done when: `test_parent_child_isolation` 断言通过。
- Verify: `cargo test -p octopus-sdk-subagent --test condensed_summary test_parent_child_isolation`。
- Stop if: 发现父事件流含原始 tool_use payload 泄漏 → Stop #5（凭据/噪声泄漏），补事件 redaction 契约后重跑。

### Task 4: `AgentTool` 注入 `TaskFn`（W3 反向回填）

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-tools/src/builtin/agent.rs`
- Create: `crates/octopus-sdk-tools/tests/agent_tool_task_fn.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.4 序号 13 / 14）

Preconditions:
- Task 3 完成（`OrchestratorWorkers` 可产出 `TaskFn` 实例）。

Step 1:
- Action: `AgentTool` 新增 `with_task_fn(self, f: Arc<dyn TaskFn>) -> Self`；默认内部 `task_fn: Arc<dyn TaskFn>` 为 `ErrorTaskFn { reason }`。定义 `trait TaskFn`（序号 14）。
- Done when: 旧测试 `cargo test -p octopus-sdk-tools` 全绿（默认 ErrorTaskFn 不破坏已有用例）。
- Verify: `cargo test -p octopus-sdk-tools`。
- Stop if: 必须修改 `AgentTool::new` 签名 → Stop #2（W3 公共面破坏），改走 `AgentToolBuilder` 方案。

Step 2:
- Action: `AgentTool::execute` 内：若 `task_fn` 为 `ErrorTaskFn` 返回 `is_error: true` + reason；否则调用 `task_fn.run(&spec, &input).await`。
- Done when: `test_agent_tool_with_task_fn` 用 `OrchestratorWorkers::into_task_fn()` 注入，返回真实 summary。
- Verify: `cargo test -p octopus-sdk-tools --test agent_tool_task_fn`。

Step 3:
- Action: 回填 `02 §2.4` 序号 13 / 14；在 `03 §3.1 tools::subagent_runtime` 行状态标 `partial`（SDK 面已迁）。
- Done when: diff 审读。

### Task 5: `GeneratorEvaluator` + `MockEvaluator` 闭环

Status: `pending`

Files:
- Create: `crates/octopus-sdk-subagent/src/gen_eval.rs`
- Create: `crates/octopus-sdk-subagent/tests/gen_eval_mock.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.10 序号 16 / 17 / 18 / 21）

Preconditions:
- Task 2 完成。

Step 1:
- Action: 定义 `trait Planner / Generator / Evaluator`（序号 17）+ `Draft`（序号 18）；`GeneratorEvaluator::new(..., max_rounds: u16)`。
- Done when: `cargo build -p octopus-sdk-subagent` 通过。
- Verify: `cargo build -p octopus-sdk-subagent`。

Step 2:
- Action: `GeneratorEvaluator::run(prompt)` 循环：`planner.expand → generator.run(None) → evaluator.judge → if Verdict::Fail { feedback } else break`；超 `max_rounds` 返回 `SubagentError::EvaluatorExhausted { rounds }`。
- Done when: 单测 `test_gen_eval_pass_on_round_2` 用 `MockEvaluator { rubric: |draft| if draft.content contains "v2" Pass else Fail }` 验证第 2 轮 Pass。
- Verify: `cargo test -p octopus-sdk-subagent --test gen_eval_mock test_gen_eval_pass_on_round_2`。

Step 3:
- Action: Evaluator 独立上下文合同：`Evaluator::judge` 的输入 `Draft` 不含 `Generator` 的推理过程；在 `test_evaluator_sees_only_draft` 中断言 `Draft.metadata` 不含 `generator_thinking` 字段。
- Done when: 断言通过。
- Verify: `cargo test -p octopus-sdk-subagent --test gen_eval_mock test_evaluator_sees_only_draft`。
- Stop if: `Generator::run` 返回的 `Draft` 携带 thinking block → 补 `Draft::strip_thinking()` 净化步骤。

Step 4:
- Action: `MockEvaluator`（序号 21）加 `#[cfg(any(test, feature = "test-utils"))]`；在 `Cargo.toml` 增 `test-utils` feature。
- Done when: `cargo test -p octopus-sdk-subagent --features test-utils` 通过。
- Verify: `cargo test -p octopus-sdk-subagent --features test-utils`。

Step 5:
- Action: 回填 `02 §2.10` 序号 16–18 / 21。

### Task 6: `AgentRegistry`（`.agents/**/*.md` frontmatter）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-subagent/src/registry.rs`
- Create: `crates/octopus-sdk-subagent/tests/agent_registry.rs`
- Create: `crates/octopus-sdk-subagent/tests/fixtures/agents/reviewer.md`
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.10 序号 20）

Preconditions:
- Task 1 完成。

Step 1:
- Action: 用 `serde_yaml` 解析 frontmatter；字段映射：`name → id`、`model → model_role`（frontmatter 先保存字符串 key；`"claude-sonnet-4-5" → "main"` 之类 canonical model 到 role key 的归一化由 `sdk-model` helper / `RoleRouter` 负责，未命中则 fallback `"main"` 并写 warn tracing）、`allowed_tools → Vec<String>`、`max_turns → u16`、`task_budget: 40000 → TaskBudget { total: 40_000, completion_threshold: 0.9 }`（snake_case → camelCase 只影响运行时字段，YAML 接受 snake_case）；body 文本 → `system_prompt`。
- Done when: `test_parse_reviewer_md` 断言 `SubagentSpec` 字段值正确。
- Verify: `cargo test -p octopus-sdk-subagent --test agent_registry test_parse_reviewer_md`。
- Stop if: `sdk-model` 不提供 canonical model → role key 的薄 helper → 本周补一个返回字符串 key 的 helper（登记到 `02 §2.3`），但 `SubagentSpec` 仍只保存 `String`。

Step 2:
- Action: `AgentRegistry::discover(roots)` 对每个 root 走 `walkdir`，收集 `*.md` → frontmatter 解析 → `HashMap<String, SubagentSpec>`；重名（跨 root）按 roots 顺序遮蔽（后者覆盖前者）。
- Done when: `test_workspace_shadow_project` 断言 workspace `reviewer.md` 遮蔽 project `reviewer.md`。
- Verify: `cargo test -p octopus-sdk-subagent --test agent_registry test_workspace_shadow_project`。

Step 3:
- Action: id 校验：`[a-z0-9-]+`，长度 ≤ 64；失败 → `SubagentError::Storage { reason: "invalid agent id" }`。
- Done when: `test_invalid_id_rejected` 通过。
- Verify: 同上。

Step 4:
- Action: 回填 `02 §2.10` 序号 20；`03 §2.1 lsp_client` 状态保留 pending（W5 不做 LSP，仅留 `PluginComponent::LspServer` 元信息登记）。

### Task 7: `octopus-sdk-plugin` crate 骨架 + Manifest schema + 三道安全门（23 / 24 / 25 / 30 / 31 / 32）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-plugin/Cargo.toml`
- Create: `crates/octopus-sdk-plugin/src/lib.rs`
- Create: `crates/octopus-sdk-plugin/src/manifest.rs`
- Create: `crates/octopus-sdk-plugin/src/security.rs`
- Create: `crates/octopus-sdk-plugin/src/error.rs`
- Create: `crates/octopus-sdk-plugin/tests/manifest_validate.rs`
- Create: `crates/octopus-sdk-plugin/tests/security_gates.rs`
- Modify: `Cargo.toml`（顶层 `members += "crates/octopus-sdk-plugin"`）
- Modify: `docs/plans/sdk/02-crate-topology.md`（回填 §2.11 序号 23–25 / 30 / 31 / 32）

Preconditions:
- Task 1 完成（`DeclSource / HookPoint / PluginsSnapshot` 已在 Level 0 合同层可用）。

Step 1:
- Action: 起 `octopus-sdk-plugin` crate；`Cargo.toml` 依赖 `octopus-sdk-contracts / -tools / -hooks / serde / serde_yaml / semver / thiserror / walkdir`。直接消费 Task 1 已下沉的 `PluginsSnapshot / PluginSummary / PluginSourceTag / ToolDecl / HookDecl / HookPoint / SkillDecl / ModelProviderDecl / DeclSource / PluginErrorKind`。
- Done when: `cargo build -p octopus-sdk-plugin -p octopus-sdk-contracts` 通过。
- Verify: 同上。

Step 2:
- Action: `PluginManifest`（序号 23）+ `PluginCompat`（序号 24）+ `PluginComponent`（序号 25）+ `SDK_PLUGIN_API_VERSION`（序号 31）。`PluginComponent` 12 变体全部登记（后 8 类落元信息结构体 `CommandDecl / McpServerDecl / LspServerDecl / ChannelDecl / ContextEngineDecl / MemoryBackendDecl / OutputStyleDecl / AgentDecl`，每个结构体 3–5 字段即可）。
- Done when: `cargo test -p octopus-sdk-plugin --test manifest_validate test_parse_minimal` 用 fixture 解析一个含 1 tool + 1 hook 的 `plugin.json` 通过。
- Verify: `cargo test -p octopus-sdk-plugin --test manifest_validate test_parse_minimal`。
- Stop if: serde 派生产生循环类型 → 把 `AgentDecl.manifest_path` 改为 `PathBuf`（不含 SubagentSpec）。

Step 3:
- Action: `security.rs` 三道门：
  - `check_path_escape(plugin_root, path) -> Result<(), PluginError>`：`fs::canonicalize` + `starts_with(plugin_root)`；
  - `check_world_writable(path) -> Result<(), PluginError>`：`unix::fs::PermissionsExt::mode() & 0o002 == 0`；Windows 跳过；
  - `check_reserved_name(name) -> Result<(), PluginError>`：正则 `^[a-z0-9-]{1,64}$` + 非 ASCII 拒绝。
- Done when: `test_security_gates`（3 个子测试）覆盖通过/拒绝各一条路径；`PluginError::{PathEscape, WorldWritable, ManifestValidationError}` 正确。
- Verify: `cargo test -p octopus-sdk-plugin --test security_gates`。
- Stop if: Windows world-writable 检测需要 ACL → 本周跳过，登记 §变更日志 + R9。

Step 4:
- Action: `compat` 校验：`semver::VersionReq::matches(semver::Version::parse(SDK_PLUGIN_API_VERSION))`；不匹配 → `PluginError::IncompatibleApi`。
- Done when: `test_compat_mismatch` 覆盖通过。
- Verify: `cargo test -p octopus-sdk-plugin --test manifest_validate test_compat_mismatch`。

Step 5:
- Action: `PluginDiscoveryConfig`（序号 30）+ `default_roots()` 返回空 vec（业务层注入）；`allow / deny` 字段。
- Done when: 结构可实例化；tests/ 覆盖 default_roots 空。
- Verify: `cargo test -p octopus-sdk-plugin`。

Step 6:
- Action: `PluginError`（序号 32）10 型 + `Display / std::error::Error` 实现；内部 `cause: String` 字段。
- Done when: `cargo clippy -p octopus-sdk-plugin -- -D warnings` 通过。
- Verify: 同上。

Step 7:
- Action: 回填 `02 §2.11` 序号 23–25 / 30 / 31 / 32。

### Task 8: `PluginRegistry` 12 类扩展点 + tools/hooks 可执行接线（26 / 27 / 28 / 29）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-plugin/src/registry.rs`
- Create: `crates/octopus-sdk-plugin/src/api.rs`
- Create: `crates/octopus-sdk-plugin/tests/registry.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（§2.11 序号 26–29）

Preconditions:
- Task 7 完成。

Step 1:
- Action: `PluginRegistry::new()` 初始化 12 类 `HashMap<id, Decl>` + concrete `ToolRegistry` / `HookRunner` handle；`register_plugin(manifest)` 遍历 `components` 同步写 declaration index，tools/hooks 另走 executable runtime registration。
- Done when: `test_register_noop_plugin` 注册 1 个 tool + 1 个 hook + 1 个 skill + 1 个 model_provider 后 `get_snapshot().plugins.len() == 1 && components_count == 4`。
- Verify: `cargo test -p octopus-sdk-plugin --test registry test_register_noop_plugin`。

Step 2:
- Action: `PluginApi<'a>::register_tool(PluginToolRegistration) / register_hook(PluginHookRegistration)` 把 executable record 接入 `ToolRegistry` / `HookRunner`；`register_skill_decl / register_model_provider_decl` 只登记 metadata。单向流：不返回底层 registry 的可变借用。
- Done when: `test_register_api_unidirectional` 断言 `PluginApi` 无 `&mut Registry` 返回路径。
- Verify: 同上。

Step 3:
- Action: `Plugin` trait（序号 28）；`PluginLifecycle::run` 对每个 `Box<dyn Plugin>` 调 `plugin.register(&mut api)` 一次。
- Done when: `test_plugin_register_once` 断言重复 register 返回 `DuplicateId`。
- Verify: `cargo test -p octopus-sdk-plugin --test registry test_plugin_register_once`。

Step 4:
- Action: `PluginLifecycle::register()` 明确分两路：`ToolDecl` / `HookDecl` 进入 declaration index；`PluginToolRegistration` / `PluginHookRegistration` 进入 runtime execution path。`SkillDecl` / `ModelProviderDecl` 保持 metadata + builder slot，不伪装成已可执行。
- Done when: W5 文档不再出现 decl-only registration 即完成 runtime 接线的表述。
- Verify: `cargo build -p octopus-sdk-plugin`（实现周）/ 文档守护扫描（当前周）。
- Stop if: `plugin → tools / hooks` 引入反向循环 → 停止并改为把 runtime registration record 收窄到更低层，但不回到 decl-only 方案。

Step 5:
- Action: 回填 `02 §2.11` 序号 26–29。

### Task 9: `PluginLifecycle::run`（discover → enablement → load → register → expose）

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-plugin/src/lifecycle.rs`（新建）
- Create: `crates/octopus-sdk-plugin/src/bundled.rs`
- Create: `crates/octopus-sdk-plugin/tests/lifecycle.rs`
- Create: `crates/octopus-sdk-plugin/bundled/example-noop-tool/plugin.json`

Preconditions:
- Task 7 / Task 8 完成。

Step 1:
- Action: `PluginLifecycle::run(registry, plugins)` 五段：
  - discover：遍历 `config.roots`，对每个 `<root>/*/plugin.json` 解析；过滤 `deny` 命中；若 `allow` 非空，只保留命中；
  - enablement：安全门 3 道 + `compat` 校验 → 输出 `enabled / disabled / blocked`；
  - load：`plugins: &[Box<dyn Plugin>]` 已由调用方提供（本周不做动态加载），按 discover 结果过滤；
  - register：对 `enabled` 插件调用 `plugin.register(&mut api)`；
  - expose：无操作（registry 已对外可读）。
- Done when: `test_lifecycle_end_to_end` 跑通（含 example-noop-tool + 1 个 deny 的插件）。
- Verify: `cargo test -p octopus-sdk-plugin --test lifecycle test_lifecycle_end_to_end`。
- Stop if: `walkdir` 触发 symlink 无限循环 → 设置 `WalkDir::follow_links(false)`。

Step 2:
- Action: `bundled/example-noop-tool/` 只保留 `plugin.json` 作为 manifest fixture；`NoopPlugin` 的 Rust 实现放进 `crates/octopus-sdk-plugin/src/bundled.rs`，并通过 `include_str!` 读取 fixture；`example_bundled_plugins() -> Vec<Box<dyn Plugin>>` 返回静态注册的 bundled plugins。
- Done when: `plugin.json` 不再伪装成独立 Cargo crate；`test_lifecycle_end_to_end` 通过 `example_bundled_plugins()` 消费静态注册的 `NoopPlugin`。
- Verify: 同上。

Step 3:
- Action: 错误路径覆盖：manifest 解析失败、世界可写、路径逃逸、重复 id 各一条测试。
- Done when: `test_error_*` 4 个子测试通过。
- Verify: `cargo test -p octopus-sdk-plugin --test lifecycle`。

### Task 10: `plugins_snapshot` store 实现 + replay（基于前置合同）

Status: `pending`

Files:
- Modify: `crates/octopus-sdk-session/src/store.rs`（或对应 store 实现）
- Create: `crates/octopus-sdk-session/tests/plugins_snapshot_stability.rs`
- Modify: `docs/plans/sdk/02-crate-topology.md`（§2.1 / §2.2 追加 `plugins_snapshot` 双分支与 `append_session_started` / `new_child_session`；§5 登记与 OpenAPI 对齐）

Preconditions:
- Task 1 完成（`SessionEvent::SessionStarted` / `SessionSnapshot` / `SessionStore` 已预留 `plugins_snapshot` 承载面）。
- Task 8 完成（`PluginRegistry::get_snapshot()` 可用）。

Step 1:
- Action: 在前置合同已扩完的前提下，实现 `plugins_snapshot` 的 JSONL + SQLite 持久化与 helper API；同步补 store/fixture/golden，并把持久化路径拆成两条显式分支：
  - Branch A：`SessionStarted` 首事件直接内嵌 `plugins_snapshot`。
  - Branch B：`SessionStarted { plugins_snapshot: None }` 只写启动元数据，紧随其后追加 `session.plugins_snapshot` 次事件。
- Done when:
  - Branch A：`test_append_session_started` 断言首事件 payload 含 `plugins_snapshot: {...}`，且 `api_version / plugins[*].id` 字段存在。
  - Branch B：`test_append_session_plugins_snapshot` 断言首事件不含快照，但第二事件为 `session.plugins_snapshot`，且 payload 含 `api_version / plugins[*].id`。
- Verify:
  - Branch A：`cargo test -p octopus-sdk-session --test plugins_snapshot_stability test_append_session_started`
  - Branch B：`cargo test -p octopus-sdk-session --test plugins_snapshot_stability test_append_session_plugins_snapshot`
- Stop if: 为表达上述双分支必须直接编辑 `contracts/openapi/octopus.openapi.yaml` 或 `packages/schema/src/generated.ts` → Stop #3。

Step 2:
- Action: 稳定性合同：构造 2 plugin + 固定 manifest，`get_snapshot()` 序列化 3 次字节一致（含内部按 id 排序）。
- Done when: `test_snapshot_byte_stable` 通过。
- Verify: 同上。

Step 3:
- Action: Replay 合同：从 JSONL 重建 `Session`；优先从首事件读取 `PluginsSnapshot`，若首事件不含快照则必须从紧随其后的 `session.plugins_snapshot` 次事件恢复，并与 `register_plugin` 时保持的 summary 相等。
- Done when: `test_snapshot_replay_embedded` 与 `test_snapshot_replay_second_event` 都通过。
- Verify: `cargo test -p octopus-sdk-session --test plugins_snapshot_stability test_snapshot_replay_embedded test_snapshot_replay_second_event`。

Step 4:
- Action: 回填 `02 §2.1 / §2.2 / §5`；明确登记两条合同分支：优先把 `plugins_snapshot` 内嵌在 `SessionStarted`，若首事件不能扩面则紧随其后发 `session.plugins_snapshot` 次事件；若触及 OpenAPI（`RuntimeSessionEvent` 扩 `plugins_snapshot` 字段或新增对应事件），先走 `pnpm openapi:bundle && pnpm schema:generate`；同时复核 `SessionSnapshot` 与外露 schema 是否需要同步扩面。
- Done when: OpenAPI diff 审读；schema 再生成无意外差异。
- Verify: `pnpm openapi:bundle && pnpm schema:check`。
- Stop if: OpenAPI 产生破坏性 diff → Stop #3（手改生成物风险），先开 `contracts/openapi/src/**` 补丁 PR。

### Task 11: W5 合同测试 + 四源合一守护扫描

Status: `pending`

Files:
- Create: `crates/octopus-sdk-subagent/tests/no_credentials_in_subagent_events.rs`
- Create: `crates/octopus-sdk-plugin/tests/manifest_no_execution.rs`
- Create: `docs/plans/sdk/scripts/w5-guard.sh`（可选辅助脚本，或仅以命令形式列在 Checkpoint）

Preconditions:
- Task 3 / Task 5 / Task 9 / Task 10 完成。

Step 1:
- Action: 子代理凭据零暴露合同：构造 `ToolCallRequest { input: { "api_key": "s3cret-xyz" } }` 经 子代理 `HookRunner(PreToolUse)` + `PermissionGate::check` 后发射的父 session 事件 JSON 扫描无 `s3cret-xyz`。
- Done when: `rg -F 's3cret-xyz'` 对事件 JSON 无命中。
- Verify: `cargo test -p octopus-sdk-subagent --test no_credentials_in_subagent_events`。
- Stop if: 命中 → 检查是否在 `SubagentOutput::Summary` 中泄漏原始 `tool_use.input` → 补 summary redaction。

Step 2:
- Action: Manifest 零执行校验：只 `parse + validate` 不 `load`，断言 `cpu_time < 10ms` + `registry.plugins.is_empty()`（只校验不注册）。
- Done when: `test_manifest_parse_no_side_effect` 通过。
- Verify: `cargo test -p octopus-sdk-plugin --test manifest_no_execution`。

Step 3:
- Action: 四源合一守护扫描命令（在 Checkpoint 贴输出）：
  ```bash
  rg "use runtime::plugin_lifecycle" crates/octopus-sdk-*
  rg "use runtime::hooks" crates/octopus-sdk-*
  rg "use plugins::manifest" crates/octopus-sdk-*
  rg "use plugins::hooks" crates/octopus-sdk-*
  rg "use tools::subagent_runtime" crates/octopus-sdk-*
  rg "octopus-sdk-plugin|octopus-sdk-subagent" Cargo.toml
  find crates/octopus-sdk-subagent crates/octopus-sdk-plugin -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'
  ```
  全部 0 命中（前 5 条）+ `Cargo.toml` 含两个新 members + 单文件 ≤ 800 行。
- Done when: 命令输出全符合预期。
- Verify: 同上。

Step 4:
- Action: `00-overview.md §3 W5 硬门禁` 追加"四源合一"具体命令（复制 Step 3 命令）+ `plugins_snapshot` 双分支 replay 测试名，明确快照可从 `SessionStarted` 或紧随其后的 `session.plugins_snapshot` 恢复。

### Task 12: W5 Weekly Gate 收尾

Status: `pending`

Files:
- Modify: `docs/plans/sdk/README.md`（W5 状态切 `in_progress → done`）
- Modify: `docs/plans/sdk/00-overview.md §10 变更日志`（追加 W5 收尾）
- Modify: `docs/plans/sdk/02-crate-topology.md §10`（追加 W5 日志）
- Modify: `docs/plans/sdk/03-legacy-retirement.md`（`runtime::plugin_lifecycle` / `plugins::{manifest, discovery, lifecycle, hooks}` / `tools::subagent_runtime` 行状态切换；`runtime::worker_boot` 维持 W5 非来源说明）
- Modify: `docs/plans/sdk/08-week-5-subagent-plugin.md`（追加最终 Checkpoint）

Preconditions:
- Task 1–11 全部 `done`。

Step 1:
- Action: 执行 `01-ai-execution-protocol.md §4` Weekly Gate Checklist 全量核对。
- Done when: 11 项全 pass。
- Verify: 各命令输出贴到本文件 Checkpoint 节。

Step 2:
- Action: 跑 `cargo build --workspace / cargo clippy --workspace -- -D warnings / cargo test --workspace`；跑 `01 §7.4` + 本文件 Task 11 Step 3 的守护扫描。
- Done when: 全部 0 命中；workspace 三项 pass。
- Verify: 同上。

Step 3:
- Action: 把 W5 的 12 个 Task `Status` 全部切 `done`；`README.md §文档索引` 的 W5 状态切 `done`；`00-overview.md §10` 追加 "2026-04-xx | W5 Weekly Gate 收尾：`08-week-5-subagent-plugin.md` 由 `in_progress` 切为 `done`。2 个 SDK crate（subagent / plugin）落地；Level 0 contracts W5 补丁完成；`plugins_snapshot` replay 合同 + 四源合一守护通过。| Codex"；同步把 `03-legacy-retirement.md §2.1 worker_boot` 的迁移周改为 `W7`，并标注 W5 仅作为 non-source 边界说明。
- Done when: 三处同批次完成。
- Verify: diff 审读。

## Exit State 对齐表（与 `00-overview.md §3 W5` 逐条）

| `00-overview.md §3 W5 出口状态` | 本 Plan 交付点 | 验证命令 |
|---|---|---|
| `Orchestrator-Workers` + `Generator-Evaluator` 最小可运行示例；子代理独立上下文窗口，返回 condensed 摘要 | Task 2 / 3 / 5 | `cargo test -p octopus-sdk-subagent --test fan_in_fan_out --test condensed_summary --test gen_eval_mock` |
| `PluginManifest / PluginRegistry / PluginLifecycle` 初版 + 最小 native plugin 示例 | Task 7 / 8 / 9 | `cargo test -p octopus-sdk-plugin --test manifest_validate --test registry --test lifecycle` |
| `ToolRegistry / HookRunner` 通过 executable runtime registration 向插件开放；`SkillDecl / ModelProviderDecl` 在 W5 保持 metadata + builder slot | Task 8 Step 4 | `cargo build -p octopus-sdk-plugin` |
| **硬门禁**：`crates/plugins/*` + `runtime::plugin_lifecycle` + `runtime::hooks` 四源合一，无重复生命周期 | Task 11 Step 3 | `rg "use runtime::plugin_lifecycle\|use runtime::hooks\|use plugins::manifest\|use plugins::hooks" crates/octopus-sdk-*` → 0 命中 |
| **硬门禁**：plugin session 快照可从 `SessionStarted` 或紧随其后的 `session.plugins_snapshot` 恢复，并可回放 | Task 10 | `cargo test -p octopus-sdk-session --test plugins_snapshot_stability` |

## Weekly Gate Checklist · W5（执行时勾选）

```md
- [ ] 本周 12 个 Task 状态 = `done` 或 明确 `blocked`（带原因）。
- [ ] `00-overview.md §3 W5 出口状态` 逐条勾选通过（上表 5 行）。
- [ ] `00-overview.md §3 W5 硬门禁` 命令实际执行过并 pass。
- [ ] 当周 Checkpoint 无缺失（每批次一条，本文件末尾累积）。
- [ ] 本周 PR 总 diff 行数分布记录完成（用于预警 R5 风险）。
- [ ] 未引入 `docs/sdk/*` 与实现的新矛盾；如有 → 追加到 `docs/sdk/README.md` 末尾的 `## Fact-Fix 勘误`。
- [ ] 新公共面符号 = `02-crate-topology.md` 登记；删除的符号 = `03-legacy-retirement.md` 勾选。
- [ ] 如本周触及 Prompt Cache 相关（本周预期不触及）：命中率守护测试绿 / n/a。
- [ ] 如本周触及业务接线（预期否）：`pnpm -C apps/desktop test` 关键 suite 绿 / n/a。
- [ ] `cargo build --workspace` 全绿；`cargo clippy --workspace -- -D warnings` 全绿。
- [ ] 完成本周"变更日志"追加到 `00-overview.md §10`。
```

## 退役登记（同批次联动 `03-legacy-retirement.md`）

| `03` 文件行 | 旧符号 | 新位置 | W5 状态切换 |
|---|---|---|---|
| `§2.1 runtime::plugin_lifecycle.rs` | `PluginLifecycle / PluginState / DegradedMode / DiscoveryResult / PluginHealthcheck / ResourceInfo / ServerHealth / ServerStatus / ToolInfo` | `octopus-sdk-plugin::{PluginLifecycle, PluginRegistry}` | `pending → replaced`（仅 SDK 面） |
| `§2.1 runtime::worker_boot.rs` | `Worker / WorkerRegistry / WorkerEvent / WorkerEventKind / WorkerEventPayload`（trust gate / ready-for-prompt / misdelivery 控制面） | W5 不作为 `sdk-subagent` 实现来源；W7 再评估业务面是否改接 `octopus-sdk-subagent` | `pending`（迁移周改 `W7`；W5 仅保留 non-source 说明） |
| `§3.1 tools::subagent_runtime.rs` | `spawn_subagent_job / spawn_subagent_task / AgentInput / AgentJob / AgentOutput / SubagentToolExecutor / agent_permission_policy / allowed_tools_for_subagent` | `octopus-sdk-subagent::{OrchestratorWorkers, AgentRegistry, SubagentContext}` + `octopus-sdk-tools::AgentTool::with_task_fn` | `pending → partial`（greenfield SDK 覆盖职责边界，不从 legacy stub 迁实现） |
| `§4 plugins::manifest.rs` | `PluginManifest / PluginManifestValidationError / PluginCommandManifest / PluginToolManifest / PluginToolDefinition / PluginToolPermission / PluginPermission / PluginHooks / PluginLifecycle / load_plugin_from_directory` | `octopus-sdk-plugin::{PluginManifest, PluginComponent, PluginCompat, PluginError}` | `pending → replaced` |
| `§4 plugins::discovery.rs` | `PluginManager / PluginRegistry / PluginSummary / RegisteredPlugin / InstalledPluginRegistry / PluginMetadata / PluginInstallSource / PluginKind / PluginError / PluginLoadFailure / builtin_plugins` | `octopus-sdk-plugin::{PluginRegistry, PluginLifecycle}` + 业务侧分发源（W7） | `pending → partial`（SDK 面） |
| `§4 plugins::hooks.rs / hook_dispatch.rs / lifecycle.rs` | `HookEvent / HookRunResult / HookRunner / PluginTool / Plugin / PluginDefinition / BuiltinPlugin / BundledPlugin / ExternalPlugin` | `octopus-sdk-hooks`（W4 已承接）+ `octopus-sdk-plugin::PluginComponent` | `pending → replaced` |
| `§4 plugins::split_module_tests.rs` | 整体测试合集 | 拆分到 `octopus-sdk-plugin/tests/`（W5 新写，不迁移，见 R5） | 保留 `must-split`，留 W7 |

> Task 12 Step 3 合入批次同步执行上述状态切换。

## Batch Checkpoint Format

每批次结束追加：

```md
## Checkpoint YYYY-MM-DD HH:MM

- Week: W5
- Batch: Task <i> Step <j> → Task <i+1> Step <j>
- Completed:
  - <item>
- Files changed:
  - `path` (+added / -deleted / modified)
- Verification:
  - `cargo test -p <crate>` → pass
  - `rg "<forbidden pattern>" crates/` → 0 hits
- Exit state vs plan:
  - matches / partial / blocked
- Blockers:
  - <none | 具体问题 + 待人判断点>
- Next:
  - <Task i+1 Step j+1 | Week 6 kick-off>
```

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-21 | 首稿：2 个 SDK crate（subagent / plugin）公共面 + 12-Task Ledger + 4 项审核决策（D1 MockEvaluator / D2 仅本地目录 / D3 worker_boot 只取 Orchestrator / D4 `.agents/**` 双 root）；R1–R12 风险登记；与 W4 叠加 Hook source 优先级登记；退役登记覆盖 7 行 | Architect (Claude Opus 4.7) |
| 2026-04-21 | 审计修复：把 plugin 注册改为 declaration/runtime 两层；`HookDecl.event` 改为 `HookPoint`；`SubagentContext.tools` 改回 `ToolRegistry`；`worker_boot` / `subagent_runtime` 改成绿色实现边界；`plugins_snapshot` 前移为合同硬门禁而不是尾部 session 补丁 | Codex |
| 2026-04-21 | 追补修复：清掉 Level 0 对 `ModelRole / ProviderId / ModelError` 的直接依赖，`SubagentSpec.model_role` / `ModelProviderDecl.provider_ref` 改为 opaque key；`plugins_snapshot` 的 Stop 条件改成先切 `session.plugins_snapshot` 次事件、只把手改生成物视为 Stop #3；noop plugin 示例改为 `src/bundled.rs` + `plugin.json` fixture；`worker_boot` 迁移周镜像改成 `W7`；新增 R13 约束 `HookPoint ↔ HookEvent` 映射 | Codex |
| 2026-04-21 | 三轮审计修复：Task 10 明确拆成 `SessionStarted` 内嵌快照与 `session.plugins_snapshot` 次事件两条完成路径；`§2.2` 配套补登记 `append_session_started / new_child_session`；Task 7 去掉 `octopus-sdk-model` 依赖；四源合一与总控里的 800 行守护命令改成真实按行数检查 | Codex |
| 2026-04-21 | 四轮审计修复：`Architecture/expose` 不再把 `PluginRegistry::get_snapshot()` 写死成首事件载荷，而是改成 session start 持久化输入，显式兼容 `SessionStarted` 内嵌与 `session.plugins_snapshot` 次事件两条分支 | Codex |
