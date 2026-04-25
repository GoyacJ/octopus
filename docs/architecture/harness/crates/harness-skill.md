# `octopus-harness-skill` · L2 复合能力 · Skill System SPEC

> 层级：L2 · 状态：Accepted
> 依赖：`harness-contracts` + `harness-memory`（读 · 复用 `MemoryThreatScanner`）
> 关联 ADR：ADR-003（Prompt Cache 锁定）/ ADR-006（插件信任域）/ ADR-011（Tool Capability Handle）

## 1. 职责

提供**技能系统**：Markdown + YAML frontmatter 形式的提示模板，多源加载优先级，per-agent allowlist，模板展开，渐进式披露给 LLM。对齐 HER-037/HER-038 / OC-22 / CC-26。

**核心能力**：

- Skill 作为 Markdown 文件（非 Rust 代码），与 `agentskills.io` 开放标准兼容（§12）
- 多源优先级：`Bundled < Plugin < User < Workspace`；`Mcp` 走独立命名空间（§3）
- Per-agent allowlist
- Frontmatter 一等字段：`platforms` / `prerequisites` / `parameters` / `config` / `hooks`
- 模板变量 `${var}` + `${config.<key>}` + 受限 shell `` !`command` ``
- **三种消费路径**：静态注入（Eager）/ 渐进披露（`SkillsListTool` / `SkillViewTool` / `SkillInvokeTool`）/ Hook 主动注入
- 无论哪种路径，注入均为 **user message**（不碰 system prompt，保 Prompt Cache · ADR-003）
- 加载期对 User/Workspace/Plugin/Mcp 来源做 **Prompt Injection 扫描**（复用 `MemoryThreatScanner`）

## 2. 对外 API

### 2.1 核心类型

```rust
pub struct Skill {
    pub id: SkillId,
    pub name: String,
    pub description: String,
    pub source: SkillSource,
    pub frontmatter: SkillFrontmatter,
    pub body: String,
    pub raw_path: Option<PathBuf>,
}

pub struct SkillFrontmatter {
    /// ≤ 64 chars（agentskills.io 兼容约束 · §12）
    pub name: String,
    /// ≤ 1024 chars（agentskills.io 兼容约束 · §12）
    pub description: String,

    /// 未声明 → 全体 Agent 可见
    pub allowlist_agents: Option<Vec<String>>,

    /// 调用期参数：每次 render 时由调用方传入；强类型 + 必填校验
    pub parameters: Vec<SkillParameter>,

    /// 启用期配置：skill 安装/启用时持久化在配置中心；模板内通过 `${config.<key>}` 引用；
    /// `secret = true` 的字段不得从 frontmatter 读默认值，必须由 `SecretResolver` 注入
    pub config: Vec<SkillConfigDecl>,

    /// 平台门控：空集合 = 全平台；非空集合 = 仅命中平台加载（§7）
    pub platforms: Vec<SkillPlatform>,

    /// 先决条件：env_vars 缺失 → 标记 `prerequisite-missing`（仍加载）；
    /// commands 缺失 → 仅 advisory（参考 HER-037 中 `prerequisites.commands` 的 advisory 行为）
    pub prerequisites: SkillPrerequisites,

    /// Skill-bound Hooks：与 skill 生命周期绑定，skill 卸载即注销（§8）
    /// 仅 AdminTrusted 来源（Bundled / Plugin{trust=AdminTrusted}）允许声明 Exec/HTTP transport
    pub hooks: Vec<SkillHookDecl>,

    pub tags: Vec<String>,
    pub category: Option<String>,

    /// 业务自定义扩展；约定放在 `octopus.*` namespace 下，避免污染开放标准字段
    pub metadata: HashMap<String, Value>,
}

pub struct SkillParameter {
    pub name: String,
    pub param_type: SkillParamType,
    pub required: bool,
    pub default: Option<Value>,
    pub description: Option<String>,
}

pub enum SkillParamType {
    String,
    Number,
    Boolean,
    Path,
    Url,
}

pub struct SkillConfigDecl {
    /// 如 `github.token` / `slack.workspace_id`
    pub key: String,
    pub value_type: SkillParamType,
    /// `true` → 走 Secret Boundary（不得 default 进 frontmatter）
    pub secret: bool,
    pub required: bool,
    pub default: Option<Value>,
    pub description: Option<String>,
}

#[non_exhaustive]
pub enum SkillPlatform {
    Macos,
    Linux,
    Windows,
}

pub struct SkillPrerequisites {
    /// 必需的环境变量名；缺失 → `SkillStatus::PrerequisiteMissing`，仍加载但 SkillTool 标记
    pub env_vars: Vec<String>,
    /// 期望存在的 PATH 命令；缺失仅 advisory（不阻断加载）
    pub commands: Vec<String>,
}

pub struct SkillHookDecl {
    /// 唯一 id；注册时拼接为 `skill:<skill_name>:<hook_id>` 避免冲突
    pub id: String,
    pub events: Vec<HookEventKind>,
    pub transport: SkillHookTransport,
}

#[non_exhaustive]
pub enum SkillHookTransport {
    /// 内置规则（无外部 IO），仅 frontmatter 声明形态
    Builtin(BuiltinHookKind),
    /// 进程外 Exec hook（仅 AdminTrusted 来源允许）
    Exec(HookExecSpec),
    /// HTTP hook（仅 AdminTrusted 来源允许）
    Http(HookHttpSpec),
}

/// 运行期 Skill 的物理来源；与 `harness-contracts::SkillSourceKind`（语义标签）一一对应，
/// 但额外携带路径 / PluginId / McpServerId 等运行期定位信息。
#[non_exhaustive]
pub enum SkillSource {
    Bundled,
    Workspace(PathBuf),
    User(PathBuf),
    Plugin(PluginId),
    Mcp(McpServerId),
}

impl SkillSource {
    /// 投影到 `harness-contracts::SkillSourceKind`，用于装配 `ToolOrigin::Skill(SkillOrigin)`。
    pub fn to_kind(&self) -> SkillSourceKind { /* ... */ }
}

// `SkillId` 的权威定义在 `harness-contracts::SkillId`，本 crate re-export 之。
pub use octopus_harness_contracts::SkillId;
```

#### 2.1.1 `SkillOrigin` 与 `SkillRegistration`（被 `harness-contracts` / `harness-session` 引用）

```rust
/// `harness-contracts::ToolOrigin::Skill` 中携带的来源元数据。
/// SkillTool / SkillInvokeTool 装配的 ToolDescriptor.origin 即指向此结构。
pub struct SkillOrigin {
    pub skill_id: SkillId,
    pub skill_name: String,
    pub source: SkillSource,
    /// 来源信任级别：影响 hooks 的 transport 准入（§8.3）
    pub trust: TrustLevel,
}

/// `harness-session::ConfigDelta::add_skills` 携带的注册项。
pub struct SkillRegistration {
    pub skill: Skill,
    /// 命中 allowlist 的 agent 集合；为空表示沿用 frontmatter 自带 allowlist_agents
    pub force_allowlist: Option<Vec<AgentId>>,
}
```

### 2.2 Loader

```rust
pub struct SkillLoader {
    sources: Vec<SkillSourceConfig>,
    shell_allowlist: HashSet<String>,
    max_shell_output: usize,
    /// 当前运行平台（用于 platforms 门控；测试可注入）
    runtime_platform: SkillPlatform,
    /// 复用 harness-memory 的扫描器；None 时跳过扫描
    threat_scanner: Option<Arc<MemoryThreatScanner>>,
}

pub enum SkillSourceConfig {
    Bundled,
    Directory { path: PathBuf, source_kind: DirectorySourceKind },
    McpServer { server_id: McpServerId },
}

/// 仅目录式加载使用；与 `harness-contracts::SkillSourceKind`（覆盖全部 5 种来源的语义标签）
/// 不同——本枚举只覆盖三种"以目录为根"的路径。
pub enum DirectorySourceKind {
    Workspace,
    User,
    Plugin(PluginId),
}

impl SkillLoader {
    pub fn default() -> Self;
    pub fn with_source(self, source: SkillSourceConfig) -> Self;
    pub fn with_shell_allowlist(self, cmds: impl IntoIterator<Item = String>) -> Self;
    pub fn with_threat_scanner(self, scanner: Arc<MemoryThreatScanner>) -> Self;
    pub fn with_runtime_platform(self, platform: SkillPlatform) -> Self;

    pub async fn load_all(&self) -> Result<LoadReport, SkillError>;
    pub async fn load_by_name(&self, name: &str) -> Result<Skill, SkillError>;
}

pub struct LoadReport {
    pub loaded: Vec<Skill>,
    /// 解析失败 / 平台不匹配 / 威胁命中等被跳过的项；详见 §11.3 降级矩阵
    pub rejected: Vec<SkillRejection>,
}

pub struct SkillRejection {
    pub source: SkillSource,
    pub raw_path: Option<PathBuf>,
    pub reason: SkillRejectReason,
}

#[non_exhaustive]
pub enum SkillRejectReason {
    ParseFrontmatter(String),
    PlatformMismatch { required: Vec<SkillPlatform> },
    ThreatDetected { pattern_id: String, category: ThreatCategory },
    NameTooLong(usize),
    DescriptionTooLong(usize),
    HookTransportNotPermitted { trust: TrustLevel },
}
```

### 2.3 Registry

```rust
pub struct SkillRegistry {
    inner: Arc<RwLock<SkillRegistryInner>>,
}

struct SkillRegistryInner {
    by_name: BTreeMap<String, Arc<Skill>>,
    by_source: HashMap<SkillSource, Vec<SkillId>>,
    /// 加载期记录的状态（prerequisite missing 等）
    status: HashMap<SkillId, SkillStatus>,
    generation: u64,
}

#[non_exhaustive]
pub enum SkillStatus {
    Ready,
    PrerequisiteMissing { env_vars: Vec<String> },
}

impl SkillRegistry {
    pub fn builder() -> SkillRegistryBuilder;
    pub fn register(&self, skill: Skill) -> Result<(), RegistrationError>;
    pub fn get(&self, name: &str) -> Option<Arc<Skill>>;

    /// 返回完整 Skill；用于 SkillViewTool / 业务直接渲染场景
    pub fn list_available_for_agent(&self, agent: &AgentId) -> Vec<Arc<Skill>>;

    /// 返回 metadata-only 摘要；SkillsListTool 默认走此路径以省 tokens
    pub fn list_summaries_for_agent(&self, agent: &AgentId, filter: SkillFilter)
        -> Vec<SkillSummary>;

    pub fn snapshot(&self) -> SkillRegistrySnapshot;
}

pub struct SkillSummary {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub category: Option<String>,
    pub source: SkillSource,
    pub status: SkillStatus,
}

pub struct SkillFilter {
    pub tag: Option<String>,
    pub category: Option<String>,
    pub include_prerequisite_missing: bool,  // 默认 false
}
```

### 2.4 模板引擎

```rust
pub struct SkillRenderer {
    vars: HashMap<String, Value>,
    config_resolver: Arc<dyn SkillConfigResolver>,
    shell_allowlist: HashSet<String>,
    max_shell_output: usize,
}

#[async_trait]
pub trait SkillConfigResolver: Send + Sync + 'static {
    async fn resolve(&self, key: &str) -> Result<Value, ConfigResolveError>;
    async fn resolve_secret(&self, key: &str) -> Result<SecretString, ConfigResolveError>;
}

impl SkillRenderer {
    pub fn new(config_resolver: Arc<dyn SkillConfigResolver>) -> Self;
    pub fn set_var(&mut self, name: &str, value: Value);
    pub async fn render(&self, skill: &Skill, params: Value)
        -> Result<RenderedSkill, RenderError>;
}

pub struct RenderedSkill {
    pub skill_id: SkillId,
    pub skill_name: String,
    pub content: String,
    pub shell_invocations: Vec<ShellInvocation>,
    /// 渲染期消耗的 config keys（含 secret，落事件时只记 hash）
    pub consumed_config_keys: Vec<String>,
}

pub struct ShellInvocation {
    pub command: String,
    pub stdout_truncated: bool,
    pub exit_code: i32,
}
```

支持的模板语法：

| 语法 | 含义 | 来源 |
|---|---|---|
| `${var_name}` | 调用期 `parameters` 注入 | `SkillRenderer::set_var` 或 `render` 的 `params` |
| `${config.<key>}` | 启用期 `config` 注入 | `SkillConfigResolver::resolve` |
| `${config.<key>:secret}` | Secret 注入 | `SkillConfigResolver::resolve_secret`（渲染结果不写入 Journal 明文） |
| `` !`<command>` `` | 受限 shell（输出上限默认 4000 字符，对齐 HER-038） | 命令必须命中 `shell_allowlist` |

**未解析变量**保留原样（如 `${unknown}`），便于用户调试，对齐 HER-038。

### 2.5 `SkillRegistryCap`（被 `ToolCapability::SkillRegistry` 引用）

`SkillsListTool` / `SkillViewTool` / `SkillInvokeTool` 通过 `ToolContext::capability::<dyn SkillRegistryCap>(ToolCapability::SkillRegistry)` 借用本 trait（ADR-011）。

```rust
#[async_trait]
pub trait SkillRegistryCap: Send + Sync + 'static {
    fn list_summaries(&self, agent: &AgentId, filter: SkillFilter)
        -> Vec<SkillSummary>;

    fn view(&self, agent: &AgentId, name: &str) -> Option<SkillView>;

    async fn render(&self, agent: &AgentId, name: &str, params: Value)
        -> Result<RenderedSkill, RenderError>;
}

pub struct SkillView {
    pub summary: SkillSummary,
    pub parameters: Vec<SkillParameter>,
    pub config_keys: Vec<String>,   // 仅 key，不带值
    pub body_preview: String,        // 默认 head 1024 chars
    pub body_full: Option<String>,   // 仅 view(name, full=true) 时携带
}
```

`SkillRegistry` 提供该 trait 的默认实现（`impl SkillRegistryCap for Arc<SkillRegistry>`）；测试用 `MockSkillRegistryCap`（`harness-contracts/testing` feature）。

## 3. 多源优先级

```rust
const PRIORITY_ORDER: &[SkillSourceKind] = &[
    SkillSourceKind::Bundled,
    SkillSourceKind::Plugin(_),
    SkillSourceKind::User,
    SkillSourceKind::Workspace,
];
```

对齐 OC-22：**后覆盖前**。例如：

- `Bundled` 有 `review-pr.md`
- `~/.octopus/skills/review-pr.md`（User）覆盖
- `data/skills/review-pr.md`（Workspace）再覆盖

**MCP 命名空间例外**：MCP Server 提供的 Skill 沿用 canonical 命名 `mcp__<server>__<skill>`（与 MCP 工具一致，见 `harness-contracts §3.4.2`）；**MCP skill 不参与本地同名覆盖**——本地 `review-pr.md` 与 `mcp__github__review-pr` 永远是两个独立条目，避免 MCP server 通过同名劫持本地 skill。

## 4. Per-Agent Allowlist

### 4.1 Frontmatter 声明

```yaml
---
name: advanced-refactor
description: Refactor code across files
allowlist_agents:
  - senior-engineer
  - refactor-specialist
---
```

### 4.2 加载时过滤

```rust
impl SkillRegistry {
    pub fn list_available_for_agent(&self, agent: &AgentId) -> Vec<Arc<Skill>> {
        self.inner.read().by_name.values().filter(|s| {
            s.frontmatter.allowlist_agents
                .as_ref()
                .map(|list| list.iter().any(|a| a == agent.as_ref()))
                .unwrap_or(true)  // 未声明 allowlist 即全体可见
        }).cloned().collect()
    }
}
```

> **位置 vs 可见性分离**（对齐 OC-22）：`PRIORITY_ORDER` 决定**哪份胜出**；`allowlist_agents` 决定**谁能用**。两个机制独立组合。

## 5. 注入位置（对齐 HER-038）

Skill 内容注入为 **user message**，**不碰 system prompt**，以保 Prompt Cache（ADR-003）：

```text
User message:
---SKILL-BEGIN: review-pr---
[Rendered skill content]
---SKILL-END---

Actual user input: "请帮我 review PR #123"
```

注入的具体位置（在 Active Context Patches 内的次序）、保留策略（transient vs persistent）、与 Hook AddContext 的优先级关系，由 `harness-context` 统一管理；详见 `context-engineering.md §11`。

## 6. 渐进式披露：内置 SkillTool 三件套

Eager 注入在 Skill 数量 > 20 时显著挤占 context；本节提供与 `ToolSearchTool` 平行的"按需消费"路径（对齐 CC-26 `SkillTool` 一等工具 + HER-037 `skills_list/skill_view/skill_manage` 三级披露）。

### 6.1 工具清单

| Tool name | 职责 | `defer_policy` 默认 | 返回 |
|---|---|---|---|
| `skills_list` | metadata-only 列表，按 `tag/category` 过滤 | `AlwaysLoad` | `Vec<SkillSummary>` |
| `skills_view` | 查看完整 SKILL.md，含 parameters / config keys | `AutoDefer` | `SkillView` |
| `skills_invoke` | 渲染 + 注入当前 turn 的 user message（不持久化为 session 配置） | `AutoDefer` | `SkillInvocationReceipt` |

> 三件套均 `ToolOrigin = Builtin`、`required_capabilities = [ToolCapability::SkillRegistry]`、`group = ToolGroup::Meta`，受 `BuiltinToolset::Default` 自动装配。`skills_view` / `skills_invoke` 默认走 `AutoDefer`：模型先调 `skills_list` 看到名字，再 `tool_search select:skills_view` 解锁详细面（与 ADR-009 路径一致）。

### 6.2 `SkillsListTool`

```rust
pub struct SkillsListTool;  // 内部仅持有元数据，运行期通过 ToolContext 借 cap

#[derive(Deserialize)]
pub struct SkillsListInput {
    pub tag: Option<String>,
    pub category: Option<String>,
    pub include_prerequisite_missing: Option<bool>,  // 默认 false
}
```

返回 `Vec<SkillSummary>`：仅 metadata，不含 body，**显著节省 tokens**。

### 6.3 `SkillsViewTool`

```rust
#[derive(Deserialize)]
pub struct SkillsViewInput {
    pub name: String,
    pub full: Option<bool>,        // 默认 false → 仅 1024 chars 预览
}
```

返回 `SkillView`（§2.5）。`body_full` 仅在 `full=true` 时填充，避免无意义吞掉 ResultBudget。

### 6.4 `SkillsInvokeTool`

```rust
#[derive(Deserialize)]
pub struct SkillsInvokeInput {
    pub name: String,
    /// 调用期参数；与 frontmatter 的 parameters 一一对应
    pub params: Value,
}

pub struct SkillInvocationReceipt {
    pub skill_name: String,
    pub injection_id: SkillInjectionId,
    /// 内容已注入当前 turn 的 user message；这里只回执元数据避免重复占 tokens
    pub bytes_injected: u64,
    pub consumed_config_keys: Vec<String>,
}
```

执行流程：

```text
SkillsInvokeTool::execute
    │
    ▼  registry_cap.render(agent, name, params)
    │
    ▼  ContextEngine::queue_active_patch(SkillPatch {
    │      lifecycle: ContentLifecycle::Transient,   // 默认；§11 详述
    │      injection_id, content, ...
    │  })
    │
    ▼  emit Event::SkillInvoked { skill_id, injection_id, bytes, ... }
    │
    └─→ Final(SkillInvocationReceipt)
```

**关键设计**：
- `SkillsInvokeTool` 返回值**不携带渲染 body**（避免与 user message 注入重复占 tokens）；
- 注入走 `Active Context Patches`，`Transient` 默认，压缩时优先剔除（仅保留 `Event::SkillInvoked` 元数据）；
- 业务可调 `SkillsInvokeTool` 时通过 `params._lifecycle = "persistent"` 升级为常驻（受 trust 矩阵限制：仅 AdminTrusted agent 允许）。

### 6.5 与 Eager 注入的关系

两条路径**互不冲突**：

| 路径 | 何时使用 | Cache 行为 |
|---|---|---|
| Eager（`SkillPrefetchStrategy::Eager`） | Skill 数量少（< 20） + 几乎每 turn 都用到 | system message 创建期一次性装配，不影响运行期 cache（ADR-003） |
| 渐进披露（SkillTool 三件套） | Skill 数量多（> 20）+ 单 turn 仅用少数 | Tool 调用属正常 user/assistant turn，**不破坏前缀 cache** |

业务可在 `SkillRegistry::builder()` 同时启用两条路径；Eager 注入的 skill **不会**从 SkillTool 列表移除（让模型仍可显式重新激活）。

## 7. Platform Gating 与 Prerequisites（对齐 HER-037）

### 7.1 Platforms

```yaml
---
name: macos-notifier
platforms: [macos]   # 仅 macOS 加载；Linux / Windows fail-closed 跳过并发 SkillRejection
---
```

加载行为：

- `frontmatter.platforms` 为空 → 全平台加载
- 非空 → 只在 `runtime_platform` 命中时加载；不命中：**fail-closed 跳过** + `SkillRejection { reason: PlatformMismatch }` + 不进入 Registry

### 7.2 Prerequisites

```yaml
---
prerequisites:
  env_vars: [GITHUB_TOKEN]
  commands: [gh, jq]    # advisory only
---
```

加载行为：

| 检查 | 命中 | 不命中 |
|---|---|---|
| `env_vars` 存在 | 正常进入 Registry | 进入 Registry，但 `SkillStatus::PrerequisiteMissing { env_vars }` |
| `commands` 在 PATH | 正常 | 仅记 `Event::SkillPrerequisiteAdvisory`，不影响加载（advisory） |

`SkillsListTool` 默认 `include_prerequisite_missing = false`，模型看不到状态异常的 skill；业务可在 UX 上提示用户配置缺失项。

## 8. Hook 绑定生命周期（对齐 CC-22 `registerSkillHooks`）

### 8.1 Frontmatter 声明

```yaml
---
name: code-review-with-audit
hooks:
  - id: audit
    events: [PostToolUse, PostToolUseFailure]
    transport:
      type: builtin
      kind: AuditLog        # BuiltinHookKind 枚举
  - id: notify
    events: [SubagentStop]
    transport:
      type: exec            # 仅 AdminTrusted 来源 frontmatter 允许
      command: /usr/local/bin/notify-skill-finished
      timeout: 5s
---
```

### 8.2 注册与生命周期

加载 skill 时，`SkillLoader` 生成 `HookHandler` 并注册到 `HookRegistry`，**handler_id 拼接为 `skill:<skill_name>:<hook_id>`** 避免冲突。

```text
load skill
  │
  ▼  生成 HookHandler  (Builtin / Exec / Http transport)
  │
  ▼  HookRegistry::register(handler) ── 普通注册路径，不绕过 trust 检查
  │
  ▼  在 SkillRegistry 记录 SkillHookBinding { skill_id, handler_ids }
  │
unload skill（reload_with 移除 / SkillRegistry::deregister）
  │
  ▼  自动反注册全部绑定 handler，避免野 hook 泄漏
```

### 8.3 Trust 准入

| Skill 来源 | `Builtin` transport | `Exec` / `Http` transport |
|---|---|---|
| `Bundled` | ✅ | ✅ |
| `Plugin{trust=AdminTrusted}` | ✅ | ✅ |
| `Plugin{trust=UserControlled}` | ✅ | ❌（`SkillRejectReason::HookTransportNotPermitted`） |
| `User` | ✅ | ❌ |
| `Workspace` | ✅ | ❌ |
| `Mcp` | ❌（MCP server 不允许通过 skill 注入 hook，避免反向劫持） | ❌ |

不满足条件 → 整个 skill 被 reject（**整体 fail-closed**，不允许"声明了但忽略"以避免静默降级）。

## 9. 内容威胁扫描（对齐 HER-038 `_MEMORY_THREAT_PATTERNS`）

复用 `harness-memory::MemoryThreatScanner`（详见 `harness-memory.md §6`），不引入新 trait，避免重复维护威胁模式库。

### 9.1 扫描范围

加载期对 **Skill body** 与 **frontmatter description** 做扫描：

| 源 | 是否扫描 | 命中 `Block` 行为 |
|---|---|---|
| `Bundled` | 跳过（仓内已审核） | — |
| `User` | 强制扫描 | reject + `Event::SkillThreatDetected` |
| `Workspace` | 强制扫描 | reject + `Event::SkillThreatDetected` |
| `Plugin{any trust}` | 强制扫描 | reject + `Event::SkillThreatDetected` |
| `Mcp` | 强制扫描 | reject + `Event::SkillThreatDetected` |

`Redact` / `Warn` 两档动作不阻断加载，仅记事件（与 memory 路径对称）。

### 9.2 事件

`Event::SkillThreatDetected` 与 `Event::MemoryThreatDetected` 共享同一 `MemoryThreatDetectedEvent` 字段集（`pattern_id / category / severity / action / content_hash`），仅 `subject` 字段区分 `memory` vs `skill`。事件**不展示原始内容**，仅记 hash（与 memory 一致）。

### 9.3 集成示例

```rust
let scanner = Arc::new(MemoryThreatScanner::default());

let loader = SkillLoader::default()
    .with_source(SkillSourceConfig::Directory {
        path: dirs::home_dir().unwrap().join(".octopus/skills"),
        source_kind: SkillSourceKind::User,
    })
    .with_threat_scanner(Arc::clone(&scanner));

// MemoryManager 也用同一个 scanner，威胁模式库统一维护
let memory = MemoryManager::new()
    .with_threat_scanner(scanner);
```

## 10. Reload 路径（与 ADR-003 对齐）

业务通过 `Session::reload_with(ConfigDelta { add_skills })` 注入新 skill。链路：

```text
Session::reload_with(ConfigDelta { add_skills: vec![SkillRegistration, ..] })
    │
    ▼  SkillLoader::validate(reg)         ── platforms / prerequisites / threat scan
    │      命中 reject ─→ Err(SkillError) 整体回滚
    │
    ▼  SkillRegistry::insert_batch(skills)
    │      产生新 SkillRegistrySnapshot（CoW，旧 snapshot 仍被旧 turn 引用）
    │
    ▼  绑定 SkillHookDecl 中的 handlers 到 HookRegistry
    │
    ▼  ConfigDelta 分类（harness-session §2.4）
    │      仅 add_skills → AppliedInPlace + OneShotInvalidation { reason: SkillsAppended }
    │
    ▼  下一 turn 起 ContextEngine 拉取新 snapshot（含新 skill 进入 SkillsListTool 视野）
    │
    ▼  Event::SessionReloadApplied { ..., cache_impact, effective_from: NextTurn }
```

**snapshot 生命周期**：

- 旧 snapshot：被当前进行中的 turn 持续引用，turn 完成后由 Arc 自动释放
- 新 snapshot：下一 turn 起生效；同时 `SkillTool` 三件套通过 `SkillRegistryCap` 看到新条目
- `add_skills` 失败时**整体回滚**，不留半成品（fail-closed）

## 11. 加载降级矩阵

| 触发 | 行为 | 事件 |
|---|---|---|
| `Bundled` 源 frontmatter 解析失败 | **硬 fail**：`SkillError` 中止整个 `load_all` | — |
| `User/Workspace/Plugin/Mcp` 源 frontmatter 解析失败 | 单 skill 跳过，其他继续 | `Event::SkillRejected { reason: ParseFrontmatter }` |
| `name > 64 chars` 或 `description > 1024 chars`（agentskills.io 标准） | 单 skill 跳过 | `Event::SkillRejected { reason: NameTooLong / DescriptionTooLong }` |
| `platforms` 不命中当前运行平台 | 单 skill 跳过（fail-closed） | `Event::SkillRejected { reason: PlatformMismatch }` |
| `prerequisites.env_vars` 缺失 | 仍加载，标记 `PrerequisiteMissing` | `Event::SkillPrerequisiteMissing` |
| `prerequisites.commands` 缺失 | advisory，正常加载 | `Event::SkillPrerequisiteAdvisory` |
| `MemoryThreatScanner` 命中 `Block` | 单 skill 跳过 | `Event::SkillThreatDetected` |
| `MemoryThreatScanner` 命中 `Redact` | 涂黑后加载 | `Event::SkillThreatDetected { action: Redact }` |
| Skill 来源不允许 `Exec/Http` hook | 整个 skill reject | `Event::SkillRejected { reason: HookTransportNotPermitted }` |
| Skill 名称冲突且优先级相同（同源、同名） | 后注册的 reject | `Event::SkillRejected { reason: Duplicate }` |

## 12. 开放标准兼容（agentskills.io）

`harness-skill` 严格遵循 [agentskills.io](https://agentskills.io) 公开 frontmatter 标准的核心约束：

| 字段 | 约束 | 强制 |
|---|---|---|
| `name` | ≤ 64 chars | ✅ |
| `description` | ≤ 1024 chars | ✅ |
| `platforms` | `[macos, linux, windows]` 子集 | ✅ |
| `prerequisites.env_vars` / `prerequisites.commands` | 字符串数组 | ✅ |
| `parameters` | 标准 schema | ✅ |
| `metadata.<vendor>.*` | 各家自由扩展 | — |

**`harness-skill` 自有扩展**全部置于 `metadata.octopus.*` namespace 下，与开放标准字段并存，确保从 `agentskills.io` 直接导入的 skill 可以无缝运行；反之我们的 skill 在去除 `metadata.octopus.*` 后也能在其它兼容运行时使用。

> 兼容模式 `compat_mode: strict`（loader 配置）：拒绝包含未知 top-level 字段的 frontmatter，仅放行 agentskills.io 标准字段 + `metadata.*`。默认模式 `lenient` 允许未知字段被忽略。

## 13. 分发与信任

`harness-skill` **不自建 skill 市场**。所有第三方 skill 的分发走两条路径：

| 路径 | 信任级别 | 说明 |
|---|---|---|
| **Plugin 携带 skills**（`plugin.yaml::capabilities.skills`） | 与 plugin 同信任域（`AdminTrusted` / `UserControlled`，详见 ADR-006） | 经 plugin manifest 签名 + Registry 校验；卸载 plugin 自动卸载携带的 skill |
| **本地文件**（`User` / `Workspace` 目录） | `UserControlled` | 用户显式放置；自动启用 `MemoryThreatScanner` 扫描 |

**禁止的形态**：

- 不允许通过 `~/.octopus/skills/` 等用户目录声明 `Exec`/`Http` hook（§8.3 trust 矩阵）；
- 不允许 MCP Server 通过自身的 skill 注入 hook（§8.3）；
- 不允许 plugin 跨信任域降级（声明 `admin-trusted` 但来源是 `~/.octopus/plugins/` 会被 `PluginRejected { reason: TrustMismatch }` 拒绝，对齐 ADR-006）。

未来若需要类似 ClawHub / Claude Code marketplace 的中心化分发，亦应作为 `Plugin Marketplace` 的能力（参考 `harness-plugin.md`），而非在 `harness-skill` 内造轮子。

## 14. Shell 命令安全

受限 shell 白名单：

```rust
impl SkillLoader {
    pub fn with_shell_allowlist(mut self, cmds: impl IntoIterator<Item = String>) -> Self {
        self.shell_allowlist.extend(cmds);
        self
    }
}

const DEFAULT_SHELL_ALLOWLIST: &[&str] = &[
    "pwd", "date", "whoami", "hostname", "uname",
];
```

- 命令不在白名单 → 替换为 `[SHELL_NOT_ALLOWED]` 占位
- 输出超过 `max_shell_output` → 截断并追加 `[...truncated N bytes]`
- `Workspace` / `User` / `Plugin{UserControlled}` 来源的 skill：内联 shell **必须**经白名单；`Bundled` 与 `Plugin{AdminTrusted}` 同样受白名单约束（无后门）

## 15. Feature Flags

```toml
[features]
default = ["workspace-source", "user-source", "skill-tool", "threat-scanner"]
bundled-source = []
workspace-source = ["dep:tokio"]
user-source = ["dep:dirs"]
plugin-source = []
mcp-source = []
skill-tool = []                   # SkillsListTool / SkillsViewTool / SkillsInvokeTool
threat-scanner = ["dep:regex"]    # 复用 harness-memory 的 scanner
```

## 16. 错误类型

```rust
#[derive(Debug, thiserror::Error)]
pub enum SkillError {
    #[error("parse frontmatter: {0}")]
    ParseFrontmatter(String),

    #[error("missing required parameter: {0}")]
    MissingParam(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("duplicate name: {0}")]
    Duplicate(String),

    #[error("threat detected: pattern={pattern_id} category={category:?}")]
    ThreatDetected {
        pattern_id: String,
        category: ThreatCategory,
    },

    #[error("platform mismatch: required={required:?}")]
    PlatformMismatch { required: Vec<SkillPlatform> },

    #[error("hook transport not permitted for trust={trust:?}")]
    HookTransportNotPermitted { trust: TrustLevel },

    #[error("name too long: {0} > 64")]
    NameTooLong(usize),

    #[error("description too long: {0} > 1024")]
    DescriptionTooLong(usize),
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("unknown var: {0}")]
    UnknownVar(String),

    #[error("unknown config key: {0}")]
    UnknownConfigKey(String),

    #[error("config resolve: {0}")]
    ConfigResolve(#[from] ConfigResolveError),

    #[error("shell not allowed: {0}")]
    ShellNotAllowed(String),

    #[error("shell exec: {0}")]
    ShellExec(#[from] std::io::Error),
}
```

## 17. 使用示例

### 17.1 Skill 文件

```markdown
---
name: daily-briefing
description: Generate daily briefing for manager
allowlist_agents: ["briefing-agent"]
platforms: [macos, linux]
prerequisites:
  env_vars: [GITHUB_TOKEN]
  commands: [gh]
parameters:
  - name: date
    type: string
    required: false
    default: "today"
  - name: area
    type: string
    required: true
config:
  - key: github.org
    type: string
    required: true
  - key: github.token
    type: string
    secret: true
    required: true
hooks:
  - id: audit
    events: [PostToolUse]
    transport:
      type: builtin
      kind: AuditLog
metadata:
  octopus:
    tags: ["briefing", "reporting"]
---

# Daily Briefing: ${area} on ${date}

Today is !`date +%Y-%m-%d`.
Org: ${config.github.org}

Generate a briefing for ${area}. Include:
1. Key updates
2. Risks & blockers
3. Planned next steps
```

### 17.2 加载与使用

```rust
let scanner = Arc::new(MemoryThreatScanner::default());

let loader = SkillLoader::default()
    .with_source(SkillSourceConfig::Directory {
        path: "data/skills".into(),
        source_kind: SkillSourceKind::Workspace,
    })
    .with_source(SkillSourceConfig::Directory {
        path: dirs::home_dir().unwrap().join(".octopus/skills"),
        source_kind: SkillSourceKind::User,
    })
    .with_shell_allowlist(["date", "pwd"])
    .with_threat_scanner(Arc::clone(&scanner));

let report = loader.load_all().await?;
tracing::info!(loaded = report.loaded.len(), rejected = report.rejected.len());

let registry = SkillRegistry::builder()
    .with_skills(report.loaded)
    .build();
```

### 17.3 渲染（业务直接调用）

```rust
let renderer = SkillRenderer::new(Arc::new(MyConfigResolver::new()));
let skill = registry.get("daily-briefing").unwrap();

let rendered = renderer.render(&skill, json!({
    "area": "backend team",
    "date": "today",
})).await?;

// rendered.content 即为可注入 user message 的文本
```

### 17.4 渲染（通过 SkillTool · 渐进披露路径）

模型自主流程：

1. 模型调 `skills_list { tag: "briefing" }` → 看到 `daily-briefing` 摘要
2. 模型调 `skills_view { name: "daily-briefing" }` → 拿到 parameters 列表
3. 模型调 `skills_invoke { name: "daily-briefing", params: { area: "backend team" } }` → 内容自动注入下一轮 user message

业务无需主动驱动；与 `tool_search` 路径一致（ADR-009）。

## 18. 测试策略

| 类 | 覆盖 |
|---|---|
| 单元 | frontmatter 解析（含开放标准约束）、模板变量、`${config.x}` 解析、shell 受限执行 |
| 多源 | 优先级覆盖（Workspace > User > Plugin > Bundled）、MCP 命名空间隔离 |
| Allowlist | Agent 不在列表则不可见；未声明则全体可见 |
| Platform Gating | runtime_platform 不命中 → reject |
| Prerequisites | env_vars 缺失 → 标记 PrerequisiteMissing；commands 缺失 → advisory only |
| Hooks | Builtin/Exec/Http 各 transport 注册成功；卸载 skill 自动反注册 |
| Trust 矩阵 | UserControlled 来源声明 Exec → 整体 reject |
| Threat Scanner | Block 命中 reject；Redact 命中涂黑加载 |
| SkillTool | `skills_list` 不返回 body；`skills_invoke` 注入 user message 但返回 receipt 不重复占 tokens |
| Reload | `add_skills` → `OneShotInvalidation`；失败整体回滚不留半成品 |
| Shell 安全 | 不在白名单的命令被替换为 `[SHELL_NOT_ALLOWED]` |
| 性能 | 1000 个 Skill 加载 + `skills_list` 查询 < 5ms |

## 19. 可观测性

| 指标 | 说明 |
|---|---|
| `skill_loaded_total` | 按 source 分桶 |
| `skill_rejected_total` | 按 reason 分桶（ParseFrontmatter / PlatformMismatch / ThreatDetected / ...） |
| `skill_render_duration_ms` | 渲染耗时 |
| `skill_invocation_total` | 通过 `SkillsInvokeTool` 的激活次数（按 skill_name 分桶） |
| `skill_invocation_per_turn` | 每 turn 激活分布（识别热点 / 死 skill） |
| `skill_view_total` | 通过 `SkillsViewTool` 的查看次数 |
| `skill_shell_invocation_total` | 受限 shell 调用总数 |
| `skill_shell_blocked_total` | 被白名单拒的调用数 |
| `skill_threat_detections_total` | 按 category 分桶 |
| `skill_prerequisite_missing_total` | 按 env_var 分桶（识别需要补全配置的 skill） |
| `skill_dead_total` | 过去 N 天未被激活的 skill 计数（可用于运营提示） |

## 20. 反模式

- Skill 注入 system message（破坏 Prompt Cache · ADR-003）
- 模板支持任意 Turing-complete 脚本（应只支持变量 + 受限 shell）
- 把 Skill 当 Tool（Skill 是提示模板；要执行代码请实现 `Tool`）
- Frontmatter 未声明 `allowlist_agents` 就把 Admin 级 Skill 公开给所有 Agent
- Skill 数量 > 50 仍坚持 `SkillPrefetchStrategy::Eager`（应启用 SkillTool 渐进披露）
- 在 `User`/`Workspace` skill 里写 `Exec` hook（trust 矩阵会整体 reject）
- 跳过 `MemoryThreatScanner`（恶意 skill 可长期劫持 Agent，对齐 memory 反模式）
- 在 `${config.<key>:secret}` 解析后将明文写入 Journal 或 SkillInvocationReceipt
- 通过 MCP server 注入 skill 然后再注入 hook（trust 矩阵硬拒绝；不要尝试绕开）

## 21. 相关

- D7 · `extensibility.md` §4 Skill 扩展（业务上手面）
- D8 · `context-engineering.md` §11 Skill 注入位置与 lifecycle
- ADR-003 · Prompt Cache 锁定（reload 路径源头）
- ADR-006 · Plugin 信任域二分（hook trust 矩阵基础）
- ADR-011 · Tool Capability Handle（`SkillRegistryCap` 借用契约）
- `harness-memory.md` §6 · MemoryThreatScanner（共享威胁模式库）
- `harness-engine.md` §6.1 · SkillPrefetcher（Eager / Lazy / Hint 加载策略）
- Evidence: HER-037, HER-038, OC-22, CC-22, CC-26
