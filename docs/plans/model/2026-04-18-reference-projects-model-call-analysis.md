# 参考项目模型调用实现梳理：Claude Code 与 OpenClaw

## 目标与范围

本文补充分析两个参考项目中“模型如何被选择、鉴权、发起请求并接回上层 agent/runtime”的实现：

- `docs/references/claude-code-sourcemap-main`
- `docs/references/openclaw/openclaw-main`

重点回答四个问题：

1. 模型调用链路如何分层。
2. provider、model、auth、transport、runtime loop 分别落在哪一层。
3. 关键实现位置在哪里，后续应从什么文件开始借鉴。
4. 对 Octopus 的模型模块演进，哪些设计最值得直接吸收。

本文只聚焦模型调用主链路，不展开单个工具业务逻辑、UI 展示细节或测试细节。

## 一句话结论

- `Claude Code` 采用的是 **Anthropic Messages 协议中心化** 设计：上层请求构造、stream event、tool loop 都围绕 Claude 协议建模，provider 切换主要发生在 client/auth/transport 层。
- `OpenClaw` 采用的是 **registry/auth/transport/plugin hook 解耦** 设计：模型解析、鉴权解析、传输策略、provider 兼容包装彼此分层，实际生成能力大量复用 `@mariozechner/pi-ai` / `@mariozechner/pi-coding-agent`。
- 对 Octopus 来说，更值得借鉴的不是“照搬某个 SDK”，而是：
  - 借 Claude Code 的 `runtime loop` 与 `provider client` 分离。
  - 借 OpenClaw 的 `model resolution / auth / request policy` 分层。

## Claude Code：Anthropic 协议中心化的多后端调用链

### 分层判断

Claude Code 的模型调用不是“先选 provider，再走完全不同的一套 runtime”，而是统一走 Claude 风格消息协议与 query loop：

1. `provider` 选择：决定是 `firstParty / bedrock / vertex / foundry`
2. `model` 选择：决定主循环用什么模型，以及 plan/大上下文场景下是否切模型
3. `client` 构造：在统一入口里处理 API key、OAuth、AWS、GCP、Azure、base URL、headers、proxy
4. `request` 组装：统一构造 Claude Messages 请求、tools、thinking、betas、cache 参数
5. `query loop`：统一消费 stream、识别 `tool_use`、执行工具、回写 `tool_result`

这意味着 Claude Code 虽然支持多个云后端，但并不是 OpenClaw 那种“provider-agnostic registry-first”架构，而是“一个上层协议模型 + 多个下层 transport/auth 后端”。

### 关键实现位置

| 层 | 作用 | 实现位置 |
| --- | --- | --- |
| Provider 选择 | 基于环境变量选择 `firstParty / bedrock / vertex / foundry`，并判断 `ANTHROPIC_BASE_URL` 是否仍属官方 Anthropic API | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/model/providers.ts:6` `getAPIProvider()`；`:25` `isFirstPartyAnthropicBaseUrl()` |
| 模型默认值与运行时模型 | 决定主循环默认模型、Opus/Sonnet 默认值、plan 模式下的 runtime model | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/model/model.ts:92` `getMainLoopModel()`；`:105` `getDefaultOpusModel()`；`:119` `getDefaultSonnetModel()`；`:145` `getRuntimeMainLoopModel()` |
| Provider 相关模型字符串映射 | 把 provider 差异折叠到模型字符串映射与 override 层 | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/model/modelStrings.ts:25` `getBuiltinModelStrings()`；`:63` `applyModelOverrides()` |
| Client 构造与鉴权 | 统一创建 Anthropic client，并在内部切 Bedrock / Vertex / Foundry / first-party auth 与 headers | `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/client.ts:88` `getAnthropicClient()` |
| 请求组装与非流式/流式调用 | 构造 messages/system/tools/thinking 请求，并执行 streaming 或 fallback non-streaming 调用 | `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts:709` `queryModelWithoutStreaming()`；`:752` `queryModelWithStreaming()`；`:1017` `queryModel()` |
| 会话级入口 | 每个会话一个 `QueryEngine`，负责跨 turn 的消息、usage、permission denials、session state 管理 | `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts:184` `QueryEngine`；`:209` `submitMessage()` |
| Query / Tool Loop | 主循环里选择 runtime model、消费 stream、收集 `tool_use`，并在回合末执行工具 | `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts:219` `query()`；`:241` `queryLoop()`；`:563` `new StreamingToolExecutor(...)`；`:572` `getRuntimeMainLoopModel(...)`；`:1382` `runTools(...)` |

### 设计要点

#### 1. provider 差异主要被压缩在 client 层

`getAnthropicClient()` 是 Claude Code 非常关键的分层点。它把以下变化都收拢在一个地方：

- first-party API key / OAuth
- Bedrock AWS credentials 与 region
- Vertex GCP project / auth
- Foundry Azure AD / API key
- custom headers、session headers、proxy fetch

这使得上层 `queryModel()` 不需要关心各家云后端细节。

#### 2. 模型切换发生在 runtime loop 边界，而不是深埋在 provider adapter 内

`query.ts:572` 在真正发请求前调用 `getRuntimeMainLoopModel(...)`，会根据：

- permission mode
- main loop model
- 是否超大上下文

来决定本轮最终模型。这个决策点放在 loop 内，而不是 SDK 内部，便于以后加入 token budget、plan mode、compaction 等规则。

#### 3. stream event 先被统一消费，再决定是否进入工具执行

`services/api/claude.ts` 负责发请求和回收 stream，`query.ts` 负责把 assistant/tool_use/tool_result 变成消息闭环。这里的核心不是单次 completion，而是：

- 先统一事件
- 再统一回合控制
- 再统一工具执行与续轮

这对支持长会话、resume、compaction、子代理都非常重要。

### 对 Octopus 最值得借鉴的点

1. `provider client/auth` 与 `turn loop` 要严格分开。
2. runtime loop 应该自己掌握模型切换，而不是把切换逻辑埋进 provider SDK。
3. stream 返回值要先标准化成内部事件，再驱动消息状态与工具循环。
4. 即便后端很多，尽量保持一个统一的上层 conversation protocol，而不是每家 provider 各写一套 loop。

## OpenClaw：Registry/Auth/Transport/Hook 解耦的 provider-agnostic 设计

### 分层判断

OpenClaw 的模型调用设计比 Claude Code 更模块化，核心不是“统一走某家协议”，而是先把模型调用拆成多个边界：

1. `model ref parsing`
2. `auth resolution`
3. `registry/discovery`
4. `resolved model`
5. `provider request policy / transport`
6. `runtime stream wrapper / compat patch`
7. `simple completion` 与 `full agent runtime` 两条路径

真正的生成执行大量委托给 `@mariozechner/pi-ai` / `@mariozechner/pi-coding-agent`，而 OpenClaw 自己重点负责“解析、注入、兼容、包装、插件扩展”。

### 关键实现位置

| 层 | 作用 | 实现位置 |
| --- | --- | --- |
| 模型引用解析 | 解析 `provider/model`、alias、默认 provider、subagent spawn model | `docs/references/openclaw/openclaw-main/src/agents/model-selection.ts:381` `resolveModelRefFromString()`；`:528` `resolveSubagentSpawnModelSelection()` |
| Provider 鉴权解析 | 在 profile、env、config、synthetic local auth、plugin synthetic auth、aws-sdk 间做优先级决策 | `docs/references/openclaw/openclaw-main/src/agents/model-auth.ts:409` `resolveApiKeyForProvider()`；`:713` `getApiKeyForModel()`；`:735` `applyLocalNoAuthHeaderOverride()`；`:767` `applyAuthHeaderOverride()` |
| Registry / Discovery | 建 auth storage 与 model registry，把外部发现能力收束到 OpenClaw 运行时入口 | `docs/references/openclaw/openclaw-main/src/agents/pi-model-discovery.ts:283` `discoverAuthStorage()`；`:290` `discoverModels()` |
| 显式模型、动态模型、fallback 模型解析 | 先查显式模型，再查插件动态模型，再回落到 configured fallback model | `docs/references/openclaw/openclaw-main/src/agents/pi-embedded-runner/model.ts:368` `resolveExplicitModelWithRegistry()`；`:449` `resolvePluginDynamicModelWithRegistry()`；`:494` `resolveConfiguredFallbackModel()`；`:605` `resolveModelWithRegistry()`；`:708` `resolveModelAsync()` |
| Provider request policy / transport | 解析 base URL、headers、auth header 注入、proxy、TLS、allowPrivateNetwork、header precedence | `docs/references/openclaw/openclaw-main/src/agents/provider-request-config.ts:182` `sanitizeConfiguredProviderRequest()`；`:603` `resolveProviderRequestPolicyConfig()`；`:668` `resolveProviderRequestConfig()`；`:696` `resolveProviderRequestHeaders()` |
| 简单 completion 路径 | 把“单次 completion”与“完整 agent runtime”拆开，减少非 agent 场景复杂度 | `docs/references/openclaw/openclaw-main/src/agents/simple-completion-runtime.ts:129` `prepareSimpleCompletionModel()`；`:202` `prepareSimpleCompletionModelForAgent()`；`:241` `completeWithPreparedSimpleCompletionModel()` |
| Provider-specific stream compat | 在不污染主循环的前提下，对 OpenAI/Google/MiniMax/Moonshot/OpenRouter 等请求和 stream 做 wrapper | `docs/references/openclaw/openclaw-main/src/agents/pi-embedded-runner/extra-params.ts:443` `applyExtraParamsToAgent()`；同文件顶部可见 `google/openai/minimax/moonshot/proxy` wrappers 的组合导入 |
| Provider 插件扩展示例 | 以 OpenRouter 为例，展示 provider 注册、动态模型、预加载与 stream 包装如何通过插件注入 | `docs/references/openclaw/openclaw-main/extensions/openrouter/index.ts:31` `id: "openrouter"`；`:57` `api.registerProvider(...)`；`:99` `resolveDynamicModel`；`:100` `prepareDynamicModel`；`:106` `wrapStreamFn` |

### 设计要点

#### 1. 模型解析与鉴权解析是两条独立链路

`resolveModelWithRegistry(...)` 只负责回答“这是什么模型、最终该用哪个 model object”。  
`resolveApiKeyForProvider(...)` 只负责回答“这个 provider 最终该用什么认证方式”。

这两个问题在 OpenClaw 中没有耦合在一个函数里，这是它最值得借鉴的地方之一。

#### 2. `resolved model` 的来源支持三层回退

`pi-embedded-runner/model.ts` 的解析顺序很清楚：

1. `resolveExplicitModelWithRegistry()`：先查配置或 registry 中已知模型
2. `resolvePluginDynamicModelWithRegistry()`：再查 provider runtime hook 生成的动态模型
3. `resolveConfiguredFallbackModel()`：最后按 provider config 补一个 fallback model

这使它天然适合：

- 插件型 provider
- 动态模型目录
- 用户只配置 base URL / api / headers 而未完整配置 catalog 的情况

#### 3. request transport policy 是一等公民

`provider-request-config.ts` 不是“给 headers 拼一下就结束”，而是完整承载：

- `baseUrl`
- `headers`
- `auth override`
- `proxy`
- `TLS`
- `allowPrivateNetwork`
- caller/default headers precedence
- attribution headers 保护

这说明 OpenClaw 把“模型请求是如何安全、合规、可路由地发出去”视为单独的架构层，而不是 provider config 的附属字段。

#### 4. provider-specific compat 通过 wrapper 注入，而不是污染主循环

`applyExtraParamsToAgent()` 先计算 `effectiveExtraParams`，再把 provider/plugin 包装器逐层套到 `streamFn` 上。  
主循环不需要知道 Google thinking、OpenAI responses、MiniMax thinking、OpenRouter cache/system headers 等兼容分支。

这比把 provider 特判散落到主循环里更易维护。

#### 5. Simple completion 与 full agent runtime 分离

`simple-completion-runtime.ts` 很值得借鉴。它把很多“只是想单次补全/分类/生成标题”的场景从完整 agent runtime 中剥离出去，避免所有调用都被迫经过复杂的 tool/runtime/session 体系。

### 对 Octopus 最值得借鉴的点

1. 把 `model resolution`、`auth resolution`、`request policy` 拆成三个明确模块。
2. 把 provider transport policy 当成正式配置层，而不是零散地拼在 adapter 中。
3. 把 provider-specific compat 放到 wrapper / hook 层，不污染 runtime turn loop。
4. 对需要“非会话式生成”的场景，增加 simple completion 路径，避免所有请求都走完整 session/tool loop。
5. 为插件型 provider 预留动态模型入口，而不是要求所有模型都先静态进入 catalog。

## 两个参考项目的核心差异

| 维度 | Claude Code | OpenClaw |
| --- | --- | --- |
| 上层协议中心 | `Anthropic Messages` 风格统一协议 | provider-agnostic，更多围绕 `resolved model + streamFn` |
| provider 切换方式 | 主要切 transport/auth backend | registry + auth + transport + plugin hook 多层切换 |
| 主循环归属 | 自己实现完整 `query/tool loop` | 运行时更多委托给 `pi-ai` / `pi-coding-agent` |
| provider 兼容策略 | 在 client/request 层吸收 | 在 `request policy + stream wrapper + plugin runtime` 层吸收 |
| extensibility | 更适合同协议多后端 | 更适合多 provider、多插件、多动态模型 |

## 对 Octopus 的组合式借鉴建议

如果 Octopus 继续沿当前多 provider、多 surface、多 runtime capability 的方向演进，建议采用“Claude Code 的 loop 分层 + OpenClaw 的配置分层”的组合策略：

1. 保持 `runtime turn loop` 独立。
   不要把 provider 特性持续塞进 `executor.rs` 分支内部，尤其不要把鉴权、headers、transport、特殊 payload patch 和 tool loop 混在一起。
2. 增加独立的 `model resolution -> auth resolution -> request policy resolution` 三段。
   这样后续补 `responses / gemini / vendor native / realtime` 时成本会明显更低。
3. 对 provider-specific payload / stream 差异，引入 wrapper 层。
   目标是让 `AssistantEvent` 标准化之前的兼容逻辑停留在 provider boundary，而不是侵入 session runtime。
4. 为轻量生成场景补一条 `simple completion` 路径。
   这类调用不必强制走完整 session、tool loop、approval 主链路。
5. 若未来希望支持插件型模型源，优先借鉴 OpenClaw 的 `resolveDynamicModel / prepareDynamicModel / wrapStreamFn` 插件边界。

## 建议优先阅读的实现入口

若只想快速建立感觉，建议按下面顺序阅读：

### Claude Code

1. `docs/references/claude-code-sourcemap-main/restored-src/src/QueryEngine.ts:184`
2. `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts:219`
3. `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/claude.ts:1017`
4. `docs/references/claude-code-sourcemap-main/restored-src/src/services/api/client.ts:88`
5. `docs/references/claude-code-sourcemap-main/restored-src/src/utils/model/model.ts:92`

### OpenClaw

1. `docs/references/openclaw/openclaw-main/src/agents/pi-embedded-runner/model.ts:605`
2. `docs/references/openclaw/openclaw-main/src/agents/model-auth.ts:409`
3. `docs/references/openclaw/openclaw-main/src/agents/provider-request-config.ts:603`
4. `docs/references/openclaw/openclaw-main/src/agents/pi-embedded-runner/extra-params.ts:443`
5. `docs/references/openclaw/openclaw-main/extensions/openrouter/index.ts:57`

## 最后判断

- 如果 Octopus 近期重点是把现有 provider family 执行链补完整，优先借鉴 `Claude Code` 的 `统一 loop + 统一事件闭环`。
- 如果 Octopus 中期目标是做成真正可扩展的多 provider/runtime 平台，优先借鉴 `OpenClaw` 的 `registry/auth/transport/hook` 解耦。
- 最稳妥的方向不是二选一，而是：
  - 用 Claude Code 的方式守住主循环边界
  - 用 OpenClaw 的方式重构模型解析与 provider 适配边界
