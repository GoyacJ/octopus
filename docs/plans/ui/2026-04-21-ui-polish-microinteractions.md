# 2026-04-21 · UI Polish & Micro-interactions (精致化抛光)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

在前几份基础计划落地后，给当前桌面端补最后一层克制抛光：统一按压反馈、快捷键展示、数字排版和搜索覆盖层细节。不要再沿用旧项目里“再造一个全局命令面板”的方向，当前仓库已经有自己的 `Cmd/Ctrl+K` 搜索入口。

## Architecture

共享抛光仍优先下沉到 `@octopus/ui` 的基础组件里。全局 `Cmd/Ctrl+K` 已经由 `apps/desktop/src/layouts/WorkbenchLayout.vue` 监听、由 `useShellStore.searchOpen` 持有状态、由 `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue` 渲染，`WorkbenchTopbar.vue` 负责当前触发器。快捷键展示要围绕这条已有链路补齐，而不是再建一个平行 overlay 或注册中心。

## Scope

- In scope:
  - 基础交互组件的统一按压反馈
  - `UiKbd` 在当前菜单、搜索覆盖层和顶栏触发器里的落地
  - 现有搜索覆盖层的结果态、空态、快捷键提示精修
  - 当前共享 chrome 的 `tabular-nums`、`tracking-tight`、`micro` 排版收口
  - 会话与 Trace 场景的统一复制上下文菜单
- Out of scope:
  - 新建平行命令面板或命令注册层
  - 新增一个并不存在的 tooltip 基础组件
  - 业务页自定义动效体系
  - 路由、权限或搜索数据源的重新设计

## Risks Or Open Questions

- `active:scale-[0.98]` 只能加在没有自带 transform 动画冲突的基础组件上，否则会出现 hover/press 叠加。
- 搜索覆盖层当前是“搜索结果”模型，不是“命令对象”模型。只要现有结果结构还能覆盖需求，就不该为了抛光去抽新抽象。
- `tabular-nums` 在中文字体链路上可能失效。数字列如果要稳定对齐，需要显式回到支持 OpenType 的数字字体栈。
- 顶栏和覆盖层里的快捷键提示必须兼容 macOS 与非 macOS 标示，但不应把平台判断散落到业务页。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Keep micro-interactions in shared components or current shell surfaces; do not leak bespoke behavior into business pages.
- Reuse the current search overlay as the global keyboard entry. Do not introduce a second global search/command state source.
- After each task, run `pnpm check:frontend-governance` so token and utility cleanup does not regress.

## Task Ledger

### Task 1: Add Restrained Press Feedback To Shared Primitives

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiButton.vue`
- Modify: `packages/ui/src/components/UiActionCard.vue`
- Modify: `packages/ui/src/components/UiRecordCard.vue`
- Modify: `packages/ui/src/components/UiSwitch.vue`
- Modify: `packages/ui/src/components/UiListRow.vue`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- The token plan’s motion baseline is available, or the components can reuse the current transition utilities without adding one-off timings.

Step 1:
- Action: Add a restrained active press state to the listed shared primitives. Disable it for `disabled` and drag-driven states, and avoid any component where an existing transform animation would conflict.
- Done when: 当前共享交互组件按下时有一致但克制的物理反馈，且没有 hover/drag 卡住的问题。
- Verify: `pnpm -C apps/desktop test -- ui-primitives && pnpm check:frontend-governance`
- Stop if: A target component already owns a transform-based motion contract that cannot be merged cleanly.

### Task 2: Roll `UiKbd` Into Current Menus And Search Surfaces

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiDropdownMenu.vue`
- Modify: `packages/ui/src/components/UiContextMenu.vue`
- Modify: `packages/ui/src/components/UiSelectionMenu.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `apps/desktop/test/ui-primitives.test.ts`
- Modify: `apps/desktop/test/search-overlay.test.ts`

Preconditions:
- The token plan’s `UiKbd` task completed.

Step 1:
- Action: Replace raw shortcut text spans with `UiKbd` in current menu rows, search overlay footer hints, and the topbar search trigger. Keep the shortcut API additive to the current menu item shape instead of rewriting menu contracts.
- Done when: 快捷键提示在菜单、覆盖层和顶栏触发器里有统一样式，不再靠零散文本类名拼出来。
- Verify: `pnpm -C apps/desktop test -- ui-primitives search-overlay && pnpm check:frontend-governance`
- Stop if: The current menu primitives cannot carry trailing shortcut content without a shared slot contract first.

### Task 3: Polish The Existing Global Search Overlay

Status: `pending`

Files:
- Modify: `apps/desktop/src/layouts/WorkbenchLayout.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchSearchOverlay.vue`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`
- Modify: `apps/desktop/src/stores/shell.ts`
- Modify: `apps/desktop/test/search-overlay.test.ts`
- Modify: `apps/desktop/test/layout-shell.test.ts`

Preconditions:
- The current `searchOpen` state remains the single source of truth for the overlay.

Step 1:
- Action: Refine the current overlay’s open/close polish, active-result framing, empty-state copy, and shortcut hints while keeping `WorkbenchLayout.vue` + `useShellStore` as the only global entry. Any new behavior must live on top of the current search overlay, not beside it.
- Done when: 现在的 `Cmd/Ctrl+K` 入口本身就承载了这份抛光计划，不再需要一份平行的命令面板计划。
- Verify: `pnpm -C apps/desktop test -- search-overlay layout-shell && pnpm check:frontend-governance`
- Stop if: Product now needs non-search global actions that the current result model cannot express without a separate contract.

### Task 4: Tighten Shared Typography On Current Chrome

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiPageHeader.vue`
- Modify: `packages/ui/src/components/UiBadge.vue`
- Modify: `packages/ui/src/components/UiNotificationRow.vue`
- Modify: `apps/desktop/src/components/conversation/ConversationMessageBubble.vue`
- Modify: `apps/desktop/test/ui-primitives.test.ts`
- Modify: `apps/desktop/test/conversation-surface.test.ts`

Preconditions:
- The token plan’s typography roles landed.

Step 1:
- Action: Move header/badge/meta/timestamp styling onto the new shared typography roles: tighter heading tracking where appropriate, `text-micro` for badge and timestamp metadata, and `tabular-nums` for visible time or numeric labels that should align consistently.
- Done when: 共享 chrome 的标签、时间戳、页头和消息元信息读起来是一套语言，不再混用旧像素字号。
- Verify: `pnpm -C apps/desktop test -- ui-primitives conversation-surface && pnpm check:frontend-governance`
- Stop if: The token plan has not landed the required text roles yet.

### Task 5: Standardize Copy Actions On Current AI Surfaces

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiContextMenu.vue`
- Modify: `apps/desktop/src/components/conversation/ConversationMessageBubble.vue`
- Modify: `apps/desktop/src/views/project/TraceView.vue`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/trace-view.test.ts`

Preconditions:
- Task 2 completed.

Step 1:
- Action: Add one consistent copy-action set for current AI surfaces through the existing context-menu primitive: copy message content, copy trace detail, and copy current route/permalink where it already exists in the view model. Keep clipboard behavior aligned across browser and desktop hosts.
- Done when: 会话和 Trace 场景的复制能力不再是零散按钮或浏览器默认选区，而是同一套上下文菜单语义。
- Verify: `pnpm -C apps/desktop test -- conversation-surface trace-view && pnpm check:frontend-governance`
- Stop if: Clipboard handling diverges between browser host and Tauri host enough that an adapter-level helper must be defined first.

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 -> Task 3
- Completed:
  - short list
- Verification:
  - `pnpm -C apps/desktop test -- ui-primitives search-overlay` -> pass
  - `pnpm check:frontend-governance` -> pass
- Blockers:
  - none
- Next:
  - Task 4
```
