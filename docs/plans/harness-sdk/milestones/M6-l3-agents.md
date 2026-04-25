# M6 · L3 Multi-Agent · subagent + team

> 状态：待启动 · 依赖：M5 完成 · 阻塞：M7
> 关键交付：父→子 委派 + 多 Agent 长期协作（单进程内）
> 预计任务卡：10 张 · 累计工时：AI 14 小时（2 路并行约 7 小时墙钟）+ 人类评审 5 小时
> 并行度：**2 路并行**（subagent / team 互相正交）

---

## 0. 里程碑级注意事项

1. **2 路并行**：subagent 和 team 都依赖 engine，但彼此正交
2. **跨进程禁令**：harness-team **仅支持单进程内**（基于 `tokio::sync::broadcast`）；P3-3 修订项要求显式声明
3. **Subagent 不直接对用户说话**：必须通过 SubagentAnnouncement，DelegationBlocklist 默认含 `send_user_message`
4. **EngineRunner trait 反向注入**：subagent / team 通过 trait 引用 engine 实例，不直接 `use`
5. **subagent-tool feature**：引入 D2 §10 已登记破窗，默认 off

---

## 1. 任务卡总览

| Crate | 任务卡 | 内容 |
|---|---|---|
| **subagent** | M6-T01 ~ T05 | AgentTool + DelegationBlocklist + SubagentAnnouncement + ConcurrentSubagentPool |
| **team** | M6-T06 ~ T10 | 3 拓扑 + MessageBus + Coordinator + SharedMemory |

---

## 2. 路 L3-SA · `octopus-harness-subagent`

### M6-T01 · SubagentSpec + DelegationPolicy + 类型骨架

**SPEC 锚点**：
- `harness-subagent.md` §2
- `agents-design.md` §3
- ADR-004

**预期产物**：
- `src/lib.rs`
- `src/spec.rs`：SubagentSpec + DelegationPolicy + DelegationBlocklist
- `src/announcement.rs`：SubagentAnnouncement（结构化结果回父 session）

**关键不变量**：
- DelegationBlocklist 默认含：delegate / clarify / memory_write / send_user_message / execute_code（HER-014）
- max_depth = 1（默认）/ 可调至 3
- max_concurrent_children = 3

**预期 diff**：< 350 行

---

### M6-T02 · AgentTool + Spawn 流程

**SPEC 锚点**：`harness-subagent.md` §3 / §4

**预期产物**：
- `src/agent_tool.rs`：AgentTool（实现 `Tool` trait）
- `src/spawn.rs`：spawn_subagent → 独立 Session
- `tests/spawn.rs`

**关键不变量**：
- Subagent 系统提示是父的 frozen snapshot（CC-08）
- 共享父 prompt cache
- Inline MCP 受 trust 限制

**预期 diff**：< 400 行

---

### M6-T03 · ConcurrentSubagentPool + Watchdog

**SPEC 锚点**：`harness-subagent.md` §5（死锁防御）

**预期产物**：
- `src/pool.rs`：ConcurrentSubagentPool（per-parent × per-depth Semaphore）
- `src/watchdog.rs`：acquire_timeout + 死锁检测
- `tests/pool.rs`

**预期 diff**：< 300 行

---

### M6-T04 · SubagentRunnerCap 投影 + Sandbox/MCP/Memory 复用

**SPEC 锚点**：`harness-subagent.md` §6

**预期产物**：
- `src/runner_cap.rs`：SubagentRunnerCap（投影到 ToolContext）
- `src/inherit.rs`：Sandbox/MCP/Memory 继承策略

**预期 diff**：< 250 行

---

### M6-T05 · Subagent Contract Test + 安全用例

**预期产物**：
- `tests/contract.rs`
- `tests/blocklist.rs`：验证 DelegationBlocklist 默认行为
- `tests/announcement.rs`：验证 subagent 不直接对用户说话

**预期 diff**：< 200 行

---

## 3. 路 L3-TM · `octopus-harness-team`

### M6-T06 · Team + Topology + 类型骨架

**SPEC 锚点**：
- `harness-team.md` §2
- `agents-design.md` §4
- ADR-004

**预期产物**：
- `src/lib.rs`（首段显式声明"单进程内"约束，P3-3 修订）
- `src/team.rs`：Team + TeamBuilder + TeamMember + AgentId 长期映射
- `src/topology.rs`：Topology 枚举（CoordinatorWorker / PeerToPeer / RoleRouted）

**关键不变量**：
- 仅支持**单进程内** Team（不要跨进程）
- 每个 member 对应独立 Session
- AgentId 长期稳定，不随 SessionId 变

**预期 diff**：< 350 行

---

### M6-T07 · MessageBus（broadcast + Journal）

**SPEC 锚点**：`harness-team.md` §3

**预期产物**：
- `src/bus.rs`：MessageBus（基于 `tokio::sync::broadcast` + 持久化到 Journal）
- `tests/bus.rs`：fan-out / replay 一致性

**关键不变量**：
- Bus 消息持久化到 Journal（支持 replay）
- 消息 ordering：per-sender FIFO，跨 sender 无序

**预期 diff**：< 300 行

---

### M6-T08 · 3 拓扑实现

**SPEC 锚点**：`harness-team.md` §4-§6

**预期产物**：
- `src/topologies/coordinator_worker.rs`：CoordinatorWorker（一调度多 worker）
- `src/topologies/peer_to_peer.rs`：P2P
- `src/topologies/role_routed.rs`：RoleRouted（按角色规则路由）

**Cargo feature**：`coordinator-worker / peer-to-peer / role-routed`

**预期 diff**：< 450 行

---

### M6-T09 · Coordinator（特殊 Toolset）

**SPEC 锚点**：`harness-team.md` §7

**预期产物**：
- `src/coordinator.rs`：Coordinator（限定 toolset：DispatchTool / MessageTool / StopTeamTool）
- `tests/coordinator.rs`

**关键不变量**：
- Coordinator 不能直接执行任务（仅调度）

**预期 diff**：< 350 行

---

### M6-T10 · SharedMemory + Team Contract Test

**SPEC 锚点**：`harness-team.md` §8

**预期产物**：
- `src/shared_memory.rs`：SharedMemory（作为 harness-memory 的特殊 provider）
- `tests/contract.rs`：3 拓扑一致性
- `tests/team_e2e.rs`：模拟 3 成员协作完成任务

**预期 diff**：< 300 行

---

## 4. M6 Gate 检查

- ✅ 2 crate 各自 `cargo test --all-features` 全绿
- ✅ Subagent E2E：父 spawn 子 → 子调用工具 → SubagentAnnouncement 回父
- ✅ Team E2E：3 成员 CoordinatorWorker → Coordinator dispatch → workers 执行 → 汇总
- ✅ DelegationBlocklist 默认行为（5 工具屏蔽）测试通过
- ✅ "跨进程禁令"在 team crate 文档显式声明
- ✅ subagent-tool feature 启用后 D2 §10 例外登记仍合规

未全绿 → 不得开始 M7。

---

## 5. 索引

- **上一里程碑** → [`M5-l3-engine.md`](./M5-l3-engine.md)
- **下一里程碑** → [`M7-l4-facade.md`](./M7-l4-facade.md)
