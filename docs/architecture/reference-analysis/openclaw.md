# OpenClaw Reference Analysis

Scope: only `docs/references/openclaw-main` was analyzed. This document records observed facts, bounded inferences, and migration-relevant patterns. It does not design Octopus.

## 1. 项目定位

- 观察：OpenClaw 定位是运行在用户自有设备上的 personal AI assistant。Gateway 被明确定义为 control plane，产品核心是 assistant 本身。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/VISION.md`。
- 观察：OpenClaw 面向多聊天渠道、多设备节点和本地可视 Canvas。README 同时列出 WhatsApp、Telegram、Slack、Discord、Signal、WebChat、macOS、iOS、Android 等入口。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/docs/channels/index.md`、`docs/references/openclaw-main/docs/concepts/architecture.md`。
- 观察：Gateway 可以作为常驻 daemon 运行；companion apps 是可选增强，不是基础体验的前提。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/docs/gateway/index.md`。
- 推断：OpenClaw 的核心产品边界是“单用户、本地优先、跨渠道 assistant”，不是企业多租户 agent platform。依据是 README 的 personal assistant 定位和安全文档的一用户信任模型。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。

## 2. 技术栈

- 观察：项目是 Node.js + TypeScript + ESM 包。`package.json` 声明 `type: "module"`、`main: "dist/index.js"`、CLI bin `openclaw -> openclaw.mjs`。证据：`docs/references/openclaw-main/package.json`、`docs/references/openclaw-main/openclaw.mjs`。
- 观察：运行时依赖包括 `@mariozechner/pi-agent-core`、`@mariozechner/pi-ai`、`@mariozechner/pi-coding-agent`、`express`、`ws`、`typebox`、`zod`、`openai`、`sqlite-vec`、`@modelcontextprotocol/sdk`。证据：`docs/references/openclaw-main/package.json`。
- 观察：Control UI 是 Vite + Lit SPA，并通过同一 Gateway WebSocket 直连控制面。证据：`docs/references/openclaw-main/docs/web/control-ui.md`。
- 观察：移动端和桌面 companion app 也在仓库脚本中体现。`package.json` 有 Android Gradle、iOS Xcode、Swift 相关脚本。证据：`docs/references/openclaw-main/package.json`。
- 观察：OpenClaw 明确选择 TypeScript 是因为项目主要做 orchestration、protocol、tools、integrations，并希望保持 hackable。证据：`docs/references/openclaw-main/VISION.md`。

## 3. 主要入口点

- 观察：包级 CLI 入口是 `openclaw.mjs`。它检查 Node 版本，启用 Node compile cache，然后加载 build 后的 `dist/entry.js` 或 `.mjs`。证据：`docs/references/openclaw-main/openclaw.mjs`。
- 观察：`src/index.ts` 是 legacy direct file entrypoint；作为 main 运行时加载 `src/cli/run-main.ts`，作为库使用时延迟导出 `library.ts` 的能力。证据：`docs/references/openclaw-main/src/index.ts`。
- 观察：`src/cli/run-main.ts` 负责 CLI 环境规范化、dotenv、proxy、运行时检查、命令路由和 program 构建。证据：`docs/references/openclaw-main/src/cli/run-main.ts`。
- 观察：核心 CLI 命令包括 `setup`、`onboard`、`configure`、`message`、`agent`、`agents`、`status`、`health`、`sessions`、`tasks` 等。证据：`docs/references/openclaw-main/src/cli/program/core-command-descriptors.ts`。
- 观察：Gateway 进程入口的核心函数是 `startGatewayServer`。它加载配置、认证、插件、通道管理器、运行态 HTTP/WS server，并在 attach 后启动 sidecar、channels、plugin services 等。证据：`docs/references/openclaw-main/src/gateway/server.impl.ts`。

## 4. Gateway / control plane 架构

- 观察：Gateway 是单个长生命周期进程，拥有 messaging surfaces、控制面 RPC、session、tools、events。控制客户端和节点都通过 WebSocket 连接。证据：`docs/references/openclaw-main/docs/concepts/architecture.md`、`docs/references/openclaw-main/docs/gateway/index.md`。
- 观察：Gateway 使用单端口复用 WebSocket RPC、HTTP API、OpenAI-compatible endpoints、Control UI、hooks。默认 bind 是 loopback。证据：`docs/references/openclaw-main/docs/gateway/index.md`、`docs/references/openclaw-main/src/gateway/server-http.ts`。
- 观察：WebSocket 首帧必须是 `connect`。握手携带 role、scopes、caps、commands、permissions、auth、device identity。非 JSON 或非 connect 首帧会被关闭。证据：`docs/references/openclaw-main/docs/concepts/architecture.md`、`docs/references/openclaw-main/docs/gateway/protocol.md`、`docs/references/openclaw-main/src/gateway/server/ws-connection.ts`。
- 观察：Gateway 协议以 TypeBox schema 为源，生成 JSON Schema 和 Swift 模型。证据：`docs/references/openclaw-main/docs/concepts/architecture.md`、`docs/references/openclaw-main/docs/gateway/protocol.md`。
- 观察：副作用方法需要 idempotency key。`send`、`message.action`、`chat.send`、`agent` 路径都有 dedupe 或 in-flight 缓存逻辑。证据：`docs/references/openclaw-main/docs/gateway/protocol.md`、`docs/references/openclaw-main/src/gateway/server-methods/send.ts`、`docs/references/openclaw-main/src/gateway/server-methods/chat.ts`、`docs/references/openclaw-main/src/gateway/server-methods/agent.ts`。
- 观察：公开 Gateway method list 由 core methods 加 channel plugin methods 组成，事件也集中登记。证据：`docs/references/openclaw-main/src/gateway/server-methods-list.ts`。
- 推断：Gateway 是控制面、策略面和运行态协调器的合体。Agent 执行、通道生命周期、HTTP surfaces、WS clients、plugins 都围绕它收敛。证据：`docs/references/openclaw-main/src/gateway/server.impl.ts`、`docs/references/openclaw-main/docs/gateway/index.md`。

## 5. Channel adapter 机制

- 观察：每个 channel 都通过 Gateway 连接；文本能力统一存在，媒体和 reaction 能力因 channel 变化。多 channel 可同时运行，并按 chat 路由。证据：`docs/references/openclaw-main/docs/channels/index.md`。
- 观察：`ChannelPlugin` 是主要 adapter contract，包含 config、setup、pairing、security、gatewayMethods、lifecycle、allowlist、streaming、threading、messaging、agentPrompt、directory、resolver、actions、heartbeat、agentTools 等能力位。证据：`docs/references/openclaw-main/src/channels/plugins/types.plugin.ts`。
- 观察：`createChannelManager` 管理 channel account runtime、start/stop、manual stop、health monitor、backoff restart、runtime snapshot，并把 plugin gateway lifecycle 纳入统一管理。证据：`docs/references/openclaw-main/src/gateway/server-channels.ts`。
- 观察：外部 channel plugin 可以拿到 `createPluginRuntime().channel` 提供的 runtime surface；bundled channels 则可直接引用 monorepo 内部模块。证据：`docs/references/openclaw-main/src/gateway/server-channels.ts`、`docs/references/openclaw-main/src/plugins/runtime/types.ts`。
- 观察：OpenClaw 保留一个 shared `message` tool；channel plugin 负责 channel-specific action discovery、capability discovery、schema fragments 和最终执行。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`。
- 推断：Channel adapter 的设计重心不是“每个渠道一个工具”，而是“核心统一消息工具 + 插件提供渠道语义”。这降低了 core 对 Slack/Discord/Telegram/WhatsApp 等 provider 细节的耦合。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`、`docs/references/openclaw-main/src/channels/plugins/types.plugin.ts`。

## 6. Local-first assistant 设计

- 观察：OpenClaw 使用单个 embedded agent runtime；workspace 是 agent 的 cwd 和上下文根。证据：`docs/references/openclaw-main/docs/concepts/agent.md`、`docs/references/openclaw-main/docs/concepts/agent-workspace.md`。
- 观察：workspace 是默认 cwd，不是硬 sandbox。未启用 sandbox 时，绝对路径仍可访问 host 其他位置。证据：`docs/references/openclaw-main/docs/concepts/agent-workspace.md`、`docs/references/openclaw-main/SECURITY.md`。
- 观察：启动上下文通过 workspace 内的 `AGENTS.md`、`SOUL.md`、`TOOLS.md`、`BOOTSTRAP.md`、`IDENTITY.md`、`USER.md` 等文件注入；`TOOLS.md` 只是使用指导，不控制工具存在。证据：`docs/references/openclaw-main/docs/concepts/agent.md`、`docs/references/openclaw-main/docs/concepts/agent-workspace.md`。
- 观察：skills 有明确优先级：workspace、project agent、personal、managed/local、bundled、extra dirs。证据：`docs/references/openclaw-main/docs/concepts/agent.md`。
- 观察：session transcript 以 JSONL 存在 `~/.openclaw/agents/<agentId>/sessions/<SessionId>.jsonl`，session store 也由 Gateway 管。证据：`docs/references/openclaw-main/docs/concepts/agent.md`、`docs/references/openclaw-main/docs/concepts/session.md`。
- 观察：agent loop 对每个 session key 串行化执行，并通过全局 lane 控制并发。证据：`docs/references/openclaw-main/docs/concepts/agent-loop.md`、`docs/references/openclaw-main/docs/concepts/queue.md`。
- 推断：OpenClaw 的 local-first 不是只指本地存储，而是“本地工作区 + 本地配置 + 本地凭据 + 本机工具执行 + Gateway daemon”共同形成 assistant 运行边界。证据：`docs/references/openclaw-main/docs/concepts/agent-workspace.md`、`docs/references/openclaw-main/docs/gateway/index.md`、`docs/references/openclaw-main/SECURITY.md`。

## 7. Messaging / DM / pairing / allowlist

- 观察：Inbound DM 被视为 untrusted input。默认 DM pairing 下，未知 sender 只收到短 code，原消息不会进入 agent。审批后 sender 进入本地 allowlist store。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/docs/channels/pairing.md`。
- 观察：DM pairing store 位于 `~/.openclaw/credentials/`，pending request 和 approved allowlist 分文件保存，并按 default account / non-default account 区分。证据：`docs/references/openclaw-main/docs/channels/pairing.md`。
- 观察：DM allowlist 和 group authorization 是两套边界。批准 DM pairing 不会自动允许 group 命令或 group control。证据：`docs/references/openclaw-main/docs/channels/pairing.md`、`docs/references/openclaw-main/src/security/dm-policy-shared.ts`。
- 观察：session routing 默认让 direct messages 共享 main session；可通过 `session.dmScope` 改成 per-peer、per-channel-peer、per-account-channel-peer。证据：`docs/references/openclaw-main/docs/concepts/session.md`。
- 观察：回复路由是确定性的。OpenClaw 路由回消息来源 channel，模型不选择 channel。证据：`docs/references/openclaw-main/docs/channels/channel-routing.md`。
- 观察：agent routing 按 peer、thread inheritance、guild/role、team、account、channel、default agent 选择一个 agent。证据：`docs/references/openclaw-main/docs/channels/channel-routing.md`。
- 观察：`webchat` 是内部 UI channel，不是可配置 outbound channel；Gateway `send` 对它显式拒绝。证据：`docs/references/openclaw-main/docs/channels/channel-routing.md`、`docs/references/openclaw-main/src/gateway/server-methods/send.ts`。
- 推断：allowlist 主要控制“谁能触发 agent”和部分 owner-style 操作，不等同于所有上下文可见性的统一过滤。这个边界在安全文档中被明确限制。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。

## 8. Canvas / UI / console 相关设计

- 观察：Gateway HTTP server 提供 Control UI、OpenAI-compatible endpoints、hooks、canvas host、plugin routes 等 surface。证据：`docs/references/openclaw-main/docs/gateway/index.md`、`docs/references/openclaw-main/src/gateway/server-http.ts`。
- 观察：Control UI 是 Vite + Lit SPA，从 Gateway 同端口服务，并直接使用 Gateway WebSocket。证据：`docs/references/openclaw-main/docs/web/index.md`、`docs/references/openclaw-main/docs/web/control-ui.md`。
- 观察：Control UI 可做 chat、sessions、config、logs、cron、skills、nodes、exec approvals、update 等 Gateway 操作；配置写入带 hash guard 和 SecretRef preflight。证据：`docs/references/openclaw-main/docs/web/control-ui.md`。
- 观察：WebChat 的 macOS/iOS SwiftUI UI 直接连 Gateway WebSocket，使用 `chat.history`、`chat.send`、`chat.inject`，没有独立 WebChat server。证据：`docs/references/openclaw-main/docs/web/webchat.md`。
- 观察：TUI 支持 Gateway mode 和 local embedded mode。local mode 直接使用 embedded agent runtime，但 Gateway-only features 不可用。证据：`docs/references/openclaw-main/docs/web/tui.md`。
- 观察：macOS Canvas 用 WKWebView 嵌入 agent-controlled Canvas panel，文件通过自定义 scheme 服务；Gateway WebSocket 暴露 show/hide、navigate、eval、snapshot。证据：`docs/references/openclaw-main/docs/platforms/mac/canvas.md`。
- 观察：A2UI 由 Gateway canvas host 服务，路径包括 `/__openclaw__/canvas/` 和 `/__openclaw__/a2ui/`。证据：`docs/references/openclaw-main/docs/concepts/architecture.md`、`docs/references/openclaw-main/docs/platforms/mac/canvas.md`、`docs/references/openclaw-main/src/canvas-host/server.ts`。
- 观察：Canvas 内容被安全文档视为 arbitrary HTML/JS，需要当作 untrusted content 处理。证据：`docs/references/openclaw-main/docs/gateway/security/index.md`、`docs/references/openclaw-main/docs/platforms/mac/canvas.md`。

## 9. Agent runtime 与 gateway 的关系

- 观察：Agent runtime 底层基于 Pi agent core；OpenClaw 自己拥有 session management、discovery、tool wiring、channel delivery。证据：`docs/references/openclaw-main/docs/concepts/agent.md`、`docs/references/openclaw-main/docs/concepts/agent-loop.md`。
- 观察：Gateway `agent` RPC 先解析 session、更新 session store、登记 run context、返回 accepted ack，再异步运行 agent command。证据：`docs/references/openclaw-main/src/gateway/server-methods/agent.ts`、`docs/references/openclaw-main/docs/concepts/agent-loop.md`。
- 观察：`chat.send` 是 WebChat/Control UI 的 WS-native chat path。它 ack 后调用 `dispatchInboundMessage`，通过 reply pipeline 收集 block/final/tool payload，并向 `chat` event stream 广播。证据：`docs/references/openclaw-main/src/gateway/server-methods/chat.ts`、`docs/references/openclaw-main/docs/web/control-ui.md`、`docs/references/openclaw-main/docs/web/webchat.md`。
- 观察：`send` RPC 是直接 outbound delivery path，解析 channel/target/session route 后使用 channel plugin 或 outbound delivery 层发送。证据：`docs/references/openclaw-main/src/gateway/server-methods/send.ts`。
- 观察：agent loop 中 `runEmbeddedPiAgent` 负责排队、构造 pi session、订阅 pi events、转成 OpenClaw agent stream、处理 timeout 和 usage。证据：`docs/references/openclaw-main/docs/concepts/agent-loop.md`。
- 推断：Gateway 不只是 agent runtime 的 transport wrapper；它还承担 session truth、routing truth、tool policy入口、channel delivery 和 UI event bus。证据：`docs/references/openclaw-main/src/gateway/server-methods/agent.ts`、`docs/references/openclaw-main/src/gateway/server-methods/chat.ts`、`docs/references/openclaw-main/docs/concepts/session.md`。

## 10. Tool / integration 机制

- 观察：OpenClaw 将 tool 定义为 agent 可调用的 typed function；内置工具包括 exec/process、browser、web_search/web_fetch、read/write/edit/apply_patch、message、canvas、nodes、cron/gateway、media、sessions/subagents 等。证据：`docs/references/openclaw-main/docs/tools/index.md`。
- 观察：工具可通过 allow/deny、profiles、groups、provider-specific restrictions 控制。deny 胜过 allow。证据：`docs/references/openclaw-main/docs/tools/index.md`、`docs/references/openclaw-main/docs/gateway/sandbox-vs-tool-policy-vs-elevated.md`。
- 观察：Plugin 系统分为 manifest/discovery、enablement/validation、runtime loading、surface consumption 四层。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`。
- 观察：manifest 是 control-plane source of truth；runtime module 的 `register(api)` 是 data-plane behavior。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`、`docs/references/openclaw-main/docs/plugins/building-plugins.md`。
- 观察：native plugins in-process 加载，不 sandbox；加载后向 central plugin registry 注册 tools、hooks、channels、providers、gateway RPC handlers、HTTP routes、CLI、services、memory runtime 等。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`、`docs/references/openclaw-main/src/plugins/registry.ts`。
- 观察：`PluginRuntime` 给 native plugins 注入 trusted in-process runtime surface，包括 subagent 和 channel helpers。证据：`docs/references/openclaw-main/src/plugins/runtime/types.ts`、`docs/references/openclaw-main/src/plugins/registry.ts`。
- 观察：MCP 支持被放在 `mcporter` bridge，而不是 core 内一等 runtime。证据：`docs/references/openclaw-main/VISION.md`。
- 推断：Integration 机制采用“manifest 静态元数据先行 + registry 动态能力消费”的模式，降低启动和配置解释对插件运行时代码的依赖。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`。

## 11. Memory / persona / workspace instructions

- 观察：长期 memory 是 workspace 内的 Markdown 文件。`MEMORY.md` 存长期事实，`memory/YYYY-MM-DD.md` 存每日记录，`DREAMS.md` 用于 dreaming review。证据：`docs/references/openclaw-main/docs/concepts/memory.md`、`docs/references/openclaw-main/docs/concepts/agent-workspace.md`。
- 观察：memory tools 是 `memory_search` 和 `memory_get`，由 active memory plugin 提供。证据：`docs/references/openclaw-main/docs/concepts/memory.md`。
- 观察：内置 memory engine 使用 per-agent SQLite index，支持 FTS5、vector、hybrid search、CJK trigram 和 sqlite-vec。证据：`docs/references/openclaw-main/docs/concepts/memory-builtin.md`。
- 观察：Active Memory 是可选 plugin-owned blocking memory sub-agent，只在符合条件的 interactive persistent chat session 前置运行，并把相关 memory 以 hidden untrusted prompt prefix 注入主回复。证据：`docs/references/openclaw-main/docs/concepts/active-memory.md`。
- 观察：Active Memory 的 blocking memory sub-agent 只可使用 `memory_search` 和 `memory_get`。证据：`docs/references/openclaw-main/docs/concepts/active-memory.md`。
- 观察：persona / identity / user profile 来自 workspace 文件，如 `SOUL.md`、`IDENTITY.md`、`USER.md`，并在 session 启动上下文里注入。证据：`docs/references/openclaw-main/docs/concepts/agent.md`、`docs/references/openclaw-main/docs/concepts/agent-workspace.md`。
- 观察：`MEMORY.md` 和 `memory/*.md` 被视为 trusted local operator state，不是安全边界。证据：`docs/references/openclaw-main/SECURITY.md`。

## 12. Security / permission / inbound message trust boundary

- 观察：OpenClaw 明确不是 hostile multi-tenant security boundary。一个 Gateway 对应一个 trusted operator boundary。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。
- 观察：Authenticated Gateway callers 被视为 trusted operators。HTTP compatibility endpoints 和 `/tools/invoke` 的 shared-secret bearer auth 等价于 full operator access。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。
- 观察：`sessionKey`、session IDs、labels 是 routing controls，不是 per-user authorization boundary。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。
- 观察：模型不是 trusted principal。安全边界来自 host/config trust、auth、tool policy、sandboxing、exec approvals。证据：`docs/references/openclaw-main/SECURITY.md`。
- 观察：Gateway 和 Node 在同一 operator trust domain 内。pairing 一个 node 会授予该 node 上的 operator-level remote capability；node `system.run` 是远程代码执行能力。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`、`docs/references/openclaw-main/docs/gateway/protocol.md`。
- 观察：exec approvals 是 operator guardrails，不是多租户授权边界。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。
- 观察：plugins/extensions 是 Gateway 的 TCB，native plugin 可用同一 OS 权限执行。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/plugins/architecture.md`。
- 观察：inbound DMs 是 untrusted input；pairing/allowlist gate 触发权，但 allowlist 不承诺所有 supplemental context 的统一 redaction。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/docs/channels/pairing.md`、`docs/references/openclaw-main/SECURITY.md`。
- 观察：workspace `.env` 不能覆盖 `OPENCLAW_*` runtime control 变量。证据：`docs/references/openclaw-main/docs/gateway/security/index.md`。
- 观察：Control UI 和 canvas host 共处 Gateway HTTP surface；canvas arbitrary HTML/JS 需要当作 untrusted web content。证据：`docs/references/openclaw-main/docs/gateway/security/index.md`、`docs/references/openclaw-main/docs/platforms/mac/canvas.md`。

## 13. 可迁移到 Octopus 的设计优点

These are transferable patterns only. They are not an Octopus architecture proposal.

- 可迁移模式：把“assistant 产品体验”和“Gateway control plane”分开描述，避免把控制面误当产品本体。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/docs/gateway/index.md`。
- 可迁移模式：Gateway 协议采用首帧 handshake、role/scope/caps/commands 声明、device identity、device token、pairing approval。这个模式能把客户端、节点、UI、自动化入口纳入同一连接协议。证据：`docs/references/openclaw-main/docs/gateway/protocol.md`、`docs/references/openclaw-main/src/gateway/server/ws-connection.ts`。
- 可迁移模式：副作用 RPC 强制 idempotency key，并在 Gateway 做短期 dedupe/in-flight 合并。证据：`docs/references/openclaw-main/docs/gateway/protocol.md`、`docs/references/openclaw-main/src/gateway/server-methods/send.ts`、`docs/references/openclaw-main/src/gateway/server-methods/chat.ts`、`docs/references/openclaw-main/src/gateway/server-methods/agent.ts`。
- 可迁移模式：channel adapter 保持 vendor 语义在 plugin 内，core 只持有统一 message tool、routing、session、dispatch。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`、`docs/references/openclaw-main/src/channels/plugins/types.plugin.ts`。
- 可迁移模式：plugin manifest 先行，runtime register 后置。配置解释、UI schema、activation planning 不必先执行插件代码。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`。
- 可迁移模式：workspace instructions、persona、user profile、memory 用文件表达，并把哪些文件是 prompt guidance、哪些文件是 tool availability 明确拆开。证据：`docs/references/openclaw-main/docs/concepts/agent.md`、`docs/references/openclaw-main/docs/concepts/agent-workspace.md`。
- 可迁移模式：把安全模型写成明确的 trust boundary，而不是事后靠隐含假设解释。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。

## 14. 不建议迁移的设计

- 不建议直接迁移：单 Gateway 内 authenticated caller 近似 full trusted operator 的假设。该模型只适合个人 assistant，不适合混合信任或多租户边界。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。
- 不建议直接迁移：native plugins 默认 in-process 且 trusted。若扩展生态面向第三方或不完全可信来源，这个边界过宽。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/plugins/architecture.md`。
- 不建议直接迁移：host-first exec 默认。OpenClaw 文档将其解释为单用户 trusted-operator UX，但这不适合更强隔离场景。证据：`docs/references/openclaw-main/README.md`、`docs/references/openclaw-main/SECURITY.md`。
- 不建议直接迁移：direct messages 默认共享 main session。OpenClaw 自身也提示多 sender 时需要启用 DM isolation。证据：`docs/references/openclaw-main/docs/concepts/session.md`。
- 不建议直接迁移：Canvas arbitrary HTML/JS 与 Gateway HTTP surface 在同一服务进程内暴露。若没有严格 origin、auth、network isolation，这会扩大 UI 攻击面。证据：`docs/references/openclaw-main/docs/gateway/security/index.md`、`docs/references/openclaw-main/src/gateway/server-http.ts`、`docs/references/openclaw-main/src/canvas-host/server.ts`。
- 不建议直接迁移：把 HTTP compatibility endpoints、direct tool endpoint 和 operator auth 放在同一个 full-access secret boundary。这个选择方便兼容，但 credential 泄漏后的 blast radius 大。证据：`docs/references/openclaw-main/SECURITY.md`、`docs/references/openclaw-main/docs/gateway/index.md`、`docs/references/openclaw-main/docs/gateway/security/index.md`。

## 15. Unverified / open questions

- Unverified：session/current runtime projection 是否除 JSON session store 和 JSONL transcript 外还有更完整的数据库投影。已确认文档声明 session store/transcripts 在文件中；未在本轮确认是否存在另一个权威 store。证据边界：`docs/references/openclaw-main/docs/concepts/session.md`、`docs/references/openclaw-main/docs/concepts/agent.md`。
- Unverified：TypeBox schema 到 JSON Schema 到 Swift model 的完整 codegen pipeline 未完全追踪，只确认文档声明该 pipeline 存在。证据边界：`docs/references/openclaw-main/docs/concepts/architecture.md`、`docs/references/openclaw-main/docs/gateway/protocol.md`。
- Unverified：外部 plugin API 的稳定性边界仍在演进。文档明确说 capability-specific helper surfaces 不全是 frozen contract。证据：`docs/references/openclaw-main/docs/plugins/architecture.md`。
- Unverified：各 channel 对 supplemental context 的 allowlist redaction 不完全统一。安全文档明确当前行为不完全一致，但本轮未逐个 channel 审计。证据：`docs/references/openclaw-main/SECURITY.md`。
- Unverified：Canvas/A2UI 在各 companion app 上的权限、origin 和内容隔离实现细节未完整验证；本轮只确认 macOS 文档、Gateway host 和安全说明。证据边界：`docs/references/openclaw-main/docs/platforms/mac/canvas.md`、`docs/references/openclaw-main/src/canvas-host/server.ts`、`docs/references/openclaw-main/docs/gateway/security/index.md`。

## 16. 关键文件路径索引

- `docs/references/openclaw-main/README.md`：项目定位、安装、DM pairing、local-first Gateway、apps optional、安全默认。
- `docs/references/openclaw-main/VISION.md`：目标、TypeScript 选择、plugin/memory/MCP 方向、暂不合并的架构方向。
- `docs/references/openclaw-main/package.json`：包入口、脚本、运行时依赖、UI/mobile/dev 依赖。
- `docs/references/openclaw-main/openclaw.mjs`：CLI bin bootstrap。
- `docs/references/openclaw-main/src/index.ts`：legacy direct entrypoint 与 library export。
- `docs/references/openclaw-main/src/cli/run-main.ts`：CLI runtime setup、command routing、program loading。
- `docs/references/openclaw-main/src/cli/program/core-command-descriptors.ts`：核心 CLI 命令目录。
- `docs/references/openclaw-main/docs/concepts/architecture.md`：Gateway、clients、nodes、canvas host、WS protocol summary。
- `docs/references/openclaw-main/docs/gateway/index.md`：Gateway runbook、single process、single port、auth/default bind、OpenAI-compatible endpoints。
- `docs/references/openclaw-main/docs/gateway/protocol.md`：WS handshake、roles/scopes、node claims、broadcast scope gating、auth/device pairing。
- `docs/references/openclaw-main/src/gateway/server.impl.ts`：Gateway startup orchestration。
- `docs/references/openclaw-main/src/gateway/server-methods-list.ts`：Gateway RPC methods 和 events。
- `docs/references/openclaw-main/src/gateway/server-methods/agent.ts`：Gateway `agent` RPC。
- `docs/references/openclaw-main/src/gateway/server-methods/chat.ts`：Gateway `chat.*` RPC。
- `docs/references/openclaw-main/src/gateway/server-methods/send.ts`：Gateway outbound `send` 和 `message.action`。
- `docs/references/openclaw-main/docs/channels/index.md`：channel 总览。
- `docs/references/openclaw-main/docs/channels/pairing.md`：DM pairing、allowlist store、node pairing。
- `docs/references/openclaw-main/docs/channels/channel-routing.md`：channel/session/agent routing。
- `docs/references/openclaw-main/src/channels/plugins/types.plugin.ts`：`ChannelPlugin` contract。
- `docs/references/openclaw-main/src/gateway/server-channels.ts`：channel manager lifecycle。
- `docs/references/openclaw-main/docs/concepts/agent.md`：agent runtime、workspace、bootstrap files、skills、runtime boundary。
- `docs/references/openclaw-main/docs/concepts/agent-workspace.md`：workspace layout、memory/persona files、sandbox caveat。
- `docs/references/openclaw-main/docs/concepts/agent-loop.md`：agent loop、queue、Pi runtime bridge、streaming lifecycle。
- `docs/references/openclaw-main/docs/concepts/queue.md`：per-session lane、global lane、queue modes。
- `docs/references/openclaw-main/docs/web/control-ui.md`：Vite + Lit Control UI、WS auth、capabilities、device pairing、CSP。
- `docs/references/openclaw-main/docs/web/webchat.md`：WebChat WS methods、history behavior、no separate server。
- `docs/references/openclaw-main/docs/web/tui.md`：TUI Gateway/local modes。
- `docs/references/openclaw-main/docs/platforms/mac/canvas.md`：WKWebView Canvas、custom scheme、A2UI、Gateway commands、安全说明。
- `docs/references/openclaw-main/src/gateway/server-http.ts`：HTTP surface composition。
- `docs/references/openclaw-main/src/canvas-host/server.ts`：Canvas host handler。
- `docs/references/openclaw-main/docs/tools/index.md`：tools、skills、plugins、tool policy。
- `docs/references/openclaw-main/docs/plugins/architecture.md`：plugin capability model、load pipeline、registry、trust boundary、channel shared message tool。
- `docs/references/openclaw-main/src/plugins/registry.ts`：central plugin registry。
- `docs/references/openclaw-main/src/plugins/runtime/types.ts`：trusted plugin runtime surface。
- `docs/references/openclaw-main/docs/concepts/memory.md`：Markdown memory model、memory tools、dreaming。
- `docs/references/openclaw-main/docs/concepts/memory-builtin.md`：SQLite/FTS/vector/hybrid memory backend。
- `docs/references/openclaw-main/docs/concepts/active-memory.md`：plugin-owned blocking memory sub-agent。
- `docs/references/openclaw-main/SECURITY.md`：operator trust model、plugin trust、memory trust、inbound context boundary。
- `docs/references/openclaw-main/docs/gateway/security/index.md`：Gateway/node trust、network/UI/canvas/security hardening。
