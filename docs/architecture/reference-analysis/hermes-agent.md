# Hermes Agent — Reference Architecture Analysis

本文档分析 `docs/references/hermes-agent-main` 项目的架构概念、模块边界、工作流、工具组织方式与交互模式。仅作观察，不含 Octopus 设计建议。

所有结论均引用本地文件路径作为证据。无法验证的结论用 `Unverified` 标注。

---

## 1. 项目定位与交付形态

- 由 Nous Research 维护的「多前端、单核心」Python Agent 框架，同时面向 CLI、TUI、即时通讯桥接、编辑器 (ACP)、定时任务 (cron)、批量运行、RL 训练等使用场景。([`docs/references/hermes-agent-main/README.md`](../../references/hermes-agent-main/README.md))
- 代码以一个 Python 包 + 多个子系统组成，不走微服务拆分。载荷入口集中在仓库根目录的少量「超大模块」里（`run_agent.py`、`cli.py`、`model_tools.py`、`toolsets.py`、`hermes_state.py`、`gateway/run.py`），其余能力通过子目录组织。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Project Structure；[`docs/references/hermes-agent-main/gateway/run.py`](../../references/hermes-agent-main/gateway/run.py) 11282 行、[`docs/references/hermes-agent-main/run_agent.py`](../../references/hermes-agent-main/run_agent.py) 12174 行、[`docs/references/hermes-agent-main/cli.py`](../../references/hermes-agent-main/cli.py) 11096 行)
- 单一可执行体：无论 CLI、Gateway、Cron、ACP 均复用同一 `AIAgent` 内核与同一 `ToolRegistry`，差异通过「Toolset 组合 + 平台适配器 + 会话上下文」表达。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §AIAgent / §Toolsets；[`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `get_tool_definitions`、[`docs/references/hermes-agent-main/toolsets.py`](../../references/hermes-agent-main/toolsets.py) `TOOLSETS` 字典)
- Profile 机制：通过 `HERMES_HOME` 环境变量将多实例隔离（配置、密钥、记忆、会话、技能），在任何模块导入前由 `_apply_profile_override()` 设置；所有对主目录的访问统一走 `get_hermes_home()` / `display_hermes_home()` 以保持 profile 感知。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Profiles；[`docs/references/hermes-agent-main/hermes_constants.py`](../../references/hermes-agent-main/hermes_constants.py) `get_hermes_home`)

---

## 2. 执行表面 (Execution Surfaces)

多种入口共享同一内核，边界清晰：

| 表面 | 入口 | 关键模块 | 典型职责 |
|---|---|---|---|
| 交互式 CLI | `hermes` | `cli.py` + `hermes_cli/` + `agent/display.py` | Rich + prompt_toolkit，斜杠命令、皮肤引擎、session 管理 |
| Ink TUI | `hermes --tui` | `ui-tui/` (Node/React)、`tui_gateway/server.py` | TS 渲染 + Python 后端通过 stdio JSON‑RPC 通讯 |
| Messaging Gateway | `hermes gateway start` | `gateway/run.py` + `gateway/platforms/*` | 多平台桥接（Telegram、Discord、Slack、WhatsApp、Signal、Matrix、Mattermost、Home Assistant、Email、SMS、Feishu、WeCom、DingTalk、Weixin、QQBot、BlueBubbles、Webhook、API Server 等） |
| 编辑器集成 | `hermes acp` | `acp_adapter/` (server.py/session.py/...) | Agent Client Protocol stdio 服务，供 VS Code / Zed / JetBrains 使用 |
| 定时任务 | `cron.scheduler.tick` | `cron/scheduler.py` + `cron/jobs.py` | 周期执行 agent，结果交付回消息平台或本地 |
| 批量/研究 | `batch_runner.py`、`environments/`、`tinker-atropos/` | 同上 | 批量轨迹生成、RL 训练 |
| MCP 反向暴露 | `hermes mcp serve` | `mcp_serve.py` | 把 Hermes 会话/消息作为 MCP 工具对外提供 |

证据：
- 表面清单来自 [`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Project Structure、[`docs/references/hermes-agent-main/README.md`](../../references/hermes-agent-main/README.md) Quick Reference 表格。
- ACP 入口 [`docs/references/hermes-agent-main/acp_registry/agent.json`](../../references/hermes-agent-main/acp_registry/agent.json) 指向 `hermes acp`；服务端 [`docs/references/hermes-agent-main/acp_adapter/server.py`](../../references/hermes-agent-main/acp_adapter/server.py) 实现 `acp.Agent`。
- TUI 进程模型与 stdio JSON‑RPC 合约见 [`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §TUI Architecture；Python 后端入口 [`docs/references/hermes-agent-main/tui_gateway/server.py`](../../references/hermes-agent-main/tui_gateway/server.py)。

---

## 3. 核心 Agent 运行时

### 3.1 AIAgent 作为唯一执行内核

- `AIAgent` 定义于 [`docs/references/hermes-agent-main/run_agent.py`](../../references/hermes-agent-main/run_agent.py)，构造参数约 60 项（凭证、路由、callbacks、session、预算、callback credential pool 等），但所有调用方都能以「最小子集」交互。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §AIAgent Class)
- 公共接口面向两种用法：`chat(message) -> str`（简单）与 `run_conversation(user_message, ...) -> dict`（完整，返回 `final_response + messages`）。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §AIAgent Class 签名摘要)
- 循环是**同步的**：在单线程里迭代地 `chat.completions.create` → 处理 `tool_calls` → `handle_function_call` → 把工具结果以 `tool` 角色追加，直到模型返回纯文本或达到 `max_iterations / iteration_budget`。支持「grace call」（预算耗尽后允许一次收尾调用）和主动中断 (`_interrupt_requested`)。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Agent Loop 伪码)
- 消息格式固定为 OpenAI `{role, content, tool_calls, ...}`；provider‑native 思考链以 `assistant_msg["reasoning"]` / `reasoning_content` / `reasoning_details` / `codex_reasoning_items` 持久化，以兼容 Kimi、OpenRouter、Codex 等不同的「thinking replay」要求。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) `SCHEMA_SQL` 与 v6/v7 迁移)

### 3.2 与多 Provider 的解耦

- 各 provider 以独立适配器模块封装 API 差异：`anthropic_adapter.py`、`bedrock_adapter.py`、`codex_responses_adapter.py`、`gemini_cloudcode_adapter.py`、`gemini_native_adapter.py`、`moonshot_schema.py` 等，统一转换成 OpenAI‑style 消息/工具调用。([`docs/references/hermes-agent-main/agent/anthropic_adapter.py`](../../references/hermes-agent-main/agent/anthropic_adapter.py) 头部注释；[`docs/references/hermes-agent-main/agent/`](../../references/hermes-agent-main/agent/) 列表)
- `api_mode` 参数决定走哪个接口变体（`"chat_completions"` / `"codex_responses"` / ...），并允许同一 model id 走不同路线。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §AIAgent Class 签名)
- 凭证池：`agent/credential_pool.py` 实现多凭证、多策略（`fill_first / round_robin / random / least_used`）、`429 / 402` 默认 1 小时冷却，并依赖 `hermes_cli/auth.py::PROVIDER_REGISTRY` 统一元数据。([`docs/references/hermes-agent-main/agent/credential_pool.py`](../../references/hermes-agent-main/agent/credential_pool.py) `SUPPORTED_POOL_STRATEGIES`、`EXHAUSTED_TTL_*`)

### 3.3 活动性与打断模型

- 中断以「按线程」设计：`tools/interrupt.py` 维护 `_interrupted_threads: set[int]`，`set_interrupt(active, thread_id)` / `is_interrupted()` 只关心当前线程；`_ThreadAwareEventProxy` 让旧式 `_interrupt_event.is_set()` 调用站点继续可用。这种模型是 Gateway 并发多 agent 的前提（不能让一个会话的 Ctrl+C 杀掉同进程内的其他会话）。([`docs/references/hermes-agent-main/tools/interrupt.py`](../../references/hermes-agent-main/tools/interrupt.py))
- 活动回调是线程局部的：`tools/environments/base.py::set_activity_callback` 把 gateway 发来的「心跳」接收器安装到当前线程；`_wait_for_process` 每 10s 触发一次，防止 gateway 的不活跃超时误杀长命令。([`docs/references/hermes-agent-main/tools/environments/base.py`](../../references/hermes-agent-main/tools/environments/base.py) `_activity_callback_local`、`touch_activity_if_due`)

---

## 4. 工具层 (Tool Layer)

### 4.1 ToolRegistry：单一真源

- 每个 `tools/*.py` 在 **import 时** 通过 `registry.register(name, toolset, schema, handler, check_fn, requires_env, is_async, description, emoji, max_result_size_chars)` 自我登记，形成单例 `ToolRegistry`。([`docs/references/hermes-agent-main/tools/registry.py`](../../references/hermes-agent-main/tools/registry.py) `ToolEntry`、`ToolRegistry.register`)
- 自动发现：`discover_builtin_tools()` 用 AST 扫描 `tools/` 目录，只导入「模块顶层存在 `registry.register(...)` 调用」的文件。避免维护手动 import 列表，同时避免把普通辅助模块误当成工具模块。([`docs/references/hermes-agent-main/tools/registry.py`](../../references/hermes-agent-main/tools/registry.py) `_is_registry_register_call`、`_module_registers_tools`、`discover_builtin_tools`)
- 注册表用 `threading.RLock` 保护，并对外只返回 **快照** 列表（`_snapshot_entries`）。这是 MCP 动态刷新、插件注入等运行时修改的并发前提。([`docs/references/hermes-agent-main/tools/registry.py`](../../references/hermes-agent-main/tools/registry.py) `_snapshot_state`、`deregister` 注释)
- 冲突规则：同名工具若来自不同 toolset 会被拒绝（防止插件/MCP 覆盖内建工具），但两个 `mcp-<server>` 之间的覆盖被允许（MCP 的正常刷新语义）。([`docs/references/hermes-agent-main/tools/registry.py`](../../references/hermes-agent-main/tools/registry.py) `register` 中 `both_mcp` 判断)
- 调度：`registry.dispatch(name, args, **kwargs)` 既处理同步也桥接异步（通过 `model_tools._run_async`），并将所有异常统一包成 `{"error": "..."}` JSON 返回。([`docs/references/hermes-agent-main/tools/registry.py`](../../references/hermes-agent-main/tools/registry.py) `dispatch`；[`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `_run_async`)
- 工具返回值硬约束：每个 handler 必须返回 **JSON 字符串**；`tool_error` / `tool_result` 辅助函数是仓库统一的包装。([`docs/references/hermes-agent-main/tools/registry.py`](../../references/hermes-agent-main/tools/registry.py) 底部辅助 + [`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Adding New Tools 中「All handlers MUST return a JSON string」)

### 4.2 Toolset：静态/动态组合

- `toolsets.py::TOOLSETS` 定义若干命名集合，每项含 `tools`（具体工具名）与 `includes`（嵌套 toolset 名）。这允许按用例而非按实现组织暴露面：
  - 领域集：`web`、`search`、`vision`、`image_gen`、`terminal`、`browser`、`file`、`tts`、`todo`、`memory`、`session_search`、`clarify`、`code_execution`、`delegation`、`cronjob`、`messaging`、`homeassistant`、`feishu_doc`、`feishu_drive` 等。
  - 场景集：`debugging`（组合 `web + file + terminal/process`）、`safe`（禁用终端，仅 `web + vision + image_gen`）。
  - 平台集：`hermes-cli`、`hermes-telegram`、`hermes-discord`（多出 `discord_server`）、`hermes-whatsapp`、`hermes-slack`、`hermes-signal`、`hermes-bluebubbles`、`hermes-homeassistant`、`hermes-email`、`hermes-mattermost`、`hermes-matrix`、`hermes-dingtalk`、`hermes-feishu`、`hermes-weixin`、`hermes-qqbot`、`hermes-wecom`、`hermes-wecom-callback`、`hermes-sms`、`hermes-webhook`、`hermes-api-server`、`hermes-acp`、`hermes-cron`、`hermes-gateway`（`includes` 所有平台集）。
  - 共享内核：`_HERMES_CORE_TOOLS` 常量被大多数平台集直接复用。
  - 解析：`resolve_toolset(name)` 递归展开 `includes`，自带循环检测；`"all"` / `"*"` 作为「union-of-all」伪 alias。
  - 证据：[`docs/references/hermes-agent-main/toolsets.py`](../../references/hermes-agent-main/toolsets.py) `_HERMES_CORE_TOOLS`、`TOOLSETS`、`resolve_toolset`、`get_toolset_names`。
- 动态集合：`toolsets.get_toolset(name)` 查不到静态定义时，回退到注册表（包括 plugin、MCP）提供的工具，并依 `registry.get_toolset_alias_target` 支持 MCP 服务器别名到其 `mcp-<server>` 规范名。([`docs/references/hermes-agent-main/toolsets.py`](../../references/hermes-agent-main/toolsets.py) `_get_plugin_toolset_names`、`_get_registry_toolset_aliases`；[`docs/references/hermes-agent-main/tools/registry.py`](../../references/hermes-agent-main/tools/registry.py) `register_toolset_alias`)
- 过滤管线：`model_tools.get_tool_definitions(enabled_toolsets, disabled_toolsets, quiet_mode)` 是调用方唯一暴露面。
  - 先用 toolset → 工具名集合，再用 `registry.get_definitions` 按每个 `check_fn()` 过滤（缺依赖时该工具不可见）。
  - 对 `execute_code`、`discord_server`、`browser_navigate` 做**动态 schema 后处理**：根据本次 session 实际生效的工具集，重建其 description/parameters，避免模型看到「描述里提到的工具其实不存在」而幻觉调用。([`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `get_tool_definitions` 中 `if "execute_code" in available_tool_names` / `discord_server` / `browser_navigate` 块)
  - 返回的 OpenAI 格式 tool schema 存在 `_last_resolved_tool_names` 进程全局，供 `code_execution_tool` 生成仅含「当前 session 可用工具」的 stub。([`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) 同上；[`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Known Pitfalls 中关于 `_last_resolved_tool_names` 的注记)

### 4.3 Agent 级工具（由主循环拦截）

- `model_tools.py` 定义 `_AGENT_LOOP_TOOLS = {"todo", "memory", "session_search", "delegate_task"}`：这些工具的 schema 仍在注册表里，但 `handle_function_call` 命中时会返回 sentinel 错误 —— 真正的分发由 `run_agent.py` 在主循环里拦截，因为它们需要 agent 级状态（`TodoStore` / `MemoryManager` / `SessionDB` / 子 agent 生命周期）。([`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `_AGENT_LOOP_TOOLS` 定义与拦截分支；[`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Adding New Tools 中 「Agent-level tools」段落)
- 参数类型强制：`coerce_tool_args` 依据 schema 把 `"42" → 42`、`"true" → True`、JSON 字符串 → list/dict，统一处理 LLM 输出漂移。([`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `coerce_tool_args`、`_coerce_value`)
- 兼容层：`_LEGACY_TOOLSET_MAP` 保留旧的 `web_tools / terminal_tools / ...` 命名，以供既有配置与批量脚本平滑迁移。([`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `_LEGACY_TOOLSET_MAP`)

---

## 5. 执行环境抽象 (Terminal Backends)

- `tools/environments/base.py::BaseEnvironment` 是抽象类，统一「本地 / Docker / SSH / Modal / Daytona / Singularity」六种后端的交互契约。([`docs/references/hermes-agent-main/tools/environments/base.py`](../../references/hermes-agent-main/tools/environments/base.py) `BaseEnvironment` 类文档字符串)
- 执行模型：**spawn-per-call**。每次 `execute()` 启动一个新的 `bash -c`，通过「会话快照文件」(`$TMP/hermes-snap-<sid>.sh`) 重放上一次 shell 状态（导出变量、函数、别名、shopt），并通过 stdout 中的 `__HERMES_CWD_<sid>__...` marker 或 `$TMP/hermes-cwd-<sid>.txt` 文件保留 CWD。([`docs/references/hermes-agent-main/tools/environments/base.py`](../../references/hermes-agent-main/tools/environments/base.py) `_wrap_command`、`_extract_cwd_from_output`)
- 统一的 `_wait_for_process` 处理：基于 `select()` 的非阻塞 stdout 抽取（绕过「孙子进程持有 pipe 导致永不 EOF」的 bug，`# issue #8340`）、增量 UTF‑8 解码、周期心跳、Ctrl+C/SIGTERM 下对进程组 `kill` 的保证（避免局部后端 `os.setsid` 留下 orphan）。([`docs/references/hermes-agent-main/tools/environments/base.py`](../../references/hermes-agent-main/tools/environments/base.py) `_wait_for_process` 的 `_drain`、异常处理)
- 远端后端通过 `_stdin_mode = "heredoc"` 与 `_ThreadedProcessHandle` 适配器把 SDK 的阻塞 `exec_fn() -> (output, exit_code)` 暴露成与 `subprocess.Popen` 一致的 `ProcessHandle`。([`docs/references/hermes-agent-main/tools/environments/base.py`](../../references/hermes-agent-main/tools/environments/base.py) `ProcessHandle`、`_ThreadedProcessHandle`、`_embed_stdin_heredoc`)
- 挂载式后端（Docker、Singularity、本地）无需 `FileSyncManager`；SSH/Modal/Daytona 需要前置文件同步，由子类覆写 `_before_execute()`。([`docs/references/hermes-agent-main/tools/environments/base.py`](../../references/hermes-agent-main/tools/environments/base.py) `_before_execute` 注释)
- 工厂选择通过 `TERMINAL_ENV` env var 切换，`tools/terminal_tool.py` 承担工厂与高层工具调用职责。([`docs/references/hermes-agent-main/tools/terminal_tool.py`](../../references/hermes-agent-main/tools/terminal_tool.py) 模块头文档；[`docs/references/hermes-agent-main/tools/environments/__init__.py`](../../references/hermes-agent-main/tools/environments/__init__.py))

---

## 6. 子 Agent 委派 (Delegation)

- `tools/delegate_tool.py::delegate_task` 生成子 `AIAgent` 实例，带独立 `task_id` 与隔离上下文；父 agent 只看到「调用 + 摘要」，不看到子 agent 的中间思考或工具调用。([`docs/references/hermes-agent-main/tools/delegate_tool.py`](../../references/hermes-agent-main/tools/delegate_tool.py) 模块头注释)
- 硬约束：`DELEGATE_BLOCKED_TOOLS = {"delegate_task", "clarify", "memory", "send_message", "execute_code"}` —— 禁止递归委派、禁止子 agent 向用户提问、禁止写共享 MEMORY.md、禁止跨平台副作用、鼓励子 agent 直接推理而非写脚本。([`docs/references/hermes-agent-main/tools/delegate_tool.py`](../../references/hermes-agent-main/tools/delegate_tool.py) `DELEGATE_BLOCKED_TOOLS`)
- 深度：`MAX_DEPTH = 1`（平铺）；`_get_max_spawn_depth` 允许上调至 `_MAX_SPAWN_DEPTH_CAP = 3`。子 agent 的 toolset 自动剔除「被完全 block 的」集合，避免暴露空壳。([`docs/references/hermes-agent-main/tools/delegate_tool.py`](../../references/hermes-agent-main/tools/delegate_tool.py) 常量与 `_SUBAGENT_TOOLSETS` 构造)
- 并发：`_DEFAULT_MAX_CONCURRENT_CHILDREN = 3`，通过 `ThreadPoolExecutor` 并行 `_run_single_child`；全局暂停开关 `set_spawn_paused` / `list_active_subagents` 供 TUI/Gateway 做可观测性。([`docs/references/hermes-agent-main/tools/delegate_tool.py`](../../references/hermes-agent-main/tools/delegate_tool.py) `_DEFAULT_MAX_CONCURRENT_CHILDREN`、`_register_subagent`、`interrupt_subagent`)
- 存在一个已记录的进程级别共享：`_last_resolved_tool_names` 会在子 agent 运行期间被替换/恢复，读该全局的新代码需注意「子 agent 在跑时可能短暂 stale」。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Known Pitfalls)

## 7. 代码执行 (Programmatic Tool Calling)

- `tools/code_execution_tool.py` 提供 `execute_code`：让 LLM 写 Python 脚本，通过 RPC 调用 **子集** 工具（`SANDBOX_ALLOWED_TOOLS = {web_search, web_extract, read_file, write_file, search_files, patch, terminal}`），把一连串 tool call 压缩成一个 inference turn，减少 round trip。([`docs/references/hermes-agent-main/tools/code_execution_tool.py`](../../references/hermes-agent-main/tools/code_execution_tool.py) 顶部文档与 `SANDBOX_ALLOWED_TOOLS`)
- 双传输：本地后端用 Unix Domain Socket，远端后端用文件式 RPC（请求/响应文件对拉扯），确保 container / Modal / Daytona 里也能走回调。([`docs/references/hermes-agent-main/tools/code_execution_tool.py`](../../references/hermes-agent-main/tools/code_execution_tool.py) 架构注释)
- 中间结果从不进入 LLM 上下文，只回传脚本 stdout；资源约束 `DEFAULT_TIMEOUT=300s`、`MAX_STDOUT_BYTES=50_000`。
- schema 由 `build_execute_code_schema(enabled_sandbox_tools, mode)` 在每次 `get_tool_definitions()` 里重建，保证 stub 与实际可用工具一致。([`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `get_tool_definitions` 中 `execute_code` 分支)

---

## 8. 记忆与上下文持久化

### 8.1 双层记忆架构

- `agent/memory_manager.py::MemoryManager` 明确规则：**Builtin provider 永远在位（不可移除）+ 最多一个外部 provider**。注册第二个外部 provider 会被 `add_provider` 拒绝并打 warning。([`docs/references/hermes-agent-main/agent/memory_manager.py`](../../references/hermes-agent-main/agent/memory_manager.py) `add_provider` 中 `if self._has_external` 分支)
- Provider 生命周期契约（`agent/memory_provider.py::MemoryProvider` 抽象基类）：
  - 核心：`is_available`、`initialize(session_id, **kwargs)`、`get_tool_schemas`、`handle_tool_call`、`shutdown`；
  - Turn 级：`prefetch(query, session_id)` / `queue_prefetch` / `sync_turn(user, assistant, session_id)`；
  - 会话级：`on_turn_start(turn, message, **kwargs)`、`on_session_end(messages)`、`on_pre_compress(messages) -> str`、`on_memory_write(action, target, content)`、`on_delegation(task, result, child_session_id, **kwargs)`。
  - 证据：[`docs/references/hermes-agent-main/agent/memory_provider.py`](../../references/hermes-agent-main/agent/memory_provider.py) 各抽象方法注释。
- 上下文防御：prefetch 返回的文本会被 `build_memory_context_block` 包进 `<memory-context>` + 「这是召回内容、不是用户输入」的 system note；`sanitize_context` 主动剥离上一轮可能由 provider 注入的 fence 与 system note，避免「栈上栈」污染。([`docs/references/hermes-agent-main/agent/memory_manager.py`](../../references/hermes-agent-main/agent/memory_manager.py) `_FENCE_TAG_RE`、`_INTERNAL_CONTEXT_RE`、`build_memory_context_block`)

### 8.2 Builtin 记忆

- 以 `MEMORY.md`（agent 自述）与 `USER.md`（对用户的理解）两个 Markdown 文件为主，`§` 作为条目分隔符，按字符数（不是 token）限制。([`docs/references/hermes-agent-main/tools/memory_tool.py`](../../references/hermes-agent-main/tools/memory_tool.py) 模块头文档)
- 「冻结快照」策略：系统提示只在 session 启动时加载记忆，session 内写入立刻持久化到磁盘，但 **不改系统提示** —— 用于维持 prompt cache 的稳定前缀。下一 session 才生效。([`docs/references/hermes-agent-main/tools/memory_tool.py`](../../references/hermes-agent-main/tools/memory_tool.py) 模块头；[`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Important Policies「Prompt Caching Must Not Break」)
- 记忆内容安全扫描：`_MEMORY_THREAT_PATTERNS` 列了提示词注入（`ignore previous instructions` / `you are now` / `do not tell the user` / `disregard rules` / `bypass_restrictions`）、凭据外泄（`curl ... $API_KEY`、`cat .env/credentials/.netrc/...`）、SSH 后门等模式；写入记忆前做轻量扫描。([`docs/references/hermes-agent-main/tools/memory_tool.py`](../../references/hermes-agent-main/tools/memory_tool.py) `_MEMORY_THREAT_PATTERNS`)

### 8.3 外部 Memory Providers

- 附带的插件：`plugins/memory/{honcho, mem0, supermemory, byterover, hindsight, holographic, openviking, retaindb}`。每个插件：
  - 实现 `MemoryProvider` 抽象；
  - 通过独立的发现机制（相对「通用插件」）；
  - 可选通过 `register_cli(subparser)` 注入 CLI 子命令（`plugins/memory/<name>/cli.py`），但框架**只为当前激活的**（`memory.provider` 配置）provider 暴露 CLI 命令，避免禁用的 provider 堆满 `hermes --help`。
  - 证据：[`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Memory-provider plugins；[`docs/references/hermes-agent-main/plugins/memory/`](../../references/hermes-agent-main/plugins/memory/) 目录清单。

### 8.4 Session & 消息的 SQLite 持久化

- `hermes_state.py::SessionDB`：单一 WAL SQLite（默认 `~/.hermes/state.db`），存 sessions + messages + `state_meta` 键值表。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) `SCHEMA_SQL`、`DEFAULT_DB_PATH`)
- 并发：自定义 write helper `_execute_write`，用 `BEGIN IMMEDIATE` + 20–150ms 随机抖动重试，避开 SQLite 默认忙等引起的「convoy effect」；每 50 次写后尝试 PASSIVE WAL checkpoint。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) `_WRITE_*` 常量、`_execute_write`、`_try_wal_checkpoint`)
- FTS5：`messages_fts` 虚表 + 同步触发器，为消息内容建索引；查询使用 `_sanitize_fts5_query` 把保留字、未匹配的 `"` / `(` / `)` / `+` / `{` / `}` / `^`、悬空 `AND/OR/NOT`、破折号分词、CJK 等问题全部处理掉，失败时回退到 `LIKE`。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) `_sanitize_fts5_query`、`_contains_cjk`、`search_messages` 的 CJK 回退)
- 压缩/分叉建模：`parent_session_id` 形成父子链；「压缩继续」定义为 `parent.end_reason='compression'` 且 `child.started_at >= parent.ended_at`，`get_compression_tip` 走到最新 tip；`list_sessions_rich(project_compression_tips=True)` 会把根投射成最新 tip，从而对 UI 呈现为「一段连续对话 = 一行」。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) `get_compression_tip`、`list_sessions_rich`)
- 计费/账号观测：sessions 表承载 token、cost、`billing_provider/base_url/mode`、`pricing_version`、`api_call_count` 等字段，用于 `agent/insights.py`、`agent/account_usage.py`、`/usage`、`/insights` 命令。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) v5/v8 迁移；[`docs/references/hermes-agent-main/agent/insights.py`](../../references/hermes-agent-main/agent/insights.py) 头部)
- 维护：`maybe_auto_prune_and_vacuum(retention_days, min_interval_hours, vacuum)` 在 gateway/CLI/cron 启动时幂等调用，只要离上次运行 >= `min_interval_hours` 就执行；失败日志记录但**绝不抛**。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) `maybe_auto_prune_and_vacuum`)

### 8.5 Session Search 与 Insights

- `tools/session_search_tool.py` 不直接返回原始 transcript，而是：FTS5 搜 → top‑N 唯一 session → 取 `MAX_SESSION_CHARS=100_000` 字符片段 → 由 **辅助** LLM（`agent/auxiliary_client.py`）生成逐 session 摘要。LLM 并发上限由 `auxiliary.session_search.max_concurrency` 限 1..5。([`docs/references/hermes-agent-main/tools/session_search_tool.py`](../../references/hermes-agent-main/tools/session_search_tool.py) 顶部 flow 注释、`_get_session_search_max_concurrency`)
- `agent/insights.py` 计算时间区间内的 token / cost / tool usage / 平台分布等；费率用 `agent/usage_pricing.py` 中的 `DEFAULT_PRICING` 与 provider/base_url 分支。([`docs/references/hermes-agent-main/agent/insights.py`](../../references/hermes-agent-main/agent/insights.py) 顶部、`_estimate_cost`)

### 8.6 上下文压缩

- `agent/context_compressor.py`：
  - 保头尾 token 预算、中间段用**辅助模型**摘要，但总结前先做一次「廉价预裁」把旧 tool 输出替换成 placeholder；
  - 摘要模板包含「handoff from a previous context window」「Do NOT answer questions from this summary」「resume from `## Active Task`」等防混淆语句；
  - 保留连续 `_SUMMARY_FAILURE_COOLDOWN_SECONDS = 600` 的失败冷却；
  - 摘要 token 预算按原文比例 `_SUMMARY_RATIO = 0.20`，下限 2000、上限 12000；
  - 证据：[`docs/references/hermes-agent-main/agent/context_compressor.py`](../../references/hermes-agent-main/agent/context_compressor.py) 顶部文档、常量、`SUMMARY_PREFIX`。
- 压缩触发时会通过 MemoryManager 的 `on_pre_compress(messages)` 让 provider 贡献额外文本拼接进摘要 prompt —— 方便像 Honcho 这样的外部服务把自己观察到的事实「塞回」摘要。([`docs/references/hermes-agent-main/agent/memory_manager.py`](../../references/hermes-agent-main/agent/memory_manager.py) `on_pre_compress`)
- 压缩结果在 SQLite 里形成「父 session 以 `compression` 结束 → 新 session 作为 tip」的血缘链（见 8.4）。

### 8.7 Prompt Caching 政策

- `agent/prompt_caching.py::apply_anthropic_cache_control` 实现 `system_and_3` 策略：最多 4 个 `cache_control` 断点 = 系统提示 + 最近 3 条非 system 消息。支持 `5m / 1h` TTL 与 native Anthropic SDK 两种风格。([`docs/references/hermes-agent-main/agent/prompt_caching.py`](../../references/hermes-agent-main/agent/prompt_caching.py))
- 全仓库层面的策略（硬性）：**会话中段绝不修改系统提示、toolset、memory**；只有上下文压缩才会重写历史。斜杠命令若要改这些状态（skills install/tools enable/memory reload）必须走「默认延后到下一 session；`--now` flag 显式即时失效」。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Important Policies「Prompt Caching Must Not Break」)

---

## 9. Gateway 架构与消息适配器

### 9.1 GatewayRunner 的结构

- [`docs/references/hermes-agent-main/gateway/run.py`](../../references/hermes-agent-main/gateway/run.py) 中的 `GatewayRunner` 负责：
  - 多平台适配器连接与自动重连（`_platform_reconnect_watcher`、`_failed_platforms`）；
  - 每会话 `AIAgent` 缓存（`_agent_cache: OrderedDict`，`_AGENT_CACHE_MAX_SIZE=128`，`_AGENT_CACHE_IDLE_TTL_SECS=3600`，LRU + 空闲 TTL 双淘汰）；
  - 当前活跃 agent 字典（`_running_agents` / `_running_agents_ts`）；
  - 入站消息队列（`_pending_messages`）与忙态提示（`_busy_ack_ts`）；
  - 会话级模型覆盖（`_session_model_overrides`，用于 `/model` 命令）；
  - 会话级审批（`_pending_approvals`）与「`/update` 回应」交互（`_update_prompt_pending`）；
  - 配对授权（`gateway/pairing.py::PairingStore`）；
  - 事件钩子注册（`gateway/hooks.py::HookRegistry`）；
  - 会话存储 (`gateway/session.py::SessionStore`) + 重置策略 + 交付路由 (`DeliveryRouter`)。
- 证据：[`docs/references/hermes-agent-main/gateway/run.py`](../../references/hermes-agent-main/gateway/run.py) `GatewayRunner.__init__` 初始化与注释、模块顶部 `_AGENT_CACHE_*` 常量。

### 9.2 BasePlatformAdapter：平台抽象

- [`docs/references/hermes-agent-main/gateway/platforms/base.py`](../../references/hermes-agent-main/gateway/platforms/base.py) 提供：
  - 标准 `MessageEvent`（`text`、`message_type` ∈ `TEXT/PHOTO/VIDEO/AUDIO/VOICE/DOCUMENT/STICKER/LOCATION/COMMAND`、`source: SessionSource`、`media_urls`、`reply_to_*`、`auto_skill`、`channel_prompt`、`internal` 等）、`SendResult`、`ProcessingOutcome`；
  - 抽象 `BasePlatformAdapter`：`connect / disconnect / send / edit_message / send_typing / send_*_media / get_chat_info` + 统一的重试（`_send_with_retry`，`_RETRYABLE_ERROR_PATTERNS` 不含一般 timeout，避免非幂等重复发送）、fatal error 通知 (`_set_fatal_error`)、平台锁 (`_acquire_platform_lock`，每个 profile/凭证独占)、会话级 guard (`_release_session_guard`、`_heal_stale_session_lock`)；
  - `merge_pending_message_event`：照片 burst / 相册合并、文本快速追加，保持一轮 turn 承载多条连发；
  - `resolve_channel_prompt(config_extra, channel_id, parent_id)`：支持按频道/线程覆盖 ephemeral system prompt。
- 「加一个新平台」有闭合清单：适配器 + `Platform` 枚举 + `_create_adapter` + 授权 maps + `SessionSource` 扩展 + `agent/prompt_builder.PLATFORM_HINTS` + `toolsets` 平台集 + `cron/scheduler._deliver_result` 注册 + `send_message_tool` routing + 频道目录 + 状态显示 + 设置向导 + redact + 测试。([`docs/references/hermes-agent-main/gateway/platforms/ADDING_A_PLATFORM.md`](../../references/hermes-agent-main/gateway/platforms/ADDING_A_PLATFORM.md) 16 节清单)
- 已接入平台：`telegram, discord, slack, whatsapp, signal, matrix, mattermost, homeassistant, email, sms, dingtalk, feishu, feishu_comment, wecom, wecom_callback, weixin, qqbot, bluebubbles, webhook, api_server`。（[`docs/references/hermes-agent-main/gateway/platforms/`](../../references/hermes-agent-main/gateway/platforms/) 目录）

### 9.3 消息处理管线

- `_handle_message(event)` 的固定顺序：
  1. 跳过 `internal=True` 的系统合成事件的用户鉴权；
  2. `_is_user_authorized(source)` + DM 下的 pairing 码生成（带速率限制）；
  3. `/update` 回应拦截（文件式 IPC，`.update_prompt.json` ↔ `.update_response`）；
  4. 基于 `inactivity` 的陈旧锁回收（`_raw_stale_timeout = HERMES_AGENT_TIMEOUT`）；
  5. 用 `_session_key_for_source` 生成会话键；
  6. 处理 busy 态（若同会话已有 agent 在跑）；
  7. 分发到 agent（或作为 pending 入队）。
- 证据：[`docs/references/hermes-agent-main/gateway/run.py`](../../references/hermes-agent-main/gateway/run.py) `_handle_message`（3131 行起）。
- **关键一致性约束**：`/approve`、`/deny`、`/stop`、`/new`、`/queue`、`/status` 等「必须在 agent 被阻塞时仍能到达」的命令，必须**同时**绕过 adapter 层 `_pending_messages` 和 runner 层拦截，**不能**走 `_process_message_background`。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Known Pitfalls「TWO message guards」)

### 9.4 交付与跨平台消息

- `send_message` 工具 + `gateway/delivery.py` + `gateway/channel_directory.py` 统一跨平台发送；对于 E2EE（如 Matrix）优先用「当前活跃适配器」发送，而非标准 HTTP 路径，因为独立 HTTP 无法做端到端加密。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `_deliver_result` 的 live adapter 优先分支)
- Home channel 概念：每平台一个默认目标（如 `TELEGRAM_HOME_CHANNEL`、`DISCORD_HOME_CHANNEL`...）；cron `deliver=<platform>` 会回落到该 channel。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `_HOME_TARGET_ENV_VARS`)

---

## 10. 斜杠命令中心化

- `hermes_cli/commands.py::COMMAND_REGISTRY` 是唯一数据源。每条 `CommandDef(name, description, category, aliases, args_hint, cli_only, gateway_only, gateway_config_gate)` 被下游**全部派生**出 CLI dispatch、Gateway dispatch、Gateway `/help`、Telegram `BotCommand` 菜单、Slack `/hermes` subcommand 映射、autocomplete、CLI help。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Slash Command Registry 的派生表)
- `gateway_config_gate` 允许「本来是 CLI-only」的命令在 config 键真值时在 gateway 里可用（例：`display.tool_progress_command`）。`GATEWAY_KNOWN_COMMANDS` 永远包含 config-gated 的命令，只是 help 菜单按 gate 展示。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §CommandDef fields)
- 技能型斜杠命令由 `agent/skill_commands.py` 扫描 `~/.hermes/skills/`，以 **user message**（而非 system prompt）注入，避免破坏 prompt caching。([`docs/references/hermes-agent-main/agent/skill_commands.py`](../../references/hermes-agent-main/agent/skill_commands.py) 顶部文档)

---

## 11. 插件系统

- `hermes_cli/plugins.py::PluginManager` 发现四类插件：
  1. 仓内 `plugins/<name>/`（排除 `memory/` 与 `context_engine/`，它们有独立发现器）；
  2. 用户 `~/.hermes/plugins/<name>/`；
  3. 项目 `./.hermes/plugins/<name>/`（须 `HERMES_ENABLE_PROJECT_PLUGINS` 打开）；
  4. PyPI entry points，group = `hermes_agent.plugins`。
  - 命名冲突时越靠近用户越优先。
  - 证据：[`docs/references/hermes-agent-main/hermes_cli/plugins.py`](../../references/hermes-agent-main/hermes_cli/plugins.py) 顶部文档 + `VALID_HOOKS`。
- 每个目录插件须包含 `plugin.yaml`（元数据）与 `__init__.py::register(ctx)`。
- 支持的生命周期 hook（`VALID_HOOKS`）：
  `pre_tool_call, post_tool_call, transform_terminal_output, transform_tool_result, pre_llm_call, post_llm_call, pre_api_request, post_api_request, on_session_start, on_session_end, on_session_finalize, on_session_reset, subagent_stop`。
- 通过 `PluginContext` 可：注册新工具（转派到 `ToolRegistry`）、注册 CLI 子命令（argparse 子树自动合入 `hermes <plugin> <subcmd>`）、注册 `pre_tool_call` 级 **block message**（`get_pre_tool_call_block_message` 返回非 None 即拒绝执行）。
- `transform_tool_result` 提供「结果串复写」通道，`post_tool_call` 纯观察用。([`docs/references/hermes-agent-main/model_tools.py`](../../references/hermes-agent-main/model_tools.py) `handle_function_call` 中的钩子顺序)
- 硬规则：**插件不得修改核心文件**（`run_agent.py / cli.py / gateway/run.py / hermes_cli/main.py` 等）。如果需要新能力，应扩展通用 hook/ctx 方法，而不是在核心里特判。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Plugins 最后的规则段)
- 发现时序陷阱：`discover_plugins()` 仅作为 `import model_tools` 的副作用被触发；绕过 `model_tools` 的代码路径必须显式调用（幂等）。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Plugins)
- Gateway 另一套独立的**事件钩子**：`gateway/hooks.py::HookRegistry` 通过 `~/.hermes/hooks/<name>/{HOOK.yaml, handler.py}` 注册，事件包括 `gateway:startup`、`session:start/end/reset`、`agent:start/step/end`、`command:*`（支持通配）。`emit_collect` 允许 `command:<name>` 类型的 hook 返回 allow/deny/rewrite。([`docs/references/hermes-agent-main/gateway/hooks.py`](../../references/hermes-agent-main/gateway/hooks.py))

---

## 12. 技能 (Skills) 系统

- 两个并行的技能表面：
  - `skills/` 仓内内建（按类别目录，如 `github/`、`mlops/`、`research/`...）；
  - `optional-skills/` 较重或需额外依赖的，默认**不激活**，通过 `hermes skills install official/<category>/<skill>` 安装，适配器 `tools/skills_hub.py::OptionalSkillSource`。
  - 证据：[`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Skills；[`docs/references/hermes-agent-main/skills/`](../../references/hermes-agent-main/skills/)、[`docs/references/hermes-agent-main/optional-skills/`](../../references/hermes-agent-main/optional-skills/) 目录。
- `SKILL.md` 格式（兼容 `agentskills.io` 标准 + 自有扩展）：
  - 元数据（name ≤ 64 chars、description ≤ 1024 chars）；
  - `platforms` OS gating（`macos / linux / windows`）；
  - `prerequisites.env_vars / commands`（命令检查仅 advisory）；
  - `metadata.hermes.{tags, category, config}`；`config` 里声明的字段会被 `skills.config.<key>` 纳管，setup 时提示、加载时注入。
  - 证据：[`docs/references/hermes-agent-main/tools/skills_tool.py`](../../references/hermes-agent-main/tools/skills_tool.py) 文档；[`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Skills 的 frontmatter 章节。
- 渐进式披露：`skills_list` → metadata only；`skill_view(name, path?)` → 主 SKILL.md 或附属文件；`skill_manage` → 写入/更新。([`docs/references/hermes-agent-main/tools/skills_tool.py`](../../references/hermes-agent-main/tools/skills_tool.py) 顶部 Available tools 段)
- 模板替换：`agent/skill_commands.py` 识别 `${HERMES_SKILL_DIR}`、`${HERMES_SESSION_ID}`、`` !`<shell>` `` 内联执行（输出上限 `_INLINE_SHELL_MAX_OUTPUT=4000`）。未解析 token 保留原样以便用户调试。([`docs/references/hermes-agent-main/agent/skill_commands.py`](../../references/hermes-agent-main/agent/skill_commands.py) `_SKILL_TEMPLATE_RE`、`_INLINE_SHELL_RE`)
- 与 prompt caching 互动：技能内容作为 user message 注入（非 system），所以**不会**破坏缓存前缀。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §CLI Architecture、[`docs/references/hermes-agent-main/agent/skill_commands.py`](../../references/hermes-agent-main/agent/skill_commands.py))

---

## 13. 危险命令审批与权限模型

- 唯一真源：[`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py)，覆盖「模式检测 / 会话级审批态 / CLI 交互 / Gateway 异步 / Cron 决策 / 永久白名单 / smart approval」。
- 会话键：优先 `contextvars`（gateway 多 executor 线程并发安全），其次 `session_context.get_session_env`，最后 env var 回退。`set_current_session_key` 返回 token，允许 nesting。([`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py) `_approval_session_key`、`get_current_session_key`)
- 危险模式库 `DANGEROUS_PATTERNS` 是纯正则对，描述作为「审批键」：包括递归 `rm`、`chmod 777 / o+w`、`mkfs`、`dd if=`、`sd` 设备写、`DROP TABLE`、无 `WHERE` 的 `DELETE`、`TRUNCATE`、系统配置写、`systemctl stop/restart`、`kill -9 -1`、`fork bomb`、`bash -c` 系列、语言解释器 `-e/-c` 与 heredoc 执行、`curl|sh`、对 `~/.ssh`、`~/.hermes/.env`、项目 `.env`、`config.yaml` 的重定向/`tee`、`xargs rm`、`find -exec rm`、`find -delete`、Git `reset --hard / push -f / branch -D / clean -fx`、`chmod +x && ./...` 连发、以及**保护 gateway 自身**的模式（`hermes gateway stop/restart`、`hermes update`、`pkill hermes` 及 `kill $(pgrep hermes)`）。([`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py) `DANGEROUS_PATTERNS`)
- 输入规范化：`_normalize_command_for_detection` 先通过 `strip_ansi` 去 ECMA‑48 转义、去 null byte、Unicode NFKC（化解 fullwidth Latin / 半角片假名类混淆）才做匹配。([`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py) `_normalize_command_for_detection`)
- 决策表：`check_all_command_guards(command, env_type, approval_callback)` 是唯一入口，合并 tirith 安全扫描 + 危险模式。
  - 容器型 env (`docker/singularity/modal/daytona`) 跳过审批（沙箱内认为安全）；
  - `approvals.mode = off` 或 `HERMES_YOLO_MODE` 或 `is_session_yolo_enabled()` 一切放行；
  - `mode = smart` 先调辅助 LLM 做 `APPROVE/DENY/ESCALATE` 评估（思路借鉴 OpenAI Codex Smart Approvals #13860），不确定再走人工；
  - Cron 路径按 `approvals.cron_mode = deny|approve` 决定；
  - CLI 走 `prompt_dangerous_approval`（线程 + timeout，默认 60s），选项 `once / session / always / deny`；
  - Gateway 走**队列审批**：`_gateway_queues[session_key] = [_ApprovalEntry, ...]`，每个线程独立 `threading.Event`，`/approve` 取队首，`/approve all` 一次性解，`/deny` 阻止。等待时每 ~10s 触发活动心跳，避免 `agent.gateway_timeout` 误杀。
  - 审批尺度持久化：`once` 仅当次；`session` 加入 `_session_approved[session_key]`；`always` 额外加入 `_permanent_approved` 并写回 `config.yaml` 的 `command_allowlist`；tirith 命中时禁止 `always`（强制仅 session 级）。
  - 证据：[`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py) `check_all_command_guards` 的 phase 1/2/2.5/3 分支。

---

## 14. MCP 双向桥接

- 外部 MCP 服务器作为 Hermes 工具源：[`docs/references/hermes-agent-main/tools/mcp_tool.py`](../../references/hermes-agent-main/tools/mcp_tool.py) 支持 stdio (`command + args`) 与 HTTP/StreamableHTTP (`url`) 两种传输。工具发现后以 `toolset="mcp-<server>"` 注册进同一 `ToolRegistry`，模型可按普通工具调用。文档明示的能力：自动重连 + 指数退避、环境变量过滤（stdio 子进程安全）、错误消息里剥离凭证、每服务器 `timeout / connect_timeout`、动态刷新 `notifications/tools/list_changed`、sampling（服务器向主体发回 `sampling/createMessage` 并受 `model / max_tokens_cap / timeout / max_rpm / allowed_models / max_tool_rounds / log_level` 约束）。([`docs/references/hermes-agent-main/tools/mcp_tool.py`](../../references/hermes-agent-main/tools/mcp_tool.py) 顶部配置示例、Architecture 段)
- 架构上：一个**专用后台事件循环** (`_mcp_loop`) 在 daemon 线程里跑，每个 server 是其上的一个长期 asyncio Task，把 `async with` 的生命周期绑在同一 Task（anyio 的 cancel-scope 要求），外部线程通过 `run_coroutine_threadsafe` 调度工具调用。([`docs/references/hermes-agent-main/tools/mcp_tool.py`](../../references/hermes-agent-main/tools/mcp_tool.py) Architecture 段)
- 反向暴露：`mcp_serve.py` 让 Hermes 作为 MCP 服务器向外部（Claude Code / Cursor / Codex）提供 9 个 OpenClaw 对齐工具（`conversations_list / conversation_get / messages_read / attachments_fetch / events_poll / events_wait / messages_send / permissions_list_open / permissions_respond`）+ Hermes 特有 `channels_list`。([`docs/references/hermes-agent-main/mcp_serve.py`](../../references/hermes-agent-main/mcp_serve.py) 顶部文档)

---

## 15. ACP 适配器（编辑器集成）

- [`docs/references/hermes-agent-main/acp_adapter/server.py`](../../references/hermes-agent-main/acp_adapter/server.py) 中的 `HermesACPAgent(acp.Agent)` 实现 ACP 协议方法：`initialize / authenticate / new_session / load_session / resume_session / fork_session / list_sessions / prompt / cancel / set_session_model / set_session_mode / set_config_option`。
- Session 管理独立于 gateway：`acp_adapter/session.py::SessionManager` 持久化 `SessionState`，内含 `agent: AIAgent`、`history`、`cwd`、`cancel_event`、`config_options`。([`docs/references/hermes-agent-main/acp_adapter/server.py`](../../references/hermes-agent-main/acp_adapter/server.py) `SessionState` 引用、`fork_session` / `list_sessions` 实现)
- 工具面 `hermes-acp`（定义于 `toolsets.py`）去掉 messaging / clarify / tts / image_generate，是「编辑器工作场景」专属集。
- 斜杠命令在 ACP 内本地拦截（`help / model / tools / context / reset / compact / version`），未知命令降级给 LLM。([`docs/references/hermes-agent-main/acp_adapter/server.py`](../../references/hermes-agent-main/acp_adapter/server.py) `_handle_slash_command`)
- 审批集成：每个 `_run_agent` 在 executor 线程内临时安装 `approval_cb`（通过 `conn.request_permission`）到 `terminal_tool`，并设置 `HERMES_INTERACTIVE=1` 让 `approval.py` 选「CLI-interactive」分支（而非 gateway 队列分支），执行后恢复。注释明确记录了 thread-local approval callback 必须在线程内设置这个陷阱。([`docs/references/hermes-agent-main/acp_adapter/server.py`](../../references/hermes-agent-main/acp_adapter/server.py) `_run_agent` 注释块)
- MCP 动态注入：ACP 客户端 `new/load/resume/fork` 时可附带 `mcp_servers`，由 `_register_session_mcp_servers` 通过 `tools.mcp_tool.register_mcp_servers` 注册并立刻 `get_tool_definitions(..., quiet_mode=True)` 刷新 `state.agent.tools`。([`docs/references/hermes-agent-main/acp_adapter/server.py`](../../references/hermes-agent-main/acp_adapter/server.py) `_register_session_mcp_servers`)
- 分发：`_executor = ThreadPoolExecutor(max_workers=4)`，`list_sessions` 有服务端分页上限 `_LIST_SESSIONS_PAGE_SIZE = 50` 和 cursor 推进。

---

## 16. TUI 与 tui_gateway

- 进程模型：`hermes --tui` 启动 Node (Ink) 前端，与 Python `tui_gateway` 通过 **stdio 的 newline‑delimited JSON‑RPC** 通讯。TypeScript 负责渲染（transcript、composer、prompts、activity），Python 负责会话/工具/模型调用/斜杠命令逻辑。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §TUI Architecture)
- Surface ↔ method 映射：
  - Chat streaming → `prompt.submit` → `message.delta/complete`；
  - Tool activity → `tool.start/progress/complete`；
  - Approvals / clarify / sudo / secret → 专用 request/response 对；
  - Session picker → `session.list/resume`；
  - 本地内置命令（`/help, /quit, /clear, /resume, /copy, /paste`）由 Ink 本地处理，其他都走 `slash.exec`（`_SlashWorker` 子进程）与 `command.dispatch` 回退。
  - 证据：[`docs/references/hermes-agent-main/tui_gateway/server.py`](../../references/hermes-agent-main/tui_gateway/server.py)（方法与事件目录，具体清单请以源码为准，Unverified）；章节表格 [`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §TUI Architecture。

---

## 17. Cron / 调度系统

- `cron/jobs.py` 提供 job 存储（`~/.hermes/cron/jobs.json`）与 due 计算；`cron/scheduler.py::tick()` 做一次 cycle，跨进程（gateway 内置 ticker + 独立 daemon + systemd timer）通过 `~/.hermes/cron/.tick.lock` 的 `fcntl` / `msvcrt` 文件锁保证**单跳至多一次**。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `_LOCK_FILE`、`tick`)
- 同一 tick 内多 due job 并发执行：`ThreadPoolExecutor(max_workers=_max_workers)` + `contextvars.copy_context()` 为每个 job 独立 ContextVar，避免会话/交付变量互相串线。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `tick` 底部的 executor 块)
- 工具集决议顺序（`_resolve_cron_enabled_toolsets`）：job-specific > `hermes tools` 的 `cron` 平台配置（`_DEFAULT_OFF_TOOLSETS = {moa, homeassistant, rl}` 被排除） > full default。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `_resolve_cron_enabled_toolsets`)
- Wake gate：若 job 配了 `script`，先跑该脚本（必须在 `HERMES_HOME/scripts/` 下），解析最后一行 JSON；若是 `{"wakeAgent": false}` 则直接跳过 agent 运行。脚本路径做 `relative_to(scripts_dir_resolved)` 反遍历/软链接攻击保护。`stdout / stderr` 在返回前用 `agent.redact.redact_sensitive_text` 脱敏。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `_run_job_script`、`_parse_wake_gate`)
- 静音协议：agent 返回 `"[SILENT]"` 单独作为 final response → 仅本地保存 output，不投递。其他文本按 `deliver` 字段投递到「origin 会话 / 平台 home channel / `platform:chat` 显式目标」等；多目标以逗号分隔，去重，`thread_id` 支持。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `SILENT_MARKER`、`_resolve_delivery_targets`、`_deliver_result` 中的 `wrap_response`)
- 超时模型：基于 agent 的「不活动时间」而非 wall‑clock。`HERMES_CRON_TIMEOUT` 默认 600s；依赖 `agent.get_activity_summary()` 轮询，超过阈值就 `agent.interrupt()` 并抛 `TimeoutError`。([`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) `run_job` 中 `_cron_inactivity_limit` / `_cron_future` 轮询循环)

---

## 18. 认证与配置

- 配置由**文件优先**：`~/.hermes/config.yaml`（全部设置） + `~/.hermes/.env`（**仅**密钥）。非密钥类（timeouts / thresholds / feature flags / 路径 / 显示偏好）不得进入 `.env`；如需 env var 镜像（为了 legacy 代码），在代码层从 config 桥接（例如 `gateway_timeout`、`terminal.cwd → TERMINAL_CWD`）。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Adding Configuration)
- 三条并行的 loader（关键陷阱）：`load_cli_config()`（CLI）、`load_config()`（`hermes tools` / `hermes setup` / 大多 CLI 子命令）、`gateway/run.py` + `gateway/config.py` 的直接 YAML 读取（gateway runtime）。加新 key 时要确认三条路径都能看见，否则 CLI/gateway 行为会发散。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Config loaders 表)
- `_config_version` 只有在**需要**迁移（改键/结构）时才提升；单纯加 key 由 deep-merge 兜住。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §config.yaml options)
- 密钥登记：`OPTIONAL_ENV_VARS` in `hermes_cli/config.py`，元数据含 `description / prompt / url / password / category`（`provider / tool / messaging / setting`），驱动 setup 向导与状态面板。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §.env variables)
- Providers：统一 `PROVIDER_REGISTRY`（`hermes_cli/auth.py`）+ runtime 解析器（`hermes_cli/runtime_provider.py::resolve_runtime_provider`）+ OAuth 流（`agent/google_oauth.py`、Codex / Claude Code credential 读取函数）。([`docs/references/hermes-agent-main/agent/anthropic_adapter.py`](../../references/hermes-agent-main/agent/anthropic_adapter.py) `read_claude_code_credentials`、`read_claude_managed_key`；[`docs/references/hermes-agent-main/cron/scheduler.py`](../../references/hermes-agent-main/cron/scheduler.py) 引入 `runtime_provider` 的段落)

---

## 19. 安全与隔离政策摘要

- **Prompt caching 不可破** (AGENTS.md §Important Policies)：三禁 + 斜杠命令的 `--now` flag 对齐。
- **沙箱内默认跳过 dangerous command 审批**（container/sdk 后端），本地/SSH 则走完整审批。([`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py) `check_all_command_guards` 的 `env_type in {docker, singularity, modal, daytona}` 分支)
- **Gateway 自保**：模式库显式阻止 agent 杀自己所在的 gateway / 触发 `hermes update` 自重启、以及 `pkill hermes` / `kill $(pgrep hermes)`。([`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py) 注释「Gateway lifecycle protection」附近的模式)
- **Secrets redaction**：所有 cron 脚本 stdout/stderr、`gateway/session.py` 中的 user/chat id 都有 `_hash_sender_id / _hash_chat_id` 等哈希，`agent/redact.py` 统一工具。([`docs/references/hermes-agent-main/gateway/session.py`](../../references/hermes-agent-main/gateway/session.py) `_hash_id` 系列)
- **Skill/memory content 扫描**：`_MEMORY_THREAT_PATTERNS` 检测 prompt injection 与 exfiltration。(已引于 §8.2)
- **权限硬隔离**：delegation 子 agent、`execute_code` sandbox、ACP session 都有自己的受限工具集。([`docs/references/hermes-agent-main/tools/delegate_tool.py`](../../references/hermes-agent-main/tools/delegate_tool.py)、[`docs/references/hermes-agent-main/tools/code_execution_tool.py`](../../references/hermes-agent-main/tools/code_execution_tool.py)、[`docs/references/hermes-agent-main/toolsets.py`](../../references/hermes-agent-main/toolsets.py) `hermes-acp`)
- **Tests 必须不写到 `~/.hermes/`**：`tests/conftest.py::_isolate_hermes_home` 自动夹具，profile 相关测试还需 mock `Path.home()`。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Known Pitfalls「Tests must not write」)

---

## 20. 架构取舍总览（观察）

- **集中式 singleton 工具注册表 + AST 扫描自动发现**：代价是全局可变状态与进程级共享（如 `_last_resolved_tool_names`），但消除了注册维护负担；通过 RLock + 快照读缓解并发。
- **超大文件 + 文件内职责聚合**（`run_agent.py` / `cli.py` / `gateway/run.py` 各 ~11–12k 行）：优势是跳转路径最短、文件内 grep 友好，AGENTS.md 还特别说明「file counts shift constantly — 不要把目录树当圣经，以文件系统为准」，承认结构流变。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Project Structure 的 disclaimer)
- **配置多源但 data source 单根**：config.yaml 为单源配置 + `state.db` 为单源会话/消息 + 若干 `~/.hermes/<dir>/` 为持久化文件（skills/memories/hooks/plugins/scripts/cron 等），所有路径通过 `get_hermes_home()` 居中化。
- **外部 memory provider 互斥**：作者以「工具 schema 膨胀 + 多后端冲突」为由强制「最多一个外部 provider」。是产品级决策，不是技术限制。([`docs/references/hermes-agent-main/agent/memory_manager.py`](../../references/hermes-agent-main/agent/memory_manager.py) `_has_external` 规则与注释)
- **Gateway 双 guard 的强制一致性**：运行期保护机制（必须同时绕过 adapter 级 `_pending_messages` 与 runner 级命令拦截）是一个已记录的架构约束，新增控制类命令需遵守。(§Known Pitfalls)
- **Session 压缩形成血缘链 + FTS5**：把「上下文压缩」与「长线会话检索」用同一 SQLite + `parent_session_id + end_reason + started_at` 建模，兼顾用户视图（连续对话 = 一行）与 debug 视图（`include_children=True`）。([`docs/references/hermes-agent-main/hermes_state.py`](../../references/hermes-agent-main/hermes_state.py) `get_compression_tip` 注释)
- **执行环境抽象同形**：六种后端统一走 `_wrap_command + snapshot + CWD marker` 的同一套壳逻辑，差异只在 `_run_bash`（spawn）和 `_before_execute`（文件同步需要与否）。([`docs/references/hermes-agent-main/tools/environments/base.py`](../../references/hermes-agent-main/tools/environments/base.py) 顶部类文档)
- **插件不改核心**的硬规矩 + `transform_tool_result` 这类复写 hook：保证「第三方扩展只通过公开接口影响行为」。([`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §Plugins 最后段)
- **审批设计从「CLI 阻塞输入」和「Gateway 异步队列」同构**：CLI 用 `input()` 线程 + timeout，Gateway 用 `_ApprovalEntry.event` + FIFO 队列 + 活动心跳，两者最终都落到相同的 `_session_approved / _permanent_approved` 会话态。([`docs/references/hermes-agent-main/tools/approval.py`](../../references/hermes-agent-main/tools/approval.py) `prompt_dangerous_approval` 与 `check_all_command_guards` 的 gateway 分支)

---

## 21. 尚未验证 / 需源码确认的细节（Unverified）

本次分析基于文档 + 关键文件头/索引阅读。以下结论在本次会话中未被完整源码验证，标注为 `Unverified`：

- TUI JSON‑RPC 方法/事件的完整清单：本文列出的 surface→method 对应关系来自 [`docs/references/hermes-agent-main/AGENTS.md`](../../references/hermes-agent-main/AGENTS.md) §TUI 表格；`tui_gateway/server.py` 中的方法名集合本身未逐一核对。`Unverified`.
- `run_agent.py::AIAgent` 的完整参数列表（文档只声明 ~60 项，具体实参名与默认值未逐一核对）。`Unverified`.
- `agent/prompt_builder.py::PLATFORM_HINTS` 中为每个平台提供的具体文本（文档证明该常量存在，但具体内容未展开）。([`docs/references/hermes-agent-main/gateway/platforms/ADDING_A_PLATFORM.md`](../../references/hermes-agent-main/gateway/platforms/ADDING_A_PLATFORM.md) §6) — 常量存在，内容 `Unverified`.
- `tools/skill_manager_tool.py`、`tools/skills_sync.py`、`tools/skills_hub.py` 的具体分工（本次只查看 `skills_tool.py` 头部）。`Unverified`.
- 多 provider 适配器内部的完整调用流程（只核实 `anthropic_adapter.py` 头部与公用工具函数，`bedrock_adapter.py` / `codex_responses_adapter.py` / Gemini 适配器的细节 `Unverified`）。
- `agent/context_references.py`、`agent/manual_compression_feedback.py`、`agent/shell_hooks.py`、`agent/title_generator.py` 的具体用途（文件存在，用途 `Unverified`）。
