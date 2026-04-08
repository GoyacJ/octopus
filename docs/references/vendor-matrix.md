# 厂商矩阵

更新时间：`2026-04-02`

说明：

## 官方调研快照

### 核心结论

- DeepSeek 官方公开 chat API id 目前仍以 `deepseek-chat`、`deepseek-reasoner` 为主，但官方已提供 Anthropic API 接入说明，因此 catalog 不能继续把 DeepSeek 等同为单一 `openai_chat` 厂商。
- MiniMax 官方文本与 Anthropic 兼容文档已经同时给出 `MiniMax-M2.7`、`MiniMax-M2.5`、`MiniMax-M2.1`、`MiniMax-M2`，并明确推荐 Anthropic SDK，同时保留 OpenAI SDK 兼容路径。
- Moonshot 官方 agent / 编程文档已以 `kimi-k2.5`、`kimi-k2-thinking`、`kimi-k2-thinking-turbo`、`kimi-k2-0905-preview` 为核心 lineup，并暴露 official tools / formula 等 toolset 能力。
- BigModel 官方 GLM-5 文档与能力目录已覆盖 `glm-5`、`glm-5-turbo`、`glm-4.7`，能力目录明确延展到 function calling、context cache、structured output、MCP、联网搜索、批处理。
- Qwen 官方文档显示其官方 surface 已远超当前 `compatible-mode/v1` 的单 chat 视角；至少要纳入 `qwen3-max`、`qwen3-coder-plus`、`qwen3-vl-plus` 以及 image/audio/realtime family。
- Ark/豆包官方文档已经形成 Responses / Files / Context Cache / Image / Video / Vector 的多 surface 体系；Rust 当前只把 Ark 当 chat/responses provider 明显不足。
- OpenAI 截至 `2026-04-02` 的核心 trio 以 `gpt-5.4`、`gpt-5.4-mini`、`gpt-5.4-nano` 为主，同时 Responses surface 通过 toolset metadata 挂载 web search、file search、computer use、MCP 等 built-in tools。
- Google 官方 Gemini API 已不再只是 text/chat；模型页与能力页同时覆盖 `gemini-2.5-pro`、`gemini-2.5-flash`、`gemini-2.5-flash-lite` 以及 Live、TTS、Files、Batch、Context Caching、Imagen 4、Veo 3.1、Lyria 3、Nano Banana。

### 关键官方来源

- DeepSeek: [Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion), [Models/Pricing](https://api-docs.deepseek.com/quick_start/pricing), [Anthropic API](https://api-docs.deepseek.com/guides/anthropic_api)
- MiniMax: [Text API](https://platform.minimaxi.com/docs/api-reference/text-intro), [Anthropic API](https://platform.minimaxi.com/docs/api-reference/text-anthropic-api)
- Moonshot: [Agent Support / Kimi K2.5](https://platform.moonshot.ai/docs/guide/agent-support.en-US), [Official Tools / Formula](https://platform.moonshot.ai/docs/guide/use-formula-tool-in-chatapi)
- BigModel: [GLM-5](https://docs.bigmodel.cn/cn/guide/models/text/glm-5), [Function Calling](https://docs.bigmodel.cn/cn/guide/capabilities/function-calling), [Context Cache](https://docs.bigmodel.cn/cn/guide/capabilities/cache), [MCP](https://docs.bigmodel.cn/cn/guide/capabilities/mcp)
- Qwen: [Claude Code 接入](https://help.aliyun.com/zh/model-studio/claude-code), [Qwen Coder](https://help.aliyun.com/zh/model-studio/qwen-coder), [Anthropic API Messages](https://help.aliyun.com/zh/model-studio/anthropic-api-messages), [模型列表](https://help.aliyun.com/zh/model-studio/getting-started/models)
- Ark/豆包: [Responses API](https://www.volcengine.com/docs/82379/1569618?lang=zh), [Context Cache](https://www.volcengine.com/docs/82379/1602228?lang=zh), [模型推理](https://www.volcengine.com/docs/82379/1330310), [视觉理解/模型能力页](https://www.volcengine.com/docs/82379/1330626?lang=zh)
- OpenAI: [Models](https://developers.openai.com/api/docs/models), [Tools](https://platform.openai.com/docs/guides/tools), [MCP](https://platform.openai.com/docs/guides/mcp)
- Google: [Gemini Models](https://ai.google.dev/gemini-api/docs/models), [Live API](https://ai.google.dev/gemini-api/docs/live), [Text-to-Speech](https://ai.google.dev/gemini-api/docs/text-to-speech), [Context Caching](https://ai.google.dev/gemini-api/docs/context-caching), [Batch API](https://ai.google.dev/gemini-api/docs/batch), [Files API](https://ai.google.dev/gemini-api/docs/files)

## 1. 总览

| provider_id | 官方实际情况 |
| --- | --- |
| `deepseek` | OpenAI Chat + Anthropic compat 双 conversation 面 |
| `minimax` | native text + Anthropic compat + OpenAI compat + 多媒体平台 |
| `moonshot` | conversation + official tools/formula metadata |
| `bigmodel` | conversation + context cache + MCP + web search + batch 能力目录 |
| `qwen` | OpenAI compat + Anthropic compat + image/audio/realtime family |
| `ark` | responses + files + cache + image + video + vector |
| `openai` | responses + chat + files + batch + realtime + built-in tools metadata |
| `google` | conversation + live + tts + files + batch + cache + image + video + music |

## 2. DeepSeek

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `conversation` | `openai_chat` | `request_response` + `sse` | `bearer` | `https://api.deepseek.com` | text input/output、tool calling、JSON、reasoning trace、streaming | [Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion), [Models/Pricing](https://api-docs.deepseek.com/quick_start/pricing) | `2026-04-02` | `["protocol_contract","capability_contract","model_lineup"]` |
| `conversation` | `anthropic_messages` | `request_response` + `sse` | `bearer` | `https://api.deepseek.com/anthropic` | Anthropic SDK / base URL 兼容接入 | [Anthropic API](https://api-docs.deepseek.com/guides/anthropic_api) | `2026-04-02` | `["protocol_contract","compatibility_bridge"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `deepseek-chat` | `deepseek-chat` | `DeepSeek-V3.2` 系列 | `latest_alias` | text、tool calling、JSON、streaming | 公开 id 之一 | [Models/Pricing](https://api-docs.deepseek.com/quick_start/pricing), [Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion) | `2026-04-02` | `["model_lineup","release_line","capability_contract"]` |
| `deepseek-reasoner` | `deepseek-reasoner` | `DeepSeek-V3.2` 系列 | `latest_alias` | reasoning、tool calling、JSON、streaming | 公开 id 之一 | [Models/Pricing](https://api-docs.deepseek.com/quick_start/pricing), [Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion) | `2026-04-02` | `["model_lineup","release_line","capability_contract"]` |

备注：DeepSeek 截至 `2026-04-02` 公开 API id 数量不足，官方公开 chat id 只有 `deepseek-chat` 与 `deepseek-reasoner`；第三个信息位只记录 `underlying_release = DeepSeek-V3.2` 系列，不新增伪造 API id。

## 3. MiniMax

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `conversation` | `vendor_native` | `request_response` + `sse` | `bearer` | `https://api.minimaxi.com` | text / multimodal 对话、流式、tool 调用 | [Text API](https://platform.minimaxi.com/docs/api-reference/text-intro) | `2026-04-02` | `["protocol_contract","capability_contract"]` |
| `conversation` | `anthropic_messages` | `request_response` + `sse` | `bearer` | `https://api.minimaxi.com/anthropic` | 官方推荐 Anthropic SDK 接入 | [Anthropic API](https://platform.minimaxi.com/docs/api-reference/text-anthropic-api) | `2026-04-02` | `["protocol_contract","compatibility_bridge"]` |
| `conversation` | `openai_chat` | `request_response` + `sse` | `bearer` | `https://api.minimaxi.com` | 官方声明 OpenAI SDK 兼容 | [Text API](https://platform.minimaxi.com/docs/api-reference/text-intro) | `2026-04-02` | `["compatibility_bridge"]` |
| `image` / `video` / `audio` | `vendor_native` | `request_response` | `bearer` | `https://api.minimaxi.com` | image、video、voice、music 等平台能力；额外 tool/file 能力通过 metadata 表达 | [MiniMax API 总览](https://platform.minimaxi.com/docs/api-reference/text-intro) | `2026-04-02` | `["capability_contract"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `MiniMax-M2.7` | `MiniMax-M2` | `MiniMax-M2.7` | `stable` | 新一代文本/代码对话，Anthropic compat 明确支持 | 建议作为默认高能力 conversation 模型之一 | [Text API](https://platform.minimaxi.com/docs/api-reference/text-intro), [Anthropic API](https://platform.minimaxi.com/docs/api-reference/text-anthropic-api) | `2026-04-02` | `["model_lineup","capability_contract"]` |
| `MiniMax-M2.5` | `MiniMax-M2` | `MiniMax-M2.5` | `stable` | conversation、tool 调用、结构化输出 |  | [Text API](https://platform.minimaxi.com/docs/api-reference/text-intro), [Anthropic API](https://platform.minimaxi.com/docs/api-reference/text-anthropic-api) | `2026-04-02` | `["model_lineup"]` |
| `MiniMax-M2.1` | `MiniMax-M2` | `MiniMax-M2.1` | `stable` | 中间代稳定版 | 仍应保留 lineage | [Text API](https://platform.minimaxi.com/docs/api-reference/text-intro), [Anthropic API](https://platform.minimaxi.com/docs/api-reference/text-anthropic-api) | `2026-04-02` | `["model_lineup"]` |
| `MiniMax-M2` | `MiniMax-M2` | `MiniMax-M2` | `stable` | 旧一代稳定 lineage | 非默认，但 catalog 应可表达 | [Anthropic API](https://platform.minimaxi.com/docs/api-reference/text-anthropic-api) | `2026-04-02` | `["model_lineup"]` |

## 4. Moonshot

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `conversation` | `openai_chat` | `request_response` + `sse` | `bearer` | `https://api.moonshot.cn/v1` | text、reasoning、tool calling、JSON、streaming | [Agent Support / K2.5](https://platform.moonshot.ai/docs/guide/agent-support.en-US) | `2026-04-02` | `["protocol_contract","capability_contract","model_lineup"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `kimi-k2.5` | `kimi-k2` | `kimi-k2.5` | `stable` | 编程 / agent 优先、tool calling、reasoning、streaming |  | [Agent Support / K2.5](https://platform.moonshot.ai/docs/guide/agent-support.en-US) | `2026-04-02` | `["model_lineup","capability_contract"]` |
| `kimi-k2-thinking` | `kimi-k2` | `kimi-k2-thinking` | `stable` | reasoning 加强版 | 应保留与非 thinking 的模型差异 | [Agent Support / K2.5](https://platform.moonshot.ai/docs/guide/agent-support.en-US) | `2026-04-02` | `["model_lineup"]` |
| `kimi-k2-thinking-turbo` | `kimi-k2` | `kimi-k2-thinking-turbo` | `stable` | reasoning + latency tradeoff | 对默认 fast model 选择有价值 | [Agent Support / K2.5](https://platform.moonshot.ai/docs/guide/agent-support.en-US) | `2026-04-02` | `["model_lineup"]` |
| `kimi-k2-0905-preview` | `kimi-k2` | `kimi-k2-0905-preview` | `preview` | 预览快照 | 不应成为默认 canonical alias | [Agent Support / K2.5](https://platform.moonshot.ai/docs/guide/agent-support.en-US) | `2026-04-02` | `["model_lineup","release_line"]` |

## 5. BigModel

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `conversation` | `openai_chat` | `request_response` + `sse` | `bearer` | `https://open.bigmodel.cn/api/paas/v4` | text、tool calling、JSON / structured output、streaming | [GLM-5](https://docs.bigmodel.cn/cn/guide/models/text/glm-5), [Function Calling](https://docs.bigmodel.cn/cn/guide/capabilities/function-calling) | `2026-04-02` | `["protocol_contract","capability_contract","model_lineup"]` |
| `cache` | `vendor_native` | `request_response` | `bearer` | `https://open.bigmodel.cn/api/paas/v4` | context cache | [Context Cache](https://docs.bigmodel.cn/cn/guide/capabilities/cache) | `2026-04-02` | `["capability_contract"]` |
| `batch` | `vendor_native` | `async_job` | `bearer` | `https://open.bigmodel.cn/api/paas/v4` | 批处理能力；MCP / 联网搜索等通过 capability metadata 表达 | [MCP](https://docs.bigmodel.cn/cn/guide/capabilities/mcp), [GLM-5](https://docs.bigmodel.cn/cn/guide/models/text/glm-5) | `2026-04-02` | `["capability_contract"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `glm-5` | `glm-5` | `glm-5` | `stable` | function calling、structured output、context cache、MCP | 推荐作为默认高能力 GLM | [GLM-5](https://docs.bigmodel.cn/cn/guide/models/text/glm-5), [Function Calling](https://docs.bigmodel.cn/cn/guide/capabilities/function-calling), [Context Cache](https://docs.bigmodel.cn/cn/guide/capabilities/cache), [MCP](https://docs.bigmodel.cn/cn/guide/capabilities/mcp) | `2026-04-02` | `["model_lineup","capability_contract"]` |
| `glm-5-turbo` | `glm-5` | `glm-5-turbo` | `stable` | 低时延 / 轻量路线 | 应进入 fast model 解析 | [GLM-5](https://docs.bigmodel.cn/cn/guide/models/text/glm-5) | `2026-04-02` | `["model_lineup"]` |
| `glm-4.7` | `glm-4.x` | `glm-4.7` | `stable` | 历史稳定 lineage | 仍需作为非默认可解析模型保留 | [GLM-5](https://docs.bigmodel.cn/cn/guide/models/text/glm-5) | `2026-04-02` | `["model_lineup","release_line"]` |

## 6. Qwen

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `conversation` | `openai_chat` | `request_response` + `sse` | `bearer` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | OpenAI compatible conversation | [Claude Code 接入](https://help.aliyun.com/zh/model-studio/claude-code), [模型列表](https://help.aliyun.com/zh/model-studio/getting-started/models) | `2026-04-02` | `["protocol_contract","model_lineup"]` |
| `conversation` | `anthropic_messages` | `request_response` + `sse` | `bearer` | `https://dashscope.aliyuncs.com/api/v2/apps/claude-code-proxy` 或官方 Anthropic compat 面 | Anthropic SDK / Claude Code 接入 | [Anthropic API Messages](https://help.aliyun.com/zh/model-studio/anthropic-api-messages), [Claude Code 接入](https://help.aliyun.com/zh/model-studio/claude-code) | `2026-04-02` | `["protocol_contract","compatibility_bridge"]` |
| `image` | `vendor_native` | `request_response` | `bearer` | `https://dashscope.aliyuncs.com` | image generation | [图像生成 API](https://help.aliyun.com/zh/model-studio/qwen-image-api) | `2026-04-02` | `["capability_contract"]` |
| `audio` / `realtime` | `vendor_native` | `websocket` + `request_response` | `bearer` | `https://dashscope.aliyuncs.com` | 实时语音识别 / 语音合成 / 音频输入输出 | [实时语音识别](https://help.aliyun.com/zh/model-studio/real-time-speech-recognition) | `2026-04-02` | `["capability_contract"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `qwen3-max` | `qwen3` | `qwen3-max` | `stable` | 高能力通用对话 |  | [模型列表](https://help.aliyun.com/zh/model-studio/getting-started/models) | `2026-04-02` | `["model_lineup"]` |
| `qwen3-coder-plus` | `qwen3-coder` | `qwen3-coder-plus` | `stable` | 编程/agent 优化 | Claude Code 接入主推模型之一 | [Claude Code 接入](https://help.aliyun.com/zh/model-studio/claude-code), [Qwen Coder](https://help.aliyun.com/zh/model-studio/qwen-coder) | `2026-04-02` | `["model_lineup","capability_contract"]` |
| `qwen3-vl-plus` | `qwen3-vl` | `qwen3-vl-plus` | `stable` | vision-language、多模态输入 | 体现 Qwen 已超出纯文本 chat | [模型列表](https://help.aliyun.com/zh/model-studio/getting-started/models) | `2026-04-02` | `["model_lineup","capability_contract"]` |

## 7. Ark / 豆包

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `responses` | `openai_responses` | `request_response` + `sse` | `bearer` | `https://ark.cn-beijing.volces.com/api/v3` | 官方 Responses API | [Responses API](https://www.volcengine.com/docs/82379/1569618?lang=zh) | `2026-04-02` | `["protocol_contract"]` |
| `files` | `vendor_native` | `multipart` + `request_response` | `bearer` | `https://ark.cn-beijing.volces.com/api/v3` | Files | [Responses API](https://www.volcengine.com/docs/82379/1569618?lang=zh) | `2026-04-02` | `["protocol_contract","capability_contract"]` |
| `cache` | `vendor_native` | `request_response` | `bearer` | `https://ark.cn-beijing.volces.com/api/v3` | Context Cache | [Context Cache](https://www.volcengine.com/docs/82379/1602228?lang=zh) | `2026-04-02` | `["protocol_contract","capability_contract"]` |
| `image` / `video` / `embeddings` | `vendor_native` | `request_response` + `async_job` | `bearer` | `https://ark.cn-beijing.volces.com/api/v3` | image generation、video generation、vector / embedding | [Responses API](https://www.volcengine.com/docs/82379/1569618?lang=zh), [模型推理](https://www.volcengine.com/docs/82379/1330310) | `2026-04-02` | `["capability_contract","release_line"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `doubao-seed-1.6` | `doubao-seed-1.6` | `doubao-seed-1.6` | `stable` | 通用对话/多模态主线 |  | [模型推理](https://www.volcengine.com/docs/82379/1330310), [视觉理解/模型能力页](https://www.volcengine.com/docs/82379/1330626?lang=zh) | `2026-04-02` | `["model_lineup","release_line"]` |
| `doubao-seed-1.6-thinking` | `doubao-seed-1.6` | `doubao-seed-1.6-thinking` | `stable` | reasoning 强化 | 需与通用版分轨 | [模型推理](https://www.volcengine.com/docs/82379/1330310) | `2026-04-02` | `["model_lineup","release_line"]` |
| `doubao-seed-1.6-flash` | `doubao-seed-1.6` | `doubao-seed-1.6-flash` | `stable` | 低时延路线 | 适合 fast model 解析 | [模型推理](https://www.volcengine.com/docs/82379/1330310) | `2026-04-02` | `["model_lineup","release_line"]` |

## 8. OpenAI

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `conversation` | `openai_chat` | `request_response` + `sse` | `bearer` | `https://api.openai.com/v1` | Chat Completions 兼容会话 | [Models](https://developers.openai.com/api/docs/models) | `2026-04-02` | `["protocol_contract"]` |
| `responses` | `openai_responses` | `request_response` + `sse` | `bearer` | `https://api.openai.com/v1` | 推荐主 surface；挂载 built-in tools | [Models](https://developers.openai.com/api/docs/models), [Tools](https://platform.openai.com/docs/guides/tools) | `2026-04-02` | `["protocol_contract","capability_contract"]` |
| `files` / `batch` / `realtime` / `image` / `audio` | `vendor_native` | `multipart` / `async_job` / `websocket` / `request_response` | `bearer` | `https://api.openai.com/v1` | files、batch、realtime、media generation/input | [Models](https://developers.openai.com/api/docs/models), [Tools](https://platform.openai.com/docs/guides/tools) | `2026-04-02` | `["capability_contract"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `gpt-5.4` | `gpt-5.4` | `gpt-5.4` | `stable` | reasoning、tool use、vision、Responses tools | 核心 trio 之一 | [Models](https://developers.openai.com/api/docs/models) | `2026-04-02` | `["model_lineup","capability_contract"]` |
| `gpt-5.4-mini` | `gpt-5.4` | `gpt-5.4-mini` | `stable` | 小型高性价比路线 | 核心 trio 之一 | [Models](https://developers.openai.com/api/docs/models) | `2026-04-02` | `["model_lineup"]` |
| `gpt-5.4-nano` | `gpt-5.4` | `gpt-5.4-nano` | `stable` | 更轻量路线 | 核心 trio 之一 | [Models](https://developers.openai.com/api/docs/models) | `2026-04-02` | `["model_lineup"]` |

## 9. Google

### Surface Matrix

| surface | protocol_family | transport | auth_strategy | base_url family | capability_matrix 摘要 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `conversation` | `gemini_native` | `request_response` + `sse` | `x_api_key` | `https://generativelanguage.googleapis.com` | Gemini conversation、tool calling、JSON、search grounding | [Gemini Models](https://ai.google.dev/gemini-api/docs/models) | `2026-04-02` | `["protocol_contract","model_lineup","capability_contract"]` |
| `realtime` | `gemini_native` | `websocket` | `x_api_key` | `https://generativelanguage.googleapis.com` | Live API | [Live API](https://ai.google.dev/gemini-api/docs/live) | `2026-04-02` | `["protocol_contract","capability_contract"]` |
| `audio` | `gemini_native` | `request_response` | `x_api_key` | `https://generativelanguage.googleapis.com` | TTS / audio output | [Text-to-Speech](https://ai.google.dev/gemini-api/docs/text-to-speech) | `2026-04-02` | `["protocol_contract","capability_contract"]` |
| `files` / `cache` / `batch` | `gemini_native` | `multipart` / `request_response` / `async_job` | `x_api_key` | `https://generativelanguage.googleapis.com` | Files、Context Caching、Batch | [Files API](https://ai.google.dev/gemini-api/docs/files), [Context Caching](https://ai.google.dev/gemini-api/docs/context-caching), [Batch API](https://ai.google.dev/gemini-api/docs/batch) | `2026-04-02` | `["protocol_contract","capability_contract"]` |
| `image` / `video` / `music` | `gemini_native` | `request_response` + `async_job` | `x_api_key` | `https://generativelanguage.googleapis.com` | Nano Banana、Imagen 4、Veo 3.1、Lyria 3 | [Gemini Models](https://ai.google.dev/gemini-api/docs/models) | `2026-04-02` | `["model_lineup","capability_contract","release_line"]` |

### Latest Model Lineup

| api_model_id | family | underlying_release | track | capability_matrix 摘要 | 备注 | sources[] | last_verified_at | evidence_scope |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `gemini-2.5-pro` | `gemini-2.5` | `gemini-2.5-pro` | `stable` | 高能力 conversation、多模态、tool use |  | [Gemini Models](https://ai.google.dev/gemini-api/docs/models) | `2026-04-02` | `["model_lineup","capability_contract"]` |
| `gemini-2.5-flash` | `gemini-2.5` | `gemini-2.5-flash` | `stable` | 更低时延、多模态、streaming |  | [Gemini Models](https://ai.google.dev/gemini-api/docs/models) | `2026-04-02` | `["model_lineup"]` |
| `gemini-2.5-flash-lite` | `gemini-2.5` | `gemini-2.5-flash-lite` | `stable` | 轻量路线 |  | [Gemini Models](https://ai.google.dev/gemini-api/docs/models) | `2026-04-02` | `["model_lineup"]` |

备注：`Nano Banana`、`Imagen 4`、`Veo 3.1`、`Lyria 3` 属于 Google 的媒体 surface / release line，不应混写成 conversation `api_model_id`；它们应进入 provider 的非 conversation surface 目录，并通过 `surface + protocol_family + capability_matrix` 表达。
