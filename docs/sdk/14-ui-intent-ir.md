# 14 · UI 意图 IR（UI Intent IR）

> "We virtualized the components of an agent... so that the implementation of each can be swapped without disturbing the others."
> — [Anthropic · Managed Agents](https://www.anthropic.com/engineering/managed-agents)

本章定义 SDK 层的 **UI 意图中间表达**（UI Intent Intermediate Representation），用来回答一个实践问题：

> 当 Agent 产生一次 `tool_use` / `tool_result` / 询问 / 成果物时，**SDK 应该向业务层传递什么**，才能既支持 Claude.ai 的"独立成果物展示"，又兼容 Claude Code 的"文件 + inline diff"，还保持桌面 / CLI / 未来移动端多宿主一致？

答案是一组**宿主中立、JSON 可序列化、discriminated-union**的描述符。SDK 产出**意图**（IR），业务层产出**组件**；前者是数据契约，后者是渲染实现。

## 14.1 目的与非目的

**目的**：

- 定义 SDK 向业务层传递 UI 语义时的**唯一形状**：`RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactRef` / `ArtifactKind`。
- 让 Claude.ai 风格（成果物作为一等引用）与 Claude Code 风格（文件 + 内联 diff）成为**同一 IR 的两种工具实现策略**，而非两套架构。
- 为 03 §3.1.1 已埋伏笔的 `ToolDisplayDescriptor` 正式化（补齐 5 个生命周期钩子 + 10 余种 kind）。
- 给 09 Observability 一个清晰的"UI 层 payload"形状，用于 event log 与 replay。

**非目的**：

- **不**替代 `@octopus/ui` 组件库。IR 是描述符，不是组件；组件仍然按 `AGENTS.md` §Frontend Governance 的 Shared UI Component Catalog 归属。
- **不**替代 `contracts/openapi/` 与 `packages/schema/`。IR 是**运行时**从 harness 流向业务层的载荷，不直接对外暴露 HTTP（间接通过 09 章 event 流）。
- **不**定义 CSS、主题、motion；这些属于宿主实现（桌面走 `@octopus/ui` + tokens，CLI 走 Ink 配色）。
- **不**允许插件绕过 SDK 新增 IR kind；IR 枚举扩展走 SDK 版本升级。

## 14.2 四层分层与边界契约

```
┌────────────────────────────────────────────────────────────────┐
│  L4  业务层渲染（apps/desktop · apps/cli · apps/mobile）        │
│      读 IR → 按 kind 单点 dispatch → 渲染为具体组件              │
│      桌面映射到 @octopus/ui；CLI 映射到 Ink/TUI                 │
└────────────────────────────────────────────────────────────────┘
                           ▲ 读（只读 IR）
┌────────────────────────────────────────────────────────────────┐
│  L3  SDK UI 意图契约（本章；packages/schema/src/ui-intent.ts）  │
│      discriminated union / JSON 可序列化 / 宿主中立              │
│      不得依赖任何 UI 框架；不得包含函数 / 组件 / 闭包             │
└────────────────────────────────────────────────────────────────┘
                           ▲ 生产 IR
┌────────────────────────────────────────────────────────────────┐
│  L2  Harness 执行层（01/03/04/05/06/07/08/09）                  │
│      工具 5 钩子 + 权限引擎 + 成果物管理 + 事件流 → 生成 IR      │
│      IR 作为 09 event payload 的一部分写入 runtime/events/*.jsonl│
└────────────────────────────────────────────────────────────────┘
                           ▲ 驱动
┌────────────────────────────────────────────────────────────────┐
│  L1  模型协议层（Protocol Adapter · 11 §11.6）                   │
│      各厂商流 → Canonical Message IR（text/thinking/tool_use…）  │
└────────────────────────────────────────────────────────────────┘
```

**边界契约**（每条都是非协商的）：

- L3 **只** 定义形状；**不** 定义组件映射表。映射表由 L4 各宿主自持。
- L3 描述符必须**100% JSON 可序列化**；不得出现 `React.ReactNode` / Vue `VNode` / 函数指针 / Promise / class 实例。
- L2 **必须** 把所有 UI 相关的运行时事件翻译为 L3 IR 后再对外暴露；严禁直接向业务层暴露"原始 tool_use 对象"让业务层自己 `switch(toolName)`。
- L4 **必须** 以 "IR kind 单点 dispatch" 的方式渲染；**不得** 出现工具专属组件（例如 `BashToolView.vue` / `EditToolView.vue`）。
- 新增 kind **只能** 改 L3 定义（SDK 版本升级）；L2 / L4 的变化**不**触发 IR 枚举扩展。

> 与 03 §3.1.1 的关系：03 章已在 `Tool` 接口上埋下 `displayDescriptor?: ToolDisplayDescriptor` 占位；本章把它**正式化并扩容**，并把 5 个生命周期钩子完整定义在 §14.5。`ToolDisplayDescriptor` 从此等价于 `RenderLifecycle` 的子集。

> 与 `AGENTS.md` §Frontend Governance 的关系：本章与治理规约**严格对齐**——"Shared UI must go through `@octopus/ui`；business pages must not introduce ad-hoc third-party UI"。L3 IR 正是这条规约在数据层的投影：**工具不直接选组件，只表达意图**。

## 14.3 顶层 IR 类型

以下是 TypeScript 伪代码（未来 `packages/schema/src/ui-intent.ts` 以 Zod discriminated union 实现；本章只定形状，不定代码）。

### 14.3.1 `RenderBlock` — 对话流内的最小渲染单元

```ts
type RenderBlock =
  | { kind: 'text';         text: string }
  | { kind: 'markdown';     markdown: string }
  | { kind: 'code';         language: string; code: string; collapsed?: boolean }
  | { kind: 'diff';         path: string; before: string; after: string; language: string }
  | { kind: 'list-summary'; counts: Record<string, number>; label?: string }
  | { kind: 'progress';     text: string; indeterminate?: boolean; percent?: number }
  | { kind: 'artifact-ref'; artifactId: string; kind: ArtifactKind; preview?: string }
  | { kind: 'record';       title: string; rows: Array<{ label: string; value: string; href?: string }> }
  | { kind: 'error';        message: string; retriable: boolean; remediation?: string }
  | { kind: 'raw';          mime: string; data: unknown }
```

**使用规则**：

- 每个钩子返回 `RenderBlock[]`（零到多个）；顺序即渲染顺序。
- `collapsed` / `indeterminate` / `preview` 等可选字段**宿主中立**：业务层可以选择响应也可以忽略（例如 CLI 不支持折叠就平铺）。
- `raw` 是逃生出口：任何尚未建模的类型可以先用 `mime + data` 传输；稳定后再收编为正式 kind。**新工具必须先用 `raw`**，不得擅自扩 kind。

### 14.3.2 `RenderLifecycle` — 工具生命周期的 5 个渲染点

```ts
type RenderLifecycle = {
  onToolUse?:      RenderBlock[]   // 模型刚发出 tool_use 时的"预览行"
  onToolProgress?: RenderBlock[]   // 工具流式进度（AsyncIterable<ToolProgress>）
  onToolResult?:   RenderBlock[]   // 工具返回后的结果卡片
  onToolRejected?: RenderBlock[]   // 权限拒绝 / 用户 Deny 时的说明
  onToolError?:    RenderBlock[]   // 工具抛错 / 超时 / 取消时的提示
}
```

**映射关系**（与 Claude Code 的完全对齐但类型替换）：

| Claude Code（`Tool.ts`） | 本章 | 返回类型差异 |
|---|---|---|
| `renderToolUseMessage` | `onToolUse` | `React.ReactNode` → `RenderBlock[]` |
| `renderToolUseProgressMessage` | `onToolProgress` | `React.ReactNode` → `RenderBlock[]` |
| `renderToolResultMessage` | `onToolResult` | `React.ReactNode` → `RenderBlock[]` |
| `renderToolUseRejectedMessage` | `onToolRejected` | `React.ReactNode` → `RenderBlock[]` |
| `renderToolUseErrorMessage` | `onToolError` | `React.ReactNode` → `RenderBlock[]` |

> 关键差异：Claude Code 的 render 钩子绑死在 Ink/TUI React；Octopus 的钩子**只返回数据**，宿主自行渲染。这是 "L3 中立性" 的具体落点。

### 14.3.3 `AskPrompt` — 询问用户的顶层意图

```ts
type AskPrompt = {
  kind: 'ask-user'
  questions: Array<{
    id: string
    question: string                                // 完整问句，以问号结尾
    header: string                                  // 短 chip（≤ 12 字符）
    multiSelect: boolean                            // 默认 false
    options: Array<{
      id: string
      label: string                                 // 1–5 字，选项文案
      description: string                           // 说明选项含义 / 后果
      preview?: string                              // 对比型 preview（markdown/html 片段）
      previewFormat?: 'markdown' | 'html'
    }>
  }>
}
```

**约束**（完整对齐 Claude Code `AskUserQuestionTool` 的 schema 约束）：

- `questions.length` ∈ [1, 4]；`options.length` ∈ [2, 4]。
- `question` / `options[*].label` 在各自范围内唯一（去重约束）。
- `preview` 仅在 `multiSelect = false` 时有效；`preview` / `previewFormat` 成对出现。
- 为什么是**独立顶层类型**而不是 `RenderBlock` 的一种：
  1. AskPrompt 语义上带**阻塞** / **modal** / **回写 tool_result**，与纯展示性 RenderBlock 不同质。
  2. AskPrompt 生命周期由权限引擎（06 §6）接管渲染，独立于工具的 `onToolUse` 钩子。
  3. AskPrompt 的回响要经过 `mapToolResultToToolResultBlockParam` 翻译回 tool_result 文本；RenderBlock 则是单向展示，无回响。

> 工具 `AskUserQuestion` 的 `onToolUse` 可以返回**空数组**或一个 `progress` 类的 `RenderBlock`；询问 UI 由权限引擎基于 `AskPrompt` 顶层类型独立渲染（`UiDialog` + `UiSelectionMenu`）。这与 Claude Code 的做法一致：`AskUserQuestionTool.renderToolUseMessage` 返回 `null`，由独立组件接管。

### 14.3.4 `ArtifactKind` 与 `ArtifactRef` — 成果物

```ts
type ArtifactKind =
  | 'markdown'
  | 'code'
  | 'html'
  | 'svg'
  | 'mermaid'
  | 'react'       // JSX / TSX 片段（桌面可渲染；CLI 退化为 code）
  | 'json'
  | 'binary'

type ArtifactStatus = 'draft' | 'review' | 'approved' | 'published'
  // 与 `contracts/openapi/src/components/schemas/shared.yaml::ArtifactStatus` 同枚举

type ArtifactRef = {
  kind: 'artifact-ref'
  artifactId: string                   // `@octopus/schema::artifact.ts` 的 id（`DeliverableDetail.id`）
  artifactKind: ArtifactKind
  title?: string
  preview?: string                     // 对话流缩略；≤ 400 字符建议
  previewFormat?: 'markdown' | 'text'
  version?: number                     // 缺省 = 最新版（`DeliverableSummary.latestVersion`）
  parentVersion?: number               // 对应 `DeliverableVersionSummary.parentVersion`；渲染 vN-1 → vN 链路
  status?: ArtifactStatus              // 草稿 / 待审 / 已批准 / 已发布
  contentType?: string                 // MIME，供 html/react/svg/mermaid 做二次路由（见 §14.3.4.1）
  supersededByVersion?: number         // 旧引用被新版本覆盖时填；UI 可渲染"查看 vN"跳转
}
```

**约束**：

- `artifactId` 必须对应 `@octopus/schema::artifact.ts` + `/api/v1/projects/{id}/deliverables/*` 里真实存在的记录；ID 空间由 schema / HTTP 层签发，IR 不再自创。
- `version` 省略时业务层按"最新版"渲染；要显式引用历史版本必须填完整 int。
- `preview` 是**轻量摘要**，不是全量内容；全量走 `/api/v1/projects/{id}/deliverables/{artifactId}/versions/{version}/content` 拉取（保持 event log 体积可控）。
- `parentVersion` / `supersededByVersion` 构成"版本链"：每次 `update_artifact` 产出的新 ref 必须带 `parentVersion`；若旧 turn 的 ref 被用户视图感知为过时，Harness 可回写 `supersededByVersion` 重写最近一次对话流投影（不改 event log 原文，见 §14.8）。
- `status` 流转由 `promote_artifact` 工具驱动（见 §14.12.3）；`published` 后仍可 `fork` 产出新 artifact，不可直接 mutate。
- `ArtifactRef` 同时作为 `RenderBlock.kind === 'artifact-ref'` 的 payload 内嵌到对话流；独立类型便于被跨 turn 引用（下一 turn 工具 input 可直接 `{ artifactId, version }` 指代）。

#### 14.3.4.1 `ArtifactKind` 与 schema `ResourcePreviewKind` 的分工

两者**不是同一个东西**，**不必对齐枚举**；职责拆开后各自独立演进，映射关系由 IR 层明确。

| 维度 | 14 章 `ArtifactKind` | schema `ResourcePreviewKind`（`workspace.yaml`） |
|---|---|---|
| 职责 | **渲染 hint** — 决定业务层把 `artifact-ref` 路由到哪个组件（`UiArtifactBlock` 的子渲染器） | **存储 / 预览分类** — 决定资源列表图标、预览通道、文件类型筛选 |
| 枚举 | `markdown / code / html / svg / mermaid / react / json / binary` | `text / code / markdown / image / pdf / audio / video / folder / binary / url` |
| 修改面 | 走 SDK 版本升级（本章约束） | 走 `contracts/openapi/src/**` → `pnpm openapi:bundle` → `pnpm schema:generate` |
| 存活位置 | 内存中的 `ArtifactRef` payload；event log 原样落库 | `DeliverableSummary.previewKind` / `DeliverableDetail.previewKind` 持久化字段 |

**IR → schema 映射表**（工具创建 artifact 时的默认映射；特殊场景工具可显式覆盖 `previewKind`）：

| `ArtifactKind`（IR） | `previewKind`（schema） | 典型 `contentType`（MIME） |
|---|---|---|
| `markdown` | `markdown` | `text/markdown` |
| `code` | `code` | `text/plain`（或具体语言 MIME） |
| `html` | `code` | `text/html` |
| `react` | `code` | `text/jsx` / `text/tsx` |
| `svg` | `code` | `image/svg+xml` |
| `mermaid` | `code` | `text/vnd.mermaid` |
| `json` | `code` | `application/json` |
| `binary` | `binary` | 按具体格式填 |

> 映射规则：`html / react / svg / mermaid / json` 在 schema 层统一落 `previewKind=code`（保持预览通道稳定），但在 IR 层用 `artifactKind` + `contentType` 做二次路由，确保 `UiArtifactBlock` 能选对子渲染器。

#### 14.3.4.2 `ArtifactKind` 专属安全约束

`html / react / svg / mermaid` 这类"**可执行 / 可解析**"成果物必须在业务层渲染时带安全边界；IR 只传数据不传代码，但**不解除**渲染方的责任。下表是 L4 宿主（含桌面 + 未来 web host）的**最小**安全约束集，业务层可加严不可放宽。

| `ArtifactKind` | 最小安全约束 | 违反后果 |
|---|---|---|
| `html` | 必须运行在 sandboxed iframe：`sandbox="allow-scripts"`（**不**允许 `allow-same-origin`、`allow-top-navigation`、`allow-forms-submit`、`allow-popups`）；搭配严格 CSP（`default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'`）；禁止访问宿主 DOM / cookies / localStorage | XSS、钓鱼、token 泄露 |
| `react` | 同 `html`；源码**必须**预编译为沙箱 bundle（esbuild / swc 离线编译），**禁止**运行时 `eval` / `new Function`；JSX 注入点只接受白名单 JSX runtime | RCE、供应链注入 |
| `svg` | 剥离 `<script>` / `<foreignObject>` 内脚本 / 事件属性（`on*`）/ `xlink:href="javascript:..."` / 外链 `href` / 外链 `image href`；优先用 `DOMPurify` 或等价库处理后再渲染 | XSS via SVG |
| `mermaid` | 使用 `mermaid.initialize({ securityLevel: 'strict' })` 或等价安全级；**禁止** `securityLevel: 'loose'`；节点 label 必须 HTML-escape | 模板注入 |
| `markdown` | 渲染器必须禁用原始 HTML 直通（或等价于 `DOMPurify` 过滤）；URL 协议白名单（`http` / `https` / `mailto`）；禁用 `javascript:` / `data:` 链接 | XSS via markdown |
| `code` / `json` / `binary` | 只读渲染；不得 `eval`；`binary` 下载需用户确认 | — |

> 责任分工：**IR 层**保证字段不夹带可执行内容（§14.10 反模式 #2）；**业务层**保证渲染管线带上述安全边界；**权限层**（06）保证 artifact 打开 / 下载 / 发布的审批链路。三者缺一不可。

### 14.3.5 `ProgressEvent` — 流式进度

```ts
type ProgressEvent = {
  kind: 'progress'
  toolUseId: string
  seq: number                   // 单调递增
  blocks: RenderBlock[]         // 本 tick 新增的 blocks（增量）
  final?: boolean               // 最后一 tick 标记
}
```

**使用规则**：

- 工具 `execute(...)` 的 `AsyncIterable<ToolProgress>` 每 yield 一次 → L2 产出一个 `ProgressEvent`，`seq` 单调递增。
- 业务层按 `toolUseId + seq` 去重与排序；不假设传输有序。
- `final = true` 后不应再有同 `toolUseId` 的 `ProgressEvent`；之后应收到 `onToolResult`。

## 14.4 `RenderBlock.kind` 枚举清单

IR kind 的硬约束：**总数不超过 15**。当前 10 个，预留扩展余量。新增必须走 SDK 版本升级并更新本节表格。

| kind | 语义 | 典型来源 | 桌面建议组件（`@octopus/ui`） | CLI 建议呈现 |
|---|---|---|---|---|
| `text` | 纯文本片段 | 模型 `text` block / 错误短信息 | `UiListRow`（文本样式） | 普通段落 |
| `markdown` | Markdown 片段 | `fs_read` 返回、思考摘要 | `Markdown` 渲染（走 `@octopus/ui` 的 markdown 块） | Ink Markdown |
| `code` | 代码片段 | `fs_read` / `shell_exec` 输出 | `UiCodeEditor`（只读） | 语法高亮 |
| `diff` | 单文件 before/after | `fs_write` / `fs_edit` 完成 | `UiCodeEditor`（diff 视图） | 行内 diff |
| `list-summary` | 折叠后的计数行 | 连续 `fs_read` / `fs_grep` 聚合 | `UiListRow`（badge 计数） | 一行概述 |
| `progress` | 进度行 | `ToolProgress.phase === 'stream'` | `UiListRow` + spinner | 旋转符号 + text |
| `artifact-ref` | 成果物引用卡片 | `create_artifact` / `update_artifact` | `UiArtifactBlock` | 链接 + 标题 |
| `record` | 键值对记录（人可读表） | `web_search` / `task_list` 结果 | `UiRecordCard` | 表格文本 |
| `error` | 错误 + 修复建议 | `onToolError` / `ToolError` | `UiStatusCallout`（error variant） | 红色段落 |
| `raw` | 任意 mime payload 逃生出口 | 未建模内容 | 依 mime 路由；未知则 `UiEmptyState` | 摘要 + mime 标签 |

> 为什么 `thinking` 不是一个独立 kind：Anthropic Extended Thinking 在 IR 层直接用 `markdown`（或 `text`）表达；业务层用视觉样式（折叠 / 淡色）表达"这是思考不是答复"，而不是靠 kind 区分。这避免 kind 爆炸。

> 为什么 `table` / `chart` 不是独立 kind：生成式图表是 `artifact-ref` + `ArtifactKind: 'svg' | 'json' | 'html'` 的组合；简单表格用 `record`。真正需要独立 kind 时再扩。

**kind 级安全约束速查**：

- `artifact-ref`：**安全边界由 `ArtifactKind` 决定**，见 §14.3.4.2。业务层渲染 `UiArtifactBlock` 时**必须**先读 `block.artifactKind`（不是只读 `block.kind`）做子路由。
- `markdown` / `text`：渲染器**必须**禁用原始 HTML 直通或等价 `DOMPurify` 过滤；URL 协议白名单仅 `http` / `https` / `mailto`；禁 `javascript:` / `data:` 链接。
- `code` / `diff` / `record` / `list-summary` / `progress`：只读文本路径，默认安全；不得在内嵌字段里接受脚本。
- `error`：`remediation` 仅纯文本；禁止把异常堆栈里的用户输入当 HTML / markdown 信任。
- `raw`：**逃生出口**，业务层默认**拒绝渲染**未白名单的 `mime`，只展示 "`{mime}` · {size}" 占位卡片 + 下载按钮。

## 14.5 Tool 的 5 个 render 钩子

```ts
interface Tool<Input, Output> {
  // ... 03 §3.1.1 既有字段 ...

  render?: RenderLifecycle
}
```

**钩子语义速查**：

| 钩子 | 何时产生 | 典型 block | 是否必填 |
|---|---|---|---|
| `onToolUse` | 模型刚发出 `tool_use`，Harness 正准备执行 | `text` 预览行 / `markdown` 参数摘要 | 可选；不写 = 不在对话流内显示 |
| `onToolProgress` | `execute` yield `ToolProgress` | `progress` / `list-summary` | 可选；流式工具必填 |
| `onToolResult` | 工具成功返回 | `diff` / `code` / `record` / `artifact-ref` | 强烈推荐 |
| `onToolRejected` | 权限拒绝 / 用户 Deny | `text` 简短说明 | 建议填；缺省 Harness 给通用文案 |
| `onToolError` | 工具抛错 / 超时 / 取消 | `error` block | 建议填；缺省 Harness 给通用 `error` 块 |

**示例：`fs_edit` 的 render 契约**（伪代码）：

```ts
render: {
  onToolUse:   () => [{ kind: 'text', text: `编辑 ${input.path}` }],
  onToolResult: (output) => [{
    kind: 'diff',
    path: input.path,
    before: output.before,
    after: output.after,
    language: detectLang(input.path),
  }],
  onToolError: (error) => [{
    kind: 'error',
    message: error.message,
    retriable: error.type !== 'permission',
    remediation: error.remediation,
  }],
}
```

**示例：`ask_user_question` 的 render 契约**：

```ts
render: {
  onToolUse:   () => [],                                          // 询问 UI 由权限引擎接管
  onToolResult: (output) => [{
    kind: 'record',
    title: "用户已回答",
    rows: Object.entries(output.answers).map(([q, a]) => ({ label: q, value: a })),
  }],
  onToolRejected: () => [{ kind: 'text', text: '用户拒绝回答' }],
}

// 与此同时，工具执行会产出一条 AskPrompt（独立顶层类型，由权限引擎渲染为 UiDialog + UiSelectionMenu）
```

**示例：`create_artifact` 的 render 契约**：

```ts
render: {
  onToolUse:   () => [{ kind: 'text', text: `创建成果物：${input.title}` }],
  onToolResult: (output) => [{
    kind: 'artifact-ref',
    artifactId: output.id,
    artifactKind: output.kind,
    title: output.title,
    preview: output.preview,
    version: 1,
  }],
}
```

## 14.6 Claude.ai 风格 ↔ Claude Code 风格：同一 IR 的两种工具策略

这是**本章最关键的一节**：融合两种范式的关键不是"选一个"，而是"同一 IR + 两种工具实现策略"。

| 维度 | Claude.ai 风格 | Claude Code 风格 |
|---|---|---|
| **代表工具** | `create_artifact` / `update_artifact` | `fs_write` / `fs_edit` |
| **存储位置** | `data/artifacts/` + `@octopus/schema::artifact.ts` 元数据 | 直接写磁盘工作区文件（`$projectRoot/**`） |
| **版本追溯** | `/api/v1/projects/{id}/deliverables/{id}/versions` | Git history |
| **对话流内形态** | `RenderBlock kind: 'artifact-ref'` → `UiArtifactBlock`（小卡片） | `RenderBlock kind: 'diff'` → `UiCodeEditor`（内联 diff） |
| **跨 turn 引用** | `ArtifactRef.artifactId` 可作为下一次 `update_artifact` 的 input | 模型重新 `fs_read` 相关文件 |
| **迭代链追溯** | `ArtifactRef.version` + `parentVersion` 构成显式版本链；`fork_artifact` 可从任意历史版本分支出新 artifact；`promote_artifact` 推进 `status` | `git log <path>` / `git diff <sha>..HEAD`；整工作区级快照；分支级分叉 |
| **迭代触发方** | 模型下一轮 `tool_use`（`update_artifact` / `fork_artifact` / `promote_artifact`）；见 §14.12 | 模型下一轮 `tool_use`（`fs_edit` / `fs_write`）；外部 git 工具可离线参与 |
| **侧栏 / 展开** | `UiDialog` + `UiCodeEditor`（可编辑副本） | 无独立侧栏；业务层可点击跳 IDE / 外部编辑器 |
| **适用场景** | 给用户看的**可视成果物**（UI 草图、生成的页面、演示 SVG、可执行 HTML/React） | **工程文件**（源代码、配置、脚本、文档） |

**两种策略可同时启用**（推荐）：

- 模型自主判断语义边界——"给用户看的成果物"走 artifact，"工程落地文件"走 fs_edit。
- SDK / IR 不做任何强制；选择权完全在工具实现方。
- 业务层（桌面）已同时具备 `UiArtifactBlock` 与 `UiCodeEditor`（diff 视图），两种 IR kind 都有归宿。

**反例（要避免）**：

- 把 fs_edit 的 diff 也走 artifact-ref 存进 `data/artifacts/` → 导致工作区文件与 artifact 双头真相源，git 与 artifact 版本分叉。
- 把 artifact 全量内容塞进对话流 `RenderBlock.kind: 'code'` → event log 体积爆炸，token cache 紊乱。

> 成果物双轨在 08 §8.4.4 另有详述；本节侧重"IR 的统一性"，08 章侧重"长时任务中的接力语义"。

## 14.7 宿主映射建议（L4 非规范、仅示例）

本节是**建议**而非规范；各宿主实现方自行决定组件映射。以下表格展示"IR kind 在不同宿主下的典型落点"。

| IR kind | 桌面（`apps/desktop` + `@octopus/ui`） | CLI（`apps/cli` + Ink） | 未来移动（RN） |
|---|---|---|---|
| `text` | `UiListRow` 文本 | 段落 | RN `Text` |
| `markdown` | `@octopus/ui` markdown 渲染块 | Ink `Markdown` | RN markdown 库 |
| `code` | `UiCodeEditor`（只读 + 语法高亮） | Ink SyntaxHighlight | RN CodeHighlighter |
| `diff` | `UiCodeEditor`（diff 视图） | 行内 diff | RN diff 组件 |
| `list-summary` | `UiListRow` + badge | `AgentProgressLine` 风格一行 | RN `List` 折叠 |
| `progress` | `UiListRow` + spinner | `ToolUseLoader` 风格 | RN `ActivityIndicator` |
| `artifact-ref` | `UiArtifactBlock`（打开走 `UiDialog`） | 标题 + 路径链接 | RN 卡片 |
| `record` | `UiRecordCard` | Ink `Box` 表格 | RN `SectionList` |
| `error` | `UiStatusCallout`（error variant） | 红色段落 | RN `Alert` |
| `raw` | 按 `mime` 路由，fallback `UiEmptyState` | mime 提示 + 摘要 | 同桌面 |

**`AskPrompt` 的宿主映射**（独立于 `RenderBlock`，由权限引擎接管）：

| 宿主 | 组件 |
|---|---|
| 桌面 | `UiDialog` + `UiSelectionMenu`（单/多选）+ 可选 `UiCodeEditor`（preview=markdown）/ iframe（preview=html） |
| CLI | Ink `SelectInput` + 预览面板（markdown 走纯文本渲染） |
| 移动 | RN `Modal` + `FlatList` 选项 |

## 14.8 与 09 Observability 的关系

IR 描述符是 09 章 append-only event log 的 payload 一部分：

```text
runtime/events/<session>.jsonl
  ...
  { "type":"tool_use_started",    "toolUseId":"...", "ir":{ "onToolUse":    [ {kind:"text", ...} ] } }
  { "type":"tool_use_progress",   "toolUseId":"...", "ir":{ "onToolProgress":[ {kind:"progress", ...} ] }, "seq":3 }
  { "type":"tool_use_completed",  "toolUseId":"...", "ir":{ "onToolResult": [ {kind:"diff", ...} ] } }
  { "type":"ask_user_presented",  "approvalId":"...", "prompt":{ "kind":"ask-user", ... } }
  { "type":"ask_user_answered",   "approvalId":"...", "answers":{ ... } }
  ...
```

**约束**：

- IR 作为 payload **必须原样写入**；不得在写入前做 UI 层加工（保持 replay 可复现）。
- IR 体积应通过 `artifact-ref.preview` 长度限制 / `diff` 截断策略控制；**禁止** 把整文件 inline 到 IR。
- Replay 时仅需读 event log，**不** 需要重新调模型，即可还原全部 UI 阶段。这是 09 §9 "可观测性全覆盖" 的具体落点。

> 本章只定义 IR 形状；event 类型枚举与 JSONL schema 由 09 章定义。09 章的任何变更**不**回影响本章 IR 形状。

## 14.9 与 12 Plugin 的关系

插件体系（12 §12）与本章 IR 的边界：

| 方面 | 约束 |
|---|---|
| 插件**可以**做什么 | 注册工具 / hook / skill / agent；返回 `RenderLifecycle` + `RenderBlock[]` |
| 插件**不能**做什么 | ① 引入新的 `RenderBlock.kind` 值（必须走 SDK 版本升级）② import 任何 UI 框架 ③ 绕过 L3 IR 直接向业务层传 UI payload |
| 插件应如何应对未建模类型 | 使用 `RenderBlock.kind: 'raw'`（`mime + data`）；业务层按 `mime` 路由；稳定后由 SDK 版本收编为正式 kind |
| 插件 hook 与 `onToolUse` / `onToolError` 的合流 | hook 优先级按 07 §7.6 合并；hook 可返回 `RenderBlock[]` 与工具钩子的产物**拼接**，不是覆盖 |

> 规则来源：`AGENTS.md` §Frontend Governance "do not import unapproved UI libraries directly in business pages"；12 §12.3 扩展点全景。

## 14.10 反模式

严禁以下实现模式：

1. **业务层 `switch(toolName)`**：若业务层里出现任何基于工具名的分支渲染，说明 L2 没有把 `RenderLifecycle` 传全。业务层**只** 应该 `switch(block.kind)`。
2. **IR 里夹带组件**：`RenderBlock` 字段值出现 `React.createElement` / Vue `defineComponent` / `VNode` / 函数指针 / Promise / class 实例 — 一律拒绝。
3. **Artifact 全文塞进对话流**：`artifact-ref.preview` 字段长度超过 2 KB → 拒绝；全量走 HTTP 端点。
4. **kind 无限增长**：本章规定硬上限 15；新增 kind 必须先经 3 个月 `raw` 孵化期。
5. **AskPrompt 合并进 `RenderBlock`**：有人会想"能不能把 `ask-user` 做成一种 `RenderBlock.kind`？" — **不行**，因为它有"阻塞 / 回写 tool_result"语义，与纯展示性 block 不同质（见 §14.3.3 解释）。
6. **工具直接写前端状态**：工具**只** 通过 IR 向业务层传意图；不得直接调 Pinia store / Vue emit / window 事件。
7. **业务页硬编码 artifact 类型**：业务层应该根据 `ArtifactKind` 路由到不同渲染器（`markdown` → markdown 组件、`svg` → `<img>` + `blob`、`react` → 沙箱 iframe）；**不**应该根据 `artifactId` 前缀或工具名分支。
8. **桌面 UI 组件深导入**：业务页 `import ... from '@octopus/ui/src/components/...'` 违反 `AGENTS.md` §Frontend Governance "no deep import"；必须走公共导出面。

## 14.11 Octopus 实施约束

- 本章是**规范层**。代码落地（`packages/schema/src/ui-intent.ts` 的 Zod 实现、业务层 dispatcher 骨架）**另起 plan**，不在本轮 scope。
- 落地前，03 §3.1.1 的 `ToolDisplayDescriptor` 保留作为"历史占位"字段；落地时由 `RenderLifecycle` 替代。
- 所有 IR 字段变更必须同步 13 §13.3 的 "UIIntent kinds" 列与 §13.9 历史修订。
- IR 的版本化策略与 `@octopus/schema` 保持一致：重大变更走 major 版本号；minor 只允许**追加** kind / 字段，不允许删。
- `AGENTS.md` §Frontend Governance 对"`@octopus/ui` Catalog 可用组件"的清单是 L4 桌面映射的**约束集**；若 IR 新增 kind 无现成组件，**先**在 `@octopus/ui` 扩展组件，**再**放出新 kind。

## 14.12 Artifact 迭代工具套件（Claude.ai 风格的落地）

"模型写入后可被后续 turn 引用迭代"不是本章新概念，而是靠一组**固定**工具把 §14.3.4 `ArtifactRef` 串成版本链。这些工具的 input / output **必须**对齐 `contracts/openapi/src/components/schemas/projects.yaml` 的既有 schema，禁止虚构字段。

### 14.12.1 六件套一览

| 工具 | 作用 | 对齐 HTTP | input schema | output schema |
|---|---|---|---|---|
| `create_artifact` | 创建全新 artifact（version=1） | `POST /api/v1/projects/{id}/deliverables` | `{ title, artifactKind, contentType?, textContent? \| dataBase64? }` + 隐含 `previewKind`（映射表见 §14.3.4.1） | `DeliverableSummary` → IR `ArtifactRef{ version: 1 }` |
| `read_artifact` | 拉取指定版本正文 | `GET .../deliverables/{id}/versions/{v}/content` | `{ artifactId, version? }`（省略=latest） | `DeliverableVersionContent` |
| `update_artifact` | 追加新版本 | `POST .../deliverables/{id}/versions` | `CreateDeliverableVersionInput`（含 `parentVersion`） | `DeliverableVersionSummary` → IR `ArtifactRef{ version: v+1, parentVersion: v }` |
| `list_artifacts` | 罗列当前 session / project 的 artifacts | `GET .../projects/{id}/deliverables?conversationId=...` | `{ conversationId?, projectId?, limit? }` | `DeliverableSummary[]`；见 §14.13 |
| `fork_artifact` | 从任意历史版本分叉新 artifact | `POST .../deliverables/{id}/fork` | `ForkDeliverableInput`（`{ projectId?, title? }`）+ `?fromVersion=<v>` | 新 `DeliverableDetail`（`parentArtifactId` 指回原 artifact） |
| `promote_artifact` | 晋升到项目知识库 | `POST .../deliverables/{id}/promote` | `PromoteDeliverableInput`（`{ kind, summary?, title? }`） | 更新后的 `DeliverableDetail`（`promotionState: promoted` + `promotionKnowledgeId`） |

> 工具**只**是上述 HTTP 端点的 schema 化封装；不得在工具实现里自造额外的 artifact 存储或元数据路径（违反 `AGENTS.md` §Persistence Governance"避免在 DB 行与 ad-hoc export 文件间复制 blob"）。

### 14.12.2 render 契约（每件套的 `RenderLifecycle`）

以下是**推荐**契约；工具可加严不可放宽：

```ts
// create_artifact
render: {
  onToolUse:    () => [{ kind: 'text', text: `创建成果物：${input.title}` }],
  onToolResult: (out) => [{
    kind: 'artifact-ref',
    artifactId:  out.id,
    artifactKind: out.artifactKind,
    title:       out.title,
    preview:     out.latestVersionRef?.title,
    version:     1,
    status:      out.status,
    contentType: out.contentType,
  }],
}

// update_artifact
render: {
  onToolUse:    () => [{ kind: 'text', text: `更新 ${input.artifactId} → v${(input.parentVersion ?? 0) + 1}` }],
  onToolResult: (out) => [{
    kind: 'artifact-ref',
    artifactId:    out.artifactId,
    artifactKind:  input.artifactKind,
    title:         out.title,
    preview:       out.title,
    version:       out.version,
    parentVersion: out.parentVersion,   // 用于 vN-1 → vN 链路渲染
  }],
}

// fork_artifact
render: {
  onToolUse:    () => [{ kind: 'text', text: `从 ${input.artifactId} 分叉新 artifact` }],
  onToolResult: (out) => [{
    kind: 'artifact-ref',
    artifactId:   out.id,               // 新 artifactId
    artifactKind: out.artifactKind,
    title:        out.title,
    version:      out.latestVersion,
    // parentArtifactId 不在 ArtifactRef 字段上；由业务层从 DeliverableDetail 读
  }],
}

// promote_artifact
render: {
  onToolUse:    () => [{ kind: 'text', text: `晋升到知识库：${input.title ?? '(原标题)'}` }],
  onToolResult: (out) => [{
    kind: 'record',
    title: '已晋升到知识库',
    rows: [
      { label: '知识 ID',  value: out.promotionKnowledgeId ?? '-' },
      { label: '状态',     value: out.promotionState },
      { label: 'Artifact', value: out.id, href: `/api/v1/projects/${out.projectId}/deliverables/${out.id}` },
    ],
  }],
}

// read_artifact / list_artifacts：见 §14.13 与 §14.5 record 示例
```

**要点**：
- `update_artifact` 返回的 `parentVersion` 是版本链关键字段，业务层用它做"vN-1 → vN"可视化（`UiTimelineList` 或 `UiArtifactBlock` 版本标签）。
- `fork_artifact` 产出的**新 artifact 独立计数 version**（新 artifactId 的 version 从 1 起），与原 artifact 版本链**脱钩**；业务层展示时应标注 "分叉自 {parentArtifactId}@{fromVersion}"。
- 工具**不**直接触发 `supersededByVersion` 回写；该字段只由 Harness 在对话流投影层写（§14.8），以免形成循环引用。

### 14.12.3 版本 / Status / Promotion 三条正交轴

Octopus artifact 有**三条互相正交**的状态轴，工具实现必须分清：

| 轴 | 取值 | 由谁推进 | 流转规则 |
|---|---|---|---|
| **版本号**（`version`） | `1, 2, 3, ...` 单调递增 | `update_artifact`（+1）/ `fork_artifact`（新 artifactId 从 1 起） | append-only，不可改旧版本正文 |
| **内容状态**（`ArtifactStatus`） | `draft → review → approved → published` | 业务层 UI 操作 + 后端校验；工具层通过 `update_artifact` 可选携带 status 字段（若 schema 放开） | 单向前进；`published` 不可直接编辑，需 `fork_artifact` 产生副本 |
| **知识晋升**（`DeliverablePromotionState`） | `not-promoted → candidate → promoted` | `promote_artifact` 专管 | `promoted` 后会同步生成 `KnowledgeItem`，`promotionKnowledgeId` 双向指向 |

**模型何时该用哪条轴**：
- 内容有重大修订 → `update_artifact`（版本 +1，status 可维持 draft）
- 方案定稿、需评审 → 业务层推进 status 到 `review`，工具不直接触发
- 发布后想继续改 → `fork_artifact`（新 artifactId + 新版本链）
- 成果达到可复用水准 → `promote_artifact` 写入知识库

### 14.12.4 与 `fs_edit` 的互不干扰

六件套**只**管 `data/artifacts/*`，**不触碰**工作区文件（`$projectRoot/**`）。若模型需要把 artifact 内容"落盘为工程文件"，应当：
1. `read_artifact` 拿到 `textContent`
2. `fs_write` 写到 `$projectRoot/<path>`（权限走 06 §6）
3. **不**在 `fs_write` 的 render 里再塞 `artifact-ref`；避免双真相源

反之同理：`fs_edit` 不得产出 `artifact-ref`。08 §8.4.4"artifact-ref 风格 ↔ file diff 风格"的边界即落点于此。

## 14.13 跨轮 Artifact 发现（让模型在下一 turn 找到上轮的产物）

模型要"引用上 turn 的 artifact"有两条路径，**必须同时提供**：

### 14.13.1 显式查询：`list_artifacts` 工具

模型主动发起 `tool_use`（见 §14.12 表）。默认参数按 `conversationId` 过滤当前会话范围；`projectId` 过滤跨会话项目范围。输出按 `updatedAt DESC` 排序。

```ts
// list_artifacts
render: {
  onToolUse:    () => [{ kind: 'text', text: `列出本会话 artifacts` }],
  onToolResult: (out) => [{
    kind: 'record',
    title: '当前 artifacts',
    rows: out.map((a) => ({
      label: `${a.title} (v${a.latestVersion})`,
      value: `${a.artifactKind} · ${a.status} · ${a.promotionState}`,
      href:  `/api/v1/projects/${a.projectId}/deliverables/${a.id}`,
    })),
  }],
}
```

### 14.13.2 隐式注入：`SessionStart` hook 自动补提示

在会话开始或 compaction 恢复时，`SessionStart` hook（07 §7.6）**应**把"本会话已有的 artifacts 简表"注入 system message，避免模型反复调 `list_artifacts`。简表格式**必须**轻量，**禁止**塞全量正文。

**推荐字段**（每个 artifact 一行，JSON 或简短 markdown 均可）：

| 字段 | 必填 | 说明 | context 成本 |
|---|---|---|---|
| `artifactId` | 是 | `DeliverableSummary.id` | ~30 token |
| `title` | 是 | 简短标题 | ≤ 40 token（超长截断） |
| `artifactKind` | 是 | §14.3.4 枚举 | ~5 token |
| `latestVersion` | 是 | 最新版本号 | ~3 token |
| `status` | 是 | `ArtifactStatus` | ~5 token |
| `updatedAt` | 是 | Unix 秒 | ~10 token |
| `preview` | **否** | **绝不**在此处塞正文，需要时模型走 `read_artifact` | 0 |

**每个 artifact 成本约 ~100 token**；简表硬上限建议 **20 个最近 artifact**（≈ 2 K token），超出部分模型必须走 `list_artifacts` 主动查询。这与 02 §2.2 "token 预算"约束一致。

**注入示例**（写入 `runtime/events/<session>.jsonl` 的 `session_start` 事件 + 合流进首 turn system 字段）：

```text
## Artifacts in this conversation (top 20, sorted by updatedAt DESC)
- art_01H7X9... | "登录页原型" | react | v3 | draft | 2026-04-20T10:15:00Z
- art_01H7X8... | "架构图" | mermaid | v2 | approved | 2026-04-20T09:42:00Z
- art_01H7X7... | "需求文档" | markdown | v5 | review | 2026-04-19T22:30:00Z
...
```

### 14.13.3 Compaction 幸存规则

02 §2.5 compaction 生成 `runtime/notes/<session>.md` 时，`list_artifacts` 简表**必须**写入 notes 的固定段落（建议 heading `## 已产出 artifacts`）；compaction 后的第一 turn 由 `SessionStart` hook 重新注入该段。这保证"长任务接力"（08 §8.4.4）不会丢失 artifact 存在感知。

### 14.13.4 权限边界

- `list_artifacts`：查询级，默认无需用户审批；但结果的 `artifactId` 模型拿到后调 `read_artifact` / `update_artifact` 时**必须**走对应权限策略（06 §6）。
- 跨 project 列出 artifacts 需显式 `projectId`，不得泛查全租户；否则违反 `AGENTS.md` §Persistence Governance 的数据边界。
- 简表注入**不**等价于授予模型"可以读全部 artifact 内容"：模型仍需为每个 `read_artifact` 触发单独的权限检查。

---

## 参考来源（本章）

| 来源 | 用途 |
|---|---|
| Claude Code `restored-src/src/tools/AskUserQuestionTool/AskUserQuestionTool.tsx` | `AskPrompt` 的 schema 约束（1–4 问、2–4 选项、preview 片段） |
| Claude Code `restored-src/src/tools/AgentTool/UI.tsx` | 5 个渲染钩子的职责划分与进度聚合（`processProgressMessages`） |
| Claude Code `restored-src/src/Tool.ts` | `renderToolUseMessage / renderToolResultMessage / ...` 的原始形状（返回 `React.ReactNode`，本章以 `RenderBlock[]` 替代） |
| `AGENTS.md` §Frontend Governance | Shared UI Component Catalog、deep import 禁令、接入顺序（`@octopus/ui` → `shadcn-vue` → Reka UI） |
| `AGENTS.md` §Persistence Governance | Artifact 存 `data/artifacts/` + 元数据入 SQLite；IR 作为 event payload 落 `runtime/events/*.jsonl` |
| 本仓 `docs/sdk/03-tool-system.md` §3.1.1 | `ToolDisplayDescriptor` 占位的伏笔 |
| 本仓 `docs/sdk/06-permissions-sandbox.md` §6.2–6.6 | 权限引擎接管 `AskPrompt` 渲染、审批 UI 归属 |
| 本仓 `docs/sdk/08-long-horizon.md` §8.4.4 | 成果物双轨：artifact-ref ↔ file diff |
| 本仓 `docs/sdk/09-observability-eval.md` | IR 作为 event payload 的写入协议（09 章定义 event 形状） |
| 本仓 `docs/sdk/11-model-system.md` §11.6 | Protocol Adapter + Canonical Message IR（L1 → L2 的上游） |
| 本仓 `docs/sdk/12-plugin-system.md` §12.3 / §12.10 | 插件扩展点全景 + 安全沙箱 |
| 本仓 `docs/sdk/13-contracts-map.md` §13.2 / §13.3 | 第四条真相源 "UI 意图 IR" 登记 |
