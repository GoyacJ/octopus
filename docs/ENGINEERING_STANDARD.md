# octopus · 开发技术规范（ENGINEERING_STANDARD.md）

**版本**: v0.2.0 | **状态**: 正式版 | **日期**: 2026-03-25
**依赖文档**: PRD v2.0 · SAD v2.0 · CONTRACTS v1.0 · VIBECODING 2026-03-25

---

## 目录

1. [文档目标与治理](#1-文档目标与治理)
2. [通用工程原则](#2-通用工程原则)
3. [后端开发规范（Rust / Hub / Server / Tauri）](#3-后端开发规范rust--hub--server--tauri)
4. [前端开发规范（Vue 3 / TypeScript / Pinia）](#4-前端开发规范vue-3--typescript--pinia)
5. [国际化规范（i18n）](#5-国际化规范i18n)
6. [主题规范（深色 / 浅色 / System）](#6-主题规范深色--浅色--system)
7. [统一 UI 规范](#7-统一-ui-规范)
8. [代码风格与开发流程规范](#8-代码风格与开发流程规范)
9. [附录示例与反例](#9-附录示例与反例)

---

## 1. 文档目标与治理

### 1.1 文档目的

本规范用于回答一个问题：**在 octopus 中，工程实现必须怎么做**。

本文件是项目级开发约束，目标如下：

- 统一全栈实现方式，降低后续代码风格漂移。
- 将现有架构决策落到日常编码、评审、测试和 UI 落地层面。
- 为新成员提供可直接执行的开发准则，而不是口头约定。
- 为 Code Review 提供统一裁判标准，减少“个人偏好型争论”。

### 1.2 适用范围

本规范适用于 octopus 仓库内所有代码与文档变更，覆盖：

- Hub 核心、Server 适配层、Tauri Shell。
- Vue 3 Client、共享 UI 组件、Pinia Store、Transport 层。
- 数据迁移草案、契约、事件定义、测试代码、开发流程。

以下内容**不**由本规范重复定义：

- 产品需求、能力切片与非目标：见 [PRD.md](./PRD.md)
- 系统架构、运行平面、治理模型与恢复机制：见 [SAD.md](./SAD.md)
- 正式对象、枚举与事件契约：见 [CONTRACTS.md](./CONTRACTS.md)
- AI 主导实现的执行边界、人工签收和风险控制：见 [VIBECODING.md](./VIBECODING.md)
- 架构例外与正式决策：见 [adr/README.md](./adr/README.md)

### 1.3 关键词约定

| 关键词 | 含义 |
|------|------|
| `MUST` | 强制要求；违反即视为不合规，除非已获批准的例外 |
| `MUST NOT` | 严格禁止；不得以“方便”或“临时”作为理由绕过 |
| `SHOULD` | 推荐要求；若不遵守，必须有明确理由且在评审中说明 |
| `MAY` | 可选做法；仅在不破坏一致性的前提下允许采用 |

### 1.4 例外处理机制

任何偏离本规范的实现都必须满足以下条件：

1. 在 PR 描述中明确写出偏离点、原因、影响范围和回收计划。
2. 若偏离影响架构边界、公共接口、主题 token 或 i18n 结构，必须同步更新相关设计文档。
3. 临时性例外必须写明失效条件或计划移除时间。
4. 未记录的例外视为违规实现，不得合并。

---

## 2. 通用工程原则

### 2.1 以现有架构为唯一基线

所有实现 MUST 以现有架构文档为基准，不得在代码中另起一套抽象体系。

- Client 是交互层，Hub 是执行与治理事实源；前端不得私自承载业务规则。
- `Workspace / Project / Agent / Team / Run / Knowledge / CapabilityGrant / BudgetPolicy / Approval / Artifact` 是既定核心对象，不得随意跨 Context 泄漏职责。
- `Service / Repository / Domain Event / Transport` 的职责边界是架构约束，不是风格建议。
- 当前仓库仍以文档为治理事实源，但 tracked tree 已包含 `contracts/`、`apps/`、`packages/`、`crates/` 的 Phase 1 skeleton。只有在对应 manifest、源码和验证结果实际存在时，才能声明该能力已成立。

### 2.2 单一职责与边界清晰

每个模块、文件、类型、组件都 MUST 有单一主职责。

- 一个模块只解决一个层级的问题，不得同时承担“领域规则 + 存储细节 + 展示拼装”。
- 一个 Vue 组件若同时持有复杂副作用、跨页面状态和大段展示逻辑，应拆分。
- 一个 Rust Service 若同时处理 tenant 校验、SQL 拼接、DTO 序列化和事件广播，应分层。

### 2.3 显式抽象，禁止隐式穿透

抽象 MUST 对应真实边界，禁止为了“省文件”而绕层实现。

- 前端页面 MUST 经 Store / Composable / Transport 访问远程数据，不得在任意组件里直接散落通信细节。
- 后端业务入口 MUST 经 Service 组织，不得从 handler / command 直接越过 Service 操作 Repository。
- 事件驱动行为 MUST 通过统一事件定义显式建模，不得用“顺手调用另一个模块方法”代替事件边界。

### 2.4 可读性优先于技巧性

octopus 的默认风格是可维护、可推理、可替换。

- 代码 MUST 以 6 个月后的维护者仍能快速读懂为目标。
- 禁止为了减少几行代码引入难以理解的宏技巧、泛型体操、过度链式写法或嵌套回调。
- 复用 SHOULD 建立在语义稳定之上，禁止把偶然相似的逻辑抽成“万能工具”。

### 2.5 命名与注释原则

- 命名 MUST 反映业务语义，而不是实现手段。
- 注释 SHOULD 解释“为什么”，而不是复述“做了什么”。
- 临时注释、失效 TODO、调试说明 MUST 在合并前清理。
- TODO / FIXME MUST 带上下文，至少说明原因和后续动作，禁止只写“TODO”。

### 2.6 禁止的通用反模式

以下模式在全栈范围内均 `MUST NOT` 出现：

- 绕过既定层级直接访问底层对象。
- 在无明确边界的情况下创建“通用工具箱”或“万能 util”。
- 把临时业务判断写进基础设施层或共享组件。
- 先硬编码再说，后续再抽象。
- 没有验证和测试依据就声称“已完成”。
- 在缺少 manifest、源码或工具时虚构构建、测试、运行或联调结论。

---

## 3. 后端开发规范（Rust / Hub / Server / Tauri）

### 3.1 模块职责分层

后端代码 MUST 遵循既定仓库职责划分：

| 模块 | 责任 |
|------|------|
| `octopus-hub` | 领域模型、Service、Repository trait、运行时编排、事件定义 |
| `octopus-server` | HTTP API、认证中间件、SSE、外部协议适配 |
| `octopus-tauri` | 本地命令桥接、系统集成、事件转发、密钥链适配 |

强制要求：

- 领域规则 MUST 放在 `octopus-hub`，不得放入 `octopus-server` 或 `octopus-tauri`。
- `octopus-server` 和 `octopus-tauri` MUST 作为适配层存在，不得复制业务规则。
- 任何新能力若同时作用于本地和远程模式，优先放在 `octopus-hub`。

### 3.2 Service / Repository / Trait / Domain Event 分工

| 抽象 | MUST 承担的职责 | MUST NOT 承担的职责 |
|------|----------------|--------------------|
| `Service` | 业务入口、边界校验、不变量保护、调用编排 | 直接暴露底层存储细节 |
| `Repository trait` | 持久化读写抽象、屏蔽 SQLite/Postgres 差异 | 承载业务流程或权限判断 |
| `Infrastructure impl` | trait 落地、序列化/SQL/外部 client 交互 | 决定业务规则 |
| `Domain Event` | 表达跨 Context 的事实发生 | 代替同步返回值或隐藏副作用 |

额外要求：

- 外部访问 Agent 能力 MUST 经 `AgentService`，不得直接暴露 `MemoryStore`。
- 事件命名 MUST 表达已发生事实，例如 `TaskCompleted`、`DiscussionConcluded`。
- 事件消费者 MUST 可独立理解，不得依赖隐式调用顺序。

### 3.3 错误处理

- 领域层 MUST 返回可判定的业务错误，不得把底层数据库/网络错误原样泄漏给上层接口。
- 适配层 MAY 为日志添加上下文，但不得吞错。
- `unwrap` / `expect` 仅允许出现在测试、原型脚本或明确不可恢复的启动阶段；业务路径 `MUST NOT` 使用。
- 错误信息 MUST 可用于定位问题，但 MUST NOT 暴露密钥、Token、用户隐私或内部 SQL 细节。

### 3.4 异步与并发约束

- I/O 密集操作 MUST 使用 async 接口。
- CPU 密集或阻塞调用 MUST 显式隔离，禁止阻塞 Tokio 运行时。
- 后台任务 MUST 具备生命周期归属和取消语义，禁止随意 `spawn` 后放任不管。
- 并发执行 MUST 以数据一致性和可恢复性为前提，不能因为“更快”破坏任务状态机。

### 3.5 数据访问与迁移规范

- Schema、状态机和迁移语义 MUST 与 [PRD.md](./PRD.md) 和 [SAD.md](./SAD.md) 保持一致；在专门的数据模型文档重新建立前，不得凭空发明一套独立数据基线。
- 每次迁移 MUST 聚焦一个明确目标，不得把无关改动混在同一 migration。
- SQLite 与 PostgreSQL 双实现存在时，迁移 MUST 成对更新。
- 已合入主线的 migration `MUST NOT` 被静默改写；需要变更时新增后续 migration。
- Repository 接口 SHOULD 优先暴露按 tenant 过滤的方法，避免无边界查询。

### 3.6 配置、日志与安全

- 租户边界校验 MUST 在入口处执行，不能依赖调用方“应该传对”。
- 密钥、Token、凭据 `MUST NOT` 明文落盘或输出到日志。
- 结构化日志 SHOULD 记录对象 ID、tenant_id、事件类型、失败阶段，但不得记录敏感正文。
- 高风险工具审批边界 MUST 与架构文档保持一致，不得被临时配置绕开。

### 3.7 后端代码评审检查项

- 是否把业务规则放错层。
- 是否绕过 Service 直接访问 Repository 或底层 client。
- 是否破坏领域不变量、tenant 隔离或审批边界。
- 是否引入不可控后台任务、阻塞操作或无上下文错误。
- 是否同步更新迁移草案、事件、契约说明或相关设计文档。

---

## 4. 前端开发规范（Vue 3 / TypeScript / Pinia）

### 4.1 技术基线

前端实现 MUST 以以下技术约束为默认基线：

- Vue 3 + TypeScript
- Composition API
- `<script setup>`
- Pinia
- Vue Router
- VueUse
- UnoCSS
- Vue I18n
- self-built UI components
- shared design tokens

以下做法 `MUST NOT` 作为默认方案：

- 新代码使用 Options API。
- 大量 `any`、弱类型 DTO 或“先写通再补类型”。
- 页面直接调用网络接口并持久化跨页面状态。

### 4.2 分层职责

| 层级 | 责任 | 禁止事项 |
|------|------|---------|
| `views/` | 页面编排、路由级状态、权限入口 | 直接承载通用 UI 细节或数据通信实现 |
| `components/` | 展示与局部交互 | 跨页面状态管理、全局副作用 |
| `stores/` | 跨视图状态、业务行为编排、缓存 | 承担纯展示逻辑 |
| `composables/` | 可复用状态逻辑、订阅封装、派生行为 | 偷偷创建全局单例状态 |
| `transport/` | `invoke/HTTP/SSE` 差异封装 | 拼装页面展示数据 |

### 4.3 组件规范

- 组件 MUST 尽量保持“输入明确、输出明确”，优先使用 `props / emits / slots` 描述边界。
- 单个组件 SHOULD 避免同时处理布局、表单验证、异步请求、权限判断和跨模块状态。
- 可复用组件 MUST 通过语义化 props 和 slots 暴露能力，禁止通过大量布尔开关堆出“万能组件”。
- 组件内部所有用户可见文案 MUST 走 i18n，所有颜色 MUST 走主题 token。

### 4.4 Pinia Store 规范

- Store MUST 是跨视图业务状态的唯一来源，禁止在多个页面重复维护同一份远程数据快照。
- Store action MUST 封装业务行为，而不是只充当“远程 API 原样透传”。
- Store 内部 SHOULD 保持最小必要状态，派生值优先用 `computed`。
- Store `MUST NOT` 输出只为某个局部组件存在的一次性 UI 状态，除非该状态跨页面共享。
- 所有异步 action MUST 明确 loading / error / success 状态，不得依赖隐式 UI 判断。

### 4.5 Transport 与副作用规范

- 所有 Hub 通信 MUST 通过统一 Transport 层进入，不得在页面或组件中分散处理 `invoke`、`HTTP`、`SSE`、`webhook` 或 `MCP` 差异。
- 本地 Hub 与远程 Hub 的语义 MUST 保持一致；差异属于传输层，不属于页面层。
- 事件订阅 MUST 有明确注册与释放位置，禁止重复订阅导致泄漏。
- 表单提交、通知弹窗、页面跳转等副作用 SHOULD 由页面或编排层触发，不要深埋在基础组件中。

### 4.6 可访问性与交互要求

- 所有可点击元素 MUST 可通过键盘访问。
- 焦点状态 MUST 可见，且亮暗主题下都可辨识。
- 表单错误 MUST 可感知，不能只靠颜色表达。
- 图标按钮 MUST 提供可理解的标签或辅助文本。

### 4.7 前端代码评审检查项

- 是否违反页面、组件、Store、Transport 的分层边界。
- 是否出现硬编码文案、硬编码颜色或未主题化样式。
- 是否把临时 UI 状态错误地放入全局 Store。
- 是否遗漏 loading / empty / error / disabled / focus 状态。
- 是否考虑本地 Hub 与远程 Hub 的一致语义。

---

## 5. 国际化规范（i18n）

### 5.1 基础约束

octopus 前端 MUST 自首发起支持国际化。

项目默认语言集合固定为：

```ts
type Locale = 'zh-CN' | 'en-US'
```

强制要求：

- 所有用户可见文案 MUST 接入 i18n 体系。
- 新功能在首次提交时 MUST 同时补齐 `zh-CN` 与 `en-US` 文案。
- 语言切换 MUST 支持运行时生效，不要求用户重启客户端。

### 5.2 语言选择与回退顺序

语言解析顺序 MUST 如下：

1. 用户显式选择的语言
2. 系统语言映射结果
3. 默认语言 `zh-CN`

翻译缺失时的运行时回退顺序 MUST 如下：

1. 当前激活语言
2. `zh-CN`
3. `en-US`
4. 统一的共享兜底文案

额外要求：

- `MUST NOT` 直接把 i18n key 渲染到界面上。
- 若进入第 4 级回退，系统 SHOULD 输出开发期告警，提示缺失翻译。

### 5.3 文案组织规则

i18n key MUST 采用稳定、可检索、可扩展的分层命名：

```text
feature.section.item
feature.section.actionLabel
feature.section.empty.title
feature.section.empty.description
```

推荐示例：

- `agent.form.name.label`
- `agent.form.name.placeholder`
- `discussion.feed.empty.title`
- `common.action.save`

禁止示例：

- `title1`
- `button_ok`
- `alex-role-label`
- `some-temp-copy`

### 5.4 禁止硬编码文案的范围

以下位置的展示文案都 MUST 走 i18n：

- 组件模板中的文字
- Store 中生成给 UI 使用的提示语
- 路由守卫里的错误提示或重定向原因
- Toast、Dialog、Empty State、Skeleton 占位说明
- 表单校验错误、确认文案、按钮标签

允许的例外仅限：

- 开发期日志
- 测试用例中的断言文本
- 与协议标准绑定的非展示型常量值

### 5.5 本地化格式化

日期、数字、货币、相对时间 MUST 使用 locale-aware 格式化能力，不得手写字符串拼接。

包括但不限于：

- 日期时间显示
- 数字分组和小数格式
- 百分比
- 相对时间
- 列表连接词

### 5.6 国际化评审检查项

- 是否存在任何用户可见硬编码文本。
- 新增 key 是否同时补齐 `zh-CN` 与 `en-US`。
- key 命名是否稳定且符合 `feature.section.item` 规则。
- 运行时缺失翻译时是否会暴露原始 key。
- 日期和数字是否按 locale 正确格式化。

---

## 6. 主题规范（深色 / 浅色 / System）

### 6.1 基础约束

octopus 前端 MUST 自首发起支持完整主题模式。

项目默认主题模式集合固定为：

```ts
type ThemeMode = 'light' | 'dark' | 'system'
```

强制要求：

- 默认模式 MUST 为 `system`。
- 产品内 MUST 提供显式切换入口。
- 切换主题后界面 MUST 在运行时立即生效。

### 6.2 主题解析顺序

主题模式解析顺序 MUST 如下：

1. 用户显式选择的 `light` 或 `dark`
2. 用户选择 `system` 时，跟随系统主题
3. 首次启动默认 `system`

### 6.3 主题 token 规范

所有颜色语义 MUST 通过主题 token 表达，不得在业务组件中直接散落颜色值。

最小语义 token 集如下：

| 类别 | 必需 token |
|------|-----------|
| 背景 | `background` `foreground` `card` `popover` |
| 边界 | `border` `input` `ring` |
| 强调 | `primary` `secondary` `accent` |
| 状态 | `muted` `destructive` `success` `warning` `info` |

强制要求：

- 业务组件 `MUST NOT` 直接写 `#hex`、`rgb()` 或与语义无关的临时 Tailwind 色值。
- 同一语义在亮暗主题下 MUST 保持一致含义，例如 `destructive` 永远表示危险动作，而不是“刚好看起来更醒目”。
- 新 token SHOULD 先在设计层面确认语义，再进入代码层面。

### 6.4 状态覆盖要求

每个交互组件在亮暗主题下都 MUST 明确定义以下状态：

- default
- hover
- active
- focus
- disabled
- loading
- error
- success

以下区域不得漏适配：

- 表单输入框
- 弹窗、抽屉、Popover
- 表格和列表选中态
- 空态与错误态
- Toast 与危险操作确认区

### 6.5 禁止的主题反模式

以下模式 `MUST NOT` 出现：

- 只适配主页面，忽略弹窗、表单、悬浮层。
- 用透明度碰运气做亮暗适配，不验证可读性。
- 在组件里写死 `text-black`、`bg-white`、`border-gray-200` 作为长期方案。
- 使用颜色本身传达唯一状态，不提供文案或图标辅助。

### 6.6 主题评审检查项

- 是否存在未 token 化的颜色表达。
- 是否支持 `light / dark / system` 三态。
- 切换时是否即时生效且不闪烁错乱。
- 所有交互状态是否都在亮暗主题下可辨识。
- 是否遗漏表单、弹层、空态、错误态等非主区域。

---

## 7. 统一 UI 规范

### 7.1 UI 基线

octopus 前端 MUST 以 `self-built UI components + shared design tokens + UnoCSS` 为统一 UI 基线。

实施原则：

- 优先复用共享基础组件，而不是在业务页面重复造轮子。
- 共享基础组件 SHOULD 保持原子能力；业务组合放在 feature 组件层。
- 新视觉语义优先抽成 token 或公共 variant，而不是在局部页面内临时实现。

### 7.2 布局与间距

- 间距体系 SHOULD 基于 4px 递进，主要布局优先使用 8px 的倍数。
- 同一页面内的区块节奏 MUST 一致，不得混用无规律的间距值。
- 列表、表单、卡片、面板之间的留白 MUST 体现层级，而不是“哪里挤就哪里加 margin”。

### 7.3 排版

- 标题层级 MUST 清晰，同层页面不应出现多套并行标题体系。
- 正文字号、行高和辅助文案风格 SHOULD 在全局保持一致。
- 弱化文本用于说明、元信息和辅助状态，不得承担主要交互信息。

### 7.4 常见组件模式

| 场景 | MUST 遵守的模式 |
|------|----------------|
| 表单 | 标签、说明、错误信息、必填标识、提交中状态完整 |
| 表格 / 列表 | 排序、筛选、空态、加载态、选中态一致 |
| 弹窗 / 抽屉 | 标题、说明、主次按钮层级清晰；危险操作需强化确认 |
| Empty State | 至少包含标题、说明、下一步动作 |
| Error State | 说明失败原因、当前影响、可执行恢复动作 |
| Loading State | 优先使用 skeleton 或局部 loading，避免整页闪烁 |

### 7.5 一致性优先级

当“页面局部更好看”和“系统整体更一致”冲突时，默认优先整体一致性。

以下内容 MUST 在全产品统一：

- 主按钮 / 次按钮 / 危险按钮语义
- 输入框焦点态
- 错误提示样式
- 空态信息结构
- 表单布局节奏
- Toast 呈现方式

### 7.6 响应式与多端准备

虽然 Phase 1 以桌面端为主，但共享前端实现 SHOULD 为后续远程 Hub 和移动端演进留出余地。

- 页面布局 SHOULD 避免依赖固定像素宽度。
- 面板和表格在窄宽度下 MUST 有退化策略。
- 文案长度在 `zh-CN` 与 `en-US` 两种语言下都应避免布局破坏。

---

## 8. 代码风格与开发流程规范

### 8.1 命名与文件规则

| 对象 | 规则 |
|------|------|
| Vue 组件文件 | `PascalCase.vue` |
| Composable | `useXxx.ts` |
| Pinia Store | `useXxxStore` |
| TypeScript 类型 / 接口 | `PascalCase` |
| Rust 模块 / 文件 / 函数 | `snake_case` |
| Rust 类型 / trait / enum | `PascalCase` |
| 常量 | `SCREAMING_SNAKE_CASE` |
| 事件名 | 面向事实的语义名；前端事件字符串推荐 `domain.action` 风格 |
| DTO | `CreateXxxRequest` / `UpdateXxxRequest` / `XxxResponse` |
| 测试名 | 明确行为与结果，例如 `should_reject_empty_system_prompt` |

### 8.2 目录组织规则

- 目录结构 MUST 优先表达边界和职责，不得只按“工具类型”平铺所有代码。
- 前端 feature 代码 SHOULD 按页面或领域聚合，避免把所有组件都堆到同一级。
- 后端模块 SHOULD 与 Context 或子系统对应，避免“misc / common / helpers”成为垃圾桶。

### 8.3 测试与验证门槛

- 新增或修改的业务行为 SHOULD 有相应测试或验证手段。
- 影响公共接口、分层边界、i18n、主题或状态流的改动 MUST 明确验证方式。
- 合并前 MUST 至少验证本次变更涉及的关键路径，不得只靠肉眼判断。
- 若因阶段原因暂不补自动化测试，PR 中 MUST 明确写出风险与人工验证步骤。
- 当前默认验证集合 SHOULD 以文档存在性检查、旧引用扫描、聚焦 diff 复核和 `git diff --check` 为基础；若改动触达 `contracts/`、`packages/`、`apps/` 或 `crates/`，还 SHOULD 执行 `pnpm run typecheck:web`、`pnpm run test:web`、`cargo test --workspace` 与 `cargo build --workspace`。

### 8.4 Git 与提交规范

- 分支名 SHOULD 使用 `type/topic` 结构，例如 `feature/multi-hub-switch`、`fix/theme-token-leak`、`docs/engineering-standard`。
- 提交信息 SHOULD 使用可读、可检索的语义化格式；推荐 Conventional Commits。
- 一次提交 SHOULD 聚焦单一意图，避免把文档、重构、功能和格式化混在一起。

### 8.5 PR 与 Review 要求

每个 PR MUST 清楚说明：

- 改了什么
- 为什么改
- 风险点是什么
- 如何验证
- 是否影响文档、i18n、主题、公共接口或迁移

Reviewer MUST 至少检查以下内容：

- 是否遵守层级边界
- 是否引入硬编码文案或硬编码颜色
- 是否破坏主题 / i18n / tenant / 审批等系统级约束
- 是否补足必要文档与验证说明
- 若变更涉及治理入口或契约源，是否同步更新 `README.md`、`AGENTS.md`、`docs/CONTRACTS.md`、`.github/workflows/guardrails.yml` 与 `.github/pull_request_template.md`

### 8.6 Definition of Done

一个变更只有在以下条件同时满足时才算完成：

1. 实现符合本规范与相关设计文档。
2. 涉及的关键路径已验证。
3. 必要的文档、类型、迁移、示例或接口说明已同步。
4. 前端改动已检查 i18n 与亮暗主题兼容。
5. Review 问题已处理或明确记录待处理原因。
6. 若仓库仍处于 `doc-first` 阶段，结论没有超出当前 tracked tree 能证明的事实边界。

---

## 9. 附录示例与反例

### 9.1 Vue 组件示例

```vue
<script setup lang="ts">
const props = defineProps<{
  titleKey: string
  descriptionKey: string
}>()

const { t } = useI18n()
</script>

<template>
  <section class="rounded-xl border bg-card text-card-foreground">
    <header class="space-y-1 p-4">
      <h2 class="text-lg font-semibold">{{ t(props.titleKey) }}</h2>
      <p class="text-sm text-muted-foreground">
        {{ t(props.descriptionKey) }}
      </p>
    </header>
  </section>
</template>
```

要点：

- 文案通过 `t()` 输出。
- 颜色通过语义 token 输出。
- 组件边界通过 `props` 保持清晰。

### 9.2 Pinia Store 示例

```ts
export const useAgentStore = defineStore('agent', () => {
  const agents = ref<Agent[]>([])
  const loading = ref(false)
  const error = ref<string | null>(null)
  const transport = useTransport()

  async function fetchAll() {
    loading.value = true
    error.value = null

    try {
      agents.value = await transport.call<Agent[]>('list_agents')
    } catch (err) {
      error.value = 'agent.list.loadFailed'
      throw err
    } finally {
      loading.value = false
    }
  }

  return { agents, loading, error, fetchAll }
})
```

要点：

- Store 负责业务状态与行为，而不是页面拼装。
- 错误对 UI 暴露为可翻译 key，而不是硬编码句子。

### 9.3 Rust Service / Repository 示例

```rust
pub struct AgentService<R: AgentRepository> {
    repo: R,
}

impl<R: AgentRepository> AgentService<R> {
    pub async fn get_agent(
        &self,
        tenant_id: &TenantId,
        agent_id: &AgentId,
    ) -> Result<Agent, AgentError> {
        let agent = self.repo
            .find_by_id(tenant_id, agent_id)
            .await?
            .ok_or(AgentError::NotFound)?;

        Ok(agent)
    }
}
```

要点：

- Service 负责边界语义。
- Repository 负责持久化实现。
- tenant_id 显式进入边界，而不是依赖调用方默认正确。

### 9.4 i18n 组织示例

```json
{
  "agent": {
    "form": {
      "name": {
        "label": "名称",
        "placeholder": "请输入 Agent 名称"
      }
    }
  },
  "common": {
    "action": {
      "save": "保存"
    }
  }
}
```

### 9.5 主题 token 示例

```css
:root {
  --background: 0 0% 100%;
  --foreground: 222 47% 11%;
  --card: 0 0% 100%;
  --card-foreground: 222 47% 11%;
  --border: 214 32% 91%;
  --primary: 222 89% 55%;
  --destructive: 0 84% 60%;
}

.dark {
  --background: 222 47% 11%;
  --foreground: 210 40% 98%;
  --card: 224 39% 14%;
  --card-foreground: 210 40% 98%;
  --border: 217 33% 24%;
  --primary: 213 94% 68%;
  --destructive: 0 72% 51%;
}
```

### 9.6 反例

以下写法 `MUST NOT` 出现：

```vue
<template>
  <div class="bg-white text-black">
    Save Agent
  </div>
</template>
```

原因：

- `Save Agent` 是硬编码英文文案，不可国际化。
- `bg-white text-black` 是硬编码颜色，不支持深浅主题。
- 没有语义层，无法进入统一设计系统。

---

*本规范是 octopus 的默认开发基线。若与临时实现冲突，以本规范为准；若与架构文档冲突，必须先更新设计文档并完成例外说明。*
