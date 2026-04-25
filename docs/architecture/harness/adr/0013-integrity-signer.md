# ADR-0013 · `IntegritySigner` 默认实现与密钥治理

> 状态：Accepted
> 日期：2026-04-25
> 关联：ADR-0007（权限决策事件化）、`crates/harness-permission.md §6.1`、`security-trust.md §6.1 / §6.4`

## 1. 背景

ADR-0007 把权限决策事件化，`harness-permission §6.1` 又给 `FilePersistence`（`~/.octopus/permission-rules.json`）加了 HMAC 完整性签名 trait `IntegritySigner`，避免离线篡改"绕过 SDK 注入 `Decision::AllowPermanent`"。但当前文档只定义了 trait 形态与读写路径，留下三个未决问题：

1. **默认实现**：`DefaultHmacSigner` 用什么算法 / canonical bytes / 实现 crate？
2. **密钥治理**：dev / CI / 生产三种环境如何拿到 HMAC 密钥？密钥轮换的延迟迁移窗口多长？
3. **fail-closed 是否可关**：能否给一个 `--allow-unsigned-permission-rules` 或类似开关？

不回答这些问题，业务方在落地时会"先用空实现跑通"——失去签名保护反而以为已经合规。本 ADR 集中回答。

## 2. 决策

### 2.1 默认实现：`DefaultHmacSigner`

- 默认算法：**HMAC-SHA256**（由 `ring::hmac` 提供，FIPS 140-2 兼容、跨平台、无外部依赖）。
- 实现 crate：`harness-permission::integrity` 模块；`HarnessBuilder::with_decision_persistence` 在装配 `FilePersistence` 时若未注入 `IntegritySigner`，**自动**绑定 `DefaultHmacSigner` + `KeychainCredentialSource`（见 §2.3）。
- 业务层若提供自定义 `IntegritySigner`（如 `Vault HMAC API`、`Cloud KMS HMAC`），SDK 默认实现退让；`HarnessBuilder` 在装配期记录 `Event::SignerRegistered { algorithm, key_id, source: Custom }`（仅落 metric，不入 Journal，避免 PII）。

### 2.2 Canonical bytes 规则（与 §6.1 文件格式绑定）

写签前对**除 `signature` 字段外**的整张 JSON 做规范化，再喂 `IntegritySigner::sign`：

| 维度 | 规则 |
|---|---|
| 字段顺序 | 对象内 key 按 lex 升序；嵌套对象递归适用 |
| 编码 | UTF-8 / 无 BOM |
| 空白字符 | 删除所有 `\n` / `\t`；key-value 之间无空格 |
| 数值 | 整数原样；浮点保留 `serde_json` 默认输出（业务端禁止 NaN / Infinity，违例直接 `Persistence::SerializationRejected`） |
| 字符串 | 非 ASCII 字符统一转义为 `\uXXXX`（与 RFC 8785 / JCS 子集一致），保证不同 OS / locale 输出一致 |
| 算法版本字段 | `fingerprint_alg`、`schema_version` **必须**进入签名范围；`IntegrityAlgorithm` 也必须包含在 canonical 字节里，使"事后改算法"必然导致 `Mismatch` |

`harness-permission` 提供工具函数：

```rust
pub mod integrity {
    pub fn canonical_bytes(record: &serde_json::Value) -> Result<Vec<u8>, IntegrityError>;
    pub fn default_signer(creds: Arc<dyn CredentialSource>, tenant: TenantId)
        -> Arc<dyn IntegritySigner>;
}
```

### 2.3 密钥治理：三环境策略

| 环境 | `CredentialSource` 实现 | 行为 |
|---|---|---|
| **生产** | `KeychainCredentialSource`（macOS Keychain / Windows DPAPI / Linux libsecret） 或 `VaultCredentialSource`（HashiCorp Vault / Cloud KMS） | 启动期 `fetch` 失败即 `HarnessBuilder::build` 返回 `PermissionError::Persistence("missing integrity key")`，**不**降级到无签 |
| **CI** | `EphemeralCredentialSource`：每次进程启动随机生成 32-byte HMAC key，仅存内存 | CI 跑完即销毁；测试不应假设跨进程持久化 |
| **dev / 本地** | `EnvCredentialSource`：从 `OCTOPUS_PERMISSION_DEV_KEY=<base64>` 读取；缺失时 `HarnessBuilder` 在 `--profile dev` 下 fallback 到 `EphemeralCredentialSource`，并打 `tracing::warn!("dev mode: using ephemeral integrity key, persisted decisions will not survive restart")` | 当前 `cargo test` 路径默认走 ephemeral |

**禁止**直接把密钥写在 `~/.octopus/permission.toml` 等明文配置里——这等于把 HMAC 退化成"密码学伪装的明文校验和"。`harness-permission` 在反序列化配置时检查关键字段名，命中即拒绝。

### 2.4 密钥轮换与 `key_id` 命名

- 命名规范：`octopus-permission-<yyyy-mm>` 默认按月轮换；可选业务自定义 `<provider>-<purpose>-<rev>`（如 `vault-permission-v3`）。
- 轮换流程：
  1. 业务调 `CredentialSource::rotate(CredentialKey { ... key_label: new_key_id })` 注入新密钥；
  2. `FilePersistence` 接到新 `key_id` 后**写入路径**改用新 key 签名；
  3. **读路径**仍接受 30 天内的旧 `key_id`（默认窗口；可配 `IntegrityConfig::legacy_key_grace`）；超出窗口 `IntegrityError::UnknownKeyId`，按 `PermissionPersistenceTampered` 触发警报与备份；
  4. 业务可主动调 `FilePersistence::resign_all(...)` 完成"延迟迁移 → 全量重签"。

### 2.5 算法降级硬封禁

- HMAC-SHA256 是当前默认；HMAC-SHA512 / HMAC-Blake3 视为"更强"，可作为未来升级路径，但**降级被禁**：
  - 读到记录的 `IntegrityAlgorithm` 比配置的弱 → `IntegrityError::AlgorithmDowngrade`；
  - 升级（256 → 512）必须整体重签 + `schema_version` +1，与 ADR-0007 的 `PermissionSnapshot` 升版同步。

### 2.6 fail-closed 不可关

- **不**提供 `--allow-unsigned-permission-rules` 或 `IntegrityConfig::tolerate_missing_signature` 字段。
- 历史记录（首次启用 HMAC 之前的 `permission-rules.json`）通过一次性 `migrate_v2_to_v3` 命令重写后再读取；SDK **不**在主路径接受未签名记录。
- 任何"读到无签 / 验签失败 / 算法降级 / 未知 key_id"都触发同一审计事件 `PermissionPersistenceTampered`，配合 `<file>.tampered.<ts>` 备份。

## 3. 影响

### 3.1 正向

- 文件级离线篡改被关闭；攻击者必须同时拥有"目标 tenant 的 HMAC 密钥"才能伪造 `AllowPermanent`。
- 所有环境共用同一 `IntegritySigner` trait 与 fail-closed 路径，没有"开发环境跳过签名"的捷径。
- key_id 在记录中显式存储，运行期回看能立刻判断是哪批密钥签发；轮换审计直接落 `PermissionPersistenceTampered.key_id` 维度。

### 3.2 代价

- SDK 启动期对 OS keychain / Vault 增加一个可失败的依赖；离线分发的 desktop 应用必须在首次启动时引导"创建 / 注入" `OCTOPUS_PERMISSION_DEV_KEY`。
- `cargo test` 默认走 `EphemeralCredentialSource`，跨进程持久化的集成测试**必须**手动注入 `EnvCredentialSource` 以保留 key。
- 密钥轮换需要业务方建立"30 天延迟迁移 + 主动 `resign_all`" 操作 SOP。

### 3.3 兼容性

- 现有未签名的 `permission-rules.json` 走 `migrate_v2_to_v3` 一次性重写，签发当前默认 `key_id`；命令幂等。
- 第三方 `DecisionPersistence` 实现（业务自定义）必须满足 §6.1 的 `supports_integrity()` 返回 `true`，否则 `HarnessBuilder` 装配期 fail-closed 拒绝装载（与 `harness-permission §13` 反模式对齐）。

## 4. 落地清单

| 项 | 责任模块 | 说明 |
|---|---|---|
| `DefaultHmacSigner` 实现 | `harness-permission::integrity` | 基于 `ring::hmac::Key` + `verify`；`Drop` 时 `zeroize` 密钥 |
| `canonical_bytes` 工具 | `harness-permission::integrity` | RFC 8785 子集；测试覆盖 NaN/Infinity 拒绝、嵌套排序、Unicode 转义 |
| `KeychainCredentialSource` | `apps/desktop` 适配 | macOS `Security.framework` / Windows DPAPI / Linux `libsecret` |
| `EphemeralCredentialSource` | `harness-permission::testing` | 仅 `testing` feature；进程启动随机生成 |
| `migrate_v2_to_v3` CLI | `octopus-cli` | 老库一次性签发；写迁移日志到 `runtime/migrations/` |
| 装配期校验 | `harness-sdk::HarnessBuilder` | 未注入 signer 时自动挂默认；强制检查 `CredentialSource::fetch` 在 builder 阶段成功 |
| 文档同步 | `harness-permission §6.1` / `security-trust §6.4` | 在 §6.1 / §6.4 增加"详见 ADR-0013"反向引用 |

## 5. 参考证据

| Evidence ID | 来源 | 要点 |
|---|---|---|
| ADR-0007 | `docs/architecture/harness/adr/0007-permission-events.md` | 权限决策事件化奠定可审计基线 |
| HER-040 | `reference-analysis/evidence-index.md` | hermes-agent 持久化决策的疲态：缺失完整性保护 |
| OC-24 | `reference-analysis/evidence-index.md` | OpenClaw 把"沙箱通过"误当审批理由——本 ADR 巩固"权限信号必须由 SDK 严控的源" |
| RFC 8785 | https://datatracker.ietf.org/doc/html/rfc8785 | JCS 规范，本 ADR canonical bytes 取其严格子集 |
| FIPS 140-2 | NIST | HMAC-SHA256 默认选择的合规依据 |
