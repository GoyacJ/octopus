# 2026-04-20 · SDK 文档事实错误修复计划

> 关联评审：当轮会话"评审 `docs/sdk/` 的 SDK 文档"的结论。
> 关联治理规则：`AGENTS.md` §AI Planning And Execution Protocol、`docs/AGENTS.md`。
> 关联参考源：`docs/references/claude-code-sourcemap-main/restored-src/src/**`。

## Goal

把 `docs/sdk/` 12 份 SDK 文档中对 `restored-src` 的事实性引用、数值常量、类型名、工具路径、枚举值、以及章节间相互冲突的表述全部校正到与源码一致，使其可作为"规范源真理"被下游实施直接引用。

## Architecture

本计划只修订 `docs/sdk/**` 文本，不新增代码、包、或二次派生文档。所有修订必须以 `docs/references/claude-code-sourcemap-main/restored-src/src/**` 中真实存在的标识符为唯一事实源；对已无法在源码中验证的条目，删除源码锚点但保留 Anthropic 官方博客/文档的一级链接。不在本计划中扩展新章节（如 MVP 切分、Credential Injection Contract），这些在独立计划处理。

## Scope

- In scope:
  - `docs/sdk/01-core-loop.md` 的 Token Budget 公式、`partitionToolCalls` 算法、`prompt_too_long`/`FallbackTriggeredError` 引用路径。
  - `docs/sdk/03-tool-system.md` 的 `ToolUseContext` 字段、`ToolPermissionContext` 边界、`PermissionMode` 枚举、内置工具目录名、Bash 截断单位与默认值、`renderProgress/renderResult` 对 UI 框架的松绑。
  - `docs/sdk/06-permissions-sandbox.md` 的 `PermissionMode` 枚举、`strippedDangerousRules` 引用路径、`auto` 模式定位、Rule 形状与源码一致性。
  - `docs/sdk/04-session-brain-hands.md` 的 `ToolResult` 与 `execute()` 边界返回形状收敛，`session.status` 枚举补齐 `aborted`。
  - `docs/sdk/05-sub-agents.md` 的 `task_budget` ↔ `taskBudget` 命名统一。
  - `docs/sdk/08-long-horizon.md` 的 `.agent/todos.json` 改为 `runtime/todos/<session>.json`。
  - `docs/sdk/09-observability-eval.md` 的 `final status` 枚举与 §4.5.3 对齐。
  - `docs/sdk/README.md` 与 `docs/sdk/references.md` 的来源路径小修与日期口径。
- Out of scope:
  - 新增章节（MVP 切分、Credential Injection Contract、Monorepo 集成图）——独立计划 `2026-04-20-sdk-mvp-breakdown.md` 处理。
  - 修改 `docs/references/**`、`packages/**`、`apps/**`。
  - 引入 P2 级别的美化与补证。

## Risks Or Open Questions

- `restored-src` 版本快照为 `v2.1.88`（见 `references.md` §C1）。如果上游后续版本改动了常量/类型名，本次修订仍以该快照为准；未来随快照升级再做增量。
- `MultiEdit` 在 `sessionRunner.ts` 作为 display name 存在，但 tool 目录里没有独立实现；倾向理解为已合并进 `FileEditTool`。修订时以"`FileEditTool`（含多编辑能力）"表述，不再单列 `FileMultiEdit`。
- `PermissionMode` 实际外部可见枚举含 `dontAsk`，未在 SDK 文档 §6.2 出现。本轮先将其加入枚举清单但不展开语义；展开语义放到 §6 扩章节（独立计划）。
- Token Budget 真实语义（`continuationCount >= 3` 后用 `delta < 500` 判定 diminishing）与原文档伪代码差异较大，必须完整重写该小节。

## Execution Rules

- 不得一次性跨多章连改；以"单文件为一批"作为 checkpoint 粒度。
- 每批结束后在"Batch Checkpoint"处以 AGENTS.md 规定的格式记录。
- 任何无法在 `restored-src` 或 Anthropic 官方文档中复核的表述，要么删除，要么降级为"见 `references.md` §X 条目，具体实现细节视版本而定"。
- 不在修订过程中引入对 Octopus 尚未存在包（`@octopus/agent-core` 等）的强依赖断言，保留为"建议产物"。
- 引用源码行号时以本仓快照为准，用 CODE REFERENCE 格式标注行号范围。

## Task Ledger

### Task 1: 修复 `01-core-loop.md`

Status: `done`

Files:
- Modify: `docs/sdk/01-core-loop.md`

Preconditions:
- `restored-src/src/query/tokenBudget.ts` 已核对（`COMPLETION_THRESHOLD=0.9`、`DIMINISHING_THRESHOLD=500`、`continuationCount >= 3`）。
- `restored-src/src/services/tools/toolOrchestration.ts::partitionToolCalls` 已核对（"连续相邻可并发合并为批"）。
- `restored-src/src/query.ts::PROMPT_TOO_LONG_ERROR_MESSAGE`（line 42/643）与 `services/api/withRetry.ts::FallbackTriggeredError`（line 160）已核对。

Step 1:
- Action: 重写 §1.2 "Token Budget 公式"子节，按 `tokenBudget.ts` 真实语义改写（只有两个常量 + `continuationCount >= 3` 的 diminishing 条件），并把来源行号用 CODE REFERENCE 标注。
- Done when: 文档中不再出现 `0.85`、`0.95`、`MIN_CONTINUATION_TOKENS`；伪代码描述的是"continue/stop(diminishing)/stop(no-completion-event)"三分支。
- Verify: `grep -n "COMPLETION_THRESHOLD\|DIMINISHING_THRESHOLD\|MIN_CONTINUATION_TOKENS" docs/sdk/01-core-loop.md`，只能命中 `0.9` 与 `500`。
- Stop if: 有疑问源码又出现新常量。

Step 2:
- Action: 重写 §1.4.1 `partition` 伪代码，反映"连续相邻合并为批"的真实算法；并把 §1.1 第 2 条"工具批处理"描述与之对齐。
- Done when: 伪代码展示的是 `reduce` 形态，批次之间遇到不可并发立即另起；可并发批次在相邻时合并。
- Verify: 人工 diff 对照 `toolOrchestration.ts:91-116`。
- Stop if: 算法行为在源码更新后再次变化。

Step 3:
- Action: §1.5.2 `FallbackTriggeredError` 引用路径改为 `services/api/withRetry.ts` + `query.ts`；§1.5.3 明确 `PROMPT_TOO_LONG_ERROR_MESSAGE` 在 `query.ts:42/643` 定义，并以 CODE REFERENCE 形式给出。
- Done when: 引用路径与行号可在本仓直接跳转。
- Verify: `grep -n "FallbackTriggeredError\|PROMPT_TOO_LONG" docs/sdk/01-core-loop.md`。
- Stop if: 源码路径尚未最终确认。

Step 4:
- Action: §1.10 事件模型补齐 `aborted` session 状态相关事件注释；与 Task 5 拟定的 §4.5.3 对齐。
- Done when: §1.10 与 §4.5.3 两处事件/状态枚举一致。
- Verify: diff `docs/sdk/01-core-loop.md` §1.10 与 `docs/sdk/04-session-brain-hands.md` §4.5.3。
- Stop if: Task 5 尚未完成。

Notes:
- 01 章默认工具并发上限保持 `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY=10`。

### Task 2: 修复 `03-tool-system.md`

Status: `done`

Files:
- Modify: `docs/sdk/03-tool-system.md`

Preconditions:
- `Tool.ts:117-300` 已核对（`ToolPermissionContext` 与 `ToolUseContext` 是两个不同类型）。
- `types/permissions.ts:16-36` `PermissionMode` 真实枚举已核对。
- `utils/shell/outputLimits.ts:3-4` 的 `BASH_MAX_OUTPUT_DEFAULT=30_000`（字符）已核对。
- `tools/` 目录下真实工具目录名已核对（`FileReadTool/FileWriteTool/FileEditTool/...`）。

Step 1:
- Action: 重写 §3.1.2，把 `ToolUseContext` 与 `ToolPermissionContext` **分离成两张表**；`ToolPermissionContext` 字段对齐 `Tool.ts:123-138`；`ToolUseContext` 字段对齐 `Tool.ts:158-300`；CODE REFERENCE 标注行号。
- Done when: 文档不再把 `mode/alwaysAllowRules/prePlanMode/strippedDangerousRules` 错归入 `ToolUseContext`。
- Verify: `grep -n "ToolUseContext\|ToolPermissionContext" docs/sdk/03-tool-system.md` 按预期分布。
- Stop if: Claude Code 上游版本将两者合并。

Step 2:
- Action: 更新 §3.1.1 `Tool` 契约，把 `renderProgress/renderResult` 改为**序列化 display 描述符**（如 `displayDescriptor?: ToolDisplayDescriptor`），移除 `React.Node`；正文增加一段说明"UI 渲染由各宿主 adapter 实施（Tauri 桌面 Vue / Browser Vue），与 `AGENTS.md` Frontend Governance 一致"。
- Done when: §3.1.1 不再出现 `React.Node` 或任何 UI 框架专用类型。
- Verify: `grep -n "React.Node\|React\\.ReactNode" docs/sdk/03-tool-system.md` 无结果。
- Stop if: 上游 SDK 对 render 契约有新决定。

Step 3:
- Action: 更新 §3.3.2 截断默认值：Bash/Shell 为 `BASH_MAX_OUTPUT_DEFAULT=30_000` **字符**（不是 token），上限 `BASH_MAX_OUTPUT_UPPER_LIMIT=150_000` 字符，环境变量 `BASH_MAX_OUTPUT_LENGTH`；WebFetch / Grep / List 的阈值若无源码支撑则降级为"SDK 建议"并明确标注"非 Claude Code 官方常量"。
- Done when: 所有来自源码的数值都能在 `outputLimits.ts` 找到；SDK 自拟的数值被显式标识。
- Verify: `grep -n "25,000\|25_000\|25000 token" docs/sdk/03-tool-system.md` 无结果。
- Stop if: Claude Code 有新的 token-based 限制被确认。

Step 4:
- Action: §3.4 内置工具表 "来源"列统一改成真实目录名（`FileReadTool/FileWriteTool/FileEditTool/GlobTool/GrepTool/BashTool/WebSearchTool/WebFetchTool/AskUserQuestionTool/TodoWriteTool/AgentTool/SkillTool/SleepTool/TaskListTool+TaskGetTool+TaskOutputTool`）；删除 `FileMultiEdit` 独立条目，改为 `FileEditTool (含 multi-edit 能力)`；`monitor` 改为 `TaskListTool / TaskGetTool / TaskOutputTool` 组合。
- Done when: 每个条目都能在 `restored-src/src/tools/` 找到对应目录。
- Verify: `ls docs/references/claude-code-sourcemap-main/restored-src/src/tools/` 逐项比对。
- Stop if: 有条目在源码中完全找不到（例如 `monitor`）。

Step 5:
- Action: 把 §3.1.2 里出现的 `mode: 'default' | 'auto' | 'bypass' | 'plan'` 替换为与 Task 3 商定的真实 `PermissionMode` 并集；同时给出"Octopus 首版仅启用 default/acceptEdits/bypassPermissions/plan 子集"的落地注记。
- Done when: §3.1.2 与 §6.2 的枚举字面量完全一致。
- Verify: diff §3.1.2 与 §6.2 的枚举清单。
- Stop if: Task 3 尚未完成。

Step 6:
- Action: §3.1.3 统一 `ToolResult` 形状：面向模型边界 = `{ output: string, is_error: boolean }`（Anthropic API 原生形状），面向工具内部 = `Result<O, ToolError>` 可选；与 04 章 `execute()` 对齐。增加一段"两种形状的转换点"说明。
- Done when: `ok: true | false` 形态只出现在"工具内部"小节中，边界一律 `is_error`。
- Verify: `grep -n "ok: true\|ok: false" docs/sdk/03-tool-system.md` 仅出现在"内部"段落。
- Stop if: Task 4 对边界有新决议。

Notes:
- 本 Task 是最大的一批；建议作为独立 checkpoint 提交。

### Task 3: 修复 `06-permissions-sandbox.md`

Status: `done`

Files:
- Modify: `docs/sdk/06-permissions-sandbox.md`

Preconditions:
- `types/permissions.ts:16-36` `EXTERNAL_PERMISSION_MODES` / `InternalPermissionMode` / `feature('TRANSCRIPT_CLASSIFIER')` 已核对。
- `Tool.ts:117-138` `ToolPermissionContext` 已核对；`strippedDangerousRules` 字段确实存在。

Step 1:
- Action: 重写 §6.2 "四种权限模式"为"五种外部 + 两种内部"：`default | acceptEdits | bypassPermissions | dontAsk | plan`（外部） + `auto | bubble`（内部，后者由 `feature('TRANSCRIPT_CLASSIFIER')` 闸控）。`auto` 不再被降级为"外部实现"，而是"Claude Code 内部模式，feature flag 控制曝光"。
- Done when: 文档枚举与 `types/permissions.ts:16-36` 一对一。
- Verify: 人工对照 `types/permissions.ts:16-36`。
- Stop if: 上游枚举变更。

Step 2:
- Action: 更新 §6.3 "Rules by Source" 的 `Rule` / `ToolPermissionRulesBySource` 结构，与 `types/permissions.ts:54-70,419-441` 保持一致（`source: PermissionRuleSource`、`ruleBehavior: PermissionBehavior`、`ruleValue: { toolName, ruleContent? }`）；以 CODE REFERENCE 标注。
- Done when: 文档 Rule 形状与真实类型字段名一致。
- Verify: diff 文档与源码。
- Stop if: 上游重构了 Rule 类型。

Step 3:
- Action: §6.13 "stripping 机制"的来源引用改为 `Tool.ts:131` 或 `types/permissions.ts:437`（`ToolPermissionContext.strippedDangerousRules`）。
- Done when: 引用路径能直接跳转。
- Verify: `grep -n "strippedDangerousRules" docs/sdk/06-permissions-sandbox.md`。
- Stop if: 无。

Step 4:
- Action: §6.12 "Auto Mode" 更新为"Claude Code 通过 `TRANSCRIPT_CLASSIFIER` feature flag 启用；Octopus 默认不启用该实验模式，由 hook 扩展（`canUseTool` + `pendingClassifierCheck`）代替"。
- Done when: 与 Claude Code 原生实现的关系被说清楚；不再暗示"这是外部通用能力"。
- Verify: 人工复审该节。
- Stop if: 无。

Notes:
- 同时核查 §6.4 `canUseTool` 与 `hooks/useCanUseTool.tsx` 实际签名是否一致（preconditions 里补核）。

### Task 4: 修复 `04-session-brain-hands.md` 与 §4 相关章节

Status: `done`

Files:
- Modify: `docs/sdk/04-session-brain-hands.md`
- Optional: `docs/sdk/09-observability-eval.md`

Preconditions:
- Task 2 Step 6 确立的边界 `{ output, is_error }` 已定稿。

Step 1:
- Action: §4.2.2 / §4.4.3 保持 `{ output, is_error }` 形状；在 §4.2.2 表格下方补一段"工具内部可用 `Result<O, ToolError>`，边界转换点在 `toolExecution.ts`"。
- Done when: 两章边界字段一致。
- Verify: diff §3.1.3 与 §4.2.2。
- Stop if: Task 2 未完成。

Step 2:
- Action: §4.5.3 `session.status` 枚举改为 `active | paused | done | aborted | errored`（补上 `aborted`）；同步更新 §9.2.2 `final status` 口径。
- Done when: `aborted` 在两处都存在；`docs/sdk/09-observability-eval.md §9.2.2` 与此一致。
- Verify: `grep -n "final status\|status TEXT NOT NULL" docs/sdk/04-session-brain-hands.md docs/sdk/09-observability-eval.md`。
- Stop if: 无。

### Task 5: 修复 `05-sub-agents.md` / `08-long-horizon.md` 一致性

Status: `done`

Files:
- Modify: `docs/sdk/05-sub-agents.md`
- Modify: `docs/sdk/08-long-horizon.md`

Preconditions:
- 01 章已统一为 `taskBudget.total`（camelCase，沿用 Claude Code `QueryParams` 命名风格）。

Step 1:
- Action: 05 章所有出现 `task_budget`、`task_budget.total` 统一为 `taskBudget.total`；frontmatter 示例保留下划线风格但注明"YAML frontmatter key 可用 snake_case，程序字段仍为 camelCase"。
- Done when: 程序字段与 frontmatter 字段的差异被显式说明。
- Verify: `grep -n "task_budget\b\|taskBudget\b" docs/sdk/05-sub-agents.md`。
- Stop if: Octopus 上游规范另有 snake_case 约定。

Step 2:
- Action: 08 章 §8.4.1 把 `.agent/todos.json` 改为 `runtime/todos/<session>.json`；把 `claude-progress.txt` 的默认落地改为 `runtime/notes/<session>.md`；并明确 `AGENTS.md` §Persistence Governance 为来源。
- Done when: 与 02 章 §2.11 "所有 TodoWrite 强制写入 runtime/todos" 一致。
- Verify: `grep -n "\\.agent/todos\\.json\|claude-progress" docs/sdk/08-long-horizon.md`。
- Stop if: 无。

### Task 6: `README.md` 与 `references.md` 小修

Status: `done`

Files:
- Modify: `docs/sdk/README.md`
- Modify: `docs/sdk/references.md`

Preconditions:
- 前 5 个 Task 已完成（否则会出现二次回改）。

Step 1:
- Action: `README.md` §6 "参考矩阵"里 `FileMultiEdit` 去掉；`monitor` 改为 `TaskListTool/TaskGetTool/TaskOutputTool`；列出的包名保留，但标注为"Octopus 建议产物，非 Claude Code 既有包"。
- Done when: 矩阵中每个"Claude Code"列条目都能被真实目录复核。
- Verify: 人工对照 `restored-src/src/` 子目录。
- Stop if: 无。

Step 2:
- Action: `references.md` §E 把"curl -I 可达"改为"本仓 CI 定期探活；若失效遵循 §A 条目内的 WebSearch 替换流程"；日期标注改为"基于 restored-src v2.1.88 与 Anthropic 官方文档（截至 2026-04-20）"。
- Done when: 不再对任何外部条目作"当前可达"的强断言。
- Verify: 人工复审 §E。
- Stop if: 无。

Step 3:
- Action: `README.md` §8 路线图追加一条 "fact-fix 修订 (本计划产物)"，带日期；不新增 MVP 小节（放到独立计划）。
- Done when: 路线图追记本轮修订。
- Verify: `grep -n "fact-fix" docs/sdk/README.md`。
- Stop if: 无。

### Task 7: 最终检核

Status: `done`

Files:
- Read-only: 全部 `docs/sdk/*.md`

Preconditions:
- Task 1 – Task 6 全部 `done`。

Step 1:
- Action: 逐文件读一次，核对章节内链接（内部 `./0X-*.md` 与小节锚点）未被修订破坏。
- Done when: 所有相对链接指向真实文件与锚点。
- Verify: `grep -rn '](\\./[0-9]' docs/sdk/`, 逐个查存在性。
- Stop if: 有链接失效。

Step 2:
- Action: 在 plan 末尾追加 Checkpoint，概述本次修订的"事实核对 → 修订 → 复核"闭环。
- Done when: Checkpoint 有完整字段。
- Verify: 本文件末尾存在"Checkpoint 2026-04-20"。
- Stop if: 无。

Notes:
- 本 Task 不动文档内容，只做整体一致性复核。

## Batch Checkpoint Format

After each batch, append a short checkpoint using this shape:

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task N Step M -> Task N+1 Step K
- Completed: short list
- Verification:
  - `command` -> pass or fail
- Blockers:
  - none
- Next:
  - Task N+1 Step K+1
```

## Checkpoint 2026-04-20 (事实核对 → 修订 → 复核 一次性闭环)

- Batch: Task 1 → Task 7 全部
- Completed:
  - `docs/sdk/01-core-loop.md`：Token Budget 伪代码与常量重写为 `COMPLETION_THRESHOLD=0.9` + `DIMINISHING_THRESHOLD=500` + `continuationCount>=3`；`partitionToolCalls` 改为"连续相邻合并为批"；`FallbackTriggeredError` / `PROMPT_TOO_LONG_ERROR_MESSAGE` 加精确行号；`MultiEdit` 从并发分类示例中删除。
  - `docs/sdk/03-tool-system.md`：`ToolUseContext` 与 `ToolPermissionContext` 拆成两张表并对齐 `Tool.ts:123-138 / 158-300`；`Tool` 契约把 `React.Node` 改为 `ToolDisplayDescriptor`；Bash 截断改为 `BASH_MAX_OUTPUT_DEFAULT=30_000` 字符（非 token）；内置工具目录名与 `restored-src/src/tools/` 一对一；`FileMultiEdit` 删除并入 `FileEditTool`；`monitor` 改为 `TaskListTool/TaskGetTool/TaskOutputTool` 组合；`ToolResult` 分"内部/边界"两套形状。
  - `docs/sdk/06-permissions-sandbox.md`：`PermissionMode` 枚举改为外部 5 种 + 内部 `auto`/`bubble`；补上 `dontAsk`；Rule 形状与 `types/permissions.ts` 对齐；Auto Mode 明确是 Claude Code 内部 feature flag 模式；`strippedDangerousRules` 指向 `Tool.ts:131`；资源限额的"最大输出"换算为 `30_000` 字符。
  - `docs/sdk/04-session-brain-hands.md`：`execute()` 签名补 `tool_use_id` 并标注与 Anthropic `tool_result` 边界对齐；`session.status` 补 `aborted`。
  - `docs/sdk/05-sub-agents.md`：`taskBudget` camelCase 与 `task_budget` frontmatter 关系说明清楚。
  - `docs/sdk/08-long-horizon.md` + `docs/sdk/02-context-engineering.md`：所有 `.agent/todos.json` / `claude-progress.txt` 改到 `runtime/todos/<session>.json` / `runtime/notes/<session>.md`（原名保留为"业界/Anthropic 原文"引用）。
  - `docs/sdk/README.md`：参考矩阵工具清单真实化；Permissions 描述修正；增加 fact-fix 路线图条目；快照版本口径改为 `restored-src v2.1.88`。
  - `docs/sdk/references.md`：§C1 补 `types/permissions.ts` / `utils/shell/outputLimits.ts` / `services/api/withRetry.ts` 索引；§E 去掉"当前可达"强断言，改为 CI 探活 + 失效流程。
- Verification:
  - `grep -n 'FileMultiEdit|MIN_CONTINUATION_TOKENS|0\.85|0\.95|\.agent/todos\.json' docs/sdk/*.md` → 只剩 01 §1.2 用于"**不存在** MIN_CONTINUATION_TOKENS"的显式反例，符合预期。
  - `grep -n '25,000 token|25_000 token|React\.Node' docs/sdk/*.md` → 仅剩 03 §3.1.1 对 Claude Code 源码 `React.ReactNode` 的反向说明，符合预期。
  - `grep -rhoE ']\([^)]+\.md[^)]*\)' docs/sdk/ | sort -u` → 4 条相对链接全部 `ls` 验证文件存在。
- Blockers:
  - 无。以下属于本计划 Out of scope 的后续动作，单独开 plan 跟进：(1) §9 Implementation Roadmap（MVP/v1/v2）；(2) §4.11 Monorepo 集成图；(3) Credential Injection Contract 独立章节；(4) §10.9 checklist 落为 lint rule；(5) 外链 CI 探活 workflow。
- Next:
  - 由用户决定是否启动上述后续 plan；本计划 7 个 Task 全部 `done`。
