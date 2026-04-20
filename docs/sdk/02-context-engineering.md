# 02 · 上下文工程（Context Engineering）

> "Context engineering is the art and science of curating what will go into the limited context window from the universe of possible information."
> — [Anthropic · Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)（2025-09）

本章是本 SDK 最核心的一章。**Token 是稀缺资源**；每一 token 都在消耗 Transformer 的有限注意力预算。

## 2.1 为什么"上下文窗口大"不够用

### 2.1.1 Context Rot

经验数据（[Chroma Research "Context rot: How increasing input tokens impacts LLM performance", 2025](https://research.trychroma.com/context-rot)）：

- 模型性能随输入 token 增加**非线性下降**
- 即便仍在官方上下文长度内，准确率也在衰减
- 没有模型免疫（Sonnet 4.5、Opus 4、GPT-4/5、Gemini 2.5 均表现）

### 2.1.2 Attention Budget

Transformer 每新 token 与全部先前 token 建立 N² 成对关系；n 越大，学到的关系密度越稀。因此：

- **长上下文 ≠ 高信息密度**
- Harness 的任务是**最大化单位 token 的"信噪比"**

> 来源：[Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §"Why context engineering matters"。

## 2.2 上下文的四种操作

引用 LangGraph / Anthropic 社区普遍采用的分类（本 SDK 直接使用）：

| 操作 | 含义 | 典型手段 |
|---|---|---|
| **Write** | 写到"外部记忆"而不是塞进窗口 | `TodoWrite`、`NOTES.md`、Agent Skills 笔记、外部 DB |
| **Select** | 按相关度**进**窗口 | 语义检索（RAG）、`Grep`/`Glob` 工具、MCP `list_resources` |
| **Compress** | 让同样语义占更少 token | Compaction（摘要）、清理工具输出、截断、字段选择 |
| **Isolate** | 把上下文**切开**不互相污染 | Subagent（独立上下文窗口）、多会话、Skill 沙箱 |

> 来源：[Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §"Context engineering for long-horizon tasks"。

## 2.3 系统提示词（System Prompt）设计

### 2.3.1 Goldilocks Zone

两种失败模式：

- **过度工程**：把业务规则/分支写死，提示词>50KB。结果：新模型变笨（被强约束）、维护地狱。
- **过度模糊**：只给一句"你是一个有用的 AI"。结果：模型靠启发式瞎蒙。

正确姿态：

> "The right altitude: specific enough to guide behavior effectively, yet flexible enough to provide strong heuristics that guide behavior."

### 2.3.2 推荐结构（XML + Markdown）

```xml
<role>
  你是 Octopus 的 … 角色名。职责：…。非目标：…。
</role>

<tools_guidance>
  何时用 Read/Grep/Glob（"just-in-time"），何时用 WebSearch，
  何时调 Subagent，何时把信息写入 NOTES.md。
</tools_guidance>

<process>
  默认流程：Explore → Plan → Implement → Verify。
  遇到歧义：用 AskUserQuestion 而非硬猜。
</process>

<safety>
  - 修改代码前必须先 Read
  - 凡涉及凭据/Secrets，调用 hooks:approve-credentials
</safety>

<output>
  Markdown；代码引用用 fenced code + 文件路径；...
</output>
```

**来源**：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Set up CLAUDE.md"；Hermes `agent/prompt_builder.py`。

### 2.3.3 避免"kitchen sink 提示词"

反模式：

- 把所有工具的详尽说明塞进 system prompt（工具说明应该在 tools schema 里）
- 复杂条件逻辑（if-else 嵌套 3 层以上）
- 不可验证的"鼓励语"（"请非常仔细地"）

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Avoid failure patterns"。

## 2.4 Just-in-Time Context Retrieval

### 2.4.1 思路

**不要把所有可能相关的信息提前塞进上下文**，而是：

- 在初始上下文里只保留**轻量引用**（文件路径、URL、query）
- 需要时让模型**主动**调用工具拉取

### 2.4.2 对比

| 策略 | 特点 | 适用 |
|---|---|---|
| **Upfront embedding / RAG** | 启动前向量检索 top-K 塞进 context | 静态知识库问答、延迟敏感 |
| **Just-in-time retrieval** | 模型通过 `Read`/`Grep`/`Glob` 按需取 | 大代码库、长任务、变化频繁的数据 |
| **Hybrid** | 先给一小撮高置信度的，其余按需 | 多数真实场景 |

Claude Code 是 JIT 的标杆：默认只暴露 `Read`/`Glob`/`Grep`/`WebSearch`，不做 embedding。**这和 Anthropic 的官方推荐一致**。

> 来源：[Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §"Just-in-time context retrieval"。

### 2.4.3 "Progressive disclosure" 的其他形式

- **File reference** 代替 "贴全文"。
- **`tool_search` meta-tool**：不预载所有 MCP 工具定义，模型先搜索"我需要什么工具"，再加载。Claude.ai 内部实现了此能力。
  > 来源：[Claude Hidden Toolkit](../references/Claude_Hidden_Toolkit.md)（docs/references 下）
- **Skill 按需激活**：`SKILL.md` 的 `description` 字段告诉模型"什么时候使用这个 skill"，只有被触发时才加载。
  > 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Create skills"。

## 2.5 Compaction（压缩）

### 2.5.1 定义

把对话前半段（或超过阈值的部分）**摘要**，然后在摘要基础上开始新窗口。

### 2.5.2 何时触发

- Token 水位 ≥ threshold（Claude Code 默认 90% warning / 95% hard）
- `prompt_too_long` 错误
- 显式 `/compact` 命令
- 进入长时任务的新阶段（"sprint" 切换）

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Aggressive context management"；Claude Code `restored-src/src/services/compact/*`。

### 2.5.3 摘要什么 / 保留什么

| 摘要（→ 丢弃详情） | 保留（→ 原样） |
|---|---|
| 几十轮的 bug 调试 flow | 关键架构决策 |
| 被后续 override 的早期方案 | 活跃的文件修改记录 |
| 冗长的 tool 输出（尤其 stderr/stdout） | 未解决的错误与 TODO |
| 重复的 Grep/Read 结果 | 用户明确说"记住这个" 的内容 |

Claude Code 的 `compact()` 会给模型一段"总结规则"的 prompt 让它自己做 summary（self-compact）。

### 2.5.4 两个弱化形式

- **Tool-result clearing**：只清旧工具输出，不动消息。成本低，语义不变。
  > 来源：Anthropic API beta feature "context management" + LangChain 社区惯例。
- **Message windowing / sliding window**：保留最近 N 条，更早的丢弃。简单但语义损失大。

### 2.5.5 Hooks: `PreCompact` / `PostCompact`

允许在压缩前后注入逻辑（例如把决策摘要写到 `NOTES.md`）。

> 来源：[Claude Agent SDK - Hooks](https://docs.claude.com/en/api/agent-sdk/hooks)。

## 2.6 结构化笔记 / Agentic Memory

### 2.6.1 思路

让代理**主动**把"关键信息"写到上下文窗口**外**的持久载体，下次用 tool 读回来。

### 2.6.2 载体类型

| 载体 | 用途 | 来源 |
|---|---|---|
| `TodoWrite` → `runtime/todos/<session>.json` | 当前任务清单 + 状态（路径由 Octopus `AGENTS.md` §Persistence Governance 规定） | [Claude Agent SDK 概览](https://docs.claude.com/en/api/agent-sdk/overview)；Claude Code `restored-src/src/tools/TodoWriteTool/*` |
| `NOTES.md`（项目根） | 不变的设计决策；长期保留 | [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) |
| `runtime/notes/<session>.md` | 会话级进度快照；每步覆盖（业界常称 `claude-progress.txt`）；路径与 08 §8.4.1 / §8.9 及本仓 `AGENTS.md` §Persistence Governance 对齐 | 同上 |
| Agent Skills `SKILL.md` | 可复用的"经验" | [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Create skills" |
| MCP `memory` / Claude 内置 `memory` 工具 | 细粒度键值/语义记忆 | [Claude Hidden Toolkit](../references/Claude_Hidden_Toolkit.md) |
| SQLite session store | 全量事件 + FTS 索引 | Hermes `hermes_state.py`；本仓 `data/main.db` |
| Learning loop skills（Hermes） | 代理把教训沉淀成 skill | Hermes README "Self-improving" |

### 2.6.3 实例：Claude Plays Pokemon / Agents.md 规约

官方博客提到：代理在长任务中维护一份 `agents.md` 或 `NOTES.md` 的效果显著提升长时一致性。本 SDK 把这种模式命名为 **Durable Scratchpad**，给它一等工具地位。

> 来源：[Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §"Structured note-taking"。

## 2.7 Sub-agent Architecture（上下文隔离）

### 2.7.1 隔离的本质

Subagent 有自己的上下文窗口 → 其巨量中间 token 不污染主代理。

### 2.7.2 信息压缩边界

主代理 ↔ 子代理之间**只传摘要**：

- 主 → 子：明确的任务 + 必要的初始上下文
- 子 → 主：**高密度的结论**（不是过程日志）

### 2.7.3 典型模式

- **Research**：lead agent 派多个 research subagent 并行搜索，返回 findings 摘要。
  > 来源：[Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system)
- **Code Review**：审阅 subagent 专门读大量代码后返回"问题清单"。
  > 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Set up subagents"
- **Evaluator**：生成器-评估器模式的 Evaluator 跑 Playwright 看页面后返回"craft/design/originality/functionality"四维评分。
  > 来源：[Harness design for long-running apps](https://www.anthropic.com/engineering/harness-design-long-running-apps)

详细见 [`05-sub-agents.md`](./05-sub-agents.md)。

## 2.8 工具定义的 Token 成本

工具 schema 本身占 context。Anthropic 报告：

- 单个 MCP server 完整定义可达 **2–10k tokens**
- 50+ 个 MCP 工具合计可以 **塞满一半上下文**

### 2.8.1 缓解策略

1. **只开你真的会用的**（Claude Code CLAUDE.md 约定每次 session 列出当前启用工具）
2. **`tool_search` meta-tool**：按需加载
3. **Code-execution MCP**：把工具暴露为代码 API，`import` 语义替代 tool schema
   > 来源：[Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp)
4. **命名空间 + 模糊前缀搜索**：减少冲突、降低说明复杂度
   > 来源：[Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) §"Namespace your tools"

## 2.9 示例（Examples / Few-shot）

### 2.9.1 有效示例的特征

- 覆盖**边界情况**（错误输入、空结果、歧义输入）
- 展示**期望的思考链**（短小的 `<thinking>`，不是长篇论述）
- 数量**少而精**：3–5 个好示例 > 20 个平庸示例

> 来源：[Effective context engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) §"Tools and examples"。

### 2.9.2 位置

- System prompt 内（静态）：通用模式
- User/assistant 轮次间（动态）：当轮的具体参考

## 2.10 Prompt Caching 友好性

**提示词的任何中途变更会失效整条 cache 前缀。**

规范要求：

1. 工具顺序确定性（map 遍历必须排序）
2. 不在对话中间插入/删除/修改历史 turn
3. Compaction 结束后新窗口**从 summary 开始**，建立新 cache 前缀
4. 把"频繁变动的小信息"（例如时间戳）放到末尾或 `cache_control` 之外

> 来源：[Anthropic Prompt Caching Docs](https://docs.claude.com/en/docs/build-with-claude/prompt-caching)；Hermes `AGENTS.md` §"Prompt Caching Must Not Break"；OpenClaw `CLAUDE.md` §"Prompt Cache Stability"。

## 2.11 对 Octopus 的落地约束

- **所有 Just-in-Time 工具必须以文件路径为第一公民**（因为 Octopus 的持久化层级是 config + main.db + blobs + JSONL，文件路径是跨层的通用标识符）
- **`TodoWrite` 强制写入 `runtime/todos/<session>.json`**（可被 UI + resume 复用）
- **Compaction 事件必须入 `runtime/events/*.jsonl`**（符合本仓 `AGENTS.md` 的 append-only 审计规则）
- **Runtime config 快照在 session 开始时写入事件流**（`config_snapshot_id` + `effective_config_hash`），见本仓 `AGENTS.md` §"Config snapshot rules"

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents) | 全章理论基础 |
| [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) | 提示词装配、Compaction 实践 |
| [Multi-agent research system](https://www.anthropic.com/engineering/multi-agent-research-system) | Subagent 隔离理由 |
| [Effective harnesses for long-running agents](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) | 长时任务记忆 |
| [Harness design for long-running apps](https://www.anthropic.com/engineering/harness-design-long-running-apps) | 上下文 anxiety / reset |
| [Writing tools for agents](https://www.anthropic.com/engineering/writing-tools-for-agents) | Token 成本、命名空间 |
| [Code execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) | 用代码替代工具定义 |
| [Chroma Research · Context Rot](https://research.trychroma.com/context-rot) | Context rot 实验证据 |
| Claude Code restored src `services/compact/*` | Compaction 实现参考 |
| Hermes `agent/context_compressor.py`, `agent/prompt_builder.py` | 压缩 / 提示装配实现 |
| [Claude Hidden Toolkit](../references/Claude_Hidden_Toolkit.md) | `tool_search` meta-tool 证据 |
