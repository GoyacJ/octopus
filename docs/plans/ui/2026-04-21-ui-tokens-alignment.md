# 2026-04-21 · UI Tokens Alignment (Calm Intelligence 基座对齐)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

把 `docs/design/DESIGN.md` 的 Calm Intelligence 规范 100% 落到 `@octopus/ui` 与 Tailwind token 层，清理所有与规范冲突的残留命名和一次性值，使后续 4 份计划（P2/P3/P4/P5）共享同一套可信 token。

## Architecture

所有视觉 token 的单一可信源是 `packages/ui/src/tokens.css`；Tailwind config 仅做 `var(--*)` 映射；`@octopus/ui` 组件与业务页通过 Tailwind utility 或 `var(--*)` 消费 token，禁止一次性内联值。motion preset 统一挂在 `packages/ui/src/lib/motion.ts`，所有交互动效从此一个入口取值。

## Scope

- In scope:
  - `packages/ui/src/tokens.css` 字号 scale + density + 清理 glass 别名
  - `tailwind.config.js` 颜色/字号/过渡的 Tailwind 映射
  - `packages/ui/src/lib/motion.ts` motion preset 扩建
  - 新增 `UiKbd` 共享组件
  - `docs/design/DESIGN.md` §5.5 排版表补 `Micro` 档
  - 删除 `docs/plans/design/design.md` 中与规范冲突的残留
  - `@octopus/ui` 内部一次性字号/阴影/blur/glass 引用审计并清零
- Out of scope:
  - `apps/desktop` 业务页的一次性值迁移（量大，留待 P5 抛光阶段或单独债务计划）
  - 组件行为层改动（仅动视觉 token 与静态类名）
  - 新增 `UiSkeleton / UiErrorBoundary / UiOfflineBanner / UiRestrictedState / UiCommandPalette`（归属 P2/P5）

## Risks Or Open Questions

- DESIGN.md §5.5 排版表目前到 `Badge (12px)` 为止。本计划将新增一行 `Micro (11px / 600 / uppercase / tracking 0.02em)`，用于时间戳、eyebrow、status chip 等微型元素。**此条变更须在 T1 落地前在 DESIGN.md 同步，本计划承担该文档更新**。
- Density token 只在 shell 级别提供（`UiPageShell` 暴露 `density` prop），**业务页不得直接读 density token 写一次性间距**。若业务页需要密度敏感的行为，必须复用带 density 响应的共享组件。
- `--bg-glass` 与 tailwind `glass` 颜色别名目前值等于 `surface`。直接删除会对仍引用它的业务页报错，需先 grep 全仓库引用并迁移，再删除别名。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a business-page-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.
- Every T 完成后立即跑 `pnpm check:frontend-governance && pnpm -C apps/desktop typecheck`，红即回退。

## Task Ledger

### Task 1: DESIGN.md 补 Micro 字号档

Status: `pending`

Files:
- Modify: `docs/design/DESIGN.md`

Preconditions:
- 用户已确认在 §5.5 排版表补 Micro 档（q6 = `add_micro_type`）。

Step 1:
- Action: 在 §5.5 的 Hierarchy 表格末行后新增一行 `Micro | 11px | 600 | 1.2 | 0.02em`，并在表格下方 Rules 列追加一条 "Micro 用于 eyebrow、时间戳、status chip 与 tab badge；uppercase 仅作用于 Micro 与 Badge 两档"。
- Done when: DESIGN.md 新行存在；Rules 里对 Micro 有使用边界描述。
- Verify: `rg -n "^\| Micro" docs/design/DESIGN.md` 返回 1 行命中。
- Stop if: §5.5 表格已经被别的 PR 改动过，列数或字段含义对不上本计划预设。

### Task 2: 字号 scale 落到 tokens.css + tailwind

Status: `pending`

Files:
- Modify: `packages/ui/src/tokens.css`
- Modify: `tailwind.config.js`

Preconditions:
- Task 1 已完成（DESIGN.md 规范口径先行）。

Step 1:
- Action: 在 `tokens.css` `:root` 段新增 7 档字号变量：`--font-size-page-title: 30px`、`--font-size-section-title: 22px`、`--font-size-card-title: 16px`、`--font-size-body: 14px`、`--font-size-label: 13px`、`--font-size-caption: 12px`、`--font-size-badge: 12px`、`--font-size-micro: 11px`；每档配套 `--line-height-*` 与 `--letter-spacing-*`（按 DESIGN.md §5.5 数值）。
- Done when: `rg -n "font-size-page-title" packages/ui/src/tokens.css` 命中 1；7 档字号变量全部存在。
- Verify: `pnpm -C apps/desktop typecheck` 通过。
- Stop if: tokens.css 已存在同名变量但值不一致，需人工裁决。

Step 2:
- Action: 在 `tailwind.config.js` `theme.extend.fontSize` 新增映射：`'page-title'`、`'section-title'`、`'card-title'`、`'body'`、`'label'`、`'caption'`、`'badge'`、`'micro'`，每项使用 `[fontSize, { lineHeight, letterSpacing, fontWeight }]` 三元组直接从 CSS var 取值。
- Done when: `text-body text-page-title text-micro` 等类名在 `apps/desktop/src` 任一 Vue 文件里可通过 Tailwind 编译。
- Verify: `pnpm -C apps/desktop build:ui`（或在一个临时 .vue 写 `<div class="text-micro">` 后运行 typecheck）通过。
- Stop if: Tailwind 无法解析 CSS var 的 letterSpacing（tailwind 3.x 已支持，若失败则用 JS 常量回退）。

### Task 3: Density token + UiPageShell `density` prop

Status: `pending`

Files:
- Modify: `packages/ui/src/tokens.css`
- Modify: `packages/ui/src/components/UiPageShell.vue`
- Modify: `docs/design/DESIGN.md`（§6.4 Main Canvas 补 density 说明）

Preconditions:
- Task 2 已完成。

Step 1:
- Action: 在 tokens.css 新增 3 组 density 变量：`--density-compact-row: 32px; --density-regular-row: 40px; --density-comfortable-row: 48px;` 同步定义 `--density-compact-pad-x / -pad-y / -gap` 三维；默认 `--ui-density-row / -pad-x / -pad-y / -gap` 指向 `regular`。
- Done when: 3 档 × 4 维 = 12 个 density 变量存在；默认值指向 regular。
- Verify: `rg -n "ui-density-row" packages/ui/src/tokens.css` 命中 1。
- Stop if: 数值需要对齐既有 `UiListRow / UiDataTable` 行高，若冲突须先在这两个组件里 grep 验证。

Step 2:
- Action: `UiPageShell` 暴露 `density?: 'compact' | 'regular' | 'comfortable'` prop（默认 `regular`），将其映射成根节点 `data-density="..."`，在 tokens.css 里加三条 `[data-density="compact"] { --ui-density-row: var(--density-compact-row); ... }` 规则。
- Done when: 在任意 Storybook 或 demo 页把 `UiPageShell density="compact"` 后，子级使用 `--ui-density-row` 的组件行高变为 32px。
- Verify: 手动验证或新增 `packages/ui/src/components/__tests__/UiPageShell.density.test.ts`（vitest + testing-library）断言 DOM `data-density` 属性存在。
- Stop if: `UiPageShell` 不存在 prop 合并约定（目前它是 layout 级组件，直接改 prop 风险可控）。

### Task 4: motion preset 统一

Status: `pending`

Files:
- Modify: `packages/ui/src/lib/motion.ts`
- Create: `packages/ui/src/lib/motion.css`（如需）
- Modify: `packages/ui/src/index.ts`（导出）

Preconditions:
- 无。

Step 1:
- Action: 在 `motion.ts` 新增导出：`MOTION_DURATIONS = { fast: 120, normal: 160, slow: 220 } as const` 与 `MOTION_EASINGS = { apple: 'cubic-bezier(0.32,0.72,0,1)', inOut: 'cubic-bezier(0.4,0,0.2,1)' } as const`；再导出 `makeTransition(prop: string, duration?: keyof typeof MOTION_DURATIONS, easing?: keyof typeof MOTION_EASINGS): string` 生成 CSS transition 字符串。所有时长/缓动读自 tokens，禁止 overshoot/bounce。
- Done when: 新常量导出；Jsdoc 明确声明 "do not use spring overshoot; Calm Intelligence disallows bounce in workbench surfaces"。
- Verify: `pnpm -C apps/desktop typecheck` 通过；`rg -n "MOTION_DURATIONS" packages/ui/src/lib/motion.ts` 命中。
- Stop if: 发现 `@octopus/ui` 或 `apps/desktop` 里已有另一套 motion util（需 grep 确认），存在则合并不新建。

### Task 5: 清理 `bg-glass` 别名与 `backdrop-blur` 残留

Status: `pending`

Files:
- Modify: `packages/ui/src/tokens.css`
- Modify: `tailwind.config.js`
- Modify: 所有命中 `bg-glass` / `backdrop-blur` 的 `.vue / .ts / .css` 文件

Preconditions:
- Task 2 已完成（颜色别名不再是业务页 fallback）。

Step 1:
- Action: 先全仓库 grep `rg -n "bg-glass|backdrop-blur" apps packages` 取清单；按命中点逐一替换为 `bg-surface`（或 `bg-popover` 视语义）；然后在 `tokens.css` 移除 `--bg-glass` 行；在 `tailwind.config.js` 的 `colors` 映射里删除 `glass` 键。
- Done when: `rg -n "bg-glass|backdrop-blur|--bg-glass" apps packages` 零命中。
- Verify: `pnpm -C apps/desktop build:ui && pnpm check:frontend-governance` 通过。
- Stop if: 命中点超过 20 个（grep 显示目前在 `packages/ui/src` 无 backdrop-blur，但若 `apps/desktop` 有超过 20 处需分批迁移）。

### Task 6: 新增 `UiKbd` 共享组件

Status: `pending`

Files:
- Create: `packages/ui/src/components/UiKbd.vue`
- Modify: `packages/ui/src/index.ts`（导出 `UiKbd`）
- Modify: `AGENTS.md` 根文件的 Shared UI Component Catalog 列表（Base 类别追加 `UiKbd`）

Preconditions:
- Task 2、Task 4 已完成（消费 font-size-micro + motion preset）。

Step 1:
- Action: 实现 `UiKbd` —— 单个 `<kbd>` 元素，使用 `text-micro uppercase` 字号 token、4px radius (`rounded-xs`)、1px 边框走 `border-strong`、背景走 `bg-surface-muted`；阴影走 `shadow-xs`（不允许内联 inset rgba）。props: `keys: string[]`（如 `['⌘', 'K']`），自动用分隔符 `+` 渲染多键组合。支持 `size?: 'sm' | 'md'` 两档。
- Done when: `import { UiKbd } from '@octopus/ui'` 可用；单测覆盖多键渲染、单键渲染、无 keys 时返回空。
- Verify: `pnpm -C apps/desktop test -- UiKbd`（或在 packages/ui/tests/ 内跑）通过。
- Stop if: `packages/ui/src/index.ts` 的导出风格需要新增 barrel 分组，遵循已有模式。

### Task 7: `@octopus/ui` 内部一次性字号审计清零

Status: `pending`

Files:
- Modify: `packages/ui/src/components/*.vue`（按 grep 命中点逐个）

Preconditions:
- Task 2（字号 token）、Task 6（UiKbd）已完成。

Step 1:
- Action: 跑 `rg -n "text-\[1[13]px\]" packages/ui/src` 取清单；把命中的 `text-[11px]` 改为 `text-micro`（若原本 uppercase 语义）或 `text-caption`，`text-[13px]` 改为 `text-label`。每次替换一个文件，运行 typecheck 与快照测试验证无回归。
- Done when: `rg -n "text-\[1[0-3]px\]" packages/ui/src` 零命中。
- Verify: `pnpm -C apps/desktop test && pnpm -C apps/desktop typecheck` 通过。
- Stop if: 某处 `text-[13px]` 配合了非标准 line-height / letter-spacing，说明原写法可能是 mock 对齐，需与设计师确认。

### Task 8: 删除旧 `docs/plans/design/design.md` 与目录

Status: `pending`

Files:
- Delete: `docs/plans/design/design.md`
- Delete: `docs/plans/design/`（若为空目录）

Preconditions:
- 本计划 + P2/P3/P4/P5 五份计划均已落盘（由计划蓝图阶段保障）。

Step 1:
- Action: 删除文件与空目录，提交 commit message `plans: retire ui-vision brief, split into five executable plans`。
- Done when: `ls docs/plans/design 2>/dev/null` 返回空；git status 无 `docs/plans/design/` 条目。
- Verify: `git status` 无该路径；`rg -n "plans/design/design.md"` 零命中。
- Stop if: 其它文档/PR 链接到该路径（需一并更新引用）。

### Task 9: Frontend governance 脚本增补 token 约束

Status: `pending`

Files:
- Modify: `scripts/check-frontend-governance.mjs`

Preconditions:
- Task 2、Task 5 已完成。

Step 1:
- Action: 在脚本里新增正则黑名单扫描 `apps/desktop/src` 与 `packages/ui/src`：禁止 `text-\[\d+px\]`、`backdrop-blur`、`bg-glass`、`shadow-\[[^\]]+\]` 出现；命中即报错退出。
- Done when: 新增扫描段在 CI 输出里打印 `token governance ok` 或具体命中行号；`pnpm check:frontend-governance` 在当前仓库通过（若 apps/desktop 仍有债务，先把业务页命中纳入白名单文件 `scripts/frontend-governance-allowlist.txt` 并在 P5 计划中逐步清零）。
- Verify: `pnpm check:frontend-governance` 通过。
- Stop if: `apps/desktop` 命中量 > 100 无法一次清零，需切换成白名单策略并在 P5 跟进。

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 -> Task 2
- Completed: short list
- Verification:
  - `pnpm check:frontend-governance` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 1
```
