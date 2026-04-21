# 2026-04-21 · UI Polish & Micro-interactions (精致化抛光)

> **For Codex:** REQUIRED SUB-SKILL: Use `superpowers:executing-plans` to implement this plan task-by-task.

## Goal

在前四份计划提供的稳定地基上做最后一层抛光 —— 引入统一的物理反馈（active scale）、快捷键呈现（`UiKbd` 体系化应用）、排版精致化（tabular-nums / tracking-tight / micro 标签）与全局命令面板 `⌘K`。保持 Calm Intelligence 的克制，**不允许**把任何微交互写到业务页里。

## Architecture

抛光全部下沉到 `@octopus/ui` 的 **基础组件内部**（`UiButton / UiActionCard / UiRecordCard / UiSwitch / UiDataTable` 等），业务页只享用不复写。`UiCommandPalette` 作为全局 overlay 由 `UiPageShell` 插槽挂载，命令注册走一个独立 store，命令产出者（各业务页）通过 composable 注册而不侵入 shell。

## Scope

- In scope:
  - `active:scale-[0.98]` 统一应用到可点击基础组件
  - `UiKbd` 全覆盖：`UiDropdownMenu` menu item、`UiTooltip`、Command Palette 行
  - `UiDataTable / UiMetricCard / UiStatTile` 的数字列 `tabular-nums`
  - `UiPageHero` 的 `tracking-tight`；`UiBadge` 与时间戳走 `text-micro`
  - `UiCommandPalette (⌘K)` + `useCommandRegistry` composable
  - Copy as Markdown / Copy as JSON / Copy Permalink 的统一 `UiContextMenu` 菜单项
- Out of scope:
  - 重写已有基础组件的行为
  - 业务侧命令的具体注册（本计划只提供骨架，注册清单由各业务 PR 逐步补齐）
  - 键盘快捷键的国际化文案（⌘ vs Ctrl 适配走默认）

## Risks Or Open Questions

- `active:scale-[0.98]` 在拖拽/长按场景下会与 Tauri 系统级手势冲突（可能出现 scale 黏住），需对 `draggable` 元素豁免。
- Command Palette 的命令权限边界：用户没有权限的命令不应出现在清单。需要在 `useCommandRegistry` 注册时携带 `visibleWhen` 谓词，并与 permission store 联动。
- `tabular-nums` 在中文字体栈下可能无效（中文字体不支持 OpenType features），需要确保数字列元素显式使用 `font-sans` 而非继承父级的中文字体。

## Execution Rules

- Do not start implementation until each task has exact files, acceptance, verification, and stop conditions.
- Do not collapse shared-layer work into a business-page-local workaround.
- Stop when contract ownership, source of truth, or verification output is unclear.
- Execute in small batches and update status in place after each batch.
- 每个 Task 完成后跑 `pnpm check:frontend-governance` 确保无一次性值回潮。

## Task Ledger

### Task 1: active scale 下沉到基础组件

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiButton.vue`
- Modify: `packages/ui/src/components/UiActionCard.vue`
- Modify: `packages/ui/src/components/UiRecordCard.vue`
- Modify: `packages/ui/src/components/UiSwitch.vue`
- Modify: `packages/ui/src/components/UiListRow.vue`

Preconditions:
- P1 已完成（motion preset）。

Step 1:
- Action: 在上述组件根节点添加 `transition-transform duration-fast active:scale-[0.98]`（其中 `duration-fast` 已被 tailwind extend 映射到 `var(--duration-fast)`）；对 `disabled` 状态与 `draggable="true"` 元素禁用该 class（用 `:class` 条件）。
- Done when: 鼠标按下有压感；键盘 enter 触发时也有（通过 `:active` pseudo）；disabled 状态无反馈。
- Verify: `pnpm -C apps/desktop test` 通过；手动在 5 个组件上点击验证。
- Stop if: `UiRecordCard` 内部已有自己的 transform 动画（如 hover lift），需合并策略（推荐丢掉 hover lift，只保 active scale）。

### Task 2: UiKbd 全覆盖

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiDropdownMenu.vue`
- Modify: `packages/ui/src/components/UiTooltip.vue`（若存在；否则沿用 Reka UI）
- Modify: `packages/ui/src/components/UiContextMenu.vue`

Preconditions:
- P1 Task 6（`UiKbd` 组件）已完成。

Step 1:
- Action: `UiDropdownMenu` 的 item 暴露 `shortcut?: string[]` prop；右对齐渲染 `UiKbd`；同理 `UiContextMenu`。`UiTooltip` 在内容后追加 `shortcut` 插槽位。
- Done when: 三组件渲染菜单项时右侧显示快捷键；无 shortcut 时空间不保留。
- Verify: `pnpm -C apps/desktop test` 通过；在 Workbench 顶栏任一菜单手动验证。
- Stop if: Reka UI 的 DropdownMenu item 不允许右侧额外内容（需改用 flex 自定义 item 结构）。

### Task 3: 数字列 tabular-nums

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiDataTable.vue`
- Modify: `packages/ui/src/components/UiMetricCard.vue`
- Modify: `packages/ui/src/components/UiStatTile.vue`
- Modify: `packages/ui/src/components/UiRankingList.vue`

Preconditions:
- P1 已完成。

Step 1:
- Action: 为 `UiDataTable` 列 schema 加 `align?: 'left' | 'right' | 'center'` 与 `isNumeric?: boolean`；`isNumeric === true` 时单元格加 `font-sans tabular-nums text-right`（强制 sans 防止中文字体回退）。`UiMetricCard / UiStatTile / UiRankingList` 的数值区同样应用 `font-sans tabular-nums`。
- Done when: 数字列对齐整齐；不同位数不错位。
- Verify: `pnpm -C apps/desktop test` 通过；截图回归。
- Stop if: 现 `UiDataTable` 列 schema 已有冲突字段名，需合并。

### Task 4: 排版精致化（Micro + tracking）

Status: `pending`

Files:
- Modify: `packages/ui/src/components/UiPageHero.vue`
- Modify: `packages/ui/src/components/UiBadge.vue`
- Modify: `packages/ui/src/lib/formatDateTime.ts`（或相关时间戳组件）
- Modify: `packages/ui/src/components/UiNotificationRow.vue`（时间戳）

Preconditions:
- P1 Task 1（DESIGN.md Micro 档）、Task 2（字号 token）已完成。

Step 1:
- Action: `UiPageHero` 的标题加 `tracking-tight`（与 page-title 30px/700 搭配，符合 DESIGN.md §5.5）；`UiBadge` 与所有时间戳元素统一切到 `text-micro uppercase tracking-wider`（Micro 档）。
- Done when: 页标题读起来更有张力；标签/时间戳视觉一致。
- Verify: `pnpm -C apps/desktop test && pnpm -C apps/desktop build:ui` 通过。
- Stop if: 业务页已有自写 tracking，需 grep 清理（属于 P1 Task 7 遗留，不应在此处修）。

### Task 5: UiCommandPalette 骨架 + 注册机制

Status: `pending`

Files:
- Create: `packages/ui/src/components/UiCommandPalette.vue`
- Create: `packages/ui/src/lib/useCommandRegistry.ts`
- Create: `packages/ui/src/components/__tests__/UiCommandPalette.test.ts`
- Modify: `packages/ui/src/index.ts`
- Modify: `apps/desktop/src/App.vue`（挂载）
- Modify: `apps/desktop/src/composables/registerGlobalCommands.ts`（注册内置命令：打开会话、打开 Agents、打开设置、打开权限中心等）

Preconditions:
- P1 Task 6（`UiKbd`）已完成。
- P2 Task 1（`UiSkeleton`）已完成（用于搜索结果 loading）。

Step 1:
- Action: 实现 `useCommandRegistry`：提供 `registerCommand(cmd)` 与 `listCommands()`；Command 类型 `{ id, title, description?, keywords?, icon?, shortcut?, group?, visibleWhen?(): boolean, run(): Promise<void> | void }`。
- Done when: registry 是单例；命令可注册/注销；`visibleWhen` 生效。
- Verify: `pnpm -C apps/desktop test -- useCommandRegistry` 通过。
- Stop if: 与现有任何全局 store 命名冲突（需 grep）。

Step 2:
- Action: `UiCommandPalette` 基于 `UiDialog`（不是毛玻璃！实体 surface + level-3 shadow），输入框 + 分组结果；结果行 icon + title + description + `UiKbd` shortcut + group 标签；`⌘K` 全局快捷键触发；`Esc` 关闭；上下键 + Enter 操作。结果 loading 用 `UiSkeleton`。
- Done when: 全局 `⌘K` 能唤起；搜索到的命令 enter 后 run；权限不足命令不出现。
- Verify: `pnpm -C apps/desktop test -- UiCommandPalette && pnpm -C apps/desktop build:ui` 通过。
- Stop if: Reka UI 的 Dialog portal 与键盘事件有竞态；按 a11y 要求处理 focus trap。

Step 3:
- Action: `registerGlobalCommands.ts` 注册 ≥ 8 个内置命令：打开 Dashboard、打开 Agents、打开 Tools、打开 Settings、打开权限中心、切换主题、搜索会话、反馈问题。
- Done when: `⌘K` 输入关键词能命中；命令描述清晰。
- Verify: 手动在本地跑一轮。
- Stop if: 某些目标页路由还未就位，先注册后置 TODO 而非 block 本任务。

### Task 6: Copy as X 统一上下文菜单

Status: `pending`

Files:
- Create: `packages/ui/src/lib/useCopyActions.ts`
- Modify: `packages/ui/src/components/UiContextMenu.vue`（或提供 builder）
- Modify: 业务页相关上下文菜单的调用方

Preconditions:
- Task 2 已完成（UiContextMenu 挂 UiKbd）。

Step 1:
- Action: `useCopyActions` 提供 `copyAsMarkdown(payload)` / `copyAsJSON(payload)` / `copyPermalink(routeLocation)` 三个 helper；统一通过 `navigator.clipboard.writeText` 写入，失败回退到 Tauri adapter。提供一个 `buildCopyMenuItems(payload)` 直接返回一组标准菜单项配置。
- Done when: 三 helper 可被多处复用；失败态有 toast 反馈。
- Verify: `pnpm -C apps/desktop test -- useCopyActions` 通过。
- Stop if: Tauri adapter 未暴露 clipboard fallback API，本任务可先只做浏览器分支并 TODO。

## Batch Checkpoint Format

```md
## Checkpoint YYYY-MM-DD HH:MM

- Batch: Task 1 -> Task 2
- Completed: short list
- Verification:
  - `pnpm -C apps/desktop test` -> pass
  - `pnpm check:frontend-governance` -> pass
- Blockers:
  - none
- Next:
  - Task 3
```
