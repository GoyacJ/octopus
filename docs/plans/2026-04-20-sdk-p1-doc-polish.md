# SDK 文档 P1 文档层收尾（2026-04-20）

> 依据：P0 修订完成后（`2026-04-20-sdk-p0-cross-link-fix.md`），在文档层再做一批影响**查阅体验与 normative 可追溯性**的 P1 改动。

## Goal

完成 `docs/sdk/` 文档层的 P1 收尾：术语表覆盖度、源码路径简称约定、源码行号快照脚注，确保 13 份文档对 normative 语义、路径引用、版本锚定三件事都单口径。

## Architecture

仅改 `docs/sdk/*.md`。不动代码、契约、schema。P1 本质是"让文档自洽"：读者可以不离开 `docs/sdk/` 就读懂术语、复现源码定位、理解行号为何是精确的。

## Scope

- **In scope**：
  - `docs/sdk/README.md` §7 术语表扩充
  - `docs/sdk/references.md` §C1 加 `restored-src/` 简称声明
  - `docs/sdk/references.md` §E 增补源码行号锚定说明
- **Out of scope**：
  - 12 §12.16 M0–M5 实施 plan（独立文档）
  - 逐行清理行号（经重新勘察，`docs/sdk/` 下硬编码行号**仅 8 处**，全部指向 v2.1.88 的类型定义行，保留比删除更有用）

## Risks Or Open Questions

- 术语表补多少条才"够"？原则：**只补"在其他章节高频使用且初读者易被挡住"的**；避免收录全 SDK 词典。
- `restored-src/` 简称声明放 §C1 还是 §E？结论：放 §C1 顶部（ownership 与路径绑定在 §C1），§E 继续作为验证规则专节。

## Execution Rules

- 小批：P1a → P1b → P1c 顺序；每项完成即更新 todo 状态。
- 术语表条目命名与其他章节正文已有使用形式保持一致（大小写 / 连字符）。
- 改动后 `ReadLints` 校验。

## Task Ledger

### Task P1a: README §7 术语表扩充

Status: `pending`

Files:
- Modify: `docs/sdk/README.md`

Preconditions:
- 已审读 02/06/08/11/12 各章术语用法；确认 10 条待补术语的高频性（下列候选词至少在 2 份以上章节出现）。

Step 1:
- Action：在 §7 现有 12 条后，按字母序或概念聚类补充：
  - **Durable Scratchpad**（02 §2.6.2；SDK 自定义名）
  - **Goldilocks Zone**（02 §2.3；Anthropic 原词）
  - **Agentic Memory**（02 §2.7；Anthropic 原词）
  - **Initializer + Coding Agent**（08 §8.3；长时任务模式名）
  - **Canonical Naming**（11 §11.3；跨厂商命名归一化）
  - **Protocol Adapter**（11 §11.6；`protocol_family` → canonical IR 适配层）
  - **Canonical Message IR**（11 §11.6；中立消息表达）
  - **Slot**（12 §12.9；单选扩展点机制）
  - **MCPB**（12 §12.6；`.mcpb` / `.dxt` 离线 bundle 格式）
  - **Autonomy Dial**（06 §6.2；permission mode 渐变光谱）
  - **Effective Config Hash**（README §C7；session 绑定 config 快照的哈希）
- Done when：§7 共 22 条；无重复；每条 ≤ 2 行。
- Verify：`rg -n '^\|' docs/sdk/README.md` 行数变化 +11 (含 separator 已存在情况)。
- Stop if：发现术语在主文内定义不一致 → 暂停并提示修订主文。

### Task P1b: references.md §C1 加 `restored-src/` 简称声明

Status: `pending`

Files:
- Modify: `docs/sdk/references.md`

Step 1:
- Action：在 §C1 小节开头（`**路径**:` 行下方、`**README**:` 行之前）插入："**路径简写约定**：本文件其余部分中出现的 `restored-src/src/...` 一律指 `docs/references/claude-code-sourcemap-main/restored-src/src/...`（省略前缀仅为简化正文长度）。"
- Done when：§C1 顶部出现该简写约定；其他章节引用形态不变（不做 find-replace 风暴）。
- Verify：`rg -n '路径简写约定' docs/sdk/references.md` 命中 1 处。
- Stop if：发现 §C1 以外的简称引用也存在歧义（e.g. `hermes/`）→ 单独议。

### Task P1c: references.md §E 增补行号锚定说明

Status: `pending`

Files:
- Modify: `docs/sdk/references.md`

Preconditions:
- §E 第 340 行已声明 `restored-src v2.1.88` 快照；补充一条专门讲"行号"的说明即可。

Step 1:
- Action：在 §E "源码快照版本" 行之后新增一条："**源码行号**：文档中形如 `path/file.ts:16-36` 的行号区间严格对应 v2.1.88 快照；快照升级时须以符号名（函数 / 类型 / 常量）重新定位，不要盲信旧行号。"
- Done when：§E 有显式的行号约定说明。
- Verify：`rg -n '源码行号' docs/sdk/references.md` 命中 1 处。
- Stop if：无。

### Task P1d: 验证

Status: `pending`

Step 1:
- Action：`ReadLints` 两个文件，确认无 Markdown 破坏。
- Verify：No linter errors。
- Stop if：有 lint 报错。

## Batch Checkpoint Format

Task P1a–P1c 一次性 Checkpoint；P1d 独立验证 Checkpoint。

## Checkpoint 2026-04-20 执行完成

- 批次：Task P1a → P1b → P1c → P1d
- 状态：
  - P1a `done`：README §7 术语表从 12 条扩为 23 条（新增 11 条：`Goldilocks Zone` / `Durable Scratchpad` / `Agentic Memory` / `Initializer + Coding Agent` / `Autonomy Dial` / `Protocol Adapter` / `Canonical Message IR` / `Canonical Naming` / `Slot` / `MCPB` / `Effective Config Hash`），每条附指向章节
  - P1b `done`：`references.md` §C1 顶部新增"路径简写约定"条目，明确 `restored-src/` 是 `docs/references/claude-code-sourcemap-main/restored-src/` 的省略简称
  - P1c `done`：`references.md` §E "如何验证引用" 新增"源码行号"条目，明确 8 处硬编码行号对应 v2.1.88 快照，升级快照时以符号名重定位
  - P1d `done`：`ReadLints docs/sdk/{README,references}.md` → No linter errors
- 文件变更：
  - `docs/sdk/README.md`（§7 术语表 +11 行）
  - `docs/sdk/references.md`（§C1 +1 行 + §E +1 行）
- Verification：
  - `ReadLints` → pass
  - `rg '路径简写约定' docs/sdk/references.md` → 1 命中
  - `rg '源码行号' docs/sdk/references.md` → 1 命中
  - `rg '\| \*\*Slot\*\*|Durable Scratchpad|Goldilocks' docs/sdk/README.md` → 每条 1 命中
- Prior surprise：初次勘察把行号引用规模估为 93 处，复查后真实规模仅 8 处；相应把"逐行删除"降级为"加脚注"，P1c 工作量显著缩减
- Blockers：无
- Next：
  - 若继续往代码层推进：`docs/plans/2026-04-20-plugin-system-m0m5.md` 独立 plan（依据 12 §12.16）
  - 若继续补文档：考虑新章 `13-contracts-map.md`（把 `docs/sdk/*.md` 与 `contracts/openapi/` / `packages/schema/` 的映射做成一张表）——此为 P2，**不**在本 plan 范围
