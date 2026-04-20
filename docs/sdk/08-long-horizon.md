# 08 · 长时任务（Long-horizon）模式

> "Claude plowed through dozens of sprints over 7 hours and 45 minutes... completing in one go a task that took 30 hours when we manually prompted the model iteratively."
> — [Anthropic · Harness design for long-running application development](https://www.anthropic.com/engineering/harness-design-long-running-apps)

本章解决单个任务需要"跨越多个上下文窗口"的工程挑战。

## 8.1 Why：窗口 ≠ 任务

现实中许多任务：

- 一次性代码库迁移
- 跨几十个文件的重构
- 连续几小时的研究综合
- 游戏代理（Claude Plays Pokemon）

都远超 200k token 窗口。

两种解法：

1. **让窗口容量变大**（模型厂商的活）
2. **让 harness 在多个窗口之间维护一致性**（本章的活）

## 8.2 三大核心模式

### 模式 1：Compaction-based（压缩流）

单一对话，窗口满了就压缩：

```
[full history] → compact() → [summary + recent turns] → 继续
```

**优点**：简单、对模型透明。
**缺点**：关键细节会丢；不适合"跨越多天"。

详见 [`02-context-engineering.md`](./02-context-engineering.md) §2.5。

### 模式 2：Initializer + Coding Agent（启动器流）

来自 [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)：

```
┌───────────────┐
│ Initializer   │ → 写入 init.sh、NOTES.md、CLAUDE.md
│   Agent       │ → 建立 git 初次 commit
└───────┬───────┘
        ▼
┌──────────────────┐       ┌─────────────────────────────────┐
│ Coding Agent #1  │ →→→   │ 读 NOTES.md、改代码             │
│ (fresh context)  │       │ 写 runtime/notes/<session>.md   │
│                  │       │ git commit                      │
└──────────────────┘       └─────────────────────────────────┘
        ↓ 窗口耗尽 / 重置
┌──────────────────┐
│ Coding Agent #2  │ → 读 NOTES.md + runtime/notes/<session>.md → 续作
│ (fresh context)  │
└──────────────────┘
        ... (N 个 Coding Agent 接力)
```

**关键产物**：

- `CLAUDE.md` / `AGENTS.md`：项目级常识（不随 agent 迭代）
- `NOTES.md`：不变的设计决策
- `runtime/notes/<session>.md`：易变的"我做到哪了"（Anthropic 原文叫 `claude-progress.txt`）
- **Git 的 incremental commits**：每步可回滚

**关键操作**：

- Initializer 只做初始化，然后退出（它的上下文不延续）
- Coding Agent 每次**开新窗口**（clear context），只读文件和 git history 恢复进度

> 来源：[Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) §"An Initializer agent" & "A Coding agent"。

### 模式 3：Planner / Generator / Evaluator（GAN 流）

来自 [Harness design for long-running apps](https://www.anthropic.com/engineering/harness-design-long-running-apps)：

见 [`05-sub-agents.md`](./05-sub-agents.md) §5.3。适合开放式创作（UI 设计、full-stack app）。

## 8.3 Context Reset vs Compaction

### 8.3.1 两种做法

| 做法 | 怎么做 | 优点 | 缺点 |
|---|---|---|---|
| **Compaction** | 摘要前半段、保留尾部 | 渐进、不丢当前状态 | 多次后噪声累积 |
| **Context Reset** | 直接扔掉整个窗口，只保留 artifact 文件 | 彻底干净 | 依赖文件记忆完整 |

### 8.3.2 与模型版本的关系

Anthropic 的经验（[Harness design](https://www.anthropic.com/engineering/harness-design-long-running-apps)）：

- **Sonnet 4.5** 有 "context anxiety"：接近上限会提前收工。→ **强烈需要 context reset**
- **Opus 4.5 / 4.6** 无此行为，compaction 足够。→ **context reset 变成可选**

**规范建议**：

- SDK 同时支持两种模式，以 feature flag 控制
- 默认：弱模型用 reset；强模型用 compaction
- 保留 override 接口

## 8.4 Artifact 合约（长时任务的"接力棒"）

跨窗口接力必须依靠**文件产物**。本 SDK 规范化这些 artifact：

### 8.4.1 必备 artifact

| Artifact | 类型 | 内容 | 生命周期 |
|---|---|---|---|
| `CLAUDE.md` / `AGENTS.md` | 静态 | 项目规约、不变的规则 | 跨 session、跨 agent |
| `NOTES.md` | 半静态 | 不变的设计决策 | 长期 |
| `runtime/notes/<session>.md`（进度快照） | 动态 | "我做到哪 / 下一步是啥"（业界常称 `claude-progress.txt`） | 随任务进展每步覆盖 |
| `runtime/todos/<session>.json`（`TodoWrite`） | 动态 | 结构化任务清单 | 每轮可能改 |
| Git history | 动态 | 文件级增量变更 | 永久 |
| Session JSONL（`runtime/events/*.jsonl`） | 动态 | 事件流 | 永久（审计） |

> 为什么改路径：Octopus `AGENTS.md` §Persistence Governance 明确"runtime sessions / todos / events 都在 `runtime/**`"。保留 `NOTES.md` 与 `CLAUDE.md` 的**项目根路径**（属于项目级约束产物），但把**会话级**的 progress / todos 移入 `runtime/**`，与 02 §2.11 保持一致。

### 8.4.2 格式规范

- **`runtime/notes/<session>.md`**（进度快照）：Markdown；段落化：Current state / Last change / Next action / Open questions
- **`runtime/todos/<session>.json`**：严格 schema（见 `TodoWriteTool`）
- **`NOTES.md`**：Markdown；列表式；每条决策带"why / alternatives considered"
- **Git commit message**：语义化（feat/fix/refactor/...）；单个 commit 一个逻辑步骤

### 8.4.3 起手式（Initializer 模板）

```
1. 读取用户需求（1 句 → 10 页展开 by Planner agent）
2. 生成 NOTES.md 写决策
3. 生成 CLAUDE.md 写项目约束
4. 跑 init.sh（装依赖、建 scaffold）
5. git init && git add -A && git commit -m 'init'
6. 写 runtime/notes/<session>.md（进度快照）: "已初始化，下一步: 实现 X"
7. 退出，交给 Coding Agent 持续
```

### 8.4.4 成果物双轨：artifact-ref 风格 ↔ file diff 风格

跨窗口接力的"产物"存在**两条合法路径**，由工具实现方自主选择；SDK / IR 层**不**强制。两条路径共用同一套 UI 意图 IR（见 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.6），以不同 `RenderBlock.kind` 呈现。

| 维度 | artifact-ref 风格（Claude.ai 式） | file diff 风格（Claude Code 式） |
|---|---|---|
| 代表工具 | `create_artifact` · `update_artifact` | `fs_write` · `fs_edit` |
| 存储位置 | `data/artifacts/` + `@octopus/schema::artifact.ts` 元数据（`storage_path` / `content_hash` 入 SQLite） | 工作区真实文件（`$projectRoot/**`）+ Git 历史 |
| 版本追溯 | `/api/v1/projects/{id}/deliverables/{id}/versions`（可回滚单个 artifact） | `git log <path>` / `git revert`（可回滚整次提交） |
| 对话流内形态 | `RenderBlock kind: 'artifact-ref'`（轻量卡片 → `UiArtifactBlock`） | `RenderBlock kind: 'diff'`（内联 before/after → `UiCodeEditor` diff 视图） |
| 跨 turn 引用 | `ArtifactRef.artifactId` 可作为下一工具 input；模型无需重读全文 | 模型以 `fs_read` 重新加载；接力时依赖 git log 读懂历史 |
| 适用场景 | **给用户看的可视成果物**（UI 草图、生成 HTML/React、可视化 SVG、演示 mermaid、长文档） | **工程落地文件**（源代码、配置、脚本、`NOTES.md` / `runtime/notes/` 本身） |

**两轨可同时启用**：模型自主判断——"给用户看的成果物"走 artifact，"工程文件"走 `fs_edit`。桌面已同时具备 `UiArtifactBlock` 与 `UiCodeEditor`（diff 视图），业务层按 IR kind 单点 dispatch 即可。

**反例（要避免）**：

- 把 `fs_edit` 的 diff 也走 artifact-ref 存进 `data/artifacts/` → 工作区文件与 artifact 双头真相源，git 与 artifact 版本分叉。
- 把 artifact 全量内容塞进对话流 `RenderBlock.kind: 'code'` → event log 体积爆炸，Prompt cache 紊乱（违反 C1）。
- 业务页按工具名（而非 `block.kind`）分支渲染 → 违反 14 §14.10 反模式 #1。

> 契约落点见 [`13-contracts-map.md`](./13-contracts-map.md) §13.3（08 行 UIIntent kinds = `artifact-ref` / `diff`）。

## 8.5 Feature List：给长任务的明确完成定义

Anthropic 强调：**长任务需要预先列出可检核项**。

```yaml
features:
  - id: F001
    desc: 用户可以注册
    done_when:
      - POST /api/register 返回 200
      - DB 中出现记录
      - UI 显示成功页
      - e2e 测试通过
  - id: F002
    ...
```

**效果**（Anthropic 数据）：

- 有 feature list：Claude 能一次跑完 30h 等价任务
- 无 feature list：Claude 经常"自我宣告完成"但遗漏需求

> 来源：[Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) §"Pass the feature list in your agent's environment"。

## 8.6 增量 Commit 与可验证性

### 8.6.1 规则

- 每个有意义的步骤一个 commit
- Commit message = 该步骤"做了什么"
- 失败的尝试先 stash / branch，不污染主线

### 8.6.2 为什么

- 可审计（每步可 diff）
- 可回滚（`git reset --hard <sha>`）
- 可继续（下一个 Coding Agent 可 `git log` 理解历史）

## 8.7 Browser/Environment Test Loop

Anthropic 在 full-stack 长任务中强调：

> "Give the agent a browser."

- Playwright MCP 让 Evaluator 看到渲染的真实页面
- 不是"编译通过就算完成"，而是"截图 + 测试 + 验证"

> 来源：[Harness design for long-running apps](https://www.anthropic.com/engineering/harness-design-long-running-apps)。

## 8.8 典型失败模式 + 对策

| 失败 | 表现 | 对策 |
|---|---|---|
| **过早收工** | 说"已完成"但 todo 还剩 5 项 | Feature list + Stop hook + Evaluator |
| **过度重构** | 改了 10 个无关文件 | `NOTES.md` 写明"不要动 X" |
| **忘记做什么** | 跨 session 后 Coding Agent 找不着北 | 强制写 `runtime/notes/<session>.md`（原 `claude-progress.txt`）+ 读取 hook |
| **循环自检** | 一直在 lint fix 而不前进 | Budget 限流 + 进度比较 hook（本 sprint 有实质进展吗？） |
| **Evaluator 滥好人** | 给 Generator 过高评分 | 用**独立上下文**的 Evaluator；标准化评分 rubric |

## 8.9 Octopus 落地约束

- `claude-progress.txt` 对齐到 `runtime/notes/<session>.md`
- `TodoWrite` 对齐到 `runtime/todos/<session>.json`
- 每个 Coding Agent session 开启前必须：
  1. `Read` `NOTES.md` / `runtime/notes/<session>.md` / last 5 commits
  2. `emit_event` 一条 `context_restored` 事件
- Feature list 建议以 JSON / YAML 存于 `docs/plans/YYYY-MM-DD-<topic>.md`（本仓 `AGENTS.md` 已要求计划文档）

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) | Initializer + Coding Agent、artifact 列表、Feature list |
| [Harness design for long-running apps](https://www.anthropic.com/engineering/harness-design-long-running-apps) | Planner/Generator/Evaluator、Context reset vs compaction、Browser loop |
| [Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) | Compaction、note-taking、subagent |
| [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | 长时任务与子代理结合 |
| 本仓 `AGENTS.md` §AI Planning Protocol | docs/plans 模板与生命周期 |
