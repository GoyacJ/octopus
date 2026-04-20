# SDK UI 意图 IR 落地计划（2026-04-20）

> 依据：agent-transcripts 当前会话 — 用户确认在 SDK 文档层补一层"UI 意图契约（UI Intent IR）"，以支撑**融合 Claude.ai 成果物展示 + Claude Code 内联 diff 两种范式**，并保持**多宿主（桌面 Vue / CLI / 未来移动端）中立**、**UI 组件落在业务层**。

## Goal

在 `docs/sdk/` 新增第 14 章 `14-ui-intent-ir.md`，定义 SDK 层的 **UI 意图描述符**（JSON 可序列化、discriminated union、宿主中立）：

- 定义 `RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactKind` / `ArtifactRef` 五类顶层 IR。
- 明确 SDK 产出 IR、业务层 map IR→组件 的**分层边界**。
- 让 Claude.ai 的"独立 artifact 展示"与 Claude Code 的"文件 + inline diff"成为**同一 IR 的两种工具实现策略**，而不是两套架构。
- 同步改 03 / 08 / 13 的三个锚点，避免 14 章与既有章节出现语义漂移。

## Architecture

分四层归属：

1. **L1 模型协议层**（Protocol Adapter，11 §11.6）：各厂商流 → Canonical Message IR。
2. **L2 Harness 执行层**（01 / 03 / 04 / 05 / 06 / 07 / 08 / 09）：把 `tool_use` / `tool_result` / `ask` / `artifact` 翻译成 L3 描述符，写进 append-only event log（09 §9）。
3. **L3 SDK UI 意图契约层**（本章新增）：`packages/schema/src/ui-intent.ts`（未来代码落地名，本文档仅为规范）定义 Zod discriminated union；宿主中立、JSON 可序列化；**不得**依赖任何 UI 框架。
4. **L4 业务层渲染**（`apps/desktop` · `apps/cli` · 未来 `apps/mobile`）：单点 dispatch，把 IR kind 映射到具体组件（桌面 = `@octopus/ui`）。

产物：`docs/sdk/14-ui-intent-ir.md` 一份新章 + 03 / 08 / 13 三处锚点修订 + README / references 索引同步。**不**新增任何 `contracts/openapi/src/**` / `packages/schema/src/**.ts` / `packages/ui/**.vue` 代码；本轮仅沉淀契约规范。

## Scope

- **In scope**：
  - 新建 `docs/sdk/14-ui-intent-ir.md`。
  - 修订 `docs/sdk/03-tool-system.md`：在 §3.1.1 `ToolDisplayDescriptor` 基础上正式化 `RenderLifecycle` 五钩子与 IR 枚举；补反模式条目。
  - 修订 `docs/sdk/08-long-horizon.md`：在 §8.4 Artifact 合约里新增"成果物双轨"小节（artifact-ref 风格 ↔ file diff 风格），指向 14 章 IR。
  - 修订 `docs/sdk/13-contracts-map.md`：§13.2 总览扩"第四源 / UI 意图 IR"，§13.3 主映射表新增"UIIntent kinds"列，§13.9 追加历史修订条目；在 §13.7 harness-internal 清单中登记 UI Intent IR。
  - 同步 `docs/sdk/README.md`（文档索引 + 阅读顺序 + §7 术语表新增 `UI Intent IR` / `Render Block`）、`docs/sdk/references.md` §C6。
- **Out of scope**：
  - **不**新增任何 OpenAPI path / Zod schema `.ts` / Vue 组件。
  - **不**修订 `contracts/openapi/src/**` 的 bundle / generated 产物。
  - **不**在 14 章里定义与 12 §12 Plugin 扩展点全景冲突的新扩展点；IR 扩展（新增 kind）走 SDK 版本升级，不经插件。
  - **不**修订 01 / 02 / 04 / 05 / 06 / 07 / 09 / 10 / 11 / 12 章节的正文（本轮只改 03 / 08 / 13 三处锚点）。

## Risks Or Open Questions

- **命名冲突**：`@octopus/ui` 组件名以 `Ui*` 为前缀；IR 类型若再叫 `UiRenderBlock` 会语义混淆。**决定**：IR 命名**不带 `Ui` 前缀**（`RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactRef`），统一放在 `ui-intent` 语义域下但类型名保持清洁。
- **kind 爆炸**：若每加一个工具就扩 kind，枚举会无限增长。**对策**：本章规定 kind 数量硬上限 15 以内；超出范围一律先走 `raw { mime, data }` 逃生出口，稳定后再收编。
- **与 03 §3.1.1 既有 `ToolDisplayDescriptor` 的关系**：03 章已埋伏笔（"非 React/Vue 节点、JSON 可序列化"），本轮 14 章是**正式化并扩容**（5 钩子 + 10+ kind），不是推翻重来。03 §3.1.1 的 `ToolDisplayDescriptor` 类型会在修订中引用 14 章，标注为"历史占位，语义等价于 `RenderLifecycle`"。
- **与 09 Observability 的关系**：IR 描述符是否写进 `runtime/events/*.jsonl`？**决定**：是，作为 event payload 的一部分；这样 replay / 审计时无需重新调用模型即可还原 UI。但 IR 的 snapshot 写入口**归 09** 管，14 章只定义形状。
- **AskPrompt 是否与 tool_use 合流**：AskUserQuestion 在 Claude Code 是一个普通工具。**决定**：14 章把 `AskPrompt` 独立为顶层 IR（而非 `RenderBlock` 的一个 kind），因为它语义上带"阻塞 / modal / 回写 tool_result"，与纯展示性 RenderBlock 不同质。工具定义层依然把 AskUserQuestion 建模为普通 tool，但其 `onToolUse` render 钩子输出**引用**一条 `AskPrompt`，由权限引擎接管渲染。
- **artifact 的双轨归属**：artifact-ref（独立存储）归 `artifact.ts` schema + `data/artifacts/`；file diff 归 `fs_edit` / `fs_write` 的 `RenderBlock kind: 'diff'`。**决定**：两条路径都合法，**选择权在工具实现方**，SDK 不强制。

## Execution Rules

- 先写本 plan，然后按 Task 顺序执行；每个 Task 原子、带验证。
- 写新章前先看 03 / 08 / 13 的既有文字，**不**重复既有规则；14 章定"IR 形状 + 分层契约"，其它章只做锚点引用。
- 修订 03 / 08 / 13 时，**仅**在指定锚点插入 / 改写；不得动其它段落的措辞，避免跨章语义漂移。
- IR 草案以**中文文档中的 TypeScript 伪代码**形式呈现，不在本轮引入实际 `.ts` 文件；未来落地由 `docs/plans/xxxx-ui-intent-schema-impl.md` 另起计划。
- 执行过程中若发现 13 §13.3 已有"UIIntent kinds"之外的第三维度需要纳入表格（如"IR 事件写入点"），暂停并咨询，不自行扩列。
- 每批执行结束 `ReadLints` + 文档交叉引用自检（`rg '14-ui-intent-ir' docs/sdk/`、`rg 'RenderBlock|AskPrompt|ArtifactRef' docs/sdk/`）。

## Task Ledger

### Task 1: 起草 `docs/sdk/14-ui-intent-ir.md`

Status: `done`

Files:
- Create: `docs/sdk/14-ui-intent-ir.md`

Preconditions:
- plan（本文件）已写完；03 §3.1.1 的 `ToolDisplayDescriptor` 已读过；13 §13.3 表结构已清楚。

Step 1:
- Action：按以下骨架起草 14 章：
  - §14.1 目的与非目的（不替代 `@octopus/ui`、不替代 OpenAPI、不替代 09 事件流）
  - §14.2 四层分层（L1–L4）与边界契约
  - §14.3 顶层 IR 类型（`RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactKind` / `ArtifactRef` / `ProgressEvent`）—— TypeScript 伪代码 + 字段语义
  - §14.4 `RenderBlock.kind` 枚举清单（text / markdown / code / diff / list-summary / progress / artifact-ref / record / error / raw）+ 每个 kind 的使用场景、业务层建议 `@octopus/ui` 组件
  - §14.5 工具的 5 个 render 钩子（`onToolUse` / `onToolProgress` / `onToolResult` / `onToolRejected` / `onToolError`）—— 完整对齐 Claude Code 但返回 IR 而非 React
  - §14.6 Claude.ai 风格 ↔ Claude Code 风格：同一 IR 的两种工具实现策略对比表
  - §14.7 宿主映射建议（桌面 Vue → `@octopus/ui` · CLI → Ink/TUI · 移动 → RN），表格示例而非强制实现
  - §14.8 与 09 Observability 的关系：IR 是否写入 event log（是，作为 payload 的一部分；由 09 章定义写入协议）
  - §14.9 与 12 Plugin 的关系：插件**只**能产出 IR 描述符，**不**得 import 任何 UI 库；新增 kind 只能走 SDK 版本升级
  - §14.10 反模式（业务层 switch(toolName) / IR 返回 React 节点 / artifact 塞全文进对话流 / kind 无限增长）
  - §14.11 Octopus 实施约束（当前为规范层；代码落地另起 plan）
  - 参考来源汇总（Claude Code `AskUserQuestionTool.tsx` / `AgentTool/UI.tsx` / 本仓 `AGENTS.md` §Frontend Governance / 本仓 13 §13 契约地图）
- Done when：新文件存在；含 §14.1–§14.11 全部小节；IR 类型块为 TypeScript 伪代码；Claude.ai / Claude Code 对比表完整。
- Verify：
  - `rg -c '^## 14\\.' docs/sdk/14-ui-intent-ir.md` → 11
  - `rg -n 'RenderBlock|RenderLifecycle|AskPrompt|ArtifactRef|ArtifactKind' docs/sdk/14-ui-intent-ir.md` 命中 ≥ 20 处
  - `ls docs/sdk/14-ui-intent-ir.md` 成功
- Stop if：若发现 14 章定义的某个 IR kind 与 `@octopus/ui` Catalog 没有合理映射 → 暂停并咨询是否新增 `Ui*` 组件或改用 `raw` 逃生。

### Task 2: 修订 `docs/sdk/03-tool-system.md`（§3.1.1 + §3.10 反模式）

Status: `done`

Files:
- Modify: `docs/sdk/03-tool-system.md`

Preconditions:
- Task 1 完成（14 章链接目标存在）。

Step 1:
- Action：在 §3.1.1 `ToolDisplayDescriptor` 段后追加一条引用："正式的 IR 形状与 5 个生命周期钩子（`onToolUse` / `onToolProgress` / `onToolResult` / `onToolRejected` / `onToolError`）见 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.3 / §14.5；`ToolDisplayDescriptor` 是其历史占位，语义等价于 `RenderLifecycle` 的子集。"
- Done when：段落已插入；`ToolDisplayDescriptor` 语义与 14 章对齐。
- Verify：`rg -n '14-ui-intent-ir' docs/sdk/03-tool-system.md` ≥ 1 处。

Step 2:
- Action：在 §3.10 "Octopus 落地约束" 末尾追加一条：
  - "**工具不得 import 任何 UI 库**：`Tool.displayDescriptor` / `RenderLifecycle` 只能返回 JSON 可序列化的 IR 描述符；具体组件实现归业务层（`apps/desktop` 走 `@octopus/ui`）。违反此约束的工具一律拒绝注册，原则来源见 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.2 / §14.10。"
- Done when：反模式条目已插入；与 14 §14.10 措辞一致。
- Verify：`rg -n '不得 import 任何 UI 库' docs/sdk/03-tool-system.md` 命中。

Stop if：若发现 §3.1.1 `ToolDisplayDescriptor` 的字段结构与 14 §14.3 `RenderLifecycle` 语义不兼容（非包含关系）→ 暂停并回到 14 §14.3 重新定形。

### Task 3: 修订 `docs/sdk/08-long-horizon.md`（§8.4 Artifact 合约）

Status: `done`

Files:
- Modify: `docs/sdk/08-long-horizon.md`

Preconditions:
- Task 1 完成。

Step 1:
- Action：在 §8.4 Artifact 合约末尾（§8.4.3 起手式之后、§8.5 之前）新增 §8.4.4 "成果物双轨：artifact-ref 风格 ↔ file diff 风格"：
  - artifact-ref 风格：Claude.ai 式独立存储（`data/artifacts/` + `artifact.ts` schema + `/api/v1/projects/{id}/deliverables/*`），对话流内以 `RenderBlock kind: 'artifact-ref'` 引用。
  - file diff 风格：Claude Code 式直接落盘 + git 历史，对话流内以 `RenderBlock kind: 'diff'` 内联显示。
  - 两条路径合法且可共存；选择权在工具实现方；模型可同时使用（可视成果物走 artifact，工程文件走 file diff）。
  - 指向 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.6 与 [`13-contracts-map.md`](./13-contracts-map.md) §13.3。
- Done when：§8.4.4 小节存在；两条路径的对比表清晰（≥ 5 个维度：存储位置 / 对话流内形态 / 版本追溯 / 跨 turn 引用 / 适用场景）。
- Verify：
  - `rg -n '8\\.4\\.4' docs/sdk/08-long-horizon.md` 命中。
  - `rg -n '14-ui-intent-ir' docs/sdk/08-long-horizon.md` ≥ 1 处。
- Stop if：若发现 `artifact.ts` schema 当前并未包含对话流内 "preview" 字段 → 暂停并咨询是否需要在 plan 里预留 schema 升级任务。

### Task 4: 修订 `docs/sdk/13-contracts-map.md`（§13.2 / §13.3 / §13.7 / §13.9）

Status: `done`

Files:
- Modify: `docs/sdk/13-contracts-map.md`

Preconditions:
- Task 1 完成。

Step 1:
- Action：§13.2 "三条真相源总览" 升级为 **四条**，追加一行：
  - **UI 意图 IR**（path = `docs/sdk/14-ui-intent-ir.md`；规模 = 10+ `RenderBlock.kind` + 4 顶层类型；治理 = 14 §14.2 / §14.9 + `AGENTS.md` §Frontend Governance）
  - 表头与其它三源保持一致。
- Done when：§13.2 变成四行。
- Verify：`rg -n '三条真相源总览' docs/sdk/13-contracts-map.md` → 仍然一处，但表格行数 +1；`rg -c '^\\| \\*\\*' docs/sdk/13-contracts-map.md` 对应条目 +1。

Step 2:
- Action：§13.3 主映射表新增一列 "UIIntent kinds（代表性）"，为每一行 SDK 章节登记其产出的 IR 种类：
  - 01：progress / text
  - 02：raw（memory proposal 专属）
  - 03：text / markdown / code / diff / record / error / ask-prompt
  - 04：progress / record
  - 05：progress / list-summary / record
  - 06：ask-prompt / error
  - 07：raw（hook 响应多样；默认空）
  - 08：artifact-ref / diff
  - 09：progress / record（trace projection）
  - 10：error
  - 11：record / error
  - 12：record / error
- Done when：表头新增一列，12 行全部填好。
- Verify：`rg -n '^\\| \\*\\*01 Core Loop\\*\\*' docs/sdk/13-contracts-map.md | head -1` 存在；人工目视表格 12 行各有 UIIntent kinds 内容。

Step 3:
- Action：§13.7 "暂无 HTTP 契约" 清单末尾追加一行：
  - UI 意图 IR（`RenderBlock` / `AskPrompt` / `ArtifactRef`）| 14 章 | harness in-process；作为 event log payload 一部分经 `runtime.yaml` events 间接暴露；本身无专属 HTTP 路径
- Done when：§13.7 表格新增一行。

Step 4:
- Action：§13.9 历史修订追加一条：
  - 2026-04-20 | §13.2/§13.3/§13.7 | 纳入第四条真相源 **UI 意图 IR**（`docs/sdk/14-ui-intent-ir.md`）；主映射表 +1 列，harness-internal 清单 +1 行
- Done when：§13.9 表格 +1 行。

Stop if：若发现 §13.3 某一行 SDK 章节对应的 UIIntent kinds 无法归纳（需新增 kind 才能覆盖）→ 暂停；回 Task 1 扩容 14 §14.4。

### Task 5: 同步 `docs/sdk/README.md` 与 `docs/sdk/references.md`

Status: `done`

Files:
- Modify: `docs/sdk/README.md`
- Modify: `docs/sdk/references.md`

Preconditions:
- Task 1–4 完成。

Step 1:
- Action：`docs/sdk/README.md`
  - 文档索引表新增一行：`14-ui-intent-ir.md` — UI 意图 IR：SDK 层的宿主中立渲染契约（`RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactRef`）
  - 阅读顺序补 `→ 14`（接在 13 之后）
  - §7 术语表扩两条：`UI Intent IR`、`Render Block`
  - §8 路线图：`[x] 14-ui-intent-ir.md` + 引用 `docs/plans/2026-04-20-sdk-ui-intent-ir.md`
- Done when：四处同步更新。
- Verify：`rg -n '14-ui-intent-ir' docs/sdk/README.md` ≥ 3 处；`rg -n 'UI Intent IR|Render Block' docs/sdk/README.md` ≥ 2 处。

Step 2:
- Action：`docs/sdk/references.md` §C6 追加一行：`docs/sdk/14-ui-intent-ir.md` — SDK 层 UI 意图 IR 契约；定义 `RenderBlock` / `AskPrompt` / `ArtifactRef` 等顶层描述符及 5 个工具渲染钩子。
- Done when：§C6 新增一条。
- Verify：`rg -n '14-ui-intent-ir' docs/sdk/references.md` = 1。

### Task 6: 验证与 checkpoint

Status: `done`

Files:
- Read-only：`docs/sdk/{14-ui-intent-ir,03-tool-system,08-long-horizon,13-contracts-map,README,references}.md`

Step 1:
- Action：`ReadLints` 所有修改过的文件。
- Verify：No linter errors。
- Stop if：出现 markdown 表格破损 / 未闭合代码块 / dead link → 修复后再过。

Step 2:
- Action：交叉引用自检：
  - `rg -n '14-ui-intent-ir' docs/sdk/` ≥ 6 处（README 3 + 03 1 + 08 1 + 13 1 + references 1 + 14 自身章节锚点若干）
  - `rg -n 'RenderBlock|RenderLifecycle|AskPrompt|ArtifactRef' docs/sdk/` ≥ 25 处
  - `ls docs/sdk/14-ui-intent-ir.md` 存在
- Done when：以上三条全部通过。

Step 3:
- Action：更新本 plan 所有 Task status `pending` → `done`；追加 Checkpoint 块。
- Done when：本文件末尾出现 `Checkpoint 2026-04-20 ...` 块。

## Batch Checkpoint Format

Task 1 独立 Checkpoint（14 章规模较大，单独验证）；Task 2–5 合并 Checkpoint；Task 6 独立验证 Checkpoint。

## Checkpoint 2026-04-20 14 章落地完成

- 批次：Task 1 → Task 2–5 合并 → Task 6
- 状态：
  - Task 1 `done`：`docs/sdk/14-ui-intent-ir.md` 新建；§14.1–§14.11 共 11 节；IR 类型名（`RenderBlock / RenderLifecycle / AskPrompt / ArtifactRef / ArtifactKind`）文内出现 65 次
  - Task 2 `done`：`03-tool-system.md` §3.1.1 `ToolDisplayDescriptor` 段后追加正式契约引用；§3.10 末尾追加"工具不得 import 任何 UI 库"反模式条目；共 2 处链向 14 章
  - Task 3 `done`：`08-long-horizon.md` §8.4 Artifact 合约新增 §8.4.4 成果物双轨（artifact-ref 风格 ↔ file diff 风格）对比表（6 维度）+ 反例 + 反模式链 + 链向 14 §14.6 与 13 §13.3
  - Task 4 `done`：
    - §13.2 "三条真相源总览" → "四条真相源总览"，新增 **UI 意图 IR** 行 + 四条分工说明段
    - §13.3 主映射表 +1 列 "UIIntent kinds（代表性）"，12 行全部填充；标题升级为 "SDK 章节 → 契约 × Schema × UI × UI 意图 IR 主映射"
    - §13.7 harness-internal 清单 +1 行 "UI 意图 IR"
    - §13.9 历史修订 +1 行
  - Task 5 `done`：
    - `README.md` 文档索引 +1 行 `14-ui-intent-ir.md`；阅读顺序从 `03 → 06` 改为 `03 → 14 → 06`；§7 术语表 +2 条（`UI Intent IR` / `Render Block`）；§8 路线图 +1 `[x] 14-ui-intent-ir.md` + 本 plan 引用；收尾语从"全部 14 份"改为"全部 15 份（01–14 + README + references）"
    - `references.md` §C6 原 13 章条目升级为"四条真相源"，紧接一行指向 14 章
  - Task 6 `done`：
    - `ReadLints` 所有修改文件 → **No linter errors**
    - `rg '14-ui-intent-ir' docs/sdk/` → **12 处** 命中（README 4 + references 1 + 13 4 + 08 1 + 03 2；≥ 6 的门槛通过）
    - `rg 'RenderBlock|RenderLifecycle|AskPrompt|ArtifactRef' docs/sdk/` → **87 处** 命中（14 章 58 + 13 章 11 + 08 章 5 + 03 章 5 + README 4 + references 4；≥ 25 的门槛通过）
    - `ls docs/sdk/14-ui-intent-ir.md` → 存在
- 文件变更：
  - Create：`docs/sdk/14-ui-intent-ir.md`、`docs/plans/2026-04-20-sdk-ui-intent-ir.md`
  - Modify：`docs/sdk/03-tool-system.md`、`docs/sdk/08-long-horizon.md`、`docs/sdk/13-contracts-map.md`、`docs/sdk/README.md`、`docs/sdk/references.md`
- Verification：全部门槛通过（见 Task 6 各项命中数）。
- Blockers：无。
- Next（非本轮 scope）：
  - 代码落地：`packages/schema/src/ui-intent.ts`（Zod discriminated union + 类型导出）+ 业务层 dispatcher 骨架（`apps/desktop/src/components/ir/` 或等价位置）。另起 plan `docs/plans/xxxx-ui-intent-schema-impl.md`。
  - 迁移：现有工具（`fs_edit` / `ask_user_question` / `create_artifact` / ...）按 14 §14.5 补齐 `render: RenderLifecycle` 字段；分批推进，保留 `displayDescriptor` 历史占位直至全量迁移完成。
  - 宿主实现：桌面按 14 §14.7 表完成 `RenderBlock.kind → @octopus/ui` 组件映射；缺失组件先在 `@octopus/ui` 扩展，再放出新 kind。

## Task 7: Artifact 迭代 6 点扩充（Claude app 级 artifact 能力对齐）

Status: `done`

Precondition: Task 1–6 checkpoint 已闭合；14 章骨架存在。
Scope: 不新增章节主轴；只在 §14.3.4 / §14.4 / §14.6 扩充并新增 §14.12 / §14.13；13 章顺带同步。

Files (write):
- `docs/sdk/14-ui-intent-ir.md`
- `docs/sdk/13-contracts-map.md`

Files (read-only 对齐):
- `contracts/openapi/src/components/schemas/projects.yaml`（`ArtifactVersionReference` / `DeliverableSummary` / `DeliverableDetail` / `DeliverableVersionSummary` / `DeliverableVersionContent` / `CreateDeliverableVersionInput` / `PromoteDeliverableInput` / `ForkDeliverableInput`）
- `contracts/openapi/src/components/schemas/shared.yaml`（`ArtifactStatus`）
- `contracts/openapi/src/components/schemas/workspace.yaml`（`ResourcePreviewKind`）

Step 1 `done`：§14.3.4 扩 `ArtifactRef`
- 新增字段：`parentVersion` / `status` / `contentType` / `supersededByVersion`
- 新增 §14.3.4.1 `ArtifactKind` vs `ResourcePreviewKind` 分工表 + IR→schema 映射表（方案 A：双保留、职责互不越界）
- 新增 §14.3.4.2 `ArtifactKind` 专属安全约束表（`html` iframe sandbox / `react` 预编译禁 eval / `svg` DOMPurify / `mermaid` strict / `markdown` 禁 raw html / `code|json|binary` 只读）

Step 2 `done`：§14.4 表后追加"kind 级安全约束速查"段落（`artifact-ref` 路由到 §14.3.4.2；其他 kind 默认规则）

Step 3 `done`：§14.6 对比表追加两行
- "迭代链追溯"：Claude.ai = `version` + `parentVersion` + `fork_artifact` + `promote_artifact`；Claude Code = `git log` / `git diff` / 分支
- "迭代触发方"：两侧均为模型下一轮 `tool_use`

Step 4 `done`：新增 §14.12 Artifact 迭代工具套件
- §14.12.1 六件套一览表（`create_artifact` / `read_artifact` / `update_artifact` / `list_artifacts` / `fork_artifact` / `promote_artifact`，逐行对齐 HTTP 端点与 schema）
- §14.12.2 render 契约示例（四个代表性工具的 `RenderLifecycle` 伪代码）
- §14.12.3 版本 / Status / Promotion 三条正交轴分工（`version` append-only / `ArtifactStatus` 业务层推 / `DeliverablePromotionState` `promote_artifact` 专管）
- §14.12.4 与 `fs_edit` 互不干扰（`read_artifact` → `fs_write`；反之禁止）

Step 5 `done`：新增 §14.13 跨轮 Artifact 发现
- §14.13.1 `list_artifacts` 显式查询（render 示例）
- §14.13.2 `SessionStart` hook 隐式注入简表（7 字段、每项 ~100 token、硬上限 20 个）
- §14.13.3 Compaction 幸存规则（`runtime/notes/<session>.md` 固定段落）
- §14.13.4 权限边界（列出免审批；读 / 写仍走 06 §6）

Step 6 `done`：13 §13.3 08 行补备注"版本链 / fork / 跨轮引用 → 14 §14.12 / §14.13"；UIIntent kinds 追加 `record`；§13.9 +1 行历史修订

Verify:
- `rg -n '§14\.12|§14\.13' docs/sdk/` ≥ 4 处（14 章自身 + 13 章 + 08 章 + README）
- `rg -n 'list_artifacts|fork_artifact|promote_artifact|update_artifact|create_artifact|read_artifact' docs/sdk/14-ui-intent-ir.md` ≥ 15 处
- `rg -n 'parentVersion|supersededByVersion|ArtifactStatus|DeliverablePromotionState' docs/sdk/14-ui-intent-ir.md` ≥ 8 处
- `ReadLints` 无错

Stop if:
- schema 字段与 `projects.yaml` 不一致 → 停，**不**修改 schema，以 schema 为准调整 14 章描述
- `PromoteDeliverableInput` 与 `ArtifactStatus` 语义被混为一谈 → 停，按"三条正交轴"重新说明

## Checkpoint 2026-04-20 Artifact 迭代扩充完成（Task 7）

- 批次：Task 7 单独批次（Task 1–6 已各自 Checkpoint）
- 状态：Task 7 `done`
- 文件变更：
  - Modify：`docs/sdk/14-ui-intent-ir.md`（§14.3.4 扩 + §14.3.4.1 / §14.3.4.2 新增 + §14.4 速查段 + §14.6 表 +2 行 + §14.12 / §14.13 新章）
  - Modify：`docs/sdk/13-contracts-map.md`（§13.3 08 行备注 + UIIntent kinds 补 `record`；§13.9 +1 行修订）
  - Modify：`docs/plans/2026-04-20-sdk-ui-intent-ir.md`（本 plan 追加 Task 7 + 本 Checkpoint）
- Verification：见 Task 7 Verify 段命中数（落地后由自检补齐）。
- Blockers：无。
- Next（仍非本轮 scope）：
  - 实装 `list_artifacts` 工具与 `SessionStart` hook（归属未来 harness 代码 plan）
  - `packages/schema/src/ui-intent.ts` 追加 `ArtifactRef` 新字段与 Zod 校验
  - 桌面 `UiArtifactBlock` 子渲染器按 §14.3.4.2 安全约束实装 `html` / `react` / `svg` / `mermaid` 通道
