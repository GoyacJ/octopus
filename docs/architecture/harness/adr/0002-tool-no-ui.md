# ADR-002 · Tool 接口不得包含 UI 渲染

- **状态**：Accepted
- **日期**：2026-04-24
- **决策者**：架构组
- **影响范围**：`harness-tool` / 所有 Tool 实现（内置 + 业务扩展 + Plugin）

## 1. 背景与问题

参考项目中 Claude Code 的 `Tool` 接口同时承载：

- `schema`（模型契约）
- `render`（UI 表现，类型为 `React.ReactNode`）
- `permission` 元数据
- `concurrency` 标志

这种设计导致**非 Ink（Node + React）消费者无法复用 Tool 实现**（CC-37）。例如：

- 在 Vue 前端，`React.ReactNode` 需要重实现
- 在 TUI（非 Ink），渲染节点不可移植
- 在 Tauri Host，需要桥接 React → WebView
- 在 HTTP Server（JSON API），渲染节点根本无意义

本 SDK 要支持 Desktop（Tauri+Vue）、Server（HTTP JSON）、CLI（TUI / stdout）、第三方集成（MCP Server）多种形态。Tool 耦合 UI 将导致所有消费者负担。

## 2. 决策

**Tool 接口只包含 schema + 元数据 + 执行逻辑，不得携带 UI 渲染**。

```rust
#[async_trait]
pub trait Tool: Send + Sync + 'static {
    fn descriptor(&self) -> ToolDescriptor;
    fn input_schema(&self) -> &JsonSchema;
    fn output_schema(&self) -> Option<&JsonSchema>;
    fn properties(&self) -> ToolProperties;
    async fn validate(&self, input: &Value, ctx: &ToolContext) -> Result<()>;
    async fn check_permission(&self, input: &Value, ctx: &ToolContext)
        -> PermissionCheck;
    async fn invoke(&self, input: Value, ctx: ToolContext) -> ToolResult;
}
```

**禁止事项**：

- ❌ `fn render(&self, result: &ToolResult) -> ReactNode` 等渲染方法
- ❌ 在 `ToolDescriptor` 中加 `icon_svg` / `ansi_color` / `markdown_template` 等呈现字段
- ❌ 在 `ToolResult` 中携带 `html` / `react_element` / `svg` 负载（改用 `ToolResult::structured(Value)`）

**正确做法**：

- Tool 返回 `ToolResult` 结构化数据（JSON）
- 业务层消费 `EventStream` 时，按自己的 UI 框架决定如何呈现
- 表现层元信息（如图标）放在业务层的 `ToolRenderRegistry`，与 SDK 解耦

## 3. 替代方案

### 3.1 A：Tool 含多格式渲染器（Ink / HTML / TUI / CLI）

- ❌ SDK 膨胀：维护多套渲染代码
- ❌ 消费者被迫依赖所有渲染 crate
- ❌ 无法覆盖全部 UI 需求

### 3.2 B：Tool 提供可选 render + 默认空实现

- ❌ 半耦合，仍违反 P1 内核纯净
- ❌ 诱导业务在 Tool 里写渲染

### 3.3 C：完全剥离 UI（采纳）

- ✅ Tool 只做语义；UI 由业务层决定
- ✅ 多端自由
- ✅ 可通过 MCP Server Adapter 透明暴露给任意 MCP 客户端

## 4. 权衡

| 维度 | 代价 | 缓解 |
|---|---|---|
| 业务层重复渲染代码 | 每个前端写一套 render | 提供 `ToolDescriptor::category` 支持按类型归类；业务可封装通用 renderer |
| 图标/颜色等元数据缺失 | SDK 不提供 | 业务侧建 `ToolPresentationRegistry`，按 `tool_name` 查找 |
| 无法自动生成 HTML 文档 | SDK 不管 | 业务可用 `input_schema / output_schema` 反向生成 |

## 5. 后果

### 5.1 正面

- SDK 体积小
- 多端一致性
- MCP Server Adapter 纯 JSON 易实现
- 测试不依赖 UI 框架

### 5.2 负面

- 业务层需要自己做渲染
- 新前端接入必须实现 Tool renderer（但本来就必须）

## 6. 实现指引

- `ToolResult` 类型：
  ```rust
  pub enum ToolResult {
      Text(String),
      Structured(Value),
      Blob { content_type: String, blob_ref: BlobRef },
      Mixed(Vec<ToolResultPart>),
  }
  ```

### 6.1 `ToolResultPart` —— 正向白名单（v1.8 升级）

> 历史版本仅给出"反向黑名单"（禁止 `html` / `react_element`），但 v1.7 之后
> `ToolResult::Mixed` 在 PTC（ADR-0016）/ 子代理 announcement / Skill invoke
> 等场景被广泛使用，需要明确"允许且仅允许"哪些语义变体。
>
> 权威来源：`crates/harness-contracts.md §3.5` 的 `ToolResultPart` 与
> `ReferenceKind` 定义。本节摘要其设计原则：

| 允许的语义变体 | 用途 |
|---|---|
| `Text` | 纯文本片段 |
| `Structured { value, schema_ref }` | 已知 schema 的结构化负载 |
| `Blob { content_type, blob_ref, summary }` | 大文本 / 二进制落盘引用（ADR-0010） |
| `Code { language, text }` | 代码片段（脚本输出 / `execute_code` 步骤产物） |
| `Reference { kind, title, summary }` | 外部 URL / 文件 / Transcript / ToolUse / Memory 引用 |
| `Table { headers, rows, caption }` | 行 × 列语义数据（**非** UI Table widget） |
| `Progress { stage, ratio, detail }` | 时序进度（仅事件溯源 / UI 时间线） |
| `Error { code, message, retriable }` | 部分失败片段；外层 `ToolResultEnvelope.is_error` 仍是真相源 |

### 6.2 反向黑名单（保留并扩展）

`ToolResultPart` **绝不**接受以下语义类型；任何 PR 引入即视为违反 ADR：

- ❌ `Html` / `ReactElement` / `VueComponent` / `SvelteSnippet`
- ❌ `TauriCommand` / `EguiNode` / `WebViewBridge`
- ❌ `Markdown`（仍属"渲染层概念"——业务侧若需要让 LLM 生成 markdown，请落 `Text`）
- ❌ 任何嵌入 base64 大块二进制的 inline 字段（应走 `Blob.blob_ref`）

### 6.3 强制守卫

- `ToolDescriptor.output_schema` 的 JSON Schema 中也禁止出现 `format: html` / `format: react`
- `harness-tool::ToolOrchestrator::collect_with_budget`（ADR-0010 §2.3）在
  序列化 `Mixed` 时按上表正向白名单一一校验；命中黑名单 fail-closed
  `ToolError::MalformedToolResult { reason: DisallowedVariant(...) }`
- Lint 规则：`clippy::disallowed-types` 禁止 Tool 实现 import `react_*` / `tauri::*` / `egui::*`
- 业务侧若需要新增变体，必须走 ADR + `harness-contracts §3.5` 增量评审，不得在 SDK 默认实现里偷偷加

## 7. 监测与强制

- CI `cargo-deny` 规则阻止 `harness-tool` 依赖 UI crate
- Code review 必须拒绝违反本 ADR 的 PR
- `harness-plugin` 的 manifest validation 拒绝声明 `render_format` 字段的 tool

## 8. 相关

- D3 · `api-contracts.md` §Tool
- D7 · `extensibility.md` §Tool 扩展
- `crates/harness-tool.md`
