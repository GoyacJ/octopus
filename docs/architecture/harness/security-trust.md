# D9 · 安全与信任域

> 依赖 ADR：ADR-006（插件信任域二分）, ADR-007（权限决策事件化）
> 状态：Accepted · 任何安全边界变动必须走 ADR

## 1. 威胁模型

| 威胁类别 | 具体场景 | 对应防护 |
|---|---|---|
| **T1** 恶意工具调用 | 业务误用导致 `rm -rf /` / 泄漏密钥 | 权限 + 沙箱 + 危险命令库 |
| **T2** Prompt Injection | 用户输入或外部数据携带绕过指令 | Memory/MCP 栅栏 + sanitize |
| **T3** 供应链注入 | 恶意 Plugin / MCP Server 注册 | 信任域二分 + Manifest 签名 |
| **T4** 密钥泄漏 | 凭证出现在日志 / Trace / Event 中 | Redactor + `secret_ref` 机制 |
| **T5** 越权审批 | 低权限用户绕审批直接触发破坏性操作 | 审批事件化 + 沙箱正交 |
| **T6** 上下文污染 | Memory 中写入后门 / 长期记忆被劫持 | Memdir 威胁扫描 + 外部 Provider Slot 上限 1 |
| **T7** 侧信道 | 日志/指标泄漏业务数据 | Trace Redaction + 租户 ID 哈希 |

---

## 2. 信任域分层

```text
┌──────────────────────────────────────────────────────────┐
│ Trust Level 0 · SDK 内核（harness-* crate）              │
│   - 完整 Rust 代码 + 测试 + Review                        │
│   - 可直接操作所有 Registry、访问 Event Journal           │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ Trust Level 1 · Admin-Trusted Plugin / 业务基础设施       │
│   - 工作空间管理员显式安装                                │
│   - 可注册破坏性 Tool、exec Hook、全量 MCP Server        │
│   - 需签名（非本地）                                      │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ Trust Level 2 · User-Controlled Plugin                   │
│   - 用户个人 / ~/.octopus/plugins                        │
│   - 禁止破坏性 Tool、exec Hook、`strictPluginOnlyCustomization` |
│   - 默认权限模式 Plan / AcceptEdits                       │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ Trust Level 3 · External LLM / MCP Server / Memory       │
│   - 输出均视为**不可信**输入                              │
│   - 必须通过栅栏（<external-untrusted>）                  │
│   - sanitize_context 每轮剥离旧栅栏                      │
└──────────────────────────────────────────────────────────┘
┌──────────────────────────────────────────────────────────┐
│ Trust Level 4 · End User Input                           │
│   - 用户直接输入（消息 / 参数）                           │
│   - 可触发审批流，但不能直接跨越 Level 1/2 的能力限制     │
└──────────────────────────────────────────────────────────┘
```

---

## 3. 权限、沙箱、信任域的三维正交

| 维度 | 功能 | 单独能保证什么 |
|---|---|---|
| **Permission** | 决定 Tool 是否可调用 | 用户知情同意 |
| **Sandbox** | 决定命令在哪执行 | 爆炸半径控制 |
| **Trust Level** | 决定加载谁 | 供应链完整性 |

**关键**：三者**不可替代**。

- 即使 Sandbox 是 Docker 容器，`rm -rf /workspace` 仍需审批（反例 HER-041）
- 即使 Plugin 是 admin-trusted，`Bash` 工具仍需用户审批（fail-closed）
- 即使 Permission 通过，User-controlled Plugin 仍不能 `execute_code`

---

## 4. 危险命令检测（对齐 HER-039）

### 4.1 检测流程

```text
Tool::invoke("bash", { command: "rm -rf ~" })
    │
    ▼
[Pre-normalize] strip_ansi + NFKC
    │
    ▼
[Pattern Match] DANGEROUS_PATTERNS 正则库
    │
    ▼ (命中)
[PermissionCheck::DangerousCommand { pattern, severity }]
    │
    ▼
[Broker] 即使用户配置 `always_allow`，危险命令仍强制询问
```

### 4.2 默认模式库（简表）

`DangerousPatternLibrary` 提供三个工厂：`default_unix()` / `default_windows()` / `default_all()`（详见 `crates/harness-permission.md` §4）。

#### 4.2.1 Unix/bash/zsh（`default_unix`）

| 类别 | 示例 Pattern | 严重度 |
|---|---|---|
| 删除 | `rm -rf /`, `rm -rf ~`, `rm -rf $HOME` | Critical |
| 权限 | `chmod 777 /`, `chown -R root:root /` | High |
| 系统 | `systemctl stop`, `shutdown -h now` | Critical |
| Fork Bomb | `:(){ :\|:& };:` | Critical |
| 下载执行 | `curl ... \| sh`, `wget ... \| bash` | High |
| Heredoc Exec | `<<< 'sh'` 变种 | Medium |
| Git 破坏 | `git reset --hard origin/main`, `git push --force main` | High |
| Harness 自杀 | `pkill octopus`, `rm -rf $OCTOPUS_HOME` | Critical |

#### 4.2.2 Windows / PowerShell / cmd（`default_windows`）

| 类别 | 示例 Pattern | 严重度 |
|---|---|---|
| 删除 | `Remove-Item -Recurse -Force C:\`, `del /s /q C:\`, `rmdir /s /q C:\` | Critical |
| 系统 | `Stop-Computer`, `Restart-Computer`, `shutdown /s /t 0` | Critical |
| 磁盘 | `Format-Volume`, `diskpart` + `clean` | Critical |
| 下载执行 | `Invoke-WebRequest ... \| Invoke-Expression`, `iwr ... \| iex`, `(New-Object Net.WebClient).DownloadString(...) \| iex` | High |
| 安全策略 | `Set-ExecutionPolicy Unrestricted`, `Add-TrustedPublisher` | High |
| 反病毒 | `Set-MpPreference -DisableRealtimeMonitoring` | Critical |
| 持久化后门 | 写 `HKCU:\Software\...\Run`, `schtasks /create` 恶意任务 | High |

#### 4.2.3 平台自适应（`default_all`）

`RuleEngineBroker::with_platform_dangerous_library(ShellKind)` 根据运行时 `ShellKind` 自动选择；跨平台服务端建议使用 `default_all()`（合并两库，按 Pattern 自动匹配）。

完整库见 `crates/harness-permission.md` §4。

### 4.3 业务扩展

```rust
pub struct DangerousPatternRule {
    pub id: String,
    pub pattern: Regex,
    pub severity: Severity,
    pub description: String,
}

impl RuleEngineBroker {
    pub fn with_dangerous_pattern(mut self, rule: DangerousPatternRule) -> Self { ... }
}
```

业务层可追加（如检测业务专属的"生产库清理"命令）。

---

## 5. Prompt Injection 防护

### 5.1 外部数据栅栏（对齐 OC-34 / HER-017）

所有来自不可信来源的数据，用 XML 栅栏包裹：

```xml
<external-untrusted source="web-fetch" url="...">
[内容]
</external-untrusted>
```

- **来源标签**：`memory / mcp-tool / web-fetch / file-read / team-message`
- **sanitize_context** 每轮剥离上轮的栅栏（防止累积注入）
- 模型被明确指示：栅栏内容是**数据**，不是指令

### 5.2 Memory 威胁扫描（对齐 HER-019）

```rust
pub struct MemoryThreatScanner {
    patterns: Vec<ThreatPattern>,
}

pub struct ThreatPattern {
    pub id: String,
    pub regex: Regex,
    pub category: ThreatCategory,
}

pub enum ThreatCategory {
    PromptInjection,
    Exfiltration,
    Backdoor,
    Credential,
}
```

写 Memory 前扫描；命中则：

- 拒绝写入
- 发 `Event::MemoryThreatDetected { pattern_id, category, content_hash }`
- 不展示原始内容（只日志 hash）

**复用范围**：同一 `MemoryThreatScanner` 也用于 **Skill 加载期内容扫描**（详见 `crates/harness-skill.md §9`），以共享威胁模式库；事件以 `Event::SkillThreatDetected` 形态发出，字段与 `MemoryThreatDetected` 同构。

### 5.3 上下文消毒

```rust
pub struct ContextSanitizer {
    strip_old_fences: bool,
    strip_unknown_xml: bool,
    normalize_newlines: bool,
}
```

每轮 context assembly 前运行。

---

## 6. 凭证管理

### 6.1 `CredentialSource` Trait

```rust
#[async_trait]
pub trait CredentialSource: Send + Sync + 'static {
    async fn fetch(&self, key: CredentialKey) -> Result<CredentialValue>;
    async fn rotate(&self, key: CredentialKey) -> Result<()>;
}

/// 凭证键必须含 `tenant_id`，避免多租户共享 ban_list / cooldown
/// 详见 `crates/harness-model.md` §2.4
pub struct CredentialKey {
    pub tenant_id: TenantId,
    pub provider_id: String,
    pub key_label: String,
}

pub struct CredentialValue {
    secret: SecretString,
    metadata: CredentialMetadata,
}
```

- `secret` 字段使用 `secrecy::SecretString`（Debug trait 打印为 `[REDACTED]`）
- 业务层实现（如 1Password / HashiCorp Vault / AWS Secrets Manager）
- ban_list / cooldown 严格按 `CredentialKey` 三元组分桶；跨租户不污染（HER-048 修复）

### 6.2 配置文件中的引用

配置文件**不得**直接存密文。改用引用：

```json
{
  "providers": {
    "openai": {
      "api_key_ref": "vault:octopus/openai/prod/key"
    }
  }
}
```

### 6.3 Redactor

```rust
pub trait Redactor: Send + Sync + 'static {
    fn redact(&self, text: &str) -> String;
    fn register_pattern(&mut self, pattern: Regex);
}
```

用于：

- Trace span 属性
- Event body（选择性开启）
- 日志输出

默认模式库：OpenAI/Anthropic/Google API key 格式、JWT、常见云厂商密钥格式。

### 6.4 决策持久化的完整性保护（HMAC）

`harness-permission` 的 `FilePersistence` 把 `Decision::AllowSession` / `AllowPermanent` 写到 `~/.octopus/permission-rules.json`（按 `tenant × ExecFingerprint` 分桶）。**离线可写**资产存在 T1 / T5 类风险：能读到 `~/.octopus/` 的本地进程可在 SDK 不知情时追加 AllowPermanent 行，绕过审批。

**强制要求**（trait / 落盘格式见 `crates/harness-permission.md §6.1`；默认实现 / 密钥治理 / 轮换 / canonical bytes 见 **ADR-0013**）：

1. **HMAC-SHA256 签名**：每条记录写盘前用 `IntegritySigner::sign` 计算 MAC（`signature.algorithm` / `key_id` / `mac` 与 canonical 字节绑定）。
2. **密钥来源**：`IntegritySigner` 的密钥统一走 `CredentialSource::fetch(CredentialKey { tenant_id, provider_id: "octopus-permission", key_label: "<key_id>" })`，**不**直接在配置里暴露密文。
3. **验签 fail-closed**：读路径验证不通过时**不**返回 `ResolvedDecision`，把命中视为"无规则"回 Broker；同时写入 `Event::PermissionPersistenceTampered { reason, key_id, file_path_hash, fingerprint, at }`，原文件重命名为 `<file>.tampered.<ts>` 备份。
4. **算法降级硬封禁**：读到 `IntegrityAlgorithm` 较弱（如曾用 SHA-1 / MD5 的历史记录）时直接判 `AlgorithmDowngrade`，不接受兼容；迁移到更强算法必须整体重签（schema_version +1，与 ADR-007 的 `PermissionSnapshot` 升版同步）。
5. **不可禁用事件**：`PermissionPersistenceTampered` 与 `PermissionRequested` / `MemoryThreatDetected` 等列入"必记事件"（§8.1）；SIEM / 监控应订阅该事件触发告警。
6. **HarnessBuilder 装配校验**：`HarnessBuilder::with_decision_persistence` 在装配时检查实现是否声明 `supports_integrity()`；纯本地 `FilePersistence` 必须挂 `IntegritySigner`，**未挂则 fail-closed 拒绝构建**——不留"先跑起来再说"的灰区。
7. **JournalBasedPersistence 不重复签**：Journal 已经走 ADR-001 的 append-only / schema_version / merkle root 守护；HMAC 仅针对"离线可写"资产（`FilePersistence` + 业务自定义文件型实现）。

---

## 7. 租户隔离

### 7.1 多租户模型（对齐 OC-01 默认单租户）

> `TenantId` 是 `TypedUlid<TenantScope>` 的 type alias（定义在 `harness-contracts` §3.1），**不是** newtype struct。本节示例使用 `TenantId` 请视同 `TypedUlid<TenantScope>`。

```rust
pub struct TenantPolicy {
    pub id: TenantId,
    pub display_name: String,
    pub allowed_tools: Option<HashSet<String>>,
    pub allowed_providers: Option<HashSet<String>>,
    pub max_concurrent_sessions: Option<u32>,
    pub event_retention_days: Option<u32>,
}
```

默认使用 `TenantId::SINGLE`，业务层不感知多租户。

### 7.2 横切注入

- `EventStore::append(tenant, session, events)` 必须带 tenant
- `PermissionBroker::decide(tenant, request, ctx)` 必须带 tenant
- `TenantHarness` 封装：`harness.for_tenant(tenant)` 返回受限视图

### 7.2.1 Session ≠ 身份凭证（对齐 R-16 / OC-09）

**硬约束**：

- `SessionId` 是**路由键**（路由 turn 到正确的消息历史）与**上下文容器**（绑定 memdir / tool snapshot），**不是** auth token
- 不得用"持有 `SessionId` 即视为已鉴权"的模式；鉴权必须由独立机制承担：
  - MCP Server Adapter：`McpServerAuth::StaticBearer / OAuthValidator / MutualTls / Custom`
  - HTTP API（业务层实现）：Bearer / OAuth 2.0 / SSO / mTLS
  - IPC/Tauri：OS user + IPC 凭据
- `SessionId` 在响应中可见时不需要特殊脱敏（对照 access token 需要脱敏），但也**不要**通过日志广播
- `TypedUlid::timestamp_ms()` 可读出生成时间，多租户/高敏感场景若担心泄漏节奏特征，业务层可选用随机 128-bit ID 自行覆盖（SDK 不强制）

**典型反模式**：

```rust
// ❌ 反模式：MCP Server Adapter 只凭 SessionId 访问
handler.handle(Request {
    session_id: client_provided_session_id,
    // ... 无 auth header ...
}) -> ...;

// ✅ 正确：先鉴权，鉴权通过后再用 SessionId 路由
match auth.verify(request.headers).await? {
    Verified { tenant_id, scopes } => {
        // 在此处检查 session 归属该 tenant + scope 匹配
        let session = harness.for_tenant(tenant_id)
            .session_by_id(request.session_id)
            .ok_or(NotFound)?;
        handler.handle_in_session(session, request.payload).await
    }
    _ => Err(Unauthorized),
}
```

### 7.3 隔离保证

| 资源 | 隔离级别 | 说明 |
|---|---|---|
| Event Journal | 强隔离 | 不同 tenant 的 event 不能互读 |
| Projection | 强隔离 | 独立索引 |
| Memory | 强隔离 | Memdir 按 tenant 分目录 |
| Permission Rule | 强隔离 | tenant 粒度规则 |
| Tool Registry | 弱隔离 | 共享注册，按 `TenantPolicy::allowed_tools` 裁剪 |
| Model Credential Pool | 可选 | 配置 `per_tenant_credential_pool` 开启 |

---

## 8. 日志与审计

### 8.1 必记事件（不可禁用）

- 所有 `PermissionRequested` / `PermissionResolved`
- 所有 `PermissionPersistenceTampered`（HMAC 验签失败 / 算法降级 / 未知 key_id / 缺签名；详见 §6.4）
- 所有 `PermissionRequestSuppressed`（受 `DedupGate.suppression_max_events_per_window` 限速；详见 `permission-model.md §6.3` 与 `event-schema.md §3.6.3`）
- 所有 `ToolUseRequested` / `ToolUseCompleted` / `ToolUseFailed`
- 所有 `SubagentSpawned` / `SubagentAnnounced` / `SubagentPermissionForwarded` / `SubagentPermissionResolved`
- 所有 `PluginLoaded` / `PluginRejected` / `ManifestValidationFailed`
- 所有 `MemoryThreatDetected`

### 8.2 可禁用事件（观测增强）

- `AssistantDeltaProduced`（量大，默认记录但可关）
- `ToolUseCompleted.result`（大 blob，默认只记 hash）

### 8.3 审计查询

```rust
impl Harness {
    pub async fn audit_query(
        &self,
        filter: AuditFilter,
    ) -> Result<BoxStream<AuditRecord>>;
}
```

审计查询不能被 `User-controlled Plugin` 调用；仅 `Admin-Trusted Plugin` 与业务层可访问。

---

## 9. 信任链（Supply Chain）

### 9.1 SDK 自身

- 每个 crate `Cargo.toml` 使用固定版本（无 `*` / `^` 宽松约束）
- `cargo-deny` CI 规则：禁止未审核依赖
- `cargo-audit` 定时跑

### 9.2 Plugin 签名与命名空间（Admin-Trusted）

```rust
pub struct PluginManifest {
    pub manifest_schema_version: u32,
    pub name: String,
    pub version: Version,
    pub trust_level: TrustLevel,
    pub signature: Option<ManifestSignature>,
    pub capabilities: PluginCapabilities,
    pub dependencies: Vec<PluginDependency>,
}

pub struct ManifestSignature {
    pub algorithm: SignatureAlgorithm,
    pub signer: String,
    pub signature: Bytes,
    pub timestamp: DateTime<Utc>,
}
```

- Admin-Trusted Plugin **必须**有有效签名；signer 由 `TrustedSignerStore`（ADR-0014）治理
- User-Controlled Plugin 签名可选；签名通过**不会**让来源默认为 `UserControlled` 的插件升级到 `AdminTrusted`（避免签名伪造路径）
- **命名空间防护**：`PluginManifest.name` 必须满足 `harness-plugin §5.2` 语法（小写 ASCII / 同形字符拒绝 / 保留前缀 `octopus-` `harness-` `mcp-` 仅 `AdminTrusted` 可用），防止"伪装成官方插件"的钓鱼路径
- **Discovery 不执行代码**：Manifest-first 阶段只允许文件 IO 与签名校验，禁止 dlopen / 子进程 / 网络调用（详见 `harness-plugin §3.1`）；ADR-0015 把该约束抬到类型层（`PluginManifestLoader::enumerate` 的返回类型不允许产出 `Arc<dyn Plugin>`）
- **Signer 治理**（ADR-0014）：`TrustedSignerStore` 提供启用窗口（`activated_at` / `retired_at`）+ 撤销列表（`revoked_at`）；Manifest 必须由"签名时刻处于启用窗口"的 signer 签发；私钥泄露场景通过 `revoked_at = NOW` 让所有该 signer 签的 manifest 在下一次 Discovery 立即被拒（`Event::PluginRejected { reason: SignerRevoked }`）；已 Activated 插件**不**自动 deactivate，避免线上抖动
- **与 ADR-0013 IntegritySigner 完全独立**：本节 `TrustedSignerStore` 是上游分发链的非对称验签（Ed25519 / RsaPkcs1Sha256），凭证来源为 `SignerProvenance`（BuiltinOfficial / BuilderInjected / PkiEndpoint / PolicyFile）；ADR-0013 `IntegritySigner` 是本地权限决策文件的对称 HMAC，凭证来源为 `CredentialSource`（Keychain / Vault / Env / Ephemeral）。两者**不共享**实现也不共享配置；强行合并会让密钥泄露 blast radius 越界
- **Capability-Scoped ActivationContext**（ADR-0015）：`PluginActivationContext` 仅注入 manifest 已声明范围内的窄接口 handle（`ToolRegistration / HookRegistration / McpRegistration / SkillRegistration / MemoryProviderRegistration / CoordinatorStrategyRegistration`）；插件试图越权注册未声明能力会被类型 + 运行期双重拦截，落 `RegistrationError::Undeclared*` + `plugin_capability_registration_rejected_total` 指标
- **生命周期审计**（与 §8.1 单一来源）：
  - `Event::PluginLoaded`：状态转入 `Activated`
  - `Event::PluginRejected`：Manifest **已解析**，因业务规则（Trust / 命名 / 依赖 / Slot / 签名 / 撤销 / Admission）被拒；`reason` 取值见 `harness-plugin §4 RejectionReason`
  - `Event::ManifestValidationFailed`：Manifest 在 Discovery / 解析阶段就失败（YAML 错 / schema 不通过 / `manifest_schema_version` 不被支持 / cargo extension 元数据子命令冷输出 malformed），尚未构造出可信 `PluginId`；事件以 `manifest_origin` 标识来源，`failure` 字段判别五种子情况

### 9.3 MCP Server

- stdio / http-local：视为与 harness 同信任域
- http-remote：必须配 `auth`（Bearer Token / OAuth / mTLS）
- 所有 MCP 工具名在 Registry 走 canonical 形态 `mcp__<server_id>__<tool>`（见 `harness-contracts §3.4.2`），避免与内建重名并兼容 LLM 工具命名字符集

---

## 9.X 配置严格校验与 last-known-good（对齐 OC-31）

`HarnessOptions` 及所有 `*Options` / `*Spec` 反序列化采用严格模式：

### 9.X.1 严格校验

```rust
pub struct OptionsParseMode {
    pub reject_unknown_fields: bool,   // 默认 true
    pub reject_duplicate_fields: bool, // 默认 true
    pub deny_null_for_required: bool,  // 默认 true
}

impl HarnessOptions {
    pub fn parse(raw: &str, mode: OptionsParseMode)
        -> Result<Self, ConfigError>;
}

pub enum ConfigError {
    UnknownField { path: String, field: String },
    DuplicateField { path: String, field: String },
    InvalidValue { path: String, reason: String },
    SchemaViolation { details: String },
    LastKnownGoodMissing,
}
```

规则：

1. **未知字段拒启动**（使用 `#[serde(deny_unknown_fields)]`），防止配置漂移
2. **类型错误拒启动**（如 `number` 给了 `string` 值）
3. 错误信息必须定位到 JSON Path，便于业务排查

### 9.X.2 Last-Known-Good 回退

业务层（如 `octopus-server` / `octopus-cli`）启动时按以下顺序尝试：

```rust
pub struct LastKnownGoodConfig {
    pub primary_path: PathBuf,
    pub backup_path: PathBuf,          // 建议 `<primary>.lkg`
    pub max_rollback_attempts: u32,    // 默认 1
}

async fn load_with_fallback(cfg: LastKnownGoodConfig)
    -> Result<HarnessOptions, ConfigError>
{
    match HarnessOptions::parse_file(&cfg.primary_path, OptionsParseMode::default()).await {
        Ok(opts) => {
            // 成功 → 复制为 last-known-good
            fs::copy(&cfg.primary_path, &cfg.backup_path).await.ok();
            Ok(opts)
        }
        Err(primary_err) => {
            tracing::warn!(?primary_err, "primary config invalid, attempting LKG");
            if cfg.backup_path.exists() {
                let opts = HarnessOptions::parse_file(&cfg.backup_path, OptionsParseMode::default()).await?;
                // 发 Event::ConfigRolledBackToLKG 记录
                Ok(opts)
            } else {
                Err(ConfigError::LastKnownGoodMissing)
            }
        }
    }
}
```

### 9.X.3 密钥分离

- 业务设置（`*.options.yaml` / `*.options.json`）**禁止**包含明文密钥
- 密钥放 `~/.octopus/credentials/*.yaml`（权限 `0600`）或通过 `CredentialSource` trait（见 §6.1）注入
- SDK 在反序列化 Options 时检测到疑似密钥字段（正则匹配：`api_key` / `token` / `secret` / `password` 等）**仅当字符串值非 `ref:xxx` / `env:xxx` / `vault:xxx` 形式时**记 Warning；不阻塞启动但建议重构

## 10. 与 Octopus 仓库治理对齐

| 治理规则 | 本 SDK 映射 |
|---|---|
| `config/*` 文件优先于数据库 | `CredentialSource::FileBased` 默认读 `~/.octopus/credentials/` |
| 密钥不应进 `main.db` | `SqliteEventStore` 默认 `Redactor` 默认开启 |
| `runtime/events/*.jsonl` 审计流 | `JsonlEventStore` 落盘 |
| `data/` vs `runtime/` vs `logs/` | SDK 尊重业务层传入的路径，不硬编码 |

---

## 10.X Tool Search 安全考量（ADR-009）

Deferred Tool Loading 引入"模型按需解锁工具 schema"的机制，同时也扩大了攻击面。本节罗列关键风险与缓解。

### 10.X.1 风险矩阵

| 风险 | 场景 | 缓解 |
|---|---|---|
| **权限规避** | 攻击者通过 prompt injection 诱导模型 `tool_search` 把高危工具 materialize 进来再调用 | 权限决策链路不变：materialize 只让 schema 可见，实际 invoke 仍走 `PermissionBroker`（ADR-007）；`DeferPolicy` 不替代 `PermissionMode` |
| **Tool schema 投毒** | 恶意 MCP server 在 `tools/list_changed` 推送欺骗性 `search_hint` 以提升命中率 | `DefaultScorer` 的 `search_hint` 权重（4）不超过 `name_part_exact`（10/12）；`PreToolSearch` Hook 可拦截；MCP 工具 `trust_level` 可在 `McpServerSpec.trust_level` 声明 |
| **Scoring Oracle** | 攻击者通过观察 `ToolSearchQueried.scored` 反推其他工具的 hint，进行信息萃取 | `ToolSearchQueriedEvent` 默认不落盘 `scored` 明细；可观测性导出走脱敏 Redactor；仅调试模式保留 |
| **Deferred Pool 膨胀 DoS** | 租户通过 MCP 注册数千个工具耗尽 coalescer / scorer 内存 | `ToolRegistryBuilder` 支持 `max_deferred_pool_size`（默认 1024）；超限时拒绝注册并 `Event::EngineFailed` |
| **Coalesce 窗口滥用** | 恶意连续发起 `tool_search` 让其他 session 的 inline reload 被延迟（跨 session 干扰） | Coalescer 按 `(session_id, run_id)` 独立；不同 session 不共享窗口（见 `harness-tool-search.md` §3.4） |
| **ForceDefer 规避** | 业务端无意中把 `ToolSearchMode::Disabled`（为审计）与 `DeferPolicy::ForceDefer`（为隔离）混搭 | `ToolRegistry::register` 在 `Disabled` 模式下对 `ForceDefer` 工具返回 `RegistrationError::DeferralRequired`（硬失败，非警告） |

### 10.X.2 Kill Switch

两级 kill switch，供紧急场景使用：

1. **全局 kill switch**（`HarnessBuilder::disable_tool_search()`）—— 构造期禁用整个 Tool Search 子系统：
   - `BackendSelector` 返回 `ToolLoadingBackend::Noop`（materialize 直接失败）
   - 所有 `DeferPolicy::AutoDefer` 工具降级为 `AlwaysLoad`（全量注入）
   - `DeferPolicy::ForceDefer` 工具注册失败
   - 等价于 `ToolSearchMode::Disabled`，但作用于整个 Harness（不限单 session）
2. **租户 / Session 级 kill switch**（`SessionOptions.tool_search = ToolSearchMode::Disabled`）—— 细粒度禁用，仅影响当前 session
3. **Feature flag 级 kill switch**（`feature-flags.md` · `tool-search` feature）—— 构建期剔除整个 crate

### 10.X.3 审计保障

以下事件必记且不可禁用（ADR-009 §2.11）：

- `ToolDeferredPoolChanged`（谁在什么时候改了 deferred 池）
- `ToolSchemaMaterialized`（哪些工具被 materialize、走哪条 backend、cache 影响）

`ToolSearchQueried` 属于可禁用事件（观测增强），但 `matched` 列表始终落盘（因与后续 `ToolSchemaMaterialized` 配对追溯）。

### 10.X.4 Hook 拦截路径

`PreToolSearch` Hook 具有 `Block` 能力（见 `extensibility.md` §5.3），用于：

- 审计特定查询（比如搜索 "credential" / "secret" 关键字）
- 在合规场景拦截外源性搜索（如模型尝试通过 `select:` 解锁被策略禁用的工具）

---

## 10.Y `execute_code` 安全考量（ADR-0016）

> 元工具 `execute_code` 引入了一个**新类别**的攻击面：
> 一段受限脚本能在单次推理里发起多步工具调用。本节列出对应的纵深防御。

### 10.Y.1 风险矩阵

| 风险 | 缓解 |
|---|---|
| 模型注入恶意脚本 | `DecisionScope::ExecuteCodeScript { script_hash }` 进权限决策，`PermissionMode::Default` 默认询问；`script_hash` 同 Run 复用走 `DedupGate`；`Severity::High` |
| 脚本扩散到子代理 | `harness-subagent §2.5` 默认 blocklist + `caller_chain` 运行期校验双闸门 |
| 嵌入工具被滥用为写 / 网络通道 | `EmbeddedToolWhitelist` 默认仅 7 个 read-only built-in；扩展受 `is_destructive == false && trust ∈ {Builtin, Plugin{AdminTrusted}}` 双闸门 |
| 解释器拒绝服务（无界循环 / 内存爆炸） | `MiniLuaCodeSandbox` instructions / call_depth / wall_clock / memory 配额 |
| 脚本反射调用自身 | `EmbeddedRefusedReason::SelfReentrant` 直接 deny；事件可审计 |
| 嵌入调用绕过 Hook / Permission | 内嵌每次调用复用 `ToolOrchestrator::single_use_pipeline`，5 个介入点全保留 |

### 10.Y.2 Kill Switch（三层）

1. **构建期 Kill Switch**（`feature-flags.md programmatic-tool-calling = off`）—— SDK 不编译入解释器与元工具
2. **租户 / 装配期 Kill Switch**（`HarnessBuilder::without_tool("execute_code")`）—— 已编译 SDK 仍可在装配期剔除
3. **运行期降级**（`SessionOptions.tool_search.embedded_tools_disabled = true`，业务自定义）—— 已注入但拒绝执行

### 10.Y.3 审计保障（不可禁用事件）

- `Event::ExecuteCodeStepInvoked`（每一次嵌入调用）
- `Event::ExecuteCodeWhitelistExtended`（白名单扩展）
- `Event::ToolUseDenied { reason: SubagentBlocked }`（Subagent 层拦截）

`script` 原文进入 `Event::ToolUseRequested.input.script`，与 `ToolResultOffloaded`
落盘语义一致，超 `ToolDescriptor.budget` 时由 BlobStore 持有。

---

## 10.Z Steering Queue 安全考量（ADR-0017）

### 10.Z.1 风险矩阵

| 风险 | 缓解 |
|---|---|
| 第三方插件灌入"指令注入" | `SteeringSource::Plugin` 必须 `AdminTrusted` 且声明 `capabilities.steering = true`；user-controlled 插件 fail-closed |
| 队列被外源刷爆 | `SteeringPolicy.capacity / dedup_window`；默认 `DropOldest` 丢失最早，写出 `Event::SteeringMessageDropped { reason: Capacity }` |
| 跨 Run / RunEnded 后残留 | TTL 自然过期；`SessionEnded` 强制清空；事件全部审计 |
| 软引导被用于绕过 PermissionBroker | 软引导**不**触发 tool 调用，仅修改下一轮 user 消息；模型仍需走 `tool_use` 才能触发权限链 |
| 软引导拆 Prompt Cache | drain 仅发生在"下一轮 user 消息"段，与 ADR-0003 锁定字段不重叠 |

### 10.Z.2 Kill Switch（两层）

1. **构建期 Kill Switch**（`feature-flags.md steering-queue = off`）—— `push_steering` 返回 `SessionError::FeatureDisabled`
2. **装配期容量 0**（`SteeringPolicy::capacity = 0`）—— API 仍存在但永远 DropNewest

### 10.Z.3 审计保障（不可禁用事件）

- `Event::SteeringMessageQueued / Applied / Dropped` 三件套全部进入 EventStore
- 完整 `body` 超 `inline_threshold`（默认 8 KiB）由 BlobStore 落盘，事件仅留 `body_hash + body_blob`

---

## 11. 默认安全态

| 维度 | 默认值 |
|---|---|
| `PermissionMode` | `Default`（规则 + 询问） |
| `SandboxBackend` | `LocalSandbox`（本地受限 cwd） |
| `tool.is_destructive` | `true` |
| `tool.is_concurrency_safe` | `false` |
| `tool.defer_policy` | `AlwaysLoad`（Fail-closed；内建工具保持可见） |
| `plugin.trust_level` | `user-controlled` |
| `memory.external_provider_count` | 0（Memdir 永远存在） |
| `redactor.enabled` | `true` |
| `dangerous_pattern_library.enabled` | `true` |
| `session.tool_search` | `Auto { ratio: 0.10, min_absolute_tokens: 4_000 }` |
| `tool_search kill_switch` | 关闭（即 Tool Search 默认启用） |
| `programmatic_tool_calling` feature | **off**（M1 期；ADR-0016 §4.3） |
| `steering_queue` feature | **on**；`SteeringPolicy::default()` = `capacity 8 / ttl 60s / DropOldest / dedup 1500ms` |
| `EmbeddedToolWhitelist` | 默认 7 个 read-only built-in；业务扩展需 AdminTrusted + `is_destructive == false` |
| `SteeringSource::Plugin` | 默认 deny；需 manifest `capabilities.steering = true` 且 `AdminTrusted` |

---

## 12. 安全清单（SDK 发布前）

- [ ] 所有公开 API 有 `#[non_exhaustive]` 标注（允许未来扩展）
- [ ] 所有凭证字段使用 `SecretString`
- [ ] `Debug` / `Display` 实现不泄漏密文
- [ ] 默认配置通过 `security audit` 指令
- [ ] 至少一个 CI 任务跑 `cargo-audit`
- [ ] 所有 `rg` 命中 "password" / "secret" 字样的字段有 redact 处理

---

## 13. 紧急响应

### 13.1 漏洞披露

- 业务层发现漏洞 → 通知 `security@octopus` → SDK 团队触发热补丁流程
- 热补丁发布包含：受影响 crate `PATCH` 版本 + CVE 编号 + 迁移指引

### 13.2 密钥泄漏

- `CredentialSource::rotate` 强制轮换
- `EventStore::redact_historical(filter)` 对历史 Event body 做后置脱敏（⚠ 不是物理删除）

---

## 14. 索引

- **权限规则** → `permission-model.md`
- **Plugin 信任域** → ADR-006 + `crates/harness-plugin.md`
- **审批事件化** → ADR-007 + `crates/harness-permission.md`
- **Memory 威胁扫描** → `crates/harness-memory.md`
- **Trace Redaction** → `crates/harness-observability.md`
- **Tool Search 安全** → ADR-009 + `crates/harness-tool-search.md` + `extensibility.md` §5.3（`PreToolSearch` Hook）
