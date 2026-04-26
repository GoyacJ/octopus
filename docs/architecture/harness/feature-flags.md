# D10 · Feature Flag 手册

> 依赖 ADR：ADR-008（Crate 命名与分层）
> 状态：Accepted · 所有 feature 必须在本文登记后才能在 `Cargo.toml` 启用

## 1. 设计目标

1. **细粒度 gated**：业务层按需启用能力，减少编译时间与二进制体积
2. **默认保守**：默认 feature 只覆盖主流场景（SQLite + Local Sandbox + Interactive Broker）
3. **组合可预测**：feature 之间无隐式依赖，开启 A 不会意外激活 B
4. **testing 专用**：`testing` feature 汇总所有 mock 实现，禁止进生产

---

## 2. Feature 总览

### 2.1 harness-sdk（门面 crate，业务层直接 `features = [...]`）

```toml
[features]
default = [
    "sqlite-store",
    "jsonl-store",
    "local-sandbox",
    "interactive-permission",
    "mcp-stdio",
    "provider-anthropic",
    "tool-search",
    "steering-queue",
]
# 默认集合的选择理由见 §3.5 "Default 集合说明"

# --- 存储 ---
sqlite-store          = ["octopus-harness-journal/sqlite"]
jsonl-store           = ["octopus-harness-journal/jsonl"]
in-memory-store       = ["octopus-harness-journal/in-memory"]

# --- LLM Provider ---
# v1.0 内置 Provider 全部是可用实现；default 只默认启用 Anthropic。
provider-openai       = ["octopus-harness-model/openai"]
provider-anthropic    = ["octopus-harness-model/anthropic"]
provider-gemini       = ["octopus-harness-model/gemini"]
provider-openrouter   = ["octopus-harness-model/openrouter"]
provider-bedrock      = ["octopus-harness-model/bedrock"]
provider-codex        = ["octopus-harness-model/codex"]
provider-local-llama  = ["octopus-harness-model/local-llama"]
provider-deepseek     = ["octopus-harness-model/deepseek"]
provider-minimax      = ["octopus-harness-model/minimax"]
provider-qwen         = ["octopus-harness-model/qwen"]
provider-doubao       = ["octopus-harness-model/doubao"]
provider-zhipu        = ["octopus-harness-model/zhipu"]
provider-km           = ["octopus-harness-model/km"]
all-providers = [
    "provider-openai",
    "provider-anthropic",
    "provider-gemini",
    "provider-openrouter",
    "provider-bedrock",
    "provider-codex",
    "provider-local-llama",
    "provider-deepseek",
    "provider-minimax",
    "provider-qwen",
    "provider-doubao",
    "provider-zhipu",
    "provider-km",
]

# --- 沙箱 ---
local-sandbox         = ["octopus-harness-sandbox/local"]
docker-sandbox        = ["octopus-harness-sandbox/docker"]
ssh-sandbox           = ["octopus-harness-sandbox/ssh"]
noop-sandbox          = ["octopus-harness-sandbox/noop"]

# --- MCP ---
mcp-stdio             = ["octopus-harness-mcp/stdio"]
mcp-http              = ["octopus-harness-mcp/http"]
mcp-websocket         = ["octopus-harness-mcp/websocket"]
mcp-sse               = ["octopus-harness-mcp/sse"]
mcp-in-process        = ["octopus-harness-mcp/in-process"]
mcp-server-adapter    = ["octopus-harness-mcp/server-adapter"]

# --- 权限 ---
interactive-permission = ["octopus-harness-permission/interactive"]
stream-permission      = ["octopus-harness-permission/stream"]
rule-engine-permission = ["octopus-harness-permission/rule-engine"]

# --- 记忆 ---
memory-builtin         = ["octopus-harness-memory/builtin"]
memory-external-slot   = ["octopus-harness-memory/external-slot"]

# --- 多 Agent ---
agents-subagent        = ["octopus-harness-subagent"]
agents-team            = ["octopus-harness-team"]

# --- 观测性 ---
observability-replay   = ["octopus-harness-observability/replay"]
observability-otel     = ["octopus-harness-observability/otel"]
observability-prometheus = ["octopus-harness-observability/prometheus"]

# --- 插件 ---
plugin-dynamic-load    = ["octopus-harness-plugin/dynamic-load"]
plugin-manifest-sign   = ["octopus-harness-plugin/manifest-sign"]

# --- Tool Search（ADR-009） ---
tool-search                   = ["dep:octopus-harness-tool-search", "tool-loading-anthropic", "tool-loading-inline"]
tool-loading-anthropic        = ["octopus-harness-tool-search/backend-anthropic"]
tool-loading-inline           = ["octopus-harness-tool-search/backend-inline"]
tool-search-default-scorer    = ["octopus-harness-tool-search/scorer-default"]

# --- Programmatic Tool Calling（ADR-0016；默认 off，M1 起可灰度开启） ---
programmatic-tool-calling     = [
    "octopus-harness-tool/programmatic-tool-calling",
    "octopus-harness-sandbox/code-runtime",
]

# --- Steering Queue（ADR-0017；默认 on） ---
steering-queue                = [
    "octopus-harness-session/steering",
    "octopus-harness-engine/steering",
]

# --- 测试辅助 ---
testing = [
    "in-memory-store",
    "noop-sandbox",
    "octopus-harness-permission/mock",
    "octopus-harness-model/mock",
]
```

### 2.2 各内部 crate 自身 features

内部 crate 通常提供 `default = []`（即最小核心），feature 决定哪些实现被编译：

| Crate | 提供的 features |
|---|---|
| `harness-contracts` | — |
| `harness-model` | `http-client`（内部传输底座）/ `openai-compatible`（OpenAI-compatible Provider 共享底座）/ `openai / anthropic / gemini / openrouter / bedrock / codex / local-llama / deepseek / minimax / qwen / doubao / zhipu / km / all-providers / mock` |
| `harness-journal` | `sqlite / jsonl / in-memory / mock / blob-file / blob-sqlite` |
| `harness-sandbox` | `local / docker / ssh / noop / code-runtime`（ADR-0016） |
| `harness-permission` | `interactive / stream / rule-engine / auto-mode / mock / dangerous / integrity` |
| `harness-memory` | `builtin / external-slot / threat-scanner` |
| `harness-tool` | `builtin-toolset / programmatic-tool-calling`（ADR-0016） |
| `harness-skill` | `workspace-source / user-source / plugin-source / mcp-source` |
| `harness-mcp` | `stdio / http / websocket / sse / in-process / server-adapter / oauth` |
| `harness-hook` | `in-process / exec / http` |
| `harness-context` | `anthropic-cache / compact-aux-llm` |
| `harness-session` | `workspace-bootstrap / hot-reload-fork / steering`（ADR-0017） |
| `harness-engine` | `subagent-tool / parallel-tools / steering`（ADR-0017） |
| `harness-subagent` | — (始终内聚) |
| `harness-team` | `coordinator-worker / peer-to-peer / role-routed` |
| `harness-plugin` | `dynamic-load / manifest-sign / wasm-runtime`（wasm 实验） |
| `harness-observability` | `replay / otel / prometheus / redactor` |
| `harness-tool-search` | `backend-anthropic / backend-inline / scorer-default` |
| `harness-sdk` | 汇总（见 §2.1） |

---

## 3. 典型 Profile 组合

### 3.1 Desktop CLI 最小

```toml
octopus-harness-sdk = { version = "1", features = [
    "sqlite-store",
    "jsonl-store",
    "local-sandbox",
    "interactive-permission",
    "mcp-stdio",
    "provider-anthropic",
] }
```

约 8 MB 二进制（含 Anthropic 客户端）。

### 3.2 Server 生产（多 Provider + OTel）

```toml
octopus-harness-sdk = { version = "1", features = [
    "sqlite-store",
    "jsonl-store",
    "local-sandbox",
    "docker-sandbox",
    "stream-permission",
    "rule-engine-permission",
    "mcp-stdio",
    "mcp-http",
    "mcp-server-adapter",
    "provider-openai",
    "provider-anthropic",
    "provider-gemini",
    "agents-subagent",
    "agents-team",
    "observability-otel",
    "observability-replay",
    "plugin-manifest-sign",
] }
```

### 3.3 CI / 批处理

```toml
octopus-harness-sdk = { version = "1", features = [
    "sqlite-store",
    "local-sandbox",
    "docker-sandbox",
    "rule-engine-permission",
    "mcp-stdio",
    "provider-anthropic",
] }
# 使用 PermissionMode::DontAsk 跑完所有 tool 调用
```

### 3.4 单元测试

```toml
[dev-dependencies]
octopus-harness-sdk = { version = "1", features = ["testing"] }
```

### 3.5 Default 集合说明

默认 `default = [sqlite-store, jsonl-store, local-sandbox, interactive-permission, mcp-stdio, provider-anthropic]` 的选取原则与理由：

| Feature | 理由 |
|---|---|
| `sqlite-store` | 单机默认最佳持久化；FTS5 检索、WAL 并发、容器化部署无额外依赖 |
| `jsonl-store` | 对齐 Octopus 仓库既有 `runtime/events/*.jsonl` 治理规则；支持双写 / 外部工具消费 |
| `local-sandbox` | 零额外依赖、启动最快；Docker/SSH 按需启用 |
| `interactive-permission` | CLI / Desktop 场景的默认交互模式；Stream/Rule/Auto 按需启用 |
| `mcp-stdio` | MCP 最通用 transport；http/ws 按需启用 |
| `provider-anthropic` | 默认启用 Provider：Anthropic 对 Prompt Cache `system_and_3` 模式支持最完整，与 ADR-003 对齐度最高；默认启用不代表 Provider 支持范围 |
| `tool-search` | ADR-009 默认开启：主流场景下 MCP 工具数量不可控，开启 Tool Search 可让 `DeferPolicy::AutoDefer` 工具不破坏 Prompt Cache；关闭时全部降级为 AlwaysLoad |
| `steering-queue` | ADR-0017 默认开启：软引导是普适的"插话"语义，关闭后业务侧需自管 buffer 与硬中断的语义边界；开启时若不调 `push_steering` 则零开销 |

**不默认开的原因**：

- `programmatic-tool-calling`：ADR-0016 默认 **off**；M1 期由业务显式启用、灰度评估后改 default-on（需配合系统提示模板更新）
- `provider-openai / provider-gemini / provider-openrouter / provider-bedrock / provider-codex / provider-local-llama / provider-deepseek / provider-minimax / provider-qwen / provider-doubao / provider-zhipu / provider-km`：均为可用实现，但默认不开；原因是真实 HTTP 依赖、认证方式、区域可用性和模型价格策略差异较大，业务 profile 必须显式选择
- `docker-sandbox / ssh-sandbox`：外部依赖（docker CLI / ssh）不是必然存在
- `stream-permission / rule-engine-permission`：仅 Server / CI 需要，默认 CLI 不需要
- `agents-team / agents-subagent`：高级特性，默认 off 减少编译体积
- `observability-otel / observability-prometheus`：观测后端外部依赖重，按需启用
- `plugin-dynamic-load / plugin-manifest-sign`：高级/安全特性

**业务层应根据 profile 显式启用**（参见 §3.1-§3.4）。

---

## 4. Feature 交互规则

### 4.1 互斥（Mutually Exclusive）

**当前无强制互斥**。以下组合需谨慎：

| 组合 | 建议 |
|---|---|
| `interactive-permission` + `stream-permission` | 业务侧只能选一个 Broker 实例；同时编译不冲突 |
| 所有 `sandbox-*` | 可全开，运行时按 `SandboxPolicy::mode`（`harness-contracts.md §3.4`）选择实现 |
| 所有 `mcp-*` transport | 可全开，运行时按 `McpServerSpec::transport` 选 |

### 4.2 依赖（Required By）

| Feature | 依赖 |
|---|---|
| `mcp-server-adapter` | 至少一个 `mcp-*` transport |
| `agents-team` | `agents-subagent`（Team 成员可 spawn subagent） |
| `plugin-manifest-sign` | `plugin-dynamic-load` |
| `tool-search` | 至少一个 `tool-loading-*` backend（ADR-009 · §2.4） |
| `tool-loading-anthropic` | 与 `provider-anthropic` 搭配才有实际效果（其它 provider 的 model.supports_tool_reference = false 会自动 fallback 到 inline） |
| `programmatic-tool-calling` | `octopus-harness-tool/programmatic-tool-calling` + `octopus-harness-sandbox/code-runtime`（ADR-0016 §2.7） |
| `steering-queue` | `octopus-harness-session/steering` + `octopus-harness-engine/steering`（ADR-0017 §2.5） |

### 4.3 隐式激活（禁止）

不允许 feature A 自动激活 feature B（除非 B 是 A 的强依赖）。如业务需要，显式 `features = ["A", "B"]`。

---

## 5. 条件编译范式

### 5.1 crate 内部

```rust
// harness-model/src/lib.rs
#[cfg(feature = "openai")]
pub mod openai;

#[cfg(feature = "anthropic")]
pub mod anthropic;

#[cfg(feature = "gemini")]
pub mod gemini;

#[cfg(feature = "openrouter")]
pub mod openrouter;

#[cfg(feature = "bedrock")]
pub mod bedrock;

#[cfg(feature = "codex")]
pub mod codex;

#[cfg(feature = "local-llama")]
pub mod local_llama;

#[cfg(feature = "deepseek")]
pub mod deepseek;

#[cfg(feature = "minimax")]
pub mod minimax;

#[cfg(feature = "qwen")]
pub mod qwen;

#[cfg(feature = "doubao")]
pub mod doubao;

#[cfg(feature = "zhipu")]
pub mod zhipu;

#[cfg(feature = "km")]
pub mod km;
```

### 5.2 builtin re-export（harness-sdk）

```rust
// harness-sdk/src/builtin.rs
#[cfg(feature = "provider-anthropic")]
pub use octopus_harness_model::anthropic::AnthropicProvider;

#[cfg(feature = "provider-openai")]
pub use octopus_harness_model::openai::OpenAiProvider;

#[cfg(feature = "provider-gemini")]
pub use octopus_harness_model::gemini::GeminiProvider;

#[cfg(feature = "provider-openrouter")]
pub use octopus_harness_model::openrouter::OpenRouterProvider;

#[cfg(feature = "provider-bedrock")]
pub use octopus_harness_model::bedrock::BedrockProvider;

#[cfg(feature = "provider-codex")]
pub use octopus_harness_model::codex::CodexResponsesProvider;

#[cfg(feature = "provider-local-llama")]
pub use octopus_harness_model::local_llama::LocalLlamaProvider;

#[cfg(feature = "provider-deepseek")]
pub use octopus_harness_model::deepseek::DeepSeekProvider;

#[cfg(feature = "provider-minimax")]
pub use octopus_harness_model::minimax::MinimaxProvider;

#[cfg(feature = "provider-qwen")]
pub use octopus_harness_model::qwen::QwenProvider;

#[cfg(feature = "provider-doubao")]
pub use octopus_harness_model::doubao::DoubaoProvider;

#[cfg(feature = "provider-zhipu")]
pub use octopus_harness_model::zhipu::ZhipuProvider;

#[cfg(feature = "provider-km")]
pub use octopus_harness_model::km::KmProvider;
```

所有 `provider-*` feature 对应的可用 Provider 都必须在 `builtin` 中 gated re-export。`builtin` 不得 re-export 未通过 contract test 的 Provider。

### 5.3 测试专用

```rust
// harness-sdk/src/testing.rs
#[cfg(feature = "testing")]
pub use octopus_harness_journal::memory::InMemoryEventStore;

#[cfg(feature = "testing")]
pub use octopus_harness_sandbox::noop::NoopSandbox;
```

---

## 6. 构建矩阵（CI）

### 6.1 必测组合

```yaml
matrix:
  - default
  - sqlite-store,local-sandbox,interactive-permission,provider-anthropic
  - all-providers  # 所有内置 Provider 可用实现
  - server-profile
  - testing
```

每组合跑 `cargo check / test / clippy`。

### 6.2 构建时间预算

| 组合 | 预算（冷缓存） |
|---|---|
| `default` | < 120s |
| 全 provider | < 240s |
| `testing` | < 180s |

超预算时必须精简依赖或拆分 feature。

---

## 7. 版本兼容

| 行为 | 是否属于 SemVer Major Bump |
|---|---|
| 新增 feature | 否（minor） |
| 重命名 feature | 是（major） |
| 改变 feature 默认集合 | 是（major） |
| 修改 feature 对应的 API | 依据该 API 变动性质 |

---

## 8. 业务层 Tips

### 8.1 按需开启

business 侧仅启用自己会用到的 feature，减少攻击面与编译时间：

```toml
# Anthropic profile + 本地沙箱 + SQLite
features = ["provider-anthropic", "local-sandbox", "sqlite-store", "mcp-stdio"]

# 启用全部内置 Provider
features = ["all-providers", "local-sandbox", "sqlite-store", "mcp-stdio"]
```

### 8.2 feature 查询

运行期可查询 harness 启用了哪些 feature：

```rust
let enabled = harness.enabled_features();
assert!(enabled.contains("provider-anthropic"));
```

### 8.3 feature-gated 分支

```rust
#[cfg(feature = "docker-sandbox")]
let sandbox = DockerSandbox::new(...);

#[cfg(not(feature = "docker-sandbox"))]
let sandbox = LocalSandbox::default();
```

---

## 9. 反模式

| 反模式 | 原因 |
|---|---|
| 把业务 feature 放进 SDK（如 `feature = "my-product"`） | 违反 P1 内核纯净 |
| 用 feature 做运行期 A/B 测试开关 | feature 是编译期；用配置做运行期 |
| feature 之间互抛 `cfg(not(feature = "x"))` 错误 | 应在 feature 组合校验阶段 `compile_error!` |

---

## 10. 未来扩展位

以下 feature **已预留**但暂未实现（占位）：

| 预留 feature | 描述 |
|---|---|
| `plugin-wasm-runtime` | 用 WASM 加载不可信 Plugin |
| `store-postgres` | PostgreSQL EventStore |
| `store-kafka` | Kafka EventStore（流式） |
| `sandbox-kubernetes` | Kubernetes Pod 沙箱 |
| `model-local-gguf` | llama.cpp GGUF 直接加载 |
| `observability-prometheus-push` | Prometheus Pushgateway |

新增时更新本文档 + 对应 crate SPEC。

---

## 11. 索引

- **分层依赖** → `module-boundaries.md`
- **对外 API** → `crates/harness-sdk.md`
- **ADR-008** crate 命名策略 → `adr/0008-crate-layout.md`
- **ADR-009** Deferred Tool Loading → `adr/0009-deferred-tool-loading.md` + `crates/harness-tool-search.md`
