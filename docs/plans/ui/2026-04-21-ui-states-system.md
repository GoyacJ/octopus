# 2026-04-21 · UI States System (商业化四态统一化)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

把商业化桌面产品最核心的四种状态 —— `loading / error / offline / restricted` —— 统一到 `@octopus/ui` 共享层，让所有业务页异步与异常场景共享一套视觉与行为契约，消除"每页各自造轮子"的不稳定感。

## Architecture

四态均以共享组件形式落在 `packages/ui/src/components/`；错误边界与网络探测走 Vue `onErrorCaptured` 与 `navigator.onLine` + 自定义探针，不引入新 runtime。现存 `apps/desktop/src/components/layout/AppRuntimeErrorBoundary.vue` 将上提为 `@octopus/ui` 的 `UiErrorBoundary`，业务页通过 slot 组合；网络状态走一个新 Pinia store `useConnectionStore`（locally-first，只做状态管理，不持久化）。

## Scope

- In scope:
  - 新增 `UiSkeleton` / `UiErrorState` / `UiErrorBoundary` / `UiOfflineBanner` / `UiRestrictedState`
  - 上提 `AppRuntimeErrorBoundary.vue` 逻辑到 `@octopus/ui`
  - `apps/desktop` 业务页的 loading 态迁移（列表、详情、设置页）
  - 全局 ErrorBoundary 挂载到路由根
  - 顶栏级 `UiOfflineBanner` 接入 `useConnectionStore`
- Out of scope:
  - 后端错误码契约调整（由 OpenAPI governance 负责）
  - 重试/退避策略（由 adapter 层负责，本计划只做 UI）
  - Toast 化错误（错误仍然靠 Toast 出短信息，但页级失败态走 UiErrorState）
  - Agent 流式中断 UI（归属 P3）

## Risks Or Open Questions

- `AppRuntimeErrorBoundary.vue` 上提到 `@octopus/ui` 后，原 `apps/desktop` 引用路径会变；需确认该组件不依赖 desktop-only API（若有对 Tauri 的直接调用，须在 P1 adapter 层封装）。
- `useConnectionStore` 的"离线"定义：纯 `navigator.onLine` 不够（Tauri 环境里可能始终 true），必须对 workspace host ping 超时也视为离线；ping 端点与频率须由 host 侧确认。
- `UiRestrictedState` 三种变体（权限不足 / 试用到期 / 只读）的具体文案与 CTA，需要与产品确认。本计划仅交付**视觉与插槽契约**，文案由产品填入。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a business-page-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.
- 每个新组件必须有至少 3 个 vitest 用例：渲染、插槽、`prefers-reduced-motion` 分支。

## Task Ledger

### Task 1: UiSkeleton 共享组件

Status: `pending`

Files:
- Create: `packages/ui/src/components/UiSkeleton.vue`
- Create: `packages/ui/src/components/__tests__/UiSkeleton.test.ts`
- Modify: `packages/ui/src/index.ts`

Preconditions:
- P1 Task 2（字号 token）、Task 4（motion preset）已完成。

Step 1:
- Action: 实现三形态：`<UiSkeleton variant="line" :lines="3" />`、`variant="card"`、`variant="table" :rows="6" :cols="4"`；shimmer 用 CSS `@keyframes` + `background-size: 200% 100%`；在 `prefers-reduced-motion: reduce` 下只保留静态底色，不动画。
- Done when: 三 variant 渲染正常；`prefers-reduced-motion` 下 `animation: none`；组件只用 token（颜色走 `bg-surface-muted`，radius 走 `rounded-m`）。
- Verify: `pnpm -C apps/desktop test -- UiSkeleton` 通过。
- Stop if: shimmer 在 Safari 下掉帧（需改为 transform 动画）。

### Task 2: UiErrorState + UiErrorBoundary

Status: `pending`

Files:
- Create: `packages/ui/src/components/UiErrorState.vue`
- Create: `packages/ui/src/components/UiErrorBoundary.vue`
- Create: `packages/ui/src/components/__tests__/UiErrorState.test.ts`
- Create: `packages/ui/src/components/__tests__/UiErrorBoundary.test.ts`
- Modify: `packages/ui/src/index.ts`
- Modify: `apps/desktop/src/components/layout/AppRuntimeErrorBoundary.vue`（改为 re-export 或薄包装）

Preconditions:
- P1 已完成（token 与 motion）。

Step 1:
- Action: `UiErrorState` 是可视化展示组件，props：`title`、`description`、`errorId?`、`onRetry?`、`onCopy?`、`onFeedback?`；默认插画走 `UiDotLottie`（进入视口才加载），可关闭；视觉使用 `border border-whisper bg-surface rounded-l padding-8`。禁止 toast 化，它是**页级**失败态。
- Done when: 带 retry/copy/feedback 三按钮渲染成功；插画可被 `:show-illustration="false"` 关闭；单测覆盖三回调触发。
- Verify: `pnpm -C apps/desktop test -- UiErrorState` 通过。
- Stop if: 现 `@octopus/ui` 已有 `UiStatusCallout` 覆盖此语义——需先读该组件判断是否复用/扩展而非新建。

Step 2:
- Action: `UiErrorBoundary` 用 Vue `onErrorCaptured` 捕获子树异常，捕获后渲染 `UiErrorState` 并提供 `reset()` 方法；暴露 `#error="{ error, reset }"` 作用域插槽以便业务页自定义。把 `AppRuntimeErrorBoundary.vue` 改为调用 `UiErrorBoundary` 的薄壳，保留 desktop-only 的上报逻辑（如 Tauri 崩溃日志写入）在外层 wrapper 里。
- Done when: 路由根挂 `UiErrorBoundary`；人工在开发环境抛 `throw new Error('x')` 能触发 fallback；`reset()` 能恢复；原 `AppRuntimeErrorBoundary` 引用处无变化。
- Verify: `pnpm -C apps/desktop test && pnpm -C apps/desktop build:ui` 通过。
- Stop if: 原 `AppRuntimeErrorBoundary` 依赖非共享的 Tauri 调用；需要在 adapter 层先封装。

### Task 3: useConnectionStore + UiOfflineBanner

Status: `pending`

Files:
- Create: `apps/desktop/src/stores/connection.ts`（Pinia store）
- Create: `packages/ui/src/components/UiOfflineBanner.vue`
- Create: `packages/ui/src/components/__tests__/UiOfflineBanner.test.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `packages/ui/src/index.ts`

Preconditions:
- P1 已完成。
- host ping 端点已确认（默认先用 `GET /api/v1/health` + 5s 超时）。

Step 1:
- Action: `useConnectionStore` 暴露 `status: 'online' | 'degraded' | 'offline'`；`online` = `navigator.onLine === true && 最近一次 ping < 5s`；`degraded` = `navigator.onLine === true && ping 超时 1~2 次`；`offline` = `navigator.onLine === false || ping 连续超时 ≥ 3 次`。Ping 走 `apps/desktop/src/tauri/shell.ts` 的 `health()`（不存在则先在 adapter 层补）。
- Done when: 手动断网后 store 切到 `offline`，恢复后 3 秒内切回 `online`。
- Verify: `pnpm -C apps/desktop test -- connection.store` 覆盖 3 个状态切换。
- Stop if: `shell.ts` 无 `health()`，需要先在 P1/adapter 层里补并对齐 OpenAPI 契约（不属于本计划范围，需开新任务）。

Step 2:
- Action: `UiOfflineBanner` 是**顶栏级**条带（非 toast），`status === 'online'` 时不渲染；`degraded` 显示橙色提示（warning soft）；`offline` 显示 danger soft + 重试按钮。高度 28px，位于 topbar 下缘，占满宽度；`WorkbenchTopbar` 里 slot 或直接组合。
- Done when: 三状态视觉符合 DESIGN.md 语义色；离线时 topbar 下方出现条带不压缩主内容布局。
- Verify: `pnpm -C apps/desktop test -- UiOfflineBanner && pnpm -C apps/desktop build:ui` 通过。
- Stop if: topbar 高度与条带叠加会破坏既有布局（需联动 `--topbar-height` token）。

### Task 4: UiRestrictedState 三变体

Status: `pending`

Files:
- Create: `packages/ui/src/components/UiRestrictedState.vue`
- Create: `packages/ui/src/components/__tests__/UiRestrictedState.test.ts`
- Modify: `packages/ui/src/index.ts`

Preconditions:
- P1 已完成。
- 产品已提供三变体的默认文案或确认由插槽注入。

Step 1:
- Action: 实现 `UiRestrictedState` 支持三 variant：`permission` / `trial-expired` / `read-only`；视觉语言统一为"居中小卡片 + icon + 标题 + 描述 + 主 CTA"，但不同 variant 用不同 status color soft（permission → neutral，trial-expired → warning soft，read-only → info soft）。
- Done when: 三 variant 截图对齐；主 CTA 可通过 `action` 插槽替换。
- Verify: `pnpm -C apps/desktop test -- UiRestrictedState` 通过。
- Stop if: 产品侧 CTA 文案/跳转链接未定，暂缺省留插槽。

### Task 5: 业务页 Skeleton 迁移

Status: `pending`

Files:
- Modify: `apps/desktop/src/views/**/*.vue`（按 grep 命中点）

Preconditions:
- Task 1 已完成。

Step 1:
- Action: 跑 `rg -n "animate-pulse|role=\"status\"" apps/desktop/src/views` 取自制骨架清单；逐页迁移到 `UiSkeleton`。不允许用裸 spinner 作为列表/详情页的 loading 态（spinner 只保留给"按钮提交中"这种短停态）。
- Done when: `rg -n "animate-pulse" apps/desktop/src/views` 零命中。
- Verify: `pnpm -C apps/desktop test && pnpm -C apps/desktop typecheck` 通过；人工在三个代表性页面（agents 列表、tools 列表、settings）切慢网下看骨架态。
- Stop if: 某页骨架结构与 `UiSkeleton` 三 variant 都不匹配，需要第四 variant（回到 Task 1 增补，不要在业务页 hack）。

### Task 6: 全局 ErrorBoundary 挂载

Status: `pending`

Files:
- Modify: `apps/desktop/src/App.vue`（或路由根组件）

Preconditions:
- Task 2 已完成。

Step 1:
- Action: 在路由 `<router-view>` 外层包一层 `UiErrorBoundary`；fallback 提供 `reset()` + `重新加载页面` 两个动作；`errorId` 用 `crypto.randomUUID()` 生成并写入 `logs/`（或走 adapter 上报）。
- Done when: 路由级未捕获异常不再白屏；错误 ID 可被用户复制。
- Verify: 手动在开发环境抛异常触发；`pnpm -C apps/desktop build:ui` 通过。
- Stop if: Vue `onErrorCaptured` 在路由切换竞态下不触发，需要额外挂 `window.onerror`。

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 -> Task 2 Step 1
- Completed: short list
- Verification:
  - `pnpm -C apps/desktop test` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 2 Step 2
```
