# ADR-0017 · Steering Queue / In-flight User Messages（运行期软引导）

> 状态：Accepted
> 日期：2026-04-25
> 决策者：架构组
> 关联：ADR-0001（Event Sourcing）、ADR-0003（Prompt Cache Hard Constraint）、ADR-0007（Permission Events）、ADR-0010（Tool Result Budget）、`crates/harness-engine.md`、`crates/harness-session.md`、`crates/harness-contracts.md`、`event-schema.md`、`context-engineering.md`、`api-contracts.md`

## 1. 背景与问题

### 1.1 症状

主循环目前只有两类"用户输入"路径：

| 路径 | 时机 | 语义 |
|---|---|---|
| `Session::run_turn(turn_input)` | 用户主动开启新一轮 | 同步等模型出 `tool_calls` 或 `assistant_finish` |
| `Session::interrupt()` → `InterruptToken::trigger()` | 任意运行期 | **硬中断**：当前 stream / 当前 tool / 当前 model.infer 全部 abort，写 `Event::RunEnded { reason: Interrupted }` |

但用户日常使用中存在大量"既不是 turn 边界，也不希望硬中断"的场景：

- "刚才让你扫五个目录，现在加上第六个：`packages/`"
- "你查 license 的方向是对的，但只要 MIT、不要 BSD"
- "这次跑得太慢，跳过 tests/ 子目录"
- "刚意识到 tools/ 下不需要 grep，把 step 4 跳掉"

这类引导的本质是 **软中断 + 软追加**：保留 Run 进行中的状态、保留已花费的 token、保留 prompt cache，仅在下一次安全检查点把"用户的最新意图"合并进去。

### 1.2 行业先例

| 系统 | 做法 | 关键能力 |
|---|---|---|
| OpenClaw | 显式 `queue mode`：每个 lane 维护 `inflight + queue`，新消息进入 queue，下一个 turn 边界 drain；queue 可被另一条 message 覆盖（OC-11） | 多 lane 并行、且新消息以 lane 为粒度 |
| Hermes Agent | Gateway 层 `_pending_messages` adapter 队列 + runner 层命令拦截"双 guard"，runner 在 iteration 结束后查 pending（HER-029 / HER-031） | 单线程主循环，软中断由 adapter 桥接 |
| Claude Code | 命令面板 / SDK `query.steering` 把消息追加到当前 session message buffer，在下一轮 `model.infer` 前 prepend；硬中断走单独的 `abort` API（CC-06 周边） | 仅一种粒度的软引导 |

### 1.3 现状缺口

- `harness-session §6 中断` 只暴露 `interrupt()` → `InterruptToken::trigger()`
- `harness-engine §3 主循环` 在 13 阶段流水线里没有"drain steering queue"检查点
- `harness-contracts §3.3 Event` 没有 Steering 相关事件 variant
- 业务侧若想做"软追加"，只能在 Session 之外维护一个外部 buffer，与 EventStore / Replay / Prompt Cache 全脱节

不补上这个语义，模型会被迫在"硬中断重启"与"等用户憋到 turn 边界再说"之间二选一，前者损失成本与上下文，后者损失体验。

## 2. 决策

### 2.1 总纲

引入 `SteeringQueue`：一个绑定到 Session 的轻量队列，业务可在运行期任意时刻 `push_steering(...)` 入队；Engine 在主循环的预设 **Safe Merge Point** 之前 drain，把队列中的消息合并为下一轮 `model.infer` 的 user 消息一部分。**绝不修改已发出的 prompt** 与 system 段，从而严守 ADR-0003 Prompt Cache Hard Constraint。

设计目标（按优先级）：

1. **Cache-safe**：drain 与合并仅发生在 turn 边界（model.infer 之前）；任何已经发出过的 prompt 段不会被改写
2. **可序列化**：每条 steering 进入 EventStore（`Event::SteeringMessageQueued / Applied / Dropped`），完全可重放
3. **可观测**：与 ADR-0007 Permission Events 同一观测面；UI 可消费这些事件展示"队列状态 / 已应用 / 已丢弃"
4. **可背压**：队列容量、TTL、溢出策略都是显式参数，按 q5 决议默认 `drop_oldest`
5. **不增加新中断**：`InterruptToken` 的硬中断语义保持不变；Steering 是与之**互补**的另一条路径

### 2.2 数据模型

```rust
/// 用户在运行期"软引导"主 Agent 的一条消息。
#[derive(Debug, Clone)]
pub struct SteeringMessage {
    pub id: SteeringId,                           // = TypedUlid<SteeringScope>
    pub session_id: SessionId,
    pub run_id: Option<RunId>,                    // None = 当前 idle 时入队
    pub kind: SteeringKind,                        // Append / Replace / NudgeOnly
    pub priority: SteeringPriority,                // Normal | High（用户显式标记）
    pub body: SteeringBody,                        // §2.3
    pub queued_at: SystemTime,
    pub correlation_id: Option<CorrelationId>,    // 与 ADR-0001 Event Envelope 共用
    pub source: SteeringSource,                    // §2.7
}

#[non_exhaustive]
pub enum SteeringKind {
    /// 追加补充意图（默认）：本条 + 已有 → 合并为下一条 user 消息
    Append,
    /// 替换最近一条尚未 drain 的 Append（典型："改主意了"）
    Replace,
    /// 轻量提示：仅写入 EventStore，不进入 prompt（适合"只是想留痕"）
    NudgeOnly,
}

#[non_exhaustive]
pub enum SteeringBody {
    /// 纯文本（最常见）
    Text(String),
    /// 受限结构化字段（用于 UI 表单，例如"加目录 = packages/, 跳过 = tests/"）
    Structured {
        instruction: String,
        addenda: BTreeMap<String, Value>,        // 不会以 schema 形式注入，仅展开为文本
    },
}

#[derive(Debug, Clone, Copy)]
pub enum SteeringPriority {
    Normal,
    High,                                          // 命中下一个 safe point 一定 drain，不受窗口去重
}

#[non_exhaustive]
pub enum SteeringSource {
    User,
    Plugin { plugin_id: PluginId },               // ADR-0014/0015 capability 受限
    AutoMonitor { rule_id: String },              // 业务侧"看到 X 自动注入提醒"
}
```

### 2.3 SteeringQueue 与 SteeringPolicy

```rust
pub struct SteeringQueue {
    inner: Mutex<VecDeque<SteeringMessage>>,
    policy: SteeringPolicy,
    notify: tokio::sync::Notify,
}

pub struct SteeringPolicy {
    /// 队列容量上限（默认 8）
    pub capacity: usize,
    /// 单条消息从 queued 到 applied 的最大停留时间（默认 60s）
    pub ttl: Duration,
    /// 容量已满时的处理策略（按 q5 决议：默认 DropOldest）
    pub overflow: SteeringOverflow,
    /// 同 Run 内对相同 `body_hash` 的去重窗口（默认 1500ms）
    pub dedup_window: Duration,
}

#[non_exhaustive]
pub enum SteeringOverflow {
    /// 默认：扔掉最早未 drain 的一条；emit `Event::SteeringMessageDropped { reason: Capacity }`
    DropOldest,
    /// 扔掉新入队的一条
    DropNewest,
    /// 阻塞 `push_steering(...)` 直到队列有空位（业务侧自负 backpressure）
    BackPressure,
}

impl Default for SteeringPolicy {
    fn default() -> Self {
        Self {
            capacity: 8,
            ttl: Duration::from_secs(60),
            overflow: SteeringOverflow::DropOldest,
            dedup_window: Duration::from_millis(1500),
        }
    }
}
```

### 2.4 Session 对外 API

`harness-session.md §2.1 Session` 增量：

```rust
impl Session {
    /// 运行期任意时刻可调用；与 `interrupt()` 互补。
    pub async fn push_steering(
        &self,
        msg: SteeringRequest,
    ) -> Result<SteeringId, SessionError>;

    /// 取出当前队列快照（仅用于 UI 展示，不修改队列）
    pub fn steering_snapshot(&self) -> SteeringSnapshot;

    /// （仅 testing feature）业务直接 cancel 一条已入队但未 drain 的消息
    #[cfg(feature = "testing")]
    pub fn cancel_steering(&self, id: SteeringId) -> Result<(), SessionError>;
}

pub struct SteeringRequest {
    pub kind: SteeringKind,
    pub body: SteeringBody,
    pub priority: Option<SteeringPriority>,        // None → Normal
    pub correlation_id: Option<CorrelationId>,
    pub source: SteeringSource,
}
```

`SessionInner`（§3 内部状态）新增字段：

```rust
struct SessionInner {
    // ... 既有字段
    steering_queue: Arc<SteeringQueue>,
}
```

### 2.5 Engine 主循环 Safe Merge Point

`harness-engine §3 Agent Loop 主流程` 在 `[检查 iteration_budget / interrupt_token]` 之后、`[model.infer(prompt)]` 之前插入新阶段：

```text
─── Iteration Loop ──────────────────────────────
    │
    ▼
[检查 iteration_budget / interrupt_token]
    │
    ▼
[steering.drain_and_merge()]                                ← 新增（ADR-0017）
    │  ├─ 拿出队列中 visible_at <= now 且未 drop 的消息
    │  ├─ 合并语义：
    │  │    Append      → 顺序拼接到下一轮 user 消息尾部
    │  │    Replace     → 仅保留最新一条 Replace（同时清掉之前未 drain 的 Append）
    │  │    NudgeOnly   → 不进 prompt；只 emit `Event::SteeringMessageApplied`
    │  ├─ Event::SteeringMessageApplied { ids: [...], merged_into_message_id }
    │  └─ ttl 已过 / dedup 命中 → Event::SteeringMessageDropped { id, reason }
    │
    ▼
[Hook::UserPromptSubmit on synthesized user_message]        ← 仅当本轮有合并产生的 user 消息
    │
    ▼
[context_engine.assemble] → AssembledPrompt
    │
    ▼
[model.infer(prompt)] → stream events
    │
    ... (后续与既有流程一致)
```

**关键约束**：

- `steering.drain_and_merge()` **不会** 修改已经发出过的任何消息；它只构造**下一轮**的 user 消息
- `[Hook::UserPromptSubmit]` 见到的是合并后的消息（既有 hook 链不变）；这一步可继续 RewriteInput / Block，但 **不得**用于 cache 拆分（hook 链尾仍走 prompt cache 锁定字段校验，与 v1.6 既有要求一致）
- 当队列为空时，整个新阶段是 0 操作 + 0 事件
- `interrupt_token` 优先级仍高于 steering：硬中断触发时先终止当前 model/tool，再处理 RunEnded；steering 队列残留消息在 RunEnded 后由 `Session::run_turn` 在下一次启动时 drain（带 `Event::SteeringMessageApplied { run_id: <new_run_id> }`），或受 ttl 自然过期

### 2.6 与 ADR-0003 Prompt Cache 的协同

- `system` 段、`renderedSystemPrompt`、toolset schema 一律不动
- 合并入 prompt 的位置严格落在"下一轮 user 消息"段；此段在 cache 策略里属于"近 N 条非 system" 的滚动区，本来就受 cache miss 影响 — Steering 不引入新的 cache 风险
- 不允许 Steering 触发 `reload_with`：所有需要重启 Session 的请求只能由 `reload_with` 显式表达，与既有路径一致

### 2.7 Source 与 Capability

| Source | 触发权 | 速率限制 |
|---|---|---|
| `User` | 任意（受 SDK 暴露的入口形态决定） | `SteeringPolicy.capacity` + `dedup_window` |
| `Plugin { plugin_id }` | 仅声明 `capabilities.steering = true` 的 AdminTrusted 插件可调用；user-controlled 插件 fail-closed | 复用 capacity；priority 强制为 Normal |
| `AutoMonitor { rule_id }` | 业务侧自有 monitor（例如外部告警注入"显存爆了快停"）；rule_id 受 audit | 与 `User` 同 capacity；可选 `min_interval`（业务自定） |

`harness-plugin §2.4 PluginActivationContext` 新增可选字段：

```rust
pub struct PluginActivationContext {
    // ... 既有字段
    pub steering: Option<Arc<dyn SteeringRegistration>>, // 仅当 manifest declares
}

#[async_trait]
pub trait SteeringRegistration: Send + Sync {
    async fn push(&self, msg: SteeringRequest) -> Result<SteeringId, RegistrationError>;
}
```

业务必须在 `team_config.toml` 里 explicitly 列出"哪些 AdminTrusted 插件可推 steering"，否则即使 manifest 声明也 fail-closed。

### 2.8 与硬中断（InterruptToken）的关系

| 维度 | InterruptToken（硬中断）| SteeringQueue（软引导） |
|---|---|---|
| 终止 Run？ | 是 | 否 |
| 释放当前 model.infer / tool 流？ | 是 | 否 |
| 修改 prompt？ | 否（Run 已终止）| 不修改已发出；仅追加下一轮 user |
| 进入 EventStore？ | `RunEnded { reason: Interrupted }` | `SteeringMessageQueued / Applied / Dropped` |
| 默认 UI 行为 | 红色"取消"按钮 | 蓝色"插话"按钮 |

二者**互补**；SDK 不引入"半中断"或"挂起 model.infer 重启"等中间形态（保持 P3 Single Loop, Single Brain）。

### 2.9 多事件序列示例

```text
T0: Session::run_turn(初始 prompt)
T1: Engine 进入 Iteration Loop（无 steering queue 内容）
T2: Engine 调用 model.infer，开始 stream
T3: 用户 push_steering Append "再加 packages/ 目录"
        → Event::SteeringMessageQueued { id: s1, kind: Append, queued_at: T3 }
T4: Engine 当前轮 stream 结束，dispatch 工具
T5: 工具完成，进入下一轮 Iteration Loop
T6: Engine 在 [steering.drain_and_merge] 阶段：
        - 拿到 s1
        - 合并到下一轮 user 消息
        - Event::SteeringMessageApplied { ids: [s1], merged_into_message_id: m_X }
T7: model.infer 接收到 "上轮工具结果 + （新增）请再扫 packages/ 目录"
T8: 用户 push_steering Replace "改成只看 packages/，别的别动"
        → Event::SteeringMessageQueued { id: s2, kind: Replace }
T9: 同一轮 Iteration Loop 末尾再到 drain：s2 命中 Replace，吃掉所有未 drain Append（无）
T10: 队列空，Iteration 继续
```

## 3. 替代方案

### 3.1 在 Session 之外开"外部 buffer"，由业务侧自管

- ✅ 不动 SDK
- ❌ Replay 永远缺这一段；不同业务实现不一致；与 ADR-0001 / ADR-0003 完全脱节
- ❌ 无法与 `interrupt()` 在语义上互补（业务自管 buffer 不知道何时算"安全合并点"）

### 3.2 把 Steering 实现成 "软中断 + reload_with"

- ❌ `reload_with` 是 cache-busting 操作（即使 in-place 也有 reload 成本）
- ❌ 与 ADR-0003 §1.x reload 分类器冲突
- ❌ 杀鸡用牛刀

### 3.3 Steering 直接修改当前轮的 prompt

- ❌ 破坏已发出 prompt 的不可变性
- ❌ 与 ADR-0003 直接冲突
- ❌ 无法回滚

### 3.4 Safe Merge Point + Queue（采纳）

- ✅ 队列 + drain + safe-point 模式直接对齐 OC-11 / HER-029 / CC-06 三家共识
- ✅ 与 EventStore 自然集成；Replay 时按事件顺序重放即可
- ✅ Cache 不受影响；硬中断语义保持不变

## 4. 影响

### 4.1 正向

- 用户体验：新增"插话"路径，不破坏 Run，不损失 token
- 可观测：Steering 事件进入 EventStore，可被 Hook / Audit / UI 全程消费
- 抽象一致：依然是单 Loop / Single Brain（P3）；多消息路径统一收敛在 `Session::run_turn` + `Session::push_steering` + `Session::interrupt` 三个 API
- 可重放：Steering 事件可序列化、可回放、可 diff

### 4.2 代价

- `harness-session` 多一个 `SteeringQueue`（约 2 个 Mutex / 1 个 Notify）
- `harness-engine §3` 主循环图新增 1 阶段
- `harness-contracts §3.3` Event 新增 3 个 variant
- `event-schema.md` 新增 §3.5.1 Steering Events 整段
- 文档负担一次性增加（v1.8 修订）

### 4.3 兼容性

- v1.8 仍是文档级修订，无代码兼容压力
- `feature_flags.md` 新增 `steering_queue`（默认 **on**：行为退化为"队列容量 0 时永远 DropNewest"，业务可显式关闭）
- 既有 `interrupt()` 路径不变；既有 `run_turn()` 不变；新增 API 是叠加式

## 5. 落地清单（仅文档面）

| 项 | 责任文档 | 说明 |
|---|---|---|
| `SteeringMessage / SteeringKind / SteeringBody / SteeringPriority / SteeringSource` | `crates/harness-contracts.md §3.4` | 共享枚举与数据结构 |
| `SteeringQueue / SteeringPolicy / SteeringOverflow` | `crates/harness-session.md` 新增 §2.7 | 节加在 §2.6 Projection 之前 |
| `Session::push_steering / steering_snapshot` | `crates/harness-session.md §2.1` | 增量 |
| `SessionInner.steering_queue` | `crates/harness-session.md §3` | 增量 |
| Engine 主循环图新增 `[steering.drain_and_merge]` 阶段 | `crates/harness-engine.md §3` | 13 阶段 → 14 阶段 |
| `Event::SteeringMessageQueued / Applied / Dropped` | `crates/harness-contracts.md §3.3` + `event-schema.md §3.5.1`（新增段）| 与 ADR-0010 ToolResultOffloaded 同节 |
| `SteeringRegistration` capability handle | `crates/harness-plugin.md §2.4` + `extensibility.md §11` | 增量 |
| `feature-flags.md` `steering_queue`（默认 on） | `feature-flags.md` | 增量 |
| `api-contracts.md` 新增 `Session::push_steering` | `api-contracts.md` §x.x | 增量 |
| `context-engineering.md` 说明 Steering 消息合并不破坏缓存 | `context-engineering.md` §x | 增量 |
| `comparison-matrix.md` R-20 回填 | `reference-analysis/comparison-matrix.md` | → ADR-0017 |

## 6. 参考证据

| Evidence ID | 来源 | 要点 |
|---|---|---|
| OC-11 | `reference-analysis/evidence-index.md` | OpenClaw queue mode：`inflight + queue` 由 lane 维护，turn 边界 drain |
| HER-029 | 同上 | Hermes Gateway `_pending_messages` adapter 队列 + runner 命令拦截"双 guard" |
| HER-031 | 同上 | Hermes runner 在 iteration 末尾查 pending；与 EventStore 协同 |
| CC-06 | 同上 | Claude Code `query.ts` 主循环把 steering 在下一轮 model.infer 前 prepend 的语义 |
| ADR-0001 | `adr/0001-event-sourcing.md` | Steering 事件可序列化、可重放 |
| ADR-0003 | `adr/0003-prompt-cache-locked.md` | Safe Merge Point 严守已发出 prompt 不可变 |
| ADR-0007 | `adr/0007-permission-events.md` | Steering 事件与 Permission 事件同一观测面 |
| ADR-0010 | `adr/0010-tool-result-budget.md` | Steering body 不走 ResultBudget（不是 tool 输出），但走 envelope 大小限制（默认 8 KiB / 条） |
