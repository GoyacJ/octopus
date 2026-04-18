# 项目管理 / 项目配置重构开发计划

## 摘要

本次按“结构重构 + 分阶段落地 + 同步清理数据边界”执行，目标是把两类任务彻底分开：

- `控制台-项目管理` 只负责项目注册、生命周期和基础元数据，成为轻量的项目台账页。
- `项目-项目配置` 成为唯一的高级配置中心，但内部明确区分 `工作区授予`、`项目启用`、`项目覆盖` 三层。
- 设计上遵循 `docs/design/DESIGN.md` 的 document-style 设置页，不模仿 Notion 视觉，只借鉴其“默认极简、渐进展开、摘要优先”的产品原则。

核心验收口径：

- 新建项目只需要最少字段即可完成。
- 同一配置项在全产品内只有一个主编辑入口。
- 用户能一眼看懂“这个能力是工作区给的，还是项目自己启用/覆盖的”。
- 页面默认不再出现超长复选框墙。

## 阶段计划

### Phase 1: 先收敛职责和写路径

先完成数据和页面职责对齐，避免 UI 改完后逻辑仍然混乱。

- 把 `ProjectsView` 收敛为“项目注册中心”：
  - 保留 `名称 / 描述 / 资源目录 / 状态 / 预设`。
  - 删除默认展开的 `模型 / 工具 / 数字员工 / 团队 / Token 额度` 选择器。
  - 改为只显示只读摘要行，如“模型 2 个 / 工具 8 个 / 数字员工 3 个 / 成员 6 人”。
  - 增加主 CTA：`打开项目配置`。
- 把 `ProjectSettingsView` 定义为唯一高级配置中心：
  - 负责能力授予、运行时细化、成员与访问。
  - 不再承担项目基础元数据编辑，名称/描述/资源目录只在 `项目管理` 改。
- 清理写路径归属：
  - `updateProject()` 只用于 `项目元数据 / 项目成员 / 项目 assignments`。
  - `saveProjectModelSettings / saveProjectToolSettings / saveProjectAgentSettings` 只用于 `projectSettings`。
  - 删除 `ProjectSettings` 中“改启用状态同时回写 assignments”的双写行为；授予和启用必须拆成两个明确动作。
- 补一层派生状态选择器，明确三套数据：
  - `workspace 可用全集`
  - `project.assignments 授予集`
  - `projectSettings 运行时启用/覆盖集`

### Phase 2: 重做“项目管理”页为轻量台账页

在不改路由的前提下重写 `workspace-console-projects` 的交互。

- 保持当前 list/detail 骨架，但详情区只展示两组内容：
  - `基础信息`：名称、描述、资源目录、状态。
  - `配置摘要`：模型/工具/数字员工/成员的计数和当前默认项。
- 新建/编辑流程统一为“最小可用创建”：
  - 必填：名称、资源目录。
  - 选填：描述、预设。
  - 创建后直接落到项目详情，并提供 `进入高级配置` 按钮。
- 引入 `预设`，但 v1 只做前端映射，不改 OpenAPI：
  - `通用项目`
  - `研发项目`
  - `文档项目`
  - `高级自定义`
- 预设仅用于初始化默认 assignments/runtime 建议值，不作为后端持久字段；后续如需统计和审计，再单独引入 `presetCode`。
- `归档/恢复` 保留在项目管理页；生命周期动作不放进项目配置页。

### Phase 3: 重做“项目配置”页为文档式高级配置中心

把现有 `tabs + card` 模式改成文档页，不再让用户在五个 Tab 间来回切。

- 页面骨架：
  - 保留现有 `project-settings` 路由和 owner-only guard。
  - 顶部 `UiPageHeader` 显示项目名、状态、一句解释。
  - 主体改为“左侧文档区 + 右侧粘性概览/完成度”，不再使用页级 `UiTabs`。
- 主文档区固定为 4 个区块，顺序不可变：
  1. `配置概览`
  2. `可用于此项目`：编辑 `project.assignments`
  3. `在此项目中启用`：编辑 `projectSettings`
  4. `成员与访问`：编辑成员；权限覆盖先显示摘要和说明，不在本轮复制一套完整 ACL 编辑器
- 每个区块默认只显示摘要，不直接展开大列表：
  - 模型：`已授予 3 个，项目启用 2 个，默认 Claude Primary`
  - 工具：`已授予 12 个，启用 10 个，2 个权限覆盖`
  - 数字员工：`已授予 4 个，启用 2 个`
  - 成员：`6 人，其中 2 人可编辑`
- 摘要行点击后用 `UiDialog` 打开局部编辑器；不新造 Drawer。
- 工具覆盖等复杂内容在对话框内再用 `UiAccordion` 做高级展开，默认不全量摊开。
- 页面文案统一显式标注来源：
  - `工作区授予`
  - `项目启用`
  - `继承工作区`
  - `项目覆盖`
- `Token 配额` 放在“在此项目中启用”区块，和默认模型一起作为运行时配置，不再出现在项目管理页。

### Phase 4: 首次体验、文案和一致性收尾

在结构稳定后补齐易用性和一致性。

- 让 `WorkbenchSidebar` 的快速新建项目入口与 `项目管理` 页共用同一套最小创建表单和预设逻辑。
- 在项目配置页右侧概览增加“配置完成度/建议下一步”，只做轻提醒，不做强向导。
- 空状态和说明文案统一成低认知负担表达：
  - 不说“运行时配置”“Assignment”等内部术语。
  - 对用户只展示“可用 / 已启用 / 已覆盖”。
- 与 `访问控制` 模块复用术语体系和层级说明，但不复制其复杂视觉结构。
- 保持现有菜单和路由名称不变，避免打断已存在的导航和测试入口。

## 接口与实现约束

- 本轮不改现有公开路由：
  - `workspace-console-projects`
  - `project-settings`
- 本轮不要求 OpenAPI 变更：
  - `CreateProjectRequest`
  - `UpdateProjectRequest`
  - `ProjectSettingsConfig`
  保持兼容。
- 本轮只做内部职责收敛：
  - `ProjectRecord.assignments` = 工作区授予到项目的能力范围
  - `ProjectSettingsConfig` = 项目在授予范围内的启用与覆盖
- 需要新增的仅是前端内部 view-model / selector：
  - `ProjectSetupPreset`
  - `ProjectCapabilitySummary`
  - `ProjectGrantState`
  - `ProjectRuntimeRefinementState`
- `项目基础信息` 的唯一编辑入口是 `ProjectsView`。
- `模型/工具/员工/成员` 的高级编辑入口统一在 `ProjectSettingsView`。
- `权限覆盖` 本轮只做摘要展示和定位说明；若要做完整项目级 ACL 编辑器，单列后续计划。

## 测试与验收

必须更新并新增以下测试场景：

- `ProjectsView`
  - 创建项目时只要求最小字段，默认不出现模型/工具/员工的大列表。
  - 预设切换会填充建议摘要，但不会要求用户立即处理高级配置。
  - 详情区能显示配置摘要和 `打开项目配置` 入口。
  - 归档/恢复行为保持不变。
- `ProjectSettingsView`
  - 页面不再依赖页级 tabs，而是渲染 4 个固定区块。
  - “可用于此项目” 修改只写 `updateProject(assignments)`。
  - “在此项目中启用” 修改只写 `saveProject*Settings`。
  - 同一资源的“授予”和“启用”状态能同时显示且不会混淆。
  - Token 配额只在项目配置页编辑。
- 路由/权限
  - 深链接到 `project-settings` 仍可访问。
  - owner-only guard 保持不变。
  - `workspace-console-projects` 仍能嵌入 `WorkspaceConsoleView`。
- 回归场景
  - 现有 fixture 数据无需迁移即可正确显示。
  - 已有项目 assignments 和 runtime config 能被正确拆解成“授予/启用/覆盖”三层摘要。

## 假设与默认

- 默认采用分 4 个 PR 落地，而不是一次性大改：
  - PR1 写路径和 selector 收敛
  - PR2 项目管理页瘦身
  - PR3 项目配置页重写
  - PR4 预设、文案、统一性和回归
- 默认不新增后端字段，也不做数据迁移。
- 默认不模仿 Notion 视觉，只采用其“摘要优先、渐进披露、默认即好用”的方法。
- 默认不在本轮把项目权限做成完整 ACL 子系统；先把成员和配置边界理顺，避免范围失控。
