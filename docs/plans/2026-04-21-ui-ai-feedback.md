# 2026-04-21 · UI AI Feedback (AI 协同流与等待焦虑)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

在保持 Calm Intelligence 克制视觉的前提下，解决 AI Agent 产品在会话与任务流中最核心的两类痛点 —— "等待焦虑" 与 "上下文迷失"；让用户**看见**模型思考过程、**掌控**滚动与任务生命周期、**理解**流式中断与恢复语义。

## Architecture

所有 AI 反馈改造落在 `@octopus/ui` 既有组件 `UiMessageCenter` / `UiTraceBlock` / `UiEmptyState` 上，禁止业务页另起炉灶。流式中断/后台化 UI 只消费 runtime session 契约（由 `apps/desktop/src/tauri/workspace-client.ts` 暴露），不自行实现状态机；媒体资产（Rive/Lottie）走 P4 的 IntersectionObserver 按需加载策略。

## Scope

- In scope:
  - `UiMessageCenter` 智能滚动锚定（stick-to-bottom + 用户上滚时新消息胶囊）
  - `UiTraceBlock` 分阶段展示（thinking / tool-call / retrieval / done 四状态）
  - `UiEmptyState` 零状态 CTA（onboarding 与 empty 两场景，禁止 dense 页滥用插画）
  - 流式"断流重连中 / 已中断可重试"两态区分的视觉契约
  - Agent 任务后台化的 UI 入口（顶栏 Tasks icon + 下拉清单）
- Out of scope:
  - runtime session 协议本身（归属 crates/runtime）
  - 流式中断的重试策略（adapter 层负责）
  - Rive/Lottie 按需加载机制实现（归属 P4）
  - Command Palette (`⌘K`)（归属 P5）

## Risks Or Open Questions

- `UiMessageCenter` 滚动锚定需要对 `ResizeObserver` + `scrollTop` 时机敏感，不同操作系统滚动惯性有差异。必须在 macOS trackpad / Windows mouse wheel / Linux 三环境手动验证。
- Agent 任务后台化要求任务状态持久化到 runtime 层（已由 SQLite projection + 事件日志支持）；UI 只需消费。**但顶栏 Tasks 下拉的入口契约尚未定义**，需要先对齐 runtime session 契约，否则 UI 实现无处可挂。本计划先只做 UI 壳，契约确认前不接数据。
- `UiTraceBlock` 现行渲染结构未知，需先 Read 现有实现再决定是扩展还是重写；若重写会破坏既有会话快照，需提供迁移策略。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a business-page-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.
- Motion 一律使用 P1 产出的 `MOTION_DURATIONS / MOTION_EASINGS`；禁止 overshoot。

## Task Ledger

### Task 1: UiMessageCenter 智能滚动锚定

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiMessageCenter.vue`
- Create: `packages/ui/src/components/__tests__/UiMessageCenter.scroll.test.ts`
- Modify: `packages/ui/src/index.ts`（若需导出新类型）

Preconditions:
- P1 已完成（motion preset 可用）。
- 已 Read `UiMessageCenter.vue` 当前实现，确认滚动容器层次。

Step 1:
- Action: 在 `UiMessageCenter` 容器上监听 `scroll` 事件，维护 `isAtBottom: boolean`（阈值 `scrollHeight - scrollTop - clientHeight < 80px`）；当新消息追加且 `isAtBottom === true` 时，用 `requestAnimationFrame` 平滑滚到底；否则不滚。
- Done when: 手动上滚查看历史时，新消息追加不强拉视线。
- Verify: 新增 vitest 用例模拟 scroll + mutation，断言 `scrollTop` 在 `isAtBottom=false` 时保持；`pnpm -C apps/desktop test -- UiMessageCenter.scroll` 通过。
- Stop if: 容器使用了 `UiVirtualList`（当前未知），虚拟滚动下阈值计算需用另一套坐标。

Step 2:
- Action: 当 `isAtBottom === false` 且有新消息时，在容器底部居中显示一个浮动胶囊 `⬇ 有 N 条新消息`（`rounded-full bg-surface border-whisper shadow-sm`，单行高 32px），点击回到底部并淡出。进入动画 120ms opacity+translate-y-4，符合 DESIGN.md §9。
- Done when: 胶囊出现/隐藏符合预期；点击后平滑滚动到底；不遮挡 composer。
- Verify: 手动 + 单测覆盖渲染条件；`pnpm -C apps/desktop test -- UiMessageCenter.scroll` 通过。
- Stop if: composer 高度变化时胶囊位置会错位（需订阅 composer 高度变化 ResizeObserver）。

### Task 2: UiTraceBlock 分阶段状态

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiTraceBlock.vue`
- Modify: `packages/ui/src/components/__tests__/UiTraceBlock.test.ts`
- Modify: `packages/schema/src/` 下 Trace 相关 schema（若需扩展 step 状态字段）

Preconditions:
- 已 Read 当前 `UiTraceBlock.vue` 实现。
- Trace schema 是否已有 step.status 字段已确认。

Step 1:
- Action: Trace 的每一步（tool call / retrieval / model thinking）渲染状态图标：
  - `pending`：灰色空心圆（静态）
  - `running`：`UiDotLottie` 微型转圈（16×16，进入视口才加载）
  - `success`：accent 填色对勾
  - `error`：danger 填色叉
  每步有可点开的 details 折叠面板（沿用现有行为），标题区显示步骤名 + 耗时 + 状态。
- Done when: 4 状态视觉符合 DESIGN.md 语义色；每状态切换伴随 160ms 过渡；单测覆盖 4 状态渲染。
- Verify: `pnpm -C apps/desktop test -- UiTraceBlock` 通过。
- Stop if: schema 里 step 没有 `durationMs` 字段；必须先在 schema 层补齐再改 UI。

Step 2:
- Action: 当整块 Trace 仍在流式更新时，最后一个 step 上方渲染一个"当前正在..."的 `aria-live="polite"` 文本，供屏幕阅读器播报。
- Done when: a11y 测试（axe）无违规；键盘用户可用 Tab 进入 step 折叠面板。
- Verify: `pnpm -C apps/desktop test -- UiTraceBlock.a11y`。
- Stop if: 现 Trace 不暴露"stream 是否结束"标志位，需在 schema 层补。

### Task 3: 流式中断两态 UI 契约

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiMessageCenter.vue`
- Create: `packages/ui/src/components/UiStreamStatus.vue`
- Create: `packages/ui/src/components/__tests__/UiStreamStatus.test.ts`
- Modify: `packages/ui/src/index.ts`

Preconditions:
- runtime adapter 已在消息流里暴露 `streamStatus: 'streaming' | 'reconnecting' | 'interrupted' | 'done'`（若无，需先开任务补，本计划不承担）。

Step 1:
- Action: `UiStreamStatus` 以消息尾部行内组件呈现：
  - `streaming` 不显示（沉默）
  - `reconnecting` 显示"正在重新连接... N/5"，warning soft 胶囊
  - `interrupted` 显示"已中断" + "重试" 按钮（可选 "保留已生成内容"），danger soft 胶囊
  - `done` 不显示
  状态切换符合 DESIGN.md §9（不 overshoot）。
- Done when: 4 状态视觉与行为符合；重试按钮触发外部回调。
- Verify: `pnpm -C apps/desktop test -- UiStreamStatus` 通过。
- Stop if: adapter 未暴露该字段，本任务阻塞直到 runtime 侧就位。

### Task 4: UiEmptyState 零状态 CTA 强化

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiEmptyState.vue`
- Modify: `packages/ui/src/components/__tests__/UiEmptyState.test.ts`

Preconditions:
- P1 已完成。
- 产品已明确"哪些页面允许有插画"（默认：onboarding、会话新建、personal center；禁止 dense workbench）。

Step 1:
- Action: 在 `UiEmptyState` 增加 `illustration?: 'lottie' | 'rive' | 'none'` 与 `ctaPrimary? / ctaSecondary?` 插槽；默认 `illustration="none"`，强制业务方显式 opt-in；`ctaPrimary` 使用 `UiButton variant="primary"`。
- Done when: 无插画的 workbench 空态保持克制；有插画的 onboarding 空态符合 DESIGN.md §8.7。
- Verify: `pnpm -C apps/desktop test -- UiEmptyState` 通过。
- Stop if: 现 `UiEmptyState` 已有某种插画 default 且业务页大量依赖；需 grep 迁移。

### Task 5: Agent 任务后台化 UI 入口

Status: `pending`

Files:
- Create: `packages/ui/src/components/UiBackgroundTasksMenu.vue`
- Create: `packages/ui/src/components/__tests__/UiBackgroundTasksMenu.test.ts`
- Modify: `apps/desktop/src/components/layout/WorkbenchTopbar.vue`

Preconditions:
- runtime session 契约暴露"活跃任务清单"（若无，本任务先只做 UI 壳 + mock）。

Step 1:
- Action: topbar 右侧新增 Tasks 图标（`lucide-vue-next` 的 `ListChecks`），未读/运行中时显示 accent 小圆点；点击打开下拉 `UiBackgroundTasksMenu`（基于 `UiDropdownMenu`），清单每行 `任务名 + 进度/状态 + 关联会话跳转`。
- Done when: 下拉可达且键盘可操作；任务切换不打断正在执行的任务；关闭 dialog 后再打开，任务仍在清单中。
- Verify: `pnpm -C apps/desktop test -- UiBackgroundTasksMenu && pnpm -C apps/desktop build:ui` 通过。
- Stop if: runtime 侧无"活跃任务"查询 API，本任务仅交付 UI 壳并 block 在契约对齐。

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 Step 1 -> Task 2 Step 1
- Completed: short list
- Verification:
  - `pnpm -C apps/desktop test` -> pass
- Blockers:
  - Task 5 waiting on runtime session contract
- Next:
  - Task 2 Step 2
```
