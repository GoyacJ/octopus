# ADR-0016 · Programmatic Tool Calling（`execute_code` 元工具）

> 状态：Accepted
> 日期：2026-04-25
> 决策者：架构组
> 关联：ADR-0002（Tool 不含 UI）、ADR-0003（Prompt Cache Hard Constraint）、ADR-0004（Agent/Team 拓扑）、ADR-0009（Deferred Tool Loading）、ADR-0010（Tool Result Budget）、ADR-0011（Tool Capability Handle）、ADR-0014（Plugin Manifest Signer）、`crates/harness-tool.md`、`crates/harness-sandbox.md`、`crates/harness-subagent.md`、`security-trust.md`、`extensibility.md`、`agents-design.md`

## 1. 背景与问题

### 1.1 症状

当前 Octopus 主循环的"工具调用"语义是 **One-Tool-Per-Inference**：模型每次推理只能发出一组 `tool_use`，由 Engine 调度执行后，结果作为 `tool_result` 在下一次推理里再供模型消费。

这个语义在多数场景已经足够好，但有一类高频场景表现拙劣：

| 场景 | 一次任务里典型的"工具往返"次数 |
|---|---|
| 在 8 个候选目录里各跑一次 grep，比对结果 | 8 次推理 + 8 次工具调用 |
| 读取 5 个文件 → 找出 3 个相互引用 → 列出对应符号 | 6~10 次推理 |
| 拉 3 个 URL → 抓取 → 简单 join → 输出 markdown 表 | 4~6 次推理 |

每一次额外的推理：
- 重新发送整个 prompt（即使有 prompt cache 也要走一次 cache hit 计费）
- 重新发送整个 toolset schema（被 ADR-0009 缓解但仍是上下文加载）
- 增加端到端延迟（典型 1.5-3s/轮）
- 模型上下文里堆叠 N 份相似的 `tool_use / tool_result` 对，挤占 budget

### 1.2 行业先例

| 系统 | 做法 | 关键能力 |
|---|---|---|
| Hermes Agent | 内置 `execute_code` 工具：模型生成一段 Python，执行环境内已注入 `read / grep / write_file / search` 等函数，模型一次推理可写"`for d in dirs: print(grep(...))`"的复合调用（HER-008 / HER-014） | 可被 Subagent blocklist 拦截 |
| Claude Code | 通过 SDK `query.programmaticToolCall` 让 SDK 消费方在主 Agent 进程外用宿主语言**预先合成多次工具调用**，结果作为单条 `tool_result` 喂回模型（CC-32 周边） | SDK 内特性 |
| OpenClaw | Channel 插件 `MessagePresentation` 提供"批量消息"组合，但不是 PTC（OC-21） | — |

三家各有差异，但共同思路是**让模型用一次推理发起 N 次工具调用**，把"编排"权力下放给一段受沙箱约束的代码片段。

### 1.3 现状缺口

- `harness-tool §4 内置 Toolset` 没有 `execute_code` 类工具
- `harness-sandbox` 只支持 Shell 沙箱（`LocalSandbox / DockerSandbox / SshSandbox / NoopSandbox`）和 stdio 子进程，没有"代码运行时沙箱"
- `harness-subagent §2.5 Default Policy` 与 ADR-0004 §2.4 的 Blocklist 已经预留了 `execute_code` 名字，但并没有对应实现，处于"占位但无实体"状态——这是历史决议的延续，本 ADR 把实体补上的同时显式确认延续

## 2. 决策

### 2.1 总纲

引入新内置工具 `execute_code`，定位为 **主 Agent 元工具**。它接收一段受限语法的"工具编排脚本"，在隔离的代码沙箱里同步执行，脚本内部仅能调用 SDK 注入的、本 Run 已被审批的、`read-only` 的工具白名单子集。脚本最终产物按 ADR-0010 流式收集 + 预算控制后作为单条 `tool_result` 返回主 Agent。

设计目标（按优先级）：

1. **不破坏 Prompt Cache Hard Constraint（ADR-0003）**：`execute_code` 在 `BuiltinToolset::Default` 中是 `AlwaysLoad`、字面 schema 永久冻结，不参与运行期 schema 漂移
2. **不绕过 Permission Broker（ADR-0007）**：脚本内嵌的每次 tool 调用仍各自走 broker，且 broker 的 `DecisionScope` 计算需带入 "by execute_code" 的 caller 标记（默认按"父层 caller 的当前规则"复用，不"放水"也不"额外加严"）
3. **默认 read-only 嵌入**：默认仅允许嵌入 `Grep / Glob / FileRead / ListDir / WebSearch / ReadBlob / ToolSearch` 七个 read-only built-in 工具
4. **不在 Subagent 可见**：与 ADR-0004 §2.4 / `harness-subagent §2.5` 默认 Blocklist 一致，`execute_code` 默认对 Subagent 不可见
5. **可审计、可重放**：脚本本体进入 `Event::ToolUseRequested.input.script`，每次内嵌 tool 调用都产出独立 `Event::ExecuteCodeStepInvoked`，与外层 `Event::ToolUseCompleted` 形成可追溯链

### 2.2 工具形态

```rust
/// 内置工具，AlwaysLoad；descriptor 字面冻结。
pub struct ExecuteCodeTool;

impl Tool for ExecuteCodeTool {
    fn descriptor(&self) -> &ToolDescriptor { /* 见 §2.3 */ }

    async fn check_permission(
        &self,
        input: &Value,
        ctx: &ToolContext,
    ) -> PermissionCheck {
        // 1. trust_level == Builtin（注册期已校验）
        // 2. ctx.caller_chain 末尾不能是 Subagent（Subagent 装配期已 blocklist，
        //    但运行期仍 fail-closed 双保险）
        // 3. 脚本本体由 PermissionBroker 按 ExecuteCodeScope 决议
        PermissionCheck::AskUser {
            subject: "执行 execute_code 脚本（包含若干内嵌工具调用）".into(),
            detail: Some(/* 脚本头 256 字符摘要 */),
            severity: Severity::High, // ADR-0007 §3.x
            scope: DecisionScope::ExecuteCodeScript {
                script_hash: blake3(&input["script"]),
            },
        }
    }

    async fn execute(&self, input: Value, ctx: ToolContext)
        -> Result<ToolStream, ToolError>
    { /* §2.4 流水线 */ }
}
```

### 2.3 ToolDescriptor 冻结字段

```rust
ToolDescriptor {
    name: "execute_code".into(),
    title: "Programmatic Tool Calling",
    description: include_str!("descriptions/execute_code.md"),
    group: ToolGroup::Meta,
    origin: ToolOrigin::Builtin,
    trust_level: TrustLevel::Builtin,
    properties: ToolProperties {
        is_concurrency_safe: false,         // 串行
        is_readonly: false,                  // 通过嵌入 read-only 工具是 readonly
                                             //    但行为上仍按"破坏性元工具"处理
        is_destructive: false,
        ..
    },
    required_capabilities: bitflags![
        ToolCapability::CodeRuntime,         // 新增（§2.7）
        ToolCapability::EmbeddedToolDispatcher, // 新增（§2.7）
    ],
    provider_restriction: ProviderRestriction::All,
    defer_policy: DeferPolicy::AlwaysLoad,    // ADR-0009 §2
    budget: ResultBudget {
        metric: BudgetMetric::Chars,
        limit: 30_000,
        on_overflow: OverflowAction::Offload, // ADR-0010 §2.2
        preview_head_chars: 2_000,
        preview_tail_chars: 2_000,
    },
    input_schema: include_str!("schemas/execute_code.input.json"),
    output_schema: Some(include_str!("schemas/execute_code.output.json")),
    search_hint: None,                        // 不参与 ToolSearch
}
```

输入 schema 关键字段：

```json
{
  "type": "object",
  "required": ["script"],
  "properties": {
    "script": {
      "type": "string",
      "maxLength": 16384,
      "description": "受限语法的工具编排脚本（详见 §2.5）。"
    },
    "expected_kind": {
      "type": "string",
      "enum": ["text", "json"],
      "default": "text"
    },
    "labels": {
      "type": "object",
      "additionalProperties": {"type": "string"},
      "description": "用于审计与脚本元数据，不参与执行。"
    }
  },
  "additionalProperties": false
}
```

### 2.4 执行流水线

```text
ExecuteCodeTool::execute(input, ctx)
  ↓
[1] script_validator（语法+长度+静态扫描）
       │  失败 → ToolEvent::Error(ScriptInvalid { reason })
       ▼
[2] ctx.cap.code_runtime.spawn(script_handle, sandbox_spec)
       │  spawn 失败 → ToolEvent::Error(SandboxBootFailed { ... })
       ▼
[3] CodeSandbox 内执行 Iter:
       │
       │  ┌── 脚本调用 emb.tool("Grep", { pattern, path }) ──────┐
       │  │  ↓                                                    │
       │  │  EmbeddedToolDispatcher::dispatch                     │
       │  │     ├─ 白名单校验（§2.6）                              │
       │  │     ├─ 转译为合法 ToolUse                              │
       │  │     ├─ 复用 ToolOrchestrator 单次调用流水线（含 Hook）│
       │  │     │   注：Hook 不会被绕过；orchestrator 内置        │
       │  │     │   PreToolUse / PostToolUse / Permission /       │
       │  │     │   ResultBudget 全套                              │
       │  │     ├─ Event::ExecuteCodeStepInvoked                   │
       │  │     └─ 返回 ToolResult 给脚本（已经过 ResultBudget）   │
       │  └────────────────────────────────────────────────────────┘
       │
       │  脚本捕获 stdout / stderr / 抛出异常 / 显式 return
       ▼
[4] CodeSandbox 退出
       │
       ▼
[5] Orchestrator 拼装 final ToolResult：
       │  - 脚本 stdout / 显式 return 进入 ToolResult.text
       │  - 内嵌每个步骤的 (tool_name, args_summary, result_summary, duration)
       │    进入 ToolResult.metadata.steps（用于 UI / 审计）
       │  - 整体走 ResultBudget；超限走 ADR-0010 offload
       ▼
ToolEvent::Final(envelope)
```

**强制约束**：

- 脚本内部 **不得** 反射调用 `execute_code` 自身（递归直接 deny）
- 脚本内部 **不得** 直接发起 model.infer（走嵌入式工具，不走推理）
- 脚本调用嵌入工具时，**每一次** 都仍走主 Engine 的 PermissionBroker；broker 的 dedup gate（`harness-permission §X DedupGate`）会自然吸收"同一脚本里 8 次相同 grep"的重复审批

### 2.5 受限脚本语法（默认实现：Lua-like 子集）

> M0 阶段以 **mini-lua** 子集落地（语法可改，决策点在"受限"而非"具体语言"）。

允许：

- 局部变量、`if/elseif/else`、`for i=1,n` / `for k,v in pairs/ipairs` / `while`
- 字符串 / 数字 / table / boolean / nil 字面量
- 二元/一元算术与比较；逻辑 `and / or / not`
- 函数定义与调用、闭包；递归深度 ≤ 32
- `emb.tool(name, args) -> result`：唯一可见的"宿主调用"
- `emb.json.encode/decode`、`emb.text.lines/split/trim`、`emb.fmt.format`
- `print(...)`：写入脚本 stdout（按 ResultBudget 计费）
- `return value`：脚本结果

禁止：

- I/O：`io.*` / `os.*` / `package.*` / `require`
- 网络：任何与外部 socket/http 直连的库
- 反射：`debug.*` / `getmetatable` / `setmetatable` 对受限内置类型
- 进程操作：`os.execute / popen / exit` 等
- 协程嵌套调用 `emb.tool`（避免 yield 状态泄露）
- 创建无界循环（运行时 instructions 计数器，§2.7 给出限额）

> 选择 mini-lua 而非 Python 的理由见 §3 替代方案 3.2。
> 业务侧若需要扩展语言，需走 ADR + Capability + Sandbox 三联评审，不得在 SDK 默认实现里偷偷加。

### 2.6 嵌入工具白名单（按 q4 决策：默认 read-only built-in）

```rust
pub struct EmbeddedToolWhitelist {
    pub names: BTreeSet<String>,
}

impl EmbeddedToolWhitelist {
    pub const DEFAULT_READONLY_BUILTIN: &'static [&'static str] = &[
        "Grep",
        "Glob",
        "FileRead",
        "ListDir",
        "WebSearch",
        "ReadBlob",
        "ToolSearch",
    ];

    /// 业务可在 `team_config.toml` 中通过 [execute_code.embedded_tools]
    /// 显式扩展（仅 Builtin / AdminTrusted Plugin 工具可加），
    /// 任何 user-controlled / mcp 工具一律拒绝；扩展操作产生
    /// `Event::ExecuteCodeWhitelistExtended` 审计。
    pub fn from_team_config(cfg: &TeamConfig) -> Result<Self, ConfigError>;
}
```

**强制约束**：

- 默认集合 **只含** §2.6 第一处声明的 7 个 read-only 工具，不可在不修改 ADR 的前提下经默认配置扩展为 write 工具
- 业务侧若声明扩展，新加入工具的 `properties.is_destructive` / `is_readonly` 与 `trust_level` 必须满足：`is_destructive == false && trust_level ∈ {Builtin, Plugin{trust: AdminTrusted}}`，否则 `ConfigError::EmbeddedToolNotPermitted`
- 任何写类 / 网络发起类 / 沙箱 shell 类工具（`Bash / FileWrite / FileEdit / SendMessage / WebFetch`）默认禁止嵌入
- 白名单生效作用域是 **每个 Run** 的 `ExecuteCodeContext`，与 Permission rule 的 `RunScope` 同生命周期

### 2.7 Capability 与 Sandbox 扩展

新增两个 `ToolCapability`（落 `harness-contracts §3.4`）：

```rust
pub enum ToolCapability {
    // 既有：SubagentRunner / TodoStore / RunCanceller / ClarifyChannel
    //       UserMessenger / BlobReader / MemdirWriter
    CodeRuntime,
    EmbeddedToolDispatcher,
}
```

`harness-sandbox.md` 新增 §3.5 `CodeSandbox`：

```rust
pub trait CodeSandbox: Send + Sync + 'static {
    fn capabilities(&self) -> CodeSandboxCapabilities;

    async fn run(
        &self,
        script: &CompiledScript,
        ctx: CodeSandboxRunContext,
    ) -> Result<CodeSandboxResult, SandboxError>;
}

pub struct CodeSandboxCapabilities {
    pub language: ScriptLanguage,            // M0 = MiniLua
    pub max_instructions: u64,                // 默认 1_000_000
    pub max_call_depth: u32,                  // 默认 32
    pub max_string_bytes: u64,                // 默认 4 MiB
    pub max_table_entries: u64,               // 默认 65_536
    pub wall_clock_budget: Duration,          // 默认 30s
    pub deterministic: bool,                  // 默认 true（无随机/时钟来源）
}
```

默认实现 `MiniLuaCodeSandbox`：

- 基于编译期嵌入的 mini-lua interpreter（无 dlopen / 无外部 ABI）
- 同步执行（不再 fork 子进程；CPU/instr 计数 + memory 上限通过 interpreter cooperative tick）
- 不暴露 OS / FS / Net 接口
- `EmbeddedToolDispatcher` 以宿主侧 callback 形式注入，脚本只能通过 `emb.tool("...")` 触达

进程沙箱（如 seatbelt / landlock / docker）**不在** `CodeSandbox` 责任面内：脚本本身不发起任何 OS 操作；嵌入工具走的是主 Engine 已存在的沙箱链（如 `Bash` 走 `LocalSandbox`，但 `Bash` 默认不在嵌入白名单里）。

### 2.8 与 Subagent 的关系（强化 ADR-0004 §2.4）

- `execute_code` 在 `harness-subagent::SubagentPolicy::default().blocklist` 中自历史以来已存在；本 ADR **不修改** 默认值，仅显式确认其语义：
  - Subagent 默认 **不可见** 该工具（装配期 `SubagentRunner` 在子集计算时直接剔除 `execute_code`）
  - 即使业务通过 `team_config.toml` 显式 allow 给某 Subagent，运行期 `ExecuteCodeTool::check_permission` 仍要求 `ctx.caller_chain` 不含 Subagent 标记 → 不满足直接 fail-closed `ToolError::DeniedToSubagent`
  - 由"双层闸门"保护：声明（blocklist）与执行（caller_chain 校验）
- 这一双层与 ADR-0004 §2.4 的 `depth_cap` 软/硬双闸是相同模式；语义保持一致

### 2.9 与 Permission / Hook / Result Budget 的协同

| 协同点 | 行为 |
|---|---|
| ADR-0007 PermissionBroker | `execute_code` 自身一次审批（`DecisionScope::ExecuteCodeScript`）；脚本内嵌每次工具调用各走自身 scope（与不走 PTC 时一致） |
| `harness-permission DedupGate` | 同一 Run 内同 `ExecuteCodeScript.script_hash` 重复出现时受 dedup（典型 retry）；脚本内嵌的相同子调用通过既有 dedup 自然命中 |
| ADR-0010 ResultBudget | 脚本输出（stdout + return）走 `BudgetMetric::Chars`；嵌入工具结果各自走自身 `ResultBudget`，不会双重计费 |
| `harness-hook` 5 介入点 | 脚本入口算一次"外层 ToolUse"，走 PreToolUse / PostToolUse；脚本内嵌每次调用通过 Orchestrator 复用全部 5 介入点 |
| ADR-0011 Capability Handle | `CodeRuntime` 与 `EmbeddedToolDispatcher` 由 EngineBuilder 装配；`default_locked()` 矩阵默认仅对 `ToolOrigin::Builtin` 工具开放 |

### 2.10 与 ADR-0003 Prompt Cache 的关系

- `execute_code` `defer_policy = AlwaysLoad`、descriptor 字面冻结、descriptor 出现在 `BuiltinToolset` 第一段（按名字排序后位置稳定）→ 不破坏 prompt cache
- 脚本本体作为 `ToolUse.input.script` 进入对话，参与 prompt cache 的"近 N 条非 system 消息"段；这与一次性发出多个普通 `tool_use` 的 cache 行为同构，无新增风险
- 嵌入工具 schema 不会被注入到 system prompt 任何片段（已存在的 toolset schema 是同一份，未发生热改）

## 3. 替代方案

### 3.1 在主 Agent 主循环里"批量 tool_use"（不走 PTC）

- 主 Agent 已经可以在一次推理里发多个 `tool_use`（concurrent 执行），看似平替
- ❌ 无法表达"先 grep 再以结果为输入跑下一个 grep"这类**有前后依赖**的多步编排
- ❌ 模型上下文里仍要堆 N 份 result，token 占用没有降低
- ❌ 无法表达"如果 step1 命中 0 行就跳过 step2"这类条件分支

### 3.2 用 Python（CPython）作为脚本语言（HER 风格）

- ✅ 模型熟悉度高
- ❌ 子进程沙箱（seccomp / seatbelt / landlock）成本远高于 mini-lua（依赖 OS 特性、内存常驻 ≥ 30MB / 进程、冷启动 ≥ 50ms）
- ❌ Python 标准库面太大，禁用清单维护成本高（`os / subprocess / ctypes / socket / threading / asyncio / multiprocessing / importlib / __builtins__` 都得封）
- ❌ 与 ADR-0011 `CodeRuntime` 期望的"零依赖、可静态裁剪"不符
- ❌ 默认安全收益与 Octopus 嵌入式发布形态（Tauri Desktop / Server）冲突
- 决议：M0 走 mini-lua；后续若有 Python 强诉求，单独立 ADR + 业务 capability provider

### 3.3 走 SDK 侧"客户端 PTC"（CC 风格）

- ✅ 不在 Agent 内核内引入解释器
- ❌ 把"如何编排"耦合到每一种 SDK 实现（CLI / Desktop / IDE 各写一遍）
- ❌ 不支持 Subagent / 自动化 / Server 形态（因为没有 SDK 侧"宿主"可以执行编排）
- ❌ 与"事件可重放"冲突：SDK 侧编排不进入 EventStore

### 3.4 把 PTC 限定为"内置 fan-out 元工具"（如 `bash_batch`）

- ✅ 简单
- ❌ 仅覆盖 fan-out，不覆盖条件分支与依赖
- ❌ 对模型而言仍是"一种特殊形状的 tool_use"，不能解决 1.1 的核心瓶颈

### 3.5 Capability Handle 集合（采纳）

- ✅ 解释器在 SDK 内核内（Rust 静态链接），无外部依赖
- ✅ 嵌入工具走 `EmbeddedToolDispatcher` capability，复用 `ToolOrchestrator` 全部 5 介入点
- ✅ 与 ADR-0011 `default_locked()` 矩阵自然集成
- ✅ Subagent / Plugin 可见性通过 capability 默认锁定 + manifest 显式声明双重保护

## 4. 影响

### 4.1 正向

- 多步工具编排合并为单次推理，端到端延迟从 N×latency 压缩到 1×latency + script_runtime（典型 < 50ms）
- 模型上下文节省：N 份 tool_use/tool_result 合并为 1 份大 envelope（且超限自动 offload）
- 脚本本体进入 EventStore（`Event::ToolUseRequested.input.script`）+ 内嵌步骤独立事件 → 完全可重放、可审计
- 嵌入工具默认 read-only + 默认对 Subagent 不可见 → 信任面收敛；与 ADR-0006 / ADR-0014 不冲突

### 4.2 代价

- 新增解释器（mini-lua）作为 SDK 默认依赖，二进制体积约 +200KB（裁剪后）
- 新增 2 个 `ToolCapability` 与一个 `CodeSandbox` trait
- `harness-permission` 的 `DecisionScope` 增加 `ExecuteCodeScript { script_hash }` 变体
- `event-schema.md` 新增 `ExecuteCodeStepInvoked` 事件
- 模型侧需要短系统提示片段教 LLM 何时优先用 `execute_code`（详见 `agents-design.md`）

### 4.3 兼容性

- v1.8 仅文档级修订，无代码兼容压力
- `feature_flags.md` 新增 `programmatic_tool_calling`（默认 **off**），M0 期可由业务显式启用、灰度评估后改 default-on
- 与既有 `BuiltinToolset` 枚举兼容：`BuiltinToolset::Default` 在 flag on 时自动包含 `execute_code`，flag off 时自动剔除（不破坏字面顺序，因为剔除后剩余顺序仍按名字排序）

## 5. 落地清单（仅文档面）

| 项 | 责任文档 | 说明 |
|---|---|---|
| `ExecuteCodeTool` 描述子（§2.3）+ 流水线（§2.4）+ 嵌入白名单（§2.6） | `crates/harness-tool.md` 新增 §4.7（`Programmatic Tool Calling`）| 紧跟在 §4.6 `BuiltinToolset` 之后 |
| 注册期校验（默认 Subagent blocklist、capability 校验、白名单合法性）| `crates/harness-tool.md` §5 | 增量 |
| `ToolCapability::CodeRuntime / EmbeddedToolDispatcher` | `crates/harness-contracts.md §3.4` | 共享枚举；2 行新增 |
| `CodeSandbox` trait + `MiniLuaCodeSandbox` 默认实现 | `crates/harness-sandbox.md` 新增 §3.5 | 节加在内置实现末尾 |
| `DecisionScope::ExecuteCodeScript { script_hash }` | `crates/harness-contracts.md §3.4.1` 权限决策共享类型 | 增量 |
| `Event::ExecuteCodeStepInvoked` | `crates/harness-contracts.md §3.3` + `event-schema.md §3.5` | 与 ADR-0010 ToolResultOffloaded 同节 |
| Subagent 与 PTC 边界声明 | `crates/harness-subagent.md §2.5`（确认延续）+ `agents-design.md §3.x` | 不修改默认值；只补"延续 + 双层闸门"说明 |
| `agents-design.md` 系统提示模板片段 | `agents-design.md` 新增 §x | 教主 Agent 何时优先选择 `execute_code` |
| `feature-flags.md` `programmatic_tool_calling`（默认 off） | `feature-flags.md` | 增量 |
| `security-trust.md` 嵌入工具调用风险 | `security-trust.md` 新增 §10.x | 与 §10.X Tool Search 安全考量同级 |
| `extensibility.md` PTC 扩展指引 | `extensibility.md` §3.x | 业务可自定义 `CodeSandbox` 必须走 ADR + capability + sandbox 三联评审 |
| `comparison-matrix.md` R-19 回填 | `reference-analysis/comparison-matrix.md` | → ADR-0016 |

## 6. 参考证据

| Evidence ID | 来源 | 要点 |
|---|---|---|
| HER-008 | `reference-analysis/evidence-index.md` | Hermes Agent-level intercepted tools `execute_code` 的存在与 Subagent 黑名单约束 |
| HER-014 | 同上 | `DELEGATE_BLOCKED_TOOLS` 与 PTC 默认对 Subagent 不可见 |
| CC-32 | 同上 | Claude Code 上下文压缩 pipeline 与 PTC 周边的预算/缓存协同 |
| OC-21 | 同上 | OpenClaw `MessagePresentation` 抽象（PTC 不属于此抽象但可对照设计哲学） |
| ADR-0002 | `adr/0002-tool-no-ui.md` | 受限输出（`ToolResult` 不含 UI 负载）的延续 |
| ADR-0004 | `adr/0004-agent-team-topology.md` | Subagent blocklist 历史决议；本 ADR 延续 |
| ADR-0009 | `adr/0009-deferred-tool-loading.md` | `AlwaysLoad` 与 `defer_policy` 的语义共用 |
| ADR-0010 | `adr/0010-tool-result-budget.md` | `ResultBudget` 与 ADR-0010 共享；脚本结果与嵌入步骤结果分别计费 |
| ADR-0011 | `adr/0011-tool-capability-handle.md` | 新增 2 个 `ToolCapability` 与 `default_locked()` 矩阵兼容 |
