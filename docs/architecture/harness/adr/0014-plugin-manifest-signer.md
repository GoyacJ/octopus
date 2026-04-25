# ADR-0014 · Plugin Manifest Signer 治理（轮换 / 撤销 / 与 ADR-0013 的关系）

> 状态：Accepted
> 日期：2026-04-25
> 决策者：架构组
> 关联：ADR-006（插件信任域二分）、ADR-013（`IntegritySigner` 默认实现，权限持久化 HMAC）、`crates/harness-plugin.md`、`security-trust.md §9.2`

## 1. 背景与问题

ADR-006 把 Plugin 强制二分为 `AdminTrusted / UserControlled`，并要求 `AdminTrusted` 必须有效签名；`PluginManifest.signature` 字段已声明算法 / signer / payload。`harness-plugin.md` 也描述了 Discovery 期签名校验。但落到运维侧仍有 5 个未决问题：

1. **Signer 注册表**：`trusted_signers: Vec<PublicKey>` 是单维列表，没有"何时启用 / 何时停用 / 由谁配置 / 多 tenant 是否独立"的约束。
2. **轮换流程**：当某官方 signer 私钥需要换发时，旧 signer 签的 manifest 是否仍然可被加载？兼容窗口多长？谁来标记？
3. **撤销路径**：私钥泄露 / signer 团队被取消授权时，如何**快速**让所有部署拒绝该 signer 之前签发的 manifest？
4. **与 ADR-013 的边界**：ADR-013 已就权限持久化 HMAC 拍板（`DefaultHmacSigner` + `IntegritySigner` trait + `KeychainCredentialSource` + 算法降级硬封禁）。Plugin Manifest Signer 是否复用同一套体系？
5. **Manifest signature timestamp 的语义**：`ManifestSignature.timestamp` 与 signer 启用窗口、Manifest 校验当下时刻三者的因果关系。

不固化这些点，业务方落地时只能"先把 trusted_signers 写死，私钥永远不换"或者"出了事再说"——既不可审计也不可恢复。

## 2. 决策

### 2.1 与 ADR-0013 完全独立

| 维度 | ADR-0013（`IntegritySigner`） | ADR-0014（`PluginManifestSigner`） |
|---|---|---|
| 用途 | 防止本地权限决策文件被离线篡改 | 校验上游分发的 plugin manifest 是否来自可信签发方 |
| 信任方向 | **自签自验**：本机生成 HMAC 密钥，本机校验 | **第三方签发**：由 Octopus 官方 / 企业 PKI 签发，本机仅校验 |
| 算法形态 | 对称（HMAC-SHA256 默认） | 非对称（Ed25519 默认；RSA-PKCS1-SHA256 兼容） |
| 凭证来源 | `CredentialSource`（Keychain / Vault / Env / Ephemeral） | `TrustedSignerStore`（静态文件 / 远端 PKI / Plugin 本身签发） |
| 轮换策略 | "30 天内旧 key_id" 的 grace window | 启用窗口 + 撤销列表（CRL-like） + Manifest 时间戳约束 |
| 失败动作 | 记 `PermissionPersistenceTampered` + 备份原文件 | 记 `Event::PluginRejected { reason: SignatureInvalid }` 或 `UnknownSigner` |

> 两者**不共享**实现也不共享配置：硬把它们合二为一会让"本地权限"与"上游供应链"两个**正交**问题混淆，导致密钥泄露的 blast radius 越界。

### 2.2 `TrustedSignerStore` trait

```rust
#[async_trait]
pub trait TrustedSignerStore: Send + Sync + 'static {
    /// 列出当前生效的 signer 集合。
    /// 实现方负责处理"启用窗口 / 撤销列表"内部状态机；
    /// 调用方拿到的视图必然是"此时此刻可用"的。
    async fn list_active(&self) -> Result<Vec<TrustedSigner>, SignerStoreError>;

    /// 按 signer_id 精确取一个；返回 `None` 表示"从未注册过"或"已过期 + 超出查询窗口"。
    async fn get(&self, id: &SignerId) -> Result<Option<TrustedSigner>, SignerStoreError>;

    /// 是否处于撤销列表（用于"查 Manifest 签名时，即使在 list_active 中找到也得交叉确认"）。
    async fn is_revoked(&self, id: &SignerId, at: DateTime<Utc>) -> Result<bool, SignerStoreError>;

    /// 订阅 signer 集合变化，让 PluginRegistry 能及时刷新。
    fn watch(&self) -> BoxStream<'static, SignerStoreEvent>;
}

pub struct TrustedSigner {
    pub id: SignerId,
    pub algorithm: SignatureAlgorithm,
    pub public_key: Bytes,
    /// 启用窗口；不在 [activated_at, retired_at] 之间签的 manifest 一律拒绝
    pub activated_at: DateTime<Utc>,
    pub retired_at: Option<DateTime<Utc>>,
    /// 撤销时间；存在且 <= 当前时间 → `is_revoked` 返回 true
    pub revoked_at: Option<DateTime<Utc>>,
    pub provenance: SignerProvenance,
}

pub struct SignerId(pub String);

#[non_exhaustive]
pub enum SignerProvenance {
    /// SDK 编译期内嵌的 octopus-official key
    BuiltinOfficial,
    /// 业务通过 `PluginRegistryBuilder::with_trusted_signer` 显式注入
    BuilderInjected,
    /// 来自 PKI 端点（HTTPS / 本地证书目录），含端点元数据
    PkiEndpoint { endpoint: Url },
    /// 来自企业策略文件（如 `data/plugin-signers.toml`）
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

> `SignerId` 是自由文本（建议命名 `<provider>-<purpose>-<rev>`，如 `octopus-official-2026-04`），与 `ManifestSignature.signer` 字段直接配对；多版本同一 `provider` 的 signer 共存通过不同 `SignerId` 区分。

### 2.3 默认实现：`StaticTrustedSignerStore`

```rust
pub struct StaticTrustedSignerStoreBuilder {
    /// 编译期内嵌的官方 signer（作为 fallback / dev 兜底）
    builtin_official: Option<TrustedSigner>,
    /// 业务通过 builder 显式注入的 signer 集合
    builder_injected: Vec<TrustedSigner>,
    /// 文件型 policy（可热更）
    policy_file: Option<PolicyFileSource>,
    /// 撤销列表
    revocation: RevocationConfig,
}
```

- **默认绑定**：`PluginRegistry::builder().with_trusted_signer(...)` 现有 API 保持不变；底层装配 `StaticTrustedSignerStore`，把传入的公钥包装为 `TrustedSigner { id: "user-injected-<idx>", provenance: BuilderInjected, .. }`
- **企业模式**：`with_signer_store(Arc<dyn TrustedSignerStore>)` 是新接口，业务可注入自定义实现（PKI / 远端目录 / Vault）；与 `with_trusted_signer` 互斥（先注入 store 后再调用 `with_trusted_signer` 直接拒绝构建）
- **撤销列表**：`RevocationConfig` 接受三种来源：本地文件（CRL 形态）、HTTP 端点（按 `refresh_interval` 拉取）、自定义 trait（业务方实现）

### 2.4 Manifest 验签流程（`harness-plugin §4` 落地点）

```text
1. 解析 ManifestSignature { algorithm, signer, signature, timestamp }
2. signer_store.get(&signer)
   → None                         → PluginRejected { reason: UnknownSigner }
   → Some(s) 但 s.algorithm 不匹配  → PluginRejected { reason: SignatureInvalid("algorithm mismatch") }
3. 检查 signer 启用窗口 + 撤销
   - timestamp ∉ [s.activated_at, s.retired_at.unwrap_or(MAX)]
                                  → PluginRejected { reason: SignatureInvalid("timestamp out of activation window") }
   - signer_store.is_revoked(&signer, NOW)
                                  → PluginRejected { reason: SignerRevoked { signer_id, revoked_at } }
4. 验签 payload（Manifest 去掉 signature 字段后 canonicalize；规则与 ADR-0013 §2.2 同款 RFC 8785 子集）
   - 失败                          → PluginRejected { reason: SignatureInvalid }
5. 通过；进入下游校验（Trust 与来源匹配 / 命名空间 / 依赖图）
```

> **签发时间约束**：Manifest 必须由"签名时刻处于启用窗口的 signer"签发；这阻断了"先用旧 signer 签，再把 signer retire"导致 retire 后仍能伪造的攻击路径。

### 2.5 轮换语义

| 场景 | 流程 |
|---|---|
| **新增 signer** | 业务方通过 `with_trusted_signer` 注入或更新 PolicyFile；存量 manifest 不受影响（仍由原 signer 验证） |
| **正常退役** | 给 signer 设 `retired_at`；`retired_at` 之前签的 manifest 仍可加载，之后签的拒绝；旧 manifest 不需要重签 |
| **私钥泄露 / 紧急撤销** | 给 signer 设 `revoked_at = NOW`；**所有**该 signer 签的 manifest 立即拒绝（不论签名时间），需要业务方提供新 signer 重签 |
| **算法升级** | 老 signer 保持启用 + 新 signer 用更强算法并行；待所有新 manifest 切到新 signer 后再 retire 老的 |

`SignerStoreEvent` 让 `PluginRegistry` 在变更发生时刷新内部缓存；已 `Activated` 的插件**不**自动 deactivate（避免线上抖动），但下次 Discovery 会按新规则重判。

### 2.6 fail-closed 不可关

- **不**提供 `--allow-unsigned-admin-trusted-plugins` 或 `tolerate_unknown_signer` 配置
- 没有任何"以警告代替拒绝"的开关；`AdminTrusted` 缺签 / 验签失败 / signer 撤销 / 签发时间超窗口都是终态拒绝
- 历史 manifest（首次启用此机制之前生成）由业务通过 `octopus plugin sign --signer <id>` 一次性补签后再分发；SDK **不**接受未签名的 `AdminTrusted` 来源

### 2.7 与 ADR-006 的关系

ADR-006 §2.4 已声明"`AdminTrusted` 必须签名"；本 ADR 把"签名"二字从字段层（`Option<ManifestSignature>`）提升到治理层（`TrustedSignerStore` + 启用窗口 + 撤销）。**ADR-006 文本不需要修订**——它的判定结论"User-Controlled 不会因为带签名而升级"在本 ADR 仍然成立：因为 `TrustedSignerStore` 只回答"signer 是否可信"，不回答"trust_level 是否升级"，后者由 `DiscoverySource` 决定。

### 2.8 Tenant 隔离

`TrustedSignerStore::list_active` 不带 tenant 参数：默认假设 signer 集合是**进程级**信任根（与 ADR-013 的"密钥按 tenant 走"形成对比）。多 tenant 场景需要不同 signer 的业务方，必须实现自定义 `TrustedSignerStore`（按 `Harness::for_tenant` 入口分流），而不是修改 SDK 默认实现——避免 SDK 把"所有 tenant 共享 signer"的常见诉求与"严苛多租户"的边缘场景混在一个 trait 里。

## 3. 替代方案

### 3.1 复用 ADR-0013 的 `IntegritySigner` trait

- ❌ 一个 trait 同时承担对称（HMAC）与非对称（Ed25519）签名，类型与凭证模型都不一致
- ❌ `CredentialSource` 拉的是**密钥**而 PluginManifestSigner 拉的是**公钥**（语义反向）
- ❌ 撤销列表 / 启用窗口对 ADR-0013 没意义，反过来 ADR-0013 的 grace window 对 manifest 也不适用

### 3.2 把 signer 列表写死在 `Cargo.toml` feature flag

- ❌ 无法运行期热轮换；私钥泄露需要重新发版
- ❌ 企业自建 signer 必须 fork SDK

### 3.3 不做撤销，只靠"等老 signer 自然过期"

- ❌ 私钥泄露的窗口太长；ADR 必须给"立即生效"的撤销路径
- ❌ 不符合常见 PKI 最佳实践（CRL/OCSP）

## 4. 影响

### 4.1 正向

- 私钥泄露场景从"全网部署各自下版本回滚"压缩到"业务方一次性更新撤销列表"
- Signer 启用窗口让"先签后 retire"型攻击在签发时间维度被堵死
- 与 ADR-0013 完全独立，避免"权限持久化与插件供应链共用同一密钥"的失误
- `TrustedSignerStore` 是开放 trait，企业 PKI 接入不需要 SDK 改动

### 4.2 代价

- `PluginRegistry::builder().with_trusted_signer(...)` 增量保持兼容；新 `with_signer_store` 与之互斥（builder 装配期校验）
- 引入"signer activation window"对 manifest 签名时间字段的解读更严格——但这本就是 `ManifestSignature.timestamp` 的设计目标
- 业务自建 PKI 需要实现 `watch()`，否则撤销不能立即生效；SDK 提供 `PollingSignerStore` 适配器（按 `refresh_interval` 拉取）

### 4.3 兼容性

- `ManifestSignature` 字段不变；`SignerId` 类型仅是 `String` newtype，不破坏现有 manifest YAML
- 现有 `with_trusted_signer(public_key)` API 保留，新 API `with_signer_store(store)` 平行存在
- ADR-006 §2.4 的"`AdminTrusted` 必签"结论不变

## 5. 落地清单（仅文档面）

| 项 | 责任文档 | 说明 |
|---|---|---|
| `TrustedSignerStore` trait + `TrustedSigner` 结构 | `crates/harness-plugin.md` 新增 §4.1 子节 | 验签与 §4 校验流程合流 |
| `RejectionReason::SignerRevoked` 新增子情况 | `crates/harness-plugin.md` §4 `RejectionReason` | 与 contracts §3.3 `PluginRejected` payload 对齐（不新增 Event variant） |
| Builder API：`with_signer_store / with_trusted_signer` 互斥规则 | `crates/harness-plugin.md` §14 使用示例 | 避免业务方混用 |
| `SignerStoreEvent` 与 PluginRegistry 刷新语义 | `crates/harness-plugin.md` §7 状态机的"重判"段落 | 撤销不强制 deactivate，但下次 Discovery 重判 |
| 与 ADR-0013 边界声明 | `security-trust.md §9.2` | 明确两者**不共享**密钥与 trait |
| signer 命名约定 | `crates/harness-plugin.md` §5（命名空间治理）末尾添加 signer 命名约定子节 | `<provider>-<purpose>-<rev>` |

## 6. 参考证据

| Evidence ID | 来源 | 要点 |
|---|---|---|
| ADR-006 | `adr/0006-plugin-trust-levels.md` | 信任域二分 + AdminTrusted 必签的上层约束 |
| ADR-0013 | `adr/0013-integrity-signer.md` | 权限持久化 HMAC 治理；本 ADR 与之划清界限 |
| CC-27 | `reference-analysis/evidence-index.md` | Claude Code 的 admin/user 二分 + signer 概念 |
| OC-18 | `reference-analysis/evidence-index.md` | OpenClaw 单一信任域反例（"插件 = Gateway"） |
| RFC 8785 | https://datatracker.ietf.org/doc/html/rfc8785 | Manifest canonical bytes 的算法基础（与 ADR-0013 同款子集） |
| RFC 5280 §5 | https://datatracker.ietf.org/doc/html/rfc5280#section-5 | CRL 撤销列表语义参考 |
