# 2026-04-21 · UI States System (商业化四态统一化)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

把当前桌面端最常见的 `loading / error / connection / restricted` 四类状态整理成当前仓库可执行的共享方案，避免继续沿用旧项目里已经失效的错误边界、离线探测和 loading 占位思路。

## Architecture

通用展示组件继续落在 `@octopus/ui`。但当前全局运行时错误捕获已经明确由 `apps/desktop/src/App.vue`、`apps/desktop/src/runtime/app-error-boundary.ts` 和 `apps/desktop/src/components/layout/AppRuntimeErrorBoundary.vue` 负责，不能再把整套运行时边界错误地上提成 `@octopus/ui`；连接状态也要建立在现有 `useShellStore` 的 `backendConnection` 与 `workspaceConnections` 之上，而不是再造一份平行连接 store。

## Scope

- In scope:
  - 新增共享 `UiSkeleton`
  - 新增共享 `UiErrorState`，并让当前 `AppRuntimeErrorBoundary.vue` 复用它
  - 基于 `useShellStore` 的连接状态提示条带
  - 新增共享 `UiRestrictedState`
  - 把当前若干真实 loading 文案/占位面迁移到 `UiSkeleton`
- Out of scope:
  - 改动 runtime 错误采集与恢复协议
  - 重新设计 host 启动失败的整页 guard
  - 新增独立连接状态 store
  - Agent 流式中断与 Trace 细节反馈

## Risks Or Open Questions

- 当前运行时错误边界依赖路由跳转、复制详情和 runtime reset token。这里只能抽共享展示层，不能把整套恢复逻辑硬搬进 `@octopus/ui`。
- 当前 shell 的连接状态语义是 `connected / disconnected / unreachable`，而不是旧计划里的 `online / degraded / offline`。连接提示必须基于这套真实枚举。
- 当前仓库没有广泛的 `animate-pulse` 债务。loading 迁移要围绕真实存在的 loading 面，而不是沿用旧项目的 grep 规则。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Keep runtime crash capture desktop-owned unless a sub-piece is proven transport-agnostic.
- Reuse `useShellStore` for connection truth; do not introduce a second source of truth for connectivity.
- Prefer targeted loading migrations on current high-traffic surfaces over repo-wide speculative cleanup.

## Task Ledger

### Task 1: Add Shared `UiSkeleton`

Status: `done`

Files:
- Create: `packages/ui/src/components/UiSkeleton.vue`
- Modify: `packages/ui/src/index.ts`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- Typography and motion baselines from the token plan are available or can be consumed from current tokens without introducing ad-hoc values.

Step 1:
- Action: Add a shared skeleton primitive that covers the current desktop needs: line blocks, card blocks, and table-row blocks. Respect `prefers-reduced-motion` through the existing `packages/ui/src/lib/motion.ts` helper path.
- Done when: Shared loading placeholders no longer require each desktop surface to invent its own copy-only loading treatment.
- Verify: `pnpm -C apps/desktop test -- ui-primitives && pnpm -C apps/desktop typecheck`
- Stop if: The current token layer cannot represent the skeleton colors or radii without adding one-off values.

### Task 2: Extract Shared Error Presentation, Keep Desktop Runtime Boundary

Status: `done`

Files:
- Create: `packages/ui/src/components/UiErrorState.vue`
- Modify: `packages/ui/src/index.ts`
- Modify: `apps/desktop/src/components/layout/AppRuntimeErrorBoundary.vue`
- Modify: `apps/desktop/tsconfig.json`
- Modify: `apps/desktop/test/app-runtime-error-boundary.test.ts`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- Current ownership of runtime crash capture remains in `App.vue` + `runtime/app-error-boundary.ts`.

Step 1:
- Action: Extract the reusable visual structure of the runtime failure page into `UiErrorState`, then refit `AppRuntimeErrorBoundary.vue` to consume that shared presentation while preserving the current desktop-only recovery actions and diagnostic copy flow.
- Done when: The app still renders the same runtime failure experience, but the visual error shell becomes reusable for non-runtime page failures.
- Verify: `pnpm -C apps/desktop test -- app-runtime-error-boundary ui-primitives && pnpm -C apps/desktop build:ui`
- Stop if: The shared component would need direct router/runtime awareness instead of staying presentation-only.

### Task 3: Surface Connection Problems From `useShellStore`

Status: `done`

Files:
- Create: `packages/ui/src/components/UiOfflineBanner.vue`
- Modify: `packages/ui/src/index.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `apps/desktop/test/layout-shell.test.ts`
- Modify: `apps/desktop/test/shell-store.test.ts`

Preconditions:
- Current shell connection semantics are confirmed from `apps/desktop/src/stores/shell.ts`.

Step 1:
- Action: Render a topbar-adjacent connection banner driven by the existing `useShellStore` records. Hide on `connected`, show warning on `disconnected`, show danger + retry affordance on `unreachable`. Keep the full-screen host-unavailable guard in `App.vue` as the owner of catastrophic backend boot failure.
- Done when: Users can distinguish workspace/session connectivity issues from total host startup failure without a second store or duplicated health polling logic.
- Verify: `pnpm -C apps/desktop test -- layout-shell shell-store && pnpm -C apps/desktop build:ui`
- Stop if: `unreachable` always resolves to the full-page guard and leaves no topbar state worth surfacing.

### Task 4: Add Shared `UiRestrictedState`

Status: `done`

Files:
- Create: `packages/ui/src/components/UiRestrictedState.vue`
- Modify: `packages/ui/src/index.ts`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- None.

Step 1:
- Action: Create a shared restricted-state primitive that covers the current desktop cases without hardcoding one product story: permission denied, read-only, and upgrade/entitlement gating. Keep actions slot-driven.
- Done when: Desktop pages gain one shared restricted-state contract instead of continuing to improvise per-page permission empty states.
- Verify: `pnpm -C apps/desktop test -- ui-primitives && pnpm -C apps/desktop typecheck`
- Stop if: Product needs materially different copy hierarchies per restriction type and rejects a single shared presentation contract.

### Task 5: Migrate Real Loading Surfaces To `UiSkeleton`

Status: `done`

Files:
- Modify: `apps/desktop/src/components/conversation/ArtifactVersionList.vue`
- Modify: `apps/desktop/src/components/layout/ConversationContextPane.vue`
- Modify: `apps/desktop/src/views/project/ProjectDeliverablesView.vue`
- Modify: `apps/desktop/src/views/project/ProjectModelsPanel.vue`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/project-deliverables-view.test.ts`

Preconditions:
- Task 1 completed.

Step 1:
- Action: Replace current loading copy/placeholder treatment on the listed high-traffic surfaces with `UiSkeleton` where the user is waiting for data blocks rather than for a button action to finish.
- Done when: Deliverable/version/context loading states use one shared skeleton language and stop reading like temporary fallback copy.
- Verify: `pnpm -C apps/desktop test -- conversation-surface project-deliverables-view && pnpm -C apps/desktop typecheck`
- Stop if: A listed surface actually needs a semantic empty/error state instead of a loading skeleton.

### Task 6: Record Current Global Runtime Boundary Baseline

Status: `done`

Files:
- Review: `apps/desktop/src/App.vue`
- Review: `apps/desktop/src/runtime/app-error-boundary.ts`
- Review: `apps/desktop/src/components/layout/AppRuntimeErrorBoundary.vue`
- Review: `apps/desktop/test/app-runtime-error-boundary.test.ts`

Preconditions:
- None.

Step 1:
- Action: Preserve the fact that the current repo already mounts a global runtime error boundary and tests it. Future work should extend this baseline, not recreate the mount point from scratch.
- Done when: This plan no longer treats global runtime error capture as missing.
- Verify: `pnpm -C apps/desktop test -- app-runtime-error-boundary`
- Stop if: The current runtime boundary mount has been removed in the meantime.

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 3 Step 1
- Completed:
  - short list
- Verification:
  - `pnpm -C apps/desktop test -- ui-primitives` -> pass
  - `pnpm -C apps/desktop test -- app-runtime-error-boundary` -> pass
- Blockers:
  - none
- Next:
  - Task 5 Step 1
```

## Checkpoint 2026-04-23 12:23 CST

- Batch: Task 1 Step 1
- Completed:
  - 新增共享 `UiSkeleton`，覆盖 line、card、table-row 三种骨架形态
  - 通过 `packages/ui/src/index.ts` 暴露 `UiSkeleton`
  - 在 `apps/desktop/test/ui-primitives.test.ts` 增加三种骨架变体和 reduced motion 断言
- Verification:
  - `pnpm -C apps/desktop test -- ui-primitives` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-23 12:29 CST

- Batch: Task 2 Step 1
- Completed:
  - 新增共享展示壳 `UiErrorState`，统一 intro、summary、actions、details 四段结构
  - 让 `AppRuntimeErrorBoundary.vue` 复用共享错误展示层，同时保留 desktop 侧的 retry、复制详情和路由恢复逻辑
  - 补齐 `apps/desktop/tsconfig.json` 的 `@octopus/ui` 路径映射，使 worktree 下的 typecheck/build 与本地源码解析保持一致
  - 在 `app-runtime-error-boundary` 和 `ui-primitives` 测试里覆盖新的结构分区与共享组件渲染
- Verification:
  - `pnpm -C apps/desktop test -- app-runtime-error-boundary ui-primitives` -> pass
  - `pnpm -C apps/desktop build:ui` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-23 12:43 CST

- Batch: Task 3 Step 1
- Completed:
  - 新增共享 `UiOfflineBanner`，统一 topbar 连接告警条的 warning 与 danger 呈现
  - 让 `WorkbenchTopbar.vue` 基于 `useShellStore().activeWorkspaceConnection` 渲染断连与不可达提示，并在 `unreachable` 时复用现有 `refreshBackendStatus()` 重试
  - 在 `layout-shell` 与 `shell-store` 测试里覆盖断连提示、不可达重试和 loopback 健康检查状态同步
- Verification:
  - `pnpm -C apps/desktop test -- layout-shell shell-store` -> pass
  - `pnpm -C apps/desktop build:ui` -> pass
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-23 12:45 CST

- Batch: Task 4 Step 1
- Completed:
  - 新增共享 `UiRestrictedState`，提供 `neutral / warning / accent` 三种受限状态语气和通用 intro/body/actions 布局
  - 通过 `packages/ui/src/index.ts` 暴露 `UiRestrictedState`
  - 在 `ui-primitives` 测试里覆盖 tone、meta、正文和 actions 插槽结构
- Verification:
  - `pnpm -C apps/desktop test -- ui-primitives` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 5 Step 1

## Checkpoint 2026-04-23 13:07 CST

- Batch: Task 5 Step 1
- Completed:
  - 将会话上下文、交付物列表、模型面板和产物版本列表的加载态统一迁移到共享 `UiSkeleton`
  - 在 `conversation-surface` 与 `project-deliverables-view` 测试里覆盖交付物列表骨架和相关高频加载面
  - 补齐 `UiSkeleton` 在真实桌面高频加载路径里的落地，结束 `UI States System` 计划剩余执行项
- Verification:
  - `pnpm -C apps/desktop test -- conversation-surface project-deliverables-view` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - `2026-04-21-ui-ai-feedback.md` Task 2 Step 1
