# 2026-04-21 · UI AI Feedback (AI 协同流与等待焦虑)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

把 AI 协同反馈改回当前桌面端真实链路：会话页负责滚动与排队，消息气泡负责单条过程/工具反馈，项目任务页承接持久后台任务，Trace 页负责运行轨迹。不要再沿用旧项目里把这些能力塞进通知中心或新建顶栏任务菜单的前提。

## Architecture

`apps/desktop/src/views/project/ConversationView.vue` 已经拥有消息列表、排队区、编排 badge 和 mediation callout，是会话级 AI 反馈的主入口。`apps/desktop/src/components/conversation/ConversationMessageBubble.vue` 负责 `processEntries`、`toolCalls`、审批按钮和单条消息状态。持久后台任务已经有 `apps/desktop/src/views/project/ProjectTasksView.vue` 与 `useProjectTaskStore`，`ConversationQueueList.vue` 只处理当前会话内的排队 turn。`apps/desktop/src/views/project/TraceView.vue` 通过重复渲染 `UiTraceBlock` 展示 `runtime.activeTrace`。本计划只在这条现有链路上补齐反馈，不新造平行入口。

## Scope

- In scope:
  - 会话消息列表的滚动锚定与“回到最新”反馈
  - `ConversationMessageBubble.vue` 的过程、工具、等待状态可读性
  - 会话排队区与项目任务页之间的清晰分工和跳转入口
  - `TraceView.vue` / `UiTraceBlock.vue` 的当前 runtime 轨迹语义对齐
- Out of scope:
  - runtime session / trace 协议本身
  - adapter 的断流重试策略
  - 通知/收件箱中心的交互
  - 搜索覆盖层与全局快捷键

## Risks Or Open Questions

- `ConversationView.vue` 现在对 `renderedMessages` 做无条件滚底。改成锚定后，必须同时覆盖首屏载入、用户发送新消息、后台补历史三种路径。
- `ConversationMessageBubble.vue` 现在能拿到 `message.status`、`processEntries`、`toolCalls` 和 `approval`。如果后续要区分更细的恢复状态，只能基于这些当前字段，不能偷偷改成读 session 级全局状态。
- `ConversationQueueList.vue` 是瞬时队列，`ProjectTasksView.vue` 是持久任务。两者不是一层东西，不能再写成一个顶栏下拉同时兜底。
- `TraceView.vue` 现在把第一条 trace 的 `tone` 套给整条时间线。混合成功/警告/错误轨迹时会失真。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Keep AI feedback work on the current conversation, task, and trace surfaces; do not create a second shell entry unless the current surfaces prove insufficient.
- Prefer extending current message and trace primitives over introducing a new view-local state machine.
- Stop when the runtime source of truth cannot be proven from current message, task, or trace records.

## Task Ledger

### Task 1: Stabilize Conversation Scroll Anchoring

Status: `done`

Files:
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/test/conversation-surface.test.ts`

Preconditions:
- Confirm the message list still uses the current `scrollContainer` element and not a virtualized list.

Step 1:
- Action: Replace the current unconditional `watch(renderedMessages)` auto-scroll with an anchored strategy on `scrollContainer`: auto-stick only on first load, on user-submitted turns, and while the viewport remains within a bottom threshold; preserve viewport position when the user has scrolled away to read history.
- Done when: 新消息到达时，用户上翻历史不会被强行拉回底部；正常连贯对话仍会跟随到最新消息。
- Verify: `pnpm -C apps/desktop test -- conversation-surface`
- Stop if: Message rendering moves to virtualization, because the current scroll math will no longer be valid.

Step 2:
- Action: Add a local “回到最新” affordance inside `ConversationView.vue` when off-screen assistant output arrives. Keep it anchored above the composer/mediation area and reuse existing `UiButton` styling instead of inventing a new floating widget system.
- Done when: 用户离开底部时能明确知道有新输出，并能一键回到最新位置。
- Verify: `pnpm -C apps/desktop test -- conversation-surface`
- Stop if: The affordance overlaps the current composer or runtime mediation band and needs a shared layout primitive first.

### Task 2: Clarify Per-Message Process Feedback

Status: `done`

Files:
- Modify: `apps/desktop/src/components/conversation/ConversationMessageBubble.vue`
- Modify: `apps/desktop/test/conversation-surface.test.ts`

Preconditions:
- Current message records still expose `status`, `processEntries`, `toolCalls`, and `approval`.

Step 1:
- Action: Rework process-toggle copy, tool-call rows, and running-state labels around the current message fields so users can distinguish “正在思考 / 正在调用工具 / 等待批准 / 等待输入 / 已完成” from the bubble itself. Keep the focused-tool highlighting tied to the expanded process panel instead of introducing a second detail region.
- Done when: 单条 assistant 消息能直接说明自己是在运行、阻塞还是完成，且工具调用和过程条目之间的关系清楚。
- Verify: `pnpm -C apps/desktop test -- conversation-surface`
- Stop if: The current schema cannot distinguish approval wait and input wait at message level; block on a separate contract task instead of inferring from session-wide state.

### Task 3: Split Session Queue From Durable Background Tasks

Status: `done`

Files:
- Modify: `apps/desktop/src/views/project/ConversationView.vue`
- Modify: `apps/desktop/src/components/conversation/ConversationQueueList.vue`
- Modify: `apps/desktop/src/views/project/ProjectTasksView.vue`
- Modify: `apps/desktop/test/conversation-surface.test.ts`
- Modify: `apps/desktop/test/project-tasks-view.test.ts`

Preconditions:
- The `project-tasks` route remains the canonical task workbench for the current project.

Step 1:
- Action: Keep `ConversationQueueList.vue` focused on queued turns for the active session, then add an explicit jump from the conversation queue/orchestration area into `ProjectTasksView.vue` for durable runs, schedules, and interventions. Reuse current route helpers and terminology instead of inventing a new topbar task popover.
- Done when: 用户能在会话里分清“这条对话还在排队”和“这个项目有长期任务在跑”，并能从当前会话跳到任务工作台。
- Verify: `pnpm -C apps/desktop test -- conversation-surface project-tasks-view`
- Stop if: Product decides the current project task workbench is not the source of truth for runtime background work.

### Task 4: Align Trace Timeline With Current Runtime Trace Records

Status: `done`

Files:
- Modify: `apps/desktop/src/views/project/TraceView.vue`
- Modify: `packages/ui/src/components/UiTraceBlock.vue`
- Modify: `apps/desktop/test/trace-view.test.ts`
- Modify: `apps/desktop/test/ui-primitives.test.ts`

Preconditions:
- Current trace entries still expose item-level `tone`, `title`, `detail`, `actor`, and `timestamp`.

Step 1:
- Action: Stop deriving one `runtimeTraceTone` from `runtime.activeTrace[0]`. Pass each trace record’s own tone into `UiTraceBlock` so mixed timelines render success, warning, error, and info states per item.
- Done when: 同一条 runtime timeline 里的不同事件不再被第一条 trace 的 tone 误染。
- Verify: `pnpm -C apps/desktop test -- trace-view ui-primitives`
- Stop if: Runtime trace records stop carrying item-level tone and need a separate mapping layer first.

Step 2:
- Action: Extend `UiTraceBlock.vue` only where the current timeline needs richer metadata, such as a small status/meta row or action slot. Do not turn it into a nested step-state machine; `TraceView.vue` still owns list composition, and each block still represents one trace item.
- Done when: Trace 行可以承载当前 runtime 需要的附加信息，但组件边界仍保持清楚。
- Verify: `pnpm -C apps/desktop test -- trace-view ui-primitives`
- Stop if: Product actually needs grouped step trees from schema, which would require a different primitive than the current block component.

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 3 Step 1
- Completed:
  - short list
- Verification:
  - `pnpm -C apps/desktop test -- conversation-surface` -> pass
  - `pnpm -C apps/desktop test -- trace-view ui-primitives` -> pass
- Blockers:
  - none
- Next:
  - Task 4 Step 2
```

## Checkpoint 2026-04-23 13:03 CST

- Batch: Task 1 Step 1 -> Task 1 Step 2
- Completed:
  - 将 `ConversationView.vue` 的消息区滚动改成锚定策略，首屏、用户提交 turn、仍在底部三种场景才自动贴底
  - 为会话页补上本地 `回到最新` 按钮，用户离开底部且有新消息到达时可直接回到最新输出
  - 在 `conversation-surface` 测试里覆盖“历史阅读时不被强制拉回底部”“从历史位置提交新 turn 会重新贴底”“离屏新输出出现后展示回到最新入口”
- Verification:
  - `pnpm -C apps/desktop test -- conversation-surface` -> pass
- Blockers:
  - none
- Next:
  - Task 2 Step 1

## Checkpoint 2026-04-23 13:11 CST

- Batch: Task 2 Step 1
- Completed:
  - 重写 `ConversationMessageBubble.vue` 的过程切换文案，直接区分 Thinking、Using tools、Waiting for approval、Waiting for input、Completed
  - 调整工具调用行文案，让运行中、等待审批、等待输入和完成后的单条工具反馈各自可读
  - 为等待输入消息补上内联 callout，并在 `conversation-surface` 测试里覆盖等待审批、等待输入和完成态文案
- Verification:
  - `pnpm -C apps/desktop test -- conversation-surface` -> pass
  - `pnpm -C apps/desktop typecheck` -> pass
- Blockers:
  - none
- Next:
  - Task 3 Step 1

## Checkpoint 2026-04-23 13:20 CST

- Batch: Task 3 Step 1
- Completed:
  - 让 `ConversationQueueList.vue` 明确只描述当前会话内排队 turn，并补上标题与说明文案
  - 在 `ConversationView.vue` 增加到 `ProjectTasksView.vue` 的明确入口，区分会话排队与项目长期任务工作台
  - 在 `ProjectTasksView.vue` 增加从会话页进入时的说明与返回入口，并用测试覆盖会话到任务页再返回的链路
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/conversation-surface.test.ts test/project-tasks-view.test.ts` -> pass
- Blockers:
  - none
- Next:
  - Task 4 Step 1

## Checkpoint 2026-04-23 13:29 CST

- Batch: Task 4 Step 1 -> Task 4 Step 2
- Completed:
  - 移除 `TraceView.vue` 里用 `runtime.activeTrace[0]` 统一染色整条时间线的逻辑，改为每条 trace item 按自己的 `tone` 渲染
  - 为 `UiTraceBlock.vue` 增加轻量 `metaItems` 行，让单条 trace 可承载当前 runtime 的 kind 和关联工具名，而不引入嵌套状态机
  - 在 `trace-view` 与 `ui-primitives` 测试里补上混合 tone 和 trace 元信息覆盖
- Verification:
  - `pnpm -C apps/desktop exec vitest run test/trace-view.test.ts test/ui-primitives.test.ts` -> pass
- Blockers:
  - none
- Next:
  - `2026-04-21-ui-performance-a11y.md` Task 1 Step 1
