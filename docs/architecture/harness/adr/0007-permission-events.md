# ADR-007 · 权限决策事件化与沙箱-权限正交

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：`harness-permission` / `harness-journal` / `harness-sandbox` / `harness-session`

## 1. 背景与问题

参考项目的权限审批持久化对比：

| 项目 | 决策持久化 | 审计能力 |
|---|---|---|
| Hermes | 进程级字典 `_session_approved` + `_always_allowed_commands` | 进程重启即丢；无历史审计 |
| OpenClaw | 审批记录在文件 `approvals/*.json` | 审计需翻文件；无法 replay |
| Claude Code | 审批为一等 Event 并入 transcript | 最完整，对齐 |

另一个重要议题是**沙箱与权限的关系**：

| 项目 | 关系 | 问题 |
|---|---|---|
| Hermes | 容器型 env 早退跳审批（HER-041） | 沙箱误替代审批，误用高 |
| OpenClaw | 沙箱 × exec-approvals 并存但 `tools.elevated` 是"后门" | 不明确 |
| Claude Code | 沙箱与 permission 正交（CC-18） | 对齐 |

## 2. 决策

### 2.1 所有权限决策事件化

审批不是进程字典 / 副本文件，而是**正规 Event**：

```rust
pub enum Event {
    // ...
    PermissionRequested {
        request_id: RequestId,
        tool_use_id: ToolUseId,
        subject: String,
        severity: Severity,
        scope_hint: DecisionScope,
        at: DateTime<Utc>,
    },
    PermissionResolved {
        request_id: RequestId,
        decision: Decision,
        decided_by: DecidedBy,
        at: DateTime<Utc>,
    },
}
```

所有审批记录进 Event Journal，可 replay，可审计，跨重启保留（通过 `EventStore::read` 重建 `AllowList`）。

### 2.2 沙箱与权限正交

**沙箱不替代权限审批**。即使 Tool 在 Docker / SSH 沙箱执行，破坏性命令仍必须走审批流。

```rust
pub fn check_permission(
    tool: &dyn Tool,
    input: &Value,
    ctx: &ToolContext,
) -> PermissionCheck {
    // 即使 sandbox 是 Docker，也要走 check
}
```

对齐 CC-18 与 OC-23（但 OC-24 `tools.elevated` 后门我们不采纳）。

### 2.3 双 Broker 形态

同时支持：

```rust
#[async_trait]
pub trait PermissionBroker {
    async fn decide(
        &self,
        request: PermissionRequest,
        ctx: PermissionContext,
    ) -> Decision;
}
```

- **DirectBroker**：业务层实现，同步回调
- **StreamBasedBroker**：SDK 将审批作为 Event 推流，业务层 `harness.resolve_permission(id, decision)` 回调

### 2.4 决策范围与契约共享类型

`DecisionScope` / `Decision` / `DecidedBy` / `RuleSource` / `PermissionSubject` / `InteractivityLevel` /
`TimeoutPolicy` / `FallbackPolicy` 等枚举与结构体统一在 L0 契约层 `harness-contracts` 定义
（见 `crates/harness-contracts.md` §3.4 / §3.4.1），本 ADR 只引用不重复声明。关键类型示例：

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

pub enum Decision {
    AllowOnce,
    AllowSession,
    AllowPermanent,
    DenyOnce,
    DenyPermanent,
    Escalate,
}
```

对 `DecisionScope` 的使用约束（如"危险命令命中必须降级到 `ExactCommand`"、"`Any` 仅 `BypassPermissions` 模式可用"）详见 `permission-model.md` §2.3。

`DecisionScope::ExactCommand` 命中匹配的**键**是 `ExecFingerprint`（canonical 算法见 `crates/harness-sandbox.md` §2.2）。`Decision::AllowSession` / `AllowPermanent` 在 Rule 存储里持久化的就是 `ExecFingerprint` 而非原始命令字符串，确保不同空白 / env / cwd 表达的同一语义命令命中同一规则。

`AllowPermanent` / `DenyPermanent` 同时写入 `Event::PermissionResolved` 和 `Rule` 存储（通过 `PermissionBroker::persist`）。

`Decision::Escalate` 不能被 Broker 单独消费——必须由 `ChainedBroker` 串联 + `PermissionTerminator` 兜底（详见 `crates/harness-permission.md §3.6`）。`RuleSource::Policy` 的 Deny 视为硬闸门，不可被任何低/高优先级源覆盖（详见 `permission-model.md §4.1`）。

### 2.5 Allowlist/Denylist 从 Event Journal 重建

```rust
impl AllowListProjection {
    pub fn replay(events: impl Iterator<Item = &Event>) -> Self {
        let mut list = Self::new();
        for ev in events {
            if let Event::PermissionResolved { decision, .. } = ev {
                list.apply(decision);
            }
        }
        list
    }
}
```

重启后仍可恢复当前 Session 的 AllowList 状态（从 Event Store）。

### 2.6 规则预检与默认模式的事件配对

为了让审计 API / replay 工具用同一份代码处理所有审批路径，**所有**进入审批流程的请求（包括规则预检命中、`PermissionMode` 默认模式直接定决策、Hook `OverridePermission` 覆盖等）都必须成对发出 `PermissionRequested + PermissionResolved`：

| 路径 | `PermissionRequested.presented_options` | `PermissionResolved.decided_by` |
|---|---|---|
| 规则引擎命中 Allow/Deny | 空 vec（标识"未真正发往 UI"） | `Rule { rule_id }` |
| `PermissionMode::AcceptEdits` / `BypassPermissions` 自动放行 | 空 vec | `DefaultMode` |
| `PermissionMode::Plan` / `DontAsk` 自动拒绝 | 空 vec | `DefaultMode` |
| Hook `OverridePermission` | 空 vec | `Hook { handler_id }` |
| 用户 UI 答复 | 完整候选项 | `User` 或 `Broker { broker_id }` |
| 超时兜底 | 完整候选项 | `Timeout { default }` |

**为什么也对未询问路径写 Requested**：

- 审计 API（"为什么这个命令被允许了"）查 `PermissionResolved.causation_id` → `PermissionRequested.event_id`，所有路径同构。
- Replay 工具（差异回放、回归测试）按 `PermissionRequested.event_id` 索引；若规则预检不写 Requested，replay 时无法重建"在何处被默认放行"的位置。
- `PermissionRequested.presented_options.is_empty()` 即可让 EventStream 过滤层判断"是否触达外部 UI"，不必查附加字段。

`InteractivityLevel::NoInteractive` 走相同规则：写 Requested + Resolved，`presented_options` 为空，`Resolved.decided_by` 反映真实 decider（多数为 `DefaultMode` 或 `Timeout`）。

### 2.7 `PermissionSnapshot` 快照（必选）

Event Journal 是 `AllowList` 的真相源，但长 session 的逐 Event replay 是 O(n)。本 ADR 把 `PermissionSnapshot` 列为**必选实现项**而非"可选优化"：

```rust
pub struct PermissionSnapshot {
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub up_to_event_id: EventId,
    pub allowlist: AllowList,
    pub denylist: DenyList,
    pub generated_at: DateTime<Utc>,
    pub schema_version: u32,
}
```

**触发策略**（具体阈值由实现选择，但必须实现）：

- 写入侧：每 N 条 `PermissionResolved` 或每 M 分钟，由 `harness-journal` projection 写一份 snapshot 到 BlobStore；写后保留旧 snapshot 至少一份用于灾备。
- 读取侧：`AllowList::replay_with_snapshot(events, snapshot)` 从 `up_to_event_id` 开始追事件，O(增量) 重建。
- 失效：`schema_version` 与 `ExecFingerprint` 算法版本绑定；版本 bump 时所有 snapshot 视为失效，回退到全量 replay。

不实现 snapshot 的 backend 在长 session（>1000 次审批）下重启延迟会显著上升；列为必选避免后续 backend 各自补丁不一致。

## 3. 替代方案

### 3.1 A：审批进程字典（Hermes 风格）

- ❌ 进程重启即丢
- ❌ 无审计
- ❌ Replay 发散

### 3.2 B：审批副本文件（OpenClaw 风格）

- ❌ 与 Event Journal 不同步
- ❌ 查询需翻文件
- ❌ 并发写竞争

### 3.3 C：审批事件化（采纳）

- ✅ 单一真相源
- ✅ Replay 可复现
- ✅ 审计完整

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| Event 数量膨胀 | 每次审批写 2 个 Event；规则预检 / 默认模式也写一对（详见 §2.6） | 影响可控（每 session 几十~几百次）；snapshot 缓解长 session replay 成本（详见 §2.7） |
| 重建 AllowList 的 O(n) 开销 | 长 session 慢 | `PermissionSnapshot` 列为必选实现项（§2.7） |
| DirectBroker 同步性能 | 审批卡住主循环 | 业务层用 `tokio::spawn` + `oneshot` 异步回调；或改用 StreamBroker |
| `Decision::Escalate` 链路复杂度 | 单 broker 不能消费 Escalate | `ChainedBroker` + `PermissionTerminator`（`crates/harness-permission.md §3.6`） |

## 5. 后果

### 5.1 正面

- 审计报告 "为什么这个命令被执行了" 可一键查询
- Replay 测试可模拟不同审批序列
- 跨 session / 跨进程的 Allowlist 一致
- 审批错误（用户误点 Allow）可回溯并修正

### 5.2 负面

- Broker 实现者要理解决策范围（复杂）
- 需配套 UI 呈现 PermissionRequest

## 6. 实现指引

### 6.1 Event 字段细节

```rust
pub struct PermissionRequestedEvent {
    pub request_id: RequestId,
    pub run_id: RunId,
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub tool_use_id: ToolUseId,
    pub tool_name: String,
    pub subject: PermissionSubject,           // contracts §3.4.1
    pub severity: Severity,
    pub scope_hint: DecisionScope,
    /// 空 vec 标识"未真正发往 UI"（规则预检 / DefaultMode / Hook 覆盖路径，详见 §2.6）。
    pub presented_options: Vec<Decision>,
    pub interactivity: InteractivityLevel,    // contracts §3.4.1
    pub at: DateTime<Utc>,
}

pub struct PermissionResolvedEvent {
    pub request_id: RequestId,
    pub decision: Decision,
    pub decided_by: DecidedBy,
    pub scope: DecisionScope,
    pub rationale: Option<String>,
    pub at: DateTime<Utc>,
}

pub enum DecidedBy {
    User,
    Rule { rule_id: String },
    DefaultMode,
    Broker { broker_id: String },
    Hook { handler_id: String },
    Timeout { default: Decision },
}
```

> `subject` 字段从原 `String + detail: Option<String>` 升级为 `PermissionSubject` 枚举，与 `harness-contracts §3.4.1` 一致；`Display for PermissionSubject` 提供 UI/审计派生字符串。
> `DecidedBy::Hook` 是新增项，用于 Hook `OverridePermission` 路径（详见 §2.6）。

### 6.2 沙箱执行的审批路径

```text
Tool::execute(input, ctx)
    │
    ▼
[check_permission]   ← 永远执行，不因 sandbox 跳过
    │
    ├─ Allow → [sandbox.execute(spec)]
    └─ Ask  → [broker.decide] → ...
```

**关键**：即使 `sandbox_backend == "docker"` 或 `"ssh"`，`check_permission` 仍然调用。`harness-sandbox` 不暴露 `bypass_permission` / `elevated` 等任何旁路 API（详见 `crates/harness-sandbox.md` §8 与 §15 反模式条目）。

`check_permission` 在范围匹配阶段使用 `ExecSpec::canonical_fingerprint(...)` 生成的 `ExecFingerprint` 作为 `DecisionScope::ExactCommand` 的查询键；`SandboxBaseConfig::passthrough_env_keys` 决定哪些 env 参与指纹运算。

### 6.3 审计查询

```rust
impl Harness {
    pub async fn audit_permissions(
        &self,
        filter: PermissionAuditFilter,
    ) -> Result<Vec<PermissionRecord>>;
}
```

## 7. 相关

- D5 · `permission-model.md`
- D4 · `event-schema.md` §PermissionRequested / PermissionResolved
- `crates/harness-permission.md`
- Evidence: CC-13, CC-14, CC-18, HER-040, HER-041, OC-23, OC-24
