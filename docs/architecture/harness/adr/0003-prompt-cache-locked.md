# ADR-003 · Prompt Cache 硬约束与编译期表达

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：`harness-session` / `harness-context` / `harness-tool` / `harness-skill` / `harness-memory`

## 1. 背景与问题

LLM Prompt Cache（Anthropic 的 cache_control / OpenAI 的 auto-cache / Gemini 的 context caching）对成本与延迟影响巨大：

- Anthropic `cache_creation` tokens 比 `cache_read` 贵 **2.5~4 倍**
- Prompt Cache 命中率 < 50% 时，长会话成本爆炸
- 缓存要求前缀**完全一致**：一字节改变即全部失效

参考项目 Hermes 在 `AGENTS.md §Prompt Caching Must Not Break` 中明确要求：**Session 运行期禁止修改 system prompt / toolset / memory 三件套**（HER-027）。

然而运行期动态需求（加工具、加 Skill、更新 Memory）是真实业务诉求。需在"缓存稳定"与"灵活扩展"间找平衡。

## 2. 决策

### 2.1 Session 运行期三件套冻结

**Session 运行期禁止修改 system prompt / toolset / memory**。违反将导致 Prompt Cache 失效、成本爆炸、Replay 发散。

### 2.2 编译期表达

利用 Rust 类型系统在编译期表达约束：

```rust
impl Session {
    pub fn set_system_prompt(&mut self, _: String) -> Result<()> {
        Err(Error::PromptCacheLocked)
    }
}

impl SessionBuilder {
    pub fn set_system_prompt(mut self, v: String) -> Self {
        self.system_prompt = Some(v);
        self
    }
}
```

创建期（`SessionBuilder`）可写；运行期（`Session`）返回 `Error::PromptCacheLocked`。

### 2.3 Hot Reload 通过 Fork 实现

对于动态扩展需求，通过 `Session::reload_with(ConfigDelta)` 实现，返回三档 `ReloadOutcome`：

```rust
pub struct ReloadOutcome {
    pub mode: ReloadMode,
    pub new_session: Option<Session>,
    pub effective_from: ReloadEffect,
    pub cache_impact: CacheImpact,
}

pub enum ReloadMode {
    AppliedInPlace,
    ForkedNewSession { parent: SessionId, child: SessionId },
    Rejected { reason: String },
}

pub enum CacheImpact {
    NoInvalidation,
    OneShotInvalidation {
        reason: CacheInvalidationReason,
        affected_breakpoints: Vec<BreakpointId>,
    },
    FullReset,
}

pub enum CacheInvalidationReason {
    ToolsetAppended,
    SkillsAppended,
    McpServerAdded,
    MemdirContentChanged,
    SystemPromptChanged,
    ToolRemoved,
    ModelSwitched,
}
```

- **AppliedInPlace**：SDK 对象层面就地应用（同一 `Session` 对象继续使用），典型场景：加 Tool / 加 Skill / 加 MCP Server / Permission Rule 扩展。
  - 重要：**"就地应用"≠"零成本"**。对绝大多数这类变更，LLM 层面的 Prompt Cache 会产生 **一次性失效**（`CacheImpact::OneShotInvalidation`），下一 turn 会重新建立 cache，此后命中率恢复。
  - 仅 `permission_rule_patch` 这类纯 SDK 侧变更不影响 LLM 请求，对应 `CacheImpact::NoInvalidation`。
- **ForkedNewSession**：创建新 Session 对象，典型场景：改 system prompt / 删工具 / 切 model / 更新 Memdir 内容。原 session 保留可继续运行；新 session 带新配置 + `CacheImpact::FullReset`。
- **Rejected**：跨租户迁移、删除正在引用的 Tool 等违禁变更，拒绝应用。

> **为何不把"加 Tool"归为 Fork**：纯加 Tool 的变更虽然破坏一次 cache，但不影响语义正确性、也不需要两份 session 数据；强制 Fork 会让业务层背负"每加一个 Tool 就多一份 session"的存储与 ID 治理负担。采用 AppliedInPlace + 一次性 cache miss 是工程上更优的权衡。

详见 D8 · `context-engineering.md` §9 《注入顺序对缓存的影响》的详细语义表。

### 2.4 Registry Snapshot

所有 Registry（ToolRegistry / HookRegistry / PluginRegistry / McpRegistry / SkillRegistry）对外暴露 `snapshot()`：

```rust
pub struct RegistrySnapshot<T> {
    inner: Arc<[T]>,
    frozen_at: SnapshotId,
}
```

Session 持有 Registry Snapshot 而非 Registry 本身。Registry 的新增/删除不影响已运行 Session。

## 3. 替代方案

### 3.1 A：允许运行期修改，缓存失效由业务承担

- ❌ 成本波动巨大，不可预测
- ❌ Replay 发散
- ❌ 对齐反例：Hermes 在 `_last_resolved_tool_names` 陷阱（HER-052）

### 3.2 B：只提供文档约定，不做编译期约束

- ❌ 容易误用；团队规模扩大后维护成本高
- ❌ 可被 Plugin 突破（Plugin 能改 Registry）

### 3.3 C：编译期约束 + Fork-based hot reload（采纳）

- ✅ 误用几乎不可能
- ✅ Hot reload 需求也能满足（通过 Fork）
- ✅ 对齐 HER-027 的治理要求

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| API 表达繁琐 | 创建期 + 运行期两套接口 | Builder 模式平摊；Session Fork 自动做大部分工作 |
| Fork 存储开销 | 每次破坏性修改产生新 Session | Session Fork 只复制 metadata + Event reference，不复制 blob |
| 学习曲线 | 业务方需理解为何"加 Tool 要 fork" | 在 overview.md 与 ADR 中明确 |

## 5. 后果

### 5.1 正面

- Prompt Cache 命中率稳定（> 80% 常见）
- Replay 强一致
- 运行期安全

### 5.2 负面

- 创建 Session 必须提前规划好 Tool / Memory
- Hot Reload 需要理解 `ReloadMode` 三档

## 6. 实现指引

### 6.1 冻结点

- System prompt（Bootstrap file + header + tools snapshot + memory snapshot）
- ToolRegistrySnapshot
- MemdirSnapshot
- CacheBreakpoint 的 `after_message_id`

### 6.2 对外 API

`Session` 直接提供的 setter 只能返回 `Err(Error::PromptCacheLocked)`；需修改必须 `reload_with`。

### 6.3 Event 轨迹

```text
SessionReloadRequested { session_id, delta_hash, at }
    │
    ▼
[校验 delta + 分类]
    │
    ▼
SessionReloadApplied { session_id, mode, cache_impact, effective_from, at }
    │
    ├─ mode = AppliedInPlace
    │        ├─ cache_impact = NoInvalidation    → 下一 turn 起生效且 cache 保留
    │        └─ cache_impact = OneShotInvalidation → 下一 turn 起生效，产生一次 cache miss
    │
    ├─ mode = ForkedNewSession → Event::SessionForked + cache_impact = FullReset
    │
    └─ mode = Rejected → 返回 Err
```

## 7. MCP `tools/list_changed` 与本 ADR 的关系

MCP Server 可主动推送 `tools/list_changed`（见 `crates/harness-mcp.md` §6）。为避免 Session 中段被动破坏 Prompt Cache：

- **SDK 默认**：收到通知只记 `Event::McpToolsListChanged`，**不立即** re-inject 工具
- **业务层触发 re-apply**：`session.reload_with(ConfigDelta { reapply_mcp_servers })`，经本 ADR §2.3 分类器：
  - 只新增工具 → `AppliedInPlace + OneShotInvalidation`
  - 存在删除 → `ForkedNewSession + FullReset`

这样把"MCP 异步变更"的时机选择权交给业务层，避免用户正在进行对话时 cache 突然失效。

## 8. 相关

- D8 · `context-engineering.md`
- `crates/harness-session.md`
- `crates/harness-context.md`
- `crates/harness-mcp.md` §6（list_changed 处理）
- Evidence: HER-027, HER-052, CC-04, CC-08
