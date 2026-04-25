# `octopus-harness-sandbox` · L1 原语 · Sandbox Backend SPEC

> 层级：L1 · 状态：Accepted
> 依赖：`harness-contracts`
> 关联：ADR-007（沙箱-权限正交）/ ADR-010（Tool Result Budget）

## 1. 职责

提供**执行环境抽象**，把"在哪执行命令"与"怎么包装命令"统一成 `SandboxBackend` trait。
对齐 HER-011 / CC-18 / OC-23（mode × scope × backend × workspaceAccess 多维旋钮）。

**核心契约**：

- 多 Backend：`Local`（含 OS 级隔离子模式）/ `Docker` / `SSH` / `Noop`(testing)
- 统一 `ExecSpec` + 稳定的 `ExecFingerprint`（被权限决策范围匹配引用）
- `SandboxPolicy`：mode × scope × workspace_access × resource_limits（被 `harness-session` / `harness-subagent` / `harness-team` 通过 `SandboxInheritance` 引用）
- Activity heartbeat（HER-012）、Session snapshot（含分层 `SessionSnapshotKind`）、CWD marker（fd-separated）
- Container lifecycle（CreatePerSession / ReusePooled / BringYourOwn / EphemeralPerExec）
- Output 预算与溢出策略（联动 ADR-010）
- **永不替代审批**（ADR-007；不提供 elevated/bypass_sandbox 后门，反例 OC-24 不采纳）

**非职责**：

- 不实现权限审批（在 `harness-permission`）
- 不实现 OS 级隔离原语本身（由具体 backend 适配 bubblewrap / seatbelt / job-object 等系统能力）
- 不持久化 Sandbox 事件（统一进 `harness-journal`，事件 schema 在 D4 `event-schema.md`）

## 2. 对外 API

### 2.1 核心 Trait

```rust
#[async_trait]
pub trait SandboxBackend: Send + Sync + 'static {
    fn backend_id(&self) -> &str;
    fn capabilities(&self) -> SandboxCapabilities;

    /// 远端 backend 的前置同步（建立连接、warm 容器、上传 workspace 增量等）。
    /// 默认实现为 no-op；同步式 backend 仍可依赖默认实现。
    async fn before_execute(
        &self,
        spec: &ExecSpec,
        ctx: &ExecContext,
    ) -> Result<(), SandboxError> { Ok(()) }

    async fn execute(
        &self,
        spec: ExecSpec,
        ctx: ExecContext,
    ) -> Result<ProcessHandle, SandboxError>;

    /// 远端 backend 在进程结束后做反向同步（拉回 stdout 落盘、回收资源、压平 workspace 改动）。
    /// 默认实现为 no-op。
    async fn after_execute(
        &self,
        outcome: &ExecOutcome,
        ctx: &ExecContext,
    ) -> Result<(), SandboxError> { Ok(()) }

    async fn snapshot_session(
        &self,
        spec: &SnapshotSpec,
    ) -> Result<SessionSnapshotFile, SandboxError>;

    async fn restore_session(
        &self,
        snapshot: &SessionSnapshotFile,
    ) -> Result<(), SandboxError>;

    async fn shutdown(&self) -> Result<(), SandboxError>;
}
```

> `before_execute` / `after_execute` 在 `LocalSandbox` / `NoopSandbox` 走默认 no-op；`DockerSandbox` / `SshSandbox` 用其完成 workspace 增量同步、镜像 warm、连接复用与凭据回收。这避免业务层把"远端预热"逻辑藏到自定义 wrapper 里。

### 2.2 ExecSpec / ExecFingerprint

```rust
pub struct ExecSpec {
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,        // BTreeMap 保证 fingerprint 稳定
    pub cwd: Option<PathBuf>,
    pub stdin: StdioSpec,
    pub stdout: StdioSpec,
    pub stderr: StdioSpec,
    pub timeout: Option<Duration>,
    pub activity_timeout: Option<Duration>,
    pub workspace_access: WorkspaceAccess,
    pub policy: SandboxPolicy,
    pub output_policy: OutputPolicy,
}

pub enum StdioSpec {
    Inherit,
    Piped,
    Null,
    File(PathBuf),
}

/// Sandbox 视角的 workspace 访问面（与 ADR-007 的权限审批正交）。
pub enum WorkspaceAccess {
    None,
    ReadOnly,
    ReadWrite { allowed_writable_subpaths: Vec<PathBuf> },
}
```

`ExecSpec` 必须能产出**稳定指纹**，用于权限决策范围匹配——`Decision::AllowSession` / `AllowPermanent` 命中 `DecisionScope::ExactCommand` 时，存入与比对的就是这个指纹：

```rust
pub struct ExecFingerprint([u8; 32]);

impl ExecSpec {
    /// 生成 canonical fingerprint：不同空白 / 顺序的语义等价命令必须哈希相同。
    /// 算法：
    /// 1. command + args 拼成 argv 列表（不做 shell 解析）
    /// 2. env 取与 `SandboxBaseConfig::passthrough_env_keys` 交集后按 key 排序
    /// 3. cwd canonicalize（仅去 `.` / `..`，不解析 symlink，避免环境差异打散指纹）
    /// 4. workspace_access 用稳定枚举编码
    /// 5. 不纳入：timeout / activity_timeout / output_policy / stdio 端点
    /// 6. BLAKE3(serialize_canonical(...)) -> [u8; 32]
    pub fn canonical_fingerprint(&self, base: &SandboxBaseConfig) -> ExecFingerprint;
}
```

> 指纹算法稳定是 ADR-007 隐含的前置条件，本节是其落点；变更算法属于破坏性升级，需经 ADR 审议。

### 2.3 SandboxPolicy

```rust
/// 跨 crate 共享的沙箱策略（被 `harness-session` / `harness-subagent` / `harness-team`
/// 通过 `SandboxInheritance` 引用）。类型定义见 `harness-contracts.md` §3.4。
pub struct SandboxPolicy {
    pub mode: SandboxMode,
    pub scope: SandboxScope,
    pub network: NetworkAccess,
    pub resource_limits: ResourceLimits,
    pub denied_host_paths: Vec<PathBuf>,
}

/// **mode**：执行隔离强度。
pub enum SandboxMode {
    None,                       // 直接 fork/exec（仅 trusted 业务）
    OsLevel(LocalIsolation),    // 同进程边界 + OS 级 confinement
    Container,                  // Docker / containerd
    Remote,                     // SSH / 远端 RPC
}

/// **scope**：是否允许影响 host 路径以外的文件系统。
pub enum SandboxScope {
    WorkspaceOnly,                  // 仅允许写入 ExecContext.workspace_root 之内
    WorkspacePlus(Vec<PathBuf>),    // 额外白名单
    Unrestricted,                   // 仅 SandboxMode::None 可用
}

pub enum NetworkAccess {
    None,
    LoopbackOnly,
    AllowList(Vec<HostRule>),       // 域名 / CIDR
    Unrestricted,
}

pub struct ResourceLimits {
    pub max_memory_bytes: Option<u64>,
    pub max_cpu_cores: Option<f32>,
    pub max_pids: Option<u32>,
    pub max_wall_clock: Option<Duration>,
    pub max_open_files: Option<u32>,
}
```

> `SandboxScope` 与 `WorkspaceAccess` 的区别：
>
> - `SandboxPolicy.scope`：**策略层**——这次会话/子任务允许访问哪些根。
> - `ExecSpec.workspace_access`：**单次 exec 调用**对 mount 的读/写要求，必须是 `policy.scope` 的子集；越界由 backend 在 `before_execute` 阶段拒绝并返回 `SandboxError::HostPathDenied`。

### 2.4 ProcessHandle

```rust
pub struct ProcessHandle {
    pub pid: Option<u32>,
    pub stdout: Option<BoxStream<Bytes>>,
    pub stderr: Option<BoxStream<Bytes>>,
    pub stdin: Option<BoxStdin>,
    pub cwd_marker: Option<BoxStream<CwdMarkerLine>>,
    pub activity: Arc<dyn ActivityHandle>,
}

#[async_trait]
pub trait ActivityHandle: Send + Sync + 'static {
    async fn wait(&self) -> Result<ExecOutcome, SandboxError>;

    /// 信号送达范围。
    async fn kill(&self, signal: Signal, scope: KillScope) -> Result<(), SandboxError>;

    fn touch(&self);
    fn last_activity(&self) -> Instant;
}

pub enum KillScope {
    Process,            // 仅前台进程
    ProcessGroup,       // 进程组（避开 fork-bomb 残留）
    SessionLeader,      // session leader（远端 backend 通常用 `kill -- -PGID`）
}

pub struct ExecOutcome {
    pub exit_status: ExitStatus,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub stdout_bytes_observed: u64,
    pub stderr_bytes_observed: u64,
    pub overflow: Option<OutputOverflow>,
}
```

> `kill` 的默认推荐是 `KillScope::ProcessGroup`：spawn-per-call 模式下子进程通过 shell wrapper 启动，残留 grandchild 必须由进程组兜底。`Process` 仅在调用方明确只想停止前台进程时使用。

### 2.5 SandboxCapabilities

```rust
pub struct SandboxCapabilities {
    pub supports_interactive_shell: bool,
    pub supports_network: bool,
    pub supports_filesystem_write: bool,
    pub supports_gpu: bool,
    pub supports_pty: bool,
    pub supports_detach: bool,                 // 长任务可解绑前台
    pub supports_workspace_sync: bool,         // before/after_execute 是否做实质同步
    pub supports_resource_limits: ResourceLimitSupport,
    pub supports_session_snapshot: bool,
    pub snapshot_kinds: BTreeSet<SessionSnapshotKind>,
    pub max_concurrent_execs: u32,
    pub default_timeout: Duration,
}

pub struct ResourceLimitSupport {
    pub memory: bool,
    pub cpu: bool,
    pub pids: bool,
    pub open_files: bool,
}
```

> Capabilities 是 SDK 与业务层之间的**协商契约**：当 `SubagentSpec.sandbox_policy = SandboxInheritance::Require(...)` 时，runner 在选择 backend 前调用 `capabilities()` 检查。不匹配时返回 `SandboxError::CapabilityMismatch { missing }`，**绝不**静默降级。

### 2.6 ExecContext

```rust
pub struct ExecContext {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub tool_use_id: Option<ToolUseId>,
    pub tenant_id: TenantId,
    pub workspace_root: PathBuf,
    pub correlation_id: CorrelationId,
    pub event_sink: EventSink,                 // 发出 SandboxExecution* 事件
    pub redactor: Arc<dyn Redactor>,
}
```

`event_sink` 抽象出"把事件投到 Journal"的能力，避免 `harness-sandbox` 反向依赖 `harness-journal`。

## 3. 内置实现

### 3.0 SandboxBaseConfig（共享配置）

```rust
pub struct SandboxBaseConfig {
    pub passthrough_env_keys: BTreeSet<String>,    // 默认仅传 PATH / LANG / LC_ALL / TERM
    pub denied_host_paths: Vec<PathBuf>,           // 黑名单（如 /etc / ~/.ssh）
    pub default_resource_limits: ResourceLimits,
    pub default_output_policy: OutputPolicy,
}
```

三个真实 backend（`Local` / `Docker` / `SSH`）通过 `SandboxBaseConfig` 注入默认行为；`Noop` 仅在测试时使用。`passthrough_env_keys` 同时是 `ExecFingerprint` 的输入，业务层在配置时即承诺了"哪些 env 参与决策匹配"。

### 3.1 `LocalSandbox`

```rust
pub struct LocalSandbox {
    base: SandboxBaseConfig,
    root: PathBuf,
    shell: ShellKind,
    isolation: LocalIsolation,
}

/// `ShellKind` 已下沉到契约层（`harness-contracts §3.4`），
/// 供 `harness-permission::DangerousPatternLibrary` 等下游免反向依赖共享。
/// 本 crate 仅 re-export，并由具体 backend 负责 shell 路径解析与 PowerShell 版本探测。
pub use harness_contracts::ShellKind;

pub enum LocalIsolation {
    /// 直接 fork/exec，无额外限制（仅 trusted SDK 自身使用）
    None,
    /// Linux：bubblewrap namespace + seccomp
    Bubblewrap(BwrapConfig),
    /// macOS：sandbox-exec / seatbelt 配置文件
    Seatbelt(SeatbeltProfile),
    /// Windows：Job Object + restricted token
    JobObject(JobObjectConfig),
}
```

`LocalIsolation` 把"OS 级 confinement"暴露成枚举，便于 `SandboxCapabilities` 表达 / 上游策略选择，但**不**在本 crate 内实现具体 confinement 系统调用——具体系统适配由独立 crate 提供并通过 cargo feature 编织进 `LocalSandbox::new_with_isolation(...)`。

**实现指引（grandchild-pipe EOF 陷阱）**：
当用 shell wrapper 包裹用户命令时，shell 自身会持有 stdout/stderr 的写端。
若 SDK 仅 `wait()` 阻塞读取，孙进程 fork detach 后会让 pipe 永不 EOF，导致读取永远阻塞。
正确做法：

- 用 `tokio::select!` / `epoll` 同时监听 child status + pipe；
- 子进程退出后给 pipe 设 deadline（如 200ms grace），到期后强制 close；
- 信号传播必须用 `KillScope::ProcessGroup` 配合 `setsid` / `setpgid`。

### 3.2 `DockerSandbox`

```rust
pub struct DockerSandbox {
    base: SandboxBaseConfig,
    image: String,
    volumes: Vec<VolumeMount>,
    network: NetworkMode,
    user: Option<String>,
    docker_socket: PathBuf,
    lifecycle: ContainerLifecycle,
}

pub enum NetworkMode {
    Host,
    Bridge,
    None,
    Custom(String),
}

pub struct VolumeMount {
    pub host_path: PathBuf,
    pub container_path: PathBuf,
    pub read_only: bool,
    pub propagation: MountPropagation,
}

pub enum MountPropagation {
    Private,
    RShared,                // 配合 sandbox 内嵌 fs 操作
}

/// 容器生命周期归属。
pub enum ContainerLifecycle {
    /// 每个 Session 创建一个长寿命容器；exec 走 `docker exec`。对齐 HER-011 spawn-per-call。
    CreatePerSession {
        keep_alive_after_exit: Duration,
    },
    /// 维护一个池，多 Session 复用容器（按 tenant 分组隔离）。
    ReusePooled {
        pool_size: u32,
        idle_timeout: Duration,
    },
    /// 容器由外部编排（k8s / nomad）创建，SDK 仅 attach。
    BringYourOwn {
        container_id: String,
    },
    /// 每次 exec 创建并销毁容器（沉重，仅 CI 一次性任务用）。
    EphemeralPerExec,
}
```

容器生命周期是公开契约：`Capabilities.snapshot_kinds` 与 `lifecycle` 联动决定 snapshot 是否可用（`EphemeralPerExec` 不支持 `SessionSnapshotKind::ShellState`）。

`snapshot_session` 在 `CreatePerSession` / `ReusePooled` 下用 `docker commit`；`BringYourOwn` 委托给容器编排器；`EphemeralPerExec` 直接拒绝（返回 `SandboxError::SnapshotUnsupported`）。

### 3.3 `SshSandbox`

```rust
pub struct SshSandbox {
    base: SandboxBaseConfig,
    host: String,
    port: u16,
    user: String,
    auth: SshAuth,
    keepalive: Duration,
    multiplex: bool,
    workspace_sync: WorkspaceSyncStrategy,
}

pub enum SshAuth {
    KeyFile(PathBuf),
    KeyInline(SecretString),
    Agent,
    Password(SecretString),
}

pub enum WorkspaceSyncStrategy {
    /// 不同步，远端必须自备工作区
    None,
    /// before_execute 用 rsync 增量上传
    RsyncPush,
    /// 双向同步（after_execute 拉回）
    RsyncBidi,
}
```

SSH 通常没有"side FD"通道，因此 `cwd_marker` 默认 `CwdChannel::Disabled`；如远端 shell 支持自定义 fd 转发可改用 `NamedPipe`。

### 3.4 `NoopSandbox`（testing）

```rust
pub struct NoopSandbox {
    recorded_execs: Arc<Mutex<Vec<ExecSpec>>>,
    response_map: HashMap<String, MockExitStatus>,
    delay: Duration,
}
```

仅在 testing 上下文使用：不真实 fork/exec、不打开 pipe、心跳走 mock 时钟。详细使用约束见 `extensibility.md §10/11`。

### 3.5 `CodeSandbox`（M1，feature `programmatic_tool_calling`）

> 代码运行时沙箱：服务于 ADR-0016 `execute_code` 元工具。
> **职责面**与 §3.1~§3.4 的进程沙箱**严格分离**——`CodeSandbox` 只负责执行
> 一段受限脚本（mini-lua）并提供"嵌入式工具调用"宿主回调；它**不**发起任何 OS
> 级 syscall、**不**触达 fs / net / process。脚本内嵌的工具调用走的是主 Engine
> 已存在的工具流水线（必要时再触达 `LocalSandbox` / `DockerSandbox`），由
> `harness-tool::ToolOrchestrator` 复用统一的 5 介入点。

#### 3.5.1 Trait

```rust
pub trait CodeSandbox: Send + Sync + 'static {
    fn capabilities(&self) -> CodeSandboxCapabilities;

    async fn run(
        &self,
        script: &CompiledScript,             // 由 harness-tool::ScriptCompiler 产出
        ctx: CodeSandboxRunContext,
    ) -> Result<CodeSandboxResult, SandboxError>;
}

pub struct CodeSandboxCapabilities {
    pub language: ScriptLanguage,            // M0 = MiniLua
    pub max_instructions: u64,                // 默认 1_000_000（cooperative tick 计数）
    pub max_call_depth: u32,                  // 默认 32
    pub max_string_bytes: u64,                // 默认 4 MiB
    pub max_table_entries: u64,               // 默认 65_536
    pub wall_clock_budget: Duration,          // 默认 30s
    pub deterministic: bool,                  // 默认 true（无随机 / 时钟来源）
}

pub struct CodeSandboxRunContext {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub parent_tool_use_id: ToolUseId,
    pub embedded_dispatcher: Arc<dyn EmbeddedToolDispatcherCap>,
    pub usage_meter: Arc<dyn UsageMeter>,
    pub event_sink: Arc<dyn EventSink>,
}

pub struct CodeSandboxResult {
    pub stdout: String,                       // print(...) 输出（按 ResultBudget 计费）
    pub return_value: Option<LuaValue>,        // 脚本 `return` 语义结果
    pub steps_summary: Vec<EmbeddedStepSummary>,
    pub stats: SandboxRunStats,                // instructions / wall_clock / peak_memory
}
```

`EmbeddedToolDispatcherCap` 在 `harness-contracts::capability` 模块定义；脚本通过
`emb.tool(name, args)` 触发的每次调用均经此接口转译为合法 `ToolUse`，最终走
`ToolOrchestrator::single_use_pipeline`。

#### 3.5.2 默认实现 `MiniLuaCodeSandbox`

| 维度 | 取值 |
|---|---|
| 语言 | mini-lua（编译期嵌入；无 `dlopen` / 无外部 ABI） |
| 进程模型 | 同步执行；不 fork 子进程；CPU/instr 计数 + memory 上限走 interpreter cooperative tick |
| OS 暴露面 | 无（不暴露 fs / net / process / clock） |
| 随机性 | 默认 `deterministic = true`（无 `math.random` / 无 `os.time`） |
| `EmbeddedToolDispatcher` | 以宿主侧 callback 注入；脚本只能通过 `emb.tool("...")` 触达 |
| 失败模式 | `SandboxError::CodeRuntime(reason)`，`reason ∈ {InstructionLimit, CallDepth, WallClock, ScriptError, MemoryLimit, EmbeddedDenied, SelfReentrant}` |

> **与进程沙箱的边界**：`CodeSandbox` 不直接调用 `LocalSandbox::execute(...)`。
> 当脚本调用 `emb.tool("Grep", ...)` 时，路径是 `MiniLuaCodeSandbox →
> EmbeddedToolDispatcherCap → ToolOrchestrator::single_use_pipeline →
> GrepTool::execute`，`GrepTool` 自身按需触达 `LocalSandbox`（与不走 PTC 时一致）。

#### 3.5.3 与 `SandboxBaseConfig` / `LocalSandbox` 的关系

`CodeSandbox` **不**继承 `SandboxBaseConfig`：本 trait 没有 `cwd` / `env` /
`network` 等概念。装配期 `EngineBuilder` 将 `CodeSandbox` 单独注入：

```rust
impl EngineBuilder {
    pub fn with_code_sandbox(self, sandbox: Arc<dyn CodeSandbox>) -> Self;
}
```

未注入 `CodeSandbox` 但 `feature_flags.programmatic_tool_calling = on` 时，
`ToolRegistry::register(ExecuteCodeTool)` 会 fail-closed
`RegistrationError::CapabilityProviderMissing { cap: "CodeRuntime" }`。

#### 3.5.4 失败语义与事件

- `instructions / wall_clock / call_depth` 任一超限：`SandboxError::CodeRuntime(...)`，
  脚本 stdout 截断到当前位置随结果回灌主 Agent；`Event::ToolUseFailed` 写入 reason
- 嵌入调用被白名单拒绝：dispatcher 写出 `Event::ExecuteCodeStepInvoked { refused_reason }`，
  脚本侧抛出 `lua_error("embedded_denied")`，业务脚本可 `pcall` 兜底
- `EmbeddedToolDispatcher` 的拒绝**不**计入 `instructions` 配额，避免
  "denied 风暴" 反向耗尽脚本预算

## 4. Command Wrapping（对齐 HER-011）

所有 backend 共享 `_wrap_command` 逻辑，且**严禁**业务层手动拼 shell 字符串。

```rust
pub struct WrappedCommand {
    pub original: ExecSpec,
    pub preamble: Vec<PreambleStep>,
    pub body: ArgVec,
    pub epilogue: Vec<EpilogueStep>,
}

pub enum PreambleStep {
    SetEnv { key: String, value: SecretString },
    UnsetEnv { key: String },
    EnableCwdMarker { channel: CwdChannel },
    EnableActivityHeartbeat { interval: Duration },
    SourceProfile { path: PathBuf },
}

pub enum EpilogueStep {
    EmitCwdMarker,
    NormalizeExitCode,
    FlushHeartbeat,
}

/// 以 argv 形态传递，避免 shell 注入。
pub struct ArgVec(pub Vec<String>);
```

- `WrappedCommand` 由 backend 内部构造；上层只填 `ExecSpec`。
- `body` 始终是 `ArgVec`，不允许字符串拼接。`shell -c "..."` 形态由 `ShellKind` 决定，`WrappedCommand::for_shell(...)` 单点封装。
- `inject_cwd_marker(...)` / `inject_activity_heartbeat(...)` 通过追加 `PreambleStep` 完成，不直接修改 `body`。

### 4.1 CWD 上报通道（Fd-Separated Protocol）

```rust
pub enum CwdChannel {
    SideFd { fd: RawFd },
    NamedPipe { name: String },
    Disabled,
}

pub struct CwdMarkerLine {
    pub sequence: u64,
    pub cwd: PathBuf,
    pub at: DateTime<Utc>,
}
```

**设计约束**：

- **禁止**把 marker 混进 `stdout` / `stderr`（反例：以前的 `\x00CWD:...\x00` 会被 `find -print0` / 二进制流污染）。
- Shell wrapper 在每条命令后 `printf '{...}' >&3`，SDK 在启动时给 backend 传 `extra_fds: [fd_3_reader]`。
- 无 side FD 的后端（如 `ssh host cmd`，无第三通道）使用 `CwdChannel::Disabled`，cwd 由业务侧显式 `pwd` 命令记账。
- **退化语义**：当 `ExecSpec.command` 直接是 binary（不经过 shell wrapper），CWD marker 自动 `Disabled`，业务层不应假设能拿到 `cwd_marker` 流。

## 5. Activity Heartbeat（对齐 HER-012）

```rust
impl LocalProcessHandle {
    async fn heartbeat_loop(activity: Arc<dyn ActivityHandle>) {
        let interval = Duration::from_secs(10);
        loop {
            tokio::time::sleep(interval).await;
            activity.touch();
        }
    }
}
```

- `activity_timeout` 与 `timeout` 正交：前者是"无 stdout/stderr 输出 + 无 touch 持续 N 秒"，后者是 wall-clock。
- 心跳必须发出 `Event::SandboxActivityHeartbeat`；连续 N 次未收到 child 输出且未触达 wall-clock 时发出 `Event::SandboxActivityTimeoutFired`，由 backend 决定 `kill(KillScope::ProcessGroup)`。

## 6. Session Snapshot

```rust
pub struct SnapshotSpec {
    pub session_id: SessionId,
    pub target_path: PathBuf,
    pub kinds: BTreeSet<SessionSnapshotKind>,
}

pub enum SessionSnapshotKind {
    /// workspace 文件系统的 tarball（适用所有 backend）
    FilesystemImage,
    /// shell 状态：环境变量 / cwd / 别名 / 函数表
    ShellState,
    /// 容器/VM 完整 image（仅 Docker 等支持）
    ContainerImage,
}

pub struct SessionSnapshotFile {
    pub path: PathBuf,
    pub size: u64,
    pub content_hash: [u8; 32],
    pub kind: SessionSnapshotKind,
    pub metadata: SnapshotMetadata,
}
```

> 不区分 Kind 时，"snapshot" 在 SSH backend 上语义模糊（远端没有 image 概念）。`Capabilities.snapshot_kinds` 显式声明每个 backend 支持的种类，`snapshot_session` 在请求未支持的 kind 时返回 `SandboxError::SnapshotUnsupported { kind }`。

## 7. Output 处理

```rust
pub struct OutputPolicy {
    pub max_inline_bytes: u64,                 // 直接保留在 ToolResult 中
    pub overflow: OutputOverflowPolicy,
    pub redact_secrets: bool,
}

pub enum OutputOverflowPolicy {
    /// 截断后保留 head/tail 预览，剩余落 BlobStore，对齐 ADR-010
    SpillToBlob {
        head_bytes: u32,
        tail_bytes: u32,
    },
    /// 直接截断，丢弃多出部分
    Truncate,
    /// 立刻 kill 子进程并返回 `SandboxError::OutputBudgetExceeded`
    AbortExec,
}

pub struct OutputOverflow {
    pub policy: OutputOverflowPolicy,
    pub blob_ref: Option<BlobRef>,
    pub original_bytes: u64,
}
```

- `OutputPolicy.max_inline_bytes` 是 sandbox 层的硬上限，独立于 `ResultBudget`（ADR-010）。`ResultBudget` 在 Tool 层做"语义友好截断"，而 `OutputPolicy` 防止单次 exec 撑爆内存。
- 触发 `SpillToBlob` 时发出 `Event::SandboxOutputSpilled { blob_ref, original_bytes }`，由 Tool 层把 `blob_ref` 拼进 `ToolResult`。
- 慢消费者（业务方读 `ProcessHandle.stdout` 太慢）通过 bounded channel 做背压：channel 满后 backend 暂停读 child stdout 并 `Event::SandboxBackpressureApplied`，触发 backpressure 而不是丢字节。

## 8. 与权限的关系（强调）

```rust
// Sandbox 层禁止 bypass 审批
impl LocalSandbox {
    pub async fn execute(&self, spec: ExecSpec, ctx: ExecContext) -> Result<ProcessHandle, SandboxError> {
        // 不调用 check_permission；权限由 Tool::check_permission 负责
        // 即使在 Docker 里，破坏性命令仍需审批（ADR-007）
    }
}
```

- 对齐 ADR-007 / CC-18 / OC-23。
- **不采纳**：HER-041（容器型 env 早退跳审批）、OC-24（`tools.elevated` 后门）。
- `SandboxBackend` trait 不暴露 `bypass_permission` / `elevated` 等任何旁路 API；任何尝试新增该路径的变更必须指向反模式条目并被拒绝。

## 9. 事件

Sandbox 仅产生**与权限/Tool 不可替代**的事件，其余统一委托给 Tool 层：

| Event | 触发时机 | 必记 |
|---|---|---|
| `SandboxExecutionStarted` | `execute()` 即将 fork/exec | 是 |
| `SandboxExecutionCompleted` | child wait 返回 | 是 |
| `SandboxActivityHeartbeat` | 每次 `touch()` 触发 | 否（采样） |
| `SandboxActivityTimeoutFired` | 触发 inactivity kill | 是 |
| `SandboxOutputSpilled` | 输出落 BlobStore | 是 |
| `SandboxBackpressureApplied` | 慢消费者背压触发 | 否 |
| `SandboxSnapshotCreated` | snapshot 产物落地 | 是 |
| `SandboxContainerLifecycleTransition` | 容器创建/复用/销毁 | 是 |

具体字段定义见 D4 `event-schema.md` §3.x。"必记"表示 `SessionEventSinkPolicy::DEFAULT_NEVER_DROP_KINDS` 默认包含。

## 10. Feature Flags

```toml
[features]
default = []
local = ["dep:tokio"]
local-bubblewrap = ["local"]      # Linux OS-level isolation
local-seatbelt = ["local"]        # macOS OS-level isolation
local-job-object = ["local"]      # Windows OS-level isolation
docker = ["dep:bollard"]
ssh = ["dep:russh"]
noop = []
```

## 11. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("backend unavailable: {0}")]
    Unavailable(String),

    #[error("capability mismatch: missing {missing:?}")]
    CapabilityMismatch { missing: Vec<String> },

    #[error("exec timeout after {0:?}")]
    Timeout(Duration),

    #[error("inactivity timeout")]
    InactivityTimeout,

    #[error("output budget exceeded: {bytes} bytes")]
    OutputBudgetExceeded { bytes: u64 },

    #[error("non-zero exit: {0}")]
    NonZeroExit(i32),

    #[error("workspace path denied: {0}")]
    HostPathDenied(PathBuf),

    #[error("resource limit exceeded: {limit}")]
    ResourceLimitExceeded { limit: String, current: u64 },

    #[error("snapshot unsupported by backend: {kind:?}")]
    SnapshotUnsupported { kind: SessionSnapshotKind },

    #[error("snapshot failed: {0}")]
    SnapshotFailed(String),

    #[error("container lifecycle error: {0}")]
    ContainerLifecycleError(String),

    #[error("cancelled")]
    Cancelled,

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}
```

## 12. 使用示例

```rust
use octopus_harness_sandbox::docker::{DockerSandbox, ContainerLifecycle, MountPropagation};
use octopus_harness_sandbox::{
    ExecSpec, OutputPolicy, OutputOverflowPolicy,
    SandboxPolicy, SandboxMode, SandboxScope, NetworkAccess, ResourceLimits,
    WorkspaceAccess, VolumeMount, NetworkMode,
};

let sandbox = DockerSandbox::builder()
    .image("octopus-workspace:latest")
    .mount(VolumeMount {
        host_path: "/home/user/workspace".into(),
        container_path: "/workspace".into(),
        read_only: false,
        propagation: MountPropagation::Private,
    })
    .network(NetworkMode::Bridge)
    .lifecycle(ContainerLifecycle::CreatePerSession {
        keep_alive_after_exit: Duration::from_secs(300),
    })
    .build()?;

let spec = ExecSpec {
    command: "cargo".into(),
    args: vec!["test".into()],
    cwd: Some("/workspace".into()),
    timeout: Some(Duration::from_secs(300)),
    activity_timeout: Some(Duration::from_secs(60)),
    workspace_access: WorkspaceAccess::ReadWrite { allowed_writable_subpaths: vec![] },
    policy: SandboxPolicy {
        mode: SandboxMode::Container,
        scope: SandboxScope::WorkspaceOnly,
        network: NetworkAccess::AllowList(vec!["crates.io".into()]),
        resource_limits: ResourceLimits::default(),
        denied_host_paths: vec!["/etc".into()],
    },
    output_policy: OutputPolicy {
        max_inline_bytes: 1 << 20,
        overflow: OutputOverflowPolicy::SpillToBlob { head_bytes: 4096, tail_bytes: 4096 },
        redact_secrets: true,
    },
    ..Default::default()
};

let mut handle = sandbox.execute(spec, ctx).await?;
let mut stdout = handle.stdout.take().unwrap();
while let Some(chunk) = stdout.next().await {
    print!("{}", String::from_utf8_lossy(&chunk?));
}
let outcome = handle.activity.wait().await?;
```

## 13. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | `WrappedCommand` 各 Step 顺序、`ExecFingerprint` 的 canonical 不变性 |
| 单元 | `SandboxPolicy` ↔ `SandboxCapabilities` 的协商失败路径 |
| 集成 | 每个 backend 的 simple echo / cwd marker 流 |
| Timeout | `timeout` / `activity_timeout` / grandchild EOF 各自触发 |
| Snapshot | `FilesystemImage` / `ContainerImage` / `ShellState` 往返 |
| Output | `SpillToBlob` / `Truncate` / `AbortExec` 三策略命中 |
| Noop | 业务层 Tool 调用路径覆盖 |

## 14. 可观测性

| 指标 | 说明 |
|---|---|
| `sandbox_exec_duration_ms` | 每次执行耗时 |
| `sandbox_exec_exit_codes` | 按 backend × exit code 分桶 |
| `sandbox_activity_timeouts_total` | 活动超时次数 |
| `sandbox_output_spilled_bytes_total` | 输出溢出落 Blob 总量 |
| `sandbox_concurrent_execs` | 当前并发执行数 |
| `sandbox_container_lifecycle_transitions_total` | 容器生命周期转换次数（按 lifecycle × kind 分桶） |

## 15. 反模式

- Sandbox backend 里做 `check_permission`（职责越界）。
- 在 Sandbox 层拼接 shell 命令字符串而非 `ArgVec`（防注入）。
- 引入任何 `bypass_sandbox` / `elevated` / `skip_permission` API（参见 §8 与 ADR-007；OC-24 路径不采纳）。
- 用 `stdout` / `stderr` 回传 CWD marker（参见 §4.1；HER 早期方案被业务字节污染）。
- 仅 `wait()` 阻塞读 child 而不监听 grandchild pipe EOF（参见 §3.1，会永挂）。
- 静默忽略 `Capabilities` 不匹配并降级到弱 backend（必须返回 `CapabilityMismatch` 让上层选择）。
- 把 `activity_timeout` 当 wall-clock timeout 使用（两者正交，参见 §5）。

## 16. 相关

- D7 · `extensibility.md` §7 Sandbox 扩展
- D9 · `security-trust.md` §3 三维正交
- D4 · `event-schema.md` §SandboxExecution* / §SandboxOutputSpilled / §SandboxContainerLifecycleTransition
- ADR-007 权限决策事件化 / 沙箱-权限正交（§6.1 引用本节 §2.2 的 `ExecFingerprint`）
- ADR-010 Tool Result Budget（与 §7 OutputPolicy 联动）
- `crates/harness-contracts.md` §3.4（`SandboxPolicy` / `SandboxMode` / `SandboxScope` / `WorkspaceAccess` / `ExecFingerprint` / `KillScope` / `SessionSnapshotKind` 等跨 crate 共享类型）
- `crates/harness-subagent.md` §2.2（`SandboxInheritance` 引用 `SandboxPolicy`）
- Evidence: HER-011, HER-012, HER-041, OC-23, OC-24, CC-18
