# 2026-04-21 · UI Tokens Alignment (Calm Intelligence 基座对齐)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

把当前仓库里还没收口的 token、字号、密度和交互动效基线补齐，让 `@octopus/ui` 与 `apps/desktop` 后续 UI 任务都基于同一套可验证的视觉基础设施执行。

## Architecture

当前视觉 token 的单一可信源仍是 `packages/ui/src/tokens.css`。`apps/desktop` 通过根级 `tailwind.config.js` 消费这些 token，`apps/desktop/tailwind.config.js` 只复用根配置并覆盖 `content`；动效基线集中在 `packages/ui/src/lib/motion.ts`，不要再引入第二套 UI motion helper。

## Scope

- In scope:
  - 补齐 `tokens.css` 的字号 scale 与 density token
  - 给根级 `tailwind.config.js` 增补字号映射
  - 扩建 `packages/ui/src/lib/motion.ts` 的共享 preset
  - 新增 `UiKbd` 共享组件
  - 清理仍保留的 `--bg-glass` / `glass` alias
  - 扩展 `scripts/check-frontend-governance.mjs` 的 token 违规扫描
  - 审计 `@octopus/ui` 里仍在使用的一次性字号类
- Out of scope:
  - 业务页的大面积视觉重构
  - 会话、Trace、搜索覆盖层的行为调整
  - 新增状态组件（`UiSkeleton` / `UiOfflineBanner` / `UiRestrictedState`）

## Risks Or Open Questions

- `docs/design/DESIGN.md` 里的 `Micro` 档已经存在。这里不能再把“补 Micro”写成待做任务，只能把后续 token 与组件消费补齐。
- `packages/ui/src/components/` 里仍有较多 `text-[10px]` / `text-[11px]` / `text-[13px]`。这项清债需要按真实命中文件推进，不能再写成模糊的 `*.vue`。
- `tailwind.config.js` 还保留了 `glass` alias，但当前仓库没有直接使用 `bg-glass` 的业务类名。清理时要同时删 token alias 与 Tailwind 映射，避免留半截状态。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not add a second token source outside `packages/ui/src/tokens.css`.
- When changing typography utilities, verify both `@octopus/ui` primitives and `apps/desktop` consumers against the same root Tailwind config.
- Use the current in-script governance debt model; do not invent a new external allowlist file unless the existing script model proves insufficient.

## Task Ledger

### Task 1: Confirm Typography Spec Baseline

Status: `done`

Files:
- Review: `docs/design/DESIGN.md`

Preconditions:
- None.

Step 1:
- Action: Confirm the current design standard already includes the `Micro` typography row and its usage rules.
- Done when: The plan no longer asks for a duplicate DESIGN update that has already landed in the refactored repo.
- Verify: `rg -n "^\| Micro" docs/design/DESIGN.md`
- Stop if: `DESIGN.md` no longer matches the root `AGENTS.md` typography contract.

### Task 2: Land Typography Tokens In `tokens.css` + Tailwind

Status: `done`

Files:
- Modify: `packages/ui/src/tokens.css`
- Modify: `tailwind.config.js`

Preconditions:
- Task 1 confirmed the design spec baseline.

Step 1:
- Action: Add CSS variables for the current DESIGN typography roles: page title, section title, card title, body, label, caption, badge, and micro. Each role must include font size, line height, and letter spacing variables so shared components stop hardcoding typography values.
- Done when: `tokens.css` exposes one named variable set per DESIGN role and no task in later plans needs to invent typography values locally.
- Verify: `rg -n "font-size-(page-title|section-title|card-title|body|label|caption|badge|micro)" packages/ui/src/tokens.css`
- Stop if: A token name collides with an already-shipped variable that means something different.

Step 2:
- Action: Extend `tailwind.config.js` with `fontSize` entries that map to the new CSS variables. `apps/desktop/tailwind.config.js` should continue inheriting from the root config without adding a second definition.
- Done when: `text-page-title`, `text-section-title`, `text-body`, and `text-micro` compile under the current desktop build.
- Verify: `pnpm -C apps/desktop build:ui && pnpm -C apps/desktop typecheck`
- Stop if: Tailwind cannot express the variable-backed line-height or letter-spacing shape cleanly.

### Task 3: Add Density Tokens + `UiPageShell` Density Prop

Status: `done`

Files:
- Modify: `packages/ui/src/tokens.css`
- Modify: `packages/ui/src/components/UiPageShell.vue`
- Modify: `docs/design/DESIGN.md`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- Task 2 completed.

Step 1:
- Action: Add `compact / regular / comfortable` density variables in `tokens.css` for row height, horizontal padding, vertical padding, and layout gap. Keep `regular` as the default.
- Done when: Density has named token primitives instead of implicit spacing baked into each page shell.
- Verify: `rg -n "density-(compact|regular|comfortable)" packages/ui/src/tokens.css`
- Stop if: The proposed density scale conflicts with current `UiListRow` / `UiDataTable` row behavior and needs a repo-wide rebaseline first.

Step 2:
- Action: Extend `UiPageShell` with `density?: 'compact' | 'regular' | 'comfortable'` and emit a stable `data-density` marker that shared components can consume from CSS variables.
- Done when: Page-level density becomes explicit at the shared shell boundary instead of page-local class churn.
- Verify: `pnpm -C apps/desktop test -- ui-primitives && pnpm -C apps/desktop typecheck`
- Stop if: `UiPageShell` consumers already rely on undocumented prop pass-through that would break with a new explicit prop.

### Task 4: Expand Shared Motion Presets

Status: `done`

Files:
- Modify: `packages/ui/src/lib/motion.ts`
- Modify: `packages/ui/src/index.ts`

Preconditions:
- None.

Step 1:
- Action: Extend `motion.ts` beyond `prefersReducedMotion()` to export named durations/easings and a helper for composing shared transition strings. Keep Calm Intelligence constraints explicit: no bounce, no overshoot, no second motion registry.
- Done when: Later UI tasks can reference shared motion constants instead of hardcoding durations in components.
- Verify: `pnpm -C apps/desktop typecheck && rg -n "MOTION_|makeTransition|prefersReducedMotion" packages/ui/src/lib/motion.ts`
- Stop if: A second motion helper already exists elsewhere in `packages/ui` or `apps/desktop` and ownership is unclear.

### Task 5: Remove Legacy `glass` Alias

Status: `done`

Files:
- Modify: `packages/ui/src/tokens.css`
- Modify: `tailwind.config.js`

Preconditions:
- Task 2 completed.

Step 1:
- Action: Remove `--bg-glass` from `tokens.css` and remove the `glass` color key from `tailwind.config.js`. Confirm no current desktop or shared UI files still depend on `bg-glass`.
- Done when: The refactored repo no longer advertises a fake glass token that only aliases `surface`.
- Verify: `rg -n "bg-glass|glass:|--bg-glass" apps/desktop/src packages/ui/src tailwind.config.js`
- Stop if: A current surface still uses the alias and needs a same-batch replacement.

### Task 6: Add Shared `UiKbd`

Status: `done`

Files:
- Create: `packages/ui/src/components/UiKbd.vue`
- Modify: `packages/ui/src/index.ts`
- Modify: `AGENTS.md`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- Task 2 completed.

Step 1:
- Action: Implement `UiKbd` as the shared keyboard hint primitive using the new `micro` typography role and current border/radius tokens. Export it from `@octopus/ui` and register it in the root shared component catalog.
- Done when: Keyboard hint rendering stops being ad-hoc text spans and later plans can consume a real shared primitive.
- Verify: `pnpm -C apps/desktop test -- ui-primitives && pnpm -C apps/desktop typecheck`
- Stop if: The current public export surface needs a larger barrel reorganization rather than a single additive export.

### Task 7: Audit Arbitrary Typography In Shared UI

Status: `done`

Files:
- Modify: `packages/ui/src/components/UiActionCard.vue`
- Modify: `packages/ui/src/components/UiArtifactBlock.vue`
- Modify: `packages/ui/src/components/UiBadge.vue`
- Modify: `packages/ui/src/components/UiDataTable.vue`
- Modify: `packages/ui/src/components/UiEmptyState.vue`
- Modify: `packages/ui/src/components/UiInboxBlock.vue`
- Modify: `packages/ui/src/components/UiListRow.vue`
- Modify: `packages/ui/src/components/UiNotificationBadge.vue`
- Modify: `packages/ui/src/components/UiNotificationRow.vue`
- Modify: `packages/ui/src/components/UiPageHeader.vue`
- Modify: `packages/ui/src/components/UiSelectionMenu.vue`
- Modify: `packages/ui/src/components/UiTraceBlock.vue`

Preconditions:
- Tasks 2 and 6 completed.

Step 1:
- Action: Replace current arbitrary typography utilities with the shared roles introduced in Task 2. Keep editor/mono surfaces such as `UiCodeEditor` out of this batch unless the text role is part of UI chrome instead of editor content.
- Done when: Shared UI chrome no longer relies on `text-[10px]`, `text-[11px]`, `text-[12px]`, or `text-[13px]` for roles already covered by the token scale.
- Verify: `rg -n "text-\\[[0-9]+px\\]" packages/ui/src/components && pnpm -C apps/desktop test -- ui-primitives && pnpm -C apps/desktop typecheck`
- Stop if: A hit belongs to content/editor text rather than UI chrome and needs a separate typography exception.

### Task 8: Expand Frontend Governance Token Checks

Status: `done`

Files:
- Modify: `scripts/check-frontend-governance.mjs`

Preconditions:
- Tasks 2 and 5 completed.

Step 1:
- Action: Extend the current governance script to flag token regressions that still matter in the refactored repo: arbitrary pixel text sizes in shared chrome, `backdrop-blur`, and any reintroduction of `glass` alias usage. Reuse the script's current in-memory allowlist/debt model instead of inventing a separate allowlist file.
- Done when: Token regressions fail the current governance check with repo-path output that points to the exact offender.
- Verify: `pnpm check:frontend-governance`
- Stop if: Existing business-surface debt is still too large to enforce the new rule set without first narrowing the scope to shared UI.

### Task 9: Close Audit Gaps In `UiKbd` And Shared Typography Debt

Status: `done`

Files:
- Modify: `packages/ui/src/components/UiKbd.vue`
- Modify: `packages/ui/src/components/UiAccordion.vue`
- Modify: `packages/ui/src/components/UiButton.vue`
- Modify: `packages/ui/src/components/UiCheckbox.vue`
- Modify: `packages/ui/src/components/UiCombobox.vue`
- Modify: `packages/ui/src/components/UiDialog.vue`
- Modify: `packages/ui/src/components/UiDonutChart.vue`
- Modify: `packages/ui/src/components/UiDropdownMenu.vue`
- Modify: `packages/ui/src/components/UiField.vue`
- Modify: `packages/ui/src/components/UiInput.vue`
- Modify: `packages/ui/src/components/UiInspectorPanel.vue`
- Modify: `packages/ui/src/components/UiMessageCenter.vue`
- Modify: `packages/ui/src/components/UiMetricCard.vue`
- Modify: `packages/ui/src/components/UiNavCardList.vue`
- Modify: `packages/ui/src/components/UiNotificationCenter.vue`
- Modify: `packages/ui/src/components/UiPageHero.vue`
- Modify: `packages/ui/src/components/UiRadioGroup.vue`
- Modify: `packages/ui/src/components/UiRecordCard.vue`
- Modify: `packages/ui/src/components/UiSectionHeading.vue`
- Modify: `packages/ui/src/components/UiSelect.vue`
- Modify: `packages/ui/src/components/UiStatTile.vue`
- Modify: `packages/ui/src/components/UiStatusCallout.vue`
- Modify: `packages/ui/src/components/UiSurface.vue`
- Modify: `packages/ui/src/components/UiSwitch.vue`
- Modify: `packages/ui/src/components/UiTabs.vue`
- Modify: `packages/ui/src/components/UiTextarea.vue`
- Modify: `packages/ui/src/components/UiToastItem.vue`
- Modify: `scripts/check-frontend-governance.mjs`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- Tasks 2, 6, 7, and 8 completed.

Step 1:
- Action: Re-open the audit closure for shared keyboard hints and shared UI typography debt. Make `UiKbd` accept the class binding shapes used by current desktop consumers, then replace the remaining shared UI arbitrary pixel text classes with current typography roles and shrink the governance allowlist down to true editor/content exceptions only.
- Done when: `UiKbd` no longer blocks current desktop `typecheck` / `build:ui`, and shared UI chrome no longer relies on allowlisted `10px`–`13px` arbitrary text sizes to pass governance.
- Verify: `pnpm -C apps/desktop typecheck && pnpm -C apps/desktop build:ui && pnpm check:frontend-governance && rg -n "text-\\[(10|11|12|13)px\\]" packages/ui/src/components`
- Stop if: A remaining hit belongs to editor content or data-visualization content where the current typography roles are not a semantic fit and needs an explicit governance exception.

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 2 Step 1 -> Task 4 Step 1
- Completed:
  - short list
- Verification:
  - `pnpm -C apps/desktop typecheck` -> pass
  - `pnpm check:frontend-governance` -> pass
- Blockers:
  - none
- Next:
  - Task 5 Step 1
```

## Checkpoint 2026-04-23 11:30 CST

- Batch: Task 2 Step 1 -> Task 2 Step 2
- Completed:
  - 在 `packages/ui/src/tokens.css` 增加 page title、section title、card title、body、label、caption、badge、micro 的字号 token
  - 在 `tailwind.config.js` 增加对应 `fontSize` 映射，统一走变量化的 `line-height` 和 `letter-spacing`
- Verification:
  - `rg -n "font-size-(page-title|section-title|card-title|body|label|caption|badge|micro)" packages/ui/src/tokens.css` -> pass
  - `pnpm -C apps/desktop build:ui` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-23 11:34 CST

- Batch: Task 3 Step 1 -> Task 3 Step 2
- Completed:
  - 在 `packages/ui/src/tokens.css` 增加 `compact / regular / comfortable` 密度 token
  - 给 `UiPageShell.vue` 增加 `density` prop 和稳定的 `data-density` 标记，并把页面 shell 间距改成 token 驱动
  - 在 `docs/design/DESIGN.md` 补上密度约束，并给 `ui-primitives` 增加 `UiPageShell` 密度覆盖
- Verification:
  - `rg -n "density-(compact|regular|comfortable)" packages/ui/src/tokens.css` -> pass
  - `pnpm -C apps/desktop test -- ui-primitives` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-23 11:47 CST

- Batch: Task 4 Step 1
- Completed:
  - 在 `packages/ui/src/lib/motion.ts` 增加共享动效常量 `MOTION_DURATIONS`、`MOTION_EASINGS` 和 `makeTransition`
  - 通过 `packages/ui/src/index.ts` 暴露共享动效导出，后续页面和组件可直接从 `@octopus/ui` 引用
- Verification:
  - `pnpm -C apps/desktop typecheck` -> pass
  - `rg -n "MOTION_|makeTransition|prefersReducedMotion" packages/ui/src/lib/motion.ts` -> pass
- Blockers:
  - none
- Next:
  - Task 5 Step 1

## Checkpoint 2026-04-23 11:54 CST

- Batch: Task 5 Step 1
- Completed:
  - 从 `packages/ui/src/tokens.css` 删除 `--bg-glass`
  - 从 `tailwind.config.js` 删除 `glass` 颜色映射
- Verification:
  - `rg -n "bg-glass|glass:|--bg-glass" apps/desktop/src packages/ui/src tailwind.config.js` -> pass
- Blockers:
  - none
- Next:
  - Task 6 Step 1

## Checkpoint 2026-04-23 12:11 CST

- Batch: Task 6 Step 1
- Completed:
  - 新增共享原语 `packages/ui/src/components/UiKbd.vue`
  - 通过 `packages/ui/src/index.ts` 暴露 `UiKbd`
  - 在根 `AGENTS.md` 的 Shared UI Component Catalog 里登记 `UiKbd`
  - 给 `apps/desktop/test/ui-primitives.test.ts` 增加 `UiKbd` 渲染覆盖
- Verification:
  - `pnpm -C apps/desktop test -- ui-primitives` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 7 Step 1

## Checkpoint 2026-04-23 12:16 CST

- Batch: Task 7 Step 1
- Completed:
  - 把 `UiActionCard`、`UiArtifactBlock`、`UiBadge`、`UiDataTable`、`UiEmptyState`、`UiInboxBlock`、`UiListRow`、`UiNotificationBadge`、`UiNotificationRow`、`UiPageHeader`、`UiSelectionMenu`、`UiTraceBlock` 的任意像素字号替换成共享 typography roles
  - 同批把这些组件里仍残留的 `14px / 22px / 30px` 标题字号一起收口到 `text-card-title`、`text-section-title`、`text-page-title`
  - 更新 `apps/desktop/test/ui-primitives.test.ts`，让 `UiPageHeader` 断言跟随新的 token 类名
- Verification:
  - `rg -n "text-\\[[0-9]+px\\]" packages/ui/src/components/{UiActionCard.vue,UiArtifactBlock.vue,UiBadge.vue,UiDataTable.vue,UiEmptyState.vue,UiInboxBlock.vue,UiListRow.vue,UiNotificationBadge.vue,UiNotificationRow.vue,UiPageHeader.vue,UiSelectionMenu.vue,UiTraceBlock.vue}` -> clear
  - `pnpm -C apps/desktop test -- ui-primitives` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 8 Step 1

## Checkpoint 2026-04-23 12:29 CST

- Batch: Task 8 Step 1
- Completed:
  - 扩展 `scripts/check-frontend-governance.mjs`，继续沿用现有 in-memory debt/allowlist 机制
  - 新增 shared UI 任意像素字号回归检测，并保留必要 allowlist
  - 新增 `glass` alias 回归检测，同时修正 `rounded-[var(--radius-2xl)]` 的允许规则
- Verification:
  - `pnpm check:frontend-governance` -> pass
- Blockers:
  - none
- Next:
  - `2026-04-21-ui-states-system.md` Task 1 Step 1

## Checkpoint 2026-04-23 17:50 CST

- Batch: Task 9 Step 1
- Completed:
  - 把 `packages/ui/src/components/UiKbd.vue` 的 `class` prop 收口到 `HTMLAttributes['class']`，让桌面端当前数组 class 绑定通过类型检查和构建
  - 把 shared UI arbitrary pixel text debt 收缩到真正例外 `packages/ui/src/components/UiCodeEditor.vue`
  - 收紧 `scripts/check-frontend-governance.mjs` 的 shared UI typography allowlist，只保留 editor/content 例外
- Verification:
  - `pnpm -C apps/desktop typecheck` -> pass
  - `pnpm -C apps/desktop build:ui` -> pass
  - `pnpm check:frontend-governance` -> pass
  - `rg -n "text-\[(10|11|12|13)px\]" packages/ui/src/components` -> only `UiCodeEditor.vue`
- Blockers:
  - none
- Next:
  - none
