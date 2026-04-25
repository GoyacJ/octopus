# `octopus-harness-sdk` · L4 · 对外门面 SPEC

> 层级：L4（最高） · 状态：Accepted
> 依赖：全部 L0/L1/L2/L3

## 1. 职责

**业务层唯一直接依赖的 crate**。聚合所有内部 crate 成统一门面，提供 Builder、Harness、Session、EventStream 等最终对外接口。

**设计原则**：

- 95% 业务场景通过 `prelude` 一键覆盖
- 内部 crate 变动不应穿透到业务层
- Feature flag 汇总（见 D10）
- 提供 `builtin::` / `ext::` / `testing::` 三个 re-export 命名空间

## 2. Public API 结构

```rust
// lib.rs
pub mod prelude;
pub mod ext;
pub mod builtin;
pub mod testing;

pub use self::{
    harness::{Harness, HarnessBuilder, HarnessOptions},
    session::{Session, SessionOptions},
    team::{Team, TeamBuilder},
    error::HarnessError,
};

pub use octopus_harness_contracts::{
    Event, EventStream,
    SessionId, RunId, TenantId, TeamId, AgentId,
    TurnInput,
};
```

## 3. Prelude（最小可用面）

```rust
pub mod prelude {
    pub use octopus_harness_contracts::{
        Event, Decision, PermissionMode, TurnInput,
        SessionId, RunId, TenantId, MessageId, ToolUseId,
        HarnessError as Error,
    };
    pub use crate::{
        Harness, HarnessBuilder, HarnessOptions,
        Session, SessionOptions, SessionHandle,
        Team, TeamBuilder,
        ReloadMode, ReloadOutcome, ConfigDelta,
    };
    pub use crate::ext::*;
}
```

## 4. ext::（扩展 Trait）

```rust
pub mod ext {
    pub use octopus_harness_contracts::{
        BlobStore, BlobMeta, BlobRetention, BlobError,
        Redactor, RedactRules, RedactScope, RedactPatternSet, RedactPatternKind,
    };
    pub use octopus_harness_model::{ModelProvider, CredentialSource};
    pub use octopus_harness_journal::{EventStore, Projection};
    pub use octopus_harness_sandbox::{SandboxBackend};
    pub use octopus_harness_permission::{PermissionBroker, RuleProvider};
    pub use octopus_harness_memory::{MemoryProvider};
    pub use octopus_harness_tool::{Tool, ToolDescriptor, ToolContext, ToolResult};
    pub use octopus_harness_skill::{Skill, SkillLoader};
    pub use octopus_harness_mcp::{McpTransport, McpClient, McpConnection, ElicitationHandler};
    pub use octopus_harness_hook::{HookHandler, HookEvent, HookOutcome};
    pub use octopus_harness_plugin::{Plugin, PluginManifest, TrustLevel};
    pub use octopus_harness_observability::Tracer;
}
```

## 5. builtin::（内置实现 re-export）

```rust
pub mod builtin {
    // Provider
    #[cfg(feature = "provider-anthropic")]
    pub use octopus_harness_model::anthropic::AnthropicProvider;
    #[cfg(feature = "provider-openai")]
    pub use octopus_harness_model::openai::OpenAiProvider;
    #[cfg(feature = "provider-gemini")]
    pub use octopus_harness_model::gemini::GeminiProvider;
    #[cfg(feature = "provider-openrouter")]
    pub use octopus_harness_model::openrouter::OpenRouterProvider;
    #[cfg(feature = "provider-bedrock")]
    pub use octopus_harness_model::bedrock::BedrockProvider;
    #[cfg(feature = "provider-codex")]
    pub use octopus_harness_model::codex::CodexResponsesProvider;
    #[cfg(feature = "provider-local-llama")]
    pub use octopus_harness_model::local_llama::LocalLlamaProvider;
    #[cfg(feature = "provider-deepseek")]
    pub use octopus_harness_model::deepseek::DeepSeekProvider;
    #[cfg(feature = "provider-minimax")]
    pub use octopus_harness_model::minimax::MinimaxProvider;
    #[cfg(feature = "provider-qwen")]
    pub use octopus_harness_model::qwen::QwenProvider;
    #[cfg(feature = "provider-doubao")]
    pub use octopus_harness_model::doubao::DoubaoProvider;
    #[cfg(feature = "provider-zhipu")]
    pub use octopus_harness_model::zhipu::ZhipuProvider;
    #[cfg(feature = "provider-km")]
    pub use octopus_harness_model::km::KmProvider;

    // Journal (EventStore)
    #[cfg(feature = "sqlite-store")]
    pub use octopus_harness_journal::sqlite::SqliteEventStore;
    #[cfg(feature = "jsonl-store")]
    pub use octopus_harness_journal::jsonl::JsonlEventStore;

    // Journal (BlobStore)
    #[cfg(feature = "blob-file")]
    pub use octopus_harness_journal::blob_file::FileBlobStore;
    #[cfg(feature = "blob-sqlite")]
    pub use octopus_harness_journal::blob_sqlite::SqliteBlobStore;

    // Sandbox
    #[cfg(feature = "local-sandbox")]
    pub use octopus_harness_sandbox::local::LocalSandbox;
    #[cfg(feature = "docker-sandbox")]
    pub use octopus_harness_sandbox::docker::DockerSandbox;
    #[cfg(feature = "ssh-sandbox")]
    pub use octopus_harness_sandbox::ssh::SshSandbox;

    // Permission
    #[cfg(feature = "interactive-permission")]
    pub use octopus_harness_permission::interactive::DirectBroker;
    #[cfg(feature = "stream-permission")]
    pub use octopus_harness_permission::stream::StreamBasedBroker;
    #[cfg(feature = "rule-engine-permission")]
    pub use octopus_harness_permission::rule::RuleEngineBroker;

    // Memory
    pub use octopus_harness_memory::BuiltinMemory;

    // Tools
    pub use octopus_harness_tool::builtin::{
        BashTool, FileReadTool, FileEditTool, FileWriteTool,
        GrepTool, GlobTool, WebFetchTool,
        TodoTool, AgentTool, TaskStopTool,
        BuiltinToolset,
    };

    // Observability
    #[cfg(feature = "observability-otel")]
    pub use octopus_harness_observability::OtelTracer;
    pub use octopus_harness_observability::DefaultRedactor;
}
```

## 6. testing::（Mock 集合）

```rust
#[cfg(feature = "testing")]
pub mod testing {
    pub use octopus_harness_journal::memory::InMemoryEventStore;
    pub use octopus_harness_sandbox::noop::NoopSandbox;
    pub use octopus_harness_permission::mock::{AllowAllBroker, DenyAllBroker};
    pub use octopus_harness_model::mock::MockModelProvider;
    pub use octopus_harness_tool::mock::MockTool;
    pub use octopus_harness_hook::mock::MockHook;
    pub use octopus_harness_memory::mock::InMemoryMemoryProvider;
}
```

## 7. Harness

```rust
pub struct Harness {
    inner: Arc<HarnessInner>,
}

pub struct HarnessOptions {
    pub tenant_policy: TenantPolicy,
    pub default_session_options: SessionOptions,
    pub concurrent_sessions: Option<u32>,
    pub enable_replay: bool,
}

impl Harness {
    pub async fn create_session(&self, opts: SessionOptions) -> Result<Session, HarnessError>;
    pub async fn create_team(&self, builder: TeamBuilder) -> Result<Team, HarnessError>;
    pub async fn resolve_permission(&self, request_id: RequestId, decision: Decision)
        -> Result<(), HarnessError>;
    pub async fn for_tenant(&self, tenant: TenantId) -> Result<TenantHarness, HarnessError>;
    pub fn replay_engine(&self) -> Option<Arc<ReplayEngine>>;
    pub fn enabled_features(&self) -> &HashSet<String>;
    pub async fn shutdown(self) -> Result<ShutdownReport, HarnessError>;
}

pub struct TenantHarness {
    /* scoped Harness; 所有 API 方法带 tenant 隔离 */
}
```

## 8. HarnessBuilder（Type-State）

```rust
pub struct HarnessBuilder<ModelState = Unset, StoreState = Unset, SandboxState = Unset> {
    model: ModelState,
    store: StoreState,
    sandbox: SandboxState,
    /* ... 其他 Option 字段 */
}

pub struct Unset;
pub struct Set<T>(pub T);

impl HarnessBuilder<Unset, Unset, Unset> {
    pub fn new() -> Self;
}

impl<M, S, SB> HarnessBuilder<M, S, SB> {
    pub fn with_permission_broker<B: PermissionBroker>(self, b: B) -> Self { /* ... */ }
    pub fn with_memory_provider<P: MemoryProvider>(self, p: P) -> Self { /* ... */ }
    pub fn with_blob_store<B: BlobStore>(self, b: B) -> Self { /* ... */ }
    pub fn with_tool_registry(self, registry: ToolRegistry) -> Self { /* ... */ }
    pub fn with_skill_loader(self, loader: SkillLoader) -> Self { /* ... */ }
    pub fn with_hook_registry(self, registry: HookRegistry) -> Self { /* ... */ }
    pub fn with_mcp_config(self, config: McpConfig) -> Self { /* ... */ }
    pub fn with_elicitation_handler<E: ElicitationHandler>(self, h: E) -> Self { /* ... */ }
    pub fn with_plugin_registry(self, registry: PluginRegistry) -> Self { /* ... */ }
    pub fn with_observability(self, observer: Arc<Observer>) -> Self { /* ... */ }
    pub fn with_subagent_runner(self, runner: Arc<dyn SubagentRunner>) -> Self { /* ... */ }
    pub fn with_tenant_policy(self, policy: TenantPolicy) -> Self { /* ... */ }
    pub fn with_aux_model<M: ModelProvider>(self, m: M) -> Self { /* ... */ }
    pub fn with_rule_provider<R: RuleProvider>(self, r: R) -> Self { /* ... */ }
}

/// `BlobStore` 默认解析顺序：
/// 1. 若显式 `with_blob_store` → 用业务提供的实现
/// 2. 否则若启用 `blob-sqlite` feature 且 `with_store(SqliteEventStore)` → 复用同一 SQLite pool 的 `SqliteBlobStore`
/// 3. 否则 fallback 到 `InMemoryBlobStore`（非生产）

impl<S, SB> HarnessBuilder<Unset, S, SB> {
    pub fn with_model<M: ModelProvider>(self, m: M) -> HarnessBuilder<Set<M>, S, SB> { /* ... */ }
}

impl<M, SB> HarnessBuilder<M, Unset, SB> {
    pub fn with_store<S: EventStore>(self, s: S) -> HarnessBuilder<M, Set<S>, SB> { /* ... */ }
}

impl<M, S> HarnessBuilder<M, S, Unset> {
    pub fn with_sandbox<SB: SandboxBackend>(self, sb: SB) -> HarnessBuilder<M, S, Set<SB>> { /* ... */ }
}

impl<M: ModelProvider, S: EventStore, SB: SandboxBackend>
    HarnessBuilder<Set<M>, Set<S>, Set<SB>>
{
    pub async fn build(self) -> Result<Harness, HarnessError> { /* ... */ }
}
```

**关键**：`build()` 只在 `Model + Store + Sandbox` 都 Set 时才可用，编译期保证必填依赖。

### 8.1 Builder 幂等与覆盖语义

| 行为 | 语义 | 备注 |
|---|---|---|
| 同一 setter 重复调用（如 `with_model().with_model()`） | **后调用覆盖前调用** | 业务层显式接受覆盖；不发警告也不 panic |
| `with_aux_model` 与 `with_model` | 互不冲突 | aux model 与主 model 是独立槽 |
| `with_observability` 重复调用 | 后调用覆盖；前一个 `Observer` 不会被自动 drop（业务方持有 `Arc` 时手动管理） | 需要级联多个 Observer 时业务方自己组合后再 set |
| 跨 type-state 边界的二次 `with_model` | 第二次调用类型签名为 `HarnessBuilder<Set<M>, S, SB> -> HarnessBuilder<Set<M2>, S, SB>` | 通过额外 `impl` 块支持，行为与首次设置一致 |

> 反模式：依赖"先 set 后被 reset"恢复 `Unset` 状态——type-state 是单调进入 `Set` 的。如需"清空"必须重新 `HarnessBuilder::new()`。

### 8.2 Session 不走 type-state 的设计声明

与 `HarnessBuilder` 不同，`Session` 的构造**不采用 type-state**，原因：

- Session 的必填依赖（model / event-store / sandbox 等）来自其所属 `Harness`，已在 `HarnessBuilder` 阶段以 type-state 强制；
- Session 自身只承载"会话级配置"（system_prompt / tools / hooks / memory provider 选择 / steering policy 等），均为可选项，不存在"缺失即应阻止编译"的硬必填字段；
- 强行给 Session 加 type-state 会把 5+ 维状态炸开成几十种类型组合（Skill / Hook / MCP / Memory / Steering / ToolSearch …），不仅使 IDE 自动补全爆炸，还要为每种 builder 路径写测试；KISS 原则下不值。

因此 Session 走**普通 builder**（`SessionOptions` + `Harness::create_session(opts) -> Session`），运行期校验由 `Harness::create_session` 内部完成（详见 `crates/harness-session.md §2.3`）：

```rust
impl Harness {
    pub async fn create_session(&self, opts: SessionOptions) -> Result<Session, HarnessError>;
}

#[derive(Default)]
pub struct SessionOptions {
    pub system_prompt: Option<String>,
    pub workspace_id: Option<WorkspaceId>,
    pub tools: ToolPoolSpec,
    pub hooks: HookSelection,
    pub mcp: McpSelection,
    pub memory_provider: Option<Arc<dyn MemoryProvider>>,
    pub steering_policy: SteeringPolicy,
    pub tool_search: ToolSearchMode,
    pub aux_model_override: Option<Arc<dyn AuxModelProvider>>,
    /* ... 其他可选字段 */
}
```

**对比口径**：业务方启动 Harness 时缺必填项 → 编译失败（type-state）；业务方创建 Session 时配置非法 → 运行时 `Result::Err`。这种二分让"启动时确定的硬约束"与"会话级软约束"各得其所。

## 9. 完整业务层调用示例

### 9.1 基础启动

```rust
use octopus_harness_sdk::prelude::*;
use octopus_harness_sdk::builtin::*;

async fn bootstrap() -> Result<Harness> {
    HarnessBuilder::new()
        .with_model(AnthropicProvider::from_env()?)
        .with_store(JsonlEventStore::open("runtime/events").await?)
        .with_sandbox(LocalSandbox::default())
        .with_permission_broker(
            DirectBroker::new(|req, _ctx| async move {
                // CLI 同步询问
                Decision::AllowOnce
            })
        )
        .with_memory_provider(BuiltinMemory::at("data/memdir"))
        .with_tool_registry(
            ToolRegistry::builder()
                .with_builtin_toolset(BuiltinToolset::Default)
                .build()
        )
        .build()
        .await
}
```

### 9.2 单 Turn

```rust
async fn one_turn(harness: &Harness) -> Result<()> {
    let session = harness.create_session(SessionOptions::default()
        .with_workspace_bootstrap("data/workspace")
    ).await?;

    let mut events = session.run_turn(TurnInput::user("帮我 review auth.rs")).await?;
    while let Some(ev) = events.next().await {
        match ev? {
            Event::AssistantDeltaProduced(e) => print!("{}", e.delta),
            Event::ToolUseRequested(e) => eprintln!("[tool] {}", e.tool_name),
            Event::PermissionRequested(e) => {
                // Direct broker 已经处理过；此处不会收到
            }
            Event::RunEnded(_) => break,
            _ => {}
        }
    }
    Ok(())
}
```

### 9.3 Team

```rust
async fn team_flow(harness: &Harness) -> Result<()> {
    let team = harness.create_team(
        TeamBuilder::new("pr-review")
            .topology(TeamTopology::CoordinatorWorker {
                coordinator: "orchestrator".into(),
                workers: vec!["coder".into(), "reviewer".into()],
            })
            .member(TeamMemberBuilder::new("orchestrator").role("Planner").build())
            .member(TeamMemberBuilder::new("coder").role("Coder").build())
            .member(TeamMemberBuilder::new("reviewer").role("Reviewer").build())
    ).await?;

    let mut events = team.dispatch(TeamInput::goal("review PR #123")).await?;
    while let Some(ev) = events.next().await {
        // ...
    }
    Ok(())
}
```

### 9.4 Hot Reload

```rust
async fn reload_flow(session: &Session) -> Result<()> {
    let outcome = session.reload_with(ConfigDelta {
        add_tools: vec![my_new_tool_registration()],
        ..Default::default()
    }).await?;

    match outcome.mode {
        ReloadMode::AppliedInPlace => {
            tracing::info!("下一 turn 起生效，prompt cache 保留");
        }
        ReloadMode::ForkedNewSession { child, .. } => {
            tracing::info!(?child, "破坏性变更，已 fork");
        }
        ReloadMode::Rejected { reason } => {
            tracing::warn!(%reason, "重载被拒");
        }
    }
    Ok(())
}
```

### 9.5 MCP Server Adapter

```rust
async fn expose_as_mcp(harness: Arc<Harness>) -> Result<()> {
    use octopus_harness_sdk::ext::*;
    use octopus_harness_mcp::HarnessMcpServer;

    let mcp_server = HarnessMcpServer::new(harness.clone())
        .with_exposed_capabilities(ExposedCapabilities {
            sessions_list: true,
            messages_send: true,
            events_wait: true,
            permissions_respond: true,
            ..Default::default()
        })
        .with_auth(McpServerAuth::StaticBearer(SecretString::new(token)))
        .build()?;

    mcp_server.serve_http("0.0.0.0:7820".parse()?).await
}
```

## 10. Feature Flags

完整 feature 矩阵见 D10 · `feature-flags.md`。

默认：

```toml
default = [
    "sqlite-store",
    "jsonl-store",
    "local-sandbox",
    "interactive-permission",
    "mcp-stdio",
    "provider-anthropic",
]
```

其他内置 Provider 通过各自 `provider-*` feature 或 `all-providers` 启用；默认集合不缩减 v1.0 Provider 支持范围。

## 11. 错误类型

```rust
pub type Result<T> = std::result::Result<T, HarnessError>;
```

`HarnessError` 包含所有子 crate error 的 `From` 实现，一站式处理。

## 12. 测试策略

| 类 | 覆盖 |
|---|---|
| 集成 | Harness::builder() + testing mock 跑完一 turn |
| Feature flag 矩阵 | 每个 profile 编译 + 烟雾测试 |
| 示例 | `examples/` 目录中的 quick-start / team / mcp-server 必须可编译运行 |
| Semver | `cargo-semver-checks` 每次 major bump 跑 |

## 13. 版本发布策略

- Public types 使用 `#[non_exhaustive]`（允许未来扩展）
- 公开 trait 新增方法必须有 default impl 或走 major bump
- `prelude` 只追加不删除（删除即 major bump）
- Feature flag 命名一旦发布不得改名

## 14. 反模式

- 业务层直接 `use octopus_harness_engine`（绕过门面）
- `HarnessBuilder` 链式里做 IO（应 lazy 到 `.build().await`）
- Session 跨租户共享（应 `harness.for_tenant`）
- 使用 `testing::*` 在生产

## 15. 相关

- D1 · `overview.md` §9 业务层调用示例
- D3 · `api-contracts.md`
- D10 · `feature-flags.md`
- 所有其他 crate SPEC
