# References · 一级参考资料索引

本文件汇总 `docs/sdk/*.md` 全部引用的外部与本地资料。**每条目附：类型、原文标题、链接或路径、主要覆盖点。**

约定：

- "官方"指 Anthropic 发布的博客 / 文档；有明确发布时间
- "工程参考"指开源实现或社区资源
- "本仓"指 Octopus 仓库内已有文件

---

## A. Anthropic 官方博客与论文

### A1. Building Effective Agents
- **链接**：<https://www.anthropic.com/engineering/building-effective-agents>
- **时间**：2024-12-19
- **类型**：官方工程博客
- **覆盖**：Workflow vs Agent 分类；augmented LLM；prompt chaining / routing / parallelization / orchestrator-workers / evaluator-optimizer 五种 workflow 模式；代理核心原则（简洁、透明、ACI）。
- **章节引用**：`README.md` §0, §1；`01-core-loop.md` §1.1；`05-sub-agents.md` §5.2

### A2. How we built our multi-agent research system
- **链接**：<https://www.anthropic.com/engineering/multi-agent-research-system>
- **时间**：2025
- **类型**：官方工程博客
- **覆盖**：orchestrator-worker 架构、子代理 isolation、效果评估（token 消耗 +15x 换 90% 时间）、LLM-as-judge 评测、rainbow deployment、production reliability。
- **章节引用**：`05-sub-agents.md` §5.1-5.7；`09-observability-eval.md` §9.2-9.3；`10-failure-modes.md`

### A3. Effective context engineering for AI agents
- **链接**：<https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents>
- **时间**：2025-09
- **类型**：官方工程博客
- **覆盖**：context rot、attention budget、Goldilocks zone、JIT 检索、compaction、结构化笔记、sub-agent 隔离。
- **章节引用**：`02-context-engineering.md` 全章；`01-core-loop.md` §1.3；`10-failure-modes.md` C 组

### A4. Claude Code Best Practices
- **链接**：<https://www.anthropic.com/engineering/claude-code-best-practices>
- **类型**：官方工程博客
- **覆盖**：CLAUDE.md 配置、explore-plan-implement 流程、permission modes、autonomy dial、hooks、skills、subagents、session 管理（continue / resume / rewind）、headless 模式、常见失败模式。
- **章节引用**：`01-core-loop.md` §1.6-1.8；`02-context-engineering.md` §2.3；`06-permissions-sandbox.md` §6.1-6.2；`07-hooks-lifecycle.md`；`10-failure-modes.md` H/S 组

### A5. Writing effective tools for agents
- **链接**：<https://www.anthropic.com/engineering/writing-tools-for-agents>
- **类型**：官方工程博客
- **覆盖**：ACI 原则；选对工具、命名空间、有意义且 token-efficient 的返回、`response_format` enum、prompt-engineered description、错误 remediation、pagination/truncation、评测工具使用。
- **章节引用**：`03-tool-system.md` §3.3；`10-failure-modes.md` T 组

### A6. Code execution with MCP: Building more efficient agents
- **链接**：<https://www.anthropic.com/engineering/code-execution-with-mcp>
- **类型**：官方工程博客
- **覆盖**：把 MCP 服务暴露为代码 API；progressive disclosure；token 从 150k 降到 2k 的实测；privacy preserving；skills 通过文件系统持久化。
- **章节引用**：`03-tool-system.md` §3.6.3；`02-context-engineering.md` §2.8

### A7. Claude Code Sandboxing
- **链接**：<https://www.anthropic.com/engineering/claude-code-sandboxing>
- **类型**：官方工程博客
- **覆盖**：FS + 网络双隔离；bubblewrap/seatbelt；sandboxed bash tool；Claude Code on the web；Git proxy（凭据零暴露）。
- **章节引用**：`06-permissions-sandbox.md` 全章；`04-session-brain-hands.md` §4.3.3

### A8. Desktop Extensions (DXT)
- **链接**：<https://www.anthropic.com/engineering/desktop-extensions>
- **类型**：官方工程博客
- **覆盖**：桌面级 MCP server 的打包格式 DXT；manifest/entrypoint/lifecycle；native file picker、通知、菜单集成；PromptEngine 组件。
- **章节引用**：`03-tool-system.md` §3.6；`04-session-brain-hands.md` §4.7

### A9. Effective harnesses for long-running agents
- **链接**：<https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents>
- **类型**：官方工程博客
- **覆盖**：Initializer + Coding Agent 模式；`CLAUDE.md` / `NOTES.md` / `claude-progress.txt` / incremental git commits；Feature list；browser automation 测试循环。
- **章节引用**：`08-long-horizon.md` §8.2 模式 2；`10-failure-modes.md` L 组

### A10. Harness design for long-running application development
- **链接**：<https://www.anthropic.com/engineering/harness-design-long-running-apps>
- **类型**：官方工程博客
- **覆盖**：Planner / Generator / Evaluator 三代理（GAN 式）；独立 Evaluator；context reset vs compaction（与模型版本的关系）；sprint contract；Playwright MCP。
- **章节引用**：`05-sub-agents.md` §5.3；`08-long-horizon.md` §8.2-8.3；`10-failure-modes.md`

### A11. Scaling Managed Agents: Decoupling the brain from the hands
- **链接**：<https://www.anthropic.com/engineering/managed-agents>
- **时间**：2026
- **类型**：官方工程博客
- **覆盖**：Brain / Hands / Session 三分；pet-vs-cattle；lazy provisioning；凭据零暴露；many brains/many hands；Git 代理；harness staleness。
- **章节引用**：`04-session-brain-hands.md` 全章；`README.md` §2, §C3；`10-failure-modes.md` H/S 组

### A12. Equipping agents for the real world with Agent Skills
- **链接**：<https://www.anthropic.com/news/agent-skills>
- **类型**：官方发布
- **覆盖**：Agent Skills 概念；SKILL.md 结构；按需激活；与 plugin 的关系。
- **章节引用**：`03-tool-system.md` §3.7

### A13. Claude Agent SDK — Overview
- **链接**：<https://docs.claude.com/en/api/agent-sdk/overview>
- **类型**：官方文档
- **覆盖**：SDK 能力总览：内置工具、Hooks、Subagents、MCP、Permissions、Sessions；与 Client SDK / Claude Code CLI 的区别。
- **章节引用**：`README.md` §3.4；`01-core-loop.md`；`03-tool-system.md`

### A14. Claude Agent SDK — Hooks
- **链接**：<https://docs.claude.com/en/api/agent-sdk/hooks>
- **类型**：官方文档
- **覆盖**：Hook 点清单（PreToolUse、PostToolUse、SessionStart、UserPromptSubmit、Stop、PreCompact 等）；签名与配置。
- **章节引用**：`07-hooks-lifecycle.md` 全章

### A15. Claude Agent SDK — Subagents
- **链接**：<https://docs.claude.com/en/api/agent-sdk/subagents>
- **类型**：官方文档
- **覆盖**：Subagent 注册、工具白名单、上下文隔离、Agent Definition frontmatter。
- **章节引用**：`05-sub-agents.md` §5.6

### A16. Claude Agent SDK — Sessions
- **链接**：<https://docs.claude.com/en/api/agent-sdk/sessions>
- **类型**：官方文档
- **覆盖**：Session 持久化、resume / fork 语义。
- **章节引用**：`01-core-loop.md` §1.6；`04-session-brain-hands.md`

### A17. Claude Agent SDK — MCP
- **链接**：<https://docs.claude.com/en/api/agent-sdk/mcp>
- **类型**：官方文档
- **覆盖**：MCP 客户端形态（stdio / sse / sdk in-process）；工具命名空间；工具可见性。
- **章节引用**：`03-tool-system.md` §3.6

### A18. Claude Agent SDK — Plugins
- **链接**：<https://docs.claude.com/en/api/agent-sdk/plugins>
- **类型**：官方文档
- **覆盖**：Plugin 打包与分发；`plugin.json` 清单；commands/agents/skills/hooks/mcpServers 五类组件；marketplace 源（github/git/npm/url/file/directory）；`{name}@{marketplace}` 命名；dependencies；built-in vs marketplace 区分。
- **章节引用**：`03-tool-system.md` §3.9；`12-plugin-system.md` 全章

### A19. Prompt Caching（官方文档）
- **链接**：<https://docs.claude.com/en/docs/build-with-claude/prompt-caching>
- **类型**：官方文档
- **覆盖**：cache_control 位置规则；稳定前缀要求。
- **章节引用**：`02-context-engineering.md` §2.10；`10-failure-modes.md` C1

### A20. Model Context Protocol 官方
- **链接**：<https://modelcontextprotocol.io>
- **类型**：官方规范（Anthropic 发起、多厂商共建）
- **覆盖**：协议定义、参考实现、生态。
- **章节引用**：`03-tool-system.md` §3.6

### A21. Anthropic Messages API
- **链接**：<https://docs.claude.com/en/api/messages>
- **类型**：官方 API 文档
- **覆盖**：`/v1/messages` 请求/响应结构；content blocks；tool_use；thinking blocks；streaming。
- **章节引用**：`11-model-system.md` §11.7（Protocol Adapter）

### A22. OpenAI Responses API
- **链接**：<https://platform.openai.com/docs/api-reference/responses>
- **类型**：官方 API 文档
- **覆盖**：`/v1/responses` 请求/响应结构；built-in tools（web search / file search / computer use / MCP）。
- **章节引用**：`11-model-system.md` §11.7

### A23. OpenAI Chat Completions
- **链接**：<https://platform.openai.com/docs/api-reference/chat>
- **类型**：官方 API 文档（兼容基线）
- **覆盖**：`/v1/chat/completions`；function calling；parallel tool use；为绝大多数"OpenAI-compatible" provider 的基线契约。
- **章节引用**：`11-model-system.md` §11.7

### A24. Gemini API — Models
- **链接**：<https://ai.google.dev/gemini-api/docs/models>
- **类型**：官方 API 文档
- **覆盖**：`models:generateContent`；多模态输入输出；Live/TTS/Files/Batch/Context Caching 等 surface。
- **章节引用**：`11-model-system.md` §11.7、§11.12

---

## B. 工程参考（开源 / 第三方研究）

### B1. anthropic-experimental/sandbox-runtime
- **链接**：<https://github.com/anthropic-experimental/sandbox-runtime>
- **类型**：开源参考实现
- **覆盖**：OS 级沙箱原语封装（Linux bubblewrap / macOS seatbelt）。
- **章节引用**：`06-permissions-sandbox.md` §6.7

### B2. Chroma Research · Context Rot
- **链接**：<https://research.trychroma.com/context-rot>
- **类型**：社区研究
- **覆盖**：长输入 token 对 LLM 性能的量化影响；多模型对比。
- **章节引用**：`02-context-engineering.md` §2.1；`10-failure-modes.md` C2

### B3. OpenRouter · Provider Routing
- **链接**：<https://openrouter.ai/docs/provider-routing>
- **类型**：第三方网关文档
- **覆盖**：price/throughput/latency 排序；provider allowlist/blocklist；require_parameters；data_collection 策略。
- **章节引用**：`11-model-system.md` §11.9.3

### B4. Models.dev
- **链接**：<https://models.dev>
- **类型**：社区维护的模型目录
- **覆盖**：跨厂商 model id / context / pricing 快照；可作为 Catalog 的可选远端同步源。
- **章节引用**：`11-model-system.md` §11.4

### B5. Claude Desktop Extensions (DXT) 规范
- **链接**：<https://www.anthropic.com/engineering/desktop-extensions>
- **类型**：官方博客
- **覆盖**：`.dxt`/`.mcpb` 离线 bundle 打包格式；单文件分发；manifest + runtime + 资产；签名与安全。
- **章节引用**：`12-plugin-system.md` §12.6.5

---

## C. 本仓内参考（`docs/references/`）

### C1. Claude Code（还原源）
- **路径**：`docs/references/claude-code-sourcemap-main/`
- **路径简写约定**：本文件及 `docs/sdk/*.md` 其他章节中出现的 `restored-src/src/...` 一律指代 `docs/references/claude-code-sourcemap-main/restored-src/src/...`（省略前缀仅为正文简洁，不是另一个位置）。
- **README**：`docs/references/claude-code-sourcemap-main/README.md`
- **说明**：从 `@anthropic-ai/claude-code` npm 包 sourcemap 反编译的非官方 repo（v2.1.88, ~4756 files, 1884 TS/TSX）。
- **关键文件**：
  - `restored-src/src/Tool.ts` — `ToolUseContext` / `ToolPermissionContext` / re-export `PermissionMode` / re-export `ToolPermissionRulesBySource`
  - `restored-src/src/types/permissions.ts` — `PermissionMode` 真实枚举定义 / `ToolPermissionContext` 源定义 / `PermissionRule` / `PermissionDecisionReason` / `YoloClassifierResult`
  - `restored-src/src/utils/shell/outputLimits.ts` — `BASH_MAX_OUTPUT_DEFAULT=30_000` / `BASH_MAX_OUTPUT_UPPER_LIMIT=150_000` / `getMaxOutputLength()`
  - `restored-src/src/services/api/withRetry.ts` — `FallbackTriggeredError` 定义与抛出点
  - `restored-src/src/query.ts` — 主 query loop；compaction、fallback、错误处理
  - `restored-src/src/query/tokenBudget.ts` — `BudgetTracker` / `checkTokenBudget`
  - `restored-src/src/query/stopHooks.ts` — stop hook
  - `restored-src/src/services/tools/toolOrchestration.ts` — `partitionToolCalls`、`runToolsConcurrently`、`runToolsSerially`
  - `restored-src/src/services/tools/toolExecution.ts` — 单次工具执行 + 权限 + 计时
  - `restored-src/src/services/mcp/*` — MCP 客户端
  - `restored-src/src/services/compact/*` — compaction 实现
  - `restored-src/src/hooks/*` — 各 hook 点
  - `restored-src/src/hooks/toolPermission/useCanUseTool.tsx` — 权限判定
  - `restored-src/src/tools/*` — 内置工具（Read / Write / FileEdit / Glob / Grep / BashTool / WebSearch / WebFetch / AskUserQuestion / TodoWrite / AgentTool / SkillTool / ...）
  - `restored-src/src/coordinator/coordinatorMode.ts` — 多代理协调
  - `restored-src/src/skills/*` — Skills 子系统
  - `restored-src/src/utils/model/providers.ts` — `APIProvider = firstParty | bedrock | vertex | foundry`；`getAPIProvider()` 环境切换
  - `restored-src/src/utils/model/model.ts` — `getMainLoopModel` / `getRuntimeMainLoopModel` / `firstPartyNameToCanonical` / `getCanonicalName` / 优先级链解析
  - `restored-src/src/utils/model/aliases.ts` — `MODEL_ALIASES`（sonnet/opus/haiku/best/opusplan/...）与 `MODEL_FAMILY_ALIASES`
  - `restored-src/src/utils/model/agent.ts` — subagent 模型解析；`aliasMatchesParentTier` 防降级继承；Bedrock region prefix 继承
  - `restored-src/src/utils/model/modelCapabilities.ts` — 本地缓存 `cache/model-capabilities.json`；`getModelCapability`
  - `restored-src/src/utils/model/modelAllowlist.ts` — `isModelAllowed` 白名单校验
  - `restored-src/src/services/api/client.ts` — 4 种后端 client 切换；多凭据类型（ANTHROPIC_API_KEY / AWS / GCP ADC / Azure AD）
  - **【插件相关】** `restored-src/src/types/plugin.ts` — `BuiltinPluginDefinition` / `LoadedPlugin` / `PluginRepository` / `PluginComponent`（`commands` `agents` `skills` `hooks` `output-styles`）/ `PluginError`（discriminated union ~22 型）/ `getPluginErrorMessage`
  - **【插件相关】** `restored-src/src/plugins/builtinPlugins.ts` — `BUILTIN_MARKETPLACE_NAME='builtin'` / `registerBuiltinPlugin` / `isBuiltinPluginId` / `getBuiltinPlugins` 拆成 enabled/disabled
  - **【插件相关】** `restored-src/src/plugins/bundled/index.ts` — `initBuiltinPlugins()` 注册入口
  - **【插件相关】** `restored-src/src/utils/plugins/schemas.ts` — 完整 Zod schemas：`PluginManifestSchema` 组合 10 个子 schema（metadata/hooks/commands/agents/skills/output-styles/channels/mcpServers/lspServers/settings/userConfig）/ `MarketplaceSourceSchema` 9 型 discriminatedUnion（url/github/git/npm/file/directory/hostPattern/pathPattern/settings）/ `PluginSourceSchema`（相对路径/npm/pip/url/github/git-subdir）/ `gitSha` 40 字符校验 / `DependencyRefSchema` / `PluginMarketplaceSchema` / `ALLOWED_OFFICIAL_MARKETPLACE_NAMES` 保留名白名单 / `BLOCKED_OFFICIAL_NAME_PATTERN` 冒名正则 / `NON_ASCII_PATTERN` homograph 防护 / `isBlockedOfficialName` / `validateOfficialNameSource`（官方保留名仅允许 `github.com/anthropics/*` 来源）/ `CommandMetadataSchema`（source/content 二选一 + description/argumentHint/model/allowedTools override）/ `McpbPath`（`.mcpb`/`.dxt`）/ `SettingsPluginEntrySchema` / `InstalledPluginSchema`
  - **【插件相关】** `restored-src/src/utils/plugins/*.ts` — 30+ 工具文件：`pluginLoader.ts`（核心加载）/ `marketplaceManager.ts`（市场清单管理）/ `reconciler.ts`（`diffMarketplaces` / `reconcileMarketplaces`）/ `refresh.ts`（`refreshActivePlugins`）/ `pluginBlocklist.ts`（企业策略）/ `pluginPolicy.ts` / `pluginVersioning.ts` / `dependencyResolver.ts` / `mcpbHandler.ts`（MCPB 下载/解压）/ `loadPluginAgents.ts` / `loadPluginCommands.ts` / `loadPluginHooks.ts` / `loadPluginOutputStyles.ts` / `mcpPluginIntegration.ts` / `lspPluginIntegration.ts` / `headlessPluginInstall.ts` / `performStartupChecks.tsx`
  - **【插件相关】** `restored-src/src/services/plugins/PluginInstallationManager.ts` — 背景安装；不阻塞启动；`performBackgroundPluginInstallations`；`updateMarketplaceStatus`（pending/installing/installed/failed）

### C2. Hermes Agent
- **路径**：`docs/references/hermes-agent/hermes-agent-main/`
- **README**：同上 + `AGENTS.md`
- **说明**：Nous Research 的自改进 Agent；Python；多平台（Telegram/Discord/CLI）；profile-safe 多实例。
- **关键文件**：
  - `hermes/run_agent.py` — 核心 loop
  - `hermes/model_tools.py` — 工具编排
  - `hermes/toolsets.py` — toolset 打包
  - `hermes/cli.py` — 交互式 CLI 骨架
  - `hermes/hermes_state.py` — SQLite + FTS5 session store
  - `hermes/agent/prompt_builder.py` — 提示词装配
  - `hermes/agent/context_compressor.py` — 上下文压缩
  - `hermes/agent/memory_manager.py` — 记忆/笔记
  - `hermes/tools/` — 工具实现、注册、环境
  - `hermes/gateway/` — 消息平台
  - `hermes/acp_adapter/`, `hermes/cron/` — 定时/适配
  - **【插件相关】** `hermes/plugins/__init__.py` — Python 包骨架
  - **【插件相关】** `hermes/plugins/context_engine/` — `contextEngine` slot 的实现目录
  - **【插件相关】** `hermes/plugins/memory/<name>/` — 8 个 memory backend provider（`byterover`/`hindsight`/`holographic`/`honcho`/`mem0`/`openviking`/`retaindb`/`supermemory`）；每个独立 Python 子模块 + ABC `MemoryProvider` 实现
  - **【插件相关】** `hermes/plugins/memory/mem0/__init__.py` — 具体 provider 插件实现范例：`_load_config()` 双源（env vars + `$HERMES_HOME/mem0.json`）；`_BREAKER_THRESHOLD` / `_BREAKER_COOLDOWN_SECS` circuit breaker；`MemoryProvider` ABC 实现
- **关键规约**：prompt cache 稳定性；profile-safe code；后台进程通知约定。

### C3. OpenClaw
- **路径**：`docs/references/openclaw/openclaw-main/`
- **README**：同上 + `CLAUDE.md`
- **说明**：个人设备 AI 助理；多渠道（WhatsApp/Telegram 等）；强调快速、本地、永在线。**本仓参考项目中插件体系最成熟、覆盖最广的一个**（90+ bundled extensions、12 类 capability、44 个 provider runtime hook）。
- **关键规约**：
  - 插件/扩展边界：`@openclaw/plugin-sdk/*`
  - Prompt cache 稳定性（`deterministic ordering`）
  - Channel 与 Gateway 协议
  - Sandbox 镜像（`Dockerfile.sandbox*`）
  - `~/.openclaw/agents/<id>/sessions/*.jsonl` session 目录
- **【插件相关】关键文件**：
  - `docs/plugins/architecture.md` — **插件架构权威文档**。四层架构（Manifest + discovery / Enablement + validation / Runtime loading / Surface consumption）；12 类 capability 注册表（text/cli-backend/speech/realtime-transcription/realtime-voice/media-understanding/image-generation/music-generation/video-generation/web-fetch/web-search/channel）；4 种 plugin shape（plain-capability/hybrid-capability/hook-only/non-capability）；Compatibility signals 4 级（valid/advisory/legacy warning/hard error）；Capability ownership model（plugin = 公司/功能所有权边界）；`OpenClawPluginApi` registry 单向流；Load pipeline 7 步；Manifest-first behavior；缓存策略（`OPENCLAW_DISABLE_PLUGIN_DISCOVERY_CACHE` 等）；44 个 provider runtime hook（`catalog`/`applyConfigDefaults`/`normalizeModelId`/... /`onModelSelected`）含使用时机表；HTTP route auth 规则（`gateway`/`plugin` 二选一 + admin namespace 保留）；Context engine plugin（`api.registerContextEngine` + `plugins.slots.contextEngine`）；Package pack（`openclaw.extensions[]` + `setupEntry`）；Plugin SDK import paths（narrow subpath vs forbidden root barrel）
  - `docs/plugins/building-plugins.md`、`sdk-overview.md`、`sdk-entrypoints.md`、`sdk-runtime.md`、`sdk-channel-plugins.md`、`sdk-provider-plugins.md`、`manifest.md` — 各类插件构建指南
  - `packages/plugin-sdk/` — SDK 源码；`src/plugin-entry.ts` / `plugin-runtime.ts` / `provider-entry.ts` / `provider-auth.ts` / `provider-http.ts` / `provider-model-shared.ts` / `provider-stream-shared.ts` / `provider-tools.ts` / `provider-web-search.ts` / `runtime-doctor.ts` / `security-runtime.ts` / `video-generation.ts` …
  - `packages/plugin-package-contract/src/index.ts` — `ExternalPluginCompatibility`（`pluginApiRange`/`builtWithOpenClawVersion`/`pluginSdkVersion`/`minGatewayVersion`）；`EXTERNAL_CODE_PLUGIN_REQUIRED_FIELD_PATHS`（`openclaw.compat.pluginApi` + `openclaw.build.openclawVersion` 两个必需字段）；`validateExternalCodePluginPackageJson`
  - `extensions/CLAUDE.md` — 扩展边界公共契约；禁止深导入 `src/**`、`channels/**`、`plugin-sdk-internal/**`、其他插件的 `src/**`；`api.ts`/`runtime-api.ts` barrel 暴露规则；同 provider family 去重规则
  - `extensions/<90+ 插件>` — 实际落地样本：
    - **Providers**：`openai`/`anthropic`/`anthropic-vertex`/`amazon-bedrock`/`amazon-bedrock-mantle`/`google`/`deepseek`/`minimax`/`moonshot`/`qwen`/`kimi-coding`/`zai`/`xai`/`mistral`/`openrouter`/`microsoft-foundry`/`litellm`/`together`/`groq`/`fireworks`/`huggingface`/`nvidia`/`perplexity`/`qianfan`/`byteplus`/`cloudflare-ai-gateway`/`vercel-ai-gateway`/`volcengine`/`venice`/`chutes`/`arcee`/`synthetic`/`stepfun`/`vllm`/`sglang`/`lmstudio`/`ollama`/`copilot-proxy`/`github-copilot`/`codex`/`opencode`/`opencode-go`/`kilocode`
    - **Channels**：`slack`/`telegram`/`whatsapp`/`discord`/`signal`/`imessage`/`matrix`/`msteams`/`googlechat`/`feishu`/`line`/`mattermost`/`nextcloud-talk`/`synology-chat`/`tlon`/`twitch`/`webhooks`/`voice-call`/`talk-voice`/`bluebubbles`/`irc`/`nostr`/`qq-bot`/`zalo`/`zalouser`/`xiaomi`
    - **Feature/Capability**：`image-generation-core`/`media-understanding-core`/`speech-core`/`video-generation-core`/`memory-core`/`memory-lancedb`/`memory-wiki`/`active-memory`/`voice-call`/`browser`/`device-pair`/`diagnostics-otel`/`diffs`/`exa`/`firecrawl`/`brave`/`tavily`/`duckduckgo`/`searxng`/`runway`/`fal`/`elevenlabs`/`deepgram`/`comfy`
    - **Meta**：`acpx`/`llm-task`/`lobster`/`open-prose`/`qa-channel`/`qa-lab`/`qa-matrix`/`thread-ownership`
  - `extensions/tsconfig.package-boundary.base.json` / `tsconfig.package-boundary.paths.json` — 包边界 tsconfig
  - `packages/memory-host-sdk/` — 记忆主机 SDK（memory backend plugin 使用）

### C4. Claude Hidden Toolkit
- **路径**：`docs/references/Claude_Hidden_Toolkit.md`
- **说明**：逆向总结 Claude.ai 37 个未公开内部工具；分类：context / interaction / visualization / calendar-device / search-data / memory / computer-use / meta-tool。
- **重点条目**：
  - `tool_search` meta-tool（progressive disclosure 的证据）
  - Skills 文件系统拓扑
  - Egress Proxy JWT 架构
  - `anthropic_api_in_artifacts`
  - `persistent_storage` 工具

### C5. 本仓根 `AGENTS.md`
- **路径**：`AGENTS.md`
- **说明**：Octopus 本仓的**强制治理规则**。本 SDK 的落地约束直接引用其中：
  - AI Planning And Execution Protocol
  - Frontend Governance（desktop Vue + Tauri 基线、Shared UI Catalog）
  - Request Contract Governance（`contracts/openapi/`，adapter 边界）
  - Persistence Governance（文件 / SQLite / JSONL 分工）
  - Runtime Config And Runtime Persistence Rules（三层合并、config snapshot、live session 不跟随磁盘）

### C6. 本仓 `docs/` 其他相关
- `docs/api-openapi-governance.md` — HTTP 契约流程
- `docs/design/DESIGN.md` — UI 真理源
- `docs/plans/` — 计划/执行模板
- `docs/runtime_config_api.md` — 运行时配置 API
- `docs/capability_runtime.md` — 能力 runtime 设计
- `docs/SAD.md` / `docs/PRD.md` — 项目 SAD/PRD
- `docs/sdk/13-contracts-map.md` — SDK 章节与四条真相源（OpenAPI / `@octopus/schema` / `@octopus/ui` / UI 意图 IR）的双向映射表
- `docs/sdk/14-ui-intent-ir.md` — SDK 层 UI 意图 IR 契约：定义 `RenderBlock` / `RenderLifecycle` / `AskPrompt` / `ArtifactRef` 顶层描述符与 5 个工具渲染钩子，用于融合 Claude.ai 成果物展示与 Claude Code 内联 diff 两种范式

### C7. 本仓 `docs/references/vendor-matrix.md`
- **路径**：`docs/references/vendor-matrix.md`
- **说明**：Octopus 模型系统的**事实源**。9 厂商（deepseek / minimax / moonshot / bigmodel / qwen / ark / openai / google + anthropic）× Surface Matrix × Latest Model Lineup；每条记录附官方 source + `last_verified_at` + `evidence_scope`。
- **关键字段**：`provider_id`、`surface`、`protocol_family`（openai_chat/openai_responses/anthropic_messages/gemini_native/vendor_native）、`transport`、`auth_strategy`、`base_url`、`capability_matrix`、`api_model_id`、`family`、`underlying_release`、`track`（stable/preview/latest_alias）。
- **章节引用**：`11-model-system.md` 全章（Catalog 内置快照的派生源）

### C8. Hermes `cli-config.yaml.example` 模型章节
- **路径**：`docs/references/hermes-agent/hermes-agent-main/cli-config.yaml.example`
- **说明**：Hermes 对**15+ 厂商**的显式支持规范。关键章节：
  - §"Model Configuration"（provider 枚举 + auto 检测 + base_url + context_length/max_tokens 解释）
  - §"OpenRouter Provider Routing"（sort/only/ignore/order/require_parameters/data_collection）
  - §"Smart Model Routing"（cheap_model 路由短简单请求）
  - §"Auxiliary Models"（vision / web_extract / compression 各自独立 provider+model）
  - §"Model Aliases"（`{model, provider, base_url}` 元组自定义）
  - §"Delegation"（子代理 model/provider override）
- **章节引用**：`11-model-system.md` §11.4-11.10

---

## D. 术语标准

- **MCP 规范** — <https://modelcontextprotocol.io/specification>
- **OpenTelemetry 规范** — <https://opentelemetry.io/docs/specs/>
- **ULID 规范** — <https://github.com/ulid/spec>

---

## E. 如何验证引用

- 外部链接可达性：不在本文件做"当前可达"的强断言；由 CI 周期性探活（建议用 `lychee` / `lycheeverse/lychee-action`）。外链失效时按本节末尾流程处理。
- 本地文件引用：使用相对路径，在本仓根目录（`/Users/goya/Work/weilaizhihuigu/super-agent/octopus/`）下全部有效。
- 源码快照版本：所有指向 `restored-src/src/**` 的引用以 **`restored-src v2.1.88`** 为准（见 §C1）；文档中的常量/字段名随该快照固定，未来升级以独立修订记录。
- 源码行号：文档中形如 `path/file.ts:16-36` 的行号区间严格对应 v2.1.88 快照；快照升级时须按符号名（函数 / 类型 / 常量）重新定位，不要盲信旧行号。硬编码行号规模：经核查仅 8 处（全部在 03 / 06 章的 `permissions.ts` / `Tool.ts` 类型定义引用），保留行号以提升定位精度。
- 文档时点口径：本次 `docs/sdk/` 文档的事实校正完成于 2026-04-20，参考 Anthropic 官方资料截至同日。

**外链失效处理流程**：

1. 通过 WebSearch 找到官方等价替换（优先 anthropic.com / docs.claude.com 自身跳转）；
2. 更新本文件 + 对应章节的 inline 链接；
3. 在 `docs/plans/YYYY-MM-DD-sdk-refs-update.md` 记录变更（迁移前 URL / 迁移后 URL / 受影响章节）。
