# 2026-04-21 · UI Performance & A11y (可度量稳定底线)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

把当前桌面端的“稳定流畅”变成可重复验证的基线：先补齐真实浏览器验证入口，再把性能、无障碍、减弱动效和对比度检查接进现有 `check:desktop` 体系，而不是继续假设旧项目里已经有现成的浏览器测试基础设施。

## Architecture

当前桌面前端的主验证入口仍是 `apps/desktop/test/**/*.test.ts` 下的 Vitest + jsdom。凡是需要真实浏览器布局、键盘、焦点、媒体查询或性能计时的检查，都必须先通过 browser-host 路径建立单独 harness，当前入口是根脚本 `pnpm dev:web` 对应的 `scripts/dev-web.mjs`。共享动效工具目前只有 `packages/ui/src/lib/motion.ts` 里的 `prefersReducedMotion()`，`UiDotLottie.vue` 和 `UiRiveCanvas.vue` 已经消费它，但仓库里还没有现成的浏览器测试配置、无障碍脚本或性能汇总脚本。

## Scope

- In scope:
  - 在当前仓库里建立 browser-host 端到端基线
  - 为 shell、搜索覆盖层、会话、Trace、任务页补齐真实浏览器无障碍检查
  - 建立减弱动效、键盘流和性能报告的首版脚本
  - 对当前真正存在的重型共享组件做按需初始化和渲染隔离
  - 为设计 token 增加对比度守卫脚本
- Out of scope:
  - Tauri webview 原生性能基准
  - Rust、数据库、网络层优化
  - 重新引入旧项目里的 mock 目录或平行浏览器宿主
  - 假设桌面端已经在用不存在的动画库或现成 bundle analyzer 体系

## Risks Or Open Questions

- 当前仓库没有浏览器端到端配置，也没有浏览器测试依赖。第一步必须先把 harness 搭起来，否则后面的性能和无障碍命令都无从执行。
- `pnpm dev:web` 是否能稳定产出带测试数据的浏览器宿主，需要用当前真实启动路径证明。不能回退到旧项目里那套页面内 mock 目录思路。
- `content-visibility` 和懒初始化只能打在真实收益明显、且不依赖精确测量/定位的组件上。不能按旧计划做 `*.vue` 式横扫。
- 新指标先做报告或软阈值，避免把第一轮不稳定采样直接变成阻塞。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Keep Vitest and browser-host checks separated by responsibility; do not overload `apps/desktop/test/**/*.test.ts` with browser-only assumptions.
- Add browser verification through the current `dev:web` path instead of inventing a second local host.
- Only optimize components that the current repo actually ships and that the new baseline can measure.

## Task Ledger

### Task 1: Establish Browser-Host E2E Baseline

Status: `pending`

Files:
- Modify: `package.json`
- Modify: `apps/desktop/package.json`
- Create: `apps/desktop/playwright.config.ts`
- Create: `apps/desktop/test/e2e/smoke.spec.ts`

Preconditions:
- `pnpm dev:web` must boot the current browser host without manual repo edits outside this task.

Step 1:
- Action: Add the browser E2E dependency/config and a first smoke spec that boots the current browser host, opens a known workspace/project route, and proves the shell, search trigger, and at least one project route can be exercised in a real browser.
- Done when: 仓库第一次拥有独立于 Vitest 的真实浏览器回归入口，并且它跑的是当前 browser-host 链路。
- Verify: `pnpm exec playwright test --config apps/desktop/playwright.config.ts apps/desktop/test/e2e/smoke.spec.ts`
- Stop if: Browser host cannot boot deterministic test data from the current startup path and needs a separate fixture contract first.

### Task 2: Add Core Browser A11y Checks On Current Surfaces

Status: `pending`

Files:
- Modify: `package.json`
- Create: `apps/desktop/test/e2e/a11y.spec.ts`
- Create: `apps/desktop/test/e2e/keyboard-navigation.spec.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/views/project/TraceView.vue`
- Modify: `apps/desktop/src/views/project/ProjectTasksView.vue`

Preconditions:
- Task 1 completed.

Step 1:
- Action: Add browser a11y checks for the current high-traffic routes and overlays: shell, search overlay, conversation, trace, and project tasks. Use a browser audit helper plus focused semantic assertions for keyboard order and focus retention; keep the dependency scoped to the browser test path instead of mixing it into shared UI runtime code.
- Done when: 这些核心页面能在真实浏览器里通过零阻塞级违规检查，并且有稳定的 `check:a11y` 命令入口。
- Verify: `pnpm check:a11y`
- Stop if: Portal timing or async route bootstrap makes the checks flaky enough that a shared stabilization helper must be introduced first.

### Task 3: Cover Reduced-Motion And Keyboard Flows

Status: `pending`

Files:
- Create: `apps/desktop/test/e2e/reduced-motion.spec.ts`
- Modify: `packages/ui/src/components/UiDialog.vue`
- Modify: `packages/ui/src/components/UiPopover.vue`
- Modify: `packages/ui/src/components/UiDotLottie.vue`
- Modify: `packages/ui/src/components/UiRiveCanvas.vue`

Preconditions:
- Task 1 completed.

Step 1:
- Action: Add reduced-motion coverage for dialog/popover/media flows and patch the current shared primitives where needed so `prefers-reduced-motion` disables non-essential animation without hiding state changes or breaking focus behavior.
- Done when: 减弱动效模式下，关键弹层和媒体组件仍可用，且不会继续播放非必要动画。
- Verify: `pnpm exec playwright test --config apps/desktop/playwright.config.ts apps/desktop/test/e2e/reduced-motion.spec.ts`
- Stop if: A component uses animation as the only state carrier and needs a text or static fallback design first.

### Task 4: Add Report-Only Browser Performance Baseline

Status: `pending`

Files:
- Modify: `package.json`
- Create: `apps/desktop/test/e2e/performance.spec.ts`
- Create: `scripts/check-frontend-performance.mjs`
- Modify: `apps/desktop/playwright.config.ts`

Preconditions:
- Task 1 completed.

Step 1:
- Action: Capture report-only browser metrics for current high-value flows such as app startup, search overlay open, conversation route ready, and trace route ready. Emit machine-readable output plus a human-readable summary, but keep thresholds soft until one stable baseline cycle has been observed.
- Done when: `check:frontend-performance` 能在当前仓库里稳定产出报告，而不是只剩下口头目标。
- Verify: `pnpm check:frontend-performance`
- Stop if: Metric variance is too high to compare across runs without retries or median aggregation.

### Task 5: Apply Targeted Lazy-Load And Render Containment

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiDotLottie.vue`
- Modify: `packages/ui/src/components/UiRiveCanvas.vue`
- Modify: `packages/ui/src/components/UiMetricCard.vue`
- Modify: `packages/ui/src/components/UiTimelineList.vue`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- Task 4 captured a first browser-host performance report.

Step 1:
- Action: Add lazy initialization to the current media primitives and apply `content-visibility` / containment only to the shared components that the new baseline proves are worth isolating. Keep the patch narrow and measurable.
- Done when: 共享组件的性能优化有明确基线对照，且没有引入布局、焦点或可访问性回归。
- Verify: `pnpm -C apps/desktop test -- ui-primitives && pnpm check:frontend-performance`
- Stop if: Containment breaks measurement, tooltip positioning, or viewport-dependent layout in the target component.

### Task 6: Add Token Contrast Guardrails

Status: `pending`

Files:
- Modify: `package.json`
- Create: `scripts/check-color-contrast.mjs`
- Modify: `packages/ui/src/tokens.css`

Preconditions:
- None.

Step 1:
- Action: Add a contrast-check script for the current token pairs that matter most in the desktop shell and shared UI: primary text vs background, secondary text vs surface, accent vs surface, warning/error text vs their soft surfaces. The script must fail with concrete token names and measured ratios.
- Done when: 对比度问题能在 token 层被提前发现，而不是等到页面回归里偶然暴露。
- Verify: `pnpm check:color-contrast`
- Stop if: Theme tokens are mid-migration and no stable token pair list can be agreed from the current source of truth.

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 -> Task 3
- Completed:
  - short list
- Verification:
  - `pnpm exec playwright test --config apps/desktop/playwright.config.ts apps/desktop/test/e2e/smoke.spec.ts` -> pass
  - `pnpm check:a11y` -> pass
- Blockers:
  - none
- Next:
  - Task 4
```
