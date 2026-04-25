# ADR-004 · Agent Team 拓扑抽象与 Subagent/Team 独立

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：`harness-subagent` / `harness-team` / `harness-engine` / `harness-sdk`

## 1. 背景与问题

Multi-Agent 协作有两种截然不同的语义：

| 需求 | 示例 |
|---|---|
| **临时委派**：父 Agent 遇到子问题，fork 一个有界子 Agent 探索 | Hermes `delegate_task`（HER-014）；CC `AgentTool`（CC-08） |
| **长期协同**：多个 Agent 持续交互完成大目标 | CC Coordinator Mode（CC-12）；OC Multi-Agent（OC-08） |

若两者耦合在同一 crate / trait：

- Subagent 的"有界 / 静默 / 单向"约束会污染 Team 的"长期 / 双向 / 可见性"需求
- Team 的消息总线 / 拓扑抽象在 Subagent 场景下是多余开销

## 2. 决策

### 2.1 Subagent 与 Team 独立为两个 crate

- `harness-subagent`：父→子 任务委派（单向、有界、短生命周期）
- `harness-team`：多 Agent 长期协同（双向、消息总线、共享记忆）

两者共享底层基础设施（`harness-engine` / `harness-session` / `harness-journal`），但**编排器与语义约束独立**。

### 2.2 Team 拓扑抽象

```rust
pub enum TeamTopology {
    CoordinatorWorker {
        coordinator: AgentId,
        workers: Vec<AgentId>,
    },
    PeerToPeer,
    RoleRouted(RoleRoutingTable),
    Custom(Arc<dyn TopologyStrategy>),
}
```

三种内置拓扑 + 自定义扩展位。

### 2.3 触发授权与上下文可见性正交

对齐 OC-26：

```rust
pub enum ContextVisibility {
    All,
    Allowlist(Vec<AgentId>),
    AllowlistQuote(Vec<AgentId>),
    Private,
}
```

- 触发授权：Topology + RoutingPolicy 决定
- 上下文可见性：独立旋钮（避免"能触发 = 能看见"的常见陷阱）

### 2.4 Subagent 默认 Blocklist（对齐 HER-014）

```rust
blocklist: {"delegate", "clarify", "memory_write", "send_user_message", "execute_code"}
max_depth: 1,
depth_cap: 3,
max_concurrent_children: 3,
announce_mode: StructuredOnly,
```

Subagent 默认**不能**直接对用户说话，必须通过 `SubagentAnnouncement` 结构化回父。

**软/硬双闸（`max_depth` vs `depth_cap`）**：

- `max_depth`：业务可见、可调；表达"这一类 agent 我希望最多嵌套几层"。
- `depth_cap`：架构红线、不可被业务覆盖；与 Hermes `_MAX_SPAWN_DEPTH_CAP=3` 同款，
  保护 SDK 自身不会因为某个错配的 AgentProfile 触发指数级递归 / token 失控。
- `DefaultSubagentRunner::spawn` 在每次调用时计算
  `effective = min(spec.max_depth, policy.depth_cap)`，
  `ParentContext.depth >= effective` 即 `SubagentError::DepthExceeded`。
- `depth_cap` 仅可在 `HarnessBuilder` 装配期一次性提升（ADR 例外），
  运行期任何 Tool / Agent / Plugin 路径都无法篡改。

详见 `crates/harness-subagent.md §2.5` 与 `agents-design.md §3.3`。

### 2.5 Subagent 与 Team 的组合

Team 成员可以再 `spawn` Subagent，但：

- Subagent 的 Event **不**广播到 Team
- Subagent 的 `SubagentAnnouncement` 只回给直接父成员
- Team 的 MessageBus 不感知 Subagent 内部

## 3. 替代方案

### 3.1 A：单一 `harness-agents` 合并所有 Multi-Agent 场景

- ❌ 抽象混乱（Subagent 的强约束污染 Team API）
- ❌ 拆分编排难（Subagent Runner vs Team Coordinator）
- ❌ Feature flag 粒度不够

### 3.2 B：只做 Subagent，Team 留给业务层

- ❌ 业务重复造 message bus / routing
- ❌ Replay 无法覆盖 Team 级事件

### 3.3 C：Subagent + Team 独立 crate（采纳）

- ✅ 单一职责
- ✅ 独立演化（Team 拓扑可丰富，Subagent 约束保持简单）
- ✅ Feature flag 细粒度（`agents-subagent` / `agents-team`）

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| Crate 数量 +1 | 维护成本 | 两者语义差异足够大，清晰分离更值 |
| 共享底层需 trait 抽象 | Engine / Session 都要对外暴露 | 在 `harness-engine` 定义 `trait EngineRunner`，Subagent 与 Team 依赖它 |
| 业务学习曲线 | 要理解两种语义 | 在 `agents-design.md` 明确对比表 |

## 5. 后果

### 5.1 正面

- Subagent API 简洁（只有 spawn / blocklist / announce）
- Team API 丰富（topology / routing / visibility / shared-memory）
- Replay 粒度精细（Subagent Event 与 Team Event 互不干扰）

### 5.2 负面

- 组合场景（Team 内 Subagent）需要明确定义边界（已在 `agents-design.md` §5 说明）

## 6. 实现指引

- **核心 trait**：`harness-engine::EngineRunner`（Subagent 与 Team 都实例化它）
- **共享类型**：`harness-contracts::AgentId / TeamId / SubagentId`
- **事件**：`SubagentSpawned / SubagentAnnounced` vs `AgentMessageSent / TeamTurnCompleted`
- **Feature Flag**：`agents-subagent` / `agents-team`（可独立启用）

## 7. 相关

- D6 · `agents-design.md`
- `crates/harness-subagent.md`
- `crates/harness-team.md`
- Evidence: HER-014, CC-08, CC-12, OC-08, OC-26, OC-27
