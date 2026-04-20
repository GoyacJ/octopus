# AGENTS.md — `docs/plans/sdk/`

本文件是 **`docs/plans/sdk/` 子目录的本地覆盖**，按"最近 AGENTS.md 优先"原则生效；未覆盖处沿用根 `/AGENTS.md`、`docs/AGENTS.md`、`docs/plans/AGENTS.md`（若存在）。

本目录是 **Octopus Agent Harness SDK 重构**（2026-04-20 启动，预计 6–8 周）的唯一控制面，不用于其他通用计划。

---

## 1. 命名约定（覆盖根 AGENTS.md 的日期前缀规则）

- **文件名使用顺序编号**：`NN-<topic>.md`，`NN` 从 `00` 递增，`<topic>` 全小写短横连词。
- **不使用**根 `AGENTS.md` 规定的 `YYYY-MM-DD-<topic>.md` 形式。
  - 理由：本目录是**一次连贯重构的控制面**，跨 8 周、多作者、强线性阅读顺序；序号表意优于时间戳表意。
  - 零散的、非本重构范围的计划仍遵循根约定，写在 `docs/plans/` 顶层，不进入本目录。
- **文件名在 `README.md §文档索引` 预登记**：
  - W0 文档（`00` 至 `03`）已登记；
  - W1–W8 子 Plan 的文件名已在 `README.md:16–23` 预登记为 `04-week-1-contracts-session.md` … `11-week-8-cleanup-and-split.md`。
  - 执行者**按登记名创建**，不得另起命名；如确需改名，必须先提交修改 `README.md` 索引的 PR，再创建文件。
- **禁止**在本目录引入日期前缀的文件（`YYYY-MM-DD-*.md`）；此类内容属于顶层 `docs/plans/`。

## 2. 状态流转

- 每个文档在 `README.md §文档索引` 表格的 `状态` 列登记一个状态：`draft / pending / in_progress / blocked / done`。
- **状态切换必须同批次落到索引**：
  - 新建子 Plan 的 PR → 把对应行从 `pending` 改为 `draft` 或 `in_progress`。
  - 子 Plan 进入执行 → 改为 `in_progress`。
  - 子 Plan 满足 `00-overview.md §4` 的周出口状态 + Weekly Gate checklist → 改为 `done`。
  - 任何 Stop Condition 触发（见 `01-ai-execution-protocol.md §4`） → 改为 `blocked`，并在该子 Plan 末尾登记阻塞原因。
- `00 / 01` 从 `draft` 切到 `done` 的条件：两份文档在 W8 收尾时一致审计通过；过程中若发生勘误，仍保持 `draft` 以示持续迭代中。

## 3. 模板与内容要求

- 所有子 Plan **必须**基于 `docs/plans/PLAN_TEMPLATE.md` 起草；执行与汇报使用 `docs/plans/EXECUTION_TEMPLATE.md`。
- 子 Plan 必须包含以下最小结构（模板已覆盖，此处为强提示）：
  1. `Goal / Non-goal / Scope`；
  2. 任务矩阵（原子 Task，含 `Done when`、验证命令、依赖、stop conditions 链接回 `01-ai-execution-protocol.md §4`）；
  3. 公共面变更登记（指向 `02-crate-topology.md` 的具体小节）；
  4. 退役登记（指向 `03-legacy-retirement.md` 的具体小节 + 旧符号 → 新位置）；
  5. Weekly Gate 的出口状态（与 `00-overview.md §4` 的本周出口状态**逐条对齐**）；
  6. 变更日志表。
- 子 Plan **不得**重复复制 `00 / 01 / 02 / 03` 的内容；只允许引用（`docs/plans/sdk/02-crate-topology.md §2.4`）。

## 4. 与 `docs/sdk/*` 规范源的关系

- 本目录**不修改** `docs/sdk/01–14`，除非发现与实现存在矛盾；此时按 `docs/sdk/README.md` 末尾的 `## Fact-Fix 勘误` 小节追加条目，并在对应子 Plan 的变更日志内引用该条目。
- 新增/调整公共面时，必须回溯更新 `02-crate-topology.md` 的 `§2.*`、`§4 契约差异清单`、`§5 UI Intent IR 登记表`；禁止公共面在子 Plan 中"裸增"不登记。

## 5. 守护扫描

以下命令在每个 PR 前由作者自检、在 Weekly Gate 由审核者复核：

```bash
# 5.1 命名约束：本目录不得出现日期前缀文件
find docs/plans/sdk -type f -name '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]-*.md'

# 5.2 索引完整性：本目录实际文件必须与 README.md 索引表一一对应
ls docs/plans/sdk/*.md
rg '^\| `[0-9]{2}-' docs/plans/sdk/README.md

# 5.3 AGENTS.md 本文件不计入 5.2 的索引表；索引只登记 Plan 文件
```

命中 5.1 或 5.2 不一致 → 视为 `01-ai-execution-protocol.md §5` 的 **Stop Condition #11**（命名/登记违规），阻断合入。

## 6. 子 Plan 起稿时机（滚动式；与 AI 执行的上下文特性对齐）

本目录采用 **滚动式起稿**，不做"一次性写完 W1–W8 所有 Plan"的瀑布式规划。理由：AI 执行对"最近一层"的精确性敏感；过早细化远期 Plan 会在本周实测后被推翻，形成 O(N) 级跨文件补丁。

| 文档层 | 起稿时机 | 定稿要求 |
|---|---|---|
| W0 总控（`00 / 01 / 02 / 03`） | 2026-04-20 一次性定稿 | 后续只通过 `docs/sdk/README.md` 的 `## Fact-Fix 勘误` 小节回流修正，不重写 |
| 本周 Plan（W<sub>n</sub>） | 本周**第一行生产代码提交前**必须**完整定稿** | 含 Goal / Scope / Risks / Task Ledger / Exit State 对齐表；违反 → Stop Condition #8 |
| 下一周 Plan（W<sub>n+1</sub>） | 本周 Task 进度 ≥ 60%（按 Task 数计）时起稿 | 到本周 Weekly Gate 勾选前必须由 `pending` → `draft`；不要求 Task Ledger 完整，允许先写 Goal / Scope / Risks |
| 再往后（W<sub>n+2</sub> 及以后） | **禁止提前起稿** | `README.md §文档索引` 仅保留文件名与一句描述 + `pending`；不得在任何文档出现 Task 级细节 |

### 例外

当前周发现**必然影响 W<sub>n+2</sub> 以后的决策**（例如跨周的数据模型变动、契约断裂风险），按如下顺序处理，**不得私自起稿未来周 Plan**：

1. 把决策与影响范围作为一行登记到 `README.md §关键不变量` 或 `00-overview.md §6 风险登记簿`；
2. 如决策足以推翻 `00 §4 W<sub>n+2</sub> 出口状态` → 走 Fact-Fix，改 `00` 的对应小节，同步更新 `README.md §文档索引` 里相应周的状态/描述。

### 守护扫描（加入 §5）

```bash
# 6.1 远期 Plan 禁止提前起稿：pending 状态以外的文件不得包含 Task Ledger 标题
#     （本周 Plan 与 W+1 允许 draft / in_progress；再远的周状态必须仍为 pending 且文件不存在）
ls docs/plans/sdk/[0-9][0-9]-week-*.md 2>/dev/null
# 与 README.md 索引逐行比对：任一超出"本周 + 下一周"范围且已存在的 Plan 文件 → 违规
```

命中 §6 守护 → `01-ai-execution-protocol.md §5 Stop Condition #11`（命名/登记违规的子情形）。

---

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-20 | 首稿：覆盖根 AGENTS.md 的日期前缀命名，改为 `NN-<topic>.md` 顺序编号；定义状态流转、模板要求、守护扫描 | Architect |
| 2026-04-20 | §6 新增：滚动式起稿时机；远期 Plan 禁止提前起稿；例外走 Fact-Fix 回流；与 Stop Condition #11 关联 | Architect |
