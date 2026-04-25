# `octopus-harness-journal` · L1 原语 · EventStore + Projection + Snapshot + BlobStore SPEC

> 层级：L1 · 状态：Accepted
> 依赖：`harness-contracts`

## 1. 职责

提供 **Append-Only Event Store + Projection + Snapshot + BlobStore** 的抽象与默认实现。是 ADR-001 Event Sourcing 的承载者。

**Octopus 产品集成规则**：

- `runtime/events/*.jsonl` 是产品内 runtime event / audit stream 的权威来源。
- `data/main.db` 只保存结构化状态、projection、索引、hash 与元数据。
- `SqliteEventStore` 是通用 SDK 后端，供独立嵌入方或非 Octopus 产品形态使用；不得在 Octopus 产品基线中用 `data/main.db` 作为 EventStore 真相源。
- 如业务需要从 JSONL 构建 SQLite 查询面，应实现 projection writer，而不是双写两套事件真相源。

**核心能力**：

- Append-only Event 写入
- 按 cursor 读取（支持 replay）
- Snapshot 缓存（加速 Projection 重建）
- Compaction 血缘链（父→子 session 关联，对齐 HER-023）
- Tenant-aware（对齐 OC-01 默认单租户）
- **BlobStore 默认实现**：把 Event 里的 `BlobRef` 映射到实际字节存取

## 2. 对外 API

### 2.1 核心 Trait

```rust
#[async_trait]
pub trait EventStore: Send + Sync + 'static {
    async fn append(
        &self,
        tenant: TenantId,
        session: SessionId,
        events: &[Event],
    ) -> Result<JournalOffset, JournalError>;

    async fn read(
        &self,
        tenant: TenantId,
        session: SessionId,
        cursor: ReplayCursor,
    ) -> Result<BoxStream<Event>, JournalError>;

    async fn snapshot(
        &self,
        tenant: TenantId,
        session: SessionId,
    ) -> Result<Option<SessionSnapshot>, JournalError>;

    async fn save_snapshot(
        &self,
        tenant: TenantId,
        snapshot: SessionSnapshot,
    ) -> Result<(), JournalError>;

    async fn compact_link(
        &self,
        parent: SessionId,
        child: SessionId,
        reason: ForkReason,
    ) -> Result<(), JournalError>;

    async fn list_sessions(
        &self,
        tenant: TenantId,
        filter: SessionFilter,
    ) -> Result<Vec<SessionSummary>, JournalError>;

    async fn prune(
        &self,
        tenant: TenantId,
        policy: PrunePolicy,
    ) -> Result<PruneReport, JournalError>;
}
```

### 2.1.1 `SessionFilter` & `SessionSummary`

`list_sessions` 的查询参数与返回值；**只暴露 Journal 自身能直接拿到的元数据**，富文本（如最近一条消息预览）属于 `harness-session` 的派生层。

```rust
pub struct SessionFilter {
    /// 仅列出 created_at >= since 的 session
    pub since: Option<DateTime<Utc>>,
    /// 仅列出指定 EndReason；None 表示所有
    pub end_reason: Option<EndReason>,
    /// 是否解析 compaction_lineage 把"父-子链"合并为一行（HER-023 同款）
    pub project_compression_tips: bool,
    pub limit: u32,
}

pub struct SessionSummary {
    pub session_id: SessionId,
    pub created_at: DateTime<Utc>,
    pub last_event_at: DateTime<Utc>,
    pub event_count: u64,
    pub end_reason: Option<EndReason>,
    /// 当 `project_compression_tips = true` 时，多代 session 折叠为根的 tip
    pub root_session: Option<SessionId>,
}
```

> 详细富视图（最近消息、token 使用、tool 列表）由 `harness-session` 在 SessionProjection 之上构造，不进入 Journal SQL 层。

### 2.2 Cursor

```rust
pub enum ReplayCursor {
    FromStart,
    FromOffset(JournalOffset),
    FromSnapshot(SnapshotId),
    FromTimestamp(DateTime<Utc>),
    /// 从"now - since"开始读到当前末尾后**结束**流；非 follow 模式。
    /// 真正的 follow（即长连接订阅 + offset 续接）属于 ADR-候选项，
    /// 见 §6.4「远端订阅与回放语义」。
    Tail { since: Duration },
}
```

### 2.3 Projection Trait

```rust
pub trait Projection: Sized + Send + Sync {
    type State;

    fn initial() -> Self::State;
    fn apply(state: &mut Self::State, event: &Event) -> Result<(), ProjectionError>;

    fn replay<'a>(events: impl IntoIterator<Item = &'a Event>) -> Result<Self::State, ProjectionError> {
        let mut state = Self::initial();
        for ev in events {
            Self::apply(&mut state, ev)?;
        }
        Ok(state)
    }
}
```

### 2.4 Snapshot

```rust
pub struct SessionSnapshot {
    pub session_id: SessionId,
    pub tenant_id: TenantId,
    pub offset: JournalOffset,
    pub taken_at: DateTime<Utc>,
    pub body: SnapshotBody,
}

pub enum SnapshotBody {
    Full(Vec<u8>),  // bincode
    Delta { base: SnapshotId, patch: Vec<u8> },
}
```

### 2.5 Compaction Link

```rust
pub struct CompactionLineage {
    pub child_session: SessionId,
    pub parent_session: SessionId,
    /// 复用 `harness-contracts::ForkReason`（定义见 `event-schema.md` §3 SessionForked）。
    /// 不再单独维护 `CompactReason`：fork 与 compaction 在 octopus 中是同一棵血缘树。
    pub reason: ForkReason,
    pub linked_at: DateTime<Utc>,
}
```

### 2.6 `AuditQuery` DSL

`AuditQuery` 是 Journal 暴露的**只读、声明式**审计接口。它在 `EventStore::read` 之上提供"按业务维度抽取/拼接事件"的能力，避免业务方手撸 SQL 把存储后端的细节扩散出去。这一层是 `audit-api`（`docs/architecture/harness/api-contracts.md`）的下层实现，跨进程的 RPC 形态见该文档。

> 下文的 `EventKind` / `PermissionSubjectDiscriminant` / `DecisionDiscriminant` / `DecidedByDiscriminant` 均由 `harness-contracts` 通过 `strum::EnumDiscriminants` 在对应 enum 上派生（见 `crates/harness-contracts.md §3.1` 的 derive 列表），**不**重新定义。本节的样例代码以 enum-set / `HashSet` 写法呈现是 SDK 内 trait 形态；HTTP 形态使用字符串名（如 `"PermissionRequestSuppressed"`），由 `api-contracts.md` 转换层负责双向映射。

```rust
#[async_trait]
pub trait AuditStore: Send + Sync + 'static {
    async fn query(&self, q: AuditQuery) -> Result<AuditPage, JournalError>;
}

pub struct AuditQuery {
    pub tenant: TenantId,
    pub scope: AuditScope,
    pub filter: AuditFilter,
    pub stitch: Vec<AuditStitch>,
    pub order: AuditOrder,
    pub limit: u32,
    pub cursor: Option<AuditCursor>,
}

pub enum AuditScope {
    Session(SessionId),
    Run(SessionId, RunId),
    ToolUse(SessionId, ToolUseId),
    Fingerprint { fingerprint: ExecFingerprint, since: DateTime<Utc> },
    Tenant { since: DateTime<Utc>, until: DateTime<Utc> },
}

pub struct AuditFilter {
    /// 仅返回这些 EventKind；空集表示全部。
    pub event_kinds: HashSet<EventKind>,
    /// 仅返回这些 ToolUseId；与 event_kinds 取交集。
    pub tool_use_ids: HashSet<ToolUseId>,
    /// 限定 PermissionRequested.subject 类别（如只看 DangerousCommand）。
    pub permission_subjects: HashSet<PermissionSubjectDiscriminant>,
    /// 限定 Decision 终值。
    pub decisions: HashSet<DecisionDiscriminant>,
    /// 限定 DecidedBy 来源（如只看 ParentForwarded）。
    pub decided_by: HashSet<DecidedByDiscriminant>,
    /// 限定 Severity ≥ given。
    pub min_severity: Option<Severity>,
}

pub enum AuditStitch {
    /// 命中 `PermissionRequestSuppressed` 时，自动把它的 `original_request_id` /
    /// `original_decision_id` 指向的 `PermissionRequested + Resolved` 也拼回结果集，
    /// 标记为 `causal_origin`。
    SuppressionOrigin,
    /// 命中 `SubagentPermissionForwarded` 时，把父 Session 的对应
    /// `PermissionRequested + Resolved` 也拉进来，标记为 `parent_resolution`。
    /// 反之亦然：命中父 `PermissionRequested` 若有子代理来源（`source_session_id`），
    /// 自动拉子侧的 `Forwarded + SubagentPermissionResolved`。
    ForwardedDuplex,
    /// 命中 `ToolUseDenied` / `ToolUseApproved` 时，自动拼对应的
    /// `PermissionRequested + Resolved` 与 `ToolUseRequested`。
    ToolUseLineage,
    /// 命中 `HookTriggered { outcome_summary.blocked_reason: Some(_) }` 时，
    /// 拼对应的 `ToolUseDenied { reason: HookBlocked, handler_id }`。
    HookBlockOrigin,
}

pub enum AuditOrder {
    /// 按 `EventId` 升序（默认；最稳定，跨多 session 用同一全局序）。
    EventIdAsc,
    /// 按 `at` 升序；适合人读，但跨节点不严格单调。
    TimeAsc,
    /// 按 `causation_id` 拓扑序（事件因果树深度优先）；高成本，仅 Tool/Permission lineage 推荐。
    CausationDfs,
}

pub struct AuditPage {
    pub items: Vec<AuditRecord>,
    pub next_cursor: Option<AuditCursor>,
    pub stitched_origins: HashMap<EventId, EventId>,  // suppressed → origin
}

pub struct AuditRecord {
    pub event: Event,
    pub session_id: SessionId,
    pub run_id: Option<RunId>,
    pub tags: Vec<AuditTag>,        // {causal_origin, parent_resolution, ...}
    pub stitched_from: Option<EventId>,
}
```

#### 2.6.1 典型查询样例

**Q1：某 ToolUse 的最终决策（含 dedup 跳转）**

```rust
let q = AuditQuery {
    tenant: tenant_id,
    scope: AuditScope::ToolUse(session_id, tool_use_id),
    filter: AuditFilter {
        event_kinds: {EventKind::PermissionRequested,
                      EventKind::PermissionResolved,
                      EventKind::PermissionRequestSuppressed,
                      EventKind::ToolUseApproved,
                      EventKind::ToolUseDenied},
        ..Default::default()
    },
    stitch: vec![AuditStitch::SuppressionOrigin, AuditStitch::ToolUseLineage],
    order: AuditOrder::EventIdAsc,
    limit: 64,
    cursor: None,
};
```

返回值序列化后形如：

```text
ToolUseRequested(t=1.000)
  └─ PermissionRequestSuppressed(reason=RecentlyAllowed, original=req_42)
       ├─ [stitched] PermissionRequested(req_42, t=0.500)
       └─ [stitched] PermissionResolved(req_42, decision=AllowOnce, t=0.520)
  └─ ToolUseApproved(t=1.001)
```

**Q2：某 Session 在窗口内被 dedup 了多少次**

```rust
let q = AuditQuery {
    tenant: tenant_id,
    scope: AuditScope::Session(session_id),
    filter: AuditFilter {
        event_kinds: {EventKind::PermissionRequestSuppressed},
        ..Default::default()
    },
    stitch: vec![],
    order: AuditOrder::TimeAsc,
    limit: 1024,
    cursor: None,
};
```

调用方按 `reason` 维度分桶聚合即可（`SuppressionReason::JoinedInFlight` / `RecentlyAllowed` / `RecentlyDenied` / `RecentlyTimedOut`）。

**Q3：某 fingerprint 的所有历史决策（跨 Session）**

```rust
let q = AuditQuery {
    tenant: tenant_id,
    scope: AuditScope::Fingerprint { fingerprint, since: cutoff },
    filter: AuditFilter {
        event_kinds: {EventKind::PermissionResolved},
        ..Default::default()
    },
    stitch: vec![AuditStitch::SuppressionOrigin],  // 把被去重的也拉回原决策
    order: AuditOrder::TimeAsc,
    limit: 256,
    cursor: None,
};
```

> `Fingerprint` 不是 Journal 主键，需要由 `JsonlEventStore` / `SqliteEventStore` 在 append 时维护一份 `(tenant, fingerprint) → [event_id]` 倒排索引；详见 §4.3。

**Q4：某 Subagent 的所有 forwarded permissions（双面拼接）**

```rust
let q = AuditQuery {
    tenant: tenant_id,
    scope: AuditScope::Run(parent_session_id, parent_run_id),
    filter: AuditFilter {
        event_kinds: {EventKind::SubagentPermissionForwarded,
                      EventKind::SubagentPermissionResolved},
        ..Default::default()
    },
    stitch: vec![AuditStitch::ForwardedDuplex],  // 自动拉父子两端
    order: AuditOrder::CausationDfs,
    limit: 64,
    cursor: None,
};
```

返回结果会同时包含子代理的 `Forwarded / Resolved` 与父 Session 的 `PermissionRequested / Resolved`，两者通过 `causation_id` 显式关联，UI / SIEM 可直接展示完整审批链。

**Q5：所有 fail-closed 事件（合规审计入口）**

```rust
let q = AuditQuery {
    tenant: tenant_id,
    scope: AuditScope::Tenant { since, until },
    filter: AuditFilter {
        event_kinds: {EventKind::PermissionPersistenceTampered,
                      EventKind::ToolUseDenied},
        decisions: {DecisionDiscriminant::DenyOnce,
                    DecisionDiscriminant::DenyPermanent},
        ..Default::default()
    },
    stitch: vec![AuditStitch::HookBlockOrigin],
    order: AuditOrder::TimeAsc,
    limit: 4096,
    cursor: Some(prev_cursor),
};
```

#### 2.6.2 实现约束

| 维度 | 约束 |
|---|---|
| **租户隔离** | `AuditQuery.tenant` 必须匹配调用上下文；跨租户查询直接 `JournalError::TenantMismatch` |
| **stitch 的成本** | `SuppressionOrigin` 与 `ForwardedDuplex` 各最多拉**一跳**关联事件，避免 `causation_id` 链上的递归塌方 |
| **分页** | `AuditCursor` 是不透明 byte string；后端可自由编码（offset / event_id），但不得让客户端解析其内部结构 |
| **PII** | `AuditRecord.event` 沿用 EventStore 的 redact pipeline；不为 `AuditQuery` 单独再走脱敏 |
| **不写权限** | `AuditStore` 只读；任何"批量删除"接口走 `EventStore::prune`，不混入 `AuditQuery` |

跨进程暴露形态（HTTP / gRPC）见 `api-contracts.md`；本节定义的 `AuditQuery` 是 SDK 内 trait，业务方可直接在同一进程内调用，绕开 RPC 序列化开销。

> **取消 `CompactReason`（v0.x 临时枚举）**
>
> 历史版本曾定义 `CompactReason { AutoCompact, UserRequested, HotReloadFork, ForkForIsolation }`，
> 与 `ForkReason` 语义高度重合。设计取舍后**统一为 `ForkReason`**：
>
> | 旧 `CompactReason` | 新 `ForkReason` |
> |---|---|
> | `AutoCompact` | `Compaction` |
> | `UserRequested` | `UserRequested` |
> | `HotReloadFork` | `HotReload` |
> | `ForkForIsolation` | `Isolation` |
>
> 实施层面：`compact_link` 直接收 `ForkReason`；`SessionForkedEvent.fork_reason` 即同一来源。

对齐 HER-023：父 `Event::RunEnded { reason: Compacted }` + 子 `Event::SessionForked { parent, reason }`；`list_sessions_rich` 用 tip 投射"一段连续对话 = 一行"。

## 3. `BlobStore` 默认实现

BlobStore trait 定义在 `harness-contracts` §3.6；本 crate 提供三种默认实现。

### 3.1 `FileBlobStore`

```rust
pub struct FileBlobStore {
    root: PathBuf,
    max_blob_size: u64,         // default 128 MB
    hash_algo: HashAlgo,        // default Blake3
    retention_enforcer: Arc<RetentionEnforcer>,
    /// 内容寻址：`true` 时 `BlobId = blake3(bytes)`，相同字节复用同一 BlobRef
    /// 并跳过实际写入；`false` 时 `BlobId = ULID`（每次 put 都是新 blob）。
    /// 默认 `false`——内容寻址在多用户场景下会引入跨 session 的引用泄漏风险，
    /// 仅当业务场景明确需要去重（如 image / 大型工具 fixture）才开启。
    content_addressed: bool,
}
```

- 路径约定：`<root>/<tenant>/<blob_id_prefix_2>/<blob_id>.bin`
- put 时计算 hash，写完 rename 实现原子性（write-then-rename）
- `head` 仅读 metadata 文件（`.meta.json`），不读实际字节
- **内容寻址模式（`content_addressed = true`）**：
  - `put` 计算 `hash = blake3(bytes)` → `blob_id = hash`，若 `<root>/<tenant>/<hash[..2]>/<hash>.bin` 已存在则直接返回既存 `BlobRef`，**不重写文件**；
  - `BlobMeta.retention` 取已有与新请求中**保留更宽**的策略（`RetainForever > TtlDays > TenantScoped > SessionScoped`）；
  - 同一 tenant 内严格去重；**跨 tenant 不共享**（路径包含 `tenant_id`），避免侧信道。
- **GC 与去重的相互作用**：内容寻址下需引用计数（详见 §3.5）。无引用时按 `retention` 释放。

### 3.2 `SqliteBlobStore`

```rust
pub struct SqliteBlobStore {
    pool: sqlx::SqlitePool,
    inline_threshold: u64,  // <= 阈值存 BLOB 列，> 阈值落盘走 FileBlobStore
    file_backend: Option<Arc<FileBlobStore>>,
}
```

SQL:

```sql
CREATE TABLE IF NOT EXISTS blobs (
    tenant_id TEXT NOT NULL,
    blob_id TEXT NOT NULL,
    size INTEGER NOT NULL,
    content_hash BLOB NOT NULL,
    content_type TEXT,
    retention_kind TEXT NOT NULL,
    retention_until TEXT,
    body BLOB,                     -- 小 blob 直接存；大 blob 走 file_backend
    created_at TEXT NOT NULL,
    PRIMARY KEY (tenant_id, blob_id)
) STRICT;
```

### 3.3 `InMemoryBlobStore`

```rust
pub struct InMemoryBlobStore {
    blobs: DashMap<(TenantId, BlobId), (BlobMeta, Bytes)>,
}
```

测试用；不持久化。

### 3.4 `RetentionEnforcer`

```rust
pub struct RetentionEnforcer {
    strategy: RetentionStrategy,
    journal_link: Option<Arc<dyn EventStore>>,
}

pub enum RetentionStrategy {
    Opportunistic,   // 随每次 put/get 抽样 GC
    Periodic(Duration),
    OnJournalPrune,  // 跟 Journal prune 绑定
}
```

`BlobStore::delete` 走 `RetentionEnforcer` 的 gate，业务 prune Journal 时连带清理 session-scoped blobs。

### 3.5 Blob GC 算法

GC 由 `RetentionEnforcer` 与 `EventStore::prune` 协同完成。**默认非内容寻址**模式下退化为按 `retention_kind` 直接 sweep；启用内容寻址时改为标记-清扫，伪代码：

```text
Input: 触发源 = { OnJournalPrune | Periodic | Opportunistic }
Output: BlobGcReport { scanned, retained, freed_bytes }

1. 计算"活跃引用集" live_refs:
   live_refs ← ⋃ over (tenant, session) in EventStore.list_sessions(...)
              of   collect_blob_refs_in_events(tenant, session)
   说明：collect_blob_refs_in_events 扫 Event::ToolUseCompleted/
         ToolResultOffloaded/UserMessageAppended/MemoryUpserted 的 BlobRef 字段。
2. 对每个候选 blob（按 tenant 分桶，逐桶处理避免锁争用）:
   a. 读 BlobMeta.retention：
      - SessionScoped(sid)：若 sid ∉ alive_sessions，标记 evict
      - TenantScoped：若该 tenant 已被 prune 全部 session，标记 evict
      - TtlDays(n)：若 now - created_at > n 天，标记 evict
      - RetainForever：跳过
   b. 内容寻址模式：仅当 blob_id ∉ live_refs 才允许 evict
      （即使 retention 到期，但仍被某 Event 引用 → 延后到引用消失）
3. 提交 evict：
   - FileBlobStore：先删 .bin，再删 .meta.json（崩溃容错：孤立 .meta.json 视为可清理）
   - SqliteBlobStore：DELETE WHERE blob_id IN (...)，每批 ≤ 500 行
4. 写 metric: `journal_blob_gc_freed_bytes_total`、`journal_blob_gc_scanned_total`
```

**触发节奏**：
- `Opportunistic`：每 N 次 `BlobStore::put` 抽样 1 次（默认 N=128）；
- `Periodic(d)`：后台 task；
- `OnJournalPrune`：在 `EventStore::prune` 收尾的同事务后串行执行（避免读到正在删的引用）。

**与并发写的冲突**：GC 步骤 1 的 `live_refs` 计算后、步骤 3 提交前若有新事件携带相同 `BlobRef`，可能误删。缓解：步骤 3 对每个 blob 在 `BlobStore` 端再读一次 mtime/refcount，并在 evict 前用 `RENAME → tombstone` 的两阶段提交，给 1 个 GC 周期的"宽限期"再真正删除。

## 4. 内置 EventStore 实现

### 4.1 `InMemoryEventStore`

```rust
pub struct InMemoryEventStore {
    events: DashMap<(TenantId, SessionId), Vec<Event>>,
    snapshots: DashMap<(TenantId, SessionId), SessionSnapshot>,
    lineage: parking_lot::RwLock<Vec<CompactionLineage>>,
}
```

用于测试；不持久化。

### 4.2 `JsonlEventStore`（对齐 `runtime/events/*.jsonl` 规范）

```rust
pub struct JsonlEventStore {
    root: PathBuf,
    fsync_policy: FsyncPolicy,
    rotation: RotationPolicy,
    /// Reader 端遇到坏行 / 截断尾行时的策略；写入路径不受影响
    read_policy: JsonlReadPolicy,
}

pub enum FsyncPolicy {
    Always,
    EveryNAppends(usize),
    Periodic(Duration),
    Never,
}

pub struct JsonlReadPolicy {
    /// 文件末尾不完整 JSON 行（崩溃中段写）：true=跳过并发指标，false=报错
    pub tolerate_partial_tail: bool,
    /// 中段坏行：true=跳过+发 `Event::JournalReadDegraded` 旁路事件，false=报错
    pub tolerate_invalid_lines: bool,
}

impl Default for JsonlReadPolicy {
    fn default() -> Self {
        Self { tolerate_partial_tail: true, tolerate_invalid_lines: false }
    }
}
```

- 路径约定：`<root>/<tenant>/<session>.jsonl`
- 每行一个 Event JSON

**写入并发与原子性**：

- 单行 ≤ `PIPE_BUF`（Linux 4096 / macOS 512）时 `O_APPEND` 提供原子追加；超过时退化为 `write()` 多次系统调用，可能与他进程交织。
- 因此**强制**在每次 `append` 前对 `<session>.jsonl` 加 **flock LOCK_EX**（Windows 用 `LockFileEx`），写完释放。这是 OpenClaw 同款做法（参见 `reference-analysis/openclaw.md`）。
- **跨进程语义**：flock 由 OS 强制；同一 session 文件被多个 octopus 进程并发写（CLI + Server 同时持仓）时仍安全。
- `fsync_policy` 控制持久化时机；崩溃后未 fsync 的尾行可能为不完整 JSON，由 `read_policy.tolerate_partial_tail` 处理。

**读取容错**（HER-021 类对齐）：

- `tolerate_partial_tail = true`（默认）：流式读到 EOF 前最后一行解析失败时，视为正常截断，不报错；产生 `journal_jsonl_partial_tail_total` 指标。
- `tolerate_invalid_lines`：默认 `false`，遇坏行直接 `JournalError::Serialization` 中断；改 `true` 时跳过坏行并通过旁路通道（不入 EventStore）发 `Event::JournalReadDegraded { offset, raw }` 给 observability。
- **不实现 DLQ**：单文件场景下坏行罕见，DLQ 反增维护成本；如有需要应迁移到独立的 `SqliteEventStore` 数据库文件，不能复用产品 `data/main.db` 作为事件真相源。

### 4.3 `SqliteEventStore`（通用 SDK 后端，对齐 HER-020/021/022）

`SqliteEventStore` 将 event body 写入 SQLite。它是 SDK 的可选后端，不是
Octopus 产品内 `data/main.db` 的默认职责。Octopus 产品内 SQLite 只承载
projection / index；事件 replay 以 `JsonlEventStore` 的 JSONL 为准。

```rust
pub struct SqliteEventStore {
    pool: sqlx::SqlitePool,
    write_lock: tokio::sync::Mutex<()>,
    fts_enabled: bool,
}
```

核心表：

```sql
CREATE TABLE IF NOT EXISTS events (
    tenant_id      TEXT    NOT NULL,
    session_id     TEXT    NOT NULL,
    offset         INTEGER NOT NULL,
    event_type     TEXT    NOT NULL,
    body           BLOB    NOT NULL,
    at             TEXT    NOT NULL,
    -- envelope 中的因果链字段（详见 event-schema.md §2 EventEnvelope）；
    -- 提取到列以便建索引，body 仍是事件唯一权威来源
    correlation_id TEXT,
    causation_id   TEXT,
    schema_version INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (tenant_id, session_id, offset)
) STRICT;

-- 因果链查询：定位某 turn / tool_use 的全部衍生事件
CREATE INDEX IF NOT EXISTS idx_events_correlation
    ON events(tenant_id, correlation_id)
    WHERE correlation_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_events_causation
    ON events(tenant_id, causation_id)
    WHERE causation_id IS NOT NULL;

-- 跨 session 时间扫描（如 Dashboard "最近 24h" 视图）
CREATE INDEX IF NOT EXISTS idx_events_at
    ON events(tenant_id, at);

CREATE TABLE IF NOT EXISTS snapshots (
    tenant_id  TEXT    NOT NULL,
    session_id TEXT    NOT NULL,
    offset     INTEGER NOT NULL,
    taken_at   TEXT    NOT NULL,
    body       BLOB    NOT NULL,
    PRIMARY KEY (tenant_id, session_id, offset)
) STRICT;

CREATE TABLE IF NOT EXISTS compaction_lineage (
    child_session  TEXT PRIMARY KEY,
    parent_session TEXT NOT NULL,
    reason         TEXT NOT NULL,  -- ForkReason 序列化
    linked_at      TEXT NOT NULL
) STRICT;

-- §4.3.1 kv_meta：存放 store 自身的可演化元数据；
-- 用作 prune / vacuum / migration 的最小 idempotent state，
-- 避免每次启动都要扫全表才能判断"上次清理到哪"。
CREATE TABLE IF NOT EXISTS kv_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
) STRICT;
-- 约定 key：
--   schema.version              -- 当前 DB schema 版本号
--   prune.last_run_at           -- 上次 auto-prune 时间戳
--   prune.last_horizon_offset   -- 上次清理停在哪条 offset
--   vacuum.last_run_at          -- 上次 VACUUM 时间戳
--   migration.last_applied      -- 最近一次完成的 migrator id

CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
    session_id  UNINDEXED,
    event_type  UNINDEXED,
    body,
    tokenize = 'porter unicode61'
);
```

**并发策略**：

- `PRAGMA journal_mode = WAL`、`PRAGMA synchronous = NORMAL`、`PRAGMA busy_timeout = 5000`
- `_execute_write`：`BEGIN IMMEDIATE` + 20~150ms 随机抖动重试（对齐 HER-021）
- 每 50 次写做 `PASSIVE checkpoint`，每 5000 次做 `RESTART checkpoint`
- 进程内串行：`SqliteEventStore.write_lock: tokio::sync::Mutex<()>`
- 跨进程：依赖 SQLite 自身的文件锁（WAL 模式下的 `SHARED → RESERVED → EXCLUSIVE`），**无需**应用层文件锁；同一 DB 文件被多进程并发持有时由 SQLite 保证写串行。

**FTS5 查询清洗**（A1，对齐 HER-022）：

下述 sanitizer 在 `SqliteEventStore::search_messages` 内部调用；不抽象成独立 trait（YAGNI），只保留模块级 `pub(crate) fn sanitize_fts5_query` 便于单测。

```text
sanitize_fts5_query(input: &str) -> SanitizedQuery
1. 限长：超过 256 字符直接拒绝（`JournalError::QueryTooLong`），防 FTS 内核栈撕裂。
2. 屏蔽 FTS5 元字符：`"` `*` `(` `)` `^` `:` 替换为空格；保留 `-` 仅在词首作为 NOT。
3. CJK 检测：若任意字符落在 CJK 区段（U+4E00-9FFF/U+3040-309F/U+30A0-30FF/U+AC00-D7AF），
   返回 SanitizedQuery::Like(escaped) —— 调用方走 LIKE '%kw%' fallback，
   因为 unicode61 tokenizer 对 CJK 不分词；高级方案是后续接入 jieba/cppjieba（不在本次范围）。
4. 否则返回 SanitizedQuery::Fts(quoted) —— 把每个 token 用双引号包裹后用 AND 连接，
   等价于 phrase-match 但禁用了用户自定义 OR/NEAR。
```

调用方根据返回类型分别走 `events_fts MATCH ?` 或 `body LIKE ?`，避免把脏查询直接拼进 SQL。

## 4.4 `VersionedEventStore`（Schema 迁移装饰器）

`VersionedEventStore<S>` 把任意 `EventStore` 包一层，读时透明应用 `EventMigrator` 链，把旧版本 envelope 迁移到当前版本：

```rust
pub struct VersionedEventStore<S: EventStore> {
    inner: S,
    migrators: Arc<MigratorChain>,
    strict: bool,
}
```

详细 API 与 `MigratorChain` 定义见 `event-schema.md` §7.2.1。

**关键约束**：

- 写入**永远**使用 `SchemaVersion::CURRENT`；装饰器不参与写入路径
- 读取时 envelope 的 `schema_version` 字段决定是否需要迁移
- `strict=true`（默认）找不到迁移路径即报 `JournalError::MigrationPathMissing`；`strict=false` 时跳过该事件并发 `Event::MigrationFailed`

**`MigratorChain::find_path(from, to)` 算法**：

`MigratorChain` 内部把每个已注册 `EventMigrator` 视作 `from_version → to_version` 的有向边，以**广度优先搜索**计算最短迁移路径：

```text
find_path(from: SchemaVersion, to: SchemaVersion) -> Option<Vec<&dyn EventMigrator>>
1. 若 from == to，返回 Some(vec![])。
2. 维护 BFS 队列 q: VecDeque<(SchemaVersion, Vec<edge_idx>)>，初始 push (from, []).
3. 维护 visited: HashSet<SchemaVersion> = {from}。
4. while let Some((v, path)) = q.pop_front():
     for edge in self.edges_from(v):    // 索引 edges_by_from: HashMap<Ver, Vec<usize>>
       if edge.to == to: return Some(path ++ [edge])
       if visited.insert(edge.to): q.push_back((edge.to, path ++ [edge]))
5. 返回 None（无连通路径）。
```

**复杂度**：边数 m、版本数 n，BFS 为 O(n+m)；构造期一次性 build `edges_by_from` 索引，后续每次查询 O(n+m)。

**注册期校验**：
- 检测重复边（same `(from, to)`）→ `panic!`，避免歧义；
- 检测形如 `v3 → v3` 的自环 → 允许（noop migrator，主要用于校验现有事件结构）；
- 不强制要求图无环：`v5 → v3` 这类反向迁移可用于"撤销已发布字段"，由调用方自行确保不出现 `find_path` 死循环（由 visited 集兜底）。

## 5. Tenant 隔离

```rust
impl EventStore for SqliteEventStore {
    async fn append(&self, tenant: TenantId, session: SessionId, events: &[Event]) -> Result<...> {
        // SQL WHERE 始终带 tenant_id
    }
}
```

所有公共方法 **第一参数** 都是 `TenantId`（即使单租户 `TenantId::SINGLE`），确保跨租户隔离。

## 6. Projection 示例

### 6.1 `SessionProjection`

```rust
pub struct SessionProjection {
    pub session_id: SessionId,
    pub messages: Vec<Message>,
    pub tool_uses: HashMap<ToolUseId, ToolUseRecord>,
    pub permission_log: Vec<PermissionRecord>,
    pub usage: UsageSnapshot,
    pub end_reason: Option<EndReason>,
    pub last_offset: JournalOffset,
}

impl Projection for SessionProjection {
    type State = Self;

    fn initial() -> Self { /* ... */ }

    fn apply(state: &mut Self, event: &Event) -> Result<()> {
        match event {
            Event::UserMessageAppended(ev) => state.messages.push(ev.to_message()),
            Event::AssistantMessageCompleted(ev) => { /* ... */ }
            Event::ToolUseRequested(ev) => { /* ... */ }
            // ...
        }
        Ok(())
    }
}
```

### 6.2 `UsageProjection`

聚合 token / cost / tool call 次数，用于 Dashboard。

### 6.3 Replay 确定性约束（`ReplayContext`）

Projection 必须是**纯函数**，但实务中常需要"现在的时间"或随机数（如把过期 token 抹除）。为避免直接依赖系统时钟破坏确定性，`Projection::apply` 在重放路径上接收一个轻量上下文：

```rust
pub trait Projection: Sized + Send + Sync {
    type State;
    fn initial() -> Self::State;
    /// `ctx` 在 live-apply 时由 EventStore 注入真实时钟；
    /// Replay 时由 ReplayEngine 注入由"事件本身派生"的时钟，
    /// 保证 replay(events) ≡ live_apply(events) ↦ identical state.
    fn apply(
        state: &mut Self::State,
        event: &Event,
        ctx: &ReplayContext,
    ) -> Result<(), ProjectionError>;
}

pub struct ReplayContext {
    /// 由 EventEnvelope.at 派生；不可访问真实 wall-clock
    now: DateTime<Utc>,
    /// 由 (session_id, offset) 派生的种子 RNG；不可使用 thread-local rng
    rng: ChaChaRng,
}
```

**约束**：
- 任何 Projection 实现禁止直接调用 `Utc::now()` / `SystemTime::now()` / `rand::thread_rng()`，必须经 `ctx`；
- `live-apply` 时 `ctx.now = event.envelope.at`、`ctx.rng = ChaChaRng::seed_from_u64(hash(session_id, offset))`，与 replay 完全一致；
- 这是 ADR-001 Event Sourcing "纯函数 projection" 的补强；CI 可加 lint 禁止 projection 模块直接 `use std::time::SystemTime`。

### 6.4 远端订阅与回放语义（标记为 ADR-候选）

> **状态**：暂未实施；本节记录已知约束以避免在缺议定的情况下随手实现。

需求来源：远端 UI（Tauri / Web）希望"接入即看到完整历史 + 后续增量"。
设计取舍尚未定型，候选方案：

| 方案 | 描述 | 取舍 |
|---|---|---|
| 保守（当前默认） | 客户端只看实时事件流；断线重连后通过 `since_offset` 拉历史，不保证连续 | 实现成本最低；"无缝接续"需上层补 |
| 温和 | EventStore 暴露 `subscribe(since: Option<JournalOffset>) -> Stream<Event>`，内部用窗口缓冲 | 单进程内 ok；多进程需 IPC 中继 |
| 激进 | Journal 自己做 broker（pub/sub）+ checkpoint 协议 | 复杂度高，违反 KISS |

**当前结论**：`ReplayCursor::Tail { since }` 提供"看最近 N 分钟历史 + 流式到末尾"语义但**非 follow**，由 Engine / Server 层在 Journal 之上自行做 broker（例如 `tokio::sync::broadcast` 转发新 append）。后续如有强需求再升级为独立 ADR。

### 6.5 Session Search 分层

Journal 只提供 **session-level** 检索（FTS over events.body）。**跨 session / 语义检索 / 记忆重排序** 属于 `harness-memory` 与 `harness-tool-search` 责任，不应在 Journal 内实现：

| 检索类型 | 归属 crate | 入口 |
|---|---|---|
| 单 session 内消息 / 工具结果文本 | `harness-journal` | `EventStore::search_messages(session, query)` |
| 跨 session 全局 FTS | `harness-memory` | `MemoryStore::search(MemoryQuery)` 之上的视图 |
| 语义召回（embedding） | `harness-memory` | `MemoryStore` 实现自定义 |
| 工具语义检索（ADR-009） | `harness-tool-search` | 独立 crate |

> **物化视图 `session_index`（B3）**：派生自 SessionProjection 的"每个 session 一行 + 关键字段"视图（如 last_message_excerpt、token_used、参与 tool 列表）属于 `harness-session` 的派生层；本 crate **只提供** `EventStore::list_sessions(filter)` 的最小元数据，避免 Journal 持有派生状态。具体设计在 `harness-session.md` ADR-候选项中规划。

## 7. Compaction 与自动维护

```rust
pub struct PrunePolicy {
    pub older_than: Duration,
    pub keep_snapshots: bool,
    pub keep_latest_n_sessions: Option<u32>,
    pub target_size_bytes: Option<u64>,
}

pub struct PruneReport {
    pub events_removed: u64,
    pub snapshots_removed: u64,
    pub bytes_freed: u64,
}

impl SqliteEventStore {
    pub async fn maybe_auto_prune_and_vacuum(
        &self,
        retention_days: u64,
        min_interval_hours: u64,
        vacuum: bool,
    ) -> Result<Option<PruneReport>>;
}
```

对齐 HER-024。CLI / Server / Cron 启动时 opportunistic 调用。

## 8. Feature Flags

```toml
[features]
default = []
in-memory = []
jsonl = ["dep:tokio", "dep:serde_json"]
sqlite = ["dep:sqlx", "dep:tokio"]
blob-file = ["dep:tokio"]
blob-sqlite = ["sqlite"]
blob-memory = []
```

## 9. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum JournalError {
    #[error("offset out of range: requested={requested}, max={max}")]
    OutOfRange { requested: u64, max: u64 },

    #[error("snapshot invalid: {0}")]
    SnapshotInvalid(String),

    #[error("concurrent write conflict")]
    WriteConflict,

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("sqlite: {0}")]
    Sqlite(#[from] sqlx::Error),

    #[error("serialization: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("query too long: {0} chars (max 256)")]
    QueryTooLong(usize),

    #[error("schema migration path missing: {from} -> {to}")]
    MigrationPathMissing { from: u32, to: u32 },

    #[error("file lock contention on {path}")]
    FileLockBusy { path: PathBuf },
}
```

## 10. 使用示例

```rust
use octopus_harness_journal::jsonl::JsonlEventStore;

let store = JsonlEventStore::open("runtime/events").await?;

// Append
store.append(
    TenantId::SINGLE,
    session_id,
    &[Event::RunStarted(/* ... */)],
).await?;

// Read (replay)
let mut stream = store.read(
    TenantId::SINGLE,
    session_id,
    ReplayCursor::FromStart,
).await?;

let mut proj = SessionProjection::initial();
while let Some(event) = stream.next().await {
    SessionProjection::apply(&mut proj, &event?)?;
}

// Snapshot
store.save_snapshot(
    TenantId::SINGLE,
    SessionSnapshot::from_projection(&proj),
).await?;
```

## 11. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | 每个 Event 类型 apply 正确；Projection idempotent |
| 确定性 | 同一事件流多次 replay 得到完全相同的 SessionProjection（包含 `ctx.now` / `ctx.rng` 注入） |
| 并发 | 1000 并发 append + 10 并发 read；多进程并发 append（SQLite + JSONL flock 各一组） |
| 崩溃 | JSONL：写一半 SIGKILL，读端 `tolerate_partial_tail` 行为正确；SQLite：WAL 未 checkpoint 重启可恢复 |
| FTS Sanitize | 输入 `*foo"OR"bar*` / 256+ 字符 / CJK 关键字 各自路径正确 |
| Schema 迁移 | `MigratorChain::find_path` 对 v1 → v8、含分支版本图、含自环、无连通路径四种场景断言 |
| Blob GC | 内容寻址开 / 关；retention 各组合；GC 与 put 竞争（两阶段 tombstone） |
| 压力 | 100M events、1GB DB 的 replay 性能 |
| `AuditQuery` | `SuppressionOrigin` / `ForwardedDuplex` / `ToolUseLineage` / `HookBlockOrigin` 各自 stitch 正确；跨租户拒绝；`CausationDfs` 顺序与因果树一致 |

## 12. 可观测性

| 指标 | 说明 |
|---|---|
| `journal_append_duration_ms` | 每次 append 耗时 |
| `journal_read_offset_lag` | 读消费者落后头部 offset 数 |
| `journal_wal_checkpoint_duration_ms` | SQLite checkpoint 耗时 |
| `journal_fts_query_duration_ms` | FTS5 查询耗时 |
| `journal_fts_query_rejected_total{reason}` | sanitize 阶段被拒绝的查询数（reason ∈ `too_long` / `cjk_fallback`） |
| `journal_jsonl_partial_tail_total` | JSONL 读到截断尾行的次数（`tolerate_partial_tail`） |
| `journal_jsonl_invalid_line_total` | JSONL 中段坏行数（仅 `tolerate_invalid_lines = true` 时累加，否则直接 fail） |
| `journal_prune_events_removed_total` | 累计清理 |
| `journal_blob_gc_freed_bytes_total` | Blob GC 释放字节数 |
| `journal_blob_gc_scanned_total` | Blob GC 扫描候选数 |
| `journal_blob_dedup_hits_total` | 内容寻址命中复用次数 |
| `journal_replay_nondeterminism_total` | Replay 与 live 状态 diff 计数（应恒为 0；非 0 即 bug） |

## 13. 反模式

- 直接读 SQLite 文件而不走 EventStore trait（绕过 tenant 隔离）
- Projection 里做副作用（修改外部状态）
- 长事务（应该每 append 立即提交）
- 用 `Event` 作为 message queue（应该用专门的 `InterAgentBus`）
- 在 `AuditStore` 里加任何写接口（审计层只读；写需求请走 `EventStore::prune` / `redact_historical`）
- `AuditQuery` 内联 SQL fragment（让客户端直接拼 SQL，泄漏存储后端细节，破坏跨实现可移植）
- 把 `AuditCursor` 当成可解析对象（必须保持不透明 byte string，否则后端无法切换实现）
- `AuditStitch` 链式自展开（`SuppressionOrigin` 拉到的事件再次触发 stitch；最多一跳，避免 N+1 与因果环）

## 14. 相关

- D4 · `event-schema.md`
- ADR-001 Event Sourcing
- `crates/harness-observability.md`（Replay Engine）
- Evidence: HER-020, HER-021, HER-022, HER-023, HER-024
