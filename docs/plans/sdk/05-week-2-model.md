# W2 · `octopus-sdk-model`

> 本文档遵循 `docs/plans/sdk/AGENTS.md` 与 `docs/plans/PLAN_TEMPLATE.md`；执行规约见 `docs/plans/sdk/01-ai-execution-protocol.md`。
>
> 阅读顺序：**本文件 →** `docs/sdk/11-model-system.md` → `02-crate-topology.md §2.3 / §5 / §8` → `03-legacy-retirement.md §5 + §6.1（Model Runtime 行）+ §7.6`。

## Goal

产出 **一个零业务语义的新 crate**——`crates/octopus-sdk-model`（Level 1），落地 `docs/sdk/11` 的 **Provider / Surface / Model 三层对象**、**5 种 `ProtocolAdapter`**（本周至少交付 `anthropic_messages` + `openai_chat` 两种能跑通的实现，其余三种先以 stub 占位并保留扩展点）、**`RoleRouter`**（覆盖 `main / fast / best / plan / compact` 五角色）与 **`FallbackPolicy`**（overloaded / 5xx / prompt_too_long），使之成为 W3+ 所有 SDK crate 与业务侧调用大模型的唯一通道。

## Architecture

- **Level 1 · model**：只引用 Level 0 `octopus-sdk-contracts`；允许 `reqwest / tokio / futures`；禁止引用 Level 2+ 任意 crate；禁止 `rusqlite`（目录缓存写入走业务侧注入的 `Arc<dyn ModelCatalogStore>` 扩展点，本周不落实现）。
- **三层对象 = 数据**：`Provider / Surface / Model` 均为纯数据（`#[derive(Clone, Serialize, Deserialize)]`），不含 I/O；I/O 全在 `trait ModelProvider` 的默认实现 `DefaultModelProvider` 内。
- **ProtocolAdapter 负责协议翻译**：`ModelRequest`（canonical IR）↔ vendor request/response schema；每家一个 adapter，共用 `reqwest::Client` 与 SSE 解析器。
- **RoleRouter / FallbackPolicy 静态优先**：遵循 `00-overview.md §1 取舍 #9`（Model 路由保守化）——Smart Routing 作为 plugin 不入核心；W2 仅静态声明式映射 + 多级 fallback。
- **Catalog 静态默认**：built-in snapshot 从 `docs/references/vendor-matrix.md` 派生，作为 `const` / `include_str!` 静态数据入编；**不在运行时联网同步**（遵循 `docs/sdk/11 §11.4.2`）。
- **并行保留**：`crates/api`、`crates/octopus-runtime-adapter::model_runtime::*`、`crates/octopus-model-policy` 本周 **不删不改**，作为双轨对照源；W7 统一下线。
- **凭据**：本周只支持 `AuthKind::ApiKey` / `XApiKey`（通过注入的 `Arc<dyn SecretVault>` 解引用）；`OAuth / AwsSigV4 / GcpAdc / AzureAd` 先定义枚举，运行时返回 `ModelError::AuthUnsupported { kind }`，W4 凭据零暴露合约与 OAuth 细节一起落地。

## Scope

- In scope：
  - 新建 `crates/octopus-sdk-model/` crate 骨架（`Cargo.toml` / `src/lib.rs` / `tests/`）。
  - `02 §2.3` 全部数据符号：`ProviderId / SurfaceId / ModelId / Provider / Surface / Model / ProtocolFamily / ModelTrack / AuthKind / ModelRole`。
  - `ModelRequest / ModelStream / ModelError / ProviderDescriptor / CacheBreakpoint` 的最小签名（`CacheBreakpoint` 复用 W1 `octopus-sdk-contracts::CacheBreakpoint`）。
  - **`octopus-sdk-contracts` 下沉新增 `ToolSchema`**（W1 未登记，B3 审计后追补）：`ToolSchema { name, description, input_schema }` 定义在 Level 0 contracts，`octopus-sdk-model` 与 W3 `octopus-sdk-tools` 均 re-export 该类型，避免双源。
  - `trait ModelProvider` + 默认实现 `DefaultModelProvider { catalog, adapters, http_client, secret_vault }`。
  - `trait ProtocolAdapter` + 两个 full impl：`AnthropicMessagesAdapter` / `OpenAiChatAdapter`；其余三家（`OpenAiResponsesAdapter / GeminiNativeAdapter / VendorNativeAdapter`）先落 `to_request` 返回 `ModelError::AdapterNotImplemented` 的 stub（保留文件、保留符号）。
  - `RoleRouter` 覆盖 5 角色（`main / fast / best / plan / compact`）；`FallbackPolicy` 识别 `overloaded / 5xx / prompt_too_long` 三类触发。
  - `ModelCatalog`（只读）：`list_providers / list_models / resolve / canonicalize`；built-in snapshot 来自 `vendor-matrix.md` 静态派生。
  - **Prompt Cache 基线测试**：对 `AnthropicMessagesAdapter` 的 mock 测试，3 次连续调用 `cache_read_input_tokens` 单调递增；工具顺序/system prompt 分段稳定性守护测试。
  - 与 `contracts/openapi/src/**` 的字段差异登记回 `02 §5`。
- Out of scope：
  - `octopus-runtime-adapter::model_runtime::*` 的旧代码删除（W7）。
  - `crates/octopus-model-policy` crate 目录删除（W7）；本周仅把其 ~143 行"角色 → 家族"默认映射**内容**拷贝进 `octopus-sdk-model::role_router` 的 built-in defaults。
  - OAuth / Bedrock / Vertex / Foundry 认证的 runtime 实现（本周仅保留枚举 + error 返回）。
  - Smart Routing（plugin）、Provider Routing（openrouter）、Auxiliary Models、Local Servers、Remote Catalog 同步（均非 W2 出口）。
  - `ModelCatalogStore`（覆写层持久化）实现；本周只定义 trait 形状，默认实现为 `NullCatalogStore`。
  - 完整 5 种 adapter 的生产级实现（本周 2 家 full + 3 家 stub 即满足 `00-overview.md §3 W2` 出口"至少两个 ProtocolAdapter"）。
  - 与 Session 的双写对接（W6 Brain Loop 统一落）。

## Risks Or Open Questions

| # | 风险 / 问题 | 决策建议 | 触发 Stop Condition |
|---|---|---|---|
| R1 | Prompt cache 命中率守护：W2 首次面对 `docs/sdk/11 §11.12` 稳定性要求；若 tool 顺序、system 分段、历史前缀任一变动都会击穿。 | 在 `AnthropicMessagesAdapter` 内把 tools 的写入顺序固定为"`catalog.list_models(..).tools_fingerprint` + 确定性 sort"；mock 测试 3 次连续调用 `cache_read_input_tokens` 必须单调 ≥ 上一轮（mock 值在 fixture 中递增即可）。 | 命中率 < 80% → Stop #4 |
| R2 | 与 `contracts/openapi/src/**` 字段差异：vendor-matrix 的 `SurfaceDefinition.protocolFamily` 值集合（`openai_chat / openai_responses / anthropic_messages / gemini_native / vendor_native`）与 OpenAPI 当前 `RuntimeModelSurface`（若存在）命名约定可能冲突。 | 本周**不改 OpenAPI**；逐项登记 `02 §5`，W7 adapter 下线时 platform 决定 upstream / downstream。 | 若差异涉及 `role` 枚举值 → Stop #1 |
| R3 | `crates/octopus-runtime-adapter::model_runtime::drivers/*`（4 文件 ~760 行）已有雏形（`03 §5.1` 说明"优先复用作为起点"）。直接复用 vs 从 `docs/sdk/11` 规范重写的平衡。 | 先读 4 drivers 做"形状映射"，保留方法分组（`to_request / parse_stream / auth_headers`）；但**不直接拷贝**包含旧 `capability_runtime` / adapter 私有类型的代码；重写时以 `ModelRequest` canonical IR 为边界。 | 若 4 drivers 的流式解析含 bug 需要再调试才能迁 → Stop #7 |
| R4 | `crates/api` / `crates/octopus-runtime-adapter` 本周禁止改动（并行保留原则），但 W2 新代码若与旧 `api::providers::anthropic::AnthropicClient` 的 cache fingerprint 不一致 → 可能导致未来双轨对账失败。 | 在 `AnthropicMessagesAdapter::to_request` 内记录 `tools_fingerprint = sha256(name1\tname2\t...)`；该值作为事件 payload（W6 观测层）比对 legacy。本周仅暴露 `fn tools_fingerprint(&self, tools: &[ToolSchema]) -> String` 供单元测试断言。 | 若 legacy 侧 fingerprint 无法取得 → Stop #9（legacy 隐式依赖） |
| R5 | `docs/references/vendor-matrix.md`（193 行）是事实源，但 SDK 侧把它 `include_str!` 进来会让文档成为构建依赖。 | 不直接 `include_str!` markdown；在 `src/catalog/builtin.rs` 中以**手写 Rust 数组**派生，注释中对齐 `vendor-matrix.md` 的 `last_verified_at`；CI 加断言"built-in 覆盖的 provider 集合 ⊇ vendor-matrix §1 总览的 8 厂商"。 | 若 8 厂商中某家 protocol_family 在本周无法枚举（如 MiniMax native 暂不纳入 5 family） → 该 provider 标记 `ModelTrack::Preview` 且 `adapters` 为空，在 `02 §5` 登记 | — |
| R6 | Workspace `default-members` 在 W1 已追加 `crates/octopus-sdk-contracts / crates/octopus-sdk-session`。W2 是否继续追加 `crates/octopus-sdk-model`？ | 追加。遵循 W1 Task 1 Step 2 的先例；正式"5 业务 crate 收敛"在 W7/W8。 | 若发现已有 `default-members` 约束禁止 SDK crate → Stop #8 |

## 本周 `02 §2.1 / §2.3` 公共面修订清单（同批次回填）

> 以下 14 处签名修订必须在 Task 2 / Task 3 / Task 5 / Task 7 合入批次内**同 PR** 回填到 `02-crate-topology.md`，否则视为 `Stop Condition #1` 裸增。

| # | 位置 | 修订类型 | 内容 |
|---|---|---|---|
| 1 | `02 §2.3` `Provider` | 字段追加 | 新增 `display_name: String`、`status: ProviderStatus` |
| 2 | `02 §2.3` 新增类型 | 类型新增 | `ProviderStatus { Active, Deprecated, Experimental }` |
| 3 | `02 §2.3` `Surface` | 字段追加 | 新增 `provider_id: ProviderId`、`auth: AuthKind` |
| 4 | `02 §2.3` `Model` | 字段追加 | 新增 `context_window: ContextWindow`、`aliases: Vec<String>` |
| 5 | `02 §2.3` 新增类型 | 类型新增 | `ContextWindow { max_input_tokens, max_output_tokens, supports_1m }` |
| 6 | `02 §2.3` `ModelRequest` | 字段追加 | 新增 `response_format / thinking / cache_control / max_tokens / temperature / stream` |
| 7 | `02 §2.3` 新增类型 | 类型新增 | `ResponseFormat { Json { schema }, Text }` |
| 8 | `02 §2.3` 新增类型 | 类型新增 | `ThinkingConfig { enabled, budget_tokens }` |
| 9 | `02 §2.3` 新增类型 | 类型新增 | `CacheControlStrategy { None, PromptCaching { breakpoints }, ContextCacheObject { cache_id } }` |
| 10 | `02 §2.3` `ModelError` | 变体表展开 | 11 个变体（见 Task 3 Step 2） |
| 11 | `02 §2.3` `ProtocolAdapter` | 方法追加 | `async fn auth_headers(&self, vault, provider) -> Result<Vec<(HeaderName, HeaderValue)>, ModelError>` |
| 12 | `02 §2.1` 新增类型（下沉） | 类型新增 | `ToolSchema { name, description, input_schema }` 定义在 **Level 0 contracts**（见 B3 决策） |
| 13 | `02 §2.3` `FallbackPolicy` | 符号展开 | `FallbackTrigger { Overloaded, Upstream5xx, PromptTooLong, ModelDeprecated }` + `fn should_fallback / fn next_model` |
| 14 | `02 §2.3` `DefaultModelProvider` | 符号新增 | `complete_with_fallback`（见 Task 5 Step 3） |

任何额外出现的 `pub` 符号都必须在 Task 9 Step 1 之前追加到本表与 `02 §2.1 / §2.3`，否则 Weekly Gate 阻断。

---

## Execution Rules

- 遵循 `01-ai-execution-protocol.md`：三层 Checklist + Stop Conditions 1–11 全部生效。
- 每个 Task 原子、单 PR ≤ 800 行；违反 → 拆 sub-Task。
- 公共面（`pub` 符号）变动 → 同一 PR 必须更新 `02-crate-topology.md §2.3`；违反 → Stop Condition #1 或本目录 AGENTS.md §5 不一致。
- 任何与 `contracts/openapi/src/**` 的字段差异 → 登记到 `02 §5`，**不**直接改 openapi。
- `crates/api / crates/runtime / crates/octopus-runtime-adapter / crates/octopus-model-policy` 的任何代码本周**禁止改动**。如发现只能改旧代码才能完成 W2 → Stop Condition #8（或 #9 若为隐式依赖）。
- 单文件 ≤ 800 行；`src/lib.rs` ≤ 80 行（仅 `mod` + `pub use`）。
- `default-members` 在本周结束时追加 `crates/octopus-sdk-model`。
- 禁止引入 `rusqlite` 到 `octopus-sdk-model` 的 `[dependencies]`；禁止使用 `env::var` 读取 API key（必须经 `SecretVault`）。

---

## Active Work

- Current task: `W2 not started`
- Current step: `Plan draft 已定稿，等待 W1 收尾 PR 合入后启动 Task 1`
- Execution mode: `batched`

### Pre-Task Checklist（Task 1，启动前勾选）

- [ ] 已阅读本子 Plan 的 `Goal` / `Architecture` / `Scope`。
- [ ] 已阅读 `00-overview.md §1 10 项取舍`（特别是 #9 Model 路由保守化），且当前任务未违反。
- [ ] 已阅读 `docs/sdk/11-model-system.md` §11.2 / §11.3 / §11.4 / §11.6 / §11.7 / §11.9 / §11.12。
- [ ] 已阅读 `02-crate-topology.md §1 依赖图 / §2.3 / §4 / §5`。
- [ ] 已阅读 `03-legacy-retirement.md §5（crates/api）/ §6.1 Model Runtime 行 / §7.6`。
- [ ] 已识别本 Task 涉及的 SDK 对外公共面变更（是 / 否）。
  - 当前判断：`是`（整批 §2.3 公共面本周首次落地）。
- [ ] 已识别是否涉及 `contracts/openapi/src/**` 或 `packages/schema/src/**`。
  - 当前判断：`否`（差异走 `02 §5` 登记，不改 OpenAPI）。
- [ ] 已识别是否涉及 `docs/sdk/14` UI Intent IR 变更（是 / 否）。
  - 当前判断：`否`。
- [ ] Preconditions 已全部满足；未满足项已在 `Open Questions` 中登记。
- [ ] 当前 git 工作树干净或有明确切分；本批次计划 diff ≤ 800 行（不含 generated）。
- [ ] 已识别所有 `Stop if:` 条款；遇到任一条件 → 立即停止并汇报。

---

## Task Ledger

### Task 1：crate 骨架 + workspace 登记

Status: `pending`

Files:
- Create: `crates/octopus-sdk-model/Cargo.toml`
- Create: `crates/octopus-sdk-model/src/lib.rs`
- Modify: `Cargo.toml`（workspace `default-members` 追加 `"crates/octopus-sdk-model"`）

Preconditions：W1（`04-week-1-contracts-session.md`）状态 `done`；`cargo test -p octopus-sdk-contracts -p octopus-sdk-session` 全绿；`02-crate-topology.md §2.3` 签名与本 Plan Scope 一致。

Step 1：
- Action：创建 `octopus-sdk-model` crate 骨架。`Cargo.toml` `[dependencies]` 允许 `serde / serde_json / thiserror / async-trait / tokio / futures / reqwest / bytes / tracing / sha2 / octopus-sdk-contracts`（路径依赖）；禁止 `rusqlite / tauri / axum / octopus-core / octopus-platform`。`src/lib.rs` 仅含 `mod` 声明与受控 `pub use`，≤ 80 行，先声明 `mod id; mod catalog; mod adapter; mod provider; mod role_router; mod fallback; mod error;`（文件可暂为空 `lib.rs` 之外的 stub）。
- Done when：`cargo build -p octopus-sdk-model` 成功；`wc -l crates/octopus-sdk-model/src/lib.rs` ≤ 80；`rg 'rusqlite|tauri|axum|octopus-core|octopus-platform' crates/octopus-sdk-model/Cargo.toml` 无结果。
- Verify：`cargo build -p octopus-sdk-model && wc -l crates/octopus-sdk-model/src/lib.rs && rg -n '^(rusqlite|tauri|axum|octopus-core|octopus-platform)' crates/octopus-sdk-model/Cargo.toml`
- Stop if：`reqwest` 在 workspace `[workspace.dependencies]` 中的默认 features 与 SDK 需求冲突（例如强制 `native-tls` 而 SDK 要 `rustls`）→ Stop #8（需要人决策是否扩 workspace 依赖）。

Step 2：
- Action：更新 workspace `Cargo.toml` 的 `default-members` 追加 `"crates/octopus-sdk-model"`。
- Done when：`cargo metadata --format-version=1 --no-deps | jq -r '.workspace_default_members[]' | rg 'octopus-sdk-model'` 命中。
- Verify：`cargo build && cargo metadata --format-version=1 --no-deps | jq -r '.workspace_default_members[]' | rg 'octopus-sdk-model'`
- Stop if：若发现 W1 已把 `default-members` 收敛到仅业务 crate（不太可能，但需核对）→ 回退修改，留 `TODO(W7)` 注释并记入本文件变更日志。

Notes：
- crate 目录与包名均为 `octopus-sdk-model`（短横线）。
- 本 Task 不新增 workspace 级依赖版本条目；如需新增 `sha2` / `bytes`，以 `workspace = true` 形式引用已存在项，若不存在则作为本 Task 的 Step 3 单独提交并在 PR 说明锁版本。

---

### Task 2：核心数据签名（IDs / Provider / Surface / Model / Enums）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-model/src/id.rs`
- Create: `crates/octopus-sdk-model/src/provider.rs`（含 `Provider / Surface / Model` 结构体）
- Create: `crates/octopus-sdk-model/src/enums.rs`（含 `ProtocolFamily / ModelTrack / AuthKind / ModelRole`）
- Modify: `crates/octopus-sdk-model/src/lib.rs`（模块声明 + `pub use`）

Preconditions：Task 1 完成。

Step 1：
- Action：落地 `ProviderId / SurfaceId / ModelId`（`#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]`，内部 `String`；不提供 `new_v4`，因为这三类 ID 来自 catalog，不是运行时生成）。落地 `ProtocolFamily { AnthropicMessages, OpenAiChat, OpenAiResponses, GeminiNative, VendorNative }`、`ModelTrack { Preview, Stable, LatestAlias, Deprecated, Sunset }`、`AuthKind { ApiKey, XApiKey, OAuth, AwsSigV4, GcpAdc, AzureAd, None }`、`ModelRole { Main, Fast, Best, Plan, Compact, Vision, WebExtract, Embedding, Eval, SubagentDefault }`。全部使用 `#[serde(rename_all = "snake_case")]`。
- Done when：签名与 `02-crate-topology.md §2.3` 的 `ProviderId / SurfaceId / ModelId / ProtocolFamily / ModelTrack / AuthKind / ModelRole` 段逐字对齐；JSON round-trip 测试：`ModelRole::Main` 序列化为 `"main"`；`ProtocolFamily::AnthropicMessages` 序列化为 `"anthropic_messages"`。
- Verify：`cargo test -p octopus-sdk-model enums::`
- Stop if：与 `docs/sdk/11 §11.6.1` 的标准角色集（11 个）有差异 → 本周以 `02 §2.3` 登记的 10 个公共值为执行基线（`rerank` 暂缓，差异必须同步登记到 `docs/sdk/README.md ## Fact-Fix 勘误`）；若 W2 需要把 `rerank` 纳入公共面，则 Stop #1 并回填 `02 §2.3`。

Step 2：
- Action：落地 `Provider { id: ProviderId, display_name: String, status: ProviderStatus, auth: AuthKind, surfaces: Vec<SurfaceId> }`、`Surface { id: SurfaceId, provider_id: ProviderId, protocol: ProtocolFamily, base_url: String, auth: AuthKind }`、`Model { id: ModelId, surface: SurfaceId, family: String, track: ModelTrack, context_window: ContextWindow, aliases: Vec<String> }`、`ContextWindow { max_input_tokens: u32, max_output_tokens: u32, supports_1m: bool }`、`ProviderStatus { Active, Deprecated, Experimental }`。
- Done when：所有结构体 `#[derive(Debug, Clone, Serialize, Deserialize)]`；字段序列化顺序固定；单元测试 `provider_round_trip` 通过。
- Verify：`cargo test -p octopus-sdk-model provider::`
- Stop if：`Surface` 的 `provider_id` 与 `docs/sdk/11 §11.3.2` 的 `SurfaceDefinition` 未显式包含该字段 → 以 SDK 侧为准（反向索引需要），在 `02 §5` 登记 `surface.provider_id` 差异，处理方式 `align-openapi`。

Notes：
- `Provider.display_name` 取自 `vendor-matrix.md §1 总览` 的厂商中文名（Task 4 catalog 填充）。
- 本 Task 不落 `ProviderDescriptor`（轻量快照类型），移至 Task 5 与 `ModelProvider::describe()` 一起定义。
- 公共面登记：**同批次**把 Task 2 落地的 9 个类型签名核对进 `02 §2.3`；若 §2.3 中某字段缺失 → 同 PR 补 §2.3。

---

### Task 3：Canonical IR + `ProtocolAdapter` trait + 错误类型

Status: `pending`

Files:
- Create: **`crates/octopus-sdk-contracts/src/tool_schema.rs`**（`ToolSchema { name, description, input_schema }`；下沉 Level 0）
- Modify: `crates/octopus-sdk-contracts/src/lib.rs`（`mod tool_schema; pub use tool_schema::ToolSchema;`；行数守约 ≤ 80）
- Create: `crates/octopus-sdk-model/src/request.rs`（`ModelRequest / ResponseFormat / ThinkingConfig / CacheControlStrategy`，**不含** `ToolSchema`，改为 `pub use octopus_sdk_contracts::ToolSchema`）
- Create: `crates/octopus-sdk-model/src/error.rs`（`ModelError`）
- Create: `crates/octopus-sdk-model/src/adapter/mod.rs`（`ProtocolAdapter` trait + `StreamBytes` 别名）
- Modify: `crates/octopus-sdk-model/src/lib.rs`

Preconditions：Task 2 完成；确认 W1 `octopus-sdk-contracts` 无同名 `ToolSchema` 符号（`rg 'struct ToolSchema' crates/octopus-sdk-contracts/src` 无结果）。

Step 1：
- Action：**先在 `octopus-sdk-contracts` 新增 `ToolSchema { name: String, description: String, input_schema: serde_json::Value }`**（`#[derive(Debug, Clone, Serialize, Deserialize)]`，snake_case）；**同批次**回填 `02 §2.1` 新增 `ToolSchema` 签名（见本 Plan §"本周 §2.1 / §2.3 公共面修订清单" 第 12 行）。随后在 `octopus-sdk-model/src/request.rs` 落地 `ModelRequest`（按 `02 §2.3` 的 `ModelRequest / ResponseFormat / ThinkingConfig / CacheControlStrategy` 签名，`messages: Vec<Message>` 复用 `octopus_sdk_contracts::Message`；`cache_breakpoints: Vec<CacheBreakpoint>` 复用 contracts `CacheBreakpoint`；`tools: Vec<ToolSchema>` 通过 `pub use octopus_sdk_contracts::ToolSchema` re-export；新增 `response_format: Option<ResponseFormat> / thinking: Option<ThinkingConfig> / cache_control: CacheControlStrategy / max_tokens: Option<u32> / temperature: Option<f32> / stream: bool` 字段）。落地 `ResponseFormat { Json { schema: serde_json::Value }, Text }`、`ThinkingConfig { enabled: bool, budget_tokens: Option<u32> }`、`CacheControlStrategy { None, PromptCaching { breakpoints: Vec<&'static str> }, ContextCacheObject { cache_id: String } }`。
- Done when：`ModelRequest::tools_fingerprint(&self) -> String` 方法落地（SHA-256 hex，输入 = `self.tools` 的 `name` 按原顺序 `\t` 拼接），供 prompt cache 稳定性测试使用；`rg 'pub use octopus_sdk_contracts::ToolSchema' crates/octopus-sdk-model/src/request.rs` 命中；`rg 'struct ToolSchema' crates/octopus-sdk-contracts/src/tool_schema.rs` 命中；`rg 'struct ToolSchema' crates/octopus-sdk-model/src/` 无结果（禁止 model 重复定义）；`02 §2.3` 已同批补入 `ModelRequest` 新字段与 `ResponseFormat / ThinkingConfig / CacheControlStrategy` 签名。
- Verify：`cargo test -p octopus-sdk-contracts tool_schema:: && cargo test -p octopus-sdk-model request::`
- Stop if：`octopus-sdk-contracts` 已有 `ToolSchema` 且签名不同 → Stop #1（规范冲突，W1 契约变更）；若 W3 `octopus-sdk-tools::ToolSpec` 已 PR in-flight 计划不走 Level 0 → Stop #8（跨周契约歧义）。

Step 2：
- Action：落地 `ModelError`（`thiserror`）：`AuthUnsupported { kind: AuthKind } / AuthMissing { provider: ProviderId } / UpstreamStatus { status: u16, body_preview: String } / UpstreamTimeout / Overloaded { retry_after_ms: Option<u64> } / PromptTooLong { estimated_tokens: u32, max: u32 } / AdapterNotImplemented { family: ProtocolFamily } / CapabilityUnsupported { capability: String, model: ModelId } / Serialization(#[from] serde_json::Error) / Transport(#[from] reqwest::Error) / ModelNotFound { id: ModelId }`。
- Done when：`ModelError` 可被 `Display` / `Error` / `Send + Sync`；单元测试覆盖 `Overloaded` 到 `FallbackPolicy::should_fallback` 的触发（Task 7 再测，本 Task 只保证类型编译通过）；**同批次把 11 变体展开回填到 `02 §2.3` `ModelError` 段**（见本 Plan §"本周 §2.1 / §2.3 公共面修订清单" 第 6 行）。
- Verify：`cargo test -p octopus-sdk-model error:: && rg -n 'AdapterNotImplemented|CapabilityUnsupported|Overloaded' docs/plans/sdk/02-crate-topology.md`
- Stop if：需要把 `AppError`（`octopus-core`）纳入 SDK → Stop #2（业务域入侵）。

Step 3：
- Action：落地 `trait ProtocolAdapter`（按 `02 §2.3` 的 `ProtocolAdapter` 段签名）：`family / to_request / parse_stream`。新增 `async fn auth_headers(&self, vault: &dyn SecretVault, provider: &Provider) -> Result<Vec<(HeaderName, HeaderValue)>, ModelError>` 以把凭据注入隔离为 trait 方法。`StreamBytes = Pin<Box<dyn Stream<Item = Result<bytes::Bytes, ModelError>> + Send>>`。**同批次回填** `02 §2.3` `ProtocolAdapter` trait 段新增 `auth_headers` 签名（见本 Plan §"本周 §2.1 / §2.3 公共面修订清单" 第 11 行）。
- Done when：trait 可被多实现；`rustc --explain` 无对象安全问题（`parse_stream` / `to_request` 均非 `async fn`；`auth_headers` 用 `async_trait`）；`rg 'fn auth_headers' docs/plans/sdk/02-crate-topology.md` 命中。
- Verify：`cargo check -p octopus-sdk-model --tests && rg -n 'fn auth_headers' docs/plans/sdk/02-crate-topology.md`
- Stop if：`async fn` 与 `dyn ProtocolAdapter` 对象安全冲突 → 拆 `async_trait` + `Box<dyn Future>` 模式，保留 `dyn` 可用。

Notes：
- `CacheControlStrategy::PromptCaching.breakpoints` 的值域固定为 `["system", "tools", "first_user", "last_user"]`（`docs/sdk/11 §11.12.2`），用 `&'static str` 而非 `enum`，避免跨 crate 变更成本。
- 本 Task 预估 ~400 行（含测试）；若超 600 行拆成 Task 3a（request + error）+ Task 3b（adapter trait）。

---

### Task 4：`ModelCatalog` + built-in snapshot（vendor-matrix 派生）

Status: `pending`

Files:
- Create: `crates/octopus-sdk-model/src/catalog/mod.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/mod.rs`（聚合器；`pub fn all_providers() -> Vec<Provider>` 等函数按 provider 聚合调用）
- Create: `crates/octopus-sdk-model/src/catalog/builtin/anthropic.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/openai.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/google.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/deepseek.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/minimax.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/moonshot.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/bigmodel.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/qwen.rs`
- Create: `crates/octopus-sdk-model/src/catalog/builtin/ark.rs`
- Create: `crates/octopus-sdk-model/src/catalog/resolve.rs`
- Create: `crates/octopus-sdk-model/tests/catalog_builtin.rs`

> 目录式拆分（对应 S2 审计）：每个 `builtin/<provider>.rs` 独立管理本家 `Provider / Surface / Model / aliases`；`mod.rs` 仅做聚合，避免单文件突破 800 行硬约束。

Preconditions：Task 2、Task 3 完成。

Step 1：
- Action：落地 `struct ModelCatalog { providers: Vec<Provider>, surfaces: Vec<Surface>, models: Vec<Model>, aliases: HashMap<String, ModelId> }`；方法：`new_builtin() -> Self / list_providers / list_models(filter) / resolve(reference: &str) -> Option<ResolvedModel> / canonicalize(id: &str) -> Option<ModelId>`。`ResolvedModel { provider: Provider, surface: Surface, model: Model }`。
- Done when：`ModelCatalog::new_builtin()` 编译通过；`list_providers().len() >= 8`（对齐 `vendor-matrix.md §1` 的 8 厂商）。
- Verify：`cargo test -p octopus-sdk-model --test catalog_builtin`
- Stop if：某厂商的 `ProtocolFamily` 不在 5 枚举内（如某 vendor 使用完全私有协议且无 `vendor_native` 归类） → 该厂商 `status = Experimental`、`surfaces = []`，登记 `02 §5`。

Step 2：
- Action：填充 `builtin/<provider>.rs` 的静态数据（Rust 数组）。每个厂商至少一个 `Surface`（`conversation` + `protocol_family`）+ 代表性 `Model`。覆盖（按 `vendor-matrix.md`）：`anthropic / openai / google / deepseek / minimax / moonshot / bigmodel / qwen / ark`。至少 `anthropic.claude-opus` 与 `openai.gpt-5.4` 两家有 `aliases: ["opus", "gpt-5"]` 等短名，供 `resolve` 测试。`builtin/mod.rs` 通过 `pub fn all_providers() / all_surfaces() / all_models() / all_aliases()` 聚合调用各 provider 模块。
- Done when：`catalog_builtin.rs` 覆盖至少 9 个 provider_id；`resolve("opus")` 返回 `anthropic` 的 Opus 家族 stable 最新；`canonicalize("anthropic.claude-opus-4-6-v1:0")` 返回 `claude-opus-4-6`（测试 Bedrock 命名归一化）；每个 `builtin/<provider>.rs` 的 `wc -l` ≤ 250。
- Verify：`cargo test -p octopus-sdk-model --test catalog_builtin -- --nocapture && find crates/octopus-sdk-model/src/catalog/builtin -type f -name '*.rs' -exec wc -l {} \; | awk '$1 > 250'`（期望无输出）
- Stop if：`vendor-matrix.md` 中某厂商的 `last_verified_at` 早于 2026-01 → 在对应 `builtin/<provider>.rs` 顶部注释标注 `STALE(YYYY-MM-DD)` 并继续；不阻塞 W2，但在 `00 §6 风险登记簿` append。

Step 3：
- Action：落地 `resolve.rs` 的"引用解析优先级链"（`docs/sdk/11 §11.5`）简化版：`alias → canonical → family → full_id`；`family` 通配选 `ModelTrack::Stable` 且其所属 `Provider.status = Active` 的最新（按同 family 的 release/alias 优先级取最新稳定项）。
- Done when：测试 `resolve("opus")` 与 `resolve("claude-opus-4-6")` 返回同一 `ModelId`；`resolve("unknown/xxx")` 返回 `None`。
- Verify：`cargo test -p octopus-sdk-model catalog::resolve::`
- Stop if：`aliases` 冲突（同一短名指向不同 provider） → 按"近者优先"取第一个注册的，冲突登记 `02 §5`。

Notes：
- `builtin/<provider>.rs` 手写不使用宏生成；默认已按 provider 拆文件（S2 审计结论），避免单文件突破 800 行硬约束。
- `catalog/mod.rs` 暴露 `ModelCatalog` + `new_builtin`，`builtin::*` 保持 `pub(crate)`，不向 crate 外泄露实现细节。
- 本 Task 不落"目录同步"（`octopus models refresh`）；保留为 W5 plugin 扩展点。

---

### Task 5：`ModelProvider` trait + `DefaultModelProvider`

Status: `pending`

Files:
- Create: `crates/octopus-sdk-model/src/provider_impl.rs`（`DefaultModelProvider`）
- Modify: `crates/octopus-sdk-model/src/lib.rs`（导出 `ModelProvider / ProviderDescriptor`）

Preconditions：Task 3、Task 4 完成。

Step 1：
- Action：落地 `trait ModelProvider`（按 `02 §2.3` 的 `ModelProvider / ProviderDescriptor / ModelStream` 签名）：`async fn complete(&self, req: ModelRequest) -> Result<ModelStream, ModelError>` + `fn describe(&self) -> ProviderDescriptor`。`ProviderDescriptor { id: ProviderId, supported_families: Vec<ProtocolFamily>, catalog_version: String }`。`ModelStream = Pin<Box<dyn Stream<Item = Result<AssistantEvent, ModelError>> + Send>>`（`AssistantEvent` 来自 `octopus-sdk-contracts`）。
- Done when：`trait ModelProvider` 对象安全（`async_trait`）；`ProviderDescriptor` 可序列化。
- Verify：`cargo check -p octopus-sdk-model`
- Stop if：`AssistantEvent` 签名与 W1 contracts 不匹配（例如缺 `PromptCache` 变体） → 回 W1 补契约，Stop #8。

Step 2：
- Action：落地 `DefaultModelProvider { catalog: Arc<ModelCatalog>, adapters: HashMap<ProtocolFamily, Arc<dyn ProtocolAdapter>>, http_client: reqwest::Client, secret_vault: Arc<dyn SecretVault> }`；`impl ModelProvider for DefaultModelProvider`：
  1. `resolve(&req.model)` → `ResolvedModel`
  2. `adapter = adapters[surface.protocol]` → 否则 `ModelError::AdapterNotImplemented`
  3. `adapter.auth_headers(vault, provider)` → 构造请求
  4. `adapter.to_request(&req)` → `reqwest::RequestBuilder` → 发送
  5. 返回 `adapter.parse_stream(raw_bytes_stream)`
- Done when：单元测试 `default_provider_returns_adapter_not_implemented_for_gemini` 通过（只注册 anthropic + openai adapter 时，请求 gemini model 返回 `AdapterNotImplemented`）。
- Verify：`cargo test -p octopus-sdk-model provider_impl::`
- Stop if：`reqwest::Client` 的代理 / TLS 配置需要业务侧注入 → 在 `DefaultModelProvider::builder()` 暴露 `with_http_client(Arc<reqwest::Client>)` 可选注入，保留默认构造。

Step 3（fallback 包装器；对应 B4 审计 + `00 §3 W2 出口状态` 的 FallbackPolicy 覆盖）：
- Action：在 `DefaultModelProvider` 上新增公开方法 `async fn complete_with_fallback(&self, req: ModelRequest, policy: &FallbackPolicy) -> Result<ModelStream, ModelError>`。语义：首次调用 `self.complete(req.clone())`；若返回 `Err(e)` 且 `policy.should_fallback(&e).is_some()`，取 `policy.next_model(&req.model)`（若为 `None` 则直接上抛），构造 `req2 = ModelRequest { model: next, ..req }`，再调用一次 `self.complete(req2)`；第二次失败或无 `next_model` 时透传错误。本周**最多一次**二级重试，不做指数退避。**同批次回填 `02 §2.3`**：新增 `DefaultModelProvider::complete_with_fallback` 公开签名（见本 Plan §"本周 §2.1 / §2.3 公共面修订清单" 第 10 行）。
- Done when：单元测试 `fallback_triggers_on_overloaded_then_succeeds` 通过（首次返回 `Overloaded`、策略指向下一个模型、二次返回流式成功）；`fallback_does_not_trigger_on_unrelated_error` 通过（`AdapterNotImplemented` 不触发 fallback）；`fallback_exhausted_after_one_retry` 通过（`next_model` 返回 `None` 或二次仍失败时不做第三次）。
- Verify：`cargo test -p octopus-sdk-model provider_impl::fallback`
- Stop if：需要指数退避 / 链式多跳 → 延至 W6 `FallbackOrchestrator`，本 Task 保留单次语义；若 `FallbackPolicy::next_model` 接口需要 `ResolvedModel` 而非 `ModelId` → 回到 Task 7 Step 2 调整签名，Stop #1 公共面修订。

Notes：
- 本 Task 不做真实网络请求测试；所有测试用 `wiremock` 或手写 `MockTransport`（W2 用 mock 即可，真实联调放 W6 E2E）。`wiremock / tokio-test` 为 `[dev-dependencies]`，不进生产依赖树。
- `SecretVault` trait 必须来自 `octopus-sdk-contracts`（W1 已定义）；若未定义 → Stop #8。
- Step 3 的 fallback 包装是 "single-shot retry"：语义清晰但不处理 canary / 抖动 / 熔断（`00 §3 W2 出口状态` 只要求覆盖触发识别 + 单次切换，不要求完整链式）。

---

### Task 6：`AnthropicMessagesAdapter` + `OpenAiChatAdapter`

Status: `pending`

Files:
- Create: `crates/octopus-sdk-model/src/adapter/anthropic_messages.rs`
- Create: `crates/octopus-sdk-model/src/adapter/openai_chat.rs`
- Create: `crates/octopus-sdk-model/src/adapter/sse.rs`（共用 SSE parser）
- Create: `crates/octopus-sdk-model/src/adapter/stubs.rs`（`OpenAiResponsesAdapter / GeminiNativeAdapter / VendorNativeAdapter` 三个 stub）
- Create: `crates/octopus-sdk-model/tests/adapter_anthropic.rs`
- Create: `crates/octopus-sdk-model/tests/adapter_openai.rs`

Preconditions：Task 3、Task 5 完成。`crates/api/src/providers/{anthropic.rs, openai_compat.rs}` 与 `crates/octopus-runtime-adapter/src/model_runtime/drivers/*` 作为**只读参考源**（不修改）。**Pre-Task 核对 `AssistantEvent` 变体清单**：
- 执行 `rg -n 'pub enum AssistantEvent' crates/octopus-sdk-contracts/src/`，记录命中位置；再读该文件 enum 定义段并把变体逐个列进本 Plan Task 6 Notes；
- 本 Task 以 **W1 当前 contracts 实际公共面** 为真相源：`TextDelta / ToolUse / Usage / PromptCache / MessageStop`；
- 若执行前 W1 contracts 已扩成更细粒度流式事件 → 先同批回填 `02 §2.1 / §2.3` 与本 Task Notes，再启动 Step 2/3；
- 若当前 contracts 缺失 `ToolUse`、`Usage` 或 `MessageStop` 任一必要变体 → Stop #8（跨周契约断裂），回 W1 补 contracts。

**PR 边界约定**（对应 S1 审计）：Task 6 的 4 个 Step 按 "1 Step = 1 PR" 合入，每个 PR 独立通过 CI；顺序严格串行：Step 1 → Step 2 → Step 3 → Step 4；禁止 Step 2 与 Step 3 并行（SSE 基础设施依赖先就位）。

Step 1：
- Action：落地 `sse.rs`：`IncrementalSseParser`（行缓冲 + `data:` / `event:` 字段）；返回 `Stream<Item = Result<SseEvent, ModelError>>`。不从 `crates/runtime/src/sse.rs` 拷贝代码；重写以契合 `ModelError`。
- Done when：单元测试覆盖 (a) 正常多事件 (b) 跨 chunk 切断 (c) `[DONE]` 哨兵。
- Verify：`cargo test -p octopus-sdk-model adapter::sse::`
- Stop if：SSE 边界处理与 Anthropic / OpenAI 任一家不一致 → 记 `02 §5`，保留 `vendor_sse_quirks` 配置项（W3+ 填充）。

Step 2：
- Action：落地 `AnthropicMessagesAdapter`。`to_request`：组装 `{model, messages: canonical→anthropic blocks, tools: [...], system: [...], max_tokens, stream: true, cache_control}`；确保 `tools` 按 `ModelRequest::tools` 原顺序写入（稳定排序在 Task 7 再做全局保障）。`parse_stream`：识别 Anthropic SSE 事件（`message_start / content_block_start / content_block_delta / content_block_stop / message_delta / message_stop`）→ 映射为 **当前 contracts 公共面** `AssistantEvent::{TextDelta / ToolUse / Usage / PromptCache / MessageStop}`；其中 tool-use 相关的 start/delta/stop 事件先在 adapter 内缓冲，待拿到完整 `id + name + input` 后一次性发出 `AssistantEvent::ToolUse`，`message_stop` 归一化到 `AssistantEvent::MessageStop { stop_reason }`。`auth_headers`：`x-api-key: {secret_vault.get("anthropic_api_key")}` + `anthropic-version: 2023-06-01` + `anthropic-beta: prompt-caching-2024-07-31`（若 `cache_control != None`）。
- Done when：`tests/adapter_anthropic.rs` 通过 3 组测试：(a) 非 stream 单 text turn，(b) tool_use turn，(c) prompt cache 触发（`cache_creation_input_tokens > 0`）。用 `wiremock` 模拟 HTTP。
- Verify：`cargo test -p octopus-sdk-model --test adapter_anthropic`
- Stop if：Anthropic 最新 SSE 事件名与旧 `crates/api/src/providers/anthropic.rs` 不一致且携带**当前 contracts 无法表达的新必要语义**（例如必须保真的 `reasoning_delta`）→ Stop #8，登记 `02 §5` + `docs/sdk/README.md ## Fact-Fix 勘误`；若仅为可忽略元数据，则记录到 Task 6 Notes 并保持现有映射。

Step 3：
- Action：落地 `OpenAiChatAdapter`。`to_request`：`POST /v1/chat/completions` schema；`messages: canonical→openai chat`；`tools: [{type:"function", function:{...}}]`；`tool_choice`；`stream: true`。`parse_stream`：识别 `choices[].delta.{content, tool_calls, finish_reason}`，将 `content` 归一化为 `AssistantEvent::TextDelta`，将流式 `tool_calls` 片段聚合为单个 `AssistantEvent::ToolUse { id, name, input }`，将 `finish_reason` 归一化为 `AssistantEvent::MessageStop { stop_reason }`。`auth_headers`：`Authorization: Bearer {secret}`。
- Done when：`tests/adapter_openai.rs` 通过 3 组测试：(a) 单 text turn，(b) tool_call turn（含 `finish_reason = "tool_calls"`），(c) `usage.prompt_tokens / completion_tokens` 归一化到 `Usage`。
- Verify：`cargo test -p octopus-sdk-model --test adapter_openai`
- Stop if：OpenAI 侧 `usage` 缺少 `cache_read_input_tokens` 字段 → 映射为 0，登记 `02 §5`。

Step 4：
- Action：落地 `stubs.rs`：`OpenAiResponsesAdapter / GeminiNativeAdapter / VendorNativeAdapter` 三个 struct，`impl ProtocolAdapter` 的 `to_request` 返回 `Err(ModelError::AdapterNotImplemented { family })`，`parse_stream` 同理；`family()` 正常返回对应枚举。保留文件与符号，供 `DefaultModelProvider::builder()` 按 `ProtocolFamily` 查 `HashMap` 时命中。
- Done when：三个 stub 编译通过；`tests/adapter_stubs.rs` 验证 `to_request` 返回 `AdapterNotImplemented`。
- Verify：`cargo test -p octopus-sdk-model adapter::stubs::`
- Stop if：规范要求 W2 出口包含 Gemini adapter full impl → 重新审 `00 §3 W2` 出口；当前"至少两个 ProtocolAdapter"已满足，保持 stub。

Notes：
- mock fixtures 放 `crates/octopus-sdk-model/tests/fixtures/`（将来 W7 迁入 `crates/mock-anthropic-service` 的资产）；**本周不引用** `crates/mock-anthropic-service`（并行保留）。
- Task 6 按 Step 拆 **4 个独立 PR**（SSE / Anthropic / OpenAI / Stubs），每 PR 目标 ≤ 250 行，总量预估 600–800 行分布在 4 个 PR 中；任一 PR 超 300 行必须再次切分。
- AssistantEvent 变体清单来源：Pre-Task 核对步骤的实际结果，具体清单在 Task 6 启动 PR 里落笔前补录。

---

### Task 7：`RoleRouter` + `FallbackPolicy` + Prompt Cache 稳定性契约测试

Status: `pending`

Files:
- Create: `crates/octopus-sdk-model/src/role_router.rs`
- Create: `crates/octopus-sdk-model/src/fallback.rs`
- Create: `crates/octopus-sdk-model/tests/role_router.rs`
- Create: `crates/octopus-sdk-model/tests/fallback.rs`
- Create: `crates/octopus-sdk-model/tests/prompt_cache_stability.rs`

Preconditions：Task 4、Task 6 完成。**Pre-Task 核对 `crates/octopus-model-policy` 源文件**（对应 S5 审计）：
- 执行 `rg --files crates/octopus-model-policy/src/` 列出所有源文件；
- 读取每个 `.rs` 文件（预估共 ~143 行），记录其实际职责清单（`ModelRole → ModelFamily` 映射？canonical name 归一化？allowlist？）；
- 若职责**仅为** role → family 映射 → 继续 Step 1 原路径；
- 若**含**额外职责（例如 canonical name 归一化 / allowlist / budget limits），需把相应职责拆分：
  - canonical name 归一化归属 `Task 4 Step 3 resolve.rs`（若 Task 4 已实现则只需对齐，否则回 Task 4 补）；
  - allowlist / budget 相关职责不进 W2 Scope，登记到本 Plan §Scope Out 并推至 W4 `sdk-policy` 或业务侧；
  - 在 Task 9 Step 1 的变更日志里记录"model-policy 的 X 职责延后至 W?"。

Step 1：
- Action：落地 `RoleRouter { defaults: HashMap<ModelRole, ModelId>, overrides: HashMap<ModelRole, ModelId> }`；`resolve(role: ModelRole) -> Option<ModelId>`：先查 `overrides`，再查 `defaults`；`new_builtin(catalog: &ModelCatalog) -> Self` 把 `crates/octopus-model-policy` 的"角色 → 家族"默认**映射内容**（以 Preconditions 核对结果为准，不是代码）复制成 Rust 常量，最小覆盖：
  - `Main` → `catalog.resolve("opus")`
  - `Fast` → `catalog.resolve("haiku")`
  - `Best` → `catalog.resolve("opus-1m")` 或退化到 `opus`
  - `Plan` → `catalog.resolve("opus")`（与 Main 相同；`docs/sdk/11 §11.6.3` 的 `opusplan` 行为 W4 Plan mode 再接）
  - `Compact` → `catalog.resolve("haiku")` 或 `gemini-2.5-flash`
- Done when：`tests/role_router.rs` 覆盖 5 角色均返回非 `None`；`overrides` 能覆盖 `defaults`。
- Verify：`cargo test -p octopus-sdk-model --test role_router`
- Stop if：`catalog` 中缺 `opus` / `haiku` alias → 回 Task 4 补 catalog，Stop #7。

Step 2：
- Action：落地 `FallbackPolicy { chain: Vec<ModelId>, triggers: Vec<FallbackTrigger> }`；`FallbackTrigger { Overloaded, Upstream5xx, PromptTooLong, ModelDeprecated }`；`should_fallback(&self, err: &ModelError) -> Option<FallbackTrigger>`；`next_model(&self, current: &ModelId) -> Option<&ModelId>`。
- Done when：`tests/fallback.rs` 覆盖：`Overloaded` → `should_fallback = Some(Overloaded)`；链中第二个模型被选中；`ModelNotFound` 不触发 fallback。
- Verify：`cargo test -p octopus-sdk-model --test fallback`
- Stop if：fallback 需要与 session event stream 对接（`model_fallback` 事件） → W2 只在 `FallbackPolicy` 内记录 `struct FallbackRecord { from, to, trigger }` 供调用方消费；实际事件写入放 W6。

Step 3：
- Action：落地 **Prompt Cache 稳定性契约测试** `prompt_cache_stability.rs`。用 mock Anthropic server（`wiremock`）：3 次连续 `DefaultModelProvider::complete` 调用，每次传入 **相同的** `system_prompt / tools / messages[:n-1]`（仅最后一条 user message 变化）；mock 响应中 `cache_read_input_tokens` 固定为 `[0, 120, 240]`（单调递增）。断言：
  - 每次请求的 `tools_fingerprint` 相同；
  - 每次请求 JSON body 的 `system` / `tools` 字段序列化字节**逐字节相等**；
  - 解析出的 `Usage.cache_read_input_tokens` 在三次间单调递增。
- Done when：测试绿；失败时 panic 信息含 diff 指向不稳定字段。
- Verify：`cargo test -p octopus-sdk-model --test prompt_cache_stability`
- Stop if：mock 中 `cache_read_input_tokens` 递增但实际代码路径对 `tools` 做了 `HashSet` 去重导致顺序不稳 → Stop #4（Prompt Cache 命中率下降）。

Notes：
- 该 Step 3 是 `00-overview.md §3 W2` 硬门禁的直接映射，必须在本 Task 完成。
- `FallbackPolicy` 的链式重试 / 指数退避 **不** 在本 Task 实现；仅提供 trigger 判定。重试循环放 Task 5 `DefaultModelProvider::complete` 的包装器，W2 内做"一次 fallback 尝试"即可（避免与 W6 `recovery_recipes` 冲突）。

---

### Task 8：与 `contracts/openapi/src/**` 差异登记 + `docs/sdk` Fact-Fix

Status: `pending`

Files:
- Modify: `docs/plans/sdk/02-crate-topology.md §5 契约差异清单`
- Modify: `docs/sdk/README.md ## Fact-Fix 勘误`（仅当实际发现规范矛盾时）

Preconditions：Task 2–Task 7 全部 `done`。

Step 1：
- Action：逐项 diff `octopus-sdk-model` 新增的 9 类公共类型（`Provider / Surface / Model / ProtocolFamily / ModelRole / AuthKind / ModelTrack / ModelRequest / ModelError`）与 `contracts/openapi/src/**` 既有同名类型。已知差异（预期登记）：
  1. `SurfaceId` vs OpenAPI（可能无此概念）—— `align-openapi`；
  2. `ModelRole` 10 值 vs OpenAPI `RuntimeModelRole`（可能 4–5 值）—— `align-sdk` 或 `dual-carry`；
  3. `ProtocolFamily` 5 值 vs OpenAPI 侧（可能不存在）—— `align-openapi`；
  4. `ModelRequest.cache_breakpoints` vs OpenAPI 侧（可能不存在）—— `align-openapi`。
- Done when：`02 §5` 至少新增 4 行非占位数据；每行 `状态 = open`；不另起第二套表头。
- Verify：`rg -n '# \| 日期 \| 来源' docs/plans/sdk/02-crate-topology.md | wc -l` 应为 1（唯一表头）；`rg -c 'align-openapi|align-sdk|dual-carry|no-op' docs/plans/sdk/02-crate-topology.md` ≥ `W1 已有计数 + 4`。
- Stop if：OpenAPI 侧某字段命名仅差大小写（如 `inputTokens` vs `input_tokens`） → 处理方式登记 `no-op`，不阻塞。

Step 2：
- Action：若在 Task 6 发现 Anthropic / OpenAI SSE 事件名与 `docs/sdk/11` 描述不一致 → 在 `docs/sdk/README.md` 末尾 `## Fact-Fix 勘误` 追加条目（格式：`- [YYYY-MM-DD] 章节 §11.x：原文 "..."；实测 "..."；影响：...；引用：本 Plan Task 6 Step X`）；若无矛盾，本 Step 无操作。
- Done when：Fact-Fix 条目数变化与实际发现一致；若 Task 6 未发现矛盾则保持不变并在本 Step 备注"no-op"。
- Verify：`rg '^- \[2026-' docs/sdk/README.md | wc -l`
- Stop if：规范矛盾需要改 `docs/sdk/11` 主体文字而非 Fact-Fix 小节 → Stop #8（走专项决策 Plan）。

Notes：
- 本 Task 是**文档 only**，单 PR ≤ 150 行 diff。
- 不涉及 `pnpm openapi:bundle` / `pnpm schema:generate`（本周不改 OpenAPI）。

---

### Task 9：公共面冻结 + Weekly Gate 收尾

Status: `pending`

Files:
- Modify: `docs/plans/sdk/02-crate-topology.md §2.1 / §2.3`（若实际 `pub` 符号与登记仍有 diff；Task 2/3/5/7 已做同批次回填，此处仅"补漏"）
- Modify: `docs/plans/sdk/README.md`（W2 行状态 → `done`）
- Modify: `docs/plans/sdk/00-overview.md §10`（追加 W2 Weekly Gate 结果到总控变更日志）
- Modify: `docs/plans/sdk/05-week-2-model.md`（本文件；Task 全 `done`、追加 Checkpoint、变更日志）

> **S6 审计**：**不修改** `03-legacy-retirement.md`。W2 只做内容迁移、不删代码；按 `03 §0 使用规则`，状态 `pending → done` 只在"真删除旧代码"的 PR 里发生（W7）。本周的 legacy 回链仅登记在本 Plan §"Legacy 退役登记回链"表，不动 `03` 的 `状态` 列。

Preconditions：Task 1–Task 8 全部 `done`；`cargo test -p octopus-sdk-model` 全绿；`cargo test -p octopus-sdk-contracts` 全绿（`ToolSchema` 下沉后 contracts 回归）；`cargo clippy -p octopus-sdk-model -p octopus-sdk-contracts -- -D warnings` 全绿；`cargo clippy --workspace -- -D warnings` 全绿；`cargo build --workspace` 全绿。

Step 1：
- Action：核对 `crates/octopus-sdk-model + crates/octopus-sdk-contracts`（含 W2 新增的 `ToolSchema`）所有 `pub (struct|enum|trait|fn|type|const)` 符号集合 ⊆ `02 §2.1 / §2.3` 登记集合；若出现未登记的 `pub` → 要么回到本 Plan 内收回 `pub`，要么补登 §2.1 / §2.3；**二选一**必须在本 Task 内闭环。
- Done when：`rg 'pub (struct|enum|trait|fn|type|const) ' crates/octopus-sdk-model/src crates/octopus-sdk-contracts/src/tool_schema.rs` 输出逐项能在 `02 §2.1` 或 `§2.3` 中找到对应行；`crates/octopus-sdk-model/src/lib.rs` ≤ 80 行。
- Verify：作者自核对（附在 Checkpoint 的 `Completed` 列表中）。
- Stop if：实际代码有 `02` 未登记的 `pub` 且有生产使用方 → 必须补登，不可回收。

Step 2：
- Action：执行 `01-ai-execution-protocol.md §4 Weekly Gate` 全部勾选；与 `00-overview.md §3 W2` 的出口状态 / 硬门禁逐条对齐；更新本文件"任务状态 + Checkpoint + 变更日志"；把 `README.md` 的 W2 行状态从 `pending`/`draft`/`in_progress` 改为 `done`；并在 `00-overview.md §10` 追加一行 W2 Weekly Gate 结果。
- Done when：
  - `cargo test -p octopus-sdk-model` 全绿；
  - `cargo test -p octopus-sdk-contracts` 全绿（`ToolSchema` 下沉后 contracts 自身回归）；
  - `cargo clippy -p octopus-sdk-model -p octopus-sdk-contracts -- -D warnings` 全绿；
  - `cargo clippy --workspace -- -D warnings` 全绿；
  - `cargo build --workspace` 全绿；
  - `find crates/octopus-sdk-model -type f -name '*.rs' -exec wc -l {} \; | awk '$1 > 800'` 无输出（单文件行数硬约束；S4 审计：使用 `wc -l` 行数版替代 `-size +800c` 字节版）；
  - `cargo test -p octopus-sdk-model --test prompt_cache_stability` 绿（W2 硬门禁 · Prompt Cache 基线）；
  - `cargo test -p octopus-sdk-model provider_impl::fallback` 绿（W2 硬门禁 · Fallback 单次切换 · 对应 B4）；
  - `rg 'RoleRouter|FallbackPolicy' crates/octopus-sdk-model/src/` 至少各 3 处；
  - `rg '^\| 05-week-2-model\.md \| .* \| `done` \|' docs/plans/sdk/README.md` 命中；
  - `rg 'W2|05-week-2-model|octopus-sdk-model' docs/plans/sdk/00-overview.md` 在 `§10` 新增一条当周收尾记录。
- Verify：执行上述 11 条命令。
- Stop if：任一硬门禁失败 → Weekly Gate 未通过；W2 保持 `in_progress`，不切 W3；在 `00-overview.md §6 风险登记簿` 追加阻塞条。

Notes：
- Step 2 是本 Plan 的唯一出口；未执行不得声明本周完成。
- 本 Task 是**文档 only**，单 PR ≤ 150 行 diff。

---

## Exit State 对齐表（与 `00-overview.md §3 W2` 逐条对齐）

| `00 §3 W2` 出口状态 | 本 Plan 对应任务 | 验证命令 |
|---|---|---|
| Provider / Surface / Model 三层存在 | Task 2 | `cargo test -p octopus-sdk-model provider::` |
| `anthropic_messages` + `openai_chat` 两 adapter 迁移完成 | Task 6 Step 2 / Step 3 | `cargo test -p octopus-sdk-model --test adapter_anthropic --test adapter_openai` |
| `RoleRouter` 覆盖 5 角色（main / fast / best / plan / compact） | Task 7 Step 1 | `cargo test -p octopus-sdk-model --test role_router` |
| `FallbackPolicy` 覆盖 overloaded / 5xx / prompt_too_long（**识别触发条件**） | Task 7 Step 2 | `cargo test -p octopus-sdk-model --test fallback` |
| `FallbackPolicy` 在 `DefaultModelProvider` 内触发**单次重试切换** | Task 5 Step 3 | `cargo test -p octopus-sdk-model provider_impl::fallback` |
| Canonical Naming / catalog 静态默认来自 `vendor-matrix.md` | Task 4 | `cargo test -p octopus-sdk-model --test catalog_builtin` |
| Prompt cache 基线测试：3 次连续调用 `cache_read_input_tokens` 持续增长 | Task 7 Step 3 | `cargo test -p octopus-sdk-model --test prompt_cache_stability` |
| `cargo test -p octopus-sdk-model` 全绿 | Task 9 Step 2 | `cargo test -p octopus-sdk-model` |

---

## 公共面登记回链（必填）

本 Plan 所有对外 `pub` 符号集中登记在 `02-crate-topology.md §2.3`；对 §2.3 的任何签名修正必须在 Task 2 / Task 3 / Task 5 / Task 7 的同批 PR 内完成，不得延后。

## Legacy 退役登记回链（必填）

| 本 Plan 任务 | `03-legacy-retirement.md` 条目 | 状态变化 |
|---|---|---|
| Task 6 | §5 `crates/api/src/providers/{anthropic.rs, openai_compat.rs}` | W2 期间 `pending`（仅复制形状不删代码） |
| Task 6 | §5.1 `adapter/model_runtime/drivers/*`（4 文件） | W2 期间 `pending`（仅形状参考） |
| Task 7 Step 1 | §7.6 `crates/octopus-model-policy` | W2 期间 `pending`（内容已迁入 `role_router.rs`，crate 目录 W7 删） |

> 说明：`pending → done` 的状态迁移统一发生在 W7 "业务侧切换 + 11 个遗留 crate 下线"；W2 只做**内容迁移**与**并行保留**，不改 legacy 代码。

---

## Batch Checkpoint Format

按 `01-ai-execution-protocol.md §6.1` 追加（本文档末尾）。

```md
## Checkpoint YYYY-MM-DD HH:MM

- Week: W2
- Batch: Task <i> Step <j> → Task <i+1> Step <j>
- Completed:
  - <item>
- Files changed:
  - `path` (+added / -deleted / modified)
- Verification:
  - `cargo test -p octopus-sdk-model` → pass
  - `cargo clippy -p octopus-sdk-model -- -D warnings` → pass
- Exit state vs plan:
  - matches / partial / blocked
- Blockers:
  - <none | 具体问题 + 待人判断点>
- Next:
  - <Task i+1 Step j+1 | Task 9 | Weekly Gate>
```

---

## 变更日志

| 日期 | 变更 | 责任人 |
|---|---|---|
| 2026-04-21 | 首稿（9 Task Ledger + Exit State 对齐表 + Legacy 退役回链） | AI Agent |
| 2026-04-21 | 方案审计后一次性修订 4 阻塞级 + 6 建议级共 10 处：<br>① B1 新增"本周 §2.1/§2.3 公共面修订清单"总表（10 行）；<br>② B2 Task 3 Step 2/3 Done when 增加 `ModelError` 11 变体与 `auth_headers` 回填 §2.3；<br>③ B3 `ToolSchema` 下沉到 `octopus-sdk-contracts`（Level 0），`octopus-sdk-model` 仅 re-export，避免 W3 重复定义；<br>④ B4 Task 5 新增 Step 3 `complete_with_fallback` 单次重试包装器，Exit State 对齐表新增独立行；<br>⑤ S1 Task 6 明确按 Step 拆 4 个独立 PR（SSE / Anthropic / OpenAI / Stubs），串行合入；<br>⑥ S2 `catalog/builtin/<provider>.rs` 默认按 provider 拆文件（9 个源文件 + `mod.rs` 聚合），每文件 ≤ 250 行；<br>⑦ S3 Task 6 Preconditions 增加 `AssistantEvent` 变体核对前置步骤；<br>⑧ S4 Task 9 Step 2 `find ... -size +800c` 改为 `find ... -exec wc -l` 行数验证；<br>⑨ S5 Task 7 Step 1 Preconditions 增加 `crates/octopus-model-policy` 源码实际职责核对；<br>⑩ S6 Task 9 Files 移除 `03-legacy-retirement.md`，不动 `03` 状态列。<br>观察级 3 项（O1 Provider.surfaces 规范 vs 实现索引型差异 / O2 `PromptCacheRecord` 磁盘持久化归属 / O3 `tools_fingerprint` 签名一致化）留待 Task 8 Step 2 Fact-Fix 或执行中确认。 | AI Agent |
| 2026-04-21 | 评审修订：闭合 6 条 review findings，改为以当前 `AssistantEvent` 公共面执行 Task 6；补齐 `ModelRequest`/`ResponseFormat`/`ThinkingConfig`/`CacheControlStrategy` 的 `02 §2.3` 回填义务；Task 9 恢复 `cargo clippy --workspace -- -D warnings` 与 `00-overview.md §10` 变更日志要求；修正 `fallback` 测试目标、`Provider.status` 过滤语义，以及 `ModelRole` 偏差的 Fact-Fix 回链。 | Codex |
