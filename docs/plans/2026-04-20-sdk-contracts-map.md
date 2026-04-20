# SDK 新章 `13-contracts-map.md` 落地计划（2026-04-20）

> 依据：P1 完成后用户确认（agent-transcripts 当前会话），继续补第 13 章 `13-contracts-map.md`，作为 SDK 文档层与代码/契约层之间的"入口地图"。

## Goal

为 `docs/sdk/` 新增第 13 章 `13-contracts-map.md`，作为 SDK **规范层 01–12** 与 Octopus **真相源三条线**（`contracts/openapi/` · `packages/schema/` · `packages/ui/`）之间的双向导航表。读者（尤其是 AI Coding Agent）打开 SDK 任一章节后，都能在 1 跳内定位到其对应的契约文件、schema 文件或 UI 实现位置。

## Architecture

本章纯粹是"索引 + 地图"，**不**引入新规范。它是下列既有规则在文档层的可见投影：

- `AGENTS.md` §Request Contract Governance：`/api/v1/*` 必须在 `contracts/openapi/src/**` 定义
- `AGENTS.md` §Frontend Governance：`@octopus/schema`（feature-based）与 `@octopus/ui`（Catalog 清单）是唯一共享入口
- `contracts/openapi/AGENTS.md`：`paths/` / `components/schemas/` / `components/parameters/` / `components/responses/` 分层
- `docs/api-openapi-governance.md`：变更顺序 `src/**` → `pnpm openapi:bundle` → `pnpm schema:generate`

核心产物：**一张三列映射表**（SDK 章节 / OpenAPI 路径 / Schema 文件 / 备注），再配"未映射清单"专门标示"目前是纯 harness 内部机制、不走 HTTP"的 SDK 能力。

## Scope

- **In scope**：
  - 新建 `docs/sdk/13-contracts-map.md`
  - 更新 `docs/sdk/README.md` 的文档索引表 + 路线图 + 阅读顺序
  - 更新 `docs/sdk/references.md` 的内部引用章（§C6）
- **Out of scope**：
  - 不**新增**任何 openapi path / schema / ui 组件
  - 不修订 01–12 章的任何正文
  - 不做自动化校验（未来用 CI 脚本对表，属后续优化）

## Risks Or Open Questions

- **映射颗粒度**：全表一行一映射太碎（01 章 20 + 节×5 真相源 = 100+ 行）；采用**章节级**（§NN，不到 §NN.Y.Z）粒度，每行 ≤ 3 个代表性目标
- **未映射条目**：hook / brain loop / context compaction 等纯内部机制需要显式列出"暂无 HTTP 契约"，避免读者误以为"没找到 = 遗漏"
- **动态性**：openapi path / schema 数量会随产品演进而变，本章约定"**只**对齐当前快照；变更时追加修订记录"，不做机械同步
- **与 `AGENTS.md` 的边界**：本章**不**复述 Persistence / Runtime Config 治理规则，只做引用链接；避免双头真相源

## Execution Rules

- 先写 plan（本文件），然后按 Task 顺序执行，每个 Task 独立原子
- 映射表严格基于**实际磁盘状态**：
  - OpenAPI：只写 `contracts/openapi/src/paths/*.yaml` 里实际存在的资源路径前缀
  - Schema：只引 `packages/schema/src/*.ts` 里实际存在的文件
  - UI：只引 `packages/ui/src/components/*.vue` 里实际存在的组件
- 改动后 `ReadLints`；Markdown 表格渲染自检

## Task Ledger

### Task 1: 起草 `13-contracts-map.md`

Status: `pending`

Files:
- Create: `docs/sdk/13-contracts-map.md`

Step 1:
- Action：按以下骨架起草
  - §13.1 目的与非目的
  - §13.2 三条真相源总览（table）
  - §13.3 SDK 章节 → 契约 / Schema / UI 映射主表（01–12 每章一行，关键章再展开）
  - §13.4 OpenAPI paths × SDK 章节 反向索引（paths/*.yaml 在 SDK 哪些章节被涉及）
  - §13.5 Schema 文件 × SDK 章节 反向索引（packages/schema/src/*.ts 被哪些章节使用）
  - §13.6 UI Catalog（本仓 `AGENTS.md` §Frontend Governance 列的 Shared UI Component Catalog）× SDK 章节
  - §13.7 "暂无 HTTP 契约"的 SDK 能力清单（hooks / brain loop / 内部 compaction / prompt cache 策略 / ...）
  - §13.8 变更规则：改动任一真相源时**也**要订正本章（追加 "历史修订" 条目）
- Done when：新文件存在；所有引用的 yaml / ts / vue 均指向磁盘真实存在的文件
- Verify：
  - `rg -l 'contracts/openapi/src/paths' docs/sdk/13-contracts-map.md` → 1 命中
  - 每个列出的 yaml 实际 `ls` 能找到
- Stop if：发现 SDK 章节中声明的能力在 openapi/schema/ui 层找不到对应物且又不该归入 §13.7 → 暂停询问（避免制造 ghost mapping）

### Task 2: 更新 README.md

Status: `pending`

Files:
- Modify: `docs/sdk/README.md`

Step 1:
- Action：
  - §文档索引表新增一行 `13-contracts-map.md`
  - §阅读顺序更新：`README → 04 → 11 → 12 → 01 → 02 → 03 → 06 → 05 → 07 → 08 → 09 → 10 → 13`
  - §8 路线图新增 `[x] 13-contracts-map.md`
- Done when：三处同步更新
- Verify：`rg -n '13-contracts-map' docs/sdk/README.md` 至少 3 处命中

### Task 3: 更新 references.md

Status: `pending`

Files:
- Modify: `docs/sdk/references.md`

Step 1:
- Action：在 §C6 "本仓 `docs/` 其他相关" 末尾加一条："`docs/sdk/13-contracts-map.md` — SDK 章节与三条真相源（OpenAPI / Schema / UI）的双向映射表"
- Done when：§C6 出现该条目
- Verify：`rg -n '13-contracts-map' docs/sdk/references.md` 1 命中

### Task 4: 验证

Status: `pending`

Step 1:
- Action：`ReadLints` 三个改动文件
- Verify：No linter errors；`ls` 表格中引用的每个 yaml / ts / vue 均存在
- Stop if：发现任一引用指向不存在的文件

## Batch Checkpoint Format

Task 1–3 一次性 Checkpoint；Task 4 独立验证 Checkpoint。

## Checkpoint 2026-04-20 执行完成

- 批次：Task 1 → Task 2 → Task 3 → Task 4
- 状态：
  - Task 1 `done`：`docs/sdk/13-contracts-map.md`（142 行，9 小节）新建完成
    - §13.1 目的 / §13.2 三源总览 / §13.3 SDK → 契约主映射（12 行）/ §13.4 OpenAPI 反向（9 行）/ §13.5 Schema 反向（31 行）/ §13.6 UI Catalog 映射 / §13.7 harness-internal（10 项无 HTTP 能力）/ §13.8 变更规则 / §13.9 历史修订
  - Task 2 `done`：README 更新 3 处（文档索引 +1 行 / 阅读顺序加 `→ 13` / 路线图 +4 行含新 plan 引用）
  - Task 3 `done`：references.md §C6 末尾 +1 行指向 13 章
  - Task 4 `done`：`ReadLints` 三个文件 → No linter errors；磁盘核对：31 份 schema `.ts`、9 份 paths `.yaml`、`AGENTS.md` UI Catalog 条目全部存在
- 文件变更：
  - Create：`docs/sdk/13-contracts-map.md`
  - Modify：`docs/sdk/README.md`、`docs/sdk/references.md`
- Verification：
  - `ReadLints docs/sdk/{13-contracts-map,README,references}.md` → pass
  - `rg '13-contracts-map' docs/sdk/` → README 2 处、references 1 处、13 章自身 1 处
  - Schema 存在性：31 ts 文件 `test -f` 全通过
- Blockers：无
- Next：
  - 文档层收尾完毕（01–13 + README + references = 15 份文档）
  - 代码层下一步：起草 `docs/plans/2026-04-20-plugin-system-m0m5.md`（12 §12.16）待用户确认
