# Octopus 内置工具基准对比与优化建议

## 目标与结论

本文对三个参考项目的内置工具体系做一次面向实现的对比：

1. `docs/references/claude-code-sourcemap-main`
2. `docs/references/openclaw`
3. `docs/references/hermes-agent`

目标不是做工具名并集，而是回答三个问题：

1. Octopus 现在的 built-in tools 缺什么，哪些设计已经不合理。
2. 哪些能力应该进入 Octopus built-in core，哪些应该留在 plugin / MCP / provider-owned 层。
3. 对于开发期项目，怎样一次性切到长期可演进的工具架构，而不是继续叠加兼容层。

## 文档定位

本文是 built-in tool 架构与工具池裁剪的上位基准，不是直接执行用的任务账本。

- 架构裁剪、命名收敛、built-in / plugin 边界判断，以本文为决策输入。
- 具体落地步骤、文件改动顺序、验证命令与 stop conditions，以 `docs/plans/agent/2026-04-19-toolsearch-exposure-runtime-plan.md` 为执行真相。
- 如果本文建议与更高层治理文档或正式 implementation plan 冲突，以仓库 `AGENTS.md`、`docs/AGENTS.md`、`docs/plans/AGENTS.md` 和具体 implementation plan 为准。

一句话结论：

Octopus 不应该照搬 Claude Code / OpenClaw / Hermes 的完整工具池，而应该吸收三者各自最强的部分，形成一套更干净的四层模型：

1. `BuiltinCapabilityCatalog` 作为 built-in 的唯一真相。
2. `ToolCatalogView` 作为目录、ToolSearch、UI 展示的派生视图。
3. `CapabilityExposureState` 作为 deferred discovery / activation / exposure 的持久化状态。
4. `Plugin / MCP / PromptSkill / Resource` 作为扩展层，不挤进 built-in core。

同时，Octopus 当前的 built-in tools 需要做一次 clean-cutover：

- 从平铺注册表切到分类 catalog。
- 从字符串分发切到 catalog-backed handler ownership。
- 从混合命名切到统一 `snake_case`。
- 从“工具集合”思路切到“工具家族 + 角色/模式/profile 暴露策略”。

## 当前 Octopus 基线

当前实现的关键锚点：

- `crates/tools/src/tool_registry.rs`
- `crates/tools/src/builtin_exec.rs`
- `crates/tools/src/capability_runtime/provider.rs`
- `crates/tools/src/capability_runtime/state.rs`
- `crates/octopus-runtime-adapter/src/capability_planner_bridge.rs`
- `crates/octopus-runtime-adapter/src/capability_executor_bridge.rs`

当前 built-in tool 名称来自 `crates/tools/src/tool_registry.rs`：

- worker primitives:
  - `bash`
  - `read_file`
  - `write_file`
  - `edit_file`
  - `glob_search`
  - `grep_search`
  - `NotebookEdit`
  - `LSP`
- web:
  - `WebFetch`
  - `WebSearch`
- control-plane:
  - `TodoWrite`
  - `ToolSearch`
  - `SendUserMessage`
  - `Config`
  - `EnterPlanMode`
  - `ExitPlanMode`
  - `StructuredOutput`
  - `AskUserQuestion`
  - `Sleep`
  - `RemoteTrigger`
- host / test-only / misc:
  - `REPL`
  - `PowerShell`
  - `TestingPermission`

当前主要问题不是“工具数量少”，而是 ownership 和形态都还不对：

1. `mvp_tool_specs()` 同时承担定义、搜索源和部分暴露语义，built-in 没有正式 catalog。
2. `builtin_exec.rs` 仍然用裸字符串 `match` 分发，定义和执行入口是双轨。
3. 工具命名混用 `snake_case`、`PascalCase`、动词短语，缺少统一规则。
4. worker primitive、control-plane、host-specific 工具混在同一个平铺集合。
5. `SessionCapabilityState` 只有 `activated_tools`，表达不了 deferred discovery / exposure。
6. 当前 built-in pool 几乎没有 session / orchestration / automation / browser / artifact 这几个成熟 agent runtime 必备家族。

## 参考项目基准

### Claude Code

核心锚点：

- `docs/references/claude-code-sourcemap-main/restored-src/src/tools.ts:193-367`
- `docs/references/claude-code-sourcemap-main/restored-src/src/constants/tools.ts:36-112`
- `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/agentToolUtils.ts:70-224`
- `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts:158-260`
- `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts:1118-1246`

从 `getAllBaseTools()` 可以看出 Claude Code 的 built-in 工具池大致包括：

- coding primitives:
  - `BashTool`
  - `GlobTool`
  - `GrepTool`
  - `FileReadTool`
  - `FileEditTool`
  - `FileWriteTool`
  - `NotebookEditTool`
  - `LSPTool`
  - `REPLTool`
  - `PowerShellTool`
- web/context:
  - `WebFetchTool`
  - `WebSearchTool`
  - `ListMcpResourcesTool`
  - `ReadMcpResourceTool`
  - `ToolSearchTool`
- control-plane:
  - `TaskOutputTool`
  - `EnterPlanModeTool`
  - `ExitPlanModeV2Tool`
  - `AskUserQuestionTool`
  - `TodoWriteTool`
  - `BriefTool`
  - `ConfigTool`
- orchestration / session:
  - `AgentTool`
  - `TaskStopTool`
  - `SendMessageTool`
  - `TaskCreateTool`
  - `TaskGetTool`
  - `TaskUpdateTool`
  - `TaskListTool`
  - `EnterWorktreeTool`
  - `ExitWorktreeTool`
  - `TeamCreateTool`
  - `TeamDeleteTool`
- automation / runtime:
  - `SleepTool`
  - `WorkflowTool`
  - `CronCreateTool`
  - `CronDeleteTool`
  - `CronListTool`
  - `RemoteTriggerTool`
  - `MonitorTool`
- UX / artifacts:
  - `SendUserFileTool`
  - `PushNotificationTool`
  - `SubscribePRTool`
  - `SnipTool`

最值得借鉴的不是工具数量，而是三件事：

1. built-in pool 装配有单点。
2. agent / async / coordinator mode 有明确工具裁剪规则。
3. deferred tools 通过 `ToolSearch` + discovered state 才真正暴露给模型。

不建议直接照搬的部分：

- feature flag 和环境分支过多，成熟产品包袱重。
- 部分 `ant` 专用工具和 UI/平台耦合较强。
- 内部类型与 wire 结构偏 Anthropic 生态，不应成为 Octopus canonical model。

### OpenClaw

核心锚点：

- `docs/references/openclaw/openclaw-main/src/agents/pi-tools.ts:246-694`
- `docs/references/openclaw/openclaw-main/src/agents/openclaw-tools.ts:51-317`
- `docs/references/openclaw/openclaw-main/src/agents/tool-catalog.ts:13-346`
- `docs/references/openclaw/openclaw-main/src/gateway/server-methods/tools-catalog.ts:60-155`
- `docs/references/openclaw/openclaw-main/src/plugins/tools.ts:71-184`
- `docs/references/openclaw/openclaw-main/src/agents/apply-patch.ts:83-127`
- `docs/references/openclaw/openclaw-main/src/agents/pi-tools.read.ts:1-260`

OpenClaw 有两层值得区分：

1. coding runtime 实际装配的工具池。
2. 面向产品和配置的工具目录 / profile / group 体系。

从 `createOpenClawCodingTools()` 和 `createOpenClawTools()` 可见，其实际工具池至少覆盖：

- coding primitives:
  - `read`
  - `write`
  - `edit`
  - `apply_patch`
  - `exec`
  - `process`
- web:
  - `web_search`
  - `web_fetch`
- orchestration / sessions:
  - `sessions_list`
  - `sessions_history`
  - `sessions_send`
  - `sessions_yield`
  - `sessions_spawn`
  - `subagents`
  - `session_status`
  - `agents_list`
  - `update_plan`
- automation / runtime:
  - `cron`
  - `gateway`
- UI / media / product:
  - `canvas`
  - `nodes`
  - `message`
  - `tts`
  - `image`
  - `image_generate`
  - `music_generate`
  - `video_generate`
  - `pdf`

从 `tool-catalog.ts` 可见，它还把工具按 section 和 profile 做了产品级分组：

- sections:
  - `fs`
  - `runtime`
  - `web`
  - `memory`
  - `sessions`
  - `ui`
  - `messaging`
  - `automation`
  - `nodes`
  - `agents`
  - `media`
- profiles:
  - `minimal`
  - `coding`
  - `messaging`
  - `full`

最值得借鉴的部分：

1. tool catalog 和 tool profile 是正式产品概念，不是临时文案。
2. tool policy pipeline 很清晰，profile、provider、agent、group、sandbox 都可以裁剪工具面。
3. `apply_patch`、`process`、`sessions_*` 这些是非常实用的 agent runtime 工具，不应缺席。

需要避免直接复制的部分：

1. OpenClaw 的 core tool catalog 与实际 runtime pool 不是同一个单点真相，存在漂移风险。
2. `gateway`、`nodes`、`canvas`、`message`、`music_generate` 等带明显产品语义，不适合进入 Octopus built-in core。
3. 某些 provider-owned / plugin-owned 工具在 catalog 中看起来像 core，边界不够干净。

### Hermes Agent

核心锚点：

- `docs/references/hermes-agent/hermes-agent-main/model_tools.py:132-184`
- `docs/references/hermes-agent/hermes-agent-main/model_tools.py:234-260`
- `docs/references/hermes-agent/hermes-agent-main/tools/registry.py:125-258`
- `docs/references/hermes-agent/hermes-agent-main/toolsets.py:29-63`
- `docs/references/hermes-agent/hermes-agent-main/toolsets.py:66-243`
- `docs/references/hermes-agent/hermes-agent-main/website/docs/developer-guide/tools-runtime.md:19-245`
- `docs/references/hermes-agent/hermes-agent-main/tools/session_search_tool.py:1-220`
- `docs/references/hermes-agent/hermes-agent-main/tools/code_execution_tool.py:1-220`
- `docs/references/hermes-agent/hermes-agent-main/tools/clarify_tool.py:1-141`

Hermes 的 built-in runtime 更像“宽而实用”的 registry：

- web:
  - `web_search`
  - `web_extract`
- runtime / shell:
  - `terminal`
  - `process`
- file:
  - `read_file`
  - `write_file`
  - `patch`
  - `search_files`
- browser:
  - `browser_navigate`
  - `browser_snapshot`
  - `browser_click`
  - `browser_type`
  - `browser_scroll`
  - `browser_back`
  - `browser_press`
  - `browser_get_images`
  - `browser_vision`
  - `browser_console`
- planning / memory / recall:
  - `todo`
  - `memory`
  - `session_search`
  - `clarify`
- orchestration:
  - `delegate_task`
  - `send_message`
  - `mixture_of_agents`
- automation / execution:
  - `cronjob`
  - `execute_code`
- multimodal / other:
  - `vision_analyze`
  - `image_generate`
  - `text_to_speech`
  - `ha_list_entities`
  - `ha_get_state`
  - `ha_list_services`
  - `ha_call_service`
  - RL training 工具组

最值得借鉴的部分：

1. tool 自注册 + central registry 非常直接，扩展成本低。
2. `toolset` 是实用的暴露单位，比平铺 allowlist 更容易维护。
3. `session_search`、`clarify`、`execute_code`、`delegate_task` 这些工具都非常贴近真实 agent 工作流。

不建议直接复制的部分：

1. `todo` / `memory` / `session_search` / `delegate_task` 在 agent loop 中有 special-case interception，registry 不是完全闭环。
2. 工具数量持续增长后，registry 很容易变成“大而杂”的堆积点。
3. `homeassistant`、RL training 等垂域能力不应进入 Octopus built-in core。

## 对比结论

| 维度 | Claude Code | OpenClaw | Hermes Agent | 对 Octopus 的结论 |
| --- | --- | --- | --- | --- |
| built-in 装配 | 强，单点装配明确 | 中，coding pool 与 catalog 分离 | 中，registry 单点但 agent-loop 有旁路 | Octopus 必须做单点装配 |
| 角色/模式裁剪 | 很强 | 很强 | 中，偏 toolset 选择 | Octopus 必须有 role/profile policy |
| ToolSearch / deferred | 最强 | 弱 | 弱 | Octopus 应吸收 Claude 的 discovery/exposure 机制 |
| 产品目录 / profile | 中 | 最强 | 中 | Octopus 应吸收 OpenClaw 的 catalog view |
| 实用工具广度 | 高 | 高 | 很高 | Octopus 要补齐实用工具，但不能把垂域能力塞进 core |
| session / subagent | 强 | 强 | 强 | Octopus 需要正式的 orchestration tool family |
| automation | 强 | 中 | 中 | Octopus 需要 cron / monitor / trigger 能力 |
| browser / artifact | 中 | 中 | 强 | Octopus 至少需要 browser 与 image/pdf 读取能力 |
| 架构风险 | feature 分支过多 | catalog/runtime 双真相 | registry/agent-loop 双真相 | Octopus 应从一开始避免双真相 |

## Octopus 应该采用的目标分层

### 1. Built-in core 只保留通用 agent runtime 能力

built-in core 的入选标准：

1. 跨 host、跨 workspace、跨 actor 都成立。
2. 属于 agent runtime 的基础能力，而不是具体产品功能。
3. 需要参与 ToolSearch、role policy、approval、resume、audit。
4. 未来即便换 provider / host transport，也仍然成立。

### 2. plugin / MCP / provider-owned 不得伪装成 built-in

以下能力不应进入 built-in core：

- 渠道或业务产品能力：
  - `gateway`
  - `nodes`
  - `canvas`
  - `message` 这类带特定产品语义的发送工具
- 垂域集成：
  - `homeassistant`
  - RL training
- provider-owned 搜索或媒体：
  - `x_search`
  - music / video / TTS 类能力
- 可以作为 plugin / MCP prompt / resource 存在的业务能力

### 3. catalog 是 built-in registry 的派生视图，不是第二真相

Octopus 需要一个正式工具目录，但目录只能来源于 canonical capability metadata 派生，不能自己维护另一份核心清单。

推荐结构：

- `BuiltinCapabilityCatalog`
  - built-in 唯一真相
  - 含 schema、permission、category、role visibility、deferred policy、handler key
- `CapabilityExposureState`
  - discovered / activated / exposed / hidden
- `ToolCatalogView`
  - section、profile、display summary、source kind、default visibility
  - 用于 ToolSearch、UI、调试、文档

### 4. 命名规则必须一次性统一

建议 Octopus 内部 canonical tool name 全部改成 `snake_case`，并采用“家族前缀 + 动作”或“对象 + 动作”命名：

- 文件:
  - `read_file`
  - `write_file`
  - `edit_file`
  - `apply_patch`
- shell/runtime:
  - `shell_exec`
  - `process_manage`
  - `lsp_query`
- web/browser:
  - `web_search`
  - `web_fetch`
  - `browser_navigate`
  - `browser_snapshot`
- control-plane:
  - `tool_search`
  - `update_plan`
  - `ask_user`
  - `structured_output`

当前这种 `WebFetch` / `WebSearch` / `ToolSearch` / `NotebookEdit` / `SendUserMessage` 混搭命名，不应继续保留。

## 建议的 Octopus built-in 工具家族

### P0: 必须进入 built-in core

这是 Octopus 近期最应该拥有的正式 built-in core。

#### A. Worker primitives

- `shell_exec`
- `process_manage`
- `read_file`
- `write_file`
- `edit_file`
- `apply_patch`
- `glob_search`
- `grep_search`
- `lsp_query`
- `notebook_edit`
- `web_search`
- `web_fetch`

原因：

- Claude / OpenClaw / Hermes 三家都证明这些是 agent 的基础工作面。
- Octopus 当前最明显缺项是 `apply_patch` 和 `process_manage`。
- `apply_patch` 比单纯 `edit_file` 更适合多文件、结构化、安全回放的修改链路。

#### B. Control-plane built-ins

- `tool_search`
- `update_plan`
- `ask_user`
- `structured_output`
- `sleep`

原因：

- `TodoWrite` 应被 `update_plan` 替代，和 Octopus 的 plan governance 对齐。
- `AskUserQuestion` 应收敛为结构化的 `ask_user`，支持 open question + short choices。
- `tool_search` 已存在雏形，但必须升级为正式 exposure state machine。

#### C. Runtime config / runtime control

- `runtime_config_get`
- `runtime_config_validate_patch`
- `runtime_config_save_patch`

原因：

- 这比一个过于宽泛的 `Config` 更符合 Octopus 当前 runtime config 治理方向。
- 配置工具应直接映射 Octopus 自己的 scope / patch / validate 语义。

### P1: 应尽快补齐的 orchestration / session 家族

- `spawn_agent`
- `send_agent_input`
- `wait_agent`
- `close_agent`
- `session_status`
- `session_history`
- `session_search`
- `session_yield`

参考来源：

- Claude Code: `AgentTool`, `SendMessageTool`, `TaskStopTool`, task family
- OpenClaw: `sessions_*`, `subagents`, `session_status`
- Hermes: `delegate_task`, `session_search`, `send_message`

原因：

- Octopus 不是单轮工具代理，而是运行时代理系统。
- 没有正式的 session / subagent 家族，后续 team/workflow/runtime 都会变成旁路逻辑。
- `session_search` 是一个非常实用但当前 Octopus 缺失的能力，尤其适合本地优先 runtime。

### P1: 应尽快补齐的 automation 家族

- `cron_create`
- `cron_list`
- `cron_delete`
- `remote_trigger`
- `monitor`

原因：

- Claude Code 和 OpenClaw 都证明 automation 不是锦上添花，而是 runtime control-plane 的一部分。
- Octopus 已有 `Sleep` / `RemoteTrigger` 雏形，但缺少完整 scheduling / watch / monitor 工具组。

### P2: 推荐补齐的 browser / artifact 家族

- `browser_navigate`
- `browser_snapshot`
- `browser_click`
- `browser_type`
- `browser_scroll`
- `browser_back`
- `browser_press`
- `browser_console`
- `image_read`
- `pdf_read`

原因：

- Hermes 的 browser suite 非常实用，覆盖“网页可见状态”和“模型-页面协作”。
- OpenClaw 的 `image` / `pdf` 也说明 artifact 读取不应完全依赖外部工具。
- Octopus 后续如果要做桌面与浏览器混合工作流，这组工具会非常高频。

### P2: 谨慎引入的效率工具

- `execute_code`

参考来源：

- Hermes `execute_code`

建议：

- 这是高价值工具，但不是 P0。
- 它可以显著减少多轮工具往返，但实现要求高，必须有严格的 sandbox、allowed-tool subset、stdout/result contract。
- 如果引入，必须作为 `code_execution` 家族单独治理，不得与 `shell_exec` 混为一谈。

## 明确不建议进入 built-in core 的能力

以下能力建议直接作为 plugin / MCP / provider-owned：

- `gateway`
- `nodes`
- `canvas`
- `homeassistant`
- `x_search`
- `music_generate`
- `video_generate`
- `text_to_speech`
- RL training 工具组

原因很简单：

1. 这些能力不是 Octopus agent runtime 的稳定基础面。
2. 它们往往强依赖外部服务、产品上下文或特定 provider。
3. 一旦放进 built-in core，会反过来污染 catalog、policy、approval 和 persistence 设计。

## 对当前 Octopus 的具体调整建议

### 建议保留并重构

- `read_file`
- `write_file`
- `edit_file`
- `glob_search`
- `grep_search`
- `NotebookEdit`
- `LSP`
- `WebFetch`
- `WebSearch`
- `ToolSearch`
- `AskUserQuestion`
- `Sleep`
- `RemoteTrigger`

处理方式：

- 全部迁入 `BuiltinCapabilityCatalog`
- 统一命名
- 统一 family/category metadata
- 统一进入 role/profile/exposure 策略

### 建议替换或拆分

- `bash` -> `shell_exec`
- `TodoWrite` -> `update_plan`
- `Config` -> `runtime_config_get` + `runtime_config_validate_patch` + `runtime_config_save_patch`
- `AskUserQuestion` -> `ask_user`
- `REPL` / `PowerShell` -> `shell_exec` 的 host/backend 能力，而不是独立 canonical tool
- `SendUserMessage` -> 重新定义语义

其中 `SendUserMessage` 需要明确：

- 如果它表示“当前回合向用户输出最终消息”，那不应是工具。
- 如果它表示“异步会话 / 子代理 / 后台任务主动通知用户”，则应保留，但要改成明确的 orchestration / notification family。

### 建议新增

- `apply_patch`
- `process_manage`
- `update_plan`
- `spawn_agent`
- `send_agent_input`
- `wait_agent`
- `close_agent`
- `session_status`
- `session_history`
- `session_search`
- `session_yield`
- `cron_create`
- `cron_list`
- `cron_delete`
- `monitor`
- `browser_*`
- `image_read`
- `pdf_read`

### 建议移出正式 built-in 目录

- `TestingPermission`

原因：

- 这类工具可以保留在测试 harness 或 feature-gated test runtime 中。
- 它们不应出现在正式 `BuiltinCapabilityCatalog` 或 `ToolCatalogView` 里，否则会污染 profile、ToolSearch 和运行时暴露语义。

## 推荐实施顺序

### 第一阶段

先解决架构地基：

1. 建 `BuiltinCapabilityCatalog`
2. 建 `CapabilityExposureState`
3. 统一命名和 category
4. 把 control-plane built-ins 从 worker tools 中剥离

这部分已经与 `docs/plans/agent/2026-04-19-toolsearch-exposure-runtime-plan.md` 同方向，可直接作为上位输入。

### 第二阶段

补齐最缺的实用工具：

1. `apply_patch`
2. `process_manage`
3. `update_plan`
4. orchestration/session 家族
5. cron/monitor 家族

约束：

- 第二阶段不得与第一阶段并行启动；必须在 `BuiltinCapabilityCatalog`、`CapabilityExposureState`、统一命名和 control-plane 分类落地后再进入。
- 如果第一阶段尚未完成，就直接补工具家族，只会把旧平铺注册表继续做大，违背本文的 clean-cutover 前提。

### 第三阶段

再补 browser / artifact / code_execution：

1. `browser_*`
2. `image_read`
3. `pdf_read`
4. `execute_code`

约束：

- 第三阶段同样建立在前两阶段已经完成的前提上，尤其不能在 exposure state、approval、resume、audit 语义尚未稳定前引入高交互、高副作用工具家族。

## 最终建议

对 Octopus，真正值得吸收的不是某个参考项目的“工具名单”，而是三种能力：

1. 学 Claude Code 的单点装配、角色裁剪和 deferred exposure。
2. 学 OpenClaw 的目录化、profile 化、policy pipeline 化。
3. 学 Hermes 的实用工具意识，把 `apply_patch`、`session_search`、`clarify`、`execute_code`、`delegate` 这些高频工具正式产品化。

但在实现上，Octopus 应避免三种坏模式：

1. 不要复制 Claude Code 的 feature-flag 矩阵。
2. 不要复制 OpenClaw 的 catalog/runtime 双真相。
3. 不要复制 Hermes 的 registry/agent-loop 双旁路。

最适合 Octopus 当前阶段的路线是：

- built-in core 做小，但要做正。
- 实用工具先补齐 P0 / P1 家族。
- 垂域能力全部放到 plugin / MCP / provider-owned。
- 在开发期一次性切到 clean architecture，不为旧平铺实现继续叠兼容层。
