# 11 · 模型系统（Multi-Vendor, Multi-Model）

> 本章定义 Octopus SDK **模型层**的抽象。目标：一次装配、多厂商/多协议可插拔；业务层只与"能力契约（capabilities）+ 模型角色（role）"对话，不与具体厂商 SDK 绑定。
>
> **事实源**：`docs/references/vendor-matrix.md`（每季度更新）是本仓厂商-接口-模型的唯一事实源；本章只定义**如何把矩阵接入 SDK**，不重复罗列模型名。

## 11.1 Why：为什么要多厂商多模型

1. **模型异构场景真实存在**：
   - 主循环用最强模型（Opus / GPT-5 / Gemini 2.5 Pro）
   - 快分类 / 路由 / 摘要用便宜模型（Haiku / Flash / Mini）
   - 视觉分析用多模态强的模型
   - 压缩摘要用长上下文且便宜的模型
   - 子代理可自选档位
2. **规避单厂商风险**：
   - API 配额、价格调整、停服、政策变更
   - 合规/数据驻留要求（例：中国大陆业务需要国内厂商 + 国内机房）
3. **模型快速迭代**：新模型每月发布，硬编码 ID 会过期。
4. **成本与质量可调**：Smart routing / fallback / auxiliary 让每一分钱花在合理位置。

### 参考证据

| 项目 | 支持的 Provider 数量 / 形态 |
|---|---|
| **Claude Code** | 4 种后端：firstParty / Bedrock / Vertex / Foundry；全部回到 Claude 家族 |
| **Hermes Agent** | 15+ provider：openrouter/nous/anthropic/openai-codex/copilot/gemini/zai/kimi-coding/minimax/huggingface/xiaomi/arcee/kilocode/ai-gateway/custom(local) |
| **OpenClaw** | 以 Anthropic 为主；架构允许多模型但实际主推 Claude |
| **Octopus（本仓）** | vendor-matrix.md 列 **9 个厂商**：deepseek/minimax/moonshot/bigmodel/qwen/ark/openai/google + anthropic（本仓 default） |

> 来源：
> - Claude Code `restored-src/src/utils/model/providers.ts`（`APIProvider = firstParty | bedrock | vertex | foundry`）
> - Hermes `cli-config.yaml.example` §"Model Configuration" / §"Auxiliary Models"
> - 本仓 `docs/references/vendor-matrix.md`

## 11.2 顶层抽象：Provider / Surface / Model 三层

直接继承本仓 `vendor-matrix.md` 的事实源模型：

```
                       ┌──────────────────────────┐
                       │  Provider (厂商)          │
                       │  deepseek / openai / ...  │
                       │  认证、账单域、合规域       │
                       └────────────┬─────────────┘
                                    │ 1 : N
                                    ▼
                       ┌──────────────────────────┐
                       │  Surface (接口面)         │
                       │  conversation / responses │
                       │  image / video / audio    │
                       │  files / cache / batch    │
                       │  realtime / embeddings    │
                       └────────────┬─────────────┘
                       protocol_family ∈
                       { openai_chat,
                         openai_responses,
                         anthropic_messages,
                         gemini_native,
                         vendor_native }
                                    │ 1 : N
                                    ▼
                       ┌──────────────────────────┐
                       │  Model (具体模型)          │
                       │  api_model_id / family /  │
                       │  underlying_release /     │
                       │  track                    │
                       └──────────────────────────┘
```

### 三层各自的稳定性

| 层 | 变化速度 | 例子 |
|---|---|---|
| Provider | 慢（以季度/年计）| 新增/下架一个厂商 |
| Surface | 中（以月计）| 新增 "realtime" 面、"cache" 面 |
| Model | 快（以周/月计）| `gpt-5.4` 替换 `gpt-5.3` |

**设计含义**：代码应绑定在 Provider+Surface 层（稳定），不要硬编码 Model ID。

## 11.3 关键数据结构

### 11.3.1 `ProviderDefinition`

```ts
type ProviderDefinition = {
  id: string                          // 'anthropic' / 'openai' / 'deepseek' ...
  displayName: string
  status: 'active' | 'deprecated' | 'experimental'
  auth: AuthScheme                    // 见 §11.8
  baseUrls: BaseUrlFamily[]           // 主 + 合规/区域副本
  surfaces: SurfaceDefinition[]       // 见下
  billing?: {
    currency: string
    pricingSource?: string            // 指向官方价目
  }
  compliance?: {
    dataResidency?: string[]          // 'cn' | 'us' | 'eu' | ...
    sensitiveDataAllowed?: boolean
  }
  sources: string[]                   // 官方文档 URL（参考 vendor-matrix.md）
  lastVerifiedAt: string              // ISO 日期
}
```

### 11.3.2 `SurfaceDefinition`

```ts
type SurfaceDefinition = {
  id: 'conversation' | 'responses' | 'image' | 'video' | 'audio'
     | 'realtime' | 'files' | 'cache' | 'batch' | 'embeddings'
  protocolFamily: ProtocolFamily      // 'openai_chat' | 'openai_responses'
                                      // | 'anthropic_messages' | 'gemini_native'
                                      // | 'vendor_native'
  transport: TransportKind[]          // 'request_response' | 'sse' | 'websocket'
                                      // | 'async_job' | 'multipart'
  authStrategy: 'bearer' | 'x_api_key' | 'oauth' | 'aws_sigv4' | 'gcp_adc'
  baseUrl: string
  capabilities: CapabilityMatrix
}
```

### 11.3.3 `ModelDefinition`

```ts
type ModelDefinition = {
  providerId: string                  // 所属 provider
  surfaceId: SurfaceDefinition['id']  // 所属 surface
  apiModelId: string                  // 发请求用的字符串（e.g. 'claude-opus-4-6'）
  family: string                      // 'claude-opus' / 'gpt-5.4' / 'kimi-k2' ...
  underlyingRelease: string           // 最底层发布版本
  track: 'stable' | 'preview' | 'latest_alias' | 'deprecated'
  capabilities: CapabilityMatrix
  contextWindow: {
    maxInputTokens: number
    maxOutputTokens: number
    supports1M?: boolean              // Claude Opus 1M variant 等
  }
  pricing?: ModelPricing              // per 1M input/output tokens
  cutoffDate?: string
  aliases?: string[]                  // 用户层短名
  deprecatedAt?: string
  sunsetAt?: string
  sources: string[]
  lastVerifiedAt: string
}
```

### 11.3.4 `CapabilityMatrix`

```ts
type CapabilityMatrix = {
  input: {
    text: boolean
    image: boolean
    audio: boolean
    video: boolean
    pdf: boolean
    fileRef: boolean                  // file_id 参考
  }
  output: {
    text: boolean
    image: boolean
    audio: boolean
    streaming: boolean                // SSE 流式
  }
  features: {
    toolUse: boolean                  // function calling
    parallelToolUse: boolean
    structuredOutput: boolean         // JSON schema 约束
    reasoning: boolean                // 扩展思考
    reasoningVisible: boolean         // reasoning traces 可输出
    contextCache: boolean             // 服务端 cache
    promptCache: boolean              // Anthropic-style prompt caching
    webSearch: boolean                // 内置搜索
    computerUse: boolean
    mcpBuiltIn: boolean               // 内置 MCP 客户端
    batch: boolean
    fineTuning: boolean
  }
  safety: {
    moderationApi: boolean
  }
}
```

> 来源：直接来自 `vendor-matrix.md` 的"capability_matrix 摘要"字段，统一 schema 化。

## 11.4 Model Catalog：目录与装载

**Catalog** = ProviderDefinition[] + 索引。

### 11.4.1 三种目录来源（优先级由高到低）

| 来源 | 位置 | 用途 |
|---|---|---|
| **用户/工作区覆写** | `config/runtime/*.json` → `models.*` | 私有模型、本地 server、用户自定义 alias |
| **内置快照** | 本 SDK 内嵌（从 `vendor-matrix.md` 派生） | 离线可用的基线目录；SDK 发布时固化 |
| **远端目录** | 可选第三方（如 `models.dev`） | 运行时补新模型；需网络 |

**合并**：deep-merge + alias 冲突按**近者优先**（user > workspace > built-in > remote）。

> 参考：
> - Hermes `cli-config.yaml` §"Model Aliases" 使用 `models.dev` 作为远端目录
> - Claude Code `getModelCapability()` 使用本地 `cache/model-capabilities.json`（`restored-src/src/utils/model/modelCapabilities.ts`）

### 11.4.2 目录同步

- 默认**不访问网络**（本地 snapshot 足够）
- 用户主动 `octopus models refresh` 触发一次远端同步
- 同步结果写入 `data/main.db`（可查）+ `runtime/model-cache.json`（快照）
- **绝不**在会话启动时阻塞于远端 API

### 11.4.3 目录查询 API

```ts
catalog.listProviders(): ProviderDefinition[]
catalog.getProvider(id): ProviderDefinition | undefined
catalog.listModels(filter?: {
  providerId?: string
  surfaceId?: string
  family?: string
  track?: Track
  capability?: (keyof CapabilityMatrix)[]
}): ModelDefinition[]
catalog.resolve(reference: string): ResolvedModel
  // reference 可以是 alias / modelId / '<provider>/<model>'
```

## 11.5 Model Reference 解析（Resolution）

### 11.5.1 优先级链（来自 Claude Code + Hermes 融合）

```
1. 当前 turn 的显式 override          (tool 或 subagent 指定)
2. Session 级 override                 (/model 切换)
3. Startup flag / env var              (--model / OCTOPUS_MODEL / ANTHROPIC_MODEL)
4. Agent Definition frontmatter.model  (静态声明)
5. Project runtime config (model)
6. Workspace runtime config (model)
7. User runtime config (model)
8. SDK 内置默认（role-based）
```

> 来源：Claude Code `restored-src/src/utils/model/model.ts`（`getUserSpecifiedModelSetting`、`getMainLoopModel`）；Hermes `cli-config.yaml` §"Model Configuration"；本仓 `AGENTS.md` §Runtime Config（user < workspace < project 合并规则）。

### 11.5.2 User-Level Aliases（自定义短名）

```yaml
model_aliases:
  opus:
    provider: anthropic
    model: claude-opus-4-6
  glm:
    provider: bigmodel
    model: glm-5
  local-qwen:
    provider: custom
    base_url: "http://localhost:11434/v1"
    model: "qwen3.5:72b"
```

**解析 alias**：先查 user aliases → SDK 内置 aliases（见 §11.6）→ catalog 模糊匹配（family alias）。

> 来源：Hermes `cli-config.yaml` §"Model Aliases"；Claude Code `restored-src/src/utils/model/aliases.ts`（内置 `sonnet`/`opus`/`haiku`/`best`/`opusplan` 等）。

### 11.5.3 Family Aliases（通配）

短名不带版本时，按 **family 通配** 选最新 stable：

- `opus` → `catalog.listModels({family:'claude-opus', track:'stable'})[latest]`
- `sonnet` → `catalog.listModels({family:'claude-sonnet', track:'stable'})[latest]`
- `haiku` → 同理
- `gpt-5` → 选当前家族最高版本

> 来源：Claude Code `MODEL_FAMILY_ALIASES = ['sonnet','opus','haiku']`（`aliases.ts`）。

### 11.5.4 Canonical Name

不同 provider/region 下，同一模型可能有多种 ID：

- `claude-opus-4-6`（firstParty）
- `anthropic.claude-opus-4-6-v1:0`（Bedrock）
- `us.anthropic.claude-opus-4-6-v1:0`（Bedrock cross-region）

**canonicalize**：统一映射为 `claude-opus-4-6`。用于：

- 定价（一张表对多种写法）
- 日志聚合
- 能力查询

> 来源：Claude Code `firstPartyNameToCanonical` + `getCanonicalName`（`restored-src/src/utils/model/model.ts`）。

## 11.6 模型角色（Model Roles）

业务代码不应写 `model_id`，而应指定**角色**，由 SDK 根据策略映射到具体模型。

### 11.6.1 标准角色集

| Role | 典型用途 | 默认候选 |
|---|---|---|
| `main` | Agent 主循环 | Opus / GPT-5 / Gemini Pro |
| `fast` | 分类、路由、小判定 | Haiku / GPT-5 Nano / Flash Lite |
| `best` | 需要最强推理时显式升档 | Opus(1M) / GPT-5 max-reasoning |
| `plan` | Plan Mode 专用 | Opus / reasoning-enhanced 版本 |
| `compact` | 上下文压缩摘要 | Haiku / Flash / Mini |
| `vision` | 图像分析 | 支持多模态的主模型或 Vision-specialized |
| `web_extract` | 网页摘要 | Flash / Haiku |
| `embedding` | 向量化 | 各厂商 embeddings surface |
| `rerank` | 排序 | 可选 |
| `eval` | Evaluator subagent | main 级别或更强 |
| `subagent_default` | 子代理默认 | 继承 parent 或 `sonnet` 类 |

### 11.6.2 角色到模型的映射（Role Resolver）

```ts
roleResolver.resolve('main', { providerPreference?, capabilitiesRequired? })
  → ResolvedModel
```

**解析算法**：

```
1. 读 runtime config 的 roles.<role>.model（用户显式映射）
2. 若无：按 role → family 默认表（如 main → claude-opus / openai-gpt-5 / gemini-pro）
3. 叠加 providerPreference（用户或 agent definition 指定的 provider 列表）
4. 过滤 capabilitiesRequired（例：vision 角色过滤 input.image=true）
5. 选 track=stable 且 status=active 的最新版
6. fallback 到内置默认
```

### 11.6.3 Plan-aware 角色切换（Claude Code 的 `opusplan`）

```ts
// 在 plan mode 下自动升档到更强的推理模型
runtimeModelSelector.pickForTurn({
  role: 'main',
  permissionMode: 'plan'  // → 若配置了 plan 专用模型则用之
})
```

> 来源：Claude Code `getRuntimeMainLoopModel`、`opusplan` 机制（`model.ts`）。

### 11.6.4 Subagent Inherit

- 子代理默认 `model: 'inherit'`
- Agent Definition 可指定具体 alias 或 family
- 家族别名**不降级**：父代理 Opus 4.6，子代理声明 `opus` → 继承 4.6 而不是 catalog default

> 来源：Claude Code `restored-src/src/utils/model/agent.ts`（`getAgentModel`、`aliasMatchesParentTier`）。

## 11.7 协议适配层（Protocol Adapter）

### 11.7.1 为什么需要

不同 `protocol_family` 的请求/响应 **schema 不同**：

| protocol_family | request | response | 备注 |
|---|---|---|---|
| `openai_chat` | `/v1/chat/completions` | `choices[].message.tool_calls[]` | 传统 |
| `openai_responses` | `/v1/responses` | 纳入 built-in tools、reasoning | 新规范 |
| `anthropic_messages` | `/v1/messages` | `content[]` blocks、`tool_use` | 最强的 tool_use 支持 |
| `gemini_native` | `models:generateContent` | `candidates[].content.parts[]` | 多模态 native |
| `vendor_native` | 厂商私有 | 厂商私有 | 用作 fallback |

### 11.7.2 统一中立中间表示（Canonical Message IR）

SDK 内部用**中立 IR**传递消息，适配层在边界双向翻译：

```ts
type CanonicalMessage =
  | { role: 'system',    content: ContentBlock[] }
  | { role: 'user',      content: ContentBlock[] }
  | { role: 'assistant', content: ContentBlock[], toolUses?: ToolUse[] }
  | { role: 'tool',      toolUseId: string, content: ContentBlock[] }

type ContentBlock =
  | { type: 'text',           text: string }
  | { type: 'image',          source: ImageSource }
  | { type: 'audio',          source: AudioSource }
  | { type: 'file_ref',       fileId: string }
  | { type: 'thinking',       signature: string }  // reasoning trace

type ToolUse = { id, name, input, cache_control? }
```

### 11.7.3 Adapter 契约

```ts
interface ProtocolAdapter {
  id: ProtocolFamily
  request(params: {
    model: string
    messages: CanonicalMessage[]
    tools?: ToolSpec[]
    responseFormat?: ResponseFormat
    thinking?: ThinkingConfig
    cacheControl?: CacheControlStrategy
    maxTokens?: number
    temperature?: number
    stream?: boolean
    signal?: AbortSignal
  }): AsyncIterable<CanonicalStreamEvent>
  mapError(vendorError: unknown): CanonicalError
}
```

每个 protocol_family 一个 adapter 实现：`AnthropicMessagesAdapter` / `OpenAIChatAdapter` / `OpenAIResponsesAdapter` / `GeminiNativeAdapter` / `VendorNativeAdapters...`

### 11.7.4 能力降级

若业务请求用到目标模型**不支持**的能力（如 `structuredOutput` 或 `parallelToolUse`），adapter 有三种策略（按 severity）：

| 降级策略 | 触发条件 | 行为 |
|---|---|---|
| `reject` | 默认 | 抛 `CapabilityUnsupportedError` |
| `emulate` | 业务显式允许 | 用 prompt instruction 模拟（例如 JSON schema 变成 "请按此 schema 输出" + post-parse） |
| `downgrade` | 业务显式允许 | 自动切到同 role 下支持该能力的模型 |

## 11.8 认证（Auth）

### 11.8.1 多凭据类型

| Scheme | 示例 | 安全注意 |
|---|---|---|
| `api_key_header` | `Authorization: Bearer ...` | 最常见；vault 管理 |
| `api_key_query` | `?key=...`（Gemini） | URL 日志隐患；尽量避免 |
| `x_api_key` | `x-api-key: ...`（Anthropic） | 同上 |
| `oauth` | Claude.ai OAuth / Copilot OAuth | refresh token 管理 |
| `aws_sigv4` | Bedrock | 走 aws-sdk；IAM 托管 |
| `gcp_adc` | Vertex | Application Default Credentials |
| `azure_ad` | Foundry | DefaultAzureCredential 链式 |

> 来源：Claude Code `restored-src/src/utils/auth.ts` + `services/api/client.ts`（4 种后端并存）；Hermes `cli-config.yaml`（十余种 provider 凭据各自对应 env var）。

### 11.8.2 凭据不进沙箱

严格遵循 §6.8：

- 凭据存入 vault / OS keychain；**不写入** runtime config 明文
- 出站请求经凭据注入代理；沙箱进程无凭据
- `runtime_config` 只存**引用**（"secret ref"）而非明文

> 来源：本仓 `AGENTS.md` §"Sensitive config values must not be written back..." + [Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Credentials never enter the sandbox"。

### 11.8.3 多区域 / 合规切换

Provider 可拥有多 `baseUrl`（区域副本）：

```yaml
providers:
  anthropic:
    baseUrls:
      - region: us,       url: https://api.anthropic.com
      - region: eu,       url: https://api-eu.anthropic.com
  minimax:
    baseUrls:
      - region: global,   url: https://api.minimaxi.com
      - region: cn,       url: https://api.minimax.chat
```

按用户或工作区的 `dataResidency` 自动选择。

## 11.9 路由策略（Routing）

### 11.9.1 静态路由（声明式）

```yaml
roles:
  main:      { provider: anthropic, model: claude-opus-4-6 }
  fast:      { provider: anthropic, model: claude-haiku-4-5 }
  compact:   { provider: google,    model: gemini-2.5-flash }
  vision:    { provider: openai,    model: gpt-5.4 }
```

### 11.9.2 Smart Routing（按复杂度）

```yaml
smart_routing:
  enabled: true
  rules:
    - when: "turn.input_chars < 160 and turn.input_words < 28"
      use: { role: fast }
    - default:
      use: { role: main }
```

> 来源：Hermes `cli-config.yaml` §"Smart Model Routing"。

### 11.9.3 OpenRouter / AI Gateway 风格（Provider Routing）

对"第三方网关型" provider（openrouter/ai-gateway），支持次级偏好：

```yaml
providers:
  openrouter:
    provider_routing:
      sort: "throughput"       # "price" | "throughput" | "latency"
      only: ["anthropic", "google"]
      ignore: ["deepinfra"]
      order: ["anthropic", "google"]
      require_parameters: true
      data_collection: "deny"
```

> 来源：Hermes `cli-config.yaml` §"OpenRouter Provider Routing"。

### 11.9.4 Fallback（级联回退）

```yaml
roles:
  main:
    primary:   { provider: anthropic, model: claude-opus-4-6 }
    fallbacks:
      - { provider: openai,  model: gpt-5.4 }
      - { provider: bigmodel, model: glm-5 }
    triggers:
      - overloaded_error
      - 5xx
      - prompt_too_long        # 此种情况需先 compact
      - model_deprecated
```

**Fallback 规则**：

- 按 triggers 匹配错误
- 5xx/overloaded：指数退避 N 次 → 切 fallback
- 保持**消息语义等价**（canonical IR 使 fallback 可跨协议）

> 来源：Claude Code `FallbackTriggeredError` + `fallbackModel`（`restored-src/src/services/api/withRetry.ts` + `query.ts`）。

### 11.9.5 不做无声降级（Silent Degradation）

- fallback 触发**必须**写 `event.model_fallback`
- 若 fallback 后能力矩阵弱于 primary，必须在响应里附 `degradedCapabilities[]`
- 业务可据此决定是否提示用户 / 拒绝

## 11.10 Auxiliary Models（辅助模型）

除了 `main` / `fast`，以下**非核心路径**可用独立模型：

| Auxiliary | 用途 | 建议 |
|---|---|---|
| `vision` | `vision_analyze` 工具 / 浏览器截图理解 | Flash / GPT-5 Mini（成本低） |
| `web_extract` | 把长网页摘要给主代理 | Flash / Haiku |
| `compact` | 上下文压缩摘要生成 | 长上下文 + 廉价（Flash） |
| `session_search` | FTS 结果摘要 | Flash |
| `classifier` | Auto Mode 的决策分类器 | 超小模型 |

设计：

- 每个 auxiliary 独立的 `(provider, model)` 配置
- 默认 `auto` → SDK 按可用凭据选最便宜候选
- 用户可覆写

> 来源：Hermes `cli-config.yaml` §"Auxiliary Models (Advanced — Experimental)"（vision / web_extract / compression）。

## 11.11 Context Window Discovery

### 11.11.1 Why

不同模型/provider 对 `max_input_tokens` / `max_output_tokens` 值不一致；硬编码会过时。

### 11.11.2 来源优先级

```
1. runtime config 显式 override
2. catalog 内 ModelDefinition.contextWindow
3. provider 的 `/models` endpoint 发现（若有）
4. 保守默认（128k input / 8k output）
```

> 来源：Claude Code `getModelCapability()` 从本地缓存 + 动态 fetch（`modelCapabilities.ts`）；Hermes 自动检测 + override。

### 11.11.3 使用场景

- Compaction 触发阈值（% of maxInputTokens）
- 请求校验（避免发出注定 400 的 prompt）
- `[1m]` 变体（Claude Opus 1M context）感知

## 11.12 Prompt Caching / Context Cache

### 11.12.1 两种模式

| 模式 | 表现形式 | 支持 provider | 注意 |
|---|---|---|---|
| **Prompt Caching**（客户端声明） | 请求里打 `cache_control` 标记；服务端自动复用 | Anthropic、部分厂商 | 4 个断点；break 点固定 |
| **Context Cache**（服务端对象） | 预先创建 cache 对象，后续请求 ref 它 | Google、Ark、BigModel | 有 lifetime；需要管理 id |

### 11.12.2 SDK 契约

```ts
adapter.request({
  ...
  cacheControl: {
    strategy: 'prompt_caching' | 'context_cache_object' | 'none'
    breakpoints?: 'system' | 'tools' | 'first_user' | 'last_user'
    cacheObjectId?: string
  }
})
```

Adapter 把 `strategy` 翻译成各 protocol 的具体字段。

### 11.12.3 稳定性与治理

**必须遵守** `C1. Prompt Cache 稳定性`（见 `README.md` §4）：

- 工具顺序确定
- 历史 turn 不改
- 只在尾部 append 或 compaction

违反会：cache miss → 成本暴涨 + 延迟上升。

> 来源：[Anthropic Prompt Caching](https://docs.claude.com/en/docs/build-with-claude/prompt-caching)；`vendor-matrix.md` 显示 Google / Ark / BigModel 各自支持不同的 context cache 形式；OpenClaw `CLAUDE.md`、Hermes `AGENTS.md` 都单列此约束。

## 11.13 Token 计量与预算

### 11.13.1 Token 源

| 来源 | 精度 | 用法 |
|---|---|---|
| **服务端响应 `usage`** | 权威 | 计费、budget 判定 |
| **客户端 tokenizer 估算** | 近似 | pre-call 校验，避免超窗 |

> Hermes `cli-config.yaml` 明示："Tracks actual token usage from API responses (not estimates)"。不要自己估，用响应真值。

### 11.13.2 Budget Integration

- §1.9 定义的 `taskBudget.total` 接到 **实际 tokens**（`response.usage.total_tokens`）
- 跨模型统计：各模型 usage 单位可能不同（`total_tokens` vs `input_tokens + output_tokens`），adapter 负责归一化
- 跨 provider 计费：统一换算为 "USD cents" 落 Session（配 `ModelPricing`）

## 11.14 本地模型（Local Servers）

### 11.14.1 支持的 Runtime

| Runtime | 协议 | base_url 默认 | auth |
|---|---|---|---|
| **Ollama** | `openai_chat` 兼容 | `http://localhost:11434/v1` | 无（或 ignored） |
| **LM Studio** | `openai_chat` 兼容 | `http://localhost:1234/v1` | 无 |
| **vLLM** | `openai_chat` 兼容 | `http://localhost:8000/v1` | 可选 token |
| **llama.cpp server** | `openai_chat` 兼容 | `http://localhost:8080/v1` | 可选 |
| **Ollama Cloud** | `openai_chat` 兼容 | `https://ollama.com/v1` | `OLLAMA_API_KEY` |

### 11.14.2 设计

- Provider id = `custom` + user-chosen alias
- Surface 默认只暴露 `conversation`（除非运行时自发现支持 embeddings 等）
- Capability Matrix 由用户显式声明或自动探测（本地 `/v1/models`）
- **无凭据注入代理**：本地连接通常 loopback；若跨机器则复用代理层

> 来源：Hermes `cli-config.yaml` §"Local servers (LM Studio, Ollama, vLLM, llama.cpp)"。

## 11.15 模型生命周期治理

### 11.15.1 track 与状态机

```
preview ──┬── stable ── deprecated ── sunset
          └── latest_alias ─────────── sunset
```

- **preview**：公测；**不可作为默认 canonical alias**（防止用户"躺枪" breaking change）
- **stable**：正式；允许 alias、auto-select
- **latest_alias**：指向家族最新（如 `deepseek-chat` 永远 = 当前最新 V3.x）
- **deprecated**：废弃窗口期；仍可用但触发警告
- **sunset**：下线；调用直接失败并建议替代

> 来源：`vendor-matrix.md` 的 `track` 字段约定（stable/preview/latest_alias）。

### 11.15.2 Allowlist

```yaml
models:
  allowlist:
    - family: claude-opus
    - family: claude-sonnet
    - apiModelId: gpt-5.4
    - apiModelId: gpt-5.4-mini
  denylist:
    - track: preview    # 禁止 session 自动选 preview 版本
    - status: deprecated
```

> 来源：Claude Code `restored-src/src/utils/model/modelAllowlist.ts`（`isModelAllowed`）。

### 11.15.3 弃用提醒

- SDK 启动时检查 default model 是否仍 active
- 模型 deprecated 时在 UI/CLI 提示 + 给出迁移建议（`catalog.suggestReplacement(modelId)`）

## 11.16 可观测性（Model-specific）

除通用事件外（见 `09-observability-eval.md`），针对模型层采集：

- `model_resolved`：本 turn 选了哪个 `(provider, surface, model)`（含解析路径）
- `model_fallback`：fallback 触发原因 + from/to
- `model_degraded`：能力降级（如 `emulate` 路径触发）
- `model_cache_hit_ratio`：prompt cache 命中
- `model_usage`：真实 token 使用 + 估算成本
- `model_capability_mismatch`：请求了不支持能力

## 11.17 对 Octopus 的落地约束

- **事实源**：`docs/references/vendor-matrix.md` → 派生 built-in catalog，启动时加载，**不在运行时联网同步**；主动 refresh 命令才走网络
- **配置**：所有 model 相关配置遵循本仓 `AGENTS.md` §Runtime Config 分层（user < workspace < project）；**不落 main.db 作为权威**
- **Session 快照**：每个 session 必须记录 `modelSnapshot: { role → (provider, model, version) }` 进入 `config_snapshot`
- **Secret 存储**：API key / OAuth 存 OS keychain（macOS Keychain / Windows Credential Manager / libsecret）+ 配置里只放引用；符合本仓"Sensitive config values must not be written back..."
- **多 Host 一致**：Tauri 宿主与 Browser 宿主必须暴露**相同的** `listProviders` / `listModels` / `resolveModel` adapter 合约（本仓 `AGENTS.md` §Host consistency rule）
- **前端 UI**：`/settings/models` 界面用 `UiDataTable`/`UiSelect` 组件呈现 catalog（遵循 `docs/design/DESIGN.md` + `@octopus/ui`）；**不**允许 business 页直 fetch 模型列表
- **API 契约**：若模型查询需要通过 `/api/v1/*` 暴露给前端，按 `docs/api-openapi-governance.md` 流程在 `contracts/openapi/src/**` 定义
- **Provider 注册**：所有 Provider（含文中 9 厂商）必须通过 Plugin Registry 暴露的 `api.registerProvider(...)` 注册；核心**不**对 provider id 做 switch，业务层也**不**得在 Protocol Adapter 之外做分支；built-in provider 与第三方 provider 走**同一条**扩展点（见 [12 §12.3 扩展点全景](./12-plugin-system.md)、[12 §12.8.4 Register](./12-plugin-system.md) 与 [12 §12.16 实施优先级](./12-plugin-system.md)）

## 11.18 反模式

| 反模式 | 症状 | 纠正 |
|---|---|---|
| **硬编码 model_id** | 新模型发布后到处改 | 用 role + catalog |
| **在业务层写 provider switch** | `if provider==='anthropic' else if ...` | 走 Protocol Adapter |
| **凭据放配置明文** | 泄漏、审计失败 | keychain + 引用 |
| **运行时改工具顺序** | prompt cache 击穿 | deterministic ordering |
| **fallback 默默降级** | 用户以为用的是 Opus，实际是 Flash | 必发 `model_fallback` 事件 |
| **无 canonical naming** | 一个模型在日志里 5 种写法 | `getCanonicalName` 归一化 |
| **目录实时联网** | session 启动被外网拖死 | 启动用本地 snapshot |
| **subagent 降级继承** | 父 Opus 4.6 子代理 opus-alias 落到目录 default | `aliasMatchesParentTier` 继承父版本 |

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| 本仓 `docs/references/vendor-matrix.md` | **事实源**：9 厂商 × surface × protocol_family × capability_matrix × model lineup |
| 本仓 `AGENTS.md` §Runtime Config / §Persistence Governance | 配置分层、secret 存储、host 一致性 |
| Claude Code restored src `utils/model/` 全系 | providers / aliases / canonical name / allowlist / agent model inherit / 1M context / modelCapabilities 发现 |
| Claude Code restored src `services/api/client.ts` | 4 种后端 client 切换（firstParty/Bedrock/Vertex/Foundry） |
| Hermes `cli-config.yaml.example` §Model Configuration/Routing/Auxiliary | 15+ provider、OpenRouter routing、Smart routing、Auxiliary models、Model Aliases、Local servers |
| Hermes `AGENTS.md` §"Prompt caching must not break" | 缓存稳定性约束 |
| OpenClaw `CLAUDE.md` §"Prompt Cache Stability" | 同上约束 |
| [MCP 官网](https://modelcontextprotocol.io) | MCP 协议（纳入工具层） |
| [Anthropic Prompt Caching](https://docs.claude.com/en/docs/build-with-claude/prompt-caching) | prompt caching 规范 |
| [Models.dev](https://models.dev) | 可选第三方 model catalog |
| [OpenRouter](https://openrouter.ai) | 第三方网关 + provider routing 规范 |
