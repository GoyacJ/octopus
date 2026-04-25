# ADR-006 · 插件信任域二分（Admin-Trusted vs User-Controlled）

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：`harness-plugin` / `harness-tool`（注册规则） / `harness-hook`（exec hook 许可） / `harness-mcp`（远端 server 授权）

## 1. 背景与问题

Plugin 是批量注册 Tool / Skill / Hook / MCP Server 的机制。参考项目的信任处理对比：

| 项目 | 信任模型 | 问题 |
|---|---|---|
| Hermes | 插件不得修改核心文件（HER-035），但加载的 Plugin 全权限 | 单一信任域，难以区分"企业已审核"与"用户临时" |
| OpenClaw | 原生插件与 Gateway **同信任域**（OC-18） | 任何插件都能修改 Gateway 状态，权限面过大 |
| Claude Code | 区分 admin-trusted vs user-controlled（CC-27） | 最成熟，对齐 |

**问题**：如果所有 Plugin 一视同仁，则：

- 用户无法安全安装来自社区的 Plugin（会获得完整权限）
- 企业审核的可信 Plugin 也被限制（无法注册破坏性 Tool）
- 供应链攻击（Plugin 含恶意代码）影响面 = 整个 SDK

## 2. 决策

**Plugin 强制二分信任域：Admin-Trusted 与 User-Controlled**。

### 2.1 信任级别

```rust
pub enum TrustLevel {
    AdminTrusted,
    UserControlled,
}
```

### 2.2 能力矩阵

| 能力 | Admin-Trusted | User-Controlled |
|---|---|---|
| 注册 Tool（非破坏性） | ✅ | ✅ |
| 注册 Tool（破坏性，`is_destructive=true`） | ✅ | ❌ |
| 注册 Exec Hook（`HookExecSpec`） | ✅ | ❌ |
| 注册 HTTP Hook | ✅ | ✅（需 `HookHttpSecurityPolicy.allowlist` 非空 + `ssrf_guard` 全部启用） |
| 声明 Hook `failure_mode = FailClosed` | ✅ | ❌（强制 `FailOpen`） |
| 注册 MCP Server（stdio / local） | ✅ | ✅ |
| 注册 MCP Server（remote HTTP） | ✅ | ❌ |
| 注册 Skill | ✅ | ✅ |
| 设置 `strictPluginOnlyCustomization` | ✅ | ❌ |
| 直接读 Journal | ✅（via 提供的 trait） | ❌ |
| 修改 PermissionBroker 行为 | ✅ | ❌ |

> Hook transport 字段定义见 `crates/harness-hook.md §3.2 / §3.3 / §3.4`；`failure_mode` 与 fail-open / fail-closed 语义见同文件 §2.6.1。

### 2.3 来源判定

| 来源 | 默认信任级别 |
|---|---|
| Workspace 管理员显式安装（`data/plugins/`） | `AdminTrusted`（要求签名） |
| `~/.octopus/plugins/`（用户本地） | `UserControlled` |
| 项目内 `.octopus/plugins/`（需 `--allow-project-plugins` flag） | `UserControlled`（除非签名） |
| Cargo extension（`octopus-plugin-*`） | `AdminTrusted`（因经过 `cargo install`） |

### 2.4 签名要求

`AdminTrusted` Plugin **必须**有有效签名（可配置信任的 signer 列表，默认只信任 Octopus 官方）：

```rust
pub struct ManifestSignature {
    pub algorithm: SignatureAlgorithm,
    pub signer: String,
    pub signature: Bytes,
}
```

`UserControlled` Plugin 签名可选；但如果带签名也不会自动升级为 `AdminTrusted`（避免签名被伪造）。

### 2.5 运行期校验

Plugin 注册能力时，Registry 校验 `TrustLevel` 是否允许：

```rust
impl ToolRegistry {
    pub fn register_from_plugin(
        &self,
        tool: Box<dyn Tool>,
        trust: TrustLevel,
    ) -> Result<(), RegistrationError> {
        let props = tool.properties();
        if props.is_destructive && trust != TrustLevel::AdminTrusted {
            return Err(RegistrationError::TrustViolation {
                required: TrustLevel::AdminTrusted,
                provided: trust,
            });
        }
        // ...
    }
}
```

## 3. 替代方案

### 3.1 A：单一信任域，由运行期权限模式控制

- ❌ `PermissionMode` 只能控制单次调用，不能控制注册时的能力
- ❌ 用户安装的恶意 Plugin 可以注册"看起来无害"的 Tool，实际调用时绕过审批

### 3.2 B：多级信任（System / Workspace / User / Anonymous）

- ❌ 运营负担重
- ❌ 参考项目没有一家做到四级
- ❌ 用户选择困难

### 3.3 C：二分（采纳）

- ✅ 语义清晰（企业审核 vs 用户自取）
- ✅ 能力矩阵简单
- ✅ 对齐 CC-27

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| 签名基础设施 | Admin-Trusted 需要签名 | 提供 `octopus plugin sign` 工具；企业可自建 signer |
| User-Controlled 能力受限 | 无法 `Bash` 等破坏性工具 | 用户可以改走 `PermissionMode::AcceptEdits` 或 Admin 审核后升级 |
| 项目插件歧义 | 项目目录下的插件算 User 还是 Admin | 默认 User；除非配置 `trusted_project_plugins: true` |

## 5. 后果

### 5.1 正面

- 供应链攻击影响限制在 User 域
- 企业可以放心分发 Admin 插件
- 与 Octopus 仓库的 `admin_scopes / user_scopes` 治理原则一致

### 5.2 负面

- User-Controlled Plugin 使用上有限制
- 签名校验增加冷启动时间（可缓存）

## 6. 实现指引

### 6.1 Manifest 校验

```yaml
name: my-plugin
version: 1.0.0
trust_level: admin-trusted
signature:
  algorithm: ed25519
  signer: octopus-official
  signature: base64encoded...
capabilities:
  tools:
    - send_invoice
```

若 `trust_level` 与实际来源不匹配（如 `~/.octopus/plugins/` 下声明 `admin-trusted`），Plugin Registry 拒绝加载并发 `Event::PluginRejected { reason: TrustMismatch }`。

### 6.2 `strictPluginOnlyCustomization`

Admin-Trusted Plugin 可以在 HarnessOptions 设置此标志，禁止 User-Controlled Plugin 注册任何 Tool（企业托管模式常用）。

### 6.3 审计

所有 Plugin 加载事件必须记录：

```rust
Event::PluginLoaded { plugin_id, trust_level, capabilities, manifest_hash }
Event::PluginRejected { plugin_id, reason }
```

## 7. 相关

- `crates/harness-plugin.md`
- D9 · `security-trust.md` §9 信任链
- Evidence: CC-27, OC-18, HER-035
