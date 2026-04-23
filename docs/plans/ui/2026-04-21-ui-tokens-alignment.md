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

Status: `pending`

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

Status: `pending`

Files:
- Modify: `packages/ui/src/tokens.css`
- Modify: `packages/ui/src/components/UiPageShell.vue`
- Modify: `docs/design/DESIGN.md`

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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

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

Status: `pending`

Files:
- Modify: `scripts/check-frontend-governance.mjs`

Preconditions:
- Tasks 2 and 5 completed.

Step 1:
- Action: Extend the current governance script to flag token regressions that still matter in the refactored repo: arbitrary pixel text sizes in shared chrome, `backdrop-blur`, and any reintroduction of `glass` alias usage. Reuse the script's current in-memory allowlist/debt model instead of inventing a separate allowlist file.
- Done when: Token regressions fail the current governance check with repo-path output that points to the exact offender.
- Verify: `pnpm check:frontend-governance`
- Stop if: Existing business-surface debt is still too large to enforce the new rule set without first narrowing the scope to shared UI.

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
