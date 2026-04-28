# ADR-0015 · Plugin Loader 二分 + Capability-Scoped ActivationContext

> 状态：Accepted
> 日期：2026-04-25
> 决策者：架构组
> 关联：ADR-006（插件信任域二分）、ADR-0014（Plugin Manifest Signer 治理）、`crates/harness-plugin.md`、`extensibility.md §11`

## 1. 背景与问题

`harness-plugin.md` v1.6 已经把 manifest-first / lazy runtime 写进 §3.1 硬约束（"Discovery 阶段不得加载 / 链接 / 执行任何插件代码"）；§7 也定义了完整的生命周期状态机。但 trait 层面有两个未解决的问题：

1. **加载职责合一**：当前没有 `Loader` trait，"读 manifest" 与"实例化插件"被默认堆在 `PluginRegistry` 内部。这意味着：
   - "Discovery 阶段不得执行代码"只能靠**SPEC 文字约束**+ 代码 review 把守，类型系统无法保护
   - 第三方提供的"自定义 Discovery"（例如远端注册中心）必须自己重新实现一套 Manifest 解析与校验，不能复用 SDK
   - 静态链接 / 动态库 / Cargo extension fork / WASM 等多种"runtime 加载形态"无法平滑共存

2. **ActivationContext 是大而全的句柄包**：当前 `PluginActivationContext` 把 `tool_registry / hook_registry / mcp_registry / skill_registry / memory_manager` 全部 `Arc<...>` 一起塞给插件，无论 manifest 声明了哪些 capabilities。这意味着：
   - manifest 只声明 `tools` 的插件依然能拿到 `hook_registry` 句柄并越权注册 hook
   - "manifest 是单一事实来源"在类型层面没有保护
   - Registry 视图与全局视图不可区分（业务实现"在测试场景注入 Mock Registry"时容易把全局对象误传）

不固化这两点，前面 ADR-006 / ADR-0014 设的所有信任边界都得不到类型层面的保护——单凭 review 兜底太脆弱。

## 2. 决策

### 2.1 Loader 二分：`PluginManifestLoader` 与 `PluginRuntimeLoader`

```rust
/// 阶段 A：从 DiscoverySource 到 ManifestRecord。
///
/// **类型层硬约束**：trait 方法不接收任何 ActivationContext / Registry，
/// 因此实现者**没有路径**把"加载完的代码"注册到 Registry。
/// `harness-plugin §3.1` 的"Discovery 不得执行插件代码"硬约束借此进入类型系统。
#[async_trait]
pub trait PluginManifestLoader: Send + Sync + 'static {
    /// 给定一个 DiscoverySource，扫出该源下所有 ManifestRecord（不解释、不验签、不展开）。
    /// 解释 / 验签 / 命名空间 / 依赖图等都由 PluginRegistry 在拿到 ManifestRecord 后做。
    async fn enumerate(&self, source: &DiscoverySource)
        -> Result<Vec<ManifestRecord>, ManifestLoaderError>;
}

pub struct ManifestRecord {
    /// 已解析的 Manifest 结构（YAML/JSON → Rust 结构体）
    pub manifest: PluginManifest,
    /// 解析时的源文件 / Cargo bin 元数据 / 远端 URL 等溯源信息
    pub origin: ManifestOrigin,
    /// 解析时计算的稳定哈希；进入 `Event::PluginLoaded.manifest_hash`
    pub manifest_hash: [u8; 32],
}

#[non_exhaustive]
pub enum ManifestOrigin {
    File { path: PathBuf },
    CargoExtension { binary: PathBuf, package_metadata: BTreeMap<String, Value> },
    RemoteRegistry { endpoint: Url, etag: Option<String> },
}

/// 阶段 B：从已校验通过的 ManifestRecord 到运行期 `Arc<dyn Plugin>`。
///
/// 只有 PluginRegistry 在 `activate(id)` 时才会调用本 trait；
/// 业务方实现 PluginManifestLoader 时**无法**通过其他路径触发 PluginRuntimeLoader。
#[async_trait]
pub trait PluginRuntimeLoader: Send + Sync + 'static {
    /// 是否能加载某个 Manifest（按 origin / capabilities 决定）
    fn can_load(&self, manifest: &PluginManifest, origin: &ManifestOrigin) -> bool;

    /// 实例化插件；调用方保证 `manifest` 已通过 §4 全部 Validation
    async fn load(
        &self,
        manifest: &PluginManifest,
        origin: &ManifestOrigin,
    ) -> Result<Arc<dyn Plugin>, RuntimeLoaderError>;
}
```

### 2.2 默认实现矩阵

| Loader | 实现 | 用途 |
|---|---|---|
| `FileManifestLoader` | 默认 ManifestLoader | 扫 `data/plugins/*/plugin.{json,yaml,yml}` / `~/.octopus/plugins/*/plugin.{json,yaml,yml}` / `.octopus/plugins/*/plugin.{json,yaml,yml}`；只读 YAML/JSON |
| `InlineManifestLoader` | 测试 / helper ManifestLoader | 只返回显式注入的 `ManifestRecord`，不读文件、不执行插件代码 |
| `CargoExtensionManifestLoader` | 后续卡 | 在 `$PATH` 中找 `octopus-plugin-*` 可执行；解析 binary 内嵌 metadata（如 `cargo metadata` JSON 或 `--manifest` 子命令的**冷启动输出**），但**不**调用任何插件主流程 |
| `StaticLinkRuntimeLoader` | 默认 RuntimeLoader | 编译期链接：`Plugin` 是已存在于二进制内的 type；通过 manifest `name@version` 找到注册过的工厂函数 |
| `DylibRuntimeLoader` | feature `dynamic-load` | M5-T08 仅提供 API / error boundary；真实 `dlopen` 需另行完成 unsafe 治理修订 |
| `CargoExtensionRuntimeLoader` | 后续卡 | 派生子进程，按 stdio JSON-RPC 协议代理 Plugin trait 调用 |
| `WasmRuntimeLoader` | feature `wasm-runtime`（实验） | wasmtime 加载 WASI module，沙箱内调用 |

> CargoExtensionManifestLoader 的"解析 binary 内嵌 metadata"是**冷启动冷退出**的元数据子命令调用，不进入插件主流程；这一点必须在 SPEC 显式说明（详见落地清单）。如未来认为此调用形态仍然违反 §3.1 硬约束，可将 cargo extension 的 manifest 改为"独立 `plugin.yaml` 与可执行同目录共存"形态——本 ADR 把决定权留给后续 follow-up，但默认实现的边界已锁死。

### 2.3 PluginRegistry 与 Loader 的装配

```rust
impl PluginRegistryBuilder {
    /// 注册 ManifestLoader（可多个；按声明顺序对每个 DiscoverySource 询问 can_load）
    pub fn with_manifest_loader(self, loader: Arc<dyn PluginManifestLoader>) -> Self;

    /// 注册 RuntimeLoader（可多个；activate 时按声明顺序询问 can_load）
    pub fn with_runtime_loader(self, loader: Arc<dyn PluginRuntimeLoader>) -> Self;

    /// M5-T08 默认装配：FileManifestLoader + StaticLinkRuntimeLoader
    pub fn build(self) -> PluginRegistry;
}
```

**装配规则**：

- 业务方未注入任何 ManifestLoader 时，SDK 自动绑定 `FileManifestLoader`
- 业务方未注入 RuntimeLoader 时，SDK 默认绑定 `StaticLinkRuntimeLoader`
- `dynamic-load` feature 在 M5-T08 只暴露 `DylibRuntimeLoader` 占位边界；不得绕过全仓 `unsafe_code = forbid`
- 多个 RuntimeLoader 都 `can_load` 时，按声明顺序取第一个；若全部返回 false，`PluginError::ActivateFailed("no runtime loader can handle origin: ...")`

### 2.4 Capability-Scoped ActivationContext

把 `PluginActivationContext` 从"Registry 句柄包"重构为"capability handle 集合"：

```rust
pub struct PluginActivationContext {
    pub trust_level: TrustLevel,
    pub plugin_id: PluginId,
    pub config: Value,
    pub workspace_root: Option<PathBuf>,

    /// 仅当 manifest.capabilities.tools 非空时为 Some；
    /// `dyn ToolRegistration` 是窄接口 trait，仅暴露注册当前插件 declared 工具的方法。
    pub tools: Option<Arc<dyn ToolRegistration>>,
    pub hooks: Option<Arc<dyn HookRegistration>>,
    pub mcp: Option<Arc<dyn McpRegistration>>,
    pub skills: Option<Arc<dyn SkillRegistration>>,
    pub memory: Option<Arc<dyn MemoryProviderRegistration>>,
    pub coordinator: Option<Arc<dyn CoordinatorStrategyRegistration>>,
}
```

每个窄接口 trait 形如：

```rust
#[async_trait]
pub trait ToolRegistration: Send + Sync {
    /// 注册一个 Tool；implementer **必须**在 manifest.capabilities.tools 中声明同名 entry，
    /// 否则返回 `RegistrationError::UndeclaredTool`。
    async fn register(&self, tool: Arc<dyn Tool>) -> Result<(), RegistrationError>;

    /// 已声明但当前插件还没注册的工具列表（用于 activate 期间检查"声明而未实现"）
    fn pending_declared(&self) -> Vec<&str>;
}

/// HookRegistration / McpRegistration / SkillRegistration / MemoryProviderRegistration / CoordinatorStrategyRegistration
/// 形态同 ToolRegistration：只暴露当前插件声明范围内的注册接口。
```

**类型层面保证**：

- 插件 manifest 声明 `tools: [a, b]` 但 `hooks: []` → `ctx.hooks == None` → 没有路径注册 hook
- 插件试图注册未在 manifest 中声明的工具 `c` → `RegistrationError::UndeclaredTool` 直接拒绝
- 插件 deactivate 时，每个 handle 内部记录的"由本插件注册的能力"都会一并下线，避免"一处注册多处遗忘"

### 2.5 Activation 流程

```text
PluginRegistry::activate(id)
  ├─ 拓扑解析依赖（§9）
  ├─ 选定 RuntimeLoader（按 §2.3 顺序）
  ├─ runtime_loader.load(manifest, origin) → Arc<dyn Plugin>
  ├─ 构造 PluginActivationContext，按 manifest.capabilities 决定哪些 handle 注入 Some
  ├─ plugin.activate(ctx) → PluginActivationResult
  ├─ 校验 result：
  │   - registered_tools 必须 ⊆ manifest.capabilities.tools.name 集
  │   - 类似检查 hooks / mcp / skills / memory_provider / coordinator
  │   - pending_declared() 返回非空 → Warning（声明而未实现）
  ├─ Slot 校验（§6）；通过即落 PluginLifecycleState::Activated
  └─ 失败 → 回滚已注册能力，状态置 Failed
```

> 校验 `registered_tools ⊆ declared_tools` 是把"manifest 是单一事实来源"原则**双向**强制：注册多于声明拒绝，声明多于注册告警。

### 2.6 与 ADR-006 / 0009 / 0011 的协作

- **ADR-006**：本 ADR 不动信任域二分；TrustLevel 仍由 `DiscoverySource` 决定，capability handle 在装配期就把 `trust_level` 传下去（注册时叠加判定）
- **ADR-009 Deferred Tool Loading**：`ToolRegistration::register` 内部走 `ToolRegistry::register_from_plugin`，与 `DeferPolicy` 兼容
- **ADR-011 Tool Capability Handle**：本 ADR 的"Capability handle"是**插件→Registry**方向；ADR-011 的 `ToolCapability` 是**工具→Engine 能力**方向；两者同名词不同语义，已在 SPEC 中加注释区分

### 2.7 Discovery 路径不再走 PluginRegistry 内部

```rust
impl PluginRegistry {
    pub async fn discover(&self) -> Result<Vec<DiscoveredPlugin>, PluginError> {
        let mut all_records = Vec::new();
        for loader in &self.manifest_loaders {
            for source in &self.discovery_sources {
                all_records.extend(loader.enumerate(source).await?);
            }
        }

        // ManifestRecord -> DiscoveredPlugin 的 Validation 路径（§4 / §5 / §9）
        validate_and_resolve(all_records, &self.signer_store, &self.naming_policy, /* ... */)
    }
}
```

> `loader.enumerate` 在类型上**只能**返回 `ManifestRecord`，不允许返回 `Arc<dyn Plugin>`；这是 §3.1 硬约束在类型系统的体现。

## 3. 替代方案

### 3.1 单一 `PluginLoader` trait 同时承担两阶段

- ❌ 类型系统无法阻止"在 enumerate 里 dlopen / fork"
- ❌ 业务自定义 ManifestLoader（如 PKI 远端目录）会被"必须给我返回 Plugin"绑死

### 3.2 把 ActivationContext 改为闭包参数 + Plugin trait 拆 5 个 activate 方法

```rust
async fn activate_tools(&self, ctx: ToolActivationContext) -> ...;
async fn activate_hooks(&self, ctx: HookActivationContext) -> ...;
// ...
```

- ✅ 类型层面更严格
- ❌ Plugin trait 一次有 5+ 个可选 activate 方法，业务实现要写一堆 `default { Ok(...) }`
- ❌ 与 manifest 的 capabilities 字段是"声明 → 检查"关系不一致；改成"声明 → 选择性 dispatch"会引入额外失败路径

### 3.3 Capability handle 集合（采纳）

- ✅ Plugin trait 仍然是单 `activate(ctx)`，但 ctx 内的 handle 按 manifest 注入
- ✅ 注册时双向校验"声明 vs 实际注册"
- ✅ 回滚路径清晰（每个 handle 内部记录注册集合）

## 4. 影响

### 4.1 正向

- §3.1 manifest-first 硬约束进入类型系统：业务实现 `PluginManifestLoader` 时**没有路径**调用 RuntimeLoader
- 自定义 Discovery（PKI / 远端目录 / Workspace 索引）落地路径清晰：实现 `PluginManifestLoader`，复用 SDK 的 Validation / Trust / Slot / Dependency 全套校验
- 多 runtime 形态（静态 / dylib / cargo extension / wasm）平滑共存
- "插件越权注册" 在 capability handle 处被类型 / 运行期双重拦截
- Plugin deactivate 路径"声明即注销"，少一种"漏删"的 bug 形态

### 4.2 代价

- `PluginActivationContext` 字段形态变更属于破坏性 SPEC 调整（业务尚未实现，所以代码层无影响；文档层需修订）
- 新增两个 trait 与若干 Registration 子 trait；`crates/harness-plugin.md` SPEC 需扩充 §2 / §3 / §6
- `module-boundaries.md §6` 需补 `harness-plugin ↔ Loader` 检查项

### 4.3 兼容性

- 现有 `with_trusted_signer / with_signer_store`（来自 ADR-0014）保持不变；本 ADR 在它们之上加 `with_manifest_loader / with_runtime_loader`
- 现有 `PluginRegistry::discover / activate / deactivate / list_activated / snapshot / state` API 保持不变
- `Plugin` trait 签名不变（仍然是 `fn manifest / activate / deactivate`）；变化在 `PluginActivationContext` 字段

## 5. 落地清单（仅文档面）

| 项 | 责任文档 | 说明 |
|---|---|---|
| `PluginManifestLoader` / `PluginRuntimeLoader` trait | `crates/harness-plugin.md` 新增 §3.2 子节 | 类型层硬约束的语义说明 |
| 默认实现矩阵（File / CargoExtension / StaticLink / Dylib / Wasm） | `crates/harness-plugin.md` §3.2 | 含 cargo extension 的"冷启动冷退出"说明 |
| `PluginActivationContext` 重构为 capability handle 集合 | `crates/harness-plugin.md` §2.4 重写 | 含每个 handle trait 的窄接口签名 |
| `ToolRegistration` 等子 trait 的 `register / pending_declared` 语义 | `crates/harness-plugin.md` §2.4 / §6 | 与 manifest declared 的双向校验 |
| `RegistrationError::UndeclaredTool` 等错误形态 | `crates/harness-plugin.md` §13 | 与 §4 `RejectionReason` 对齐 |
| `PluginRegistryBuilder::with_manifest_loader / with_runtime_loader` | `crates/harness-plugin.md` §14 使用示例 | 默认装配规则与互斥说明 |
| `module-boundaries.md §6` 检查项补充 | `module-boundaries.md` | `harness-plugin ↔ Loader` 不得反向依赖 |
| `extensibility.md §11.3` 流程描述更新 | `extensibility.md` | 把 "Discovery / Validation / Runtime Load" 三段映射到 ManifestLoader / Validator / RuntimeLoader |

## 6. 参考证据

| Evidence ID | 来源 | 要点 |
|---|---|---|
| HER-033 | `reference-analysis/evidence-index.md` | manifest-first 与代码加载分离的设计实践 |
| OC-17 | `reference-analysis/evidence-index.md` | OpenClaw 的 manifest-first / lazy runtime 模式 |
| CC-38 | `reference-analysis/evidence-index.md` | Claude Code 的 plugin host 二分（声明 vs 实例化） |
| HER-035 | `reference-analysis/evidence-index.md` | 插件不得修改核心；本 ADR 把约束落到类型层 |
| ADR-006 | `adr/0006-plugin-trust-levels.md` | 信任域二分；本 ADR 在 trait 形态层补强 |
| ADR-0011 | `adr/0011-tool-capability-handle.md` | "Capability" 词在 Tool 侧的另一层语义；与本 ADR 不混淆 |
