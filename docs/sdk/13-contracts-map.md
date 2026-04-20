# 13. 契约地图 · SDK 章节 ⇄ OpenAPI ⇄ Schema ⇄ UI

> 本章是**索引文档**，不引入任何新规范。所有规则来源于 `AGENTS.md` §Request Contract Governance / §Frontend Governance、`contracts/openapi/AGENTS.md` 与 `docs/api-openapi-governance.md`；本文只负责把 `docs/sdk/01–12` 与磁盘上的真相源钉在一起，降低读者与 AI Coding Agent 的定位成本。

> 快照口径：本章基于 **2026-04-20** 仓库实际磁盘状态；任何真相源变更时必须同步追加本章末尾的"历史修订"条目。

## 13.1 目的与非目的

**目的**：

- 给 SDK 01–12 的每一章提供"这一章的契约落在哪里 / UI 落在哪里"的单跳入口
- 给 OpenAPI paths / Schema / UI 组件提供"这份文件被 SDK 哪些章节用"的反向索引
- 显式标注"暂无 HTTP 契约"的 SDK 能力，避免被误以为是遗漏

**非目的**：

- 不复述 `AGENTS.md` 的治理规则（仅引用）
- 不重新定义任何路径 / schema / UI 契约
- 不做 1:1 端点级映射（颗粒度为**章节** × **资源组**，每行 ≤ 3 个代表性目标）
- 不替代代码搜索；本章是"入口"，具体字段以源文件为准

## 13.2 四条真相源总览

| 真相源 | 路径 | 规模（2026-04-20 快照） | 治理 |
|---|---|---|---|
| **OpenAPI paths** | `contracts/openapi/src/paths/*.yaml` | 9 份 path 文件 | `contracts/openapi/AGENTS.md`；变更顺序：`src/**` → `pnpm openapi:bundle` → `pnpm schema:generate` |
| **OpenAPI components** | `contracts/openapi/src/components/{schemas,parameters,responses}/*.yaml` | 10 schemas + common/errors | 同上；大 payload 须提升到 `components/schemas`，不得内联 |
| **`@octopus/schema`** | `packages/schema/src/*.ts` | 32 份 feature-based TS + `generated.ts` | `AGENTS.md` §Frontend Governance："Shared schemas must be defined in feature-based files"；`index.ts` 只做公共导出面 |
| **`@octopus/ui`** | `packages/ui/src/components/*.vue` | 64 份 `Ui*.vue` + `tokens.css` | `AGENTS.md` §Frontend Governance：Shared UI Component Catalog；不得深导入 `src/*` |
| **UI 意图 IR** | `docs/sdk/14-ui-intent-ir.md`（规范层）+ 未来 `packages/schema/src/ui-intent.ts`（代码层，另起 plan） | 10 种 `RenderBlock.kind` + 4 顶层类型（`RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactRef`） | 14 §14.2 / §14.9 + `AGENTS.md` §Frontend Governance；kind 新增走 SDK 版本升级，**不**经插件 |
| **OpenAPI bundle（生成物）** | `contracts/openapi/octopus.openapi.yaml` | 单文件 | **禁止手改**；由 `pnpm openapi:bundle` 输出 |
| **Schema generated（生成物）** | `packages/schema/src/generated.ts` | 单文件 | **禁止手改**；由 `pnpm schema:generate` 输出 |

> 9 份 paths 分别是：`access-control.yaml`、`catalog.yaml`、`host.yaml`、`misc.yaml`、`projects.yaml`、`runtime.yaml`、`system-auth.yaml`、`tasks.yaml`、`workspace.yaml`。

> 四条真相源的分工：前三条（OpenAPI paths / components / `@octopus/schema`）定义 HTTP 契约与数据形状；`@octopus/ui` 定义**组件实现**；**UI 意图 IR** 定义从 harness 流向业务层的**渲染意图契约**——SDK 产出 IR，业务层产出组件，两者解耦。详见 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.2 四层分层。

## 13.3 SDK 章节 → 契约 × Schema × UI × UI 意图 IR 主映射

颗粒度：章节级；每行最多列 3 个代表性目标。未列出不等于无关，必要时请以源文件为准。

"UIIntent kinds"列登记**该章 harness 执行时可能产出**的代表性 `RenderBlock.kind`（及独立顶层类型 `AskPrompt` / `ArtifactRef`，缩写为 `ask-prompt` / `artifact-ref`）。完整枚举见 [`14-ui-intent-ir.md`](./14-ui-intent-ir.md) §14.4。

| SDK 章节 | OpenAPI paths（代表性前缀） | OpenAPI 组件 schemas | `@octopus/schema`（ts） | `@octopus/ui`（代表性组件） | UIIntent kinds（代表性） | 备注 |
|---|---|---|---|---|---|---|
| **01 Core Loop** | `runtime.yaml` `/api/v1/runtime/sessions`、`/sessions/{id}/events`、`/sessions/{id}/turns` | `runtime.yaml`、`shared.yaml` | `runtime.ts`、`runtime-config.ts`、`task.ts` | — | `progress` · `text` | 核心循环是 harness 内部机制；HTTP 层只暴露 session / turn / event 的读取入口 |
| **02 Context Engineering** | `runtime.yaml` `/sessions/{id}/events`、`/memory-proposals/{proposalId}` | `runtime.yaml`（memory proposal schemas） | `memory-runtime.ts`、`knowledge.ts`、`runtime.ts` | — | `raw`（memory proposal 专属） | Write / Select / Compress / Isolate 四操作中仅 Write（记忆提议）与 Select（知识检索）有 HTTP 暴露 |
| **03 Tool System** | `catalog.yaml` `/workspace/catalog/tools`、`/workspace/catalog/mcp-servers`、`/workspace/catalog/skills` | `catalog.yaml` | `catalog.ts`、`shell.ts`、`capability-runtime.ts` | `UiInboxBlock`、`UiCodeEditor`、`UiDataTable` | `text` · `markdown` · `code` · `diff` · `record` · `error` · `ask-prompt` | MCP Server 走 catalog；工具调用本身是 harness 内部（见 §13.7） |
| **04 Session / Brain / Hands** | `runtime.yaml` `/sessions`、`/sessions/{id}`（含快照/恢复语义） | `runtime.yaml`、`shared.yaml` | `runtime.ts`、`agent-runtime.ts`、`runtime-config.ts` | `UiTimelineList`、`UiRecordCard` | `progress` · `record` | `config_snapshot_id` / `effective_config_hash` 经 `runtime-config` 装载 |
| **05 Sub-agents** | `runtime.yaml` `/sessions/{id}/subruns/{subrunId}/cancel` + `catalog.yaml` agents | `runtime.yaml`、`catalog.yaml` | `agent-runtime.ts`、`team-runtime.ts`、`runtime.ts` | `UiHierarchyList`、`UiRecordCard` | `progress` · `list-summary` · `record` | 子代理身份与编排经 agent-runtime 载入；`list-summary` 对应 `AgentTool/UI.tsx` 的连续工具聚合 |
| **06 Permissions & Sandbox** | `access-control.yaml` 全部 + `runtime.yaml` `/sessions/{id}/approvals/{approvalId}`、`/auth-challenges/{challengeId}` | `access-control.yaml`、`auth.yaml`、`runtime.yaml` | `access-control.ts`、`permissions.ts`、`governance.ts`、`auth.ts` | `UiDialog`、`UiSelectionMenu`、`UiContextMenu` | `ask-prompt` · `error` | Rules by Source 与审批提示；`ask-prompt` 顶层由权限引擎接管渲染 |
| **07 Hooks / Lifecycle** | **无专属 HTTP**（见 §13.7） | — | `runtime-config.ts`（hooks 字段）、`agent-runtime.ts` | — | `raw`（hook 响应多样；默认空） | 纯 harness + 文件配置；`config/runtime/*.json` 是配置源 |
| **08 Long-horizon** | `runtime.yaml` `/sessions` + `projects.yaml` deliverables（含 `/versions` / `/fork` / `/promote`）+ `tasks.yaml` | `projects.yaml`、`tasks.yaml` | `artifact.ts`、`task.ts`、`observation.ts` | `UiArtifactBlock`、`UiTraceBlock`、`UiTimelineList` | `artifact-ref` · `diff` · `record`（artifact 列表） | 成果物双轨：artifact-ref（Claude.ai 式）与 diff（Claude Code 式）共存，见 08 §8.4.4 / 14 §14.6；**artifact 版本链 / fork / 跨轮引用** → 14 §14.12 / §14.13 |
| **09 Observability / Eval** | `runtime.yaml` events + `access-control.yaml` audit | `runtime.yaml`、`access-control.yaml` | `observation.ts`、`updates.ts`、`transport-records.ts` | `UiTraceBlock`、`UiTimelineList`、`UiMetricCard` | `progress` · `record` | tracing 读取入口；写入走 `runtime/events/*.jsonl` append-only；IR 作为 event payload 一部分原样写入 |
| **10 Failure Modes** | 跨章（无专属） | 跨章 | 跨章 | 跨章 | `error` | Checklist 型章节，关注"哪些 API/schema/组件需要守住边界" |
| **11 Model System** | `catalog.yaml` `/workspace/catalog/models`、`/workspace/catalog/provider-credentials` + `runtime.yaml` `/config/configured-models/probe` | `catalog.yaml`、`runtime.yaml` | `catalog.ts`、`runtime-config.ts` | `UiDataTable`、`UiSelect`、`UiInspectorPanel` | `record` · `error` | Provider / Surface / Model 三层；provider 凭据 UI |
| **12 Plugin System** | `catalog.yaml` mcp-servers / skills / tools / agents + `management-projection` | `catalog.yaml`、`shared.yaml` | `catalog.ts`、`runtime-config.ts`、`governance.ts` | `UiInspectorPanel`、`UiDataTable`、`UiBadge` | `record` · `error` | built-in plugin（例如 MCP server）在 catalog 下；第三方分发形态见 §12.6；插件**不得**扩 IR kind |

> 规则提醒：**任何 `/api/v1/*` 新增或修改**都必须走 `contracts/openapi/src/** → pnpm openapi:bundle → pnpm schema:generate → adapter/store/server/tests`。直接改生成物是禁止的。详见 `docs/api-openapi-governance.md`。

## 13.4 OpenAPI paths × SDK 章节（反向索引）

| OpenAPI path 文件 | 路径前缀示例 | 被哪些 SDK 章节涉及 |
|---|---|---|
| `access-control.yaml` | `/api/v1/access/**`（users/org/roles/policies/menus/audit/sessions） | 06 Permissions / 09 Observability（audit） |
| `catalog.yaml` | `/api/v1/workspace/agents` · `/workspace/catalog/{mcp-servers,skills,tools,models,provider-credentials,management-projection}` | 03 Tools / 05 Sub-agents / 11 Models / 12 Plugins |
| `host.yaml` | `/api/v1/host/**`（bootstrap/health/notifications/preferences/update/workspace-connections） | 宿主集成（跨 04/09；参见各 host 通道实现） |
| `misc.yaml` | `/api/v1/apps` · `/inbox` · `/knowledge` | 02 Context（knowledge）；按 `contracts/openapi/AGENTS.md` §misc 规则，**不**把新功能默认塞入 misc |
| `projects.yaml` | `/api/v1/projects/**`（含 deliverables / resources / runtime-config / pet / team-links） | 04 Session / 08 Long-horizon / 11 Runtime Config |
| `runtime.yaml` | `/api/v1/runtime/**`（bootstrap / config / sessions / events / turns / approvals / auth-challenges / subruns / memory-proposals / generations） | 01 Loop / 02 Context / 04 Session / 05 Sub-agents / 06 Permissions / 09 Observability |
| `system-auth.yaml` | `/api/v1/system/**`（bootstrap/health/auth/login/session） | 06 Permissions（登录）/ 04 Session（bootstrap） |
| `tasks.yaml` | `/api/v1/projects/{id}/tasks/**`（launch / rerun / runs / interventions） | 08 Long-horizon / 05 Sub-agents（intervention）/ 09 Obs（runs） |
| `workspace.yaml` | `/api/v1/workspace/**`（overview / deliverables / knowledge / promotion-requests / pet / resources / teams / personal-center / filesystem） | 04 Session / 08 Long-horizon / 11 Runtime Config（个人中心） |

## 13.5 `@octopus/schema` 文件 × SDK 章节（反向索引）

仅列 feature-based `.ts`；`index.ts` 与 `generated.ts` 不出现在此表（前者只导出、后者禁止手改）。

| Schema 文件 | 主要职责 | 被哪些 SDK 章节使用 |
|---|---|---|
| `access-control.ts` | 用户 / 组织 / 角色 / 权限 / 菜单 / 会话 | 06 Permissions |
| `actor-manifest.ts` | 参与者元数据 | 05 Sub-agents / 12 Plugins（身份） |
| `agent-import.ts` · `agent-runtime.ts` | 代理导入 / 运行时身份 | 05 Sub-agents / 12 Plugins |
| `app.ts` | 应用级元数据 | 12 Plugins（shape=app） |
| `artifact.ts` | 制品 / 交付物 | 08 Long-horizon |
| `asset-bundle.ts` | 资产包 | 12 Plugins（bundled 分发） |
| `auth.ts` | 认证凭据 / 挑战 | 06 Permissions |
| `capability-runtime.ts` | 能力运行时 | 03 Tools / 12 Plugins |
| `catalog.ts` | agents / mcp-servers / skills / tools / models 目录 | 03 Tools / 11 Models / 12 Plugins |
| `governance.ts` | 治理规则 / 审计 | 06 Permissions / 09 Obs |
| `knowledge.ts` | 知识库条目与候选 | 02 Context（仅 `ConversationMemoryItem` / `Knowledge*` 类型显式导出，其余内部） |
| `memory-runtime.ts` | 记忆运行时 | 02 Context |
| `notifications.ts` | 通知 | host 层 / 跨章 |
| `observation.ts` | 观测事件 / tracing | 09 Observability |
| `permissions.ts` | 权限裁决 | 06 Permissions |
| `project-settings.ts` | 项目设置 | 04 Session / 11 Runtime Config |
| `runtime-config.ts` | 三层 runtime config（user < workspace < project） | 04 Session / 07 Hooks / 11 Models / 12 Plugins |
| `runtime-policy.ts` | 运行时策略 | 06 Permissions / 09 Obs |
| `runtime.ts` | session / run / turn / event 顶层 | 01 Loop / 04 Session / 05 Sub-agents |
| `shared.ts` | 跨 feature 复用原语 | 全章引用 |
| `shell.ts` | shell_exec 相关 schema | 03 Tools / 06 Permissions |
| `task.ts` | 任务定义 / 进度 | 08 Long-horizon |
| `team-runtime.ts` | 团队运行时 | 05 Sub-agents |
| `transport-records.ts` | 传输日志 / 错误记录 | 09 Observability |
| `updates.ts` | host 更新通道 | host 层（跨章） |
| `workbench.ts` | 工作台视图投影 | 04 Session / 11 Models（视图层） |
| `workflow-runtime.ts` | 工作流运行时 | 05 Sub-agents（若走工作流编排） |
| `workspace.ts` · `workspace-plane.ts` · `workspace-protocol.ts` | 工作区根与 plane / 协议 | 04 Session / 09 Obs / 12 Plugins |

## 13.6 `@octopus/ui` Catalog × SDK 章节

本节的 Catalog **以 `AGENTS.md` §Frontend Governance 列表为准**（它是规约）；磁盘上的 64 个 `Ui*.vue` 是实现（有的规约未列、属扩展组件）。业务页必须走 `@octopus/ui` 公共导出，不得深导入 `src/*`。

| Catalog 类别（来自 `AGENTS.md`） | 代表性组件 | 主要被 SDK 哪些章节涉及 |
|---|---|---|
| **Base** | `UiButton` · `UiInput` · `UiTextarea` · `UiCheckbox` · `UiSwitch` · `UiSelect` · `UiRadioGroup` · `UiSectionHeading` | 全章 UI |
| **Layout** | `UiSurface` · `UiPanelFrame` · `UiToolbarRow` · `UiPageHero` · `UiNavCardList` | 全章 UI |
| **Data Display** | `UiBadge` · `UiEmptyState` · `UiMetricCard` · `UiRankingList` · `UiTimelineList` · `UiRecordCard` · `UiListRow` · `UiStatTile` · `UiPagination` | 04 Session · 08 Long-horizon · 09 Obs · 11 Models · 12 Plugins |
| **Context Blocks** | `UiArtifactBlock` · `UiTraceBlock` · `UiInboxBlock` | 03 Tools · 08 Long-horizon · 09 Obs |
| **Composite** | `UiDialog` · `UiPopover` · `UiDropdownMenu` · `UiCombobox` · `UiTabs` · `UiAccordion` · `UiContextMenu` · `UiSelectionMenu` · `UiDataTable` · `UiVirtualList` | 06 Permissions（Dialog / SelectionMenu）· 11 Models（DataTable / Select） |
| **Media/Editor** | `UiCodeEditor` · `UiIcon` · `UiDotLottie` · `UiRiveCanvas` | 03 Tools（CodeEditor）· motion policy（Lottie/Rive） |

> 接入顺序（来自 `AGENTS.md`）：**`@octopus/ui` 已有 → 缺则在 `@octopus/ui` 扩展 → 参考 `shadcn-vue` 但落在 `@octopus/ui`**；`Dialog` / `Popover` / `DropdownMenu` / `Combobox` / `Tabs` / `Accordion` / `ContextMenu` 必须基于 Reka UI 原语。

## 13.7 暂无 HTTP 契约的 SDK 能力（harness-internal）

以下能力是 **harness 内部机制**，**不**在 `/api/v1/*` 暴露；对它们的接入发生在 SDK 内部（`packages/agent-*` 后续包）或者本地配置文件，而**不**走 openapi。

| SDK 能力 | 所属章节 | 入口 |
|---|---|---|
| Agent 核心循环 (query loop / tool loop) | 01 | harness in-process；事件经 `runtime.yaml` events 暴露结果 |
| Prompt 构建 + 分段装配 | 01 · 02 | harness in-process |
| Prompt Cache 稳定性策略 | 01 · C1 约束 | harness in-process；工具顺序决定性 |
| Compaction / tool-result clearing | 02 | harness in-process；经 `PreCompact` / `PostCompact` hooks |
| Tool 并发批处理 (`partitionToolCalls`) | 03 | harness in-process；`isConcurrencySafe` 自报 |
| Hook 注入点（`PreToolUse` / `Stop` 等） | 07 | `config/runtime/*.json` + `packages/hooks`；**无** HTTP |
| Canonical Naming / Protocol Adapter / Canonical Message IR | 11 | `packages/agent-core` 范畴；可能暴露 catalog 视图，但 adapter 本身无 HTTP |
| Slot 选择 / 依赖解析 / `plugins doctor` | 12 | CLI + `config/runtime`；诊断面板经 `management-projection` 暴露 |
| Sandbox（bubblewrap / seatbelt） | 06 | OS 级；无 HTTP |
| Git Proxy（凭据零暴露） | 06 | 子进程代理；无 HTTP |
| UI 意图 IR（`RenderBlock` / `AskPrompt` / `ArtifactRef`） | 14 | harness in-process；作为 event log payload 一部分经 `runtime.yaml` `/sessions/{id}/events` **间接**暴露；**本身无专属 HTTP 路径**（替代原始 tool_use 数据的宿主中立形状） |

## 13.8 变更规则

- 改动任一真相源时**必须**追加本章末尾的"历史修订"条目；仅加一行（日期 + 简述 + 影响行）。
- 本章与 11 §11.17 / 12 §12.15 等章内的"Octopus 落地约束"应保持**语义一致**；若出现冲突以章内约束为准，并在本章追加修订。
- 本章**不做**自动校验；未来可由 CI 脚本对齐 `contracts/openapi/src/paths/**` × `docs/sdk/13-contracts-map.md` 的行范围（属后续优化，非本轮 Scope）。

## 13.9 历史修订

| 日期 | 影响行 | 摘要 |
|---|---|---|
| 2026-04-20 | 全章 | 首次落地：基于仓库当前快照（9 份 paths + 32 份 schemas + 64 份 Ui 组件）建立 SDK ⇄ OpenAPI ⇄ Schema ⇄ UI 双向索引；标注 10 项"harness-internal"能力为无 HTTP 契约 |
| 2026-04-20 | §13.2 / §13.3 / §13.7 | 纳入第四条真相源 **UI 意图 IR**（`docs/sdk/14-ui-intent-ir.md`）；§13.3 主映射表 +1 列 "UIIntent kinds"，§13.7 harness-internal 清单 +1 行 |
| 2026-04-20 | §13.3（08 行） | 14 §14.12 Artifact 迭代六件套（create / read / update / list / fork / promote）+ §14.13 跨轮发现机制落地；13 §13.3 08 行备注补"版本链 / fork / 跨轮引用 → 14 §14.12 / §14.13"，UIIntent kinds 追加 `record`（artifact 列表） |

## 参考来源（本章）

| 来源 | 用途 |
|---|---|
| `AGENTS.md` §Request Contract Governance | `/api/v1/*` 变更强制顺序 |
| `AGENTS.md` §Frontend Governance | `@octopus/schema` feature-based、`@octopus/ui` Catalog、deep import 禁令 |
| `AGENTS.md` §Persistence Governance | 为何部分 SDK 能力走文件而非 HTTP（见 §13.7） |
| `contracts/openapi/AGENTS.md` | paths / components 分层规则 |
| `docs/api-openapi-governance.md` | bundle / schema:generate 流程 |
| `docs/sdk/references.md` | 与 SDK 章节对应的外部一级来源 |
