# 00 · Vibecoding 策略

> 状态：Accepted · 本文是 AI 执行行为的**底线规则**
> 适用对象：所有参与 Harness SDK 实现的 Codex AI 会话与人类 reviewer

---

## 1. 核心命题

> **架构 SPEC 是模板，不是建议。AI 的工作是"把模板搬运成代码"，不是"参考模板再设计"。**

Octopus Harness SDK 的 19 crate / 18 ADR / 10 D 文档已经是高保真规范——所有 trait 签名、struct 字段、enum 变体、错误类型都在 SPEC 中以 Rust 代码形式给出。Vibecoding 模式把这些 SPEC 当作 **AI 的输入模板**，输出是"实现 + 测试 + 文档注释"，不是"重新讨论接口形态"。

---

## 2. 五条铁律

### 铁律 1 · SPEC 即模板（Spec-as-Template）

| 行为 | 允许 | 禁止 |
|---|---|---|
| trait/struct 签名 | **逐字 copy 自 SPEC** | 凭"经验"重命名字段、调整泛型边界 |
| 枚举变体 | 与 SPEC 完全一致 | 漏掉变体、合并变体、改名 |
| 错误类型 | 沿用 `ToolError / SandboxError / ...` 既有族 | 自定义新错误家族 |
| 公开 API | 与 D3 `api-contracts.md` 一致 | 把私有方法误标 `pub` |

**违反后果**：PR 直接 reject，任务卡作废重发。

**例外（SPEC 缺陷应急流程）**：SPEC 自身有歧义或缺失时，AI 必须按以下"卡顿应急"流程处理（实施前评估 P2-4）：

1. **停下编码**——不再产出实现代码
2. **输出 PR draft**（不 merge，仅供 maintainer 看到 AI 当前推断）：
   - PR title：`[SPEC-CLARIFY-REQUIRED] [Mx-Tyy] <SPEC 矛盾点摘要>`
   - PR body 包含：
     - 矛盾点定位（文件路径 + 行号）
     - 候选解 A / 候选解 B 各自实现概要
     - AI 当前倾向（含理由）
     - 影响半径：本卡之外哪些任务卡受波及
   - 仅推送分支 + 开 draft PR，**不要标 ready-for-review**
3. **会话挂起**：在最后输出标记 `[SPEC-CLARIFY-REQUIRED]` 标签 + 候选解列表，等候 maintainer 裁决
4. **绝不在代码里"先实现再补"**

> 治理理由：让 AI 推断显式化 + 可溯源。Maintainer 看 draft PR 比读 chat 信息高效得多；同时 AI 不会陷入 reset 循环。

### 铁律 2 · 一卡一 PR（One Card One PR）

- 每张任务卡 = 一个 git 分支 + 一个 PR
- PR diff 上限 **≤ 500 行**（含测试与文档），超出则任务卡设计失败 → 拆分
- PR 不允许"顺手修复"另一个 crate 的代码（即使确实有 bug，开新任务卡）
- PR 必须含**所有质量闸门通过的截图或日志**（见 `03-quality-gates.md`）

### 铁律 3 · 失败即 Reset（Fail-Reset）

AI 输出未通过 5 道闸门时：

```text
正确做法：git reset --hard HEAD~  → 重写任务卡 prompt → 重新跑 codex
错误做法：手动改 AI 代码使其通过 lint  ← 严禁
```

**理由**：AI 一旦在错误路径上"被人类调试拽通"，下一次会复制错误模式。让它彻底失败、重新开始，比"打补丁"成本低。

**唯一例外**：编译错误是因为 SPEC 本身的 bug（如 trait 签名漏 `async`），按铁律 1 例外路径处理。

### 铁律 4 · 上下文必锚定（Context-Anchoring）

每个 Codex 会话开始时，**必须**：

1. 读取当前任务卡（`milestones/Mx-*.md` 中对应 ID）
2. 读取任务卡 `SPEC 锚点` 字段指向的文件 + 行号片段
3. 读取任务卡 `ADR 锚点` 字段指向的 ADR 全文
4. 读取依赖任务卡的 PR diff（如有）

**禁止**：仅凭任务卡描述"猜"实现细节。详见 `04-context-anchoring.md`。

### 铁律 5 · 测试与实现同 PR（Test-First / Test-Co-PR）

每个 trait 实现 PR **必须**同时包含：

- 至少 1 个 mock 实现（位置：`crate-name/src/mock.rs`，`#[cfg(any(test, feature = "mock"))]` 门控）
- 至少 1 个正向用例（happy path）
- 至少 1 个反向用例（error / boundary）
- contract-test：验证实现满足 trait 文档约束（如 `BroadcastChannel` 的 fan-out 语义）

**位置约定**：

```text
crates/octopus-harness-permission/
├── src/
│   ├── lib.rs
│   ├── broker/
│   │   ├── direct.rs           ← 实现
│   │   └── stream.rs
│   ├── mock.rs                 ← Mock 实现
│   └── ...
├── tests/                      ← integration 用例
│   ├── direct_broker.rs
│   ├── stream_broker.rs
│   └── contract.rs             ← contract-test
└── Cargo.toml
```

---

## 3. AI 工作流

### 3.1 单任务卡执行流程

```text
┌──────────────────────────────────────────────────────────────────┐
│  1. 派发：人类 / scheduler 选择一个 ready 状态的任务卡             │
└──────────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────┐
│  2. Codex 启动：装载任务卡 prompt 模板（`02-task-template.md`）    │
│     - 读取 SPEC 锚点 + ADR 锚点（强制）                            │
│     - 读取依赖任务卡的产物清单                                     │
└──────────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────┐
│  3. 实现：在 git worktree（main 分支）中开新 branch                 │
│     - 从 main 创建 feature/Mx-Tyy-<slug>                           │
│     - 严格按任务卡"预期产物"清单生成文件                            │
│     - 执行铁律 1（逐字搬运 SPEC）+ 铁律 5（测试同 PR）              │
└──────────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────┐
│  4. 自检：本地跑 5 道闸门（`03-quality-gates.md`）                  │
│     - cargo fmt --check                                            │
│     - cargo check / clippy / test                                  │
│     - cargo deny check（feature 矩阵）                             │
│     - SPEC 一致性自检（grep trait 签名）                           │
└──────────────────────────────────────────────────────────────────┘
        │
   ┌────┴────┐
   通过      不通过 → 铁律 3：reset + 重派
   │
   ▼
┌──────────────────────────────────────────────────────────────────┐
│  5. PR：开 PR 到 main，附 5 道闸门日志                              │
│     - PR title: [M2-T03] octopus-harness-permission · DirectBroker │
│     - PR description: 按 `02-task-template.md` PR 模板填写         │
└──────────────────────────────────────────────────────────────────┘
        │
        ▼
┌──────────────────────────────────────────────────────────────────┐
│  6. 评审 + 合入                                                    │
│     - 人类 reviewer 检查 SPEC 一致性 + 闸门日志                    │
│     - 合入后更新 `01-roadmap.md` 进度块                            │
└──────────────────────────────────────────────────────────────────┘
```

### 3.2 并行任务卡的协调

- **可并行**：任务卡 `依赖` 字段为空交集（如 M2 五原语彼此正交）
- **冲突预防**：每个任务卡声明"可能修改的根目录文件"清单（如 `Cargo.toml workspace.members`）；同清单的任务卡不得并行
- **合并顺序**：先合 `Cargo.toml workspace` 类基础设施 PR，再合具体 crate PR

### 3.3 跨任务卡的"上下文遗忘"问题

Codex 默认无跨会话记忆。为防止"下一卡忘了上一卡的约定"：

| 信息 | 承载位置 |
|---|---|
| 全局规则 | `00-strategy.md`（本文）+ `04-context-anchoring.md` |
| 当前里程碑约定 | `milestones/Mx-*.md` 头部"里程碑级注意事项" |
| 任务卡之间的契约 | 每张任务卡的"对外契约"段（其他卡可引用） |
| 已完成卡的产物 | PR title 中的任务卡 ID + 进度块表格 |

---

## 4. 反模式（禁止）

| 反模式 | 后果 | 替代方案 |
|---|---|---|
| **AI 自行"扩展"接口** | 架构静默偏离 | 按铁律 1，强制走 ADR |
| **PR 跨多个 crate** | 评审困难、回滚成本高 | 拆 PR |
| **TODO/FIXME 入主分支** | 隐性技术债 | 开新任务卡补做，PR 不允许遗留 TODO |
| **测试 mock 与生产共用代码路径** | 测试不可信 | mock 加 `#[cfg]` 门控，禁止 prod 引入 |
| **任务卡内容 < 100 字** | AI 上下文不足 | 补 SPEC 锚点 + 验收命令 |
| **任务卡 prompt 自说自话** | AI 偏离 SPEC | 必引 SPEC + ADR 行号 |
| **跳过 contract-test** | 接口语义不可证 | 强制每 trait 有 contract-test |
| **绕过 cargo deny 例外登记** | feature 依赖图失控 | 必须在 D2 §3.7 + §10 登记 |
| **直接读 D 文档生成代码** | 文档可能过时 | 以 SPEC + ADR 为准（README §0 已声明） |

---

## 5. AI 角色边界

| 决策 | 由谁负责 |
|---|---|
| trait 签名定义 | **架构层（D 文档 + ADR）**，AI 不参与 |
| 错误枚举变体 | 架构层（contracts crate SPEC），AI 不增减 |
| 算法选型（如排序、HashMap vs BTreeMap） | **AI 自决**（性能要求由 SPEC 给出，但实现选型 AI 决定）|
| 模块内部组织（私有 fn / 内部 struct）| **AI 自决** |
| 测试数据 fixture | **AI 自决**（但不得走旁路） |
| 文档注释（`///`）| **AI 自决**（但必须引用 SPEC 章节）|
| Cargo features 的最终启用矩阵 | **架构层**（feature-flags.md），AI 不动 |
| Workspace 拓扑（成员列表）| **架构层 + 本 Plan**，AI 不动 |

---

## 6. 与既有治理的对齐

| 治理点 | 本 Plan 的态度 |
|---|---|
| `AGENTS.md` 仓库根规则 | 全部继承（worktree 从 main、docs/AGENTS 优先级、persistence 治理）|
| `docs/AGENTS.md` 文档规则 | 继承（plan 不承载 normative rule）|
| `docs/architecture/harness/` v1.8.1 SPEC | **唯一架构真相源**（SPEC > ADR > overview > CHANGELOG）|
| `contracts/openapi/` | 不涉及（SDK 不是 HTTP API）|
| `cargo-deny / cargo-depgraph` CI | M0 起强制接入 |

---

## 7. 紧急熔断

如下情况触发，**立即暂停所有 Codex 会话**，召开人类评审：

1. AI 连续 3 张任务卡触发"铁律 3 reset"
2. AI 在 PR 中提出 ADR 修订建议
3. SPEC 与既有 `runtime/events/*.jsonl` / `data/main.db` 治理出现实质冲突
4. 上游依赖（`tokio` / `serde` / `anthropic-tokenizer`）主版本变化
5. 评审报告（`audit/`）出现新一轮 P0 问题

---

## 8. 索引

- **任务卡模板** → [`02-task-template.md`](./02-task-template.md)
- **质量闸门** → [`03-quality-gates.md`](./03-quality-gates.md)
- **上下文锚定** → [`04-context-anchoring.md`](./04-context-anchoring.md)
- **路线图** → [`01-roadmap.md`](./01-roadmap.md)
