# D5 · 权限模型

> 依赖 ADR：ADR-007（权限决策事件化）
> 状态：Accepted

## 1. 设计目标

1. **可审计**：每次审批决策落为 `PermissionRequested` / `PermissionResolved` Event（ADR-007）
2. **多形态**：CLI 同步询问、UI 异步事件驱动、CI 规则自动批准 — 同一模型覆盖
3. **分层治理**：允许在多个配置源叠加规则（user/workspace/project/cli/session）
4. **Fail-Closed**：默认拒绝；明确允许才通过（对齐 CC-03）
5. **沙箱与权限正交**：沙箱通过不跳过审批（对齐 OC-23 / ADR-007）

## 2. 核心概念

### 2.1 六种权限模式（对齐 CC-13）

| Mode | 行为 | 典型场景 |
|---|---|---|
| **Default** | 规则 + 兜底询问 | 开发者本地使用 |
| **Plan** | 只允许只读工具；破坏性操作直接阻止 | 计划阶段（plan mode） |
| **AcceptEdits** | 规则 + 文件编辑默认允许 | 受信任场景的加速 |
| **BypassPermissions** | 跳过所有询问（仅规则拒绝仍生效） | 托管运行、CI |
| **DontAsk** | 规则不命中时拒绝（从不弹窗） | 服务端批处理 |
| **Auto** | Claude Code `ant` 特殊模式，辅助 LLM 判定 | 实验功能（可选 feature） |

### 2.2 决策结果

```rust
pub enum Decision {
    AllowOnce,
    AllowSession,
    AllowPermanent,
    DenyOnce,
    DenyPermanent,
    Escalate,
}
```

- `AllowOnce`：本次调用放行，下次仍需审批
- `AllowSession`：当前 Session 内所有同类调用放行
- `AllowPermanent`：跨 Session 持久化（写入规则源）
- `DenyPermanent`：跨 Session 持久化拒绝
- `Escalate`：交给下一层审批者（如 Broker 无法决定时交给用户）

### 2.3 决策范围（Scope）

`DecisionScope` 是 L0 契约类型，定义在 `harness-contracts`（见 `crates/harness-contracts.md` §3.4）：

```rust
pub enum DecisionScope {
    ExactCommand { command: String, cwd: Option<PathBuf> },
    ExactArgs(Value),
    ToolName(String),
    Category(String),
    PathPrefix(PathBuf),
    GlobPattern(String),
    Any,
}
```

| Variant | 语义 | 匹配键 | 典型用例 |
|---|---|---|---|
| `ExactCommand { command, cwd }` | 绑定到精确命令的语义指纹 | `ExecFingerprint`（详见 §2.3.1） | `bash` / Shell 类 Tool 审批 |
| `ExactArgs(value)` | 绑定到规范化后的完整 input hash | BLAKE3 of canonical(`tool_name + tool_input`) | 结构化参数的业务 Tool |
| `ToolName(name)` | 按工具名粗粒度放行 | `tool_name` 字符串 | CI 模式 `bypass_permissions` |
| `Category(kind)` | 按工具类别（`readonly` / `fs-edit` / `network`） | `ToolGroup` 枚举 | Workspace 规则分类 |
| `PathPrefix(path)` | 文件操作的目录前缀 | `path.starts_with(prefix)` | `allow-edits-under /workspace/` |
| `GlobPattern(pat)` | glob 模式 | glob 编译后的 matcher | `deny rm * /prod/**` |
| `Any` | 全部放行（仅 `BypassPermissions` 模式可用） | 无（恒等命中） | CI 跑批 |

**关键约束（对齐 OC-24）**：

1. 业务工具应优先使用 `ExactCommand` / `ExactArgs` / `PathPrefix`，使用 `ToolName` 或 `Any` 要在审批 UI 上给出显式 "粗粒度" 提示。
2. 危险命令库命中时（`DangerousPatternLibrary::detect`）**强制降级**为 `ExactCommand` scope，不允许 `ToolName` / `Any` 级持久化。
3. 跨 MCP server 的授权以 `GlobPattern("mcp__<server>__*")` 或 `Category("mcp__<server>")` 表达；MCP 工具命名 canonical 规则见 `harness-contracts §3.4.2`，**不再使用**独立的 `McpTool` / `Network` 变体（这些概念由 `DangerousPatternLibrary` + `DecisionScope::ExactArgs` 组合承载）。

### 2.3.1 `ExactCommand` 的指纹匹配

`ExactCommand { command, cwd }` 在 AllowList 比对、Rule 持久化、Event 写入三处都用同一把指纹键：`ExecFingerprint`。

```text
DecisionScope::ExactCommand { command, cwd }
        │
        │  AllowList::lookup / DecisionPersistence::load_by_fingerprint
        ▼
   ExecFingerprint   ◄────  ExecSpec::canonical_fingerprint(&base)
                            （算法：harness-sandbox.md §2.2）
```

**匹配键的生成方**与**比对方**：

| 角色 | 责任 | 落点 |
|---|---|---|
| 生成方 | 由 Tool 在 `check_permission` 路径里把 `ExecSpec` 投影成 `ExecFingerprint` | `harness-tool` Shell 类工具 / `harness-sandbox::ExecSpec::canonical_fingerprint` |
| 比对方 | `AllowList::lookup_by_fingerprint(...)`、`DecisionPersistence::load_by_fingerprint(...)` | `harness-permission.md` §2.3 / §6 |
| 见证方 | `Event::PermissionResolved.scope` 持有原 `DecisionScope::ExactCommand`，并把 `ExecFingerprint` 作为 envelope 元数据落 Journal | `event-schema.md` §3.6 + ADR-007 §6.1 |

**约束**：

- 指纹算法变更视为**破坏性升级**：所有已持久化的 `Decision::AllowSession` / `AllowPermanent` 失效，必须随 Schema 版本号一并迁移；详见 ADR-007。
- `command` 字段保留**可读原文**仅供 UI 展示与审计，不参与匹配；匹配只看指纹。
- `cwd` 在指纹里使用 canonicalize 后的形态（去 `.` / `..` 但不解析 symlink），避免 `/srv/app/../app` 与 `/srv/app` 命中不同规则。
- `SandboxBaseConfig::passthrough_env_keys`（见 `crates/harness-sandbox.md` §3.0）决定哪些 env 进入指纹运算；**业务层在配置该集合时即承诺了**"哪些环境差异会导致重新审批"。

## 3. 权限请求与响应

### 3.1 `PermissionRequest`

```rust
pub struct PermissionRequest {
    pub request_id: PermissionRequestId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub subject: PermissionSubject,
    pub caller: CallerInfo,
    pub created_at: DateTime<Utc>,
}
```

> `PermissionSubject` 定义在 `harness-contracts §3.4.1`，本节只解释语义；调用方按上下文选择最具体的 variant：
>
> | Variant | 适用调用方 | 与 `DecisionScope` 的常见配对 |
> |---|---|---|
> | `CommandExec` | Shell / Bash 类工具（自带规范化 `argv` 与 `ExecFingerprint`） | `ExactCommand` |
> | `FileWrite` / `FileDelete` | 文件写入 / 删除工具 | `PathPrefix` / `GlobPattern` |
> | `NetworkAccess` | 出站 HTTP / fetch / MCP-HTTP transport | `Category("network")` / `GlobPattern` |
> | `DangerousCommand` | `DangerousPatternLibrary` 命中后强制使用 | `ExactCommand`（强制降级） |
> | `McpToolCall` | 任何 MCP 工具（含 in-process / stdio / HTTP） | `GlobPattern("mcp__<server>__*")` 或精确 canonical 名 |
> | `ToolInvocation` | 通用 fallback（结构化业务工具） | `ExactArgs` / `ToolName` |
> | `Custom` | 业务自定义类型；`kind` 由调用方声明 | 由调用方决定 |

**主体派生原则**：

- `subject` 与 `scope_hint` 必须语义一致（如 `CommandExec` 必须配 `ExactCommand`），否则 Broker 应拒绝并发 `Event::UnexpectedError`。
- `subject.fingerprint` 与 `scope_hint::ExactCommand` 的指纹**必须同源**（同一 `ExecSpec::canonical_fingerprint` 调用），由 Engine/Tool 在装配 Request 时一次性计算。

### 3.2 `PermissionContext`

```rust
pub struct PermissionContext {
    pub mode: PermissionMode,
    pub previous_mode: Option<PermissionMode>,   // 进入 Plan 前的原 mode（CC `prePlanMode` 等价）
    pub rule_snapshot: RuleSnapshot,
    pub sandbox_state: Option<SandboxState>,
    pub history: Vec<PriorDecision>,
    pub hooks: Vec<HandlerId>,

    /// 调用上下文是否可阻塞等待用户决策；由调用方（Engine / Subagent Runner /
    /// Cron Driver / Gateway）按上下文设置，Broker 不得猜测。
    pub interactivity: InteractivityLevel,

    /// 等待用户/外部决策的超时与心跳策略。`None` 表示沿用 Broker 默认。
    pub timeout_policy: Option<TimeoutPolicy>,

    /// 无规则命中且 Broker 也不能给出明确决策时的兜底；
    /// 不显式设置时按 `FallbackPolicy::AskUser`（fail-closed 在 NoInteractive 下降级为 DenyAll）。
    pub fallback_policy: FallbackPolicy,
}
```

`InteractivityLevel` / `TimeoutPolicy` / `FallbackPolicy` 定义在 `harness-contracts §3.4.1`。

**字段语义说明**：

- `previous_mode`：进入 `PermissionMode::Plan` 时由 Engine 填充；离开 Plan 时 Engine 据此回滚（对齐 CC-13 `prePlanMode`）。Broker 不直接读写。
- `interactivity`：
  - `FullyInteractive` —— Broker 可发 `Event::PermissionRequested` 并阻塞等待 `resolve`。
  - `NoInteractive` —— Broker **不得**发 `Event::PermissionRequested`；走 `FallbackPolicy` 直接定决策；事件流仅记录最终 `PermissionResolved`。
  - `DeferredInteractive` —— Broker 把请求挂到父 Session 的 EventStream（典型场景：Gateway 多平台 / Subagent 上提父级 / OpenClaw `sessions_spawn` 风格的 announce-and-wait），同时受 `TimeoutPolicy` 兜底。
- `timeout_policy`：`StreamBasedBroker` 与 `ChainedBroker` 据此配 sweeper 与 heartbeat（详见 `crates/harness-permission.md §3.2 / §3.6`）。

### 3.3 `Decision::Escalate` 的闭环

`Decision::Escalate` 是"我无法决定，下一个 Broker 来"的语义，由 `ChainedBroker` 串联多个 Broker 形成决策链：

```
RuleEngineBroker → AuxLlmBroker(可选) → StreamBasedBroker → TimeoutDecider(FallbackPolicy)
```

链尾必有终结者（`TimeoutDecider` / `FallbackPolicy::DenyAll`），保证不出现"永远 Escalate"的活锁。详见 `crates/harness-permission.md §3.6`。

## 4. 规则引擎

### 4.1 规则分层（对齐 CC-14 + 多租户 Policy）

从低到高优先级：

```
user_settings  <  workspace_settings  <  project_settings
               <  local_settings      <  flag_settings
               <  policy_settings     <  cli_arg_rules
               <  command_rules       <  session_rules
```

**合并顺序**：低优先级规则先解析 → 高优先级覆盖。

**`policy_settings`（`RuleSource::Policy`）的特殊语义**：

- 由租户 / 运营方 / Admin-Trusted 渠道下发，业务层不可写入。
- Policy 的 `Decision::DenyOnce` / `DenyPermanent` 视为**硬闸门**，不可被任何低优先级源覆盖；同名 Allow 规则即使来自 `Session` 也无效。
- Policy 的 `Allow*` 仍按常规合并顺序处理（即可被更高优先级源转 Deny）。
- 持久化层（`DecisionPersistence`）必须拒绝把任何决策写回到 `Source = Policy`，避免运行期污染下发文件。

### 4.2 规则结构

```rust
pub struct Rule {
    pub id: RuleId,
    pub source: RuleSource,                       // 定义在 harness-contracts §3.4.1
    pub priority: u32,
    pub predicate: RulePredicate,
    pub decision: Decision,
    pub applies_to_modes: Vec<PermissionMode>,
    pub comment: Option<String>,
}

pub enum RulePredicate {
    ToolName(String),
    ToolNamePattern(GlobPattern),
    McpToolPattern { server: String, tool_pattern: GlobPattern },
    CommandPattern(RegexPattern),
    PathPattern(GlobPattern),
    NetworkHost(HostMatcher),
    Custom { key: String, value: Value },
    All(Vec<RulePredicate>),
    Any(Vec<RulePredicate>),
}
```

> `RuleSource` 定义在 `harness-contracts §3.4.1`（9 元枚举），本节只引用，不重复声明，避免事实源漂移（对应 contracts §3.4 的契约层去重原则）。

### 4.3 Deny 规则的特权语义（对齐 CC-15）

**Deny-Rule 在模型看到工具之前就剥离该工具**：

- 被 deny 的 Tool 从 `assembleToolPool` 中移除，**不进入模型的工具列表**
- MCP 工具的 deny 支持通配：`mcp__<server>__*` 屏蔽整个服务器的工具（canonical 形态见 `harness-contracts §3.4.2`）

这比运行时拦截更安全（模型根本不知道这个工具存在，无法通过 prompt 注入绕过）。

## 5. Broker 双形态（ADR-007）

### 5.1 DirectBroker（同步回调）

```rust
pub struct DirectBroker<F>
where
    F: Fn(PermissionRequest, PermissionContext) -> BoxFuture<'static, Decision>
        + Send + Sync + 'static,
{
    callback: F,
    persistence: Arc<dyn DecisionPersistence>,
}

impl<F> DirectBroker<F> {
    pub fn new(callback: F) -> Self;
    pub fn with_persistence(self, persistence: Arc<dyn DecisionPersistence>) -> Self;
}
```

**场景**：CLI / 脚本 / 测试

**优点**：代码简洁，业务层直接 `async fn prompt(req, ctx) -> Decision`

**`PermissionContext` 必传**：承载 `tenant_id / session_id / run_id / cwd / timeout_policy` 等审计字段，是 ADR-007 把审批事件化要求的最小信息集合。完整定义见 `crates/harness-permission.md §3.1` 与 `crates/harness-contracts.md §3.4`。

### 5.2 StreamBasedBroker（事件驱动）

```rust
pub struct StreamBasedBroker {
    emit: mpsc::Sender<PermissionRequest>,
    resolutions: Arc<DashMap<PermissionRequestId, oneshot::Sender<Decision>>>,
    persistence: Arc<dyn DecisionPersistence>,
}

pub struct StreamBrokerHandle {
    resolutions: Arc<DashMap<PermissionRequestId, oneshot::Sender<Decision>>>,
}

impl StreamBrokerHandle {
    pub async fn resolve(
        &self,
        request_id: PermissionRequestId,
        decision: Decision,
    ) -> Result<(), PermissionError>;
}
```

**场景**：Desktop UI / Web UI / 异步系统

**流程**：
1. SDK 创建 `(StreamBasedBroker, Receiver<PermissionRequest>, StreamBrokerHandle)`
2. Engine 调 `broker.decide(req)` → Broker 把 req 发到 Receiver 并等待 `oneshot` 回答
3. 业务层从 Receiver 拿到 req → 渲染 UI 让用户决策 → `handle.resolve(id, decision)` 回调
4. `decide` 返回 Decision

**优点**：UI 完全异步，审批流可跨窗口/会话

### 5.3 组合使用

业务层可同时使用两种：如 CLI 默认 Direct，但在**危险命令**上切到 Stream（允许后台审批队列）。

## 6. 审批决策持久化

### 6.1 持久化层级

| Scope | 存储位置 | 生存期 |
|---|---|---|
| `Once` | 内存（不持久化） | 单次调用 |
| `Session` | Journal Event | Session 生命周期 |
| `Permanent` | `DecisionPersistence` 实现 | 跨 Session |

### 6.2 持久化约束（对齐 HER-040）

- 命中危险模式（如 `rm -rf /`、`curl | sh`）**禁止** `AllowPermanent`（`DecisionPersistence::save` 返回 `Forbidden`）
- 持久化规则应与规则源（`config/runtime/*.json`）对齐，业务层可导出为 "规则文件" 供版本管理

### 6.3 审批去重与疲劳治理

LLM 在流式生成 / 重试 / Replanning 阶段经常对**同一语义**的工具调用反复触发审批。若 Broker 一律走完整链路，UI 会被同样的弹窗淹没，用户被迫"无脑批准"——审批闸门事实上失效（即"审批疲劳"）。本节定义 SDK 的去重契约。

#### 6.3.1 两个相互正交的去重维度

| 维度 | 触发条件 | 行为 | 实现位置 |
|---|---|---|---|
| **In-flight 合流（join）** | 在同一 Session 内，已有 `request_id == r0` 的请求处于 `pending`，又来一个语义键 `dedup_key` 与之相同的新请求 `r1` | `r1` **不**单独走链路；订阅 `r0` 的 oneshot，结果到达后两者复用同一 `Decision` | `harness-permission` `DedupGate`（详见 `crates/harness-permission.md §3.8`） |
| **短窗口复用（recent）** | 最近 N 秒内（默认 5s）同 `dedup_key` 已经被 Deny / AskUser-canceled / Timeout 兜底拒绝 | 直接复用该决策；**不**重新发链路、不重新询问用户 | 同上 |

> **边界**：AllowList / `DecisionPersistence::load_by_fingerprint` 处理的是**已持久化的 Allow** 决策（`AllowSession` / `AllowPermanent`），命中后整条链路本就不再询问；§6.3 处理的是"还未持久化或被拒绝过"的请求，与 AllowList 互不重叠。两层共同收敛"语义相同的请求只问一次"。

#### 6.3.2 `dedup_key` 的语义

```text
dedup_key = blake3(
    tenant_id ||
    session_id ||
    PermissionSubject::dedup_signature() ||  // 见下表
    canonical(scope_hint)
)
```

| `PermissionSubject` variant | `dedup_signature()` 取值 |
|---|---|
| `CommandExec { fingerprint, .. }` | `fingerprint`（必填；缺失视为构造非法） |
| `ToolInvocation { tool, input }` | `tool || canonical_json(input)` |
| `FileWrite { path, .. }` / `FileDelete { path }` | `path`（canonicalized） |
| `NetworkAccess { host, port }` | `host:port` |
| `McpToolCall { server, tool, input }` | `mcp__<server>__<tool> || canonical_json(input)` |
| `DangerousCommand { pattern_id, command }` | `pattern_id || command_normalized` |
| `Custom { kind, payload }` | `kind || canonical_json(payload)` |

**关键约束**：

1. `dedup_key` **不进入** `PermissionRequested` / `PermissionResolved` 字段——它是 Broker 内部索引，不是审计字段；审计字段仍以 `subject` + `scope` 为主。
2. `dedup_signature` 与 `ExecFingerprint` 的算法版本绑定 ADR-007 的 schema 版本号；指纹算法升版时 dedup 缓存视同失效，不做兼容。

#### 6.3.3 与"事件成对原则"的兼容

§7 的 "成对原则" 要求每个 `PermissionResolved` 必有同 `request_id` 的 `PermissionRequested`。Dedup 命中时——

- **被合流 / 复用方**（新请求 `r1`）：**不**写 `PermissionRequested` / `PermissionResolved`，改写一条 `Event::PermissionRequestSuppressed`，指向被复用决策的 `original_request_id` 与 `original_decision_id`。
- **原请求方**（`r0`）：照常发出 `PermissionRequested + PermissionResolved` 对。
- **审计 API** 在重建"某 ToolUse 的审批结果"时，先查 `PermissionRequestSuppressed` 是否存在，若存在则跳到 `original_request_id` 取最终决策，等价语义；replay 工具按相同语义解析。

`PermissionRequestSuppressed` 的字段定义见 `event-schema.md §3.6.3`。

#### 6.3.4 不参与去重的请求

下列请求即使 `dedup_key` 相同也**强制独立审批**，不复用历史决策：

- `Severity == Critical` 的 `DangerousCommand`（即危险命令库命中）
- 当前 `PermissionMode == Plan` —— Plan 模式下用户对每一次只读决策都可能重新评估
- `subject` 中的 `input` 经 `PreToolUse` Hook 改写后**重算指纹**仍与原 `dedup_key` 相同的情形（属于 Hook 误用，由 `harness-engine` Orchestrator 在装配 Request 时校验）

#### 6.3.5 默认参数（建议）

| 参数 | 默认值 | 调整建议 |
|---|---|---|
| `recent_window` | `5s` | 后台 / Cron 上下文可拉长到 30s；CLI 交互可缩短到 2s |
| `recent_cache_capacity` | `256` 项 / Session | LRU；超出按时间逐出 |
| `allow_inflight_join` | `true` | 测试场景可关 |
| `suppression_max_dedup_events_per_window` | `64` | 防 dedup 事件刷屏；超阈值后只更新计数器，不再写新事件（详见 `crates/harness-permission.md §3.8`） |

## 7. Event 轨迹

每次审批产生的 Event 序列（**所有路径都成对发出 `Requested + Resolved`**，便于审计与 replay）：

```text
ToolUseRequested        ← Engine 发出工具调用请求
    │
    ▼
PermissionRequested      ← 进入审批流即写一条；不论后续走哪条分支
    │
    ├─ [规则引擎预检] 命中 → PermissionResolved（decided_by=Rule { rule_id }）
    │
    ├─ [Hook PreToolUse] 返回 PreToolUseOutcome { override_permission: Some(_), .. } → PermissionResolved（decided_by=Hook { handler_id }）
    │
    ├─ [Mode Default] AcceptEdits/BypassPermissions/Plan/DontAsk 直接定决策
    │       └→ PermissionResolved（decided_by=DefaultMode）
    │
    └─ [Broker 决策] DirectBroker / StreamBasedBroker / ChainedBroker
            │
            ├─ User/UI 答复 → PermissionResolved（decided_by=Broker|User）
            └─ TimeoutPolicy 触发 → PermissionResolved（decided_by=Timeout { default }）
                    │
                    ▼
                ToolUseApproved / ToolUseDenied
                    │
                    ▼
                ToolUseCompleted / ToolUseFailed
```

**关键约束**：

1. **成对原则**：每个 `PermissionResolved` 必须有同 `request_id` 的前驱 `PermissionRequested`；规则预检 / DefaultMode / Hook 覆盖等"看似无询问"的路径也要先写 Requested 再写 Resolved，便于审计 API 与 replay 工具一致处理。
2. **因果链**：`PermissionRequested.causation_id` 指向触发它的 `ToolUseRequested.event_id`；`PermissionResolved.causation_id` 指向同 request 的 `PermissionRequested.event_id`。
3. **顺序约束**：`PermissionResolved` 必须在 `ToolUseApproved` / `ToolUseDenied` 之前。
4. **`InteractivityLevel::NoInteractive` 例外**：仍写 Requested + Resolved，但 Requested 的 `presented_options` 字段为空，标识此次"未真正发往 UI"。`PermissionRequested` Event 不会触达外部订阅者（由 EventStream 过滤层处理，详见 `event-schema.md §3.6`）。

## 8. 危险命令检测

### 8.1 模式库（对齐 HER-039）

```rust
pub struct DangerousPattern {
    pub id: String,
    pub regex: Regex,
    pub danger_level: DangerLevel,
    pub reason: &'static str,
}

pub enum DangerLevel {
    Catastrophic,
    Destructive,
    Exfiltration,
    PrivilegeEscalation,
    Suspicious,
}
```

默认模式库覆盖：

- `rm -rf /*` / `dd of=/dev/*` / `mkfs` / `:(){ :|:& };:` (fork bomb)
- `chmod 777 *` / `chown -R` / `sudo rm`
- `curl * | sh` / `wget * | bash` / heredoc-exec
- `git reset --hard HEAD~*` / `git push --force origin main`
- SSH 后门添加、`.bashrc` 写入、`/etc/sudoers` 修改
- 环境变量 exfil：`env > /tmp/*` / `cat ~/.aws/credentials`

### 8.2 输入预处理

```rust
pub fn normalize_for_detection(input: &str) -> String {
    let stripped = strip_ansi_codes(input);
    unicode_normalization::nfkc(&stripped).collect()
}
```

防绕过：ANSI 转义码注入、Unicode 同形字（如西里尔 `ⅿ` 冒充 `m`）。

## 9. Hook 集成（对齐 CC-23）

Hook 可在两类事件上覆盖权限决策，**契约形态不同**：

- `PreToolUse` 事件：必须用复合三件套 `HookOutcome::PreToolUse(PreToolUseOutcome { override_permission: Some(_), .. })`，因为 PreToolUse 上下文携带 `tool_input` 可能要联动改写——单一 `HookOutcome::OverridePermission` 在该事件下不被接受（dispatcher 视为 `HookReturnedUnsupported`）。
- `PermissionRequest` 事件：上下文里没有 `tool_input` 可改，使用单一 `HookOutcome::OverridePermission(Decision)`。

```rust
#[async_trait]
impl HookHandler for MyAuditHook {
    fn handler_id(&self) -> &str { "my-audit" }
    fn interested_events(&self) -> &[HookEventKind] {
        &[HookEventKind::PreToolUse, HookEventKind::PermissionRequest]
    }

    async fn handle(&self, event: HookEvent, _ctx: HookContext)
        -> Result<HookOutcome, HookError>
    {
        match event {
            HookEvent::PreToolUse { input, .. } if contains_pii(&input) => {
                Ok(HookOutcome::PreToolUse(PreToolUseOutcome {
                    override_permission: Some(Decision::DenyOnce),
                    additional_context: Some(ContextPatch {
                        role: ContextPatchRole::UserSuffix,
                        content: "[my-audit] 检测到 PII，已拒绝".into(),
                        apply_to_next_turn_only: true,
                    }),
                    rewrite_input: None,
                    block: None,
                }))
            }
            HookEvent::PermissionRequest { subject, .. } if subject.is_high_risk() => {
                Ok(HookOutcome::OverridePermission(Decision::DenyOnce))
            }
            _ => Ok(HookOutcome::Continue),
        }
    }
}
```

Hook 覆盖的决策仍会生成 `PermissionResolved` Event，`decided_by = Hook { handler_id }`。

**多 handler 决策合成**：同 priority 下若多个 handler 同时覆盖权限且决策冲突（如 A: AllowOnce、B: DenyOnce），dispatcher 按"Deny 压过 Allow"裁决，并写一条 `Event::HookPermissionConflict` 记录所有参与方与最终 winner（详见 `crates/harness-hook.md §2.6` 与 `event-schema.md §3.7.2`）。

## 10. 沙箱 × 权限正交性（ADR-007）

**禁止**：沙箱通过就跳过权限（Hermes 的做法：HER-041）。

**正确**：沙箱通过后仍进入 `PermissionBroker::decide`。沙箱与权限是两层独立防御。

**原因**：
- 沙箱失效（逃逸漏洞、配置错误）时仍有权限兜底
- 沙箱内的 `rm -rf /app` 对容器内数据同样破坏性
- 审批提供**人在回路**（human-in-the-loop）兜底

### 10.1 三层边界与组装顺序

Tool / Permission / Sandbox 三层在每次命令调用上的边界与执行顺序如下：

```text
┌─────────────────────────── Tool 层 (harness-tool) ───────────────────────────┐
│ ① 由 input 投影出 ExecSpec                                                    │
│ ② fp = ExecSpec::canonical_fingerprint(&base)        ← harness-sandbox §2.2  │
│ ③ allowlist.lookup_by_fingerprint(&fp)               ← 短路命中：跳到 ⑥      │
│ ④ dangerous_pattern_lib.detect(command)              ← 命中强制 ExactCommand │
│ ⑤ 构造 PermissionCheck { scope_hint, fingerprint=Some(fp), .. }              │
└──────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌────────────────────── Permission 层 (harness-permission) ───────────────────┐
│ ⑥ broker.decide(PermissionRequest, PermissionContext) → Decision            │
│ ⑦ 写 Event::PermissionRequested / PermissionResolved（fingerprint 入字段）   │
│ ⑧ AllowPermanent / AllowSession → DecisionPersistence::save(.. fingerprint) │
└──────────────────────────────────────────────────────────────────────────────┘
                                    │
                       Decision::Allow*│  Decision::Deny* → 短路
                                    ▼
┌──────────────────────── Sandbox 层 (harness-sandbox) ───────────────────────┐
│ ⑨ sandbox.before_execute(&spec, &ctx)                ← 远端预热 / 路径校验   │
│ ⑩ handle = sandbox.execute(spec, ctx)                ← Event::SandboxExec*  │
│ ⑪ outcome = handle.activity.wait()                                          │
│ ⑫ sandbox.after_execute(&outcome, &ctx)              ← 反向同步             │
└──────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
                        Tool 收尾 → ToolResultEnvelope
```

**层间不可越界的约束**：

| 边界 | 禁止 | 原因 |
|---|---|---|
| Tool → Permission | Tool 不得在 `check_permission` 之前调用 `sandbox.execute` | 否则越过审批 |
| Permission → Sandbox | Broker 不得读 `SandboxBackend` 状态作为决策输入 | 否则把"沙箱看起来安全"当成审批理由（HER-041 反例） |
| Sandbox → Permission | `SandboxBackend` 不得调用 `PermissionBroker` 或绕过它 | 否则成为旁路（OC-24 反例） |
| Hook → 各层 | `PreToolUse` Hook 只能改写 input；改写后 Tool 必须**重新计算指纹**再走 Allowlist | 否则 Hook 会让旧指纹的 Allow 套到新命令上，构成静默升权 |

> 三层边界的具体调用展开（含失败原子性、CWD marker 通道、`before/after_execute` 不可插 Hook 等约束）见 `crates/harness-tool.md` §2.7.1。

## 11. 多租户

所有 `PermissionRequest` / `Rule` / `PersistedDecision` 都带 `tenant_id`：

- 不同租户的规则不互相污染
- `DecisionPersistence` 实现按 `tenant_id` 分区存储
- 默认单租户 `TenantId::SINGLE`（降低心智成本）

## 12. 测试策略

| 测试类型 | 覆盖点 |
|---|---|
| 规则引擎单测 | 每种 Predicate；优先级合并顺序 |
| Broker 测试 | Direct/Stream 等价行为；超时、取消、重复 resolve |
| 危险命令 | 每条模式命中 + 绕过测试（ANSI/Unicode） |
| Hook 覆盖 | Hook 返回 OverridePermission 时 Event 正确生成 |
| 事件轨迹 | ToolUseRequested → PermissionRequested → Resolved → ToolUseApproved 因果链完整 |
| 多租户 | 租户 A 的规则不影响租户 B |

## 13. 反模式（禁止）

| 反模式 | 原因 |
|---|---|
| 在进程字典 / thread-local 存审批决策 | 不可审计、不可回放（HER-040 反例） |
| 用 `bool` 表示 Allow/Deny | 丢失 scope、decider、时效信息 |
| 让 Hook 直接调用 Tool 绕过 Broker | 违反审批正交性 |
| 容器型沙箱默认跳审批 | 违反 ADR-007 |
| 粗粒度 Allow（如 `Allow bash *`） | 违反 OC-24 绑定约束 |
| Tool 在 `check_permission` 内自行调用 `PermissionBroker::decide` | Broker 必须由 Engine/Orchestrator 唯一调用，避免重复询问与因果链断裂；详见 `crates/harness-tool.md §调用契约` |
| 后台 / Subagent 上下文使用 `InteractivityLevel::FullyInteractive` | 触发交互 UI 但无人响应 → 永久阻塞；调用方必须按上下文设置 `NoInteractive` / `DeferredInteractive` |
| `Decision::Escalate` 链尾无终结者 | 活锁；`ChainedBroker` 必须在链尾挂 `TimeoutDecider` 或 `FallbackPolicy::DenyAll` |
| `RuleSource::Policy` 写入运行时（持久化覆盖下发文件） | Policy 由 Admin-Trusted 渠道下发，运行时只读；持久化层应拒绝 `Source = Policy` 的写入 |
| 不挂 `DedupGate` / `recent_window = 0` 全程裸跑 Broker | LLM 流式重试 / 同语义并发会用相同弹窗淹没用户 → 审批疲劳；§6.3.5 给出默认参数 |
| `DedupGate` 把 `Severity::Critical` 危险命令也纳入合流 / 窗口复用 | 危险操作只问一次的语义破窗；`DangerousCommand` + `Critical` 必须强制旁路（§6.3.4） |
| 用 `dedup_key` 作为审计字段（写入 `PermissionRequested` / `PermissionResolved`） | dedup 是 Broker 内部索引；审计仍以 `subject` + `scope` 为主，避免事实源漂移 |
