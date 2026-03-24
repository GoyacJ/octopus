# octopus 开发规范（Development Standards）v1.0

更新时间：2026-03-24  
文档状态：Draft  
文档定位：强制型工程基线  
适用范围：`octopus` 单仓工程、控制平面、运行时内核、Web/H5、Desktop、Mobile、插件 SDK 与配套工具链  
关联文档：`docs/PRD.md`、`docs/SAD.md`

---

## 1. 文档目的

本文档定义 `octopus` 项目的统一开发规范，作为当前项目第一份正式工程基线。后续任何代码、脚手架、CI、目录结构、组件体系、数据库迁移、协议契约与交付流程，均必须遵守本规范。

本文档的目标是：

1. 锁定项目的完整技术栈选型。
2. 锁定项目的工程组织方式、设计模式与代码风格。
3. 锁定统一的 UI/UX、tokens、国际化与主题系统规范。
4. 锁定数据库兼容策略、接口契约策略、测试门禁与提交规范。
5. 避免在项目初期因个人偏好、工具摇摆或目录失控造成后续重构成本。

本规范使用以下术语：

- `必须`：强制要求，不允许绕过。
- `禁止`：明确禁用。
- `推荐`：默认做法，若偏离必须在 ADR 或设计文档中说明原因。
- `例外`：经架构评审批准后可以偏离的情况。

---

## 2. 总体约束

### 2.1 项目级工程决策

`octopus` 的工程基线固定为：

1. 单仓 `Monorepo`。
2. Rust Core 负责控制平面与运行时内核。
3. 前端和客户端统一使用 Vue 3 生态。
4. Desktop 与 Mobile 统一采用 Tauri 2。
5. 插件 SDK 采用 TypeScript-first。
6. 后端形态采用模块化单体，而非微服务优先。
7. 本地默认数据库为 `SQLite`，生产推荐数据库为 `PostgreSQL`。
8. 默认本地对象存储基线为文件系统；`Redis` 与第三方 `S3` 仅作为可选适配层。
9. 协作流采用 `GitHub Flow + Conventional Commits`。

### 2.2 规范优先级

优先级从高到低如下：

1. 法律与安全要求。
2. `docs/PRD.md`。
3. `docs/SAD.md`。
4. 本开发规范。
5. ADR（经批准的例外或补充）。
6. 模块内局部 README / package 文档。
7. 个人偏好。

若存在冲突，以更高优先级文档为准。

### 2.3 例外机制

以下变更不得直接通过代码提交完成，必须先提交 ADR 或设计文档：

1. 主技术栈变更。
2. Monorepo 目录边界变更。
3. 运行时架构模式变更。
4. OpenAPI / Protobuf 契约源变更。
5. tokens 体系结构变更。
6. 自建组件 API 破坏性变更。
7. 数据库兼容策略变更。
8. 插件宿主或扩展契约变更。

ADR 目录固定为：`docs/adr/`。

### 2.4 VibeCoding 执行模型

`octopus` 采用 AI 深度参与的开发方式，但执行模型固定为“人主边界，AI 主细节”。

规范要求：

1. 需求边界、验收标准、架构例外、安全例外必须由人工确认。
2. AI 只能在已批准范围内推进实现，不得自扩需求或自改架构边界。
3. 默认按单条 MVP 纵切片推进，禁止并发铺开多个高耦合能力域。
4. 所有高风险能力必须在进入默认可用面前纳入审批、沙箱、审计和恢复校验。
5. 具体执行流程见 `docs/VIBECODING.md`。

---

## 3. 完整技术栈选型

### 3.1 基础工程与仓库工具链

| 类别 | 选型 | 说明 |
| --- | --- | --- |
| 版本控制 | Git + GitHub | 项目主协作平台 |
| Node 运行时 | Node.js LTS | 统一前端与工具链运行时 |
| Node 包管理 | `pnpm` | 使用严格依赖解析与 workspace 管理 |
| Monorepo 编排 | `Turborepo` | 管理前端 apps/packages 的构建、测试与缓存 |
| Rust 工具链 | Rust stable + Cargo workspace | 管理 Rust Core、多 crate 结构与构建 |
| 协议管理 | `Buf` + Protobuf | 管理内部 gRPC 契约 |
| HTTP 契约 | OpenAPI | 管理外部 HTTP API 契约 |
| OpenAPI 校验 | `Spectral` | 校验 OpenAPI 风格与一致性 |
| 提交钩子 | `lefthook` | 统一 Node/Rust 项目的本地钩子执行 |
| CI 平台 | GitHub Actions | 统一检查、构建、测试、发布流水线 |

### 3.2 前端与客户端技术栈

| 类别 | 选型 | 说明 |
| --- | --- | --- |
| 前端语言 | TypeScript | 严格模式，禁止降级为宽松 JS |
| 主框架 | Vue 3 | 统一采用 Composition API |
| SFC 模式 | `<script setup lang="ts">` | 默认写法 |
| 构建工具 | Vite | 统一 Web/H5/Tauri 前端构建 |
| 路由 | Vue Router 4 | 管理控制面应用路由 |
| 全局状态 | Pinia | 仅处理客户端共享状态与 UI 状态 |
| 服务端状态 | `@tanstack/vue-query` | 管理远程数据获取、缓存、失效与重试 |
| 通用组合式能力 | VueUse | 复用常见浏览器与交互 composables |
| 样式引擎 | UnoCSS | 原子化样式与主题变量映射 |
| 设计 tokens | 自建 `packages/design-tokens` | 唯一视觉事实来源 |
| 主题系统 | `system` / `light` / `dark` | 必须支持跟随系统、浅色、深色 |
| 国际化 | Vue I18n | 至少支持 `zh-CN`、`en-US` |
| 组件基础层 | Reka UI primitives | 仅作为无样式底层 primitives 使用 |
| 自建组件库 | `packages/ui` | 对外唯一 UI 组件入口 |
| 图标方案 | `UnoCSS preset-icons + Iconify` | 默认功能图标族用 `Lucide` |
| 品牌图标 | `simple-icons` | 仅品牌/logo 场景允许 |
| 组件文档 | Storybook | 自建组件展示、状态验证、视觉评审 |
| 单元/组件测试 | Vitest + Vue Test Utils | Vue 组件与 composables 测试 |
| E2E 测试 | Playwright | Web/桌面控制面端到端验证 |
| 类型检查 | `vue-tsc` | 前端类型门禁 |
| 代码检查 | ESLint + Prettier | 代码质量与格式基线 |

### 3.3 客户端分发与运行形态

| 表面 | 选型 | 说明 |
| --- | --- | --- |
| Web | Vue SPA | 主控制面 |
| H5 / PWA | Vue SPA + PWA 配置 | 轻控制面、通知跳转、审批与状态查看 |
| Desktop | Tauri 2 + Vue 前端 | 强控制面 + 本机节点 |
| Mobile | Tauri Mobile + Vue 前端 | 控制优先，不承担重型执行 |

### 3.4 后端与运行时技术栈

| 类别 | 选型 | 说明 |
| --- | --- | --- |
| 核心语言 | Rust | 控制平面、运行时、节点协议、治理内核 |
| 异步运行时 | Tokio | Rust 异步基础设施 |
| HTTP 服务 | Axum | 外部控制面 HTTP API |
| gRPC | Tonic | 控制面与节点之间的内部协议 |
| 数据访问 | SQLx | 支持 `SQLite` / `PostgreSQL` 双兼容 |
| 序列化 | Serde | JSON / 配置 / 事件序列化 |
| 错误建模 | `thiserror` + `anyhow` | 领域错误显式建模，边界层统一封装 |
| 日志与追踪 | `tracing` + OpenTelemetry | 结构化日志、trace、metrics |
| 中间件 | `tower` | 限流、超时、重试、追踪等通用中间件 |
| 配置加载 | 环境变量 + `.env`（本地） | 配置来源统一，不允许写死凭据 |
| 格式化 | `rustfmt` | Rust 代码格式基线 |
| 静态检查 | `clippy` | 默认 `-D warnings` |
| 测试 | `cargo test` / `cargo nextest` | Rust 单元与集成测试基线 |

### 3.5 数据与基础设施技术栈

| 类别 | 选型 | 说明 |
| --- | --- | --- |
| 默认本地数据库 | SQLite | 个人、本地、轻量部署默认选型 |
| 生产推荐数据库 | PostgreSQL | 团队、生产、共享部署推荐选型 |
| 数据迁移 | `sqlx migrate` | 统一迁移工具 |
| 默认对象存储 | 文件系统 | 本地默认承载 artifact 与 blueprint 包 |
| 可选缓存层 | Redis adapter | 仅作为可选适配层，不是默认强依赖 |
| 可选对象存储 | S3-compatible adapter | 仅作为可选适配层，不是默认强依赖 |
| 本地部署 | Docker / Docker Compose | 自托管优先交付基线 |
| 指标观测 | Prometheus + OpenTelemetry | 指标与 tracing 集成 |
| 可视化观测 | Grafana | 观测面板 |
| 日志聚合 | Loki 或等价方案 | 结构化日志集中查看 |

### 3.6 契约与代码生成栈

| 类别 | 选型 | 说明 |
| --- | --- | --- |
| HTTP API 契约源 | OpenAPI | 外部 API 唯一契约源 |
| 前端 HTTP 类型 | `openapi-typescript` | 从 OpenAPI 生成 TS 类型 |
| 前端 API 调用层 | 基于生成类型的自建 client | 不允许手写游离 DTO |
| 内部 RPC 契约源 | Protobuf + Buf | gRPC 唯一契约源 |
| 插件契约 | `PluginManifest` Schema | 插件元数据与能力声明契约 |
| 设计 tokens 输出 | CSS Variables + TS + JSON | 统一被 UnoCSS、组件库、客户端消费 |

---

## 4. Monorepo 与目录结构规范

### 4.1 仓库组织

仓库采用双工作区模型：

1. Node 侧统一使用 `pnpm workspace + Turborepo`。
2. Rust 侧统一使用 Cargo workspace。
3. 任何人不得引入第二套主构建系统。
4. 根目录仅允许保留仓库级配置，不允许堆放业务代码。

### 4.2 推荐目录骨架

```text
octopus/
├─ apps/
│  ├─ web/                    # Web 主控制面
│  ├─ desktop/                # Tauri Desktop 壳与集成
│  └─ mobile/                 # Tauri Mobile 壳与集成
├─ packages/
│  ├─ design-tokens/          # 设计 tokens 源与输出
│  ├─ ui/                     # 自建组件库
│  ├─ icons/                  # AppIcon 与图标封装
│  ├─ i18n/                   # 国际化资源与工具
│  ├─ composables/            # 可跨 app 复用的通用 composables
│  ├─ api-client/             # OpenAPI 生成类型与前端 client
│  ├─ eslint-config/          # 前端 lint 基线
│  ├─ tsconfig/               # TS 配置基线
│  └─ shared/                 # 前端共享工具，不承载业务域
├─ crates/
│  ├─ octopus-domain/         # 领域模型与核心规则
│  ├─ octopus-application/    # 用例编排与应用服务
│  ├─ octopus-runtime/        # Run/Trigger/AskUser/Approval 状态机与运行时
│  ├─ octopus-api-http/       # Axum HTTP API
│  ├─ octopus-api-grpc/       # Tonic gRPC 服务
│  ├─ octopus-infra-sqlite/   # SQLite 适配
│  ├─ octopus-infra-postgres/ # PostgreSQL 适配
│  ├─ octopus-node-runtime/   # 节点执行面
│  ├─ octopus-plugin-host/    # 插件宿主
│  └─ octopus-shared/         # Rust 共享类型与公共能力
├─ proto/
│  ├─ openapi/                # HTTP API 契约
│  ├─ grpc/                   # Protobuf 契约
│  └─ schemas/                # PluginManifest 等 schema
├─ deploy/
│  ├─ docker/                 # Docker 相关配置
│  ├─ compose/                # Docker Compose 示例
│  └─ env/                    # 部署环境模板
├─ scripts/                   # 仓库级自动化脚本
├─ docs/
│  ├─ adr/                    # 架构决策记录
│  └─ ...
├─ turbo.json
├─ pnpm-workspace.yaml
├─ Cargo.toml
└─ README.md
```

### 4.3 目录边界规则

1. `apps/` 只承载可运行应用，不承载共享设计系统或通用 SDK。
2. `packages/` 只承载前端共享能力，不承载后端逻辑。
3. `crates/` 只承载 Rust 代码，不允许混入前端资源或业务文档。
4. `proto/` 是契约源目录，不允许存放运行时代码。
5. `deploy/` 只存部署相关资源，不存业务代码。
6. `scripts/` 只存仓库级自动化脚本，不得替代正式包或 crate。
7. 根目录禁止新增游离脚本、临时 SQL、临时 JSON、手工导出的图片或未归类文档。

### 4.4 Turborepo 规范

1. 所有前端任务必须定义在各自 package/app 的 `package.json` 中。
2. 根 `package.json` 只允许通过 `turbo run <task>` 委托执行，不允许写实际任务逻辑。
3. 在代码、脚本、CI 中必须使用 `turbo run`，禁止写入 `turbo build` 这类 shorthand。
4. 缓存任务必须显式声明 `outputs`。
5. 默认启用基于依赖图的任务编排，禁止根脚本手工串行 `cd` 执行各子包命令。

### 4.5 pnpm 规范

1. 所有 Node 依赖统一由 `pnpm` 管理。
2. 依赖版本必须在 workspace 级集中管理，优先使用 catalog / workspace protocol。
3. CI 中必须使用 `--frozen-lockfile`。
4. 禁止引入 phantom dependency；每个包只声明自己真正使用的依赖。
5. 第三方补丁与覆盖必须记录在集中配置中，禁止私自 patch 本地 `node_modules`。

---

## 5. 架构与设计模式规范

### 5.1 后端架构模式

后端统一采用以下模式：

1. 模块化单体。
2. Ports and Adapters。
3. 事件驱动运行时。
4. 显式用例层与领域层分离。
5. 读写分离思维，但不强制拆库拆服务。

#### 5.1.1 层次划分

Rust 代码必须按以下职责分层：

1. `domain`
   - 领域对象、值对象、状态机规则、策略判定输入输出。
2. `application`
   - 用例编排、命令处理、查询处理、聚合工作流。
3. `infrastructure`
   - SQLx 仓储、文件系统、Redis/S3 adapter、外部集成。
4. `transport`
   - HTTP/gRPC handler、请求映射、认证接入、协议边界。

禁止：

1. 在 handler 中直接写业务规则。
2. 在 domain 中直接依赖 SQLx、Axum、Tonic、Redis、S3 SDK。
3. 在 infrastructure 中反向依赖 transport 层。

#### 5.1.2 运行时模式

运行时相关代码必须满足：

1. `Run / Trigger / AskUser / Approval` 统一事件驱动。
2. 状态变更通过显式事件与投影实现。
3. Side effect 通过 adapter 执行，不能在核心状态机里直接硬编码。
4. 所有恢复入口必须显式处理幂等与 freshness check。

### 5.2 前端架构模式

前端统一采用以下模式：

1. Vue SPA 控制台架构。
2. Composition API + composables。
3. 路由驱动页面组织。
4. server state 与 UI state 分离。
5. 设计系统驱动 UI，而不是页面私写样式。

#### 5.2.1 状态分层

前端状态必须按以下方式处理：

1. 远程数据、缓存、失效、重试：使用 `@tanstack/vue-query`。
2. 跨页面共享的客户端状态、偏好、面板开关、局部工作流状态：使用 Pinia。
3. 组件内局部状态：优先使用 `ref` / `computed` / `watch`。
4. 浏览器能力、媒体查询、剪贴板、键盘等通用交互：优先使用 VueUse composables。

禁止：

1. 用 Pinia 存放所有接口数据。
2. 用全局 store 替代路由参数或 URL 状态。
3. 在组件树中层层 props 传递可由 composable 或 store 管理的公共状态。

#### 5.2.2 组件分层

组件库必须分为四层：

1. `tokens`
   - 视觉变量，不直接承载业务逻辑。
2. `primitives`
   - 基于 Reka UI 等无样式 primitives 的底层行为封装。
3. `base components`
   - 按统一 API、tokens、主题输出的通用组件。
4. `patterns`
   - 由多个 base components 组成的复合交互模式。

业务页面只能消费 `packages/ui` 暴露的组件与 patterns，禁止直接依赖第三方 UI 组件库。

### 5.3 数据访问与适配器模式

1. 数据库访问必须经 repository / adapter 层完成。
2. 文件系统、Redis、S3、MCP、插件、通知等外部依赖必须经显式 adapter / interface 接入。
3. 核心业务域不得直接引用第三方 SDK。
4. 可选适配层必须具备本地默认实现与可替换实现。

### 5.4 契约优先模式

1. 外部 HTTP API 以 OpenAPI 为源。
2. 内部 gRPC 以 Protobuf + Buf 为源。
3. 插件以 `PluginManifest` Schema 为源。
4. 任何跨模块共享数据结构，应优先从契约生成或由公共包统一定义。
5. 前端禁止手写游离类型绕过 OpenAPI / Protobuf 契约。

---

## 6. 设计系统、UI/UX、主题与国际化规范

### 6.1 总体设计语言

`octopus` 的前端视觉与交互风格固定为：

1. 现代简约。
2. 大气克制。
3. 交互流畅。
4. 视觉一致。
5. 吸收苹果和谷歌设计的优雅感，但不直接复制任何品牌风格。

具体执行规则如下：

1. 以高可读性和信息层级清晰为第一优先级。
2. 以中性色体系作为主基底，只允许有限、克制、语义清晰的品牌强调色。
3. 通过间距、层级、留白、分组和轻量动效建立高级感，禁止依赖过度阴影、过度渐变和过重描边。
4. 在桌面、移动、浅色、深色、中英切换时，视觉风格必须保持同一套设计语法。

### 6.2 设计 tokens 规范

`packages/design-tokens` 是唯一视觉事实来源，必须统一输出：

1. CSS Variables
2. TypeScript 常量/类型
3. JSON token 数据

tokens 至少覆盖以下维度：

1. 颜色
2. 字体家族
3. 字号
4. 字重
5. 行高
6. 间距
7. 圆角
8. 阴影
9. 边框
10. z-index / layer
11. 动效时长
12. 动效曲线
13. 图标尺寸
14. 状态语义（success/warning/error/info/focus/disabled）

禁止：

1. 在业务组件中直接写硬编码颜色值。
2. 在页面中直接写魔法数字间距、字号、圆角、阴影。
3. 在单个业务页面私自扩展主题变量命名体系。

### 6.3 主题系统规范

主题系统必须支持：

1. `system`
2. `light`
3. `dark`

规范要求：

1. 主题切换必须基于统一 theme service，不允许各 app 自写独立逻辑。
2. `system` 模式必须响应系统主题变化。
3. 所有组件必须在浅色与深色主题下可用、可读、对比度合格。
4. UnoCSS 中的颜色、阴影、边框和表面层级必须映射到 tokens，而不是写死主题值。
5. 用户主题偏好如需持久化，必须经统一偏好存储层处理。

### 6.4 国际化规范

国际化是强制要求，至少支持：

1. `zh-CN`
2. `en-US`

规范要求：

1. 所有用户可见文案必须走 i18n。
2. i18n key 必须使用英文语义命名，例如 `nav.workspace.title`。
3. 禁止直接在组件中内联中英文硬编码文案。
4. 禁止拼接翻译片段形成句子。
5. 任何新增 UI 文案的 PR，必须同时补齐中英文资源。
6. 路由标题、按钮、表单标签、提示语、空态、错误消息和通知文案全部在 i18n 范围内。

### 6.5 图标规范

图标方案固定为：

1. 渲染层使用 `UnoCSS preset-icons + Iconify`
2. 默认功能图标族使用 `Lucide`
3. 品牌图标仅允许使用 `simple-icons`
4. 产品特有图标通过 `packages/icons` 暴露的 `AppIcon` 统一封装

图标使用规则：

1. 业务代码不得直接引入杂乱第三方图标组件。
2. 同一页面默认只允许一种功能图标风格。
3. 功能图标默认使用线性风格，禁止无理由混用填充风格。
4. 图标尺寸必须走 tokens。
5. 深色模式下图标颜色必须随主题语义切换。

### 6.6 组件规范

组件体系固定为“自建外观与 API，允许底层 primitives 封装”。

必须满足：

1. `packages/ui` 是唯一对外 UI 组件入口。
2. 所有组件 API、样式、状态、交互和主题行为必须由内部组件库统一定义。
3. 允许基于 Reka UI 等 primitives 层封装无样式行为。
4. 业务代码禁止直接使用第三方现成 UI 组件库。
5. 组件必须支持主题、国际化、可访问性和统一尺寸体系。

### 6.7 交互与动效规范

1. 动效必须服务于层级切换、反馈和空间理解，不得为了“炫”而增加动作。
2. 默认动效时长应落在短时响应区间内，常规交互使用轻量 ease-out 曲线。
3. Hover、focus、pressed、disabled、loading 状态必须统一定义。
4. Modal、Drawer、Popover、Dropdown、Tooltip 的交互时序必须在整个产品中保持一致。
5. 键盘导航、焦点可见性和 Escape/Enter 等基础交互必须统一。

### 6.8 视觉框架文档要求

首版 UI 编码前，必须先在 `docs/VISUAL_FRAMEWORK.md` 中锁定以下内容：

1. 表面职责划分。
2. 一级导航与信息架构。
3. 首批页面优先级。
4. 主题与国际化的呈现边界。
5. 视觉语法与组件使用边界。

若导航结构、页面优先级或表面职责发生变化，必须同步更新该文档。

---

## 7. 代码规范

### 7.1 通用代码规范

1. 代码标识、接口名、Schema 字段、配置键、环境变量、提交类型和注释默认使用英文。
2. 架构文档、规范文档、ADR 默认使用中文。
3. 注释只解释 `why`、约束、边界和非显然设计，不解释显而易见的 `what`。
4. 所有代码必须优先清晰可读，禁止过度技巧化实现。
5. 公共逻辑必须进入公共包或公共 crate，禁止跨应用复制粘贴。
6. 生成代码必须集中在约定目录，并带有“禁止手改”标记。

### 7.2 TypeScript 与 Vue 规范

#### 7.2.1 语言与类型

1. 全部前端代码必须使用 TypeScript。
2. 必须开启 strict 模式。
3. 禁止常规业务代码使用裸 `any`。
4. 优先使用明确的接口、类型别名与字面量联合类型。
5. 复杂对象输入输出必须通过 schema 或生成类型约束。

#### 7.2.2 Vue 组件规范

1. 强制使用 Composition API。
2. 强制使用 `<script setup lang="ts">`。
3. 默认使用 PascalCase 组件名。
4. Props / Emits 必须显式建模。
5. `computed` 禁止产生副作用。
6. 组件内业务逻辑应优先下沉到 composables 或 store。
7. 除极特殊场景外，禁止使用 Options API 作为默认实现模式。

#### 7.2.3 Pinia 规范

1. 全局状态统一使用 Pinia。
2. 默认使用 setup stores。
3. store 文件命名统一为 `useXxxStore.ts`。
4. store 中不得直接混入纯展示逻辑。
5. store 与 server state 不得混用。
6. 解构 store state/getters 时必须使用 `storeToRefs()`。

#### 7.2.4 路由规范

1. 生产级路由必须使用 Vue Router。
2. 路由模块按领域拆分，禁止把所有路由堆在单文件。
3. 路由守卫必须避免循环重定向。
4. 路由参数变化导致的数据刷新必须显式处理。
5. 页面权限、工作区上下文、实验开关等前置逻辑通过标准守卫或 composable 统一实现。

#### 7.2.5 样式规范

1. 优先使用 UnoCSS utility classes。
2. 自定义 CSS 仅允许用于 utilities 不适合表达的复杂场景。
3. 全局 CSS 仅允许承载 reset、theme variables、字体基线、基础排版和极少量全局修正。
4. 业务页面不得散写主题颜色或私有样式常量。

### 7.3 Rust 规范

#### 7.3.1 语言与模块

1. Rust 代码必须使用稳定版工具链。
2. 模块名使用 `snake_case`。
3. crate 名使用统一命名风格，避免语义模糊缩写。
4. domain/application/infrastructure/transport 边界必须清晰。
5. 公共类型进入共享 crate，禁止循环依赖。

#### 7.3.2 错误处理

1. 领域错误必须显式建模。
2. 边界层可以使用 `anyhow` 聚合上下文，但领域层不得使用无结构字符串错误替代正式错误类型。
3. 禁止吞掉错误。
4. 所有外部 I/O 错误必须补上下文。

#### 7.3.3 异步与并发

1. Tokio 是唯一异步运行时。
2. 异步边界必须可观察、可取消、可超时。
3. 不得在核心路径中留下无管理的后台任务。
4. 任何长任务必须具备 tracing 上下文和取消/超时控制。

### 7.4 接口与契约代码规范

1. OpenAPI 与 Protobuf 变更必须先改契约源，再改实现。
2. 生成类型和代码禁止手工编辑。
3. HTTP handler 与 gRPC service 必须只承担协议映射和调用应用服务的职责。
4. 不允许把内部领域对象直接裸暴露为对外协议结构。

### 7.5 数据库规范

1. 核心代码必须兼容 `SQLite` 与 `PostgreSQL`。
2. SQL 必须通过 repository / adapter 管理，禁止在 handler 中散落 SQL。
3. 迁移必须可在两种数据库上定义清晰路径。
4. 默认避免依赖 PostgreSQL 独占能力作为核心路径前提。
5. 若确需数据库方言差异，必须在 adapter 层隔离。
6. 生产环境推荐 PostgreSQL；本地与轻量部署默认 SQLite。

### 7.6 文件命名规范

1. Vue 组件：`PascalCase.vue`
2. composables：`useXxx.ts`
3. stores：`useXxxStore.ts`
4. TS 普通模块：`kebab-case.ts` 或按团队统一约定保持一致
5. Rust 模块：`snake_case.rs`
6. 文档：`UPPER_SNAKE` 或 `kebab-case` 保持仓库一致

### 7.7 模块与文件演化规则

#### 7.7.1 通用原则

1. 允许模块从单文件起步，尤其是脚手架、占位、纯类型定义、低复杂度封装或尚未展开的 MVP 边界。
2. 单文件实现只是起点，不是长期默认形态；当同一文件持续增长并开始承载多个职责时，必须拆分。
3. 拆分优先按职责边界、领域边界和调用方向进行，禁止为了满足“文件更小”而制造大量无语义的一行转发模块。
4. 优先在现有 crate/package 内拆分模块、目录、composable 或组件，而不是为局部问题优先新增 crate/package。
5. 纯类型定义、纯 schema、生成代码、纯测试夹具或测试数据文件可以按其特殊性质保留较大体量，但仍必须保持可读性和可定位性。

#### 7.7.2 评审触发器

1. Rust / TypeScript 普通模块超过约 `300` 行时，作者与评审必须主动评估是否拆分。
2. Vue SFC 超过约 `250` 行时，作者与评审必须主动评估是否拆分。
3. 超过约 `450` 行的 Rust / TypeScript 文件，如果不是纯类型定义、纯 schema、纯 generated code 或纯测试数据，默认应拆分，或在 PR 中明确说明当前不拆分的合理性。
4. 上述阈值是评审触发器，不是自动失败条件，也不得替代职责判断。

#### 7.7.3 强制拆分条件

1. 同一文件同时包含两个及以上可独立命名的职责簇时，必须拆分。
2. 同时出现“协议映射 + 业务编排 + 持久化或外部 I/O”时，必须拆分。
3. 同时出现“创建 / 查询 / 更新 / 恢复 / 审计”等多个流程块且彼此可独立命名时，必须拆分。
4. 评审者难以在一次阅读中建立清晰心智模型时，应视为已触发拆分。

#### 7.7.4 Rust 细则

1. `lib.rs` 优先承担模块声明、re-export 和少量装配；除极小 crate 外，不应长期承载完整实现细节。
2. `domain`、`application`、`infrastructure`、`transport` 边界不得因单文件实现而被混淆。
3. `infrastructure` 中的 migration、queries、row mapping、repository 实现应优先拆为独立模块。
4. `runtime` 中的 service、validation、builders、state transition helpers 应优先拆为独立模块。
5. 当 crate 仍处于占位或极小实现阶段时，单个 `lib.rs` 可以保留；后续演进必须按职责自然拆分。

#### 7.7.5 TypeScript / Vue 细则

1. route、store、composable、service 可以单文件起步，但跨领域、跨页面或跨状态职责时必须拆分。
2. 路由继续按领域拆分，禁止把所有路由堆在单文件。
3. 页面 SFC 不得长期同时承载页面编排、复杂数据获取、表单规则、弹层流程和大量展示细节。
4. 需要复用、独立验证或降低耦合的逻辑，应优先拆入 composables、route modules、child components 或 services。
5. 拆分后的目录与文件命名必须保持语义清晰，禁止用 `misc`、`utils2`、`temp` 之类模糊命名稀释边界。

---

## 8. 数据、存储与适配层规范

### 8.1 数据库支持等级

数据库支持等级固定为：

1. `SQLite`：默认本地/个人部署数据库。
2. `PostgreSQL`：团队/生产推荐数据库。

这意味着：

1. 代码必须支持双兼容。
2. CI 至少要验证数据库兼容策略不被破坏。
3. 不允许默认把只适用于 PostgreSQL 的特性当成核心前提。

### 8.2 文件系统对象存储基线

1. artifact 与 blueprint 包默认使用文件系统存储。
2. 文件系统路径必须可配置。
3. 本地文件系统实现属于正式支持路径，而不是“临时开发模式”。

### 8.3 Redis 可选适配层

Redis 仅作为可选适配层进入规范，可用于：

1. 缓存
2. 速率限制
3. 短期协调状态
4. 轻量异步通知桥接

规范要求：

1. 默认部署不能强依赖 Redis。
2. 使用 Redis 的模块必须先定义抽象接口。
3. 无 Redis 时必须存在可接受的默认实现。

### 8.4 S3 可选适配层

第三方 `S3` / `S3-compatible` 对象存储仅作为可选适配层进入规范，可用于：

1. 远程 artifact 存储
2. blueprint 包存储
3. 静态资源或导出包分发

规范要求：

1. 默认部署不能强依赖 S3。
2. S3 访问必须通过 `ObjectStore` 之类的统一接口。
3. 不允许在业务域直接散落 S3 SDK 调用。

---

## 9. 测试、质量门禁与 CI 规范

### 9.1 测试分层

测试至少分为以下层次：

1. 单元测试
2. 组件测试
3. 集成测试
4. 契约测试
5. E2E 测试
6. 视觉/主题回归测试（组件库与关键页面）

### 9.2 前端测试规范

1. 组件与 composables 使用 Vitest + Vue Test Utils。
2. 跨页面流程、主题切换、国际化切换、审批流与关键交互使用 Playwright。
3. 组件库必须有 Storybook 场景。
4. 新增组件必须至少覆盖默认态、禁用态、错误态、加载态、浅色、深色和中英文文案显示。

### 9.3 后端测试规范

1. Rust 单元与集成测试使用 `cargo test` 或 `cargo nextest`。
2. 关键状态机、恢复逻辑、审批逻辑、策略判定和数据库 adapter 必须有自动化测试。
3. 涉及 `SQLite` / `PostgreSQL` 差异的模块必须有兼容性测试。

### 9.4 契约与质量检查

以下检查为强制门禁：

#### Node / Frontend

1. `pnpm install --frozen-lockfile`
2. `turbo run lint`
3. `turbo run typecheck`
4. `turbo run test`
5. `turbo run build`

#### Rust / Backend

1. `cargo fmt --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo test` 或 `cargo nextest run`

#### 契约 / 文档 / 安全

1. `buf lint`
2. OpenAPI lint
3. `cargo deny` 或等价依赖安全检查
4. 前后端生成代码同步检查
5. 文档同步检查（涉及架构、契约、组件 API、tokens、数据库策略变更时）

### 9.5 PR 必备证据

任何 PR 至少必须提供：

1. 变更摘要
2. 风险说明
3. 验证方式
4. 相关截图或录屏（UI 变更必需）
5. 文档同步说明

UI 变更必须附带：

1. 浅色模式截图
2. 深色模式截图
3. 中文截图
4. 英文截图

### 9.6 故障归因与报错处置

出现报错、失败或偏航时，必须采用统一三步法：

1. 先分类：按 `planning`、`context`、`tool schema`、`policy`、`resume/idempotency`、`human coordination` 等 failure taxonomy 归类。
2. 再定界：明确问题位于需求、契约、架构、实现、环境或验证哪个边界。
3. 后修复：只做最小必要改动，并立即执行回归验证与文档同步。

禁止在未完成分类和定界前并行堆叠多个不相关修复。

---

## 10. Git、提交、PR 与 ADR 规范

### 10.1 分支策略

协作流固定为 `GitHub Flow`：

1. `main` / `master` 为受保护主干。
2. 所有功能、修复、重构都通过短生命周期分支完成。
3. 不允许直接提交到主干。

推荐分支命名：

1. `feature/<scope>-<slug>`
2. `fix/<scope>-<slug>`
3. `refactor/<scope>-<slug>`
4. `docs/<scope>-<slug>`
5. `chore/<scope>-<slug>`

### 10.2 提交规范

提交信息必须使用 Conventional Commits：

```text
type(scope): summary
```

允许的 `type`：

1. `feat`
2. `fix`
3. `refactor`
4. `docs`
5. `test`
6. `build`
7. `ci`
8. `chore`
9. `perf`
10. `revert`

示例：

1. `feat(runtime): add run state projector abstraction`
2. `fix(ui): align dark theme token mapping`
3. `docs(standards): define sqlite and postgres compatibility rules`

规范要求：

1. `scope` 必须是模块、包、crate、领域或文档域。
2. 破坏性变更必须显式标记。
3. 禁止使用无信息量提交，如 `update`、`fix bug`、`misc`。

### 10.3 Pull Request 规范

PR 必须满足：

1. 小而清晰，单一目的。
2. 通过全部自动化门禁。
3. 包含验证结果。
4. 涉及 UI 的 PR 带截图或录屏。
5. 涉及架构或契约的 PR 带文档更新。
6. 若新增或保留跨职责的大文件，必须说明当前不拆分仍合理的理由。

以下变更必须有额外说明：

1. tokens 结构变更
2. 组件 API 变更
3. OpenAPI / Protobuf 变更
4. 数据库迁移变更
5. 安全策略变更

### 10.4 ADR 规则

以下情况必须新增或更新 ADR：

1. 主技术栈调整
2. 包边界或 crate 边界调整
3. 运行时模式调整
4. 数据存储策略调整
5. 组件体系或 tokens 体系重大调整
6. 契约、协议或插件机制重大调整

### 10.5 Git 质量管控落地

仓库必须通过以下设施把 Git 规则变成执行机制：

1. `.github/pull_request_template.md`
2. `lefthook.yml`
3. GitHub Actions 最小质量工作流
4. 分支保护与主干禁止直推策略

在仓库初始化阶段，如果完整 lint/test/build 链路尚未具备，至少必须保证：

1. 根配置可解析。
2. 必需文档存在。
3. Cargo workspace 可解析。
4. PR 模板和提交规范已落地。

---

## 11. 安全、配置与依赖规范

### 11.1 配置规范

1. 所有配置必须来自环境变量、配置文件或部署注入。
2. 本地开发允许 `.env`，但必须提供 `.env.example`。
3. 禁止在代码中写死秘密、令牌、数据库密码、API Key。
4. 配置项命名统一使用英文大写下划线或契约定义格式。

### 11.2 依赖准入规范

新增依赖必须满足：

1. 有明确用途。
2. 有持续维护记录。
3. 与当前技术栈兼容。
4. 不与现有依赖职责重叠。
5. 有许可证与安全风险评估。

禁止：

1. 为一个小功能引入重型依赖。
2. 引入与现有主栈重复的框架。
3. 在业务层直接引入底层基础设施 SDK。

### 11.3 安全扫描

1. Rust 依赖使用 `cargo deny` 或等价工具检查。
2. Node 依赖使用 `pnpm audit` 或等价安全扫描。
3. 关键依赖升级必须说明兼容性影响。
4. 插件与外部扩展相关依赖必须重点审查供应链风险。

---

## 12. 发布、版本与文档同步规范

### 12.1 版本策略

1. 项目对外版本遵循 SemVer。
2. 在项目早期，内部包和 crate 允许以仓库主版本对齐为主。
3. 破坏性变更必须显式说明迁移影响。

### 12.2 发布前检查

发布前必须完成：

1. 全量 CI 通过
2. 契约与生成代码同步
3. 文档同步
4. 安全与依赖检查
5. 迁移说明与兼容性说明

### 12.3 文档同步要求

以下变更必须同步更新文档：

1. 技术栈变更
2. 目录结构变更
3. 组件 API 变更
4. tokens 体系变更
5. 数据库兼容策略变更
6. OpenAPI / Protobuf / PluginManifest 变更
7. 运行时或架构边界变更

默认同步目标包括：

1. 本开发规范
2. 相关 ADR
3. 模块 README
4. 对外或对内接口说明
5. `docs/VIBECODING.md`
6. `docs/VISUAL_FRAMEWORK.md`

---

## 13. 明确禁止项

以下行为在项目中明确禁止：

1. 在业务页面直接写硬编码主题值。
2. 在业务代码中直接依赖第三方 UI 组件库。
3. 继续引入 React 相关主栈作为控制面实现基线。
4. 在 handler 中直接写业务规则或散落 SQL。
5. 在核心路径中直接调用 Redis、S3、文件系统、MCP 或插件 SDK 而不经 adapter。
6. 在前端内联中英文文案而不走 i18n。
7. 在深色模式下只做局部 patch 而不经过 tokens 系统。
8. 用 Pinia 存放所有远程接口数据。
9. 在根 `package.json` 写实际任务逻辑替代 `turbo run`。
10. 引入第二套主前端框架、主构建系统或主状态管理体系。
11. 在未更新文档和契约的情况下合并架构级变更。
12. 将仅适用于 PostgreSQL 的能力默认为所有环境可用。
13. 让 AI 在未确认范围和验收标准的情况下连续扩写核心能力。
14. 用“后面再补安全/审批/审计”作为跳过硬约束的理由。

---

## 14. 默认执行清单

当新增一个 app、package、crate 或核心功能时，开发者必须确认：

1. 是否符合既定目录边界。
2. 是否使用了已选定主技术栈。
3. 是否复用了统一 tokens、主题、i18n 与自建组件。
4. 是否遵守了 OpenAPI / Protobuf / PluginManifest 契约源规则。
5. 是否遵守了 `SQLite` / `PostgreSQL` 双兼容约束。
6. 是否将 Redis/S3 依赖隔离在 adapter 层。
7. 是否补齐了测试、截图、文档和 ADR。
8. 是否已锁定本轮 MVP 纵切片的验收边界。
9. 是否限制了 AI 本轮可修改的目录、契约和任务范围。

---

## 15. 结论

`octopus` 的开发规范不是“建议清单”，而是工程准入门槛。当前项目必须围绕以下稳定基线推进：

1. Rust Core + Vue 3 控制面。
2. Vue SPA + Tauri 2 多端复用。
3. 自建组件、自建 tokens、自建主题体系。
4. `SQLite` 默认、`PostgreSQL` 生产推荐。
5. 文件系统默认对象存储，Redis/S3 可选适配。
6. Monorepo、契约优先、测试门禁与文档同步并行。

后续所有代码、脚手架、CI 与目录演化，都必须把这份规范视为正式输入，而不是事后补充说明。
