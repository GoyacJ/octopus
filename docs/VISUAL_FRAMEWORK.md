# Octopus 视觉框架与信息架构基线

更新时间：2026-03-25  
文档状态：Draft  
文档定位：控制面视觉框架、信息架构与页面边界  
关联文档：`docs/PRD.md`、`docs/SAD.md`、`docs/ARCHITECTURE.md`、`docs/ENGINEERING_STANDARD.md`

---

## 1. 文档目的

本文档用于锁定 `Octopus` 控制面的表面职责、一级导航、关键页面边界与模型管理信息架构，避免前端实现出现导航漂移、职责混叠和文档不一致。

---

## 2. 设计方向

`Octopus` 的视觉方向保持以下原则：

1. 现代简约，强调秩序与治理感。
2. 冷静克制，不做聊天玩具化表达。
3. 强信息层级，列表、侧栏、表单、时间线优先。
4. 多端语法一致，允许信息密度不同但不允许语义漂移。
5. 高风险、等待、审批、异常、恢复必须具备统一视觉语义。

实现边界：

1. 颜色、间距、圆角、阴影、动效来自共享 tokens。
2. 用户文案必须支持 `zh-CN` 与 `en-US`。
3. 主题必须支持 `system`、`light`、`dark`。

---

## 3. 表面职责

| 表面 | 角色 | 当前职责 |
| --- | --- | --- |
| Web | 主控制面 | Hub 管理、Agent 管理、Team/Discussion、模型中心、扩展治理 |
| Desktop | 强控制面 | 复用 Web 结构，增加本地 Hub 与本机集成入口 |
| Mobile | 轻控制面 | 审批、状态查看、轻量接管、通知跳转 |

---

## 4. 一级信息架构

Web / Desktop 的一级导航按业务域组织：

1. `Overview`
2. `Agents`
3. `Teams`
4. `Discussions`
5. `Inbox`
6. `Hub Management`
7. `Extensions`
8. `Audit`
9. `Settings`

说明：

1. `Hub Management` 是管理员视角入口，承载用户、租户、模型中心、MCP 注册等管理能力。
2. `Extensions` 下继续分 `Skills`、`Tools`、`MCP` 等能力治理面。
3. 当前文档未为重型任务编排器、Marketplace 或营销页面预留一级入口。

---

## 5. 模型管理信息架构

### 5.1 Hub Management > Models

模型管理是 Hub 管理面下的一级子页面，不与 Agent 编辑页混合：

1. `Providers`
   - 配置 Provider 名称、端点、鉴权方式、secret binding、状态。
2. `Catalog`
   - 管理 Provider 下可用模型条目，区分 `inference` / `embedding`。
3. `Profiles`
   - 生成对业务可见的模型档案，固定名称、说明和参数预设。
4. `Tenant Policies`
   - 为租户配置允许使用的模型档案与默认绑定。

页面目标：

1. Provider 关注“连接与凭据”。
2. Catalog 关注“原始模型目录”。
3. Profile 关注“给业务对象消费的稳定档案”。
4. Tenant Policy 关注“可见性与默认值”。

### 5.2 Agent 编辑页

Agent 编辑页不再让普通用户直接输入 `provider + model + api_key_ref`。

Agent 能力配置改为：

1. 选择一个 `model_profile_id`。
2. 只读展示 `model_profile_summary`：
   - 档案名称
   - 模型类型（推理 / Embedding 不允许混用）
   - Provider 名称
   - 实际模型 ID
   - 参数预设摘要

设计原则：

1. 选择器展示“业务友好名称”，不是裸模型字符串。
2. 仅展示当前租户允许的模型档案。
3. 默认值来自 `TenantModelPolicy`，用户可以在允许范围内覆盖。

### 5.3 本地 Hub 与远程 Hub 差异

本地单用户 Hub：

1. 默认只有一个租户策略视图。
2. 更强调 Provider 启用和默认模型初始化。
3. 如果用户选择本地 Ollama，优先引导启用本地推理和本地 Embedding 档案。

远程多租户 Hub：

1. `Hub Admin` 负责 Provider / Catalog / Profile。
2. `Tenant Admin` 负责 Tenant Policy。
3. 普通成员只在 Agent 创建页消费模型档案，不直接接触 Provider 密钥。

---

## 6. 关键页面边界

当前文档重点覆盖的页面：

1. 应用壳与全局导航。
2. Agent 列表与详情壳。
3. Agent 编辑页的模型档案选择器。
4. Hub Management > Models。
5. Hub Management > Users / Tenants。

当前未在本文重点展开的页面：

1. 模型成本看板。
2. 模型健康探测可视化。
3. 自动 fallback 编排 UI。
4. 插件 Marketplace。

---

## 7. 布局语法

### 7.1 Web / Desktop

1. 左侧导航固定业务域。
2. 顶部栏承载当前 Hub、当前租户、搜索、主题、语言、身份操作。
3. 主内容区优先采用“列表 + 详情”“表格 + 侧栏”“表单 + 预览”的治理型布局。

### 7.2 Mobile

1. 以审批、状态查看和轻量接管优先。
2. 长表单与模型中心管理页默认不作为一线入口。
3. 若需要查看模型信息，仅展示 Agent 绑定摘要，不提供管理员配置入口。

---

## 8. 组件与文案边界

1. 业务页面只能消费内部 UI 组件和共享 tokens。
2. 模型类型、Provider 状态、默认绑定位点等都使用统一语义标签组件。
3. 不在组件内硬编码中英文文案。
4. 不在 Agent 配置页暴露 secret binding 或原始 API Key。

---

## 9. 维护规则

以下变化必须同步更新本文档：

1. 一级导航调整。
2. 模型中心信息架构调整。
3. Agent 编辑页配置流程调整。
4. 本地 Hub 与远程 Hub 的入口职责变化。
