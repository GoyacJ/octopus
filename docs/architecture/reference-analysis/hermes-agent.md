# Hermes Agent Reference Analysis

范围：只分析 `docs/references/hermes-agent-main`。证据路径均相对仓库根目录。

## 1. 项目定位

观察：Hermes Agent 是一个本地优先的 AI agent 项目。它把 CLI 对话、消息网关、定时任务、子代理、终端工具和多模型 Provider 放在同一个 agent runtime 周围。证据：`docs/references/hermes-agent-main/README.md:14-24`、`docs/references/hermes-agent-main/website/docs/developer-guide/architecture.md:11-49`。

观察：项目公开形态包括 CLI 命令、共享命令、消息入口和开发文档入口。证据：`docs/references/hermes-agent-main/README.md:53-83`、`docs/references/hermes-agent-main/README.md:91-108`。

推断：它的核心不是单一聊天 UI，而是一个可被 CLI、gateway、cron、TUI gateway 和 ACP 适配层复用的 agent execution core。依据是入口点都汇入 `AIAgent` 或共享 runtime/provider/tool 层。证据：`docs/references/hermes-agent-main/website/docs/developer-guide/architecture.md:136-171`、`docs/references/hermes-agent-main/run_agent.py:680-803`。

## 2. 技术栈

观察：主体是 Python 包，要求 Python `>=3.11`。核心依赖包含 OpenAI、Anthropic、httpx、python-dotenv、rich、pydantic、prompt_toolkit 等。证据：`docs/references/hermes-agent-main/pyproject.toml:5-37`。

观察：可选能力通过 extras 拆分，覆盖 messaging、cron、MCP、honcho、ACP、Bedrock、web、RL 等。证据：`docs/references/hermes-agent-main/pyproject.toml:39-115`。

观察：包入口脚本包括 `hermes`、`hermes-agent`、`hermes-acp`。证据：`docs/references/hermes-agent-main/pyproject.toml:117-129`。

观察：会话状态使用 SQLite，包含 FTS5、WAL、压缩 lineage 和线程安全连接。证据：`docs/references/hermes-agent-main/hermes_state.py:1-15`、`docs/references/hermes-agent-main/hermes_state.py:36-168`。

## 3. 主要入口点

观察：CLI 主入口在 `hermes_cli/main.py`，通过 argparse 组织模型、provider、工具、会话、gateway 等参数。证据：`docs/references/hermes-agent-main/hermes_cli/main.py:6623-6740`。

观察：CLI 在启动后会做 plugin/hook discovery，并默认进入 chat route。证据：`docs/references/hermes-agent-main/hermes_cli/main.py:8885-8958`。

观察：消息网关入口在 `gateway/run.py`，消息处理 pipeline 经过授权、平台上下文和 `AIAgent.run_conversation`。证据：`docs/references/hermes-agent-main/gateway/run.py:3131-3143`、`docs/references/hermes-agent-main/gateway/run.py:6578-6589`、`docs/references/hermes-agent-main/gateway/run.py:6765-6775`。

观察：TUI gateway 后台线程也创建 `AIAgent`。证据：`docs/references/hermes-agent-main/tui_gateway/server.py:2436-2464`。

观察：cron 有 CLI wrapper 和调度器入口。证据：`docs/references/hermes-agent-main/hermes_cli/cron.py:1-6`、`docs/references/hermes-agent-main/hermes_cli/cron.py:41-124`、`docs/references/hermes-agent-main/cron/scheduler.py:1-9`。

观察：ACP 入口通过 `hermes-acp` 脚本暴露，并有权限桥接模块。证据：`docs/references/hermes-agent-main/pyproject.toml:120`、`docs/references/hermes-agent-main/acp_adapter/permissions.py:1-79`。

## 4. 核心模块

观察：`run_agent.py` 是 agent runtime 中心，负责 `AIAgent` 初始化、system prompt、conversation loop、tool call 和最终响应处理。证据：`docs/references/hermes-agent-main/run_agent.py:680-803`、`docs/references/hermes-agent-main/run_agent.py:4057-4222`、`docs/references/hermes-agent-main/run_agent.py:8630-8935`、`docs/references/hermes-agent-main/run_agent.py:11093-11470`。

观察：工具系统由 `model_tools.py`、`tools/registry.py`、`toolsets.py` 和 `tools/*.py` 组成。证据：`docs/references/hermes-agent-main/AGENTS.md:64-74`、`docs/references/hermes-agent-main/model_tools.py:1-20`、`docs/references/hermes-agent-main/tools/registry.py:1-15`、`docs/references/hermes-agent-main/toolsets.py:68-193`。

观察：状态、记忆和上下文分别由 `hermes_state.py`、`tools/memory_tool.py`、`agent/context_engine.py`、`agent/context_compressor.py`、`agent/memory_manager.py` 承担。证据：`docs/references/hermes-agent-main/hermes_state.py:1-15`、`docs/references/hermes-agent-main/tools/memory_tool.py:1-24`、`docs/references/hermes-agent-main/agent/context_engine.py:1-26`、`docs/references/hermes-agent-main/agent/context_compressor.py:1-18`、`docs/references/hermes-agent-main/agent/memory_manager.py:1-27`。

观察：Provider/backend 抽象集中在 `hermes_cli/runtime_provider.py`、`hermes_cli/providers.py` 和 `agent/transports/*`。证据：`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:1`、`docs/references/hermes-agent-main/hermes_cli/providers.py:1-18`、`docs/references/hermes-agent-main/agent/transports/base.py:1-89`。

## 5. Agent runtime / agent loop

观察：`AIAgent` 构造参数面很宽，初始化时会组合 provider fallback、工具、session DB、memory、context compressor 等能力。证据：`docs/references/hermes-agent-main/run_agent.py:680-803`、`docs/references/hermes-agent-main/run_agent.py:1290-1585`。

观察：system prompt 是分层生成的，包含 SOUL、tool guidance、memory、skills、context files、日期和平台提示。证据：`docs/references/hermes-agent-main/run_agent.py:4057-4222`。

观察：`run_conversation` 的 turn 生命周期包括输入清理、task_id、history copy、todo hydration、memory nudge、system prompt cache、preflight compression、API message 构造、streaming/non-streaming 调用、工具执行和最终响应恢复。证据：`docs/references/hermes-agent-main/run_agent.py:8630-8935`、`docs/references/hermes-agent-main/run_agent.py:9180-9450`、`docs/references/hermes-agent-main/run_agent.py:11093-11470`。

观察：agent loop 自己拥有部分工具，包括 `todo`、`memory`、`session_search`、`delegate_task`。证据：`docs/references/hermes-agent-main/model_tools.py:353-357`、`docs/references/hermes-agent-main/run_agent.py:7675-7752`。

观察：官方开发文档把 agent loop 描述为负责 turn lifecycle、工具执行、budget、fallback、compression 和 persistence。证据：`docs/references/hermes-agent-main/website/docs/developer-guide/agent-loop.md:59-79`、`docs/references/hermes-agent-main/website/docs/developer-guide/agent-loop.md:126-161`、`docs/references/hermes-agent-main/website/docs/developer-guide/agent-loop.md:178-219`。

## 6. Tool system

观察：`model_tools.py` 是工具 orchestration 的公开 API，负责 builtin、MCP、plugin 工具发现，并按 toolsets 过滤工具定义。证据：`docs/references/hermes-agent-main/model_tools.py:1-20`、`docs/references/hermes-agent-main/model_tools.py:138-152`、`docs/references/hermes-agent-main/model_tools.py:202-346`。

观察：`tools/registry.py` 通过注册表管理工具 schema 和 dispatch，并拒绝非 MCP 工具 shadowing。证据：`docs/references/hermes-agent-main/tools/registry.py:56-73`、`docs/references/hermes-agent-main/tools/registry.py:176-228`、`docs/references/hermes-agent-main/tools/registry.py:258-309`。

观察：toolset 分为 Hermes core toolsets 和平台 toolsets。证据：`docs/references/hermes-agent-main/toolsets.py:29-63`、`docs/references/hermes-agent-main/toolsets.py:68-193`、`docs/references/hermes-agent-main/toolsets.py:293-425`。

观察：工具执行支持顺序和并发两种路径，并发只对被判断为安全且独立的批次启用。证据：`docs/references/hermes-agent-main/run_agent.py:260-331`、`docs/references/hermes-agent-main/run_agent.py:7633-7655`、`docs/references/hermes-agent-main/run_agent.py:7779-8080`、`docs/references/hermes-agent-main/run_agent.py:8082-8463`。

推断：Hermes 把“模型可见工具定义”“工具注册/dispatch”“运行时工具批处理策略”拆开，但 `run_agent.py` 仍直接参与工具执行决策。证据：`docs/references/hermes-agent-main/model_tools.py:202-357`、`docs/references/hermes-agent-main/tools/registry.py:258-309`、`docs/references/hermes-agent-main/run_agent.py:7633-8080`。

## 7. Memory / session / context 机制

观察：用户配置和存储默认在 `~/.hermes`。证据：`docs/references/hermes-agent-main/CONTRIBUTING.md:184-196`。

观察：session storage 是 SQLite，包含 FTS5 搜索、WAL、写入重试、session create/end/reopen 和 system prompt 存储。证据：`docs/references/hermes-agent-main/hermes_state.py:36-119`、`docs/references/hermes-agent-main/hermes_state.py:122-221`、`docs/references/hermes-agent-main/hermes_state.py:382-446`。

观察：memory tool 是文件后端，围绕 `MEMORY.md` / `USER.md`、frozen system-prompt snapshot 和 live disk writes 工作，目录在 `get_hermes_home()/memories`。证据：`docs/references/hermes-agent-main/tools/memory_tool.py:1-24`、`docs/references/hermes-agent-main/tools/memory_tool.py:49-55`、`docs/references/hermes-agent-main/tools/memory_tool.py:105-140`。

观察：memory 写入前有 threat scan。证据：`docs/references/hermes-agent-main/tools/memory_tool.py:65-102`。

观察：session search 使用 FTS5，并可调用辅助 LLM 做摘要。证据：`docs/references/hermes-agent-main/tools/session_search_tool.py:1-16`。

观察：context engine 是可插拔生命周期接口，context compressor 默认把历史压缩成 summary，并显式避免压缩摘要变成 active instruction。证据：`docs/references/hermes-agent-main/agent/context_engine.py:1-26`、`docs/references/hermes-agent-main/agent/context_engine.py:65-147`、`docs/references/hermes-agent-main/agent/context_compressor.py:1-18`、`docs/references/hermes-agent-main/agent/context_compressor.py:38-49`。

观察：MemoryManager 管理 built-in memory 和最多一个 external provider。证据：`docs/references/hermes-agent-main/agent/memory_manager.py:1-27`、`docs/references/hermes-agent-main/agent/memory_manager.py:83-119`、`docs/references/hermes-agent-main/agent/memory_provider.py:1-31`。

## 8. Skill system

观察：skill 是包含 `SKILL.md` 的目录，使用 YAML frontmatter，采用 progressive disclosure：先列 metadata，再按需 view 内容。证据：`docs/references/hermes-agent-main/tools/skills_tool.py:1-13`、`docs/references/hermes-agent-main/tools/skills_tool.py:28-46`、`docs/references/hermes-agent-main/tools/skills_tool.py:671-733`。

观察：本地 skills 默认位于 `~/.hermes/skills`，同时支持 local、external 和 plugin skill 来源。证据：`docs/references/hermes-agent-main/tools/skills_tool.py:84-89`、`docs/references/hermes-agent-main/tools/skills_tool.py:546-620`、`docs/references/hermes-agent-main/tools/skills_tool.py:828-882`、`docs/references/hermes-agent-main/tools/skills_tool.py:899-964`。

观察：skill 系统包含 disabled/platform 过滤、untrusted path 和 prompt injection 警告、supporting file containment 检查、链接资源扫描、环境变量与凭证 readiness 检查。证据：`docs/references/hermes-agent-main/tools/skills_tool.py:978-1035`、`docs/references/hermes-agent-main/tools/skills_tool.py:1037-1208`、`docs/references/hermes-agent-main/tools/skills_tool.py:1217-1337`。

观察：agent 可以通过 skill manager 创建、编辑、删除 skill，并有可选安全扫描、目录和大小限制。证据：`docs/references/hermes-agent-main/tools/skill_manager_tool.py:1-33`、`docs/references/hermes-agent-main/tools/skill_manager_tool.py:47-127`。

观察：CLI/gateway 共享 slash command helper，可做 template variable substitution、inline shell expansion、skill payload 加载和 skill-declared config 注入。证据：`docs/references/hermes-agent-main/agent/skill_commands.py:1-6`、`docs/references/hermes-agent-main/agent/skill_commands.py:53-128`、`docs/references/hermes-agent-main/agent/skill_commands.py:152-232`。

观察：skills 会被注入 system prompt，并有 skill nudge 机制。证据：`docs/references/hermes-agent-main/run_agent.py:1537-1543`、`docs/references/hermes-agent-main/run_agent.py:4160-4176`。

## 9. Scheduler / automation

观察：cron jobs 存在 `~/.hermes/cron/jobs.json`，输出存在 `~/.hermes/cron/output/{job_id}/{timestamp}.md`。证据：`docs/references/hermes-agent-main/cron/jobs.py:1-6`、`docs/references/hermes-agent-main/cron/jobs.py:35-44`。

观察：job 支持 once、interval、cron expression、ISO timestamp，并通过 secure dir/file permissions、atomic save、fsync、chmod 管理本地文件。证据：`docs/references/hermes-agent-main/cron/jobs.py:73-96`、`docs/references/hermes-agent-main/cron/jobs.py:123-210`、`docs/references/hermes-agent-main/cron/jobs.py:326-365`。

观察：job 字段包含 prompt、schedule、repeat、delivery、origin、skills、model/provider/base_url、script、enabled_toolsets。证据：`docs/references/hermes-agent-main/cron/jobs.py:374-480`。

观察：cron 工具支持 create/list/update/pause/resume/remove/run，schema 明确要求 fresh session、自包含 prompt、禁止 recursive cron。证据：`docs/references/hermes-agent-main/tools/cronjob_tools.py:223-390`、`docs/references/hermes-agent-main/tools/cronjob_tools.py:394-476`。

观察：scheduler 每 60 秒 tick，并用 file lock 防止 overlap；执行 job 时会创建 SessionDB，实例化 `AIAgent`，禁用 cronjob/messaging/clarify，跳过 context/memory，使用 cron 平台上下文。证据：`docs/references/hermes-agent-main/cron/scheduler.py:1-9`、`docs/references/hermes-agent-main/cron/scheduler.py:733-928`、`docs/references/hermes-agent-main/cron/scheduler.py:1075-1208`。

观察：pre-run script 必须位于 `~/.hermes/scripts`，执行有 timeout 和 redaction；`wakeAgent:false` 可跳过唤醒 agent。证据：`docs/references/hermes-agent-main/cron/scheduler.py:524-629`。

推断：recurring job 更偏 at-most-once 行为，因为 recurring jobs 在执行前推进 next_run，并有 catch-up grace / stale fast-forward 逻辑。证据：`docs/references/hermes-agent-main/cron/jobs.py:650-755`。

## 10. Subagent / parallelism

观察：subagent 由 `delegate_task` 工具实现，child `AIAgent` 隔离 context、toolsets 和 terminal，parent 只接收 summary。证据：`docs/references/hermes-agent-main/tools/delegate_tool.py:1-17`。

观察：child 默认禁止 `delegate_task`、`clarify`、`memory`、`send_message`、`execute_code` 等工具，并限制并发和 spawn depth。证据：`docs/references/hermes-agent-main/tools/delegate_tool.py:39-48`、`docs/references/hermes-agent-main/tools/delegate_tool.py:69-75`、`docs/references/hermes-agent-main/tools/delegate_tool.py:324-377`。

观察：delegate 系统有 pause flag、active registry、interrupt/list active、heartbeat、timeout、task_id 和 cleanup。证据：`docs/references/hermes-agent-main/tools/delegate_tool.py:86-159`、`docs/references/hermes-agent-main/tools/delegate_tool.py:1019-1499`。

观察：child agent 构建时会继承或覆盖 provider，只取 toolsets 交集，并跳过 memory/context。证据：`docs/references/hermes-agent-main/tools/delegate_tool.py:763-1016`。

观察：单任务 direct 执行，batch 使用 `ThreadPoolExecutor` 并行。证据：`docs/references/hermes-agent-main/tools/delegate_tool.py:1618-1787`。

观察：除 subagent 外，agent loop 内部也能并行执行安全工具批次，batch runner 还支持多进程批量 prompt 运行、checkpoint、trajectory 和 stats。证据：`docs/references/hermes-agent-main/run_agent.py:290-331`、`docs/references/hermes-agent-main/run_agent.py:7779-8080`、`docs/references/hermes-agent-main/batch_runner.py:1-10`、`docs/references/hermes-agent-main/batch_runner.py:233-360`。

## 11. Model provider / backend abstraction

观察：runtime provider resolution 是 CLI、gateway、cron 和 helpers 共享层。证据：`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:1`。

观察：base URL 可自动识别 api_mode；有效 api_mode 包括 chat completions、Codex responses、Anthropic messages、Bedrock converse。证据：`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:39-63`、`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:144-153`。

观察：Provider resolution 支持 credential pool、custom provider、OpenRouter/custom runtime、Anthropic、Codex、Nous、Qwen、Gemini、Copilot、Bedrock 等路径。证据：`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:156-234`、`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:237-253`、`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:287-393`、`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:448-920`。

观察：provider identity 的来源在 `hermes_cli/providers.py`，包含 Hermes overlays、alias、transport 到 api_mode 的 mapping、models.dev merge。证据：`docs/references/hermes-agent-main/hermes_cli/providers.py:1-18`、`docs/references/hermes-agent-main/hermes_cli/providers.py:34-166`、`docs/references/hermes-agent-main/hermes_cli/providers.py:172-322`、`docs/references/hermes-agent-main/hermes_cli/providers.py:337-404`。

观察：transport interface 明确由 transport 负责 api_mode data path，由 `AIAgent` 负责 lifecycle/retry；实现通过 registry discovery 加载 anthropic、codex、chat、bedrock。证据：`docs/references/hermes-agent-main/agent/transports/base.py:1-89`、`docs/references/hermes-agent-main/agent/transports/__init__.py:14-51`。

观察：辅助模型客户端有独立 router、fallback 和默认值。证据：`docs/references/hermes-agent-main/agent/auxiliary_client.py:1-35`、`docs/references/hermes-agent-main/agent/auxiliary_client.py:58-96`、`docs/references/hermes-agent-main/agent/auxiliary_client.py:132-147`。

## 12. Security / permission / allowlist

观察：Hermes 的安全模型假设单租户，本地 backend 默认，危险操作经 approval system。证据：`docs/references/hermes-agent-main/SECURITY.md:18-31`。

观察：安全文档覆盖 redaction、MCP env filtering、execute_code env stripping、subagent depth limit、skip_memory、container backend、`.env` 权限、非公开 gateway、skills guard、OSV check 等。证据：`docs/references/hermes-agent-main/SECURITY.md:33-77`。

观察：`tools/approval.py` 是危险命令检测、approval、session state、smart approval 和 permanent allowlist 的单一来源。证据：`docs/references/hermes-agent-main/tools/approval.py:1-9`、`docs/references/hermes-agent-main/tools/approval.py:80-205`、`docs/references/hermes-agent-main/tools/approval.py:353-490`、`docs/references/hermes-agent-main/tools/approval.py:620-764`。

观察：cron 默认拒绝需要人工批准的危险操作；gateway approval queue 有 timeout 和 heartbeat。证据：`docs/references/hermes-agent-main/tools/approval.py:620-688`、`docs/references/hermes-agent-main/tools/approval.py:722-764`、`docs/references/hermes-agent-main/tools/approval.py:841-950`。

观察：terminal tool 把危险操作判断委托给 `tools.approval`，并对 workdir 字符做 allowlist，`force` 参数只给内部使用。证据：`docs/references/hermes-agent-main/tools/terminal_tool.py:197-235`、`docs/references/hermes-agent-main/tools/terminal_tool.py:1391-1431`、`docs/references/hermes-agent-main/tools/terminal_tool.py:1585-1613`。

观察：路径安全有 resolve + containment helper 和 traversal component check。证据：`docs/references/hermes-agent-main/tools/path_security.py:1-43`。

观察：gateway 授权顺序包含平台 allow-all、pairing approved、平台/全局 allowlist、默认拒绝；未授权 DM 可触发 pairing code，pairing 文件用 chmod 0600 原子写入。证据：`docs/references/hermes-agent-main/gateway/run.py:2930-3190`、`docs/references/hermes-agent-main/gateway/pairing.py:1-18`、`docs/references/hermes-agent-main/gateway/pairing.py:33-72`、`docs/references/hermes-agent-main/gateway/pairing.py:150-218`。

观察：API server 配置 API key 时使用 Bearer token；未配置时只允许 local-only no-key。证据：`docs/references/hermes-agent-main/gateway/platforms/api_server.py:660-680`。

观察：shell hooks 使用 `shell=False`，首次使用需要写入 `~/.hermes/shell-hooks-allowlist.json` 的 consent。证据：`docs/references/hermes-agent-main/agent/shell_hooks.py:9-23`、`docs/references/hermes-agent-main/agent/shell_hooks.py:588-683`。

## 13. 可迁移到 Octopus 的设计优点

建议：保留“工具定义、工具注册、toolset 选择、运行时调度”分层思路。这个分层让模型可见工具、工具实现和平台启用范围可以分开演进。证据：`docs/references/hermes-agent-main/model_tools.py:202-357`、`docs/references/hermes-agent-main/tools/registry.py:176-309`、`docs/references/hermes-agent-main/toolsets.py:68-425`。

建议：保留 Provider resolution 与 transport interface 的分离。Provider 身份、凭证、api_mode、transport data path 和 agent lifecycle/retry 各有边界。证据：`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:144-253`、`docs/references/hermes-agent-main/hermes_cli/providers.py:172-322`、`docs/references/hermes-agent-main/agent/transports/base.py:1-89`。

建议：保留 skill progressive disclosure。`skills_list` 只暴露 metadata，`skill_view` 才载入内容，能降低 prompt 面积，也便于做路径和资源检查。证据：`docs/references/hermes-agent-main/tools/skills_tool.py:1-13`、`docs/references/hermes-agent-main/tools/skills_tool.py:671-733`、`docs/references/hermes-agent-main/tools/skills_tool.py:978-1208`。

建议：保留集中 approval 和 allowlist 思路。危险命令检测、CLI approval、gateway approval、session/permanent allowlist 在一个模块中归口，减少分叉。证据：`docs/references/hermes-agent-main/tools/approval.py:1-9`、`docs/references/hermes-agent-main/tools/approval.py:353-490`、`docs/references/hermes-agent-main/tools/approval.py:620-950`。

建议：保留 bounded subagent 模型。深度、并发、禁用工具、summary-only 返回和 child isolation 都能压住代理扩散风险。证据：`docs/references/hermes-agent-main/tools/delegate_tool.py:1-17`、`docs/references/hermes-agent-main/tools/delegate_tool.py:39-75`、`docs/references/hermes-agent-main/tools/delegate_tool.py:763-1016`。

建议：保留 fresh-session scheduler 思路。cron job 明确自包含 prompt，并在运行时禁用递归 cron、messaging、clarify，减少自动化任务的隐式上下文依赖。证据：`docs/references/hermes-agent-main/tools/cronjob_tools.py:394-476`、`docs/references/hermes-agent-main/cron/scheduler.py:733-928`。

## 14. 不建议迁移的设计

建议：不照搬 `run_agent.py` 的单体中心形态。它同时覆盖构造、prompt、压缩、provider fallback、API call、tool execution 和 final response，后续修改容易扩大影响面。证据：`docs/references/hermes-agent-main/run_agent.py:680-803`、`docs/references/hermes-agent-main/run_agent.py:1290-1585`、`docs/references/hermes-agent-main/run_agent.py:4057-4222`、`docs/references/hermes-agent-main/run_agent.py:8630-9450`、`docs/references/hermes-agent-main/run_agent.py:11093-11470`。

建议：不把 `~/.hermes` 风格的用户 home 目录作为唯一状态中心照搬。Hermes 的 memory、cron、skills、shell hook allowlist 都落在用户 home 下，这适合单用户本地工具，但会增加多工作区状态归属问题。证据：`docs/references/hermes-agent-main/CONTRIBUTING.md:184-196`、`docs/references/hermes-agent-main/tools/memory_tool.py:49-55`、`docs/references/hermes-agent-main/cron/jobs.py:1-6`、`docs/references/hermes-agent-main/tools/skills_tool.py:84-89`、`docs/references/hermes-agent-main/agent/shell_hooks.py:588-683`。

建议：不把 JSON 文件 cron store 作为复杂调度的唯一事实源。Hermes 有 secure atomic save 和 file lock，但 job 查询、审计和重放能力从代码看主要依赖 jobs JSON 与输出文件。证据：`docs/references/hermes-agent-main/cron/jobs.py:35-44`、`docs/references/hermes-agent-main/cron/jobs.py:326-365`、`docs/references/hermes-agent-main/cron/scheduler.py:1075-1208`。

建议：不照搬启发式工具并行判断作为强一致安全边界。Hermes 会识别破坏性命令并只并发安全批次，但这是 runtime heuristic，不等同于持久化事务或跨进程互斥。证据：`docs/references/hermes-agent-main/run_agent.py:260-331`、`docs/references/hermes-agent-main/run_agent.py:7779-8080`。

建议：不默认开放 agent-managed skill 写入能力。Hermes 有安全扫描、大小和目录限制，但让 agent 创建/编辑技能会扩大持久化指令面。证据：`docs/references/hermes-agent-main/tools/skill_manager_tool.py:1-33`、`docs/references/hermes-agent-main/tools/skill_manager_tool.py:47-127`。

建议：不让 Provider resolution 长期集中成大优先级链。Hermes 支持很多 provider 和 custom runtime，但 resolution priority、alias、credential pool 和 transport mapping 分散在多个长函数里，后续可维护性需要控制。证据：`docs/references/hermes-agent-main/hermes_cli/runtime_provider.py:237-920`、`docs/references/hermes-agent-main/hermes_cli/providers.py:192-404`。

## 15. Unverified / open questions

- Unverified：没有确认 scheduler 在多主机或分布式环境中的锁语义。已确认的是本地 file lock 防 overlap。证据：`docs/references/hermes-agent-main/cron/scheduler.py:1-9`、`docs/references/hermes-agent-main/cron/scheduler.py:1075-1208`。
- Unverified：没有确认 recurring job 的 exactly-once 语义。代码显示执行前推进 next_run，并有 catch-up grace / stale fast-forward；这更像 at-most-once，但需要运行级测试证明。证据：`docs/references/hermes-agent-main/cron/jobs.py:650-755`。
- Unverified：没有完整确认 external memory provider 在真实 provider 中的权限边界。已确认 MemoryManager 只允许一个 external provider，并定义 lifecycle hooks。证据：`docs/references/hermes-agent-main/agent/memory_manager.py:83-119`、`docs/references/hermes-agent-main/agent/memory_provider.py:1-31`。
- Unverified：没有完整确认 MCP/plugin 工具的权限隔离边界。已确认工具发现和 registry 支持 MCP/plugin，但未追完外部工具运行沙箱。证据：`docs/references/hermes-agent-main/model_tools.py:138-152`、`docs/references/hermes-agent-main/tools/registry.py:176-228`。
- Unverified：没有完整确认 ACP adapter 的端到端启动路径。已确认脚本入口和 permission bridge。证据：`docs/references/hermes-agent-main/pyproject.toml:120`、`docs/references/hermes-agent-main/acp_adapter/permissions.py:1-79`。
- Unverified：没有确认 gateway 在公网部署时除 Bearer token、pairing、allowlist 之外的生产防护。安全文档明确不建议公开 gateway。证据：`docs/references/hermes-agent-main/SECURITY.md:62-77`、`docs/references/hermes-agent-main/gateway/platforms/api_server.py:660-680`。

## 16. 关键文件路径索引

- `docs/references/hermes-agent-main/README.md`：项目定位、命令、文档地图。
- `docs/references/hermes-agent-main/pyproject.toml`：Python 版本、依赖、extras、脚本入口。
- `docs/references/hermes-agent-main/AGENTS.md`：仓库内模块地图和工具依赖链。
- `docs/references/hermes-agent-main/website/docs/developer-guide/architecture.md`：架构概览、入口点、数据流、子系统。
- `docs/references/hermes-agent-main/website/docs/developer-guide/agent-loop.md`：agent loop lifecycle、工具执行、budget、fallback、compression、persistence。
- `docs/references/hermes-agent-main/run_agent.py`：`AIAgent`、system prompt、conversation loop、tool execution、final response。
- `docs/references/hermes-agent-main/model_tools.py`：工具发现、工具定义、agent-level tools。
- `docs/references/hermes-agent-main/tools/registry.py`：工具注册、schema、dispatch。
- `docs/references/hermes-agent-main/toolsets.py`：core 和 platform toolsets。
- `docs/references/hermes-agent-main/hermes_state.py`：SQLite session store、FTS、WAL、session lifecycle。
- `docs/references/hermes-agent-main/tools/memory_tool.py`：file-backed memory。
- `docs/references/hermes-agent-main/tools/session_search_tool.py`：session FTS 搜索。
- `docs/references/hermes-agent-main/agent/context_engine.py`：可插拔 context engine。
- `docs/references/hermes-agent-main/agent/context_compressor.py`：默认 context compression。
- `docs/references/hermes-agent-main/agent/memory_manager.py`：memory provider 管理。
- `docs/references/hermes-agent-main/tools/skills_tool.py`：skill list/view、扫描、过滤、安全检查。
- `docs/references/hermes-agent-main/tools/skill_manager_tool.py`：agent-managed skill 创建和修改。
- `docs/references/hermes-agent-main/agent/skill_commands.py`：CLI/gateway skill slash command。
- `docs/references/hermes-agent-main/cron/jobs.py`：job model、schedule parsing、jobs file、输出路径。
- `docs/references/hermes-agent-main/cron/scheduler.py`：tick、锁、job execution、agent instantiation。
- `docs/references/hermes-agent-main/tools/cronjob_tools.py`：cron 工具接口。
- `docs/references/hermes-agent-main/tools/delegate_tool.py`：subagent delegation。
- `docs/references/hermes-agent-main/batch_runner.py`：批量运行和多进程。
- `docs/references/hermes-agent-main/hermes_cli/runtime_provider.py`：provider runtime resolution。
- `docs/references/hermes-agent-main/hermes_cli/providers.py`：provider identity、aliases、transport mapping。
- `docs/references/hermes-agent-main/agent/transports/base.py`：transport interface。
- `docs/references/hermes-agent-main/agent/transports/__init__.py`：transport registry discovery。
- `docs/references/hermes-agent-main/agent/auxiliary_client.py`：辅助模型客户端。
- `docs/references/hermes-agent-main/SECURITY.md`：安全模型和 hardening。
- `docs/references/hermes-agent-main/tools/approval.py`：approval、dangerous command、allowlist。
- `docs/references/hermes-agent-main/tools/terminal_tool.py`：terminal backend 与 pre-exec checks。
- `docs/references/hermes-agent-main/tools/path_security.py`：路径 containment helper。
- `docs/references/hermes-agent-main/gateway/run.py`：gateway authorization 和 message pipeline。
- `docs/references/hermes-agent-main/gateway/pairing.py`：pairing code 和授权文件。
- `docs/references/hermes-agent-main/gateway/platforms/api_server.py`：API server auth。
- `docs/references/hermes-agent-main/acp_adapter/permissions.py`：ACP permission bridge。
- `docs/references/hermes-agent-main/agent/shell_hooks.py`：shell hooks allowlist。
