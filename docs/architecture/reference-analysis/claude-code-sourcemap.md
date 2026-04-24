# Claude Code Sourcemap Reference Analysis

## 0. 范围说明

本文只分析 `docs/references/claude-code-sourcemap-main`。

该目录是 sourcemap / reference material。它的 `README.md` 明确说明内容来自公开 npm 包和 source map 还原，仅供研究，不代表官方原始内部开发仓库结构。所以下文只抽象架构概念、模块边界、工作流、工具组织方式和交互模式，不照搬目录结构。证据来源限定为 `docs/references/claude-code-sourcemap-main/README.md`、`docs/references/claude-code-sourcemap-main/package/**` 和 `docs/references/claude-code-sourcemap-main/restored-src/**`。

## 1. 项目定位与参考价值

- 这是一个终端 coding agent 的还原参考。npm 包描述称它能理解代码库、编辑文件、运行终端命令并处理 workflows，包名和 bin 入口见 `docs/references/claude-code-sourcemap-main/package/package.json`。
- 参考价值不在目录形状，而在 agent 产品的边界划分：CLI 入口、命令系统、query loop、工具协议、权限管线、session 持久化、subagent、plugin / skill、remote bridge。证据分布在 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`。
- 该参考材料版本为 `2.1.88`，并声明还原文件数量和 TypeScript / TSX 文件数量。版本和 sourcemap 性质见 `docs/references/claude-code-sourcemap-main/README.md` 和 `docs/references/claude-code-sourcemap-main/package/package.json`。

## 2. 技术栈

- 运行形态是 Node CLI。`package.json` 声明 `type: module`、Node `>=18.0.0`、bin `claude -> cli.js`，见 `docs/references/claude-code-sourcemap-main/package/package.json`。
- CLI 层使用 Commander。主入口创建 `CommanderCommand`，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`。
- UI / REPL 侧存在 TSX 组件和 React 风格渲染线索，例如工具 UI、permission UI、task UI 文件使用 `.tsx`，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/BashTool.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/hooks/useCanUseTool.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/tasks/LocalAgentTask/LocalAgentTask.tsx`。
- 数据校验大量使用 schema。工具输入、agent frontmatter、permission 输出等使用 Zod 或 JSON schema 组织，见 `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/loadAgentsDir.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/PermissionPromptToolResultSchema.ts`。
- Git、WebSocket、HTTP、MCP、shell sandbox、tree-sitter Bash analysis 都是核心依赖方向。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/worktree.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/remote/SessionsWebSocket.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/services/mcp/client.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/bashPermissions.ts`。

## 3. 主要入口点

- 包入口是 `claude -> cli.js`，见 `docs/references/claude-code-sourcemap-main/package/package.json`。
- bootstrap 入口先处理快速路径，再动态加载完整 CLI。`--version`、daemon、environment runner 等路径在完整 CLI 之前分流，见 `docs/references/claude-code-sourcemap-main/restored-src/src/entrypoints/cli.tsx`。
- 完整 CLI 入口在 `main.tsx`。它注册主命令、全局 option、subcommand、preAction 初始化、policy / settings / plugin 注入等启动流程，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`。
- 非交互 print 模式是显式工作流。`-p/--print` 在主 CLI 选项中定义，print 模式还会跳过部分交互 subcommand 注册，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/cli/print.ts`。

## 4. CLI / command system

- 主 CLI 以 `claude` 命令为中心，默认交互，`-p/--print` 进入非交互；它还暴露 `--allowed-tools`、`--disallowed-tools`、`--mcp-config`、`--permission-mode`、`--continue`、`--resume`、`--model`、`--agents`、`--plugin-dir`、`--worktree`、`--tmux` 等控制面，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`。
- subcommand 覆盖 MCP、server、ssh、open、auth、plugin、agents、doctor、update、install、task、completion 等产品面。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`。
- slash command 被抽象成三类：`prompt`、`local`、`local-jsx`。`prompt` command 可声明 allowed tools、hooks、skillRoot、context、agent、paths 和 prompt 生成函数，见 `docs/references/claude-code-sourcemap-main/restored-src/src/types/command.ts`。
- command registry 是动态聚合，不是静态列表。它组合内置命令、workflow、skills、plugins、plugin skills，并在调用时重新检查 availability / isEnabled，见 `docs/references/claude-code-sourcemap-main/restored-src/src/commands.ts`。
- remote / bridge command 有安全白名单。`REMOTE_SAFE_COMMANDS`、`BRIDGE_SAFE_COMMANDS` 和 `isBridgeSafeCommand()` 限制远端可执行 command 类型，见 `docs/references/claude-code-sourcemap-main/restored-src/src/commands.ts`。

## 5. Coding-agent workflow

- `QueryEngine` 是 conversation lifecycle 和 session state 的上层对象。一个 conversation 对应一个 `QueryEngine`，多次 `submitMessage()` 共享状态，见 `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts`。
- `submitMessage()` 在进入 query loop 前处理 cwd、permission wrapper、system prompt、user context、system context、slash command、attachments，并先记录 transcript 以支持 resume，见 `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts`。
- `query()` / `queryLoop()` 是模型-工具循环核心。它维护 messages、toolUseContext、auto compact tracking、turn count、stop hook 状态和 pending tool summary，见 `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`。
- 每轮 query 会处理 context snip / microcompact / context collapse / autocompact，再调用模型流式接口。assistant message 中的 tool_use block 会进入工具执行，再把 tool_result 回灌下一轮，见 `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`。
- 本地 slash command 可短路模型调用。若 command 处理结果标记不需要继续 query，就直接返回本地结果，见 `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/commands.ts`。
- query loop 同时处理 fallback model、prompt-too-long、max output recovery、stop hooks、token budget continuation、abort 和 max turns。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`。

## 6. Tool execution 机制

- 工具注册由 `getAllBaseTools()` 汇总。基础工具包括 Agent / TaskOutput、Bash、Glob / Grep、Read / Edit / Write、NotebookEdit、WebFetch / WebSearch、TodoWrite、AskUserQuestion、SkillTool、Team、LSP、MCP resource、ToolSearch 等，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools.ts`。
- `Tool` 接口把执行、schema、并发安全、只读/破坏性标记、权限检查、输入校验、结果映射、UI render、MCP 元信息放在同一个协议内，见 `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`。
- `ToolUseContext` 持有 messages、read file state、MCP clients、agent id / type、permission state、prompt callbacks、file history、attribution 和 rendered system prompt。工具执行不是裸函数调用，而是带 session context 的调用，见 `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts`。
- 批量工具执行区分并发安全工具和串行工具。`runTools()` 用 `partitionToolCalls()` 分批，默认最大并发受 `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` 控制，见 `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolOrchestration.ts`。
- streaming 执行器支持工具边流入边排队。并发安全工具可并行，非并发工具独占；Bash error 会取消并行兄弟进程，见 `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/StreamingToolExecutor.ts`。
- 单个工具调用的管线是 schema parse、tool validate、Bash speculative classifier、PreToolUse hooks、permission decision、tool.call、结果映射、telemetry、PostToolUse hooks。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts`。
- hooks 能 block、追加 context、阻止 continuation、更新 MCP output、返回 permission decision。但 hook allow 不绕过 settings deny / ask rules，见 `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolHooks.ts`。

## 7. Context / session / workspace handling

- system context 会包含 git status snapshot。remote CCR 或禁用 git instructions 时会跳过 git status，见 `docs/references/claude-code-sourcemap-main/restored-src/src/context.ts`。
- user context 通过 CLAUDE.md / memory 文件加载。`--bare` 会跳过自动发现，但仍保留 explicit add-dir，见 `docs/references/claude-code-sourcemap-main/restored-src/src/context.ts`。
- memory 分 Managed、User、Project、Local 多层加载，支持 cwd 向上查找、`.claude/CLAUDE.md`、`.claude/rules/*.md`、`@include`、循环保护、最大 include depth 和路径 frontmatter，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/claudemd.ts`。
- project config 包含 allowed tools、MCP context / servers、trust dialog、MCP approvals、worktree session、remote control spawn mode 等。配置写入有 lock、backup、auth-loss guard 和 `0o600` secure mode，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/config.ts`。
- transcript 采用 JSONL，路径按 project 和 session 组织。普通消息、attachment、system 会进入 chain，progress 不参与 chain；subagent transcript 和 remote agent sidecar 有独立路径，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/sessionStorage.ts`。
- resume 会恢复 worktree session、处理 missing worktree、清理 memory / system prompt / plan cache，并处理 coordinator / normal mode、fork session、agent restoration。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/sessionRestore.ts`。
- worktree 支持 slug 校验、复用、PR/default branch 创建、sparse paths、`.worktreeinclude` 复制、settings.local 复制、hooksPath、symlink directories、agent worktree 和 cleanup，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/worktree.ts`。

## 8. Coordinator / subagent / orchestration

- coordinator mode 受 feature flag 和 `CLAUDE_CODE_COORDINATOR_MODE` 控制。resume 时可匹配原 session mode，见 `docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts`。
- coordinator prompt 定义了 worker 调度模式：并行研究、coordinator synthesis、worker implementation、worker verification；worker prompt 必须自包含，因为 worker 看不到完整用户对话。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts`。
- agent definition 支持 built-in、custom、plugin 来源，字段包括 tools、disallowedTools、skills、MCP servers、hooks、model、effort、permissionMode、maxTurns、background、memory、isolation。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/loadAgentsDir.ts`。
- `AgentTool` 输入包含 prompt、subagent_type、description、model、background、team、mode、isolation、cwd。它会选择 agent、检查 required MCP server、组装 worker tool pool、处理 worktree isolation、fork prompt cache、sync / async 执行，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/AgentTool.tsx`。
- multi-agent teammate spawn 可走 tmux 或 in-process backend，并继承父进程权限、model、settings、plugin CLI flags，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/shared/spawnMultiAgent.ts`。
- async local agent 会注册为 task，记录 progress、transcript symlink、abort controller、cleanup 和 completion / failure 状态，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tasks/LocalAgentTask/LocalAgentTask.tsx`。

## 9. Plugin / skill / extension 机制

- plugin 类型区分 builtin 和 loaded plugin。loaded plugin 记录 manifest、path、source、repository、enabled、commands / agents / skills / output styles / hooks / MCP servers / LSP servers / settings，见 `docs/references/claude-code-sourcemap-main/restored-src/src/types/plugin.ts`。
- plugin loader 支持 marketplace plugins、session-only `--plugin-dir` / SDK plugins、manifest 校验、hooks config、重复检测、enable / disable 和结构化错误。cache 路径带 marketplace / plugin / version，并做 path / version sanitization，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/plugins/pluginLoader.ts`。
- skill 搜索来源包括 managed policy、user、project、plugin、legacy commands 和 explicit add-dir。`SKILL.md` frontmatter 支持 name、description、allowed-tools、when_to_use、model、disable model invocation、hooks、context=fork、agent、effort、shell、paths 等，见 `docs/references/claude-code-sourcemap-main/restored-src/src/skills/loadSkillsDir.ts`。
- skill 会被转换成 prompt command。skill prompt 注入 base directory，并替换 session / skill dir 变量；MCP skills 被视作 remote / untrusted，不执行 inline shell commands，见 `docs/references/claude-code-sourcemap-main/restored-src/src/skills/loadSkillsDir.ts`。
- bundled skills 会在启动时注册，部分受 feature gate 控制。bundle 内文件可懒提取到磁盘供 Read / Grep 使用，写入时禁止绝对路径和 `..`，并使用 owner-only 权限，见 `docs/references/claude-code-sourcemap-main/restored-src/src/skills/bundled/index.ts` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/skills/bundledSkills.ts`。
- `SkillTool` 让模型可以调用 skill。它禁止模型调用 non-prompt command 和 disableModelInvocation skill；权限规则先 deny，再 remote canonical auto-allow，再 allow，默认 ask，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/SkillTool/SkillTool.ts`。
- MCP skill builder 通过 write-once registry 暴露 skill command factory，目的是避免 import cycle 和 bundler dynamic import 问题，见 `docs/references/claude-code-sourcemap-main/restored-src/src/skills/mcpSkillBuilders.ts`。

## 10. Remote / terminal / coding workspace 相关设计

- CLI 层暴露 remote、remote-control、ssh、server、open、worktree、tmux、sdk-url 等入口或选项，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`。
- remote session manager 负责 WebSocket subscription、HTTP user message、permission request / response 和 cancel interrupt；未知 control subtype 会返回 error，避免 server hang，见 `docs/references/claude-code-sourcemap-main/restored-src/src/remote/RemoteSessionManager.ts`。
- remote WebSocket 使用 session subscription URL、Bearer token、anthropic-version header、ping interval、永久关闭码处理和有限重连策略，见 `docs/references/claude-code-sourcemap-main/restored-src/src/remote/SessionsWebSocket.ts`。
- SDK message adapter 把 CCR SDKMessage 映射为本地 REPL message / stream event，覆盖 assistant、result、system、status、tool_progress、compact_boundary 等事件，见 `docs/references/claude-code-sourcemap-main/restored-src/src/remote/sdkMessageAdapter.ts`。
- remote permission bridge 在本地没有真实 assistant message 时创建 synthetic assistant message，并为本地未知 remote tool 创建 stub，走本地 permission fallback，见 `docs/references/claude-code-sourcemap-main/restored-src/src/remote/remotePermissionBridge.ts`。
- bridge core 使用 session 创建、bridge JWT 交换、SSE + CCR transport、token refresh、SSE 401 recovery、UUID echo dedup、initial history flush、sequence-num resume 和 inbound permission / interrupt / model / permission-mode 控制，见 `docs/references/claude-code-sourcemap-main/restored-src/src/bridge/remoteBridgeCore.ts`。
- bridge session runner 通过 child process 启动 `claude --print --sdk-url ... --input-format stream-json --output-format stream-json`，解析 stdout NDJSON 中的 activity、result、control_request、first user message，并控制 kill / stdin，见 `docs/references/claude-code-sourcemap-main/restored-src/src/bridge/sessionRunner.ts`。
- task abstraction 覆盖 local bash、local agent、remote agent、workflow、monitor MCP、dream 等后台执行单元，见 `docs/references/claude-code-sourcemap-main/restored-src/src/Task.ts` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/tasks.ts`。
- remote agent task 会检查 login、remote env、git repo、git remote、GitHub app、policy；注册 sidecar metadata、poll remote events、处理 timeout / completion / archive，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tasks/RemoteAgentTask/RemoteAgentTask.tsx`。
- local shell task 对后台 shell 输出做 stall watchdog，发现疑似交互 prompt 会发 task-notification，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tasks/LocalShellTask/LocalShellTask.tsx`。

## 11. Security / permission / approval 线索

- CLI 暴露 `--permission-mode`、`--dangerously-skip-permissions`、`--allow-dangerously-skip-permissions` 等权限入口，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`。
- permission mode 包含 default、plan、acceptEdits、bypassPermissions、dontAsk、auto 等，external mode 会排除部分内部模式，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/PermissionMode.ts`。
- permission rule 来源包括 settings、CLI arg、command、session，并按 deny、ask、tool.checkPermissions、requiresUserInteraction、bypass、allow、passthrough、dontAsk、auto classifier、headless fallback 等顺序处理，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/permissions.ts`。
- dangerous permission setup 会识别 Bash tool-level allow、wildcard、script interpreter、PowerShell invoke / add-type 等危险模式，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/permissionSetup.ts`。
- interactive permission 入口在 `useCanUseTool`。它处理 allow / deny / ask、coordinator automated checks、swarm worker classifier approval、Bash speculative classifier grace period、bridge / channel callbacks，见 `docs/references/claude-code-sourcemap-main/restored-src/src/hooks/useCanUseTool.tsx`。
- project MCP server 有单个和多选 approval dialog，见 `docs/references/claude-code-sourcemap-main/restored-src/src/services/mcpServerApproval.tsx`。
- Bash 安全层包含 tree-sitter AST permission、shell injection validator、sandbox decision 和 destructive command warning。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/bashPermissions.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/bashSecurity.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/shouldUseSandbox.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/destructiveCommandWarning.ts`。
- sandbox 注释明确 excluded commands 是便利功能，不是安全边界。这个边界说明见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/shouldUseSandbox.ts`。
- cross-machine prompt injection 被视为 bypass-immune 风险。bridge address 的 send message 要 explicit ask，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/SendMessageTool/SendMessageTool.ts`。

## 12. 可迁移到 Octopus 的设计优点

本节只提可参考的设计优点，不给 Octopus 方案。

- 入口分层清晰：bootstrap 快速路径、完整 CLI、print / interactive 模式分开，便于控制启动成本和 headless 场景。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/entrypoints/cli.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/cli/print.ts`。
- command 类型明确：prompt、local、local-jsx 分离，使 slash command 可以区分模型提示扩展、本地副作用和 UI 交互。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/types/command.ts`。
- QueryEngine 与 query loop 分层合理：前者处理 session lifecycle，后者处理模型-工具循环。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`。
- Tool 协议把 schema、权限、并发、执行、渲染、结果映射放在统一边界中，便于横切治理。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts`。
- 工具执行区分并发安全和串行工具，并保证 context modifier 有序应用。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolOrchestration.ts` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/StreamingToolExecutor.ts`。
- session transcript、subagent transcript、remote sidecar、worktree restore 形成可恢复执行模型。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/sessionStorage.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/utils/sessionRestore.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/utils/worktree.ts`。
- skill / plugin 把用户扩展、项目扩展、托管扩展和 bundled 扩展统一成 prompt command / tool invocation，可以减少扩展机制分裂。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/commands.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/skills/loadSkillsDir.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/utils/plugins/pluginLoader.ts`。
- permission pipeline 是多来源、多阶段、fail-closed 倾向的治理模型。证据见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/permissions.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/hooks/useCanUseTool.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolHooks.ts`。

## 13. 不建议迁移的设计

- 不建议迁移 sourcemap 目录结构本身。该目录来源已经声明为 source map 还原材料，不代表官方内部仓库结构，见 `docs/references/claude-code-sourcemap-main/README.md`。
- 不建议照搬 `~/.claude` 路径和配置位置假设。memory、project config、transcript 均绑定 Claude 产品路径，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/config.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/utils/sessionStorage.ts`。
- 不建议默认暴露 bypass 权限开关。`--dangerously-skip-permissions` 和 bypassPermissions 存在，但危险性从命名和 permission 管线可见，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/PermissionMode.ts`。
- 不建议照搬 ant-only / feature-gated 分支。多个内部或 feature-gated 行为穿插在 CLI、coordinator、skills、Chrome、remote 中，作为 reference 只能提取边界概念，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/skills/bundled/index.ts`。
- 不建议复制 remote / CCR 产品协议。它包含 Anthropic session endpoint、JWT bridge、CCR transport、organization uuid、anthropic-version header 等产品绑定细节，见 `docs/references/claude-code-sourcemap-main/restored-src/src/remote/SessionsWebSocket.ts` 和 `docs/references/claude-code-sourcemap-main/restored-src/src/bridge/remoteBridgeCore.ts`。
- 不建议把 destructive command warning 当成权限逻辑。该文件注释说明 warning 只影响 permission dialog 信息，不影响 permission logic 或 auto-approval，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/destructiveCommandWarning.ts`。
- 不建议把 sandbox excluded commands 当安全边界。该边界在 `shouldUseSandbox.ts` 注释中已明确，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/shouldUseSandbox.ts`。

## 14. Unverified / open questions

- Unverified：该 sourcemap 的还原文件是否完整覆盖 npm 包运行时行为。依据只来自 reference `README.md` 的 source map 还原说明，见 `docs/references/claude-code-sourcemap-main/README.md`。
- Unverified：feature flags、ant-only gates、internal-only commands 在公开构建中的实际启用矩阵。源码中有 feature/env 条件，但具体构建配置不在本次分析范围内，见 `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx`、`docs/references/claude-code-sourcemap-main/restored-src/src/commands.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts`。
- Unverified：remote CCR 服务端协议的完整语义。本地只看到 client、bridge、adapter 和 task polling 侧，见 `docs/references/claude-code-sourcemap-main/restored-src/src/remote/RemoteSessionManager.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/bridge/remoteBridgeCore.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/tasks/RemoteAgentTask/RemoteAgentTask.tsx`。
- Unverified：Bash sandbox 在不同平台上的实际隔离强度。reference 中能确认 sandbox decision 和 permission 管线，不能确认底层 sandbox 实现的完整威胁模型，见 `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/shouldUseSandbox.ts`、`docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/bashPermissions.ts`。
- Unverified：coordinator / worker workflow 是否是稳定公开产品能力。它受 feature flag / env gate 控制，见 `docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts`。
- Unverified：plugin marketplace 的服务端信任、签名、发布、审核机制。本地 loader 只显示 manifest、cache、marketplace path 和错误处理，见 `docs/references/claude-code-sourcemap-main/restored-src/src/utils/plugins/pluginLoader.ts`。

## 15. 关键文件路径索引

| 主题 | 关键路径 |
|---|---|
| Sourcemap 性质 | `docs/references/claude-code-sourcemap-main/README.md` |
| Package / bin / runtime | `docs/references/claude-code-sourcemap-main/package/package.json` |
| 产品 README | `docs/references/claude-code-sourcemap-main/package/README.md` |
| Bootstrap CLI | `docs/references/claude-code-sourcemap-main/restored-src/src/entrypoints/cli.tsx` |
| Full CLI | `docs/references/claude-code-sourcemap-main/restored-src/src/main.tsx` |
| Print mode | `docs/references/claude-code-sourcemap-main/restored-src/src/cli/print.ts` |
| Command registry | `docs/references/claude-code-sourcemap-main/restored-src/src/commands.ts` |
| Command type model | `docs/references/claude-code-sourcemap-main/restored-src/src/types/command.ts` |
| Query lifecycle | `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts` |
| Query loop | `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts` |
| Tool protocol | `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts` |
| Tool registry | `docs/references/claude-code-sourcemap-main/restored-src/src/tools.ts` |
| Tool orchestration | `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolOrchestration.ts` |
| Streaming tool execution | `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/StreamingToolExecutor.ts` |
| Tool permission / call pipeline | `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolExecution.ts` |
| Tool hooks | `docs/references/claude-code-sourcemap-main/restored-src/src/services/tools/toolHooks.ts` |
| Context | `docs/references/claude-code-sourcemap-main/restored-src/src/context.ts` |
| CLAUDE.md memory | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/claudemd.ts` |
| Config / trust | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/config.ts` |
| Session storage | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/sessionStorage.ts` |
| Session restore | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/sessionRestore.ts` |
| Coordinator | `docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts` |
| Agent definitions | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/loadAgentsDir.ts` |
| Agent tool | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/AgentTool.tsx` |
| Multi-agent spawn | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/shared/spawnMultiAgent.ts` |
| Plugin types | `docs/references/claude-code-sourcemap-main/restored-src/src/types/plugin.ts` |
| Plugin loader | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/plugins/pluginLoader.ts` |
| Skill loader | `docs/references/claude-code-sourcemap-main/restored-src/src/skills/loadSkillsDir.ts` |
| Bundled skills | `docs/references/claude-code-sourcemap-main/restored-src/src/skills/bundled/index.ts` |
| Bundled skill files | `docs/references/claude-code-sourcemap-main/restored-src/src/skills/bundledSkills.ts` |
| Skill tool | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/SkillTool/SkillTool.ts` |
| Remote manager | `docs/references/claude-code-sourcemap-main/restored-src/src/remote/RemoteSessionManager.ts` |
| Remote WebSocket | `docs/references/claude-code-sourcemap-main/restored-src/src/remote/SessionsWebSocket.ts` |
| Remote SDK adapter | `docs/references/claude-code-sourcemap-main/restored-src/src/remote/sdkMessageAdapter.ts` |
| Remote permission bridge | `docs/references/claude-code-sourcemap-main/restored-src/src/remote/remotePermissionBridge.ts` |
| Bridge core | `docs/references/claude-code-sourcemap-main/restored-src/src/bridge/remoteBridgeCore.ts` |
| Bridge runner | `docs/references/claude-code-sourcemap-main/restored-src/src/bridge/sessionRunner.ts` |
| Task model | `docs/references/claude-code-sourcemap-main/restored-src/src/Task.ts` |
| Local shell task | `docs/references/claude-code-sourcemap-main/restored-src/src/tasks/LocalShellTask/LocalShellTask.tsx` |
| Local agent task | `docs/references/claude-code-sourcemap-main/restored-src/src/tasks/LocalAgentTask/LocalAgentTask.tsx` |
| Remote agent task | `docs/references/claude-code-sourcemap-main/restored-src/src/tasks/RemoteAgentTask/RemoteAgentTask.tsx` |
| Worktree | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/worktree.ts` |
| Permission modes | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/PermissionMode.ts` |
| Permission setup | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/permissionSetup.ts` |
| Permission pipeline | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/permissions.ts` |
| Interactive permission | `docs/references/claude-code-sourcemap-main/restored-src/src/hooks/useCanUseTool.tsx` |
| MCP approval | `docs/references/claude-code-sourcemap-main/restored-src/src/services/mcpServerApproval.tsx` |
| Bash security | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/bashSecurity.ts` |
| Bash permissions | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/bashPermissions.ts` |
| Bash sandbox | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/shouldUseSandbox.ts` |
| Destructive warning | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/destructiveCommandWarning.ts` |
| Cross-machine send permission | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/SendMessageTool/SendMessageTool.ts` |
