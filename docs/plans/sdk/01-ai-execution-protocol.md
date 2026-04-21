# 01 · AI 推进规约（Execution Protocol for AI Agents）

> **For AI Agents (Codex / Claude / Cursor):** 本文档是你执行 `docs/plans/sdk/*` 任何一份子 Plan 时**必须遵守的操作规约**。
> 本文档与 `00-overview.md` 同级；违反本文档规约的一切实施视为无效，必须回滚。

## 0. 为什么需要这份协议

大规模重构中 AI Agent 最常见的失败模式是：
1. **跳过预检直接动代码**——没看清楚依赖 / 不变量就改。
2. **批次过大，验证断链**——一次改 20 个文件后无法定位回归。
3. **默默放弃 stop condition**——遇到歧义时靠"合理推断"绕过。
4. **checkpoint 缺席**——不汇报进度，工程师失去可见性。
5. **旁路 schema 生成链**——手改生成物或跳步 bundle，导致前端/后端错位。

本协议用**强制性 checklist** 与 **明确的 stop condition** 把以上五种失败堵死。

---

## 1. 三层 Checklist 模型

AI 在执行任何子 Plan 时必须**依次**通过三层 checklist。任一层 FAIL 即**禁止推进**。

```
┌─────────────────────────────────────────────────────────┐
│  Layer A · 任务启动前（Pre-Task Checklist）              │
│  目的：进入任务前的静态检查                                 │
└─────────────────────────────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│  Layer B · 批次结束后（Post-Batch Checklist）            │
│  目的：每完成一批改动后的动态验证与汇报                       │
└─────────────────────────────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│  Layer C · 每周门禁（Weekly Gate Checklist）             │
│  目的：周末的硬门禁；未过不得开启下周                         │
└─────────────────────────────────────────────────────────┘
```

---

## 2. Layer A · 任务启动前 Checklist（每个 Task 起始 ≤ 10 分钟内完成）

复制到当前子 Plan 顶部的 "Active Work" 节，逐条勾选。任一未过 → `stop and ask`。

```md
### Pre-Task Checklist（Task <n>）

- [ ] 已阅读本子 Plan 的 `Goal` / `Architecture` / `Scope`。
- [ ] 已阅读 `00-overview.md §1 10 项取舍`，且当前任务未违反。
- [ ] 已阅读 `docs/sdk/*` 中与本 Task 对应的规范章节。
- [ ] 已阅读 Task 段落的 `Files` / `Preconditions` / `Step*` 且无歧义。
- [ ] 已识别本 Task 涉及的 **SDK 对外公共面** 变更（是 / 否）。
  - 若"是"：已确认变更在 `02-crate-topology.md §对外公共面` 有登记项（或计划在本批次内新增登记）。
- [ ] 已识别是否涉及 `contracts/openapi/src/**` 或 `packages/schema/src/**`。
  - 若"是"：已准备执行 `pnpm openapi:bundle && pnpm schema:generate` 作为验证步骤。
- [ ] 已识别是否涉及 `docs/sdk/14` UI Intent IR 变更（是 / 否）。
  - 若"是"：已明确新增/修改的 `RenderBlock.kind`；插件不得自行扩 kind。
- [ ] Preconditions 已全部满足；未满足项已在 `Open Questions` 中登记。
- [ ] 当前 git 工作树干净或有明确切分；本批次计划 diff ≤ 800 行（不含 generated）。
- [ ] 已识别所有 `Stop if:` 条款；遇到任一条件 → 立即停止并汇报。
```

### 未过时的标准应对

- **单项未过** → 在 `EXECUTION_TEMPLATE.md` 的 `Needs human decision` 节写明阻塞项，停止编码。
- **多项未过** → 重新审视 Task 切分，可能需要拆成更小 Task 或补齐 Precondition。

---

## 3. Layer B · 批次结束后 Checklist

一次连续编辑 ≤ 300 行 diff 视为一个"批次"。每批结束追加到子 Plan 末尾（`Batch Checkpoint Format`）。

```md
### Post-Batch Checklist

- [ ] 本批次所有修改在 Task Ledger 的 Step 粒度可对应。
- [ ] 未引入新 `pub` 符号到 SDK 公共面，或已登记到 `02-crate-topology.md`。
- [ ] 未在 SDK crate 中引入业务域类型（Project / Task / Workspace / Deliverable / User / Org / Team）。
- [ ] 未反向依赖（下层 crate 引用上层；见 `02-crate-topology.md §依赖图`）。
- [ ] 若修改了 `contracts/openapi/src/**`：已跑 `pnpm openapi:bundle && pnpm schema:generate` 且 diff 已入库。
- [ ] 若新增/修改了事件 kind 或 `RenderBlock.kind`：已在 `02-crate-topology.md §UI Intent IR 登记表` 更新。
- [ ] 若触及凭据 / token / OAuth：已确认事件日志扫描无明文泄漏。
- [ ] 若触及 Tool 顺序或 system prompt 分段：已跑 prompt cache 守护测试。
- [ ] 运行本批次涉及 crate 的 `cargo test -p <crate>`；全绿或失败已登记。
- [ ] 运行 `cargo clippy -p <crate> -- -D warnings`；全绿或失败已登记。
- [ ] 未改生成物（`octopus.openapi.yaml` / `generated.ts`）。
- [ ] Checkpoint 文本已按 `00-overview.md §4` 格式填写。
```

### 特殊校验钩子（根据批次涉及面勾选）

- **涉及 SDK 新 trait** → `ReadLints` 核对 crate 内部文档注释；对外 trait 必须有文档。
- **涉及持久化** → 最少一个 "写→重启→读" 的恢复测试。
- **涉及 MCP 子进程** → 最少一个 "进程崩溃→is_error=true→工具可重试" 测试。
- **涉及权限 / 审批** → 最少一个 "deny → AskPrompt → approve → Allow" 路径测试。
- **涉及 sub-agent** → 最少一个 "父子独立上下文 → 返回 condensed 摘要" 测试。
- **涉及 sandbox** → 最少一个 "容器内无凭据" 断言。

---

## 4. Layer C · 每周门禁 Checklist

在每周最后一次批次结束、提交 W<n> 收尾 PR 之前执行。未全绿则**不得开启下一周**（需要在 `00-overview.md §6 风险登记簿` append 阻塞项）。

```md
### Weekly Gate Checklist · W<n>

- [ ] 本周全部 Task 状态 = `done` 或 明确 `blocked`（带原因）。
- [ ] `00-overview.md §3 W<n> 出口状态` 逐条勾选通过。
- [ ] `00-overview.md §3 W<n> 硬门禁` 命令实际执行过并 pass。
- [ ] 当周 Checkpoint 无缺失（每批次一条）。
- [ ] 本周 PR 总 diff 行数分布记录完成（用于预警 R5 风险）。
- [ ] 未引入 `docs/sdk/*` 与实现的新矛盾；如有 → 追加到 `docs/sdk/README.md` 末尾的 "## Fact-Fix 勘误" 小节（本次重构新增的勘误累积区）。
- [ ] 新公共面符号 = `02-crate-topology.md` 登记；删除的符号 = `03-legacy-retirement.md` 勾选。
- [ ] 如本周触及 Prompt Cache 相关：命中率守护测试绿。
- [ ] 如本周触及业务接线：`pnpm -C apps/desktop test` 关键 suite 绿。
- [ ] `cargo build --workspace` 全绿；`cargo clippy --workspace -- -D warnings` 全绿。
- [ ] 完成本周"变更日志"追加到 `00-overview.md §10`。
```

---

## 5. Stop Conditions（遇到即停）

以下任一条件出现，**立即停止编码**，在 `EXECUTION_TEMPLATE.md::Needs human decision` 节写明并等待人类决策：

1. 当前 Task 涉及的 `docs/sdk/*` 规范与 `00-overview.md §1 10 项取舍` 冲突。
2. 需要引入任何业务域概念（Project / Task / Workspace / Deliverable / User / Org / Team）到 SDK crate。
3. 需要修改 `contracts/openapi/octopus.openapi.yaml` 或 `packages/schema/src/generated.ts` 手动内容。
4. 发现 Prompt Cache 命中率掉到基线的 80% 以下（测试用 mock 时以断言失败为准）。
5. 发现事件日志或 tracing 中可能含明文凭据、Token、OAuth code。
6. 单 PR 预计 diff > 800 行且无法再拆分。
7. Task 的 `Done when` 条件在本次尝试下无法验证（工具不可用、环境不一致等）。
8. 子 Plan 与 `00-overview.md` 的出口状态出现歧义。
9. 发现 legacy 代码仍被隐式依赖（例如 `runtime/sessions/*.json` 在某测试中被当作真相源）。
10. `cargo test --workspace` 出现无法归因到本批次修改的失败。
11. 违反 `docs/plans/sdk/AGENTS.md` 的命名/登记约束（出现日期前缀文件；实际 `NN-*.md` 文件与 `README.md §文档索引` 不一一对应；新建子 Plan 未先登记索引即创建；守护扫描 `§5.1 / §5.2` 命中）。

---

## 6. 汇报格式

### 6.1 单次批次汇报（每批必发）

```md
## Batch Report · <date> <hh:mm>

- Plan: `docs/plans/sdk/<file>.md`
- Week: W<n>
- Task: Task <i> (Step <j> → Step <k>)
- Files changed:
  - `path` (+/- lines)
- Tests run:
  - `cargo test -p <crate>` → pass/fail（含失败条目）
  - `cargo clippy -p <crate> -- -D warnings` → pass/fail
- Contract:
  - OpenAPI: touched / untouched
  - Schema: regenerated / unchanged
  - UI Intent IR: new kind / modified kind / unchanged
- Invariants re-checked:
  - Prompt cache stability: pass / fail / n/a
  - Credentials isolation: pass / fail / n/a
  - Config snapshot: pass / fail / n/a
- Stop conditions triggered: none / <list>
- Next step: <Task i Step k+1 | new batch | weekly gate>
```

### 6.2 每周收尾汇报

使用 `docs/plans/EXECUTION_TEMPLATE.md`；额外在末尾追加：

```md
## Weekly Summary · W<n>

- Exit state: matches / partial / blocked
- Hard gate: pass / fail
- Invariants:
  - 1. Prompt cache ...: green / at-risk
  - 2. Credentials zero-leak ...: green / at-risk
  - 3. Config snapshot ...: green / at-risk
  - 4. Session dual-channel ...: green / at-risk
  - 5. UI Intent IR pipeline ...: green / at-risk
  - 6. Narrow interfaces (4 traits) ...: green / at-risk
- Open questions opened this week: <count>
- Closed this week: <count>
- Next week kick-off ready: yes / no（若 no → 阻塞原因）
```

---

## 7. 工具与命令速查

> 本节约束 AI 在本目录 Plan 执行期**只使用**以下命令集合，任何新增命令必须更新本节。

### 7.1 契约链

```bash
pnpm openapi:bundle
pnpm schema:generate
pnpm schema:check
```

### 7.2 Rust 构建与测试

```bash
cargo build --workspace
cargo test -p <crate>
cargo test --workspace
cargo clippy -p <crate> -- -D warnings
cargo clippy --workspace -- -D warnings
```

### 7.3 前端关键 suite

```bash
pnpm -C apps/desktop exec vitest run test/openapi-transport.test.ts
pnpm -C apps/desktop exec vitest run test/runtime-store.test.ts
pnpm -C apps/desktop exec vitest run test/tauri-client-runtime.test.ts
```

### 7.4 守护扫描（任何批次结束前至少跑一次）

```bash
rg "capability_runtime|CapabilityPlanner|CapabilitySurface" crates/
rg "octopus_runtime_adapter|octopus-runtime-adapter" crates/ apps/
rg "use (runtime|tools|plugins|api)::" crates/octopus-{platform,persistence,server,desktop,cli}
rg "runtime/sessions/.*\.json" crates/
find crates -type f -name '*.rs' -exec wc -l {} + | awk '$2 != "total" && $1 > 800 { print }'
```

---

## 8. 文件级操作规则

- **新建 SDK crate** → 顶层 `Cargo.toml` `members` 必须同批更新；`workspace.dependencies` 若需新增必须锁定版本。
- **删除 crate** → 必须确认无 workspace 引用：`rg "^<crate_name>" Cargo.toml */Cargo.toml`。
- **单文件行数** → ≤ 800 行（硬约束）。W1–W7 新增 `*.rs` 必须从起就守；W8 完成全量整改。
- **测试文件** → 禁止 `split_module_tests.rs` 风格的上千行合并测试文件；拆成 `tests/<feature>.rs` 或 `src/<mod>/tests.rs`。

---

## 9. 与现有仓库协议对接

本协议是对 `/AGENTS.md` "AI Planning And Execution Protocol" 节在 SDK 重构范围内的**加严细化**：

- 所有 `/AGENTS.md` 原文约束继续生效（Plan 控制文档、任务状态标记、批次汇报、不 silently run end-to-end）。
- 本协议额外新增：三层 Checklist、Weekly Gate、10 条 Stop Conditions、守护扫描命令。
- 冲突时以**更严格**的一方为准；如果 `/AGENTS.md` 未来更新与本协议矛盾，本目录 Plan 首先 `stop and ask`。

---

## 10. 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-20 | 首稿（三层 Checklist + Stop Conditions + 每周门禁） | Architect |
| 2026-04-20 | P1 修订：Weekly Gate 的 fact-fix 追加目标明确为 `docs/sdk/README.md` 末尾新增的 "## Fact-Fix 勘误" 累计登记簿 | Architect |
| 2026-04-20 | 方案 B 落地：新增 Stop Condition #11（违反 `docs/plans/sdk/AGENTS.md` 的命名/登记约束） | Architect |
| 2026-04-21 | 审计修复：`§7.4` 单文件 ≤ 800 行守护从 `find -size +800` 改为 `wc -l + awk` 行数检查，避免把文件字节大小误当成 Weekly Gate 的行数门禁 | Codex |
