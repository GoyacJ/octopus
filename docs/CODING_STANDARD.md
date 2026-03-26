# Octopus · 实现级开发规范

**状态**: 基线建立版 | **日期**: 2026-03-26
**适用范围**: 未来进入 tracked tree 的源码、测试、构建脚本与近端子树 `AGENTS.md`

---

## 1. 文档目标

本规范补齐 Octopus 在“真正开始写代码”阶段所需的统一实现规则，重点回答：

- 代码风格如何保持一致
- 前后端分层与设计模式如何收敛
- 正式对象、契约、状态机如何落到代码
- AI 写代码时哪些写法被允许，哪些写法应被禁止

本规范不改变 [`PRD.md`](./PRD.md) 的产品边界，也不改变 [`SAD.md`](./SAD.md) 的架构主决策。它只负责把这些上层约束转成实现级开发规则。

当前仓库仍处于 `doc-first rebuild` 阶段。在真实源码与 manifests 进入 tracked tree 前，本规范主要用于：

- 指导项目骨架设计
- 指导 contract 到实现的映射
- 约束 AI 在未来生成代码时保持风格稳定

## 2. 与其他规范的关系

实现级规则按以下顺序解释：

1. 显式用户指令
2. [`AGENTS.md`](../AGENTS.md)
3. [`PRD.md`](./PRD.md)
4. [`SAD.md`](./SAD.md)
5. [`ENGINEERING_STANDARD.md`](./ENGINEERING_STANDARD.md)
6. 本文档
7. [`AI_ENGINEERING_PLAYBOOK.md`](./AI_ENGINEERING_PLAYBOOK.md)
8. [`AI_DEVELOPMENT_PROTOCOL.md`](./AI_DEVELOPMENT_PROTOCOL.md)
9. [`VISUAL_FRAMEWORK.md`](./VISUAL_FRAMEWORK.md)
10. [`DELIVERY_GOVERNANCE.md`](./DELIVERY_GOVERNANCE.md)

规则：

- 本文档只能细化实现方式，不能改写上层对象边界。
- 若实现级选择会反向改变正式对象语义，必须回到 `SAD` 或 contract，而不是直接改代码规范。

## 3. 总体实现原则

Octopus 的代码实现必须持续满足以下原则：

1. 对象优先，不以临时 UI 或第三方 SDK 结构作为主模型
2. 状态显式，不用隐式字符串和散落布尔值表达核心状态
3. 分层清晰，避免 UI、状态管理、协议接入和治理判断互相穿透
4. 副作用收口，避免网络、文件、工具执行逻辑到处扩散
5. 错误可解释，失败必须可分类、可追溯、可恢复
6. 审计内建，关键动作必须能映射到 `Run / Approval / Trace / Knowledge` 等正式对象

## 4. 通用代码风格

### 4.1 命名

- 类型、类、接口、枚举、组件名使用 `PascalCase`
- 变量、函数、方法、composable、store 实例使用 `camelCase`
- 常量使用 `UPPER_SNAKE_CASE`，但不用于普通对象字面量 key
- 文件名按其职责命名，不按“临时实现阶段”命名
- 不使用 `misc`、`helper`、`temp`、`utils2`、`final-final` 这类弱语义命名

### 4.2 术语

- 正式对象保持与 `PRD/SAD/contract` 一致
- 外部协议名、SDK 名、供应商名不得直接替代正式领域名
- 视图层允许有展示文案，但领域层、状态层、契约层必须使用正式术语

### 4.3 函数与模块

- 单个函数应只负责一个明确动作
- 单个模块应围绕单一职责组织，不混合领域规则、协议适配与视图格式化
- 避免“万能 util 文件”；共用逻辑必须按职责归位
- 优先纯函数处理派生逻辑，把副作用封装在明确边界内

### 4.4 注释

- 只为“为什么这样做”写注释，不为显而易见的语句写注释
- 对恢复、幂等、权限、审批、知识写回等非直观约束，可写短注释说明
- 不写会快速过期的过程性注释

## 5. 分层与设计模式

### 5.1 前端实现模式

前端默认按以下四层组织：

- `page/screen`：页面壳层、布局和路由级组合
- `domain component`：围绕 `Run`、`ApprovalRequest`、`InboxItem`、`TraceEvent` 等正式对象的组件
- `primitive component`：基础组件与视觉原语
- `composable/store`：状态读取、用户动作编排、异步流程与派生状态

前端禁止：

- 页面组件直接承担正式权限判断
- 页面组件直接持有第三方 connector 细节
- 用样式状态代替正式领域状态
- 把 `Notification`、`Toast`、`Banner` 当作 `InboxItem` 的正式载体

### 5.2 状态管理模式

Pinia 或等价状态层默认只做：

- authority object 的本地投影
- 查询结果缓存
- 视图派生状态
- 用户动作的编排入口

状态层禁止：

- 重新发明一套独立于 contract 的领域模型
- 直接把第三方返回结构暴露给页面
- 用自由字符串拼出状态机
- 在 store 内静默吞掉失败

建议模式：

- 一个 store 负责一个明确对象域或工作面
- 跨 store 协作通过显式 action 或 application-level orchestration 完成
- 页面拿到的是 view model，不是原始外部 payload

### 5.3 Rust / Tauri / Hub 侧实现模式

后端或桌面侧逻辑默认按以下边界组织：

- `domain`：对象语义、状态机、约束、不可变规则
- `application`：用例编排、事务边界、审批和治理流程串联
- `adapter`：协议接入、存储、外部工具、MCP/A2A/HTTP 适配
- `infra`：运行时、日志、持久化驱动、配置、框架胶水

禁止模式：

- 在 domain 中直接调用外部服务
- 在 adapter 中私自改写正式对象语义
- 把 command handler 写成“所有逻辑都在一处”的大函数
- 用框架类型把领域层锁死

## 6. Vue / TypeScript 具体规则

### 6.1 组件与脚本

- 默认使用 Vue 3 Composition API 与 `<script setup lang="ts">`
- 组件按职责命名，页面组件与领域组件分开
- `PascalCase.vue` 用于组件；`useXxx.ts` 用于 composable；`useXxxStore.ts` 用于 store
- 组件 props、emits、models 应显式类型化
- 非局部状态不要藏在组件内部

### 6.2 数据与 view model

- 页面组件不直接消费外部原始响应
- 先做 DTO 到 view model 的映射，再进入页面或领域组件
- 列表行、详情面板、状态徽标等重复展示结构应抽成稳定 view model

### 6.3 UI 规则

- 组件层必须遵守 [`VISUAL_FRAMEWORK.md`](./VISUAL_FRAMEWORK.md)
- 状态样式只映射正式状态，不允许一个组件自创颜色和状态含义
- 高风险动作必须有明确确认语义，不依赖 tooltip 或 hover

## 7. 契约、状态机与事件落地规则

### 7.1 契约映射

- 公共 API、事件 envelope、持久化对象、跨平面读写都应先对齐 contract
- 代码中的 DTO、entity、projection、view model 角色要分清
- `authority source` 与 `projection/cache` 必须在类型和命名上可区分

### 7.2 状态机

- 核心状态使用枚举或等价封闭集合，不用自由文本
- 状态迁移应集中定义，不要散落在多个页面、store 或 handler 中
- 状态变化必须能解释进入条件、退出条件、异常路径和恢复路径

### 7.3 事件

- 事件名优先使用过去式，例如 `RunSubmitted`、`ApprovalGranted`
- 事件必须带最小可追溯上下文，例如 object id、actor、time、source
- 审计事件与领域事件可以关联，但不要混成一个“万能事件”

## 8. 错误处理与日志

### 8.1 错误处理

- 错误必须可分类，例如 validation、permission、budget、approval、external、recovery
- 不允许静默吞错，除非同时记录明确降级路径
- 用户可见错误与内部技术错误要分层表达
- 对外部协议结果默认低信任，不直接当系统事实

### 8.2 日志与观测

- 日志优先结构化，而不是拼接长文本
- 关键日志应带 `run_id`、对象 id、actor、source、decision 或 trace 关联键
- 高风险动作、审批、授权拒绝、恢复、补偿建议必须能进入审计链

## 9. 测试与验证规则

当前仓库仍以文档验证为主，但未来源码进入 tracked tree 后，最小要求如下：

- 领域对象变更：unit + state machine/contract tests
- 权限和治理变更：policy matrix tests
- 恢复变更：idempotency、resume、replay tests
- 交互面变更：关键状态呈现与主任务流 tests
- 协议适配变更：adapter contract tests，不直接依赖第三方在线环境作为唯一验证

实现层禁止：

- 只测 happy path
- 把人工肉眼点击当作唯一验证
- 在没有可运行测试树时谎称“测试通过”

## 10. Code Review Checklist

代码评审至少检查以下问题：

1. 是否仍使用正式对象命名，而不是临时别名
2. 是否把领域规则错误放进 UI、store 或 adapter
3. 是否把状态机实现成自由字符串或分散布尔值
4. 是否漏掉异常路径、恢复路径或幂等边界
5. 是否把外部低信任结果直接写进正式事实
6. 是否让 `ToolSearch`、UI 或第三方工具绕过治理链
7. 是否把页面视觉状态和正式领域状态混淆
8. 是否有明显的“大而全模块”或职责穿透
9. 是否有与 contract 不一致的字段、状态或事件命名
10. 验证是否与当前仓库真实能力匹配

## 11. AI 禁止写法清单

AI 在生成代码时默认禁止：

- 先写一大批目录和脚手架，再补真实切片
- 先写页面和 mock 状态，再回头补正式对象
- 直接把第三方 SDK response 当领域模型
- 把权限判断藏在前端可见性逻辑里
- 通过 prompt、注释或命名暗示代替正式状态字段
- 使用 `any`、弱类型 map、自由文本状态掩盖对象边界
- 把所有逻辑塞进单个 page、store、handler 或 service

## 12. 与近端子树 AGENTS 的关系

当未来出现 `apps/`、`crates/`、`packages/` 等真实实现子树时：

- 本文档继续作为仓库级实现基线
- 近端子树 `AGENTS.md` 应在不冲突的前提下细化本地规则
- 具体 formatter、linter、test 命令、模块布局由近端子树 `AGENTS.md` 约束

在相关 manifests、formatter、linter、test runner 真实存在前，不得声称这些工具已经执行成功。

## 13. 结论

Octopus 需要的不是“大家大概写得差不多”，而是让人类和 AI 都在同一套对象边界、分层方式、状态语义和 review 规则下写代码。只有这样，后续真正进入实现阶段时，代码风格、设计模式和治理约束才不会重新发散。
