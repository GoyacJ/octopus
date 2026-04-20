# SDK 文档 P0 交叉一致性修订计划（2026-04-20）

> 依据：`docs/sdk/` 全量审计（2026-04-20），识别出 7 处 P0 级跨章一致性问题。本计划把这些问题原子化为可执行的修订任务。

## Goal

修复 `docs/sdk/` 13 份文档间的 7 处 P0 级跨章一致性问题，使 SDK normative spec 在工具命名空间、路径约定、插件扩展点互链三个层面达到单一口径。

## Architecture

本计划只修改 `docs/sdk/*.md` 文档层；不触动 `contracts/openapi/`、`packages/schema/`、`packages/ui/` 等任何代码或契约层。所有变更都是"把 A 章的表述对齐到 B 章已正确的表述"，**不引入新规范**，不改变任何已声明的公共契约。

## Scope

- **In scope**：
  - `docs/sdk/01-core-loop.md` §1.4.2 工具名风格
  - `docs/sdk/02-context-engineering.md` §2.6.2 `NOTES.md` / `runtime/notes/` 语义对齐
  - `docs/sdk/03-tool-system.md` §3.6 末尾补 MCP Server 指向 12 §12.3
  - `docs/sdk/07-hooks-lifecycle.md` 新增 §7.11 Hook 来源与插件
  - `docs/sdk/08-long-horizon.md` §8.9 L2 progress.txt → `runtime/notes/<session>.md`
  - `docs/sdk/10-failure-modes.md` §10.6 L2 同步修订
  - `docs/sdk/11-model-system.md` §11.17 加 "Provider 通过 plugin 注册" 条目
- **Out of scope**：
  - P1 / P2 修订（术语表扩充、源码路径前缀统一、行号清理、README §7）
  - 12 §12.16 M0–M5 拆分为独立 plan
  - 潜在的 `13-contracts-map.md` 新章

## Risks Or Open Questions

- 01 §1.4.2 "safe" / "unsafe" 清单里的工具名改动，需要保留"Bash（除非显式声明 read-only）"的括注语义 → 改为 `shell_exec` 时保留该括注。
- 02 §2.6.2 表格里的路径当前是 `TodoWrite` → `runtime/todos/<session>.json`、`NOTES.md / runtime/notes/<session>.md（原 claude-progress.txt）`。应拆为两行，语义与 08 §8.4.1 一致。
- 07 §7.11 新增小节需与 §7.6 "Hook 配置源" 形成明确分工：7.6 讲优先级与来源层级，7.11 补"插件作为来源的一种"。

## Execution Rules

- 单批执行，不拆多轮；每处改动独立原子。
- 每个 Task 只改一个文件；完成后追加一次 Checkpoint。
- 改完后用 `ReadLints` 检查；若出现未改到位或破坏链接，立即停机询问。

## Task Ledger

### Task 1: [01] 工具清单 snake_case 化

Status: `pending`

Files:
- Modify: `docs/sdk/01-core-loop.md`

Preconditions:
- 03 §3.2.3 与 §3.4 已采用 `fs_read / fs_grep / fs_glob / shell_exec / fs_write / fs_edit` 命名（已验证）。

Step 1:
- Action：把 §1.4.2 "安全的" / "不安全的" 清单里的 CamelCase 工具名替换为 snake_case；保留 Bash 括注语义时改写为 `shell_exec`。
- Done when：两行清单全部 snake_case；前后文语义不变。
- Verify：`rg -n 'Read, Grep, Glob' docs/sdk/01-core-loop.md` 无命中。
- Stop if：发现 01 章其他位置仍存在 CamelCase 工具名举例 → 先询问是否一并修订。

### Task 2: [02] `NOTES.md` / `runtime/notes/` 路径对齐

Status: `pending`

Files:
- Modify: `docs/sdk/02-context-engineering.md`

Preconditions:
- 08 §8.4.1 已明确区分"项目根 `NOTES.md / CLAUDE.md`（长期）"与"会话级 `runtime/notes/<session>.md`（每步覆盖）"。

Step 1:
- Action：把 §2.6.2 载体类型表第 2 行拆为两行：`NOTES.md`（项目根，不变设计决策）与 `runtime/notes/<session>.md`（会话级进度快照），并删除"原 `claude-progress.txt`"的误导性并置。
- Done when：表述与 08 §8.4.1 一致。
- Verify：`rg -n 'claude-progress.txt' docs/sdk/02-context-engineering.md` 只在引用 Anthropic 原文处出现，不在规范路径处。
- Stop if：发现 02 章其他小节（如 §2.11）也出现路径矛盾 → 一并修订。

### Task 3: [03] §3.6 末尾补 MCP Server 指向 12 §12.3

Status: `pending`

Files:
- Modify: `docs/sdk/03-tool-system.md`

Preconditions:
- 12 §12.3 表格已把 MCP Server 列为 plugin extension point。

Step 1:
- Action：在 §3.6 末尾（§3.6.4 之后、§3.7 之前）加一段短注："在 Octopus 中 MCP Server 的注册统一走 Plugin Registry，见 12 §12.3 / §12.8.4。"
- Done when：03 章 MCP 节末有指向 12 章的明确链接。
- Verify：`rg -n '12-plugin-system.md' docs/sdk/03-tool-system.md` 至少 1 处命中。
- Stop if：无。

### Task 4: [07] 新增 §7.11 Hook 来源与插件

Status: `pending`

Files:
- Modify: `docs/sdk/07-hooks-lifecycle.md`

Preconditions:
- 12 §12.3 / §12.5.2 明确 hooks 是 plugin manifest 字段。

Step 1:
- Action：在 §7.10 "反模式" 之前、§7.9 之后插入 §7.11 "Hook 来源与插件"，说明 hook 可来自四类来源（session / project / workspace / plugin），其中 plugin 来源的 hook 在启用该插件时一并激活，在禁用时一并回收；指向 12 §12.3 / §12.8.4。
- Done when：07 章新增该小节；原章节编号保持不变（§7.10 反模式、§参考来源汇总）。
- Verify：`rg -n '^## 7\.' docs/sdk/07-hooks-lifecycle.md` 显示 7.1–7.11 齐全。
- Stop if：无。

### Task 5: [08] §8.9 L2 progress.txt → runtime/notes

Status: `pending`

Files:
- Modify: `docs/sdk/08-long-horizon.md`

Preconditions:
- §8.4.1 已定义 `runtime/notes/<session>.md`。

Step 1:
- Action：把 §8.9 列表里 "读 NOTES + progress.txt" 改为 "读 `NOTES.md` + `runtime/notes/<session>.md`"。
- Done when：`rg -n 'progress.txt' docs/sdk/08-long-horizon.md` 只在 §8.4.1 Anthropic 原文引用处出现。
- Verify：同上命令。
- Stop if：无。

### Task 6: [10] §10.6 L2 同步修订

Status: `pending`

Files:
- Modify: `docs/sdk/10-failure-modes.md`

Preconditions:
- 08 的命名已统一。

Step 1:
- Action：§10.6 L2 行 "强制读 NOTES + progress.txt" 改为 "强制读 `NOTES.md` + `runtime/notes/<session>.md`"；L4 / L5 若有类似措辞一并检查。
- Done when：10 章全文不再把 `progress.txt` 作为规范路径。
- Verify：`rg -n 'progress.txt' docs/sdk/10-failure-modes.md` 无命中。
- Stop if：无。

### Task 7: [11] §11.17 加 "Provider 通过 plugin 注册" 条目

Status: `pending`

Files:
- Modify: `docs/sdk/11-model-system.md`

Preconditions:
- 12 §12.3 / §12.8.4 / §12.16 M3 已明确 provider 是 plugin extension point。

Step 1:
- Action：在 §11.17 "对 Octopus 的落地约束" 项目列表末尾补一条："**Provider 注册**：所有 Provider（含文中 9 厂商）通过 `api.registerProvider(...)` 在 Plugin Registry 注册；核心**不**对 provider id 做 switch，详见 12 §12.3 / §12.8.4 / §12.16 M3。"
- Done when：11 章 §11.17 出现指向 12 章的明确链接。
- Verify：`rg -n '12-plugin-system.md' docs/sdk/11-model-system.md` 至少 1 处命中。
- Stop if：无。

### Task 8: 验证

Status: `pending`

Files:
- Verify only

Step 1:
- Action：`ReadLints` 上述 7 个改动文件，确认无 Markdown 破坏。
- Done when：无 lint 报错；所有交叉链接 `./NN-xxx.md` 均指向现存文件。
- Verify：`rg -n '\]\(\./(01|02|03|07|08|10|11|12)-' docs/sdk/` 每条链接文件名均存在。
- Stop if：任一链接指向不存在的锚点或文件。

## Batch Checkpoint Format

执行完 Task 1–7 后一次性 Checkpoint；Task 8 作为独立验证 Checkpoint。

## Checkpoint 2026-04-20 执行完成

- 批次：Task 1 → Task 7（Task 5 经复查判定为"审计误判"，08 章已对齐，取消）+ Task 8 验证
- 状态：
  - Task 1 `done`：01 §1.4.2 工具清单已改 snake_case
  - Task 2 `done`：02 §2.6.2 载体表拆为 `NOTES.md` / `runtime/notes/<session>.md` 两行
  - Task 3 `done`：03 新增 §3.6.5 指向 12 §12.3 / §12.8.4 / §12.10
  - Task 4 `done`：07 新增 §7.11 "Hook 来源与插件"
  - Task 5 `cancelled`：08 §8.9 已使用 `runtime/notes/<session>.md`，现有 `progress.txt` 均为引用说明（"原 / 业界常称"），保留正确
  - Task 6 `done`：10 §10.6 L2 与行 100 checklist 修为 `runtime/notes/<session>.md`
  - Task 7 `done`：11 §11.17 末尾新增 "Provider 注册" 条目，指向 12 §12.3 / §12.8.4 / §12.16
  - Task 8 `done`：`ReadLints` 无报错；`Read, Grep, Glob` / `Write, Edit, Bash` 在 docs/sdk 下已无规范用法残留；`progress.txt` 仅保留于说明性语境与 references.md 的 Anthropic 原文引用
- 文件变更：
  - `docs/sdk/01-core-loop.md`
  - `docs/sdk/02-context-engineering.md`
  - `docs/sdk/03-tool-system.md`
  - `docs/sdk/07-hooks-lifecycle.md`
  - `docs/sdk/10-failure-modes.md`
  - `docs/sdk/11-model-system.md`
- Verification：
  - `ReadLints docs/sdk/{01,02,03,07,10,11}*.md` → pass（No linter errors）
  - `rg 'Read, Grep, Glob|Write, Edit, Bash' docs/sdk/` → 无命中
  - `rg 'progress\.txt' docs/sdk/` → 仅在说明性/外部引用位置命中
  - 12 章实际 heading 校准：§12.3 扩展点全景、§12.8.4 Register、§12.10 安全与沙箱、§12.16 实施优先级（锚点已改为纯文件链接 + 文字引用，避免跨文档中文锚点不稳）
- Blockers：无
- Next：
  - P1 批（术语表扩充 / 源码路径前缀统一 / README §7 修订 / 行号清理）待用户确认
  - 12 §12.16 M0–M5 拆分为独立实施 plan 待用户确认
