# `octopus-harness-plugin` · L3 · Plugin Host SPEC

> 层级：L3 · 状态：Accepted
> 依赖：`harness-contracts` + `harness-tool`（trait） + `harness-hook`（trait） + `harness-mcp`（trait） + `harness-skill`（trait）

## 1. 职责

实现 **插件宿主**：四源发现、Manifest 校验、信任域二分、Capability Slot 管理、生命周期状态机。对齐 HER-033 / HER-035 / OC-16 / OC-17 / OC-18 / CC-27 / CC-38 / ADR-006。

**核心能力**：

- 四源发现：Workspace / User / Project / Cargo extension
- Manifest 校验（JSON schema + 签名 + 命名空间）
- `TrustLevel::AdminTrusted` / `UserControlled` 能力矩阵
- Lazy runtime 加载（manifest-first）；`PluginManifestLoader` / `PluginRuntimeLoader` 二分（详见 §3.2 与 ADR-0015）
- Capability Slot：独占/可叠加（Memory slot 独占、Tool 可叠加）
- Plugin 统一注册到对应 Registry，注入"capability-scoped handle"（详见 §2.4 与 ADR-0015）
- Manifest signer 治理：`TrustedSignerStore` 启用窗口 + 撤销列表（详见 §4.1 与 ADR-0014）

**核心原则**：

1. **Manifest-first**（HER-033 / OC-17）：发现期只解析 manifest，**不得**实例化或调用任何插件代码；运行期通过 `activate` 显式触发懒加载。本约束在 §3.2 由 `PluginManifestLoader` / `PluginRuntimeLoader` 类型层面保证（ADR-0015）
2. **Capability Ownership**：每个 Capability Slot 由其上游 Registry 拥有；插件不能旁路 Registry 直接持有共享状态（HER-035）。`PluginActivationContext` 仅注入 manifest 已声明范围内的窄接口 handle（§2.4）
3. **Trust 不可降级**（OC-18 反例）：来源决定 `TrustLevel`，签名只能"印证"声明的 Trust，不能从 `UserControlled` 自动跃迁到 `AdminTrusted`
4. **Fail-Closed**：未知字段、签名异常、Trust 不匹配、Slot 冲突、signer 被撤销 / 不在启用窗口一律拒绝；不存在"宽容模式"
5. **Manifest validation ≠ PluginRejected**：解析失败（YAML / schema / `manifest_schema_version` 不支持等）时尚无可信 `PluginId`，落 `Event::ManifestValidationFailed`；解析通过后被业务规则拒绝才落 `Event::PluginRejected`（详见 §16.1）

## 2. 对外 API

### 2.1 Plugin Trait

```rust
#[async_trait]
pub trait Plugin: Send + Sync + 'static {
    fn manifest(&self) -> &PluginManifest;

    async fn activate(
        &self,
        ctx: PluginActivationContext,
    ) -> Result<PluginActivationResult, PluginError>;

    async fn deactivate(&self) -> Result<(), PluginError>;
}
```

> `activate` 是允许执行 IO / 注册的唯一入口；`deactivate` **必须**幂等且不抛错（错误会被记 `PluginError::DeactivateFailed` 但不阻塞 Registry 关停流程）。

### 2.2 Manifest

```rust
pub struct PluginManifest {
    /// Manifest schema 版本（首版固定为 1）；用于跨大版本演化时的兼容判定。
    /// 缺省解释为 1；高于 SDK 已知版本时拒绝加载（fail-closed）。
    pub manifest_schema_version: u32,

    /// 插件命名空间内的逻辑名（详见 §5 命名空间治理）。
    pub name: String,

    pub version: semver::Version,
    pub trust_level: TrustLevel,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub repository: Option<Url>,
    pub signature: Option<ManifestSignature>,
    pub capabilities: PluginCapabilities,
    pub dependencies: Vec<PluginDependency>,
    pub min_harness_version: semver::VersionReq,
}

pub struct PluginCapabilities {
    pub tools: Vec<ToolManifestEntry>,
    pub skills: Vec<SkillManifestEntry>,
    pub hooks: Vec<HookManifestEntry>,
    pub mcp_servers: Vec<McpManifestEntry>,
    pub memory_provider: Option<MemoryProviderManifestEntry>,
    /// M5 只声明并占用 plugin 本地 coordinator strategy slot。
    /// 真实 team / subagent bridge 归 M6。
    pub coordinator_strategy: Option<CoordinatorStrategyManifestEntry>,
    pub configuration_schema: Option<Value>,
}

pub struct PluginDependency {
    /// 被依赖的 plugin 名（同空间，不跨命名空间）。
    pub name: String,
    /// SemVer 范围（缺省 `>=0.0.0` 视为"任意版本"，但仍参与命名空间唯一性约束）。
    pub version_req: semver::VersionReq,
    /// `Required` 时缺失即拒绝激活；`Optional` 时降级运行并发 Warning。
    pub kind: PluginDependencyKind,
}

pub enum PluginDependencyKind {
    Required,
    Optional,
}

pub struct ManifestSignature {
    pub algorithm: SignatureAlgorithm,
    pub signer: String,
    pub signature: Bytes,
    pub timestamp: DateTime<Utc>,
}

pub enum SignatureAlgorithm {
    Ed25519,
    RsaPkcs1Sha256,
}
```

> `manifest_schema_version` 与 `dependencies.kind` 是为长期演化预留的语义字段；当前 SDK 默认值见 §11；增加新字段时遵循 `#[non_exhaustive]` 与 §12 兼容策略。

### 2.3 Registry

```rust
pub struct PluginRegistry {
    inner: Arc<RwLock<PluginRegistryInner>>,
    /// Manifest 签名验证根；详见 §4.1 与 ADR-0014。
    /// 默认 `StaticTrustedSignerStore`，企业可注入自定义 store
    signer_store: Arc<dyn TrustedSignerStore>,
    /// Manifest 阶段加载器（§3.2 / ADR-0015）；按声明顺序对每个 source 询问
    manifest_loaders: Vec<Arc<dyn PluginManifestLoader>>,
    /// Runtime 阶段加载器（§3.2 / ADR-0015）；activate 时按声明顺序询问 can_load
    runtime_loaders: Vec<Arc<dyn PluginRuntimeLoader>>,
    discovery_sources: Vec<DiscoverySource>,
    naming_policy: PluginNamingPolicy,
    config: PluginConfig,
}

struct PluginRegistryInner {
    discovered: HashMap<PluginId, DiscoveredPlugin>,
    activated: HashMap<PluginId, Arc<dyn Plugin>>,
    state: HashMap<PluginId, PluginLifecycleState>,
}

pub struct DiscoveredPlugin {
    /// 已通过 §4 全部 Validation 的 ManifestRecord
    pub record: ManifestRecord,
    pub source: DiscoverySource,
    pub validation_result: ValidationResult,
}

pub enum DiscoverySource {
    Workspace(PathBuf),
    User(PathBuf),
    Project(PathBuf),
    CargoExtension,
}

impl PluginRegistry {
    pub fn builder() -> PluginRegistryBuilder;

    pub async fn discover(&self) -> Result<Vec<DiscoveredPlugin>, PluginError>;
    pub async fn activate(&self, id: &PluginId) -> Result<(), PluginError>;
    pub async fn deactivate(&self, id: &PluginId) -> Result<(), PluginError>;
    pub fn list_activated(&self) -> Vec<PluginManifest>;
    pub fn snapshot(&self) -> PluginRegistrySnapshot;
    pub fn state(&self, id: &PluginId) -> PluginLifecycleState;
}
```

`PluginId` 是 `name@version` 形态的稳定标识（见 §5）；`PluginLifecycleState` 见 §7；`ManifestRecord` 见 §3.2。

### 2.4 ActivationContext（Capability-Scoped）

> ADR-0015 把 ActivationContext 由"Registry 句柄包"重构为"按 manifest 声明范围注入的 capability handle 集合"。`Plugin::activate` 仍是单一入口，但能注册哪些能力由类型系统直接限制。

```rust
pub struct PluginActivationContext {
    pub trust_level: TrustLevel,
    pub plugin_id: PluginId,
    pub config: Value,
    pub workspace_root: Option<PathBuf>,

    /// 仅当 `manifest.capabilities.tools` 非空时为 `Some`；其他 handle 同理
    pub tools: Option<Arc<dyn ToolRegistration>>,
    pub hooks: Option<Arc<dyn HookRegistration>>,
    pub mcp: Option<Arc<dyn McpRegistration>>,
    pub skills: Option<Arc<dyn SkillRegistration>>,
    pub memory: Option<Arc<dyn MemoryProviderRegistration>>,
    pub coordinator: Option<Arc<dyn CoordinatorStrategyRegistration>>,
}

#[async_trait]
pub trait ToolRegistration: Send + Sync {
    /// 注册一个 Tool；要求 `tool.descriptor().name` 必须出现在 manifest 声明的工具集中
    async fn register(&self, tool: Arc<dyn Tool>) -> Result<(), RegistrationError>;
    /// 已声明但当前插件还没注册的工具列表（用于 activate 期"声明而未实现"检查）
    fn pending_declared(&self) -> Vec<&str>;
}

#[async_trait]
pub trait HookRegistration: Send + Sync {
    async fn register(&self, handler: Arc<dyn HookHandler>) -> Result<(), RegistrationError>;
    fn pending_declared(&self) -> Vec<&str>;
}

#[async_trait]
pub trait McpRegistration: Send + Sync {
    async fn register(&self, server: McpServerSpec) -> Result<McpServerId, RegistrationError>;
    fn pending_declared(&self) -> Vec<&str>;
}

#[async_trait]
pub trait SkillRegistration: Send + Sync {
    async fn register(&self, skill: Arc<dyn Skill>) -> Result<(), RegistrationError>;
    fn pending_declared(&self) -> Vec<&str>;
}

#[async_trait]
pub trait MemoryProviderRegistration: Send + Sync {
    async fn register(&self, provider: Arc<dyn MemoryProvider>) -> Result<(), RegistrationError>;
}

#[async_trait]
pub trait CoordinatorStrategyRegistration: Send + Sync {
    async fn register(&self, strategy: Arc<dyn CoordinatorStrategy>) -> Result<(), RegistrationError>;
}

pub struct PluginActivationResult {
    pub registered_tools: Vec<String>,
    pub registered_hooks: Vec<String>,
    pub registered_skills: Vec<String>,
    pub registered_mcp: Vec<McpServerId>,
    pub occupied_slots: Vec<CapabilitySlot>,
}

pub enum CapabilitySlot {
    MemoryProvider,
    CustomToolset(String),
    CoordinatorStrategy,
}
```

**类型层与运行期双向校验**：

- 插件 manifest 声明 `tools: [a, b]` 但 `hooks: []` → `ctx.hooks == None`；插件没有路径注册 hook
- 插件 manifest 声明 `coordinator_strategy` → `ctx.coordinator == Some(...)`；未声明则为 `None`
- 插件试图注册未在 manifest 中声明的工具 `c` → `RegistrationError::UndeclaredTool { name: "c" }` 直接拒绝
- Activation 完成后，Registry 校验 `registered_* ⊆ declared_*`（注册多于声明拒绝），`pending_declared()` 非空发 Warning（声明而未实现）
- Plugin deactivate 时，每个 handle 内部记录的"由本插件注册的能力"自动注销，避免"漏删"（HER-035）

> 注意：本节 "Capability handle" 是**插件 → Registry**方向的注册接口；与 ADR-0011 `ToolCapability`（**工具 → Engine**方向的能力）同名词不同语义，引用时请保持上下文清晰。

## 3. 发现与加载（对齐 HER-033 / OC-17 / ADR-0015）

```rust
pub struct DiscoveryPipeline {
    sources: Vec<DiscoverySource>,
    manifest_loaders: Vec<Arc<dyn PluginManifestLoader>>,
    manifest_validator: Arc<ManifestValidator>,
}

impl DiscoveryPipeline {
    pub async fn scan(&self) -> Result<Vec<DiscoveredPlugin>, PluginError> {
        let mut all_records = Vec::new();
        for loader in &self.manifest_loaders {
            for source in &self.sources {
                match loader.enumerate(source).await {
                    Ok(records) => all_records.extend(records.into_iter().map(|r| (r, source.clone()))),
                    Err(ManifestLoaderError::Validation(failure)) => {
                        // 解析阶段失败：落 ManifestValidationFailed 事件，不进入 discovered
                        emit_manifest_validation_failed(failure, source);
                    }
                    Err(other) => return Err(other.into()),
                }
            }
        }

        // 已解析为 ManifestRecord 的进入 §4 全套 Validation
        self.manifest_validator.validate_batch(all_records)
    }
}
```

### 3.1 Discovery 阶段硬约束（对齐 HER-033 / OC-17 / CC-38 / ADR-0015）

Discovery 流水线**只允许**：

1. 列目录、读文件、解析 YAML/JSON
2. 校验 manifest（schema、签名、命名空间、版本范围）
3. 把结果写入 `discovered` 集合

Discovery 流水线**不得**：

1. 加载 / 链接 / 执行任何插件可执行代码（包括 cargo extension binary 的"探针式"调用主流程）
2. 打开 stdio / http 子进程（即使是只读探测）
3. 依赖网络（除非业务层已显式提供 `manifest_validator` 的远端 PKI 端点）

> ADR-0015 把上述硬约束**抬到类型系统**：`PluginManifestLoader::enumerate` 的返回类型是 `Vec<ManifestRecord>`，没有任何路径产出 `Arc<dyn Plugin>`；运行期实例化必须经过 `PluginRuntimeLoader`，且后者只能由 `PluginRegistry::activate` 触发。
>
> 反例（OpenClaw）：把"插件加载"与"插件代码执行"混在一起会导致整个 SDK 启动期信任域被插件污染。Octopus **必须**保留 manifest-first 与 runtime-load 的二分。

### 3.2 Loader 二分：`PluginManifestLoader` 与 `PluginRuntimeLoader`（ADR-0015）

```rust
/// 阶段 A：从 DiscoverySource 到 ManifestRecord（不实例化、不执行代码）
#[async_trait]
pub trait PluginManifestLoader: Send + Sync + 'static {
    async fn enumerate(&self, source: &DiscoverySource)
        -> Result<Vec<ManifestRecord>, ManifestLoaderError>;
}

pub struct ManifestRecord {
    pub manifest: PluginManifest,
    pub origin: ManifestOrigin,
    /// 解析后 canonicalize 字节的稳定哈希；进入 `Event::PluginLoaded.manifest_hash`
    pub manifest_hash: [u8; 32],
}

#[non_exhaustive]
pub enum ManifestOrigin {
    File { path: PathBuf },
    CargoExtension { binary: PathBuf, package_metadata: BTreeMap<String, Value> },
    RemoteRegistry { endpoint: Url, etag: Option<String> },
}

/// 阶段 B：仅由 `PluginRegistry::activate` 触发；可实例化 / dlopen / fork 子进程
#[async_trait]
pub trait PluginRuntimeLoader: Send + Sync + 'static {
    fn can_load(&self, manifest: &PluginManifest, origin: &ManifestOrigin) -> bool;
    async fn load(
        &self,
        manifest: &PluginManifest,
        origin: &ManifestOrigin,
    ) -> Result<Arc<dyn Plugin>, RuntimeLoaderError>;
}
```

#### 3.2.1 默认实现矩阵

| Loader | feature 门 | 用途 |
|---|---|---|
| `FileManifestLoader` | 默认开启 | 扫 `data/plugins/*/plugin.{json,yaml,yml}`、`~/.octopus/plugins/*/plugin.{json,yaml,yml}`、`.octopus/plugins/*/plugin.{json,yaml,yml}`；只读 YAML/JSON |
| `InlineManifestLoader` | 测试 / helper | 只返回显式注入的 `ManifestRecord`，不读文件、不执行插件代码 |
| `CargoExtensionManifestLoader` | 后续卡 | 在 `$PATH` 中找 `octopus-plugin-*`；解析 binary **冷启动冷退出**的元数据子命令输出（如 `--harness-manifest`）。**严禁**调用插件主流程；任何 IPC 形态都视为破坏 §3.1 |
| `StaticLinkRuntimeLoader` | 永远在 | 编译期链接：从 SDK 内部工厂注册表中查 `name@version` |
| `DylibRuntimeLoader` | `dynamic-load` | M5-T08 仅提供 API / error boundary；真实 `dlopen` 需另行完成 unsafe 治理修订 |
| `CargoExtensionRuntimeLoader` | 后续卡 | 派生子进程，按 stdio JSON-RPC 协议代理 `Plugin` trait 调用 |
| `WasmRuntimeLoader` | `wasm-runtime`（实验） | wasmtime 加载 WASI module |

> CargoExtension 元数据子命令的"冷启动冷退出"边界由 `CargoExtensionManifestLoader` 实现独立测试覆盖：禁止在元数据子命令中开 socket / 写非 stdout / 启子进程；超时（默认 1s）即视为破坏约束并落 `ManifestValidationFailedEvent::CargoExtensionMetadataMalformed`。

#### 3.2.2 与 Builder 的装配

- 业务方未注入 ManifestLoader 时，SDK 自动绑定 `FileManifestLoader`
- 业务方未注入 RuntimeLoader 时，SDK 默认绑定 `StaticLinkRuntimeLoader`
- `dynamic-load` feature 在 M5-T08 只暴露 `DylibRuntimeLoader` 占位边界；不得绕过全仓 `unsafe_code = forbid`
- 多个 RuntimeLoader 都 `can_load` 时按声明顺序取第一个；全部 false 抛 `PluginError::ActivateFailed("no runtime loader can handle origin: ...")`
- Builder 提供 `with_manifest_loader / with_runtime_loader` 注入自定义实现（详见 §14）

## 4. Manifest 校验

```rust
pub struct ManifestValidator {
    schema: JsonSchema,
    signer_store: Arc<dyn TrustedSignerStore>,
    min_harness_version: semver::Version,
    naming_policy: PluginNamingPolicy,
}

pub enum ValidationResult {
    Ok,
    Warning(Vec<String>),
    Rejected(RejectionReason),
}

/// 拒绝原因。本枚举**仅**承载"manifest 已被合法解析为 `PluginManifest` 后再被业务规则拒绝"的子情况，
/// 与 `Event::PluginRejected.reason`（contracts §3.3 / ADR-006 §6.3）一一对应。
///
/// **不属于此处**：YAML / JSON 解析错误、JSON schema 不通过、`manifest_schema_version` 不被支持、
/// cargo extension 元数据子命令冷输出无法解释 —— 这些都在 `ManifestRecord` 还构造不出来的阶段，
/// 对应 `Event::ManifestValidationFailed`（详见 `event-schema.md §3.20.3` 与 §16.1）。
#[non_exhaustive]
pub enum RejectionReason {
    /// 签名算法 / 签发者 / 时间戳 / payload 与签名不匹配
    SignatureInvalid { details: String },
    /// `signer` 不在 `TrustedSignerStore::list_active` 中
    UnknownSigner { signer: String },
    /// signer 已被撤销（ADR-0014 §2.5）；撤销时间填入 `revoked_at`
    SignerRevoked { signer: String, revoked_at: DateTime<Utc> },
    /// 声明的 `trust_level` 与实际来源不匹配（详见 ADR-006 §6.1）
    TrustMismatch { declared: TrustLevel, source: DiscoverySource },
    /// 命名空间冲突（保留前缀 / 同形字符 / 重复 PluginId 等，详见 §5）
    NamespaceConflict { details: String },
    /// 必需依赖缺失或版本不满足
    DependencyUnsatisfied { dependency: String, requirement: String },
    /// 依赖图存在环
    DependencyCycle { cycle: Vec<String> },
    /// `min_harness_version` 不兼容当前 SDK
    HarnessVersionIncompatible { required: String, actual: String },
    /// 独占型 Capability Slot 已被占用（详见 §6）
    SlotOccupied { slot: String, occupant: String },
    /// Admission 策略（§10）拒绝
    AdmissionDenied { policy: PluginAdmissionPolicyDiscriminant },
}

impl ManifestValidator {
    pub fn validate(&self, record: &ManifestRecord, source: &DiscoverySource) -> ValidationResult {
        // 1. 签名校验（AdminTrusted 必签；详见 §4.1 五步流程）
        // 2. 命名空间校验（§5）
        // 3. 最小 harness 版本兼容
        // 4. trust_level 与来源匹配（ADR-006）
        // 5. capabilities 字段层面无格式冲突
    }
}
```

> 拒绝必须是**单一原因**：第一个命中的 `RejectionReason` 即终止本插件的 Validation，避免错误信息混淆。Slot 冲突 / 依赖图冲突属于"全局阶段"问题，在 §6 / §9 单独处理。

### 4.1 `TrustedSignerStore` 与签名验证（ADR-0014）

```rust
#[async_trait]
pub trait TrustedSignerStore: Send + Sync + 'static {
    async fn list_active(&self) -> Result<Vec<TrustedSigner>, SignerStoreError>;
    async fn get(&self, id: &SignerId) -> Result<Option<TrustedSigner>, SignerStoreError>;
    async fn is_revoked(&self, id: &SignerId, at: DateTime<Utc>) -> Result<bool, SignerStoreError>;
    fn watch(&self) -> BoxStream<'static, SignerStoreEvent>;
}

pub struct TrustedSigner {
    pub id: SignerId,
    pub algorithm: SignatureAlgorithm,
    pub public_key: Bytes,
    /// 启用窗口：[activated_at, retired_at) 之外签的 manifest 一律拒绝
    pub activated_at: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub provenance: SignerProvenance,
}

pub struct SignerId(pub String);

#[non_exhaustive]
pub enum SignerProvenance {
    BuiltinOfficial,
    BuilderInjected,
    PkiEndpoint { endpoint: Url },
    PolicyFile { path: PathBuf },
}

#[non_exhaustive]
pub enum SignerStoreEvent {
    Added(SignerId),
    Updated(SignerId),
    Retired(SignerId),
    Revoked(SignerId),
}
```

#### 4.1.1 五步验签流程

ManifestValidator 在 §4 的"签名校验"步骤展开为：

```text
1. 解析 ManifestSignature { algorithm, signer, signature, timestamp }
2. signer_store.get(&signer)
   → None                         → RejectionReason::UnknownSigner { signer }
   → Some(s) 但 s.algorithm 不匹配  → RejectionReason::SignatureInvalid { details: "algorithm mismatch" }
3. 启用窗口与撤销
   - timestamp ∉ [s.activated_at, s.retired_at.unwrap_or(MAX))
                                  → SignatureInvalid { details: "timestamp out of activation window" }
   - signer_store.is_revoked(&signer, NOW)
                                  → SignerRevoked { signer, revoked_at }
4. 验签（payload = manifest 去掉 signature 字段后 RFC 8785 canonicalize）
   - 失败                          → SignatureInvalid
5. 通过；进入下游校验（§4 步骤 2~5）
```

#### 4.1.2 与 ADR-013 IntegritySigner 的边界

| 维度 | ADR-0013（`IntegritySigner`） | ADR-0014（`TrustedSignerStore` + Manifest） |
|---|---|---|
| 用途 | 防止本地权限决策文件被离线篡改 | 校验上游分发的 plugin manifest 来自可信签发方 |
| 信任方向 | 自签自验 | 第三方签发，本机仅校验 |
| 算法形态 | 对称（HMAC-SHA256 默认） | 非对称（Ed25519 默认；RsaPkcs1Sha256 兼容） |
| 凭证来源 | `CredentialSource`（Keychain / Vault / Env / Ephemeral） | `TrustedSignerStore`（编译期内嵌 / Builder 注入 / PKI 端点 / 策略文件） |
| 失败动作 | `Event::PermissionPersistenceTampered` | `Event::PluginRejected { reason: SignatureInvalid / UnknownSigner / SignerRevoked }` |

> 两者**不共享**实现也不共享配置：合一会让"本地权限"与"上游供应链"两个**正交**问题混淆，导致密钥泄露的 blast radius 越界。

#### 4.1.3 默认实现：`StaticTrustedSignerStore`

- `PluginRegistry::builder().with_trusted_signer(...)` 现有 API 保持不变；底层装配 `StaticTrustedSignerStore`，把传入的公钥包装为 `TrustedSigner { id: "user-injected-<idx>", provenance: BuilderInjected, .. }`
- 企业模式：`with_signer_store(Arc<dyn TrustedSignerStore>)` 是新接口；与 `with_trusted_signer` **互斥**（先注入 store 后再 `with_trusted_signer` 直接拒绝构建）
- `SignerStoreEvent` 让 `PluginRegistry` 在变更发生时刷新缓存；已 `Activated` 的插件**不**自动 deactivate（避免线上抖动），但下次 Discovery 按新规则重判（详见 §7）

## 5. 命名空间治理（对齐 CC-27 / HER-033）

### 5.1 PluginId

```rust
pub struct PluginId {
    pub name: PluginName,
    pub version: semver::Version,
}

pub struct PluginName(String);
```

`PluginId` 在整个 Registry 内**唯一**，是 Event / 指标 / 审计中引用插件的稳定标识；`Display` 形如 `octopus-invoice@1.2.3`。

### 5.2 PluginName 语法规则

| 规则 | 约束 |
|---|---|
| 字符集 | `[a-z0-9-]`（小写 ASCII + 数字 + 连字符） |
| 长度 | `1..=64` |
| 起止 | 必须以字母开头、不得以 `-` 结尾 |
| 不允许 | 大写、Unicode、下划线、点号、空格 |
| 同形字符防护 | Validator 在解析时执行 NFC 归一化并比对 ASCII 子集；任何非 ASCII 字符直接 `NamespaceConflict` |
| 保留前缀 | `octopus-` / `harness-` / `mcp-` 仅允许 `AdminTrusted` 来源使用；`UserControlled` 来源用同前缀直接拒绝 |

### 5.3 命名空间唯一性

- 同一 `PluginName` 不允许跨 Discovery 源重复出现；冲突发生时按 §3 source 优先级（Workspace > Cargo Extension > User > Project）取胜，被覆盖者落 `Event::PluginRejected { reason: NamespaceConflict }`
- 工具名命名沿用 `harness-tool §...` 与 `harness-mcp §...` 既有规则；插件不需要把工具名前缀与 plugin name 强绑定（保留命名灵活性），但 `ToolOrigin::Plugin { plugin_id, .. }` 字段（`harness-contracts §3.4` / `harness-tool §...`）必须落 `PluginId`

### 5.4 与 ADR-006 来源判定的关系

`UserControlled` 不允许使用保留前缀，避免"伪装成官方 Admin 插件"的钓鱼路径；`AdminTrusted` 来源必须签名，签名值与 `signer` 字段一起决定可信度（不依赖名字）。

### 5.5 SignerId 命名约定（ADR-0014）

`ManifestSignature.signer` 与 `TrustedSigner.id` 的字符串值建议遵守 `<provider>-<purpose>-<rev>` 三段式，例如：

| 示例 SignerId | 含义 |
|---|---|
| `octopus-official-2026-04` | Octopus 官方 2026-04 一轮的发布 signer |
| `acme-internal-prod-r2` | Acme 公司企业生产环境第 2 轮内部 signer |
| `octopus-supply-chain-emergency-rotation` | 紧急轮换专用 signer |

- 字符集：`[a-z0-9-]`，长度 `1..=128`
- `octopus-` / `harness-` 前缀仅 SDK 自带或通过 `BuiltinOfficial` provenance 注册
- 任意非 ASCII / 大写字符直接被 `TrustedSignerStore` 拒绝注册（与 §5.2 命名空间同形字符防护一致）

## 6. Capability Ownership 与 Slot 管理

### 6.1 Capability Ownership 原则

| 原则 | 含义 |
|---|---|
| **Registry 拥有 Capability** | 所有 Tool / Hook / MCP / Skill / Memory 实例最终归属对应 Registry；插件**只是来源标签**，不持有共享状态（HER-035） |
| **Slot 归属不可转让** | Capability Slot 上一旦激活某插件，其他插件不能"接管"或"覆盖"；deactivate 后 Slot 自动释放 |
| **Origin 必须可追溯** | 任意 Tool / Hook / MCP 实例必须能反查 `PluginId`，便于审计与权限粒度判定 |

### 6.2 Slot 类型与判定准则

| Slot | 多重性 | 判定准则 |
|---|---|---|
| `MemoryProvider` | 独占 | 同时存在两个 MemoryProvider 会导致语义不一致（双写、双读、跨 provider 召回不可解释）。对齐 OC-14 `plugins.slots.memory` |
| `CoordinatorStrategy` | 独占 | 仅当 manifest 声明 `capabilities.coordinator_strategy` 时可占用；M5 只登记 plugin 本地策略，M6 再桥接 Team |
| `CustomToolset(name)` | 同名独占、不同名可并存 | Toolset 是命名集合，命名相同视为定义冲突 |
| Tool / Hook / Skill / MCP（非槽位类） | 可叠加 | 由各自 Registry 管理同名裁决（详见 `harness-tool §...` 同名裁决矩阵） |

> 如果未来引入新的"独占型能力"，必须按上述原则评估并在本节增补；这是判断是否进入 `CapabilitySlot` 的唯一准绳。

### 6.3 实现

```rust
pub struct CapabilitySlotManager {
    occupied: HashMap<CapabilitySlot, PluginId>,
}

impl CapabilitySlotManager {
    pub fn try_occupy(
        &mut self,
        slot: CapabilitySlot,
        plugin_id: &PluginId,
    ) -> Result<(), PluginError> {
        if let Some(existing) = self.occupied.get(&slot) {
            return Err(PluginError::SlotOccupied {
                slot: format!("{:?}", slot),
                occupant: existing.to_string(),
            });
        }
        self.occupied.insert(slot, plugin_id.clone());
        Ok(())
    }
}
```

## 7. 生命周期状态机

### 7.1 状态枚举

```rust
#[non_exhaustive]
pub enum PluginLifecycleState {
    /// Discovery 完成、Manifest 校验通过；尚未 activate
    Validated,
    /// activate 进行中
    Activating,
    /// 已注册到各 Registry，可用
    Activated,
    /// deactivate 进行中
    Deactivating,
    /// 已下线（Registry 中残留为 `Origin` 标签的能力会被移除或降级）
    Deactivated,
    /// 任意阶段被 Validator / Slot / 依赖图 / activate 流程拒绝
    Rejected(RejectionReason),
    /// activate 中途失败（已部分注册的能力会被回滚到 `Validated`）
    Failed(PluginError),
}
```

### 7.2 状态转换

```text
        ┌──────────────┐
        │  (file scan) │
        ▼              │
   Discovered ─► Rejected (manifest / 命名空间 / 依赖 / 签名)
        │
        │ validate Ok
        ▼
   Validated ──► Rejected (Slot / 依赖图)
        │
        │ activate()
        ▼
   Activating ──► Failed (rollback)
        │
        │ activate Ok
        ▼
   Activated ──► Deactivating ──► Deactivated
                       │
                       └──► Failed (deactivate err)
```

> 转换规则：
>
> - `Discovered` 是 Validator 的瞬态视图，不进入 `state` 字典；外部观察到的最早状态是 `Validated` 或 `Rejected`
> - `Activating / Deactivating` 不可观察跨多次调用：同一插件不允许并发 activate / deactivate，Registry 内部用锁保证
> - `Failed → Validated` 是合法回退（业务层修复依赖后可以重试 activate）；`Rejected` 是终态，必须重新 Discovery（重读 manifest）才能再次进入候选
> - 任意状态变更都会发审计事件（详见 §10）

### 7.3 与 Prompt Cache 的耦合

`activate` / `deactivate` 都会改变可见的 Tool / Hook / Skill / MCP 集合，**必然**触发 ADR-003 的 prompt cache 失效语义；具体由各 Registry 结合 `harness-tool §...` 与 `harness-context §...` 的快照机制保证。插件作者不需要手动处理 cache，但必须接受"在 Run 进行中 activate/deactivate 会导致下一轮 system prompt / toolset 重组"的事实——这也是为什么 `activate` 推荐放在 Session 创建前或 idle 间隙。

## 8. 信任域二分（对齐 ADR-006）

```rust
impl PluginRegistry {
    async fn activate_internal(&self, plugin: DiscoveredPlugin) -> Result<(), PluginError> {
        let trust = plugin.manifest.trust_level;

        for tool in &plugin.manifest.capabilities.tools {
            if tool.is_destructive && trust != TrustLevel::AdminTrusted {
                return Err(PluginError::TrustViolation {
                    capability: format!("tool:{}", tool.name),
                    required: TrustLevel::AdminTrusted,
                    provided: trust,
                });
            }
        }

        for hook in &plugin.manifest.capabilities.hooks {
            if matches!(hook.transport, HookTransport::Exec) && trust != TrustLevel::AdminTrusted {
                return Err(PluginError::TrustViolation { /* ... */ });
            }
        }

        for mcp in &plugin.manifest.capabilities.mcp_servers {
            if mcp.is_remote_http && trust != TrustLevel::AdminTrusted {
                return Err(PluginError::TrustViolation { /* ... */ });
            }
        }

        // 实际注册...
    }
}
```

> `TrustLevel` 永远由 `DiscoverySource` 推导，不接受 manifest 自我声明高于来源默认值；详见 `adr/0006-plugin-trust-levels.md §2.3 / §2.4`。

## 9. 依赖图

### 9.1 解析顺序

`activate(id)` 触发时，Registry 按拓扑序激活 `id` 的所有 `Required` 依赖；`Optional` 依赖缺失只发 Warning，不阻断激活。

```rust
impl PluginRegistry {
    /// 计算 plugin 的拓扑顺序；返回 Err(DependencyCycle) 时 Registry 全局拒绝激活相关子图
    fn resolve_activation_order(&self, root: &PluginId) -> Result<Vec<PluginId>, PluginError>;
}
```

### 9.2 版本约束

- `PluginDependency.version_req` 使用 SemVer Range 语法
- 解析顺序：`Discovered` 集合内取最高满足约束的版本；不存在时按依赖类型决定 `DependencyUnsatisfied`（Required）或 Warning（Optional）
- 同名插件多版本同时被需要时（如 A 依赖 `^1.0`、B 依赖 `^2.0`）：当前 SDK 不支持多版本共存，按 `DependencyUnsatisfied` 直接拒绝；解决方式由业务层选择（升级/卸载/分离 Workspace）

### 9.3 循环检测

依赖图必须无环；环出现时该子图内**所有**插件状态置为 `Rejected(DependencyCycle)`，不进入 `Validated`。检测算法详见 §11 测试要求。

### 9.4 Deactivate 顺序

`deactivate(id)` 与 `activate` 严格逆序：先停用反向依赖该插件的其他插件，再停用自身；任何反向依赖处于 `Activated` 状态时，`deactivate(id)` 默认拒绝（`PluginError::ActiveDependents`），业务层可显式 `deactivate_cascade` 强制级联。

## 10. 配置策略

```rust
pub struct PluginConfig {
    /// 全局总开关；false 时整个插件子系统跳过 discover/activate
    pub enabled: bool,

    /// 名单策略；默认 `AllowAll`
    pub policy: PluginAdmissionPolicy,

    /// 单插件配置（key 为 `PluginName`），传入 `PluginActivationContext.config`
    pub entries: HashMap<PluginName, Value>,
}

#[non_exhaustive]
pub enum PluginAdmissionPolicy {
    /// 全部允许（subject to TrustLevel / Slot / 依赖图）
    AllowAll,
    /// 仅允许 allowlist 内的插件加载
    Allow(HashSet<PluginName>),
    /// 加载所有，但 denylist 内的强制 `Rejected`
    Deny(HashSet<PluginName>),
}
```

适用规则：

- `enabled = false` 时，`PluginRegistry::discover` 直接返回空集；与"插件子系统未编译"区分（后者由 feature flag 控制，详见 §13）
- `Allow` / `Deny` 在 Discovery 阶段就直接决定 `Rejected`，避免无谓的签名 / 依赖解析
- `entries` 是各插件运行时配置，schema 由 `PluginCapabilities.configuration_schema` 描述；**禁止**包含明文密钥（与 `security-trust §9.X.3` 一致）

> Admission 策略是运维层手段，与信任域二分（ADR-006）正交：被 Allow 的插件仍受 TrustLevel 能力矩阵限制；被 Deny 的插件即使是 `AdminTrusted` 也不得加载。

## 11. `strictPluginOnlyCustomization`

```rust
pub struct StrictPluginOnlyPolicy {
    pub enabled: bool,
    pub enforcement: EnforcementLevel,
}

pub enum EnforcementLevel {
    Warn,
    Block,
}
```

当 Admin-Trusted Plugin 声明此 policy 时，User-Controlled Plugin 不能注册任何 Tool。

## 12. Feature Flags

```toml
[features]
default = []
dynamic-load = []
manifest-sign = []
wasm-runtime = ["dep:wasmtime"]   # 实验
```

## 13. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("manifest loader error: {0}")]
    ManifestLoader(#[from] ManifestLoaderError),

    #[error("runtime loader error: {0}")]
    RuntimeLoader(#[from] RuntimeLoaderError),

    #[error("signer store error: {0}")]
    SignerStore(#[from] SignerStoreError),

    #[error("registration error: {0}")]
    Registration(#[from] RegistrationError),

    #[error("signature invalid")]
    SignatureInvalid,

    #[error("unknown signer: {0}")]
    UnknownSigner(String),

    #[error("signer revoked: {signer} at {revoked_at}")]
    SignerRevoked { signer: String, revoked_at: DateTime<Utc> },

    #[error("trust violation: {capability} requires {required:?}, got {provided:?}")]
    TrustViolation {
        capability: String,
        required: TrustLevel,
        provided: TrustLevel,
    },

    #[error("namespace conflict: {0}")]
    NamespaceConflict(String),

    #[error("slot occupied: {slot} by {occupant}")]
    SlotOccupied { slot: String, occupant: String },

    #[error("version incompatible: requires {required}, got {actual}")]
    VersionIncompatible { required: String, actual: String },

    #[error("dependency unsatisfied: {dependency} requires {requirement}")]
    DependencyUnsatisfied { dependency: String, requirement: String },

    #[error("dependency cycle: {0:?}")]
    DependencyCycle(Vec<String>),

    #[error("active dependents present: {0:?}")]
    ActiveDependents(Vec<String>),

    #[error("plugin not found: {0}")]
    NotFound(String),

    #[error("activate failed: {0}")]
    ActivateFailed(String),

    #[error("deactivate failed: {0}")]
    DeactivateFailed(String),

    #[error("admission denied: {0:?}")]
    AdmissionDenied(PluginAdmissionPolicyDiscriminant),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// `PluginManifestLoader::enumerate` 的错误根。
/// `Validation` 子情况会被 `DiscoveryPipeline` 翻译为 `Event::ManifestValidationFailed`，
/// 不会污染 `Event::PluginRejected`。
#[derive(Debug, thiserror::Error)]
pub enum ManifestLoaderError {
    #[error("manifest validation failed at {origin:?}: {failure:?}")]
    Validation {
        origin: ManifestOrigin,
        failure: ManifestValidationFailure,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// `PluginRuntimeLoader::load` 的错误根。
#[derive(Debug, thiserror::Error)]
pub enum RuntimeLoaderError {
    #[error("no runtime loader can handle origin: {0:?}")]
    NoMatchingLoader(ManifestOrigin),
    #[error("runtime loader unsupported: {0}")]
    Unsupported(String),
    #[error("subprocess spawn failed: {0}")]
    SubprocessSpawn(String),
    #[error("wasm instantiate failed: {0}")]
    WasmInstantiate(String),
    #[error("entrypoint missing or wrong abi: {0}")]
    EntrypointAbi(String),
}

/// `TrustedSignerStore` 实现的错误根。
#[derive(Debug, thiserror::Error)]
pub enum SignerStoreError {
    #[error("invalid signer id: {0}")]
    InvalidId(String),
    #[error("policy file invalid: {0}")]
    PolicyFile(String),
    #[error("pki endpoint unreachable: {0}")]
    PkiEndpoint(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// `*Registration` trait 在 activate 期注册能力时可能产生的错误。
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    #[error("undeclared tool: {name}")]
    UndeclaredTool { name: String },
    #[error("undeclared hook: {name}")]
    UndeclaredHook { name: String },
    #[error("undeclared mcp server: {name}")]
    UndeclaredMcpServer { name: String },
    #[error("undeclared skill: {name}")]
    UndeclaredSkill { name: String },
    #[error("trust violation while registering {capability}")]
    TrustViolation { capability: String },
    #[error("registry rejected: {0}")]
    Registry(String),
}
```

`ManifestValidationFailure` 见 `event-schema.md §3.20.3`。

## 14. 使用示例

### 14.1 最简：默认 ManifestLoader / RuntimeLoader / Signer

```rust
let registry = PluginRegistry::builder()
    .with_source(DiscoverySource::Workspace("data/plugins".into()))
    .with_source(DiscoverySource::User(dirs::home_dir().unwrap().join(".octopus/plugins")))
    // 等价于把这把 key 装进默认 StaticTrustedSignerStore，provenance = BuilderInjected
    .with_trusted_signer(include_bytes!("octopus-official.pub"))
    .with_config(PluginConfig {
        enabled: true,
        policy: PluginAdmissionPolicy::AllowAll,
        entries: HashMap::new(),
    })
    .build()?;

let discovered = registry.discover().await?;
for plugin in &discovered {
    tracing::info!(
        id = %plugin.record.manifest.identity(),
        trust = ?plugin.record.manifest.trust_level,
        result = ?plugin.validation_result,
        "discovered"
    );
}

for id in [
    PluginId::parse("octopus-invoice@1.2.3")?,
    PluginId::parse("octopus-review@0.4.0")?,
] {
    registry.activate(&id).await?;
}
```

### 14.2 企业级：自定义 SignerStore + ManifestLoader + RuntimeLoader

```rust
let registry = PluginRegistry::builder()
    .with_source(DiscoverySource::Workspace("data/plugins".into()))

    // 注入企业 PKI 端点 + 撤销列表
    .with_signer_store(Arc::new(MyPkiSignerStore::new(pki_endpoint, crl_endpoint)))

    // 与默认 FileManifestLoader 平行存在的私有注册中心
    .with_manifest_loader(Arc::new(MyRemoteRegistryLoader::new(registry_endpoint)))

    // dynamic-load feature 启用后，DylibRuntimeLoader 自动加入；
    // 这里再叠加 wasm 形态
    .with_runtime_loader(Arc::new(WasmRuntimeLoader::new(wasm_engine)))

    .with_config(PluginConfig {
        enabled: true,
        policy: PluginAdmissionPolicy::Allow(allowed_plugin_names()),
        entries: per_plugin_entries(),
    })
    .build()?;

// 订阅 SignerStore 变更：撤销发生时刷新 discovered，但不强制 deactivate Activated 插件
let mut events = registry.subscribe_signer_events();
tokio::spawn(async move {
    while let Some(ev) = events.next().await {
        if let SignerStoreEvent::Revoked(id) = ev {
            tracing::warn!(signer = %id, "signer revoked; next discover will reject affected plugins");
        }
    }
});
```

### 14.3 互斥规则

- `with_signer_store` 与 `with_trusted_signer` 互斥：先注入 store 后再调 `with_trusted_signer` 会在 `build()` 期返回 builder error
- 同一 `PluginRegistryBuilder` 内 `with_manifest_loader` / `with_runtime_loader` 可多次调用；声明顺序即询问顺序
- 业务方未注入任何 ManifestLoader 时，SDK 自动绑定 `FileManifestLoader`

## 15. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | Manifest schema / 签名校验 / 未知字段 / `manifest_schema_version` 范围 |
| ManifestValidationFailed | 坏 YAML / schema violation / `manifest_schema_version` 超界 / cargo extension 元数据子命令冷输出 malformed → 落 `Event::ManifestValidationFailed`，不落 `PluginRejected` |
| 命名空间 | 大写/Unicode/同形字符/保留前缀拒绝、跨源重名优先级 |
| 信任域 | User-controlled 注册破坏性 Tool / Exec Hook / 远端 HTTP MCP 被拒 |
| Discovery 隔离 | 验证 Discovery 阶段无任何子进程 / dlopen / 网络调用（通过 `MockSandboxProbe` 注入断言） |
| Loader 二分 | 自定义 ManifestLoader 不能产出 `Arc<dyn Plugin>`（类型层断言）；自定义 RuntimeLoader 仅在 `activate` 路径被调用（启动期断言计数器为 0） |
| CapabilityHandle 越权 | 注册未声明工具 / hook / mcp / skill → `RegistrationError::Undeclared*`；声明 tools 但 ctx.hooks 必须为 None；声明 coordinator strategy 时 ctx.coordinator 必须为 Some |
| Signer 启用窗口 | timestamp 早于 `activated_at` 或 ≥ `retired_at` → `SignatureInvalid("timestamp out of activation window")` |
| Signer 撤销 | `revoked_at = NOW` 后下次 Discovery 立即落 `SignerRevoked`；已 Activated 的不强制 deactivate（行为日志） |
| Signer 轮换 | 老 + 新 signer 并行；老 signer 签的存量插件继续可用，新签必须用新 signer |
| Slot | Memory / CoordinatorStrategy / 同名 Toolset 占用后拒绝第二个 |
| 依赖图 | 拓扑顺序、版本范围满足/不满足、Required/Optional 行为差异 |
| 循环依赖 | 直接环 / 间接环 / 自环检测 |
| 状态机 | `Validated → Activating → Activated → Deactivating → Deactivated` 全路径；`Failed` 回滚到 `Validated`；`Rejected` 终态 |
| Admission | `AllowAll` / `Allow` / `Deny` 三种策略 |
| 反向依赖 | `deactivate` 默认拒绝在线反向依赖；`deactivate_cascade` 级联成功 |

## 16. 可观测性

### 16.1 必记审计事件（不可禁用）

| Event variant（contracts §3.3 / event-schema §3.20） | 触发场景 | 关键字段 |
|---|---|---|
| `Event::PluginLoaded` | 状态转入 `Activated` | `plugin_id` / `trust_level` / `capabilities` / `manifest_hash` |
| `Event::PluginRejected` | Manifest 已解析、被业务规则拒绝（信任 / 命名 / 依赖 / Slot / 签名 / 撤销 / Admission） | `plugin_id` / `reason: RejectionReason`（详见 §4） |
| `Event::ManifestValidationFailed` | Manifest 解析阶段就失败（YAML 错 / schema 不通过 / `manifest_schema_version` 不被支持 / cargo extension 元数据子命令冷输出 malformed），尚未构造出可信 `PluginId` | `manifest_origin` / `partial_name?` / `partial_version?` / `failure: ManifestValidationFailure` |

> 三者**互斥**：成功 → `PluginLoaded`；解析失败 → `ManifestValidationFailed`；解析成功后被拒 → `PluginRejected`。三个 Event variant 已纳入 `event-schema.md §9.3 DEFAULT_NEVER_DROP_KINDS` 与 `security-trust.md §8.1` 必记清单。

### 16.2 指标

| 指标 | 说明 |
|---|---|
| `plugin_discovered_total` | 按 source 分桶 |
| `plugin_activated_total` | 按 trust_level 分桶 |
| `plugin_rejected_total` | 按 `RejectionReason` 分桶 |
| `plugin_manifest_validation_failed_total` | 按 `ManifestValidationFailure` 分桶 |
| `plugin_signature_validation_duration_ms` | 签名校验耗时 |
| `plugin_signer_active_total` | 当前 `TrustedSignerStore::list_active` 长度（按 provenance 分桶） |
| `plugin_signer_revoked_total` | 累计已撤销 signer 数 |
| `plugin_dependency_resolution_duration_ms` | 依赖图解析耗时 |
| `plugin_active_total` | 当前处于 `Activated` 状态的插件数（按 trust_level 分桶） |
| `plugin_capability_registration_rejected_total` | Capability handle 越权拦截命中数（按 `RegistrationError::Undeclared*` 分桶） |

## 17. 反模式

| 反模式 | 原因 |
|---|---|
| 在 Plugin `activate` 里做长耗时 IO（应 lazy） | 违反 manifest-first / lazy runtime；阻塞 Registry 与 ADR-003 prompt cache 重组 |
| 在 Discovery 阶段 dlopen / 启动子进程 | 违反 §3.1 硬约束；插件代码在校验前不得执行 |
| 自定义 `PluginManifestLoader` 中调用插件主流程 / 启动子进程 | 违反 ADR-0015 类型层硬约束；ManifestLoader 只允许产出 `ManifestRecord`，不得返回 `Arc<dyn Plugin>` 或副作用 |
| Cargo extension 元数据子命令在冷输出阶段开 socket / 启子进程 | 越界破坏 §3.1；`CargoExtensionManifestLoader` 默认 1s 超时 + 冷退出断言，会落 `ManifestValidationFailedEvent::CargoExtensionMetadataMalformed` |
| 通过 `PluginActivationContext` 注册未在 manifest 声明的工具 / hook / mcp / skill | `RegistrationError::Undeclared*` 直接拒绝；ADR-0015 capability handle 双向校验 |
| User-controlled Plugin 声明 `trust_level: admin-trusted` | Validator 按 ADR-006 §2.3 来源判定直接 `TrustMismatch` |
| Plugin 修改 harness 核心代码 / 替换核心 trait 实现（HER-035） | 插件只能"批量封装扩展"；任何对核心 trait 的"覆盖"都必须走 ADR + SDK 主线 |
| 未声明 `min_harness_version` 导致跨版本不兼容 | Validator 默认 `*` 但会发 Warning；建议显式声明 |
| `UserControlled` 插件使用 `octopus-` / `harness-` / `mcp-` 保留前缀 | 同形钓鱼；Validator 直接 `NamespaceConflict` |
| 在 manifest `entries` / `configuration_schema` 中放明文密钥 | 与 `security-trust §9.X.3` 一致：仅允许 `ref:` / `env:` / `vault:` 引用 |
| 把 `PluginRejected` 视为 Warning 处理 | 拒绝是终态，业务层应触发告警而非简单忽略 |
| 复用 `IntegritySigner`（ADR-0013）的 HMAC key 来签 plugin manifest | ADR-0013 是本地权限对称签名；Manifest 签名是上游非对称签名（ADR-0014）；混用会让密钥泄露 blast radius 越界 |
| 撤销 signer 时强制 `deactivate` 所有 Activated 插件 | ADR-0014 §2.5 明确：撤销不强制下线，避免线上抖动；下次 Discovery 重判 |
| 把 `with_signer_store` 与 `with_trusted_signer` 同时调用 | Builder 互斥规则（§14.3）；`build()` 期返回 builder error |

## 18. 相关

- D2 · `module-boundaries.md`（依赖矩阵；§6 加入 `harness-plugin ↔ Loader` 检查项）
- D3 · `api-contracts.md`（注册入口契约）
- D4 · `event-schema.md` §3.20（PluginLoaded / PluginRejected / ManifestValidationFailed schema） / §9.3（never-drop 语义）
- D7 · `extensibility.md` §11 Plugin（业务面最小上手）
- D9 · `security-trust.md` §8.1 / §9.2（必记事件、签名、SignerStore）
- ADR-003 prompt cache 锁定（activate / deactivate 与重组）
- ADR-006 插件信任域二分
- ADR-0013 IntegritySigner（与本 crate Manifest 签名**正交**，不共享实现）
- ADR-0014 Plugin Manifest Signer 治理（TrustedSignerStore / 启用窗口 / 撤销列表）
- ADR-0015 Plugin Loader 二分 + Capability-Scoped ActivationContext
- Evidence: HER-033, HER-035, OC-16, OC-17, OC-18, CC-27, CC-38
