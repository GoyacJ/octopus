# Claude Code 式 Tooling Runtime 重建设计

## Summary

如果目标是：

- `builtin tools / skill / mcp` 的整体设计对齐 Claude Code
- Octopus 只保留并强化自己的 `permission` 与 `import/export`
- 不考虑兼容性，只追求最佳设计

那正确方向不是“在现有实现上补齐功能”，而是 **直接重建一套新的 tooling runtime**，让现有实现只作为迁移素材，不再作为架构基线。

核心判断固定为：

- **Claude Code 的一等公民不是 tool、skill、mcp，而是 runtime capability system。**
- Octopus 也应该改成这个模型。
- `builtin / skill / mcp` 只是 capability 的不同来源，不应该再是三套主干。
- `permissions` 不是外挂，而是 capability runtime 的内建控制面。
- `import/export` 不是 runtime 架构核心，而是 asset/management plane 的功能。

这次设计应直接采用“三层重建”：

1. `Capability Asset Layer`
2. `Capability Runtime Layer`
3. `Capability Management Layer`

其中真正决定 Claude Code 体验的是第二层。

## Key Changes

### 1. 彻底放弃“工具类型驱动”的主模型，改成 Capability 驱动

新的 canonical 对象不是 `ToolDefinition`，也不是 `SkillDocument`，而是：

- `CapabilitySpec`
- `CapabilityHandle`
- `CapabilitySurface`
- `CapabilityExecutionPlan`

统一字段至少包含：

- `capability_id`
- `source_kind`
  - `builtin`
  - `local_skill`
  - `bundled_skill`
  - `mcp_tool`
  - `mcp_prompt`
  - `plugin_tool`
  - `plugin_skill`
- `execution_kind`
  - `tool`
  - `prompt_skill`
  - `resource`
- `display_name`
- `description`
- `when_to_use`
- `input_schema`
- `search_hint`
- `visibility`
  - `default_visible`
  - `deferred`
  - `hidden`
- `state`
  - `ready`
  - `pending`
  - `auth_required`
  - `approval_required`
  - `degraded`
  - `unavailable`
- `permission_profile`
- `trust_profile`
- `invocation_policy`
- `concurrency_policy`

运行时只消费 `CapabilitySpec`，不再直接消费“内置工具表 / skill 文件 / mcp bridge 状态”。

### 2. 把 runtime 设计成 Claude Code 式控制面，而不是 registry

新的运行时主干固定为 5 个组件：

1. `CapabilityProviders`
   - 分别从 builtin、skill、mcp、plugin 提供原始能力
2. `CapabilityCompiler`
   - 把不同来源编译成统一 `CapabilitySpec`
3. `CapabilityPlanner`
   - 每轮请求前构建本轮 `effective capability surface`
4. `CapabilityExecutor`
   - 统一执行 tool / skill / MCP 调用
5. `CapabilityStateStore`
   - 维护 session 内的激活、授权、pending、deferred、selection 状态

重点是：

- **先计划能力暴露面，再发给模型**
- 而不是像现在这样，先给模型静态工具集合，再在执行时拦截

这意味着权限、MCP 状态、skill 激活状态，都会先作用在 `surface planning`，不是事后兜底。

### 3. Skill 不再是“读 SKILL.md 的工具”，而是 Prompt Capability

这里必须严格对齐 Claude Code 思路。

Skill 的本质不是 function tool，而是：

- 可被用户调用
- 可被模型调用
- 可改变后续上下文和工具表面
- 可选择 inline 执行或 fork 执行
- 可带 frontmatter 语义

所以 skill 在新模型里属于 `execution_kind = prompt_skill`。

Skill frontmatter 直接升级为第一等规范：

- `name`
- `description`
- `when_to_use`
- `allowed-tools`
- `arguments`
- `paths`
- `user-invocable`
- `model-invocable`
- `agent`
- `model`
- `effort`
- `context`
  - `inline`
  - `fork`

运行时行为固定为：

- skill invocation 不是“返回文档文本”
- 而是产出 `SkillExecutionResult`
  - `messages_to_inject`
  - `tool_grants`
  - `model_override`
  - `effort_override`
  - `state_updates`

也就是说，Skill 不是 registry 附件，而是 runtime 一级能力。

### 4. Tool 与 Skill 分层统一，不强行压成同一种东西

这里要避免一个常见误区：为了统一而把 skill 变成普通 tool。

Claude Code 没这么做，Octopus 也不该这么做。

正确方式是：

- **统一在 capability 层**
- **区分在 execution 层**

统一的是：

- source model
- planning
- permissions
- discovery
- lifecycle
- telemetry
- tracing
- management

区分的是：

- `tool` 走 function/tool-call 执行链
- `prompt_skill` 走 prompt-expansion / fork-agent 执行链
- `resource` 走 read/reference 链

所以应保留两个模型侧入口：

- `ToolSearch`
- `SkillTool` / `SkillDiscovery`

而不是让 `ToolSearch` 同时承担 skill 搜索。

### 5. MCP 也不要再作为旁路系统，而是作为 Capability Provider

MCP 在新架构里应该只有一个角色：

- `McpCapabilityProvider`

它负责把 MCP server 的三类能力导入 runtime：

- MCP tools -> `execution_kind = tool`
- MCP prompts / MCP skills -> `execution_kind = prompt_skill`
- MCP resources -> `execution_kind = resource`

同时 MCP provider 自己还维护连接态：

- `pending`
- `auth_required`
- `approval_required`
- `ready`
- `degraded`
- `unavailable`

这些状态直接影响 capability surface。

固定规则：

- 未 ready 的 MCP capability 不能进入默认 visible surface
- 但可以进入 discovery/search 结果，并带状态返回
- MCP auth / approval / elicitation 是 runtime mediation 机制的一部分
- MCP-derived skill 禁止隐式 shell 注入，必须通过显式授权工具调用

### 6. Permission 不再只是执行时授权，而是 runtime 的第一控制面

Octopus 最值得保留的是权限体系，但要重构接入位置。

新设计里权限要分两层：

1. `Exposure Policy`
   - 决定 capability 能否暴露给模型
2. `Execution Policy`
   - 决定 capability 被选中后能否实际执行

也就是：

- Claude Code 的 deny-before-expose
- 再加上 Octopus 自己更强的 ask/allow/deny/trust/auth 控制

每个 capability 必须内建：

- `required_permission_mode`
- `approval_policy`
- `trust_requirement`
- `auth_requirement`
- `scope_constraints`

每轮 planning 时先做：

- source filtering
- trust filtering
- permission filtering
- pending/auth/degraded filtering
- deferred classification

执行前再做：

- pre-hook
- approval/auth/elicitation mediation
- execution
- post-hook/failure-hook

这才是“把 Octopus 权限融入 Claude Code”，而不是把 Claude 式工具体系外面再包一个拦截器。

### 7. ToolSearch 应该升级为 runtime discovery，不再只是 registry 搜索

如果采用这次的重建设计，`ToolSearch` 的角色会非常明确：

- 它是 `tool` capability 的 deferred discovery controller
- 它不是 catalog 搜索
- 它也不是 skill 搜索

固定职责：

- 搜索 deferred 的 `tool` capabilities
- 返回结构化 metadata
- 支持 `select:<tool>`
- 选中后把目标 tool 放入后续轮次的 `effective tool surface`

返回结构至少包含：

- `name`
- `source_kind`
- `description`
- `permission`
- `state`
- `requires_auth`
- `requires_approval`
- `deferred`
- `search_hint`

固定边界：

- `prompt_skill` 不进入 `ToolSearch`
- `skill` 通过 `SkillDiscovery` 或 `SkillTool` 进入模型工作流
- `ToolSearch` 和 UI tools catalog 完全解耦

### 8. Import / Export 放到 Asset Layer，不污染 Runtime 主干

导入导出不是 runtime 核心，应单独定义在 `Capability Asset Layer`。

这个层只负责资产生命周期：

- builtin capability manifest
- local skill package
- bundled skill package
- MCP server config package
- plugin capability package

功能包括：

- skill import/export
- MCP import/export
- capability manifest validation
- package signing / trust metadata
- asset installation / removal
- workspace-scoped enable/disable

但它不负责：

- 本轮模型看到什么
- 本轮工具怎样执行
- 本轮权限怎样判断

这些全部留给 runtime。

### 9. 管理面改成 Projection，不再驱动运行时

桌面端 tools 页面、catalog、启停配置，都改成 `CapabilityManagementProjection`。

管理面只关心：

- capability 来自哪里
- 当前是否安装/启用
- 当前健康状态
- 权限需求
- 导入导出
- 配置编辑
- 调试状态

但它不再是 runtime source of truth。

真正 source of truth 应该是：

- asset manifests
- runtime capability state store
- session capability state

## Public Interfaces

新架构应明确新增或重做这些接口：

- `CapabilitySpec`
- `CapabilitySourceKind`
- `CapabilityExecutionKind`
- `CapabilityState`
- `CapabilityPermissionProfile`
- `CapabilityInvocationPolicy`
- `CapabilitySurface`
- `EffectiveCapabilitySurface`
- `CapabilityCompiler`
- `CapabilityPlanner`
- `CapabilityExecutor`
- `SkillExecutionResult`
- `ToolSearchResult`
- `SkillDiscoveryResult`
- `CapabilityManagementProjection`

并明确废弃当前架构中的“主干地位”：

- `ToolRegistry` 不再做 runtime 中枢
- 当前 `Skill` builtin tool 不再作为最终形态
- 当前 `McpToolRegistry` 不再承担 capability runtime 总入口
- 当前 catalog 只保留管理投影职责

## Test Plan

必须覆盖这些验收场景：

- builtin / local skill / bundled skill / MCP tool / MCP prompt 能全部编译成统一 capability graph
- planning 阶段就能完成 deny-before-expose
- skill invocation 会改变后续 tool surface
- `paths` 条件 skill 不命中时不可见，命中后可见
- MCP `pending/auth_required/degraded/unavailable` 会改变 capability visibility
- `ToolSearch` 只能发现 deferred tool capabilities
- `SkillDiscovery` 只能发现 executable prompt skills
- `ToolSearch select:<tool>` 后，目标进入下一轮 effective surface
- UI catalog 搜索不会改变模型可见能力
- import/export 只影响 asset state，不直接篡改 session runtime state
- read-only capability 可并发，write / auth / approval capability 串行或受 mediation 控制
- 不支持 tool reference 的模型可退化到宿主侧增量暴露

## Assumptions

- 本次方案是彻底重构，不要求兼容现有 `ToolRegistry / Skill / MCP bridge` 代码路径。
- 现有实现只用来提炼行为，不作为新架构骨架。
- 参考对象是 Claude Code 的整体 runtime 设计，而不是某几个工具名字或某个搜索接口。
- Octopus 的差异化能力只保留两块：
  - 更强的 permission / approval / trust 体系
  - 更完整的 import / export / management 面
- 若后续继续细化，下一步不该是“补哪个模块”，而该是先画出新的 `Capability Runtime` 模块边界与数据流，再决定代码落点。
