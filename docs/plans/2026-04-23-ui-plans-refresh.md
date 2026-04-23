# 2026-04-23 · UI Plan Refresh For Refactored Desktop

> **For Codex:** Refresh `docs/plans/ui/*.md` so they match the current repository structure, current desktop shell/runtime ownership, and current verification entrypoints. Do not implement product code from those plans in this task.

## Goal

让 `docs/plans/ui` 下 5 份 UI 执行计划回到当前仓库真实状态，移除旧项目遗留的错误文件路径、错误组件归属、错误测试入口和过时概念。

## Architecture

这次工作只改执行文档，不改产品代码。计划内容必须以当前 `apps/desktop` 壳层、`@octopus/ui` 导出面、`packages/schema` 契约和 `apps/desktop/test` 测试结构为准；已存在能力要写成“扩展/迁移/对齐”，不能继续写成“从零新建”。

## Scope

- In scope:
  - 刷新 `docs/plans/ui/*.md` 的目标、架构、任务、文件列表、前置条件、验收条件、验证命令
  - 把旧概念替换成当前真实链路，例如 `WorkbenchSearchOverlay`、`ConversationView`、`ConversationMessageBubble`、`runtime/app-error-boundary`
  - 修正失效路径、失效测试目录、失效命令、已不存在的组件引用
  - 明确哪些任务应改成 blocked/迁移/扩展，而不是继续按旧前提执行
- Out of scope:
  - 实现计划里的 UI 或 runtime 代码
  - 新增产品能力
  - 修改仓库治理规则

## Risks Or Open Questions

- 若某个旧计划依赖的产品方向已经被新架构完全替代，需要重写任务主线，而不是只修路径。
- 若当前仓库没有对应测试或依赖（例如 Playwright / axe），计划必须先写“建立基线”的前置任务，不能直接保留旧验证命令。
- 若某项能力已在当前仓库落地，需要把任务改成“补齐共享化 / 清债 / 回归覆盖”，避免重复建设。

## Execution Rules

- Do not start plan rewrites until the current file ownership and test entrypoints are verified from the repository.
- Keep each UI plan executable against the current repo; do not leave placeholders like `src/**/*.ts` without narrowing the real owner files.
- When a legacy concept no longer maps to a current component, rewrite the task around the current source of truth instead of preserving the old name.
- After each batch, update this plan so the current step is recoverable without chat context.

## Task Ledger

### Task 1: Inventory Current UI Execution Surfaces

Status: `done`

Files:
- Read: `docs/plans/ui/2026-04-21-ui-tokens-alignment.md`
- Read: `docs/plans/ui/2026-04-21-ui-states-system.md`
- Read: `docs/plans/ui/2026-04-21-ui-ai-feedback.md`
- Read: `docs/plans/ui/2026-04-21-ui-performance-a11y.md`
- Read: `docs/plans/ui/2026-04-21-ui-polish-microinteractions.md`
- Read: `docs/design/DESIGN.md`
- Read: `apps/desktop/src/App.vue`
- Read: `apps/desktop/src/layouts/WorkbenchLayout.vue`
- Read: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Read: `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- Read: `apps/desktop/src/views/project/ConversationView.vue`
- Read: `apps/desktop/src/components/conversation/ConversationMessageBubble.vue`
- Read: `apps/desktop/src/views/project/TraceView.vue`
- Read: `apps/desktop/src/components/layout/AppRuntimeErrorBoundary.vue`
- Read: `apps/desktop/test/*.test.ts`
- Read: `packages/ui/src/index.ts`
- Read: `packages/ui/src/components/*.vue`
- Read: `packages/ui/src/tokens.css`
- Read: `tailwind.config.js`

Preconditions:
- None.

Step 1:
- Action: Build a concrete mismatch inventory for each UI plan: stale file paths, stale commands, missing components, replaced concepts, and current verification entrypoints.
- Done when: Every plan has a clear list of what must be rewritten before editing starts.
- Verify: `ls docs/plans/ui && rg -n "apps/desktop/tests|__tests__|UiCommandPalette|UiMessageCenter|useConnectionStore|docs/plans/design/design.md" docs/plans/ui/*.md`
- Stop if: Current ownership for a planned surface cannot be proven from the repo.

### Task 2: Rewrite Foundation And State Plans

Status: `done`

Files:
- Modify: `docs/plans/ui/2026-04-21-ui-tokens-alignment.md`
- Modify: `docs/plans/ui/2026-04-21-ui-states-system.md`

Preconditions:
- Task 1 mismatch inventory completed.

Step 1:
- Action: Rewrite token/state plans around the current token layer, current shared UI export surface, current runtime error handling, and current shell connection state.
- Done when: No task in these two plans points at removed files, nonexistent tests, or superseded ownership boundaries.
- Verify: `rg -n "apps/desktop/tests|packages/ui/src/components/__tests__|useConnectionStore|docs/plans/design/design.md" docs/plans/ui/2026-04-21-ui-tokens-alignment.md docs/plans/ui/2026-04-21-ui-states-system.md`
- Stop if: A current state owner still cannot be identified between `@octopus/ui`, desktop shell, and runtime store.

### Task 3: Rewrite AI, Performance, And Polish Plans

Status: `done`

Files:
- Modify: `docs/plans/ui/2026-04-21-ui-ai-feedback.md`
- Modify: `docs/plans/ui/2026-04-21-ui-performance-a11y.md`
- Modify: `docs/plans/ui/2026-04-21-ui-polish-microinteractions.md`

Preconditions:
- Task 1 mismatch inventory completed.

Step 1:
- Action: Replace old message-center, command-palette, Playwright-path, and package-local test assumptions with the current conversation, trace, search overlay, topbar, and `apps/desktop/test` structure.
- Done when: These three plans describe executable work against current repo surfaces, and explicit blockers remain only where the current repo truly lacks a dependency or contract.
- Verify: `rg -n "apps/desktop/tests|packages/ui/src/components/__tests__|UiCommandPalette|UiTooltip|UiMessageCenter 智能滚动锚定|useCommandRegistry|@axe-core/playwright" docs/plans/ui/2026-04-21-ui-ai-feedback.md docs/plans/ui/2026-04-21-ui-performance-a11y.md docs/plans/ui/2026-04-21-ui-polish-microinteractions.md`
- Stop if: A feature has already been implemented under a different name and needs human product confirmation instead of a pure doc rewrite.

### Task 4: Validate Refreshed Plans

Status: `done`

Files:
- Modify: `docs/plans/2026-04-23-ui-plans-refresh.md`
- Review: `docs/plans/ui/*.md`

Preconditions:
- Tasks 2 and 3 completed.

Step 1:
- Action: Re-scan all 5 plans for nonexistent file references and stale command paths; then append a checkpoint here with the final state.
- Done when: Remaining missing references are intentional future files only, not stale or wrong current-repo paths.
- Verify: `node - <<'NODE'\nconst fs = require('fs')\nconst path = require('path')\nconst root = process.cwd()\nconst prefixes = ['docs/', 'apps/', 'packages/', 'scripts/', 'contracts/', 'crates/', 'tailwind.config.js', 'package.json', 'AGENTS.md']\nfor (const file of fs.readdirSync(path.join(root, 'docs/plans/ui')).filter(f => f.endsWith('.md')).sort()) {\n  const text = fs.readFileSync(path.join(root, 'docs/plans/ui', file), 'utf8')\n  const matches = [...text.matchAll(/`([^`]+)`/g)].map(m => m[1])\n  const paths = [...new Set(matches.filter(v => prefixes.some(prefix => v.startsWith(prefix) || v === prefix)))]\n  console.log('\\n# ' + file)\n  for (const p of paths) {\n    const exists = fs.existsSync(path.join(root, p))\n    console.log((exists ? 'OK   ' : 'MISS ') + p)\n  }\n}\nNODE`
- Stop if: The refreshed plans still point at removed repo paths or invalid test directories.

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 2 Step 1
- Completed:
  - short list
- Verification:
  - `command` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 1
```

## Checkpoint 2026-04-23 11:16 CST

- Batch: Task 1 Step 1 -> Task 4 Step 1
- Completed:
  - 重写 `docs/plans/ui/2026-04-21-ui-tokens-alignment.md`
  - 重写 `docs/plans/ui/2026-04-21-ui-states-system.md`
  - 重写 `docs/plans/ui/2026-04-21-ui-ai-feedback.md`
  - 重写 `docs/plans/ui/2026-04-21-ui-performance-a11y.md`
  - 重写 `docs/plans/ui/2026-04-21-ui-polish-microinteractions.md`
  - 清理旧测试目录、旧组件名、旧 store 名和失效路径引用
- Verification:
  - `rg -n "apps/desktop/tests|packages/ui/src/components/__tests__|UiCommandPalette|UiTooltip|useConnectionStore|docs/plans/design/design.md|apps/desktop/src/mocks|useCommandRegistry|@axe-core/playwright" docs/plans/ui/*.md` -> pass
  - refined path scan across `docs/plans/ui/*.md` -> pass; only planned future files remain as `MISS`
- Blockers:
  - none
- Next:
  - none
