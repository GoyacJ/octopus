# ADR-011 · Tool Capability Handle：Agent 级工具的能力解耦

- **状态**：Accepted
- **日期**：2026-04-25
- **决策者**：架构组
- **影响范围**：`harness-contracts` / `harness-tool` / `harness-subagent` / `harness-engine` / `harness-session` / `harness-permission` / `harness-team`

## 0. 术语对齐（避免命名漂移）

ADR 标题中的 **"Capability Handle"** 是口语化描述，对应的 Rust 类型为：

| 概念 | Rust 类型 | 位置 |
|---|---|---|
| Capability 标识枚举 | `enum ToolCapability` | `harness-contracts::capability` |
| Capability 各项窄接口 | `trait SubagentRunnerCap` / `TodoStoreCap` / `EventEmitterCap` / ...（共 7 个） | 同上 |
| Capability 注册中心 | `struct CapabilityRegistry` | `harness-contracts::capability` |
| Tool 取用入口 | `ToolContext::capability::<C>() -> Result<Arc<C>, ToolError::CapabilityMissing>` | `harness-tool::context` |

**禁止使用**：`ToolCapabilityHandle`（无此类型；早期讨论稿术语，现已收敛）。
**禁止使用**：`ToolCap`（与窄接口的 `*Cap` 后缀冲突）。
本文及所有 SPEC 引用统一使用上表名称。

---

## 1. 背景与问题

### 1.1 症状

`harness-tool.md` 当前给 `AgentTool`（子代理委派）/ `TaskStopTool`（停止子任务）/ `TodoTool`（写 TODO）等"Agent 级工具"实现时，需要直接持有：

- `Arc<dyn SubagentRunner>`（spawn / cancel 子代理）
- `Arc<TodoStore>`（修改 Run 内 TODO 表）
- `Arc<dyn EventEmitter>`（发未来计划的活动事件）
- 以及未来可能的 `Arc<dyn ClarifyChannel>`（向用户提问）/ `Arc<dyn UserMessenger>`（异步发消息）

这导致两类问题：

| 问题 | 表象 |
|---|---|
| **`Tool` 实现耦合 Engine 内部** | 每新增一个 Agent 级工具就要扩展 `ToolContext`，最终 `ToolContext` 退化成"上帝对象" |
| **不可在测试环境单独启动** | 单测一个 `BashTool` 也要拼出 `SubagentRunner` 等无关 Mock |

Hermes Agent 用"代理级工具"在主循环里**前置拦截**绕过了这个问题（`AGENT_TOOLS = {"todo_write", "spawn_subagent", ...}`，主循环识别后直接走代码分支不进 Tool 调用），但这种做法等于**把 Tool trait 撕成两半**，既要分类维护又破坏 ADR-002 "tools 不感知 UI" 的统一性。

### 1.2 现状缺口

- `harness-tool.md` 里的 `ToolContext` **没有**正式条目化的"我能拿到什么、不能拿到什么"；
- `harness-subagent.md §SubagentRunner` 是 trait，但**没有**统一的"Agent 级工具如何获取它"的入口；
- 没有定义"工具声明它需要哪些 Capability" 的注册时校验，导致循环依赖问题（例如某个 plugin tool 偷偷依赖了 `SubagentRunner`，但 plugin trust level 不允许）。

### 1.3 行业先例

| 系统 | 做法 |
|---|---|
| Claude Code | 把"agent 级"动作做成专用 tool（`Task`），直接通过 React Context (`useToolHooks`) 注入 capability，但 capability 集合是**写死在编译期**的 |
| OpenClaw | 工具与 Engine 之间隔了 Channel，capability 通过 RPC 暴露；overhead 高但解耦彻底 |
| Cursor Agent | 公开 `tool_call_metadata.capabilities`，但仅供观测，不参与解耦 |

我们的目标是取**Claude Code 的清晰边界 + OpenClaw 的可替换性**，但不引入 RPC overhead。

## 2. 决策

### 2.1 总纲

在 `harness-contracts` 引入 **`ToolCapability`** 作为命名空间一等公民；`Tool` 在 `descriptor()` 阶段**声明**自己需要哪些 capability；`ToolRegistry` 在注册时**校验**Trust Level 是否允许；`ToolOrchestrator` 在调用时通过 **`CapabilityRegistry`** 动态注入对应的 `dyn Trait` Handle。

### 2.2 `ToolCapability`：命名空间

```rust
/// 一个 Tool 在执行期可能依赖的高权限子系统能力。
/// 注册时由 ToolDescriptor.required_capabilities 声明。
/// 运行时由 ToolContext.capability::<T>() 借用。
#[non_exhaustive]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ToolCapability {
    /// 拿到 SubagentRunner（spawn/cancel 子代理）
    SubagentRunner,
    /// 操作 TodoStore（仅本 Run 可见）
    TodoStore,
    /// 中断当前 Run / 子代理（TaskStopTool 用）
    RunCanceller,
    /// 发起 Clarify（结构化问答），见 P0 内置工具 ClarifyTool
    ClarifyChannel,
    /// 异步向用户/外部 IM 发送消息（SendMessageTool 用）
    UserMessenger,
    /// 访问 BlobStore 读取 ToolResultOffloaded 落盘内容（read_blob 用）
    BlobReader,
    /// 触发 hook 链路（特殊场景；通常不开放给 plugin）
    HookEmitter,
    /// 访问 Skill 仓库（Skill 调用工具时使用）
    SkillRegistry,
    /// 业务自定义能力（注册时附带 Trust Level 矩阵）
    Custom(&'static str),
}
```

每个 capability 对应一个 trait（**这些 trait 写在 contracts crate**，仅做接口承诺，不含实现）：

```rust
pub trait SubagentRunnerCap: Send + Sync {
    fn spawn(&self, spec: SubagentSpec, parent: ParentContext)
        -> BoxFuture<'static, Result<SubagentHandle, SubagentError>>;
}

pub trait TodoStoreCap: Send + Sync {
    fn append(&self, run_id: RunId, item: TodoItem) -> Result<(), HarnessError>;
    fn list(&self, run_id: RunId) -> Vec<TodoItem>;
    fn update(&self, run_id: RunId, id: TodoId, patch: TodoPatch) -> Result<(), HarnessError>;
}

pub trait RunCancellerCap: Send + Sync {
    fn cancel_run(&self, run_id: RunId, reason: CancelReason) -> Result<(), HarnessError>;
}

pub trait ClarifyChannelCap: Send + Sync {
    fn ask(&self, prompt: ClarifyPrompt)
        -> BoxFuture<'static, Result<ClarifyAnswer, HarnessError>>;
}

pub trait UserMessengerCap: Send + Sync {
    fn send(&self, msg: OutboundUserMessage)
        -> BoxFuture<'static, Result<(), HarnessError>>;
}

pub trait BlobReaderCap: Send + Sync {
    fn read(&self, run_id: RunId, blob_id: BlobId, range: Option<ByteRange>)
        -> BoxFuture<'static, Result<Bytes, HarnessError>>;
}
```

> 注：`SubagentRunnerCap` 与 `harness-subagent.md` 的 `SubagentRunner` 关系是**前者是后者的 contracts 投影**（subagent crate 实现 `SubagentRunner`，并在 `CapabilityRegistry` 注册时 wrap 为 `Arc<dyn SubagentRunnerCap>`）。这样 `harness-tool` crate 不需要依赖 `harness-subagent`。

### 2.3 `ToolDescriptor.required_capabilities`

```rust
pub struct ToolDescriptor {
    pub name: &'static str,
    pub display_name: &'static str,
    pub schema: schemars::schema::RootSchema,
    pub properties: ToolProperties,
    pub trust_level: TrustLevel,
    /// 该 Tool 在执行期需要使用的 Capability 集合。
    /// 在 ToolRegistry::register 时被校验。
    pub required_capabilities: &'static [ToolCapability],
    /// ……（其他字段见 harness-tool.md §2.2）
}
```

### 2.4 注册时的 Trust × Capability 矩阵

```rust
pub struct CapabilityPolicy {
    /// 哪些 Trust Level 可以请求该 capability
    matrix: HashMap<ToolCapability, BitSet<TrustLevel>>,
}

impl CapabilityPolicy {
    pub fn default_locked() -> Self {
        use ToolCapability::*;
        use TrustLevel::*;
        let mut m = HashMap::new();
        m.insert(SubagentRunner,  bitset![BuiltIn, AdminTrusted]);
        m.insert(TodoStore,       bitset![BuiltIn, AdminTrusted, UserControlled]);
        m.insert(RunCanceller,    bitset![BuiltIn]);
        m.insert(ClarifyChannel,  bitset![BuiltIn, AdminTrusted]);
        m.insert(UserMessenger,   bitset![BuiltIn, AdminTrusted]);
        m.insert(BlobReader,      bitset![BuiltIn, AdminTrusted, UserControlled]);
        m.insert(HookEmitter,     bitset![BuiltIn]);
        m.insert(SkillRegistry,   bitset![BuiltIn, AdminTrusted]);
        Self { matrix: m }
    }
}
```

`ToolRegistry::register(tool)` 流程：

```text
1. 取 desc.required_capabilities × desc.trust_level
2. 任一 cap 的 trust 不在矩阵允许集 → 返回 RegistrationError::CapabilityNotPermitted
3. 否则注册成功，并缓存 tool→caps 映射用于 dump_audit()
```

这与 ADR-006（Plugin Trust Levels）形成 **trust × capability 的二维门禁**。

### 2.5 `CapabilityRegistry` 与 `ToolContext::capability`

```rust
pub struct CapabilityRegistry {
    inner: HashMap<ToolCapability, Arc<dyn Any + Send + Sync>>,
}

impl CapabilityRegistry {
    pub fn install<T: ?Sized + Send + Sync + 'static>(
        &mut self,
        cap: ToolCapability,
        impl_: Arc<T>,
    );

    pub fn get<T: ?Sized + Send + Sync + 'static>(
        &self,
        cap: ToolCapability,
    ) -> Option<Arc<T>>;
}
```

`ToolContext` 持有 `Arc<CapabilityRegistry>`，并暴露便捷方法：

```rust
impl ToolContext {
    pub fn capability<T: ?Sized + Send + Sync + 'static>(
        &self,
        cap: ToolCapability,
    ) -> Result<Arc<T>, ToolError> {
        self.cap_registry.get::<T>(cap).ok_or(ToolError::CapabilityMissing(cap))
    }
}
```

工具中典型调用：

```rust
async fn execute(&self, ctx: &ToolContext, args: Self::Args) -> Result<ToolStream> {
    let runner: Arc<dyn SubagentRunnerCap> =
        ctx.capability(ToolCapability::SubagentRunner)?;
    let handle = runner.spawn(args.spec, ctx.parent_for_subagent()).await?;
    /* … */
}
```

### 2.6 Engine 装配期注入

`harness-engine` 在初始化 Session 时构造 `CapabilityRegistry`：

```rust
let mut caps = CapabilityRegistry::default();
caps.install::<dyn SubagentRunnerCap>(ToolCapability::SubagentRunner, subagent_cap);
caps.install::<dyn TodoStoreCap>(ToolCapability::TodoStore, todo_cap);
caps.install::<dyn RunCancellerCap>(ToolCapability::RunCanceller, run_canceller);
/* … */
let cap_registry = Arc::new(caps);

let ctx_factory = ToolContextFactory::new(cap_registry, /* ... */);
```

这样 **Tool 实现不再 import Engine 内部模块**，只依赖 `harness-contracts` 的 trait。

### 2.7 测试支持：`MockCapability`

contracts crate 配套 `harness-contracts/testing` feature gate 提供：

```rust
pub struct MockCapabilityRegistry { /* ... */ }
impl MockCapabilityRegistry {
    pub fn with_subagent_runner(self, runner: impl SubagentRunnerCap + 'static) -> Self;
    pub fn with_todo_store(self, store: impl TodoStoreCap + 'static) -> Self;
    /* ... */
    pub fn build(self) -> Arc<CapabilityRegistry>;
}
```

工具单测仅需注入它需要的 cap，其它一律省略。

## 3. 不在本 ADR 范围

- **远程 Capability**（capability 通过 IPC/RPC 提供）：本 ADR 仅定义 in-process trait；远程化由后续 ADR 决定。
- **动态 Capability 协商**（运行期热插拔）：保持注册即冻结的简单语义。
- **Plugin 自定义 Capability 申报**：使用 `ToolCapability::Custom("plugin-foo:my-cap")` 占位；矩阵由插件清单 + 管控审核维护，本 ADR 不展开。

## 4. 参考 / 证据

> 编号约定与 `docs/architecture/reference-analysis/evidence-index.md` 对齐。

| Evidence ID | 来源 | 引用片段 |
|---|---|---|
| CC-08 | `reference-analysis/evidence-index.md` L123 · `claude-code-sourcemap-main/restored-src/src/Tool.ts:182-299` | `createSubagentContext` 通过 Tool `Context` 把 subagent 能力、`localDenialTracking`、冻结 `renderedSystemPrompt` 注入工具层，Task/AgentTool 从 Context 借用而非直接持有 |
| OC-16 | `reference-analysis/evidence-index.md` L84 · `openclaw-main/docs/plugins/architecture.md` §"Public capability model" / §"Plugin shapes" | 插件按 capability 合约注册（`plain / hybrid / hook-only / non-capability`），工具与 Engine 通过 capability channel 交互 |
| HER-008 | `reference-analysis/evidence-index.md` L14 · `hermes-agent-main/model_tools.py` | `_AGENT_LOOP_TOOLS = {"todo", "memory", "session_search", "delegate_task"}` 在主循环被前置拦截，不由 registry 分发——等价于 "agent 级工具需要 capability 借用" 的内部实现 |
| ADR-006 | `harness/adr/0006-plugin-trust-levels.md` | Trust Level 已存在，本 ADR 正交叠加 `CapabilityPolicy` 矩阵 |

## 5. 落地清单

| 项 | 责任 crate | 说明 |
|---|---|---|
| `ToolCapability` 枚举 + capability traits | `harness-contracts` §3.4 | 接口契约 |
| `CapabilityRegistry` | `harness-contracts` | in-process anymap，简单实现 |
| `ToolDescriptor.required_capabilities` 字段 | `harness-tool` §2.2 | 注册期校验 |
| `CapabilityPolicy::default_locked` | `harness-tool` Registry | trust × cap 矩阵 |
| `ToolContext::capability::<T>` | `harness-tool` | 借用入口 |
| `SubagentRunnerCap` impl | `harness-subagent` | wrap 现有 `SubagentRunner` |
| `TodoStoreCap` / `RunCancellerCap` impl | `harness-engine` | wrap 内部 store |
| `ClarifyChannelCap` / `UserMessengerCap` impl | `harness-session` | 走事件总线 |
| `BlobReaderCap` impl | `harness-journal` | 配合 ADR-010 read_blob 工具 |
| `MockCapabilityRegistry` | `harness-contracts/testing` | 单测专用 |
