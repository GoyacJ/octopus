# D7 · 扩展性规范

> 依赖 ADR：ADR-002（Tool 不含 UI）, ADR-005（MCP 双向）, ADR-006（插件信任域二分）
> 状态：Accepted · 业务层与第三方集成的扩展边界在此确立

## 1. 设计原则

1. **能力 Trait 化**：所有可扩展能力（Tool / Skill / Hook / MCP / Sandbox / Model / Memory / Permission / EventStore）以 `trait` 暴露，SDK 不预设实现绑定
2. **注册显式**：扩展必须通过 `Registry::register` 显式注册，禁止运行期隐式发现（`harness-plugin` 的发现阶段除外，见 §6）
3. **契约向下**：任何扩展只依赖 `harness-contracts`，不感知上层 crate
4. **Fail-Closed**：未注册 / 未批准 / 签名失败的扩展**不得**加载
5. **快照稳定**：所有 Registry 快照后不可变，保障 Prompt Cache 硬约束（ADR-003）

---

## 2. 扩展点总览

| 类别 | Trait | 注册入口 | 最小可工作示例 | 证据 |
|---|---|---|---|---|
| **LLM Provider** | `ModelProvider` | `HarnessBuilder::with_model` | `struct MyProvider; impl ModelProvider for MyProvider { ... }` | HER-048 |
| **Tool** | `Tool` | `ToolRegistry::register` | §3 示例 | CC-02, OC-19 |
| **Skill** | Markdown + frontmatter | `SkillLoader::add_source` | §4 示例 | HER-037, OC-22, CC-26 |
| **Hook** | `HookHandler` | `HookRegistry::register` | §5 示例 | HER-034, CC-22, OC-29 |
| **MCP Transport** | `McpTransport` | `McpRegistry::with_transport` | §6.1 | HER-042, CC-19 |
| **MCP Server Adapter** | `HarnessMcpServer` | `HarnessBuilder::with_mcp_server_adapter` | §6.2 | HER-042 |
| **Sandbox Backend** | `SandboxBackend` | `HarnessBuilder::with_sandbox` | §7 | HER-011, OC-23 |
| **Permission Broker** | `PermissionBroker` | `HarnessBuilder::with_permission_broker` | §8 | HER-040, CC-13 |
| **Memory Provider** | `MemoryProvider` | `HarnessBuilder::with_memory_provider` | §9 | HER-016, OC-14 |
| **EventStore** | `EventStore` | `HarnessBuilder::with_store` | §10 | ADR-001 |
| **Plugin（批量封装）** | `Plugin` | `PluginRegistry::discover` | §11 | HER-033, OC-16, CC-38 |

---

## 3. Tool 扩展

> **权威定义**：`harness-tool.md` §2.1–§2.8；ADR-010（结果预算）；ADR-011（Capability）。
>
> 本节只给出**面向业务/插件作者**的最小上手面 + 必读约束，细节以 `harness-tool.md` 为准。

### 3.1 Trait 签名（流式）

```rust
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    /// 单一权威元数据（包含 schema / properties / trust_level / capabilities / budget / origin / group 等）。
    fn descriptor(&self) -> ToolDescriptor;

    /// 可选：动态 schema（依赖当前 session 状态生成）。返回 `None` 表示用 descriptor.input_schema。
    async fn resolve_schema(&self, _ctx: &ToolContext) -> Option<JsonSchema> { None }

    /// 执行期前置校验（类型/业务规则）。不做任何副作用。
    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<(), ToolError>;

    /// 权限校验：返回 `Allow` / `AskUser { scope }` / `Deny` / `Escalate`。
    async fn check_permission(&self, input: &Value, ctx: &ToolContext) -> PermissionCheck;

    /// 真正的执行：返回流式事件。
    /// 业务可选 emit `Progress`（纯观测）/ `Partial`（可被 Hook 改写）/ `Final` / `Error`。
    fn execute(&self, input: Value, ctx: ToolContext) -> ToolStream;
}

pub type ToolStream = Pin<Box<dyn Stream<Item = ToolEvent> + Send + 'static>>;
```

**与旧版对比**：

- 旧 `invoke(input) -> ToolResult` 被 `execute(input) -> ToolStream` 取代，支持心跳 / 中间字节流 / 错误流式上报（对齐 HER-012、CC-06）；
- `input_schema` / `output_schema` / `properties` 合并进 `ToolDescriptor`，避免"多处真相"；
- Schema 若需根据 Session 状态动态变化，改实现 `resolve_schema`（对齐 HER-009）。

### 3.2 ToolDescriptor 关键字段

```rust
pub struct ToolDescriptor {
    pub name: String,                             // 全 Session 唯一
    pub display_name: String,
    pub description: String,                      // 给模型看的文案
    pub category: String,
    pub version: Version,

    pub input_schema: JsonSchema,
    pub output_schema: Option<JsonSchema>,
    pub dynamic_schema: bool,                     // true 则运行期走 resolve_schema

    pub properties: ToolProperties,               // §3.3
    pub trust_level: TrustLevel,                  // Builtin / AdminTrusted / UserControlled
    pub required_capabilities: &'static [ToolCapability],  // ADR-011
    pub budget: ResultBudget,                     // ADR-010
    pub origin: ToolOrigin,
    pub group: ToolGroup,                         // General / Filesystem / Shell / Meta / Network / Custom
    pub provider_restriction: ProviderRestriction,
    pub search_hint: Option<ToolSearchHint>,
}
```

### 3.3 `ToolProperties` 默认值（Fail-Closed，对齐 CC-03 反例）

```rust
impl Default for ToolProperties {
    fn default() -> Self {
        Self {
            is_concurrency_safe: false,
            is_read_only: false,
            is_destructive: true,                 // 默认当破坏性（拒绝自动化）
            long_running: LongRunningPolicy::default(),  // 默认无心跳
            defer_policy: DeferPolicy::AlwaysLoad,       // 默认永远可见（上层再按需降级）
        }
    }
}
```

> 反例：Claude Code 早期把 `is_concurrency_safe` 默认为 `true`，导致文件编辑工具并发冲突（参见 `reference-analysis/evidence-index.md` CC-03 条目）。本 SDK 逆转默认值。

`max_result_size_chars` 已迁移至 `ToolDescriptor.budget`（`ResultBudget::chars(n)` 等价于旧字段；默认值参考 CC-33）；`mcp_origin` 已迁移至 `ToolDescriptor.origin`。

### 3.4 裁决矩阵（Trust × Capability × Group）

| 维度 | 内置 / `AdminTrusted` Plugin | `UserControlled` Plugin | MCP 导入 | Skill 内联工具 |
|---|---|---|---|---|
| 声明 `required_capabilities` | ✅ 任意 | ❌ 仅 `BlobReader` / `SkillRegistry` | ❌ 不允许 | ❌ 不允许 |
| `group = Shell` | ✅ | ⚠ 拒绝 destructive，除非显式 allowlist | ⚠ 自动加 `is_destructive = true` | ❌ |
| `DeferPolicy::AutoDefer` | ✅ | ✅ | ✅（默认） | ✅ |
| `origin` 覆盖内置同名 | ❌ 内置必胜（`ShadowReason::BuiltinWins`） | ❌ | ❌ | ❌ |
| `ResultBudget::unbounded` | ✅ 仅 `Read` / `ListDir` 等自限工具 | ❌ 最大允许 256 KiB | ❌ 最大允许 128 KiB | ❌ 最大 64 KiB |

> 详细矩阵与 `ShadowReason` 的处理见 `harness-tool.md §2.5.1`；`CapabilityPolicy` 默认值见 ADR-011 §2.5。

### 3.5 最小示例（业务工具）

```rust
pub struct SendInvoiceTool {
    billing_client: Arc<dyn BillingClient>,
}

#[async_trait]
impl Tool for SendInvoiceTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            name: "send_invoice".into(),
            display_name: "Send Invoice".into(),
            description: "Send an invoice to a customer via email.".into(),
            category: "business".into(),
            version: "1.0.0".parse().unwrap(),

            input_schema: SEND_INVOICE_INPUT_SCHEMA.clone(),
            output_schema: Some(SEND_INVOICE_OUTPUT_SCHEMA.clone()),
            dynamic_schema: false,

            properties: ToolProperties {
                is_concurrency_safe: false,
                is_read_only: false,
                is_destructive: true,
                ..Default::default()
            },
            trust_level: TrustLevel::AdminTrusted,
            required_capabilities: &[],                 // 不借用高权限 capability
            budget: ResultBudget::chars(8 * 1024),
            origin: ToolOrigin::Plugin {
                plugin_id: "billing".into(),
                trust: TrustLevel::AdminTrusted,
            },
            group: ToolGroup::Network,
            provider_restriction: ProviderRestriction::default(),
            search_hint: None,
        }
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ToolError> {
        let _req: SendInvoiceInput = serde_json::from_value(input.clone())
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;
        Ok(())
    }

    async fn check_permission(&self, input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        PermissionCheck::AskUser {
            subject: format!(
                "发送发票给 {}？",
                input["customer_email"].as_str().unwrap_or("?")
            ),
            scope: DecisionScope::ExactArgs(input.clone()),
        }
    }

    fn execute(&self, input: Value, _ctx: ToolContext) -> ToolStream {
        let client = self.billing_client.clone();
        Box::pin(async_stream::stream! {
            let req: SendInvoiceInput = match serde_json::from_value(input) {
                Ok(v) => v,
                Err(e) => {
                    yield ToolEvent::Error(ToolError::InvalidInput(e.to_string()));
                    return;
                }
            };
            match client.send(req).await {
                Ok(res) => yield ToolEvent::Final(ToolResult::structured(
                    json!({ "invoice_id": res.id })
                )),
                Err(e) => yield ToolEvent::Error(ToolError::External(e.to_string())),
            }
        })
    }
}
```

### 3.6 注册

```rust
let registry = ToolRegistry::builder()
    .with_builtin_toolset(BuiltinToolset::Default)       // Read / Write / Bash / WebSearch / Clarify / ...
    .with_tool(Box::new(SendInvoiceTool::new(billing_client)))
    .build()?;                                            // 校验 CapabilityPolicy / 去重 / 裁决同名冲突
```

Builder 在 `build()` 阶段：

1. 检查 `required_capabilities` 在 `CapabilityRegistry` 里是否已就绪；失败抛 `RegistrationError::CapabilityMissing`；
2. 按 `trust_level` 应用 `CapabilityPolicy::check`；失败抛 `RegistrationError::CapabilityNotPermitted`；
3. 与内置工具同名 → 直接拒绝本次注册（`ShadowReason::BuiltinWins`）并 emit `Event::ToolRegistrationShadowed`；
4. 成功后按 `(origin, name)` 去重；Plugin 更高 trust 覆盖低 trust 的同名注册。

### 3.7 让业务工具被 `execute_code` 调用（ADR-0016）

业务自定义工具默认**不会**被纳入 `EmbeddedToolWhitelist`。要被 PTC 脚本作为
`tool.<name>(...)` 调用，必须同时满足：

| 条件 | 来源 | 校验时机 |
|---|---|---|
| `descriptor.is_destructive == false` | `harness-tool.md §3` | `ToolRegistry::build()` |
| `descriptor.requires_human_in_loop == false` | `harness-tool.md §3` | `ToolRegistry::build()` |
| `trust_level >= AdminTrusted` | `harness-plugin.md §6` | manifest 校验 |
| 业务通过 `team_config.toml` 显式列入白名单 | ADR-0016 §2.6 | Session 启动 |
| `team_config.toml` 同时给出 `whitelist_reason` | ADR-0016 §2.6 | 审计 |

任意一项不满足，脚本侧 `tool.<name>(...)` 调用会直接抛
`Event::ToolUseDenied` 并附 `EmbeddedRefusedReason`，**不会**升级为审批，
也**不会**触发模型重试——见 `event-schema.md §3.5.2`。

> 写有副作用的业务工具**永远不应**进入嵌入式白名单；这是 ADR-0016 与
> ADR-0007 共同维护的硬边界。

---

## 4. Skill 扩展

> **权威定义**：`harness-skill.md` §2–§13。本节只给出**面向业务/插件作者**的最小上手面 + 必读约束，详细字段语义、trust 矩阵、降级矩阵以 `harness-skill.md` 为准。

### 4.1 文件结构

Skill 是 **Markdown + YAML frontmatter**，不是 Rust 代码（对齐 HER-037 / OC-22 / CC-26）。Frontmatter 与 [agentskills.io](https://agentskills.io) 公开标准兼容，自有扩展放在 `metadata.octopus.*` namespace。

```markdown
---
name: review-pr
description: Review a pull request and post comments
allowlist_agents: ["reviewer", "senior-engineer"]
platforms: [macos, linux]                  # 可选；空 = 全平台
prerequisites:
  env_vars: [GITHUB_TOKEN]                 # 必需 env，缺失即标记 PrerequisiteMissing
  commands: [gh]                           # advisory only
parameters:                                # 调用期参数（每次 render 必填）
  - name: pr_url
    type: string
    required: true
config:                                    # 启用期配置（持久化在配置中心）
  - key: github.token
    type: string
    secret: true
    required: true
hooks:                                     # 可选；与 skill 生命周期绑定（§4.6）
  - id: audit
    events: [PostToolUse]
    transport: { type: builtin, kind: AuditLog }
metadata:
  octopus:
    tags: ["code-review"]
---

# Review PR: ${pr_url}

Org token resolved at runtime: ${config.github.token:secret}

Please carefully review the changes in ${pr_url}. Follow this process:

1. Read the PR description
2. ...
```

### 4.2 加载源与优先级

```rust
pub enum SkillSource {
    Bundled,
    Workspace(PathBuf),
    User(PathBuf),
    Plugin(PluginId),
    Mcp(McpServerId),
}
```

`Bundled / Plugin / User / Workspace` 走**同一命名空间**，加载顺序（后覆盖前）：`Bundled → Plugin → User → Workspace`（对齐 OC-22）。**MCP skill 走独立命名空间** `mcp__<server>__<skill>`，不参与本地同名覆盖（详见 `harness-skill.md §3`）。

### 4.3 注册

```rust
let loader = SkillLoader::default()
    .with_source(SkillSourceConfig::Directory {
        path: "data/skills".into(),
        source_kind: DirectorySourceKind::Workspace,
    })
    .with_source(SkillSourceConfig::Directory {
        path: dirs::home_dir().unwrap().join(".octopus/skills"),
        source_kind: DirectorySourceKind::User,
    })
    .with_threat_scanner(Arc::new(MemoryThreatScanner::default()));
```

### 4.4 模板引擎

| 语法 | 含义 |
|---|---|
| `${var}` | 调用期 `parameters` 注入 |
| `${config.<key>}` | 启用期 `config` 注入（普通值） |
| `${config.<key>:secret}` | 启用期 secret 注入（不进 Journal 明文） |
| `` !`<command>` `` | 受限 shell（输出上限 4000 字符，对齐 HER-038） |

受限 shell 白名单由 `SkillLoader::with_shell_allowlist` 配置；`Workspace`/`User`/`Plugin{UserControlled}` 来源不允许任何**绕过白名单**的内联 shell。

### 4.5 注入位置与三种消费路径

Skill 内容**始终**注入为 **user message**（不碰 system prompt，保 Prompt Cache）。具体的注入位置（在 Active Context Patches 内的次序）与 transient/persistent 语义见 D8 · `context-engineering.md §11`。

业务可选三条消费路径（互不冲突）：

| 路径 | 触发方 | 适用场景 |
|---|---|---|
| **Eager 注入** | Session 创建期（`SkillPrefetchStrategy::Eager`） | Skill 数量少（< 20）+ 几乎每 turn 都用到 |
| **SkillTool 渐进披露** | LLM 主动调 `skills_list` / `skills_view` / `skills_invoke` | Skill 数量多（> 20）+ 单 turn 仅用少数（对齐 CC-26 / HER-037） |
| **Hook 主动注入** | `PreToolUse` / `UserPromptSubmit` 等 hook 通过 `AddContext` 注入 | 业务规则触发（如检测到敏感关键词时插入合规 skill） |

### 4.6 Skill 携带 Hook 的 Trust 矩阵

| Skill 来源 | `Builtin` transport | `Exec` / `Http` transport |
|---|---|---|
| `Bundled` / `Plugin{AdminTrusted}` | ✅ | ✅ |
| `Plugin{UserControlled}` / `User` / `Workspace` | ✅ | ❌（整个 skill reject） |
| `Mcp` | ❌ | ❌ |

不满足条件时 skill **整体 fail-closed**，避免静默降级（详见 `harness-skill.md §8.3`）。

### 4.7 加载期降级摘要

| 触发 | 行为 |
|---|---|
| `Bundled` 解析失败 | 硬 fail（中止整个 `load_all`） |
| 其它来源解析失败 / 平台不匹配 / 威胁扫描 Block 命中 / Trust 不足声明 Exec hook | 单 skill 跳过，记 `Event::SkillRejected` |
| `prerequisites.env_vars` 缺失 | 仍加载，标记 `SkillStatus::PrerequisiteMissing`，`SkillsListTool` 默认隐藏 |

完整矩阵见 `harness-skill.md §11`。

---

## 5. Hook 扩展

### 5.1 事件类别

Hook 事件分为五组共 **20 类**（与 `crates/harness-hook.md` §2.2 保持一致；新增/调整必须**两边同时改**）：

```rust
#[non_exhaustive]
pub enum HookEvent {
    // ── A · 核心生命周期 ─────────────────────────────────
    UserPromptSubmit { run_id: RunId, input: TurnInput },
    PreToolUse { tool_use_id: ToolUseId, tool_name: String, input: Value },
    PostToolUse { tool_use_id: ToolUseId, result: ToolResult },
    PostToolUseFailure { tool_use_id: ToolUseId, error: ToolErrorView },
    PermissionRequest { request_id: RequestId, subject: String, detail: Option<String> },
    SessionStart { session_id: SessionId },
    Setup { workspace_root: Option<PathBuf> },
    SessionEnd { session_id: SessionId, reason: EndReason },
    SubagentStart { subagent_id: SubagentId, spec: SubagentSpecView },
    SubagentStop { subagent_id: SubagentId, status: SubagentStatus },
    Notification { kind: NotificationKind, body: Value },

    // ── B · LLM/API 层 ──────────────────────────────────
    PreLlmCall { run_id: RunId, request_view: ModelRequestView },
    PostLlmCall { run_id: RunId, usage: UsageSnapshot },
    PreApiRequest { request_id: RequestId, endpoint: String },
    PostApiRequest { request_id: RequestId, status: u16 },

    // ── C · 转换层（独占改写权）─────────────────────────
    TransformToolResult { tool_use_id: ToolUseId, result: ToolResult },
    TransformTerminalOutput { tool_use_id: ToolUseId, raw: Bytes },

    // ── D · MCP ─────────────────────────────────────────
    Elicitation { mcp_server_id: McpServerId, schema: JsonSchema },

    // ── E · Tool Search（ADR-009） ──────────────────────
    /// `tool_search` 调用发起前。可用于审计/拦截特定查询。
    PreToolSearch {
        tool_use_id: ToolUseId,
        query: String,
        query_kind: ToolSearchQueryKind,
    },
    /// backend 完成 materialize 之后、schema 可见之前。可用于添加额外 hint 或记录。
    PostToolSearchMaterialize {
        tool_use_id: ToolUseId,
        materialized: Vec<ToolName>,
        backend: ToolLoadingBackendName,
        cache_impact: CacheImpact,
    },
}
```

### 5.2 Trait

```rust
#[async_trait]
pub trait HookHandler: Send + Sync + 'static {
    fn handler_id(&self) -> &str;
    fn interested_events(&self) -> &[HookEventKind];
    fn priority(&self) -> i32 { 0 }
    async fn handle(&self, event: HookEvent, ctx: HookContext)
        -> Result<HookOutcome, HookError>;
}

/// `HookOutcome` 是分发型枚举，每个 variant 对应一个或一组事件可用的"形状"。
/// `PreToolUse(PreToolUseOutcome)` 是**唯一**支持"复合三件套"的形态——
/// 同一个 handler 可以在一次返回里同时改写输入、覆盖审批决策、追加上下文。
/// 完整字段语义与字段互斥规则见 `crates/harness-hook.md §2.3 / §2.4.1`。
#[non_exhaustive]
pub enum HookOutcome {
    Continue,
    Block { reason: String },
    PreToolUse(PreToolUseOutcome),     // PreToolUse 三件套唯一入口
    RewriteInput(Value),               // UserPromptSubmit / PreLlmCall 等单一形态
    OverridePermission(Decision),      // PermissionRequest 单独覆盖
    AddContext(ContextPatch),          // PostToolUse 等单一上下文注入
    Transform(Value),                  // TransformToolResult / TransformTerminalOutput 专属
}
```

`HookContext` 字段定义见 `crates/harness-hook.md §2.2.1`；handler 必须满足 §11 的 **replay 幂等契约**（不持久化外部副作用、不读易变全局态）。

### 5.3 能力上限（完整 20 事件）

| 事件 | 允许的 HookOutcome | 说明 |
|---|---|---|
| `UserPromptSubmit` | `Continue / RewriteInput / Block` | 入参改写点 |
| `PreToolUse` | `Continue / Block / PreToolUse(PreToolUseOutcome)` | **三件套唯一入口**——单一 `RewriteInput` / `OverridePermission` / `AddContext` 在此事件下被视为 `HookReturnedUnsupported`（详见 `crates/harness-hook.md §2.3 / §2.4.1`） |
| `PostToolUse` | `Continue / AddContext` | **不能改写 result**（结果改写专属 `TransformToolResult`） |
| `PostToolUseFailure` | `Continue / AddContext` | 错误观察 + 附加上下文 |
| `PermissionRequest` | `Continue / OverridePermission` | 审批 UI 前的兜底策略 |
| `SessionStart` | `Continue / AddContext(SystemAppend)` | 唯一能注入 system message 尾部的事件 |
| `Setup` | `Continue` | 仅观察性；触发一次/进程 |
| `SessionEnd` | `Continue` | 仅观察 |
| `SubagentStart` | `Continue / Block` | 可拦截子 agent spawn |
| `SubagentStop` | `Continue` | 仅观察 |
| `Notification` | `Continue` | 仅观察 |
| `PreLlmCall` | `Continue / RewriteInput(ModelRequest patch)` | 改写模型请求（如注入 system suffix） |
| `PostLlmCall` | `Continue` | 仅观察，用于 usage 审计 |
| `PreApiRequest` | `Continue / Block` | 可拦截原始 HTTP 请求（合规场景） |
| `PostApiRequest` | `Continue` | 仅观察 |
| `TransformToolResult` | `Continue / Transform(ToolResult)` | **专属结果改写点**（PostToolUse 的补充） |
| `TransformTerminalOutput` | `Continue / Transform(Bytes)` | Bash 等 terminal tool 原始输出改写（如脱敏） |
| `Elicitation` | `Continue / Block` | MCP server 元素级询问拦截 |
| `PreToolSearch` | `Continue / Block` | Tool Search 调用前审计/拦截（ADR-009） |
| `PostToolSearchMaterialize` | `Continue / AddContext` | materialize 之后追加提示；**不能改变 materialized 列表** |

**独占改写通道**：

- Tool **结果**改写 → 唯一通道是 `TransformToolResult`
- Tool **输入**改写 → 唯一通道是 `PreToolUse(PreToolUseOutcome { rewrite_input, .. })`（不再支持 `PreToolUse::RewriteInput` 单一形态）
- Tool **权限**覆写 → 可在 `PreToolUse(PreToolUseOutcome { override_permission, .. })` 或 `PermissionRequest::OverridePermission`
- **原始终端字节**改写 → 唯一通道是 `TransformTerminalOutput`（独立于 `ToolResult`，用于结构化结果仍保留但需脱敏日志的场景）
- **模型请求**改写 → 唯一通道是 `PreLlmCall::RewriteInput`，且必须保持 ADR-003 Prompt Cache 锁定字段不变；违反 → `HookOutcomeInconsistent { reason: PromptCacheViolation }`

> 非允许 Outcome 会被 Dispatcher 视为 `Continue` 并记 `Event::HookReturnedUnsupported`，
> 字段互斥违规会记 `Event::HookOutcomeInconsistent`，handler 失败按 `failure_mode` 决定 fail-open / fail-closed
> （结构与字段定义见 `event-schema.md §3.7`，能力详细约束见 `harness-hook.md §2.4 / §2.6 / §2.6.1`）。

### 5.4 多 transport

SDK 支持三种 Hook 注册形态（字段权威定义见 `crates/harness-hook.md §3`）：

- **In-Process**：`impl HookHandler`（Rust 直写）
- **Exec**：`HookExecSpec { command, args, env, working_dir, resource_limits, signal_policy, protocol_version }`，stdin 送 JSON，stdout 读 JSON（对齐 HER-036）；仅 `TrustLevel::AdminTrusted` 可安装
- **HTTP**：`HookHttpSpec { url, timeout, auth, security: HookHttpSecurityPolicy { allowlist, ssrf_guard, max_redirects, max_body_bytes }, protocol_version }`，POST JSON；UserControlled plugin 必须提供非空 `allowlist` 且 `ssrf_guard` 全部启用

每条 transport 都需声明 `failure_mode: FailOpen | FailClosed`：UserControlled / User / Workspace 来源强制 `FailOpen`，仅 admin-trusted 可声明 `FailClosed`（详见 `harness-hook.md §2.6.1`）。

---

## 6. MCP 扩展

### 6.1 入站 Client（消费外部 MCP Server）

```rust
#[async_trait]
pub trait McpTransport: Send + Sync + 'static {
    fn transport_id(&self) -> &str;
    async fn connect(&self, spec: McpServerSpec) -> Result<Arc<dyn McpConnection>>;
}
```

内置 Transport：`stdio / http / websocket / sse / in-process`。业务层可新增自定义 Transport（如 `quic` / `unix-socket-custom`）。

### 6.2 出站 Server Adapter（把 Harness 暴露为 MCP Server）

```rust
pub struct HarnessMcpServer {
    harness: Arc<Harness>,
    policy: McpServerPolicy,
}

impl HarnessMcpServer {
    pub async fn serve_stdio(self) -> Result<()>;
    pub async fn serve_http(self, addr: SocketAddr) -> Result<()>;
}

pub struct ExposedCapabilities {
    pub sessions_list: bool,
    pub session_get: bool,
    pub messages_read: bool,
    pub messages_send: bool,
    pub attachments_fetch: bool,
    pub events_poll: bool,
    pub events_wait: bool,
    pub permissions_list_open: bool,
    pub permissions_respond: bool,
    pub channels_list: bool,
}
```

对齐 HER-042 的 9 个工具，业务方（例如 `octopus-server`）可通过此 Adapter 将自身暴露给 Claude Code / Cursor / Codex 等外部客户端。

### 6.3 Agent-scoped 注入

子 Agent 声明的 `mcpServers` 规则：

- **`McpServerRef::Shared(id)`**：复用父连接，子 Subagent 结束**不**关闭父连接
- **`McpServerRef::Inline(spec)`**：独立连接，Subagent 结束时 RAII 关闭；`spec.source` 受 trust 限制（详见 `crates/harness-mcp.md §5.2`，与 ADR-006 二分模型联动）
- **`McpServerRef::Required(id)`**：复用但要求 `connection_state = Ready`，否则装配失败
- **`SubagentSpec.required_mcp_servers`**：声明 pattern 级依赖（如 `mcp__slack__*`），装配期由 `McpRegistry::evaluate_required` 校验，详见 `crates/harness-mcp.md §5.3` 与 `crates/harness-subagent.md §3`

对齐 CC-20。

### 6.4 注入前过滤与反向调用

| 扩展点 | 入口 | 默认 | 说明 |
|---|---|---|---|
| `McpToolFilter` | `McpServerSpec.tool_filter` | `allow=[]; deny=[]; on_conflict=DenyWins` | 注入 `ToolRegistry` 前 allow/deny 过滤；与 `harness-permission.DenyRule` 共用 canonical glob 语法 |
| `SamplingPolicy` | `McpServerSpec.sampling` | `SamplingPolicy::denied()` | MCP Server 反向调用 `sampling/createMessage` 的预算与 cache 隔离；`UserControlled` server 不允许 `AllowAuto`（fail-closed） |
| `StdioPolicy / StdioEnv` | `TransportChoice::Stdio { policy, env }` | `InheritWithDeny + default_deny_envs()` | stdio 子进程环境变量沙化，默认屏蔽常见凭证类变量 |
| `ReconnectPolicy` | `McpServerSpec.reconnect` | 指数退避 + 5min 自重置 | 断连重连策略；触发 `Event::McpConnectionLost / McpConnectionRecovered` |
| `TenantIsolationPolicy` | `McpServerPolicy.tenant_isolation` | `StrictTenant + audit_severity=High` | Server Adapter 多租户隔离；`Delegated` 模式风险自负 |

详细字段定义见 `crates/harness-mcp.md §2.2 / §2.6 / §3.4 / §6.5`。

### 6.5 自定义 Transport 扩展

`McpTransport` trait 是开放扩展点。已知扩展场景（不在 SDK 默认 transport 列表中）：

| 场景 | 推荐路径 |
|---|---|
| IDE 桥接（VS Code / JetBrains 内置） | 实现 `InProcessFactory`，通过 `TransportChoice::InProcess` 装配 |
| QUIC / gRPC 自定义 server | 自实现 `McpTransport`，在 `transport_id()` 返回 `"quic-mcp"` 等唯一标识 |
| Unix Domain Socket | 自实现，复用 `tokio::net::UnixStream` |
| WebTransport / HTTP/3 | 自实现，调用方按需开启 |

**禁止**直接在 `TransportChoice` 枚举内 fork 新变体（属于反模式，详见 `crates/harness-mcp.md §12`）；任何新 transport 都通过实现 trait 接入。

---

## 7. Sandbox 扩展

```rust
#[async_trait]
pub trait SandboxBackend: Send + Sync + 'static {
    fn backend_id(&self) -> &str;
    fn capabilities(&self) -> SandboxCapabilities;
    async fn execute(&self, spec: ExecSpec, ctx: ExecContext)
        -> Result<ProcessHandle>;
    async fn snapshot_session(&self, spec: &SnapshotSpec)
        -> Result<SessionSnapshotFile>;
    async fn shutdown(&self) -> Result<()>;
}
```

内置：`LocalSandbox / DockerSandbox / SshSandbox / NoopSandbox`（对齐 HER-011 六后端中的核心三种 + testing）。

**关键约束**：即使 Sandbox 本身是容器化的，**破坏性操作仍必须走审批**（ADR-007，对齐 OC-23 反例 HER-041）。

### 7.1 `CodeSandbox` 扩展（ADR-0016）

> `CodeSandbox` 与 `SandboxBackend` 是**两条独立 trait**，职责面互不重叠：
> 前者执行受限脚本（无 OS syscall），后者执行进程命令。本节给出业务侧若需要扩展
> 第二种语言（如 Python / JS）时的指引。

```rust
#[async_trait]
pub trait CodeSandbox: Send + Sync + 'static {
    fn capabilities(&self) -> CodeSandboxCapabilities;
    async fn run(&self, script: &CompiledScript, ctx: CodeSandboxRunContext)
        -> Result<CodeSandboxResult, SandboxError>;
}
```

内置：`MiniLuaCodeSandbox`（M0）。`CodeSandboxCapabilities` 上声明
`language / max_instructions / max_call_depth / wall_clock_budget / deterministic` 等
配额；详见 `crates/harness-sandbox.md §3.5`。

**业务扩展硬约束（ADR-0016 §3.5 替代方案分析）**：

1. 必须走 ADR + Capability + Sandbox 三联评审；不允许默认实现替换 `MiniLuaCodeSandbox`
2. 必须明确该语言的禁用清单（典型：`os / subprocess / ctypes / socket / threading`）
3. 必须给出 cooperative tick / instructions 计数等价物，否则不接受
4. `EmbeddedToolDispatcher` capability handle 仍由 SDK 注入；脚本必须**只能通过**
   宿主回调触达工具，禁止脚本侧自实现 dispatcher
5. 嵌入式工具白名单沿用 ADR-0016 §2.6 的 read-only 默认；扩展必须经由
   `team_config.toml` 显式声明并写出 `Event::ExecuteCodeWhitelistExtended`

> **反模式**：把 `LocalSandbox::execute(...)` 包装为 `CodeSandbox` 来"省事"——
> 这会让脚本通过 `os.execute(...)` 类语法绕过嵌入式工具白名单，违反 ADR-0016 §2.7
> "脚本本身不发起任何 OS 操作"的边界。

---

## 8. Permission Broker 扩展

详细见 D5 · permission-model.md。扩展通过实现：

```rust
#[async_trait]
pub trait PermissionBroker: Send + Sync + 'static {
    async fn decide(
        &self,
        request: PermissionRequest,
        ctx: PermissionContext,
    ) -> Decision;
    async fn persist(
        &self,
        decision_id: DecisionId,
        scope: DecisionScope,
    ) -> Result<()>;
}
```

业务层常见实现：企业 SSO 审批、Slack 询问、ServiceNow 工单。

---

## 9. Memory Provider 扩展

> 详细 SPEC：`crates/harness-memory.md` §2 / §4。本节仅给出扩展实现者最常关心的接口轮廓。

### 9.1 Trait 拆分（v1.4）

`MemoryProvider` 拆为 **存储面** `MemoryStore` + **生命周期面** `MemoryLifecycle`，blanket impl 合并：

```rust
#[async_trait]
pub trait MemoryStore: Send + Sync + 'static {
    fn provider_id(&self) -> &str;
    async fn recall(&self, q: MemoryQuery) -> Result<Vec<MemoryRecord>, MemoryError>;
    async fn upsert(&self, r: MemoryRecord) -> Result<MemoryId, MemoryError>;
    async fn forget(&self, id: MemoryId) -> Result<(), MemoryError>;
    async fn list(&self, scope: MemoryListScope) -> Result<Vec<MemorySummary>, MemoryError>;
}

#[async_trait]
pub trait MemoryLifecycle: Send + Sync + 'static {
    async fn initialize(&self, ctx: &MemorySessionCtx) -> Result<(), MemoryError> { Ok(()) }
    async fn on_turn_start(&self, turn: u32, msg: &UserMessageView<'_>)
        -> Result<(), MemoryError> { Ok(()) }
    async fn on_pre_compress(&self, msgs: &[MessageView<'_>])
        -> Result<Option<String>, MemoryError> { Ok(None) }
    async fn on_memory_write(&self, action: MemoryWriteAction, target: &MemoryWriteTarget,
                             content_hash: ContentHash) -> Result<(), MemoryError> { Ok(()) }
    async fn on_delegation(&self, task: &str, result: &str, child: SessionId)
        -> Result<(), MemoryError> { Ok(()) }
    async fn on_session_end(&self, ctx: &MemorySessionCtx,
                            summary: &SessionSummaryView<'_>) -> Result<(), MemoryError> { Ok(()) }
    async fn shutdown(&self) -> Result<(), MemoryError> { Ok(()) }
}

pub trait MemoryProvider: MemoryStore + MemoryLifecycle {}
impl<T: MemoryStore + MemoryLifecycle> MemoryProvider for T {}
```

**为什么拆分**：

- 简单 provider（如 in-memory / 测试 stub）只需 `MemoryStore`，`MemoryLifecycle` 用默认空实现即可
- 深度集成型 provider（Honcho / Mem0 / 自建向量库）按需重载 `on_pre_compress` / `on_session_end` 等
- 避免 Hermes 那种"一个大 trait + 6 个 `pass` 方法"的样板代码

### 9.2 约束

| 约束 | 说明 | 证据 |
|---|---|---|
| **最多 1 个外部 provider** | `MemoryManager::set_external` 第二次调用返回 `ExternalSlotOccupied` | HER-016 |
| **不允许暴露 tool schema** | Provider 与 Agent 的所有交互通过两 trait；不引入 `get_tool_schemas()` 等价物 | ADR-003 / ADR-009（工具面冻结） |
| **Recall fail-safe** | 默认 `FailMode::Skip`：provider 失败不阻塞 turn | `harness-memory.md §4.2.4` |
| **Builtin 独立** | `MEMORY.md/USER.md` 永远存在且**不参与 recall**，仅 system message 直注 | HER-018 + ADR-003 |
| **Visibility 鉴权** | `MemoryManager` 在 recall 后按 `MemoryActor` 强制过滤；provider 内部即使返回越权数据也会被剔除 | `harness-memory.md §2.3` |

### 9.3 典型扩展场景

| 场景 | 推荐实现 | 重载的生命周期 |
|---|---|---|
| 向量召回（pgvector / Milvus） | `MemoryStore`（embedder + ANN） | 通常仅 `on_pre_compress` 提示压缩 |
| 知识图谱（Neo4j） | `MemoryStore` + 图查询包装 | `on_delegation` 记录子任务调用关系 |
| Honcho / Mem0 SaaS | `MemoryStore` + 完整 `MemoryLifecycle` | `on_session_end` 长期持久化、`on_pre_compress` 注入用户画像 |
| 仅本地 SQLite | `MemoryStore` 默认 lifecycle 即可 | — |

---

## 10. EventStore 扩展

```rust
#[async_trait]
pub trait EventStore: Send + Sync + 'static {
    async fn append(
        &self,
        tenant: TenantId,
        session: SessionId,
        events: &[Event],
    ) -> Result<JournalOffset>;
    async fn read(
        &self,
        tenant: TenantId,
        session: SessionId,
        cursor: ReplayCursor,
    ) -> BoxStream<Event>;
    async fn snapshot(
        &self,
        tenant: TenantId,
        session: SessionId,
    ) -> Result<Option<SessionSnapshot>>;
}
```

内置：`InMemoryEventStore / JsonlEventStore / SqliteEventStore`。业务扩展常见：`PostgresEventStore / RedisEventStore / KafkaEventStore`。

---

## 11. Plugin（批量封装）

Plugin 是**批量封装扩展**的机制：一个 Plugin 可以同时注册 Tool / Skill / Hook / MCP Server / 配置默认值。

> **权威定义**：`crates/harness-plugin.md`。本节给出业务/插件作者面向的最小上手面与硬约束摘要，命名空间、依赖图、生命周期状态机等以 `harness-plugin.md` 为准。

### 11.1 Manifest

```yaml
# plugin.yaml
manifest_schema_version: 1
name: octopus-invoice
version: 1.2.3
trust_level: admin-trusted
min_harness_version: ">=0.10, <0.20"
dependencies:
  - name: octopus-billing-core
    version_req: "^1.0"
    kind: required
capabilities:
  tools:
    - send_invoice
    - list_invoices
  skills:
    - review-invoice
  hooks:
    - id: invoice-audit
      events: [PostToolUse]
  mcp_servers:
    - id: invoice-api
      transport: http
      url: https://invoice.example.com/mcp
```

> `manifest_schema_version` 缺省解释为 `1`；`name` 必须满足 `harness-plugin.md §5.2` 命名空间语法（小写 ASCII / 保留前缀仅 admin-trusted 可用）。

### 11.2 信任域二分（对齐 ADR-006）

| 信任级别 | 来源 | 能力上限 |
|---|---|---|
| **admin-trusted** | Workspace 管理员安装 | 全能力（含 exec hook、破坏性 Tool） |
| **user-controlled** | 用户个人 / 临时 | 受限（不能注册破坏性 Tool、不能 `strictPluginOnlyCustomization`） |

对齐 CC-27，反例 OC-18（OpenClaw 未分层，插件直接获 Gateway 全权限）。

### 11.3 发现 / 加载流程（对齐 OC-17 / HER-033 / ADR-0015）

ADR-0015 把 Discovery 与 Runtime Load 拆为两个 trait（`PluginManifestLoader` / `PluginRuntimeLoader`），让"manifest-first 不得执行代码"这一硬约束在类型层面被强制。流程：

1. **Manifest Load**（`PluginManifestLoader::enumerate`）：扫描 4 个源（workspace / user / project / cargo extensions）。**只读 Manifest 文件**或 cargo extension 的"冷启动冷退出"元数据子命令；trait 返回类型为 `Vec<ManifestRecord>`，没有路径产出 `Arc<dyn Plugin>`，因此**类型层**保证不得加载 / 执行任何插件代码、不得启动子进程、不得发起网络调用（除非业务方注入 `RemoteRegistryLoader` 等显式接受网络的实现）。详见 `harness-plugin.md §3.1` / §3.2
2. **Validation**（`ManifestValidator`）：JSON schema、`manifest_schema_version` 兼容、签名（admin-trusted 必签；signer 由 `TrustedSignerStore`（ADR-0014）治理，含启用窗口 + 撤销列表）、命名空间唯一性、Trust 与来源匹配（ADR-006）、依赖图无环、Slot 不冲突
3. **Runtime Load**（`PluginRuntimeLoader::load`）：仅由 `PluginRegistry::activate` 触发；按声明顺序找第一个 `can_load` 的 RuntimeLoader 实例化；状态机详见 `harness-plugin.md §7`
4. **Registry**：插件 `activate(ctx)` 时通过 capability handle（ADR-0015 §2.4）把声明范围内的扩展注册到对应 Registry，并以 `PluginId` 作为 `Origin` 标签；越权注册被 `RegistrationError::Undeclared*` 拦截

> 失败按"是否已解析出合法 manifest"分流：
>
> - **未解析**（YAML 错 / schema 不通过 / `manifest_schema_version` 不被支持 / cargo extension 元数据 malformed）→ `Event::ManifestValidationFailed { manifest_origin, failure }`
> - **已解析但被业务规则拒绝**（Trust / 命名 / 依赖 / Slot / 签名 / 撤销 / Admission）→ `Event::PluginRejected { plugin_id, reason }`
>
> `reason` 取值见 `harness-plugin.md §4 RejectionReason`；`failure` 取值见 `event-schema.md §3.20.3 ManifestValidationFailure`。

---

## 12. 扩展测试

每个扩展点都必须提供 `testing::` 模块的 mock：

| 扩展 | Mock | 说明 |
|---|---|---|
| `ModelProvider` | `MockModelProvider::new().respond_with(...)` | 预设输出序列 |
| `Tool` | `MockTool::noop() / MockTool::returning(...)` | 不真实执行 |
| `Hook` | `MockHook::recording()` | 记录所有 invocation |
| `SandboxBackend` | `NoopSandbox` | 不 spawn 进程 |
| `PermissionBroker` | `AllowAllBroker / DenyAllBroker` | 统一决策 |
| `MemoryProvider` | `InMemoryMemoryProvider` | HashMap 底座 |
| `EventStore` | `InMemoryEventStore` | Vec 底座 |

---

## 13. 反模式（禁止）

| 反模式 | 原因 |
|---|---|
| 在 Tool 的 `invoke` 里写 UI 代码 | ADR-002（Tool 不含 UI） |
| 在 Hook 的 `handle` 里循环 `tokio::spawn` | 阻塞 hook dispatcher |
| 在 Plugin 加载时执行网络请求 | 违反 manifest-first / lazy runtime |
| 在 Tool 描述里返回绝对路径 | 泄露本地文件系统信息 |
| 跳过 `check_permission` 实现 | fail-closed 默认拒绝 |
| 把业务凭证写入 Tool schema description | 违反 security-trust.md §安全边界 |

---

## 14. 版本兼容策略

- 扩展遵循 SemVer
- SDK 升级时，`ToolProperties` / `HookOutcome` / `Decision` 等**开放 enum** 使用 `#[non_exhaustive]`，允许新增变体不破坏外部实现
- 废弃 API 以 `#[deprecated]` 标注并给出迁移示例

## 15. 文档与清单

扩展上线前必须同步更新：

- 该 crate 的 SPEC（`crates/harness-<name>.md`）的"扩展点"一节
- `feature-flags.md` 如果需要 feature gate
- 发布 changelog（`CHANGELOG.md`）

---

## 16. 索引

- **Trait 签名总表** → `api-contracts.md`
- **Tool 规则** → `crates/harness-tool.md`
- **Plugin 加载与信任** → `crates/harness-plugin.md` + ADR-006
- **MCP 双向** → `crates/harness-mcp.md` + ADR-005
- **安全边界** → `security-trust.md`
