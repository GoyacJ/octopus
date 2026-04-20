# 12 · 插件体系（Plugin System）

> 本章定义 Octopus SDK **插件层**的抽象。目标：一份清单描述能装进 harness 的全部东西（工具、技能、命令、子代理、hook、MCP、provider、频道、context engine …）；核心代码不和任何具体实现耦合，而是从统一的 **Plugin Registry** 里读。
>
> 设计不受 Octopus 当前实现约束；直接沿用 **OpenClaw + Claude Code** 两套成熟体系的交集（前者是 Capability-based 的深度范式，后者是 Manifest + Marketplace 的广度范式），再吸收 **Hermes** 的轻量 Python 插槽式接法。

## 12.1 Why：为什么需要显式的插件体系

Agent Harness 里有超过 10 类**可扩展点**（工具、技能、命令、agent 定义、hook、MCP 服务器、LSP 服务器、模型 provider、通讯频道、上下文引擎、记忆后端、输出样式、HTTP 路由、RPC handler、后台服务 …）。如果核心代码为每一类单独实现一个注册机制，就会走向：

- **ownership 混乱**：同一个厂商的 Text/Speech/Vision/Image 分散在 5 个地方
- **hard-coded provider switch**：`if provider === 'openai' else if ...`
- **核心代码膨胀**：每加一个新厂商 / 新频道都要改核心
- **禁止第三方扩展**：二次开发必须 fork

插件体系把"**可扩展点的注册**"本身变成一个**一等公民的抽象**，让 core 只做：

1. 发现谁声称要扩展
2. 验证声明合法
3. 加载时按契约注入
4. 运行时按 registry 分派

### 三参考项目的插件机制对比

| 维度 | Claude Code | OpenClaw | Hermes |
|---|---|---|---|
| **发现方式** | built-in + marketplace (git/npm/url/file/directory/github) | manifest + workspace roots + global extension roots + bundled extensions | Python 模块目录 `plugins/<slot>/<name>/` |
| **清单格式** | `plugin.json` (Zod schema)；附 `marketplace.json` | `openclaw.plugin.json`（控制面 SoT）+ `package.json:openclaw.*`（compat / install / channel） | 无显式 manifest；入口 `__init__.py` + ABC |
| **扩展点种类** | commands / agents / skills / hooks / mcpServers / lspServers / output-styles / channels / settings / userConfig | provider / cli-backend / speech / realtime-transcription / realtime-voice / media-understanding / image-generation / music-generation / video-generation / web-fetch / web-search / channel / tools / hooks / commands / services / http-routes / gateway-RPC / context-engine | 仅 `context_engine` + `memory` 两个 slot |
| **打包格式** | plugin 目录 + npm / pip / git / MCPB 离线 bundle (`.mcpb` / `.dxt`) | plugin 目录（可含 `package.json` 的 extension 数组）+ npm / 本地 | Python 包（`pip install`） |
| **版本锁定** | `gitSha`（40 字节全 SHA）+ `semver` + `ref` + `version range` | `openclaw.compat.pluginApi` + `openclaw.build.openclawVersion` | 未显式（由 pip 管理） |
| **安全** | 保留名 + 非 ASCII 检测 + blockedMarketplaces/strictKnownMarketplaces + `npm install --omit=dev --ignore-scripts` | 路径逃逸拒绝 + 世界可写拒绝 + allowlist by id + in-process 可信（原生插件） | — |
| **依赖关系** | `dependencies[]`，`bare name` 解析为同 marketplace | 一个插件 = 一个公司/功能的所有权边界 | — |
| **热重载** | `/reload-plugins`；自动背景安装 | 短期缓存 + 重启即可 | 无 |
| **错误分类** | ~22 种 discriminated union（路径/git/manifest/mcp/lsp/hook/component/marketplace/policy/dependency/cache） | `openclaw plugins inspect` + `doctor` + 分级信号（valid / advisory / legacy / hard error） | — |
| **企业策略** | `strictKnownMarketplaces` 白名单、`blockedMarketplaces` 黑名单 | `plugins.allow` / `plugins.deny` / `plugins.enabled` / `plugins.slots` | — |
| **典型插件数量** | 增长中（official `claude-code-marketplace`、`agent-skills` 等 8 个保留名）| **90+ 个 bundled extensions**（几乎每个厂商/频道/能力都一个）| 8 个 memory provider + 1 个 context engine |

### 设计合成（Octopus 视角）

| 维度 | 取谁的做法 | 理由 |
|---|---|---|
| 扩展点种类 | OpenClaw + Claude Code 并集 | 覆盖最广；Octopus 未来要接入 10+ 种厂商 |
| Manifest schema | Claude Code 的 Zod 风格 | 严格类型；有 `validate` 子命令 |
| 所有权模型 | OpenClaw "一公司/一功能 = 一插件" | 防止核心出现 `if vendor === 'X'` |
| Registry 单向流 | OpenClaw | 核心读 registry，不 reach-into plugin 内部 |
| 分发格式 | Claude Code（git / npm / pip / marketplace + `.mcpb`）| 多语言、可离线、可签名 |
| 版本锁 | Claude Code `gitSha` | 可审计、可 reproduce |
| 安全基线 | Claude Code + OpenClaw 并集 | 保留名 + 路径逃逸 + 无 lifecycle scripts + 沙箱声明 |
| 轻量 Slot | Hermes | 小型能力（memory/context_engine）走 slot + 默认 builtin |

## 12.2 三层架构：Manifest · Registry · Runtime

```
┌──────────────────────────────────────────────────────────────────┐
│ Manifest 层（控制面 · Source of Truth）                            │
│   plugin.json / package.json:octopus.*                           │
│   描述：id, version, extension points, deps, compat, install      │
│   用途：发现、验证、UI 提示、启用/禁用决策、解析依赖                  │
│   关键特征：不执行插件代码即可读懂                                    │
└──────────────────────────────┬───────────────────────────────────┘
                               │ discover + validate + enable
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│ Registry 层（核心）                                                │
│   中央登记簿：tools / skills / commands / agents / hooks /         │
│   mcpServers / lspServers / channels / providers / routes /       │
│   services / context-engines / output-styles                      │
│   约束：单向流（plugin → registry → core）                           │
│   约束：每个 id 唯一拥有人；重复注册按策略拒绝                         │
└──────────────────────────────┬───────────────────────────────────┘
                               │ expose
                               ▼
┌──────────────────────────────────────────────────────────────────┐
│ Runtime 层（数据面）                                                │
│   插件入口：register(api) / activate(api)                           │
│   插件可用的 api：api.registerX(...) + api.runtime.*                │
│   运行时上下文：config、secrets（引用）、session、log、sandbox handle  │
└──────────────────────────────────────────────────────────────────┘
```

> 来源：OpenClaw `docs/plugins/architecture.md` §"Architecture overview"（四层模型：manifest + discovery / enablement / runtime loading / surface consumption）；Claude Code `src/types/plugin.ts`（`LoadedPlugin` + `PluginComponent`）。

**关键设计准则**：
- Manifest-first：**不加载插件代码**就能做配置校验、UI 提示、启用决策
- 安全门在运行前：路径逃逸、世界可写、来源不可信 → 在 `register(api)` 执行**前**拦截
- Registry 单向流：插件只能往 registry 写；核心只能从 registry 读

## 12.3 扩展点（Extension Points）全景

Octopus 支持以下扩展点。每一项都是一个**能力契约**（core 定义）+ **实现注册**（plugin 注册）。

| 类别 | Extension Point | 注册 API | 内容 |
|---|---|---|---|
| **对话能力** | Tool | `api.registerTool({ name, spec, handler })` | 外部操作（见 `03-tool-system.md`） |
| | Skill | `api.registerSkill({ id, manifest, files })` | Markdown + 资产（按需加载） |
| | Command | `api.registerCommand({ name, metadata, source/content })` | `/slash` 命令 |
| | Agent | `api.registerAgent({ id, definition })` | 子代理定义（见 `05-sub-agents.md`） |
| | Output Style | `api.registerOutputStyle({ id, template })` | 响应渲染风格 |
| **生命周期** | Hook | `api.registerHook({ event, handler })` 或 manifest 声明 | 见 `07-hooks-lifecycle.md` |
| **MCP 生态** | MCP Server | `api.registerMcpServer({ id, config })` | stdio / http / sdk in-process / `.mcpb` bundle |
| **IDE 集成** | LSP Server | `api.registerLspServer({ id, config })` | Language Server（代码智能感知） |
| **模型层** | Model Provider | `api.registerProvider({ id, auth, catalog, runtimeHooks })` | 见 `11-model-system.md`；含 44 个可选 runtime hook |
| | Speech Provider | `api.registerSpeechProvider(...)` | TTS |
| | Realtime Transcription | `api.registerRealtimeTranscriptionProvider(...)` | 实时 STT |
| | Realtime Voice | `api.registerRealtimeVoiceProvider(...)` | 实时语音对话 |
| | Media Understanding | `api.registerMediaUnderstandingProvider(...)` | image/audio/video 理解 |
| | Image Generation | `api.registerImageGenerationProvider(...)` | T2I |
| | Video Generation | `api.registerVideoGenerationProvider(...)` | T2V |
| | Music Generation | `api.registerMusicGenerationProvider(...)` | T2Music |
| | Web Search | `api.registerWebSearchProvider(...)` | Web Search |
| | Web Fetch | `api.registerWebFetchProvider(...)` | Web Fetch |
| | Embedding | `api.registerEmbeddingProvider(...)` | 向量化 |
| **上下文** | Context Engine | `api.registerContextEngine(id, factory)` | ingest / assemble / compact（见 `02-context-engineering.md`） |
| | Memory Backend | `api.registerMemoryBackend({ id, impl })` | `mem0`/`honcho`/`supermemory`... |
| **通讯** | Channel | `api.registerChannel({ id, adapter })` | Telegram/Discord/WhatsApp/... |
| **服务器** | HTTP Route | `api.registerHttpRoute({ path, auth, handler })` | 插件暴露 HTTP 端点 |
| | RPC Handler | `api.registerRpcHandler({ namespace, methods })` | Gateway JSON-RPC |
| | Background Service | `api.registerService({ id, start, stop })` | 常驻后台服务 |
| | CLI Registrar | `api.registerCli({ descriptors, commands })` | 子命令 |

> 来源：OpenClaw `docs/plugins/architecture.md` §"Public capability model"（12 类 capability 注册表）；Claude Code `src/types/plugin.ts` `PluginComponent` + `LoadedPlugin` 所有字段。

### 12.3.1 插件形态分类（Plugin Shape）

基于实际注册行为（而非 manifest 声明）对加载后的插件分类：

| Shape | 注册行为 | 例子 |
|---|---|---|
| **plain-capability** | 正好注册一类能力 | 只做 Speech 的 ElevenLabs |
| **hybrid-capability** | 注册多类能力（但同一 ownership） | OpenAI：Text + Speech + Image + MediaUnderstanding |
| **hook-only** | 只注册 Hook，无能力/工具 | 合规审计插件 |
| **non-capability** | 注册工具/命令/服务/路由但无能力 | 内部运维工具 |

> 来源：OpenClaw `docs/plugins/architecture.md` §"Plugin shapes"。

**`octopus plugins inspect <id>`** CLI 显示 shape + capability 分布，便于诊断。

## 12.4 所有权模型（Ownership）

> "**plugin = ownership boundary；capability = core contract**"
> — OpenClaw 原则

单个插件应该是：

- **一家厂商的全部 Octopus 面** — 例：一个 `openai` 插件同时注册 Text/Speech/Image/Vision；而不是 5 个独立 `openai-*` 插件
- **或一个完整功能** — 例：`voice-call` 插件注册 Call transport + Tools + CLI + HTTP Routes + Twilio 桥

**为什么**：

1. **用户心智**：用户装 `openai@marketplace` 一次，拿到该厂商所有能力
2. **版本一致性**：同一厂商的多个 surface 用同一个 auth 配置，版本绑死
3. **升级原子性**：一次 release 里厂商自己保证内部一致
4. **避免 id 冲突**：两个插件同时注册 `openai` provider 会被 registry 拒绝

### 何时拆分

| 情况 | 拆分方式 |
|---|---|
| 不同公司维护同一厂商的两个分支 | 显式 id 前缀（`openai-fork@community`） |
| 一家厂商有多个**独立产品线**，分开计费/合规 | 一个产品线一个插件（`anthropic` / `anthropic-vertex`） |
| 可选能力需要重依赖（只有少数用户用） | 一个主插件 + 可选扩展包（用 dependencies 引） |

> 来源：OpenClaw `docs/plugins/architecture.md` §"Capability ownership model"（含多 capability 公司插件示例）。

## 12.5 Manifest 规范

### 12.5.1 文件位置

```
<plugin-root>/
├── plugin.json              # 主清单（必需）
├── hooks/
│   └── hooks.json           # 钩子默认位置（可选）
├── commands/                # 命令默认目录（可选）
│   ├── about.md
│   └── reindex.md
├── agents/                  # 子代理定义（可选）
├── skills/                  # Skills 目录（可选）
│   └── <name>/SKILL.md
├── output-styles/           # 输出样式（可选）
├── mcp-servers/             # MCP bundle `.mcpb`（可选）
└── package.json             # 若为 npm 分发，附 octopus.* 块
```

### 12.5.2 `plugin.json` 字段（`zod` 校验）

```ts
{
  name: string,                       // kebab-case, 不含空格
  version?: string,                   // semver
  description?: string,
  author?: { name, email?, url? },
  homepage?: string,                  // URL
  repository?: string,
  license?: string,                   // SPDX
  keywords?: string[],

  dependencies?: Array<
    | string                          // 'other-plugin' (解析到同 marketplace)
    | `${string}@${string}`           // 'other-plugin@marketplace-name'
  >,

  compat?: {
    pluginApi: string,                // semver range; e.g. '^1.0.0'
    minHostVersion?: string,
    pluginSdkVersion?: string
  },

  // —— 扩展点声明（全部可选）——
  hooks?: './hooks.json' | HooksInline | Array<...>,
  commands?: './cmd.md' | string[] | Record<name, CommandMetadata>,
  agents?:   './agent.md' | string[],
  skills?:   './skill-dir' | string[],
  outputStyles?: './styles-dir' | string[],
  channels?:  ChannelMetadata[],
  mcpServers?: Record<id, McpServerConfig>,
  lspServers?: Record<id, LspServerConfig>,
  settings?:  SettingsSchema,
  userConfig?: UserConfigTemplate,

  // —— 控制面 metadata（manifest-first）——
  activation?: {
    triggers?: Array<'command' | 'channel' | 'provider' | 'startup'>,
    primaryCommand?: string,
    channelIds?: string[],
    providerIds?: string[]
  },
  setup?: {
    providers?: string[],             // 只有这些 provider 的 setup 会 narrow 到本插件
    cliBackends?: string[],
    providerAuthEnvVars?: string[],
    providerAuthAliases?: string[],
    channelEnvVars?: string[]
  },
  install?: {
    npmSpec?: string,
    pipSpec?: string,
    localPath?: string,
    defaultChoice?: 'npm' | 'pip' | 'local',
    sandbox?: 'native' | 'sandboxed'  // 见 §12.10
  }
}
```

> 来源：Claude Code `src/utils/plugins/schemas.ts`（`PluginManifestSchema` 组合 metadata + hooks + commands + agents + skills + outputStyles + channels + mcpServers + lspServers + settings + userConfig）；OpenClaw `packages/plugin-package-contract/src/index.ts`（`ExternalPluginCompatibility`：`pluginApiRange` + `builtWithOpenClawVersion` + `pluginSdkVersion` + `minGatewayVersion`）。

### 12.5.3 `package.json` 的 `octopus` 块（当插件也是 npm 包时）

```json
{
  "name": "@octopus/my-plugin",
  "version": "1.2.3",
  "main": "./dist/plugin.js",
  "octopus": {
    "extensions": ["./dist/plugin.js"],
    "setupEntry": "./dist/setup-entry.js",
    "compat": { "pluginApi": "^1.0.0" },
    "build": { "octopusVersion": "0.9.0", "pluginSdkVersion": "1.0.0" },
    "install": { "minHostVersion": "0.9.0" },
    "startup": {
      "deferConfiguredChannelFullLoadUntilAfterListen": false
    }
  }
}
```

> 来源：OpenClaw `docs/plugins/architecture.md` §"Package packs" + §"Channel catalog metadata"。

### 12.5.4 Manifest-first 的意义

在 `register(api)` **执行前**，能做的事：

- CLI 帮助 / UI 列表 / 搜索
- `plugins doctor` 做 compat 校验
- 启用决策（`plugins.enabled`、slot 占位）
- 显示 setup 向导（auth env 变量、onboarding choices）
- 第一波 narrow 加载（只加载拥有当前命令的插件）

这对**启动时间**至关重要。

> 来源：OpenClaw §"Manifest-first behavior"；Claude Code `src/services/plugins/PluginInstallationManager.ts`（背景安装 + 启动不阻塞）。

## 12.6 分发（Distribution）

### 12.6.1 分发渠道矩阵

| 渠道 | 信任模型 | 速度 | 适合 |
|---|---|---|---|
| **Built-in** | 核心等价信任 | 启动即用 | SDK 发布团队维护的关键扩展（默认 Anthropic/OpenAI/Google provider、核心工具） |
| **Bundled（Workspace-local）** | 开发时信任（工作区 `plugins/`）| 启动即用 | 项目私有插件、hotfix |
| **Marketplace** | 按 marketplace 策略 | 首次需要 clone | 官方市场 + 社区市场 |
| **Direct Install** | 按来源校验 | 需下载 | 临时试用、一次性集成 |
| **MCPB Bundle**（`.mcpb`/`.dxt`） | 数字签名 + 审核 | 需下载 | 纯 MCP server 的离线分发 |

### 12.6.2 Marketplace 源类型

`MarketplaceSource` = 以下 9 种之一：

| `source` | 字段 | 说明 |
|---|---|---|
| `url` | `url` + optional `headers` | 直链 `marketplace.json` |
| `github` | `repo: 'owner/repo'` + `ref?` + `path?` + `sparsePaths?` | GitHub 仓库；支持 sparse-checkout |
| `git` | `url` + `ref?` + `path?` + `sparsePaths?` | 任意 git；含 Azure DevOps / CodeCommit（无 `.git` 后缀） |
| `npm` | `package` | npm 包内含 manifest |
| `file` | `path` | 本地 JSON |
| `directory` | `path` | 本地目录含 `.octopus-plugin/marketplace.json` |
| `hostPattern` | `hostPattern` | 正则，用于 `strictKnownMarketplaces` 批量放行 |
| `pathPattern` | `pathPattern` | 正则，用于批量放行本地目录 |
| `settings` | `name` + `plugins[]` + `owner?` | 内联声明于 settings.json |

> 来源：Claude Code `MarketplaceSourceSchema`（discriminatedUnion）。

### 12.6.3 Plugin 源类型

`PluginSource` = 以下之一：

- 相对路径（指向 marketplace 根内的子目录）
- `{ source: 'npm', package, version?, registry? }`
- `{ source: 'pip', package, version?, registry? }`
- `{ source: 'url', url, ref?, sha? }`
- `{ source: 'github', repo, ref?, sha? }`
- `{ source: 'git-subdir', url, path, ref?, sha? }`

> 来源：Claude Code `PluginSourceSchema`。

### 12.6.4 版本锁

- **`semver`**：插件 manifest 声明版本 + marketplace 声明兼容 range
- **`gitSha`**（40 字符小写）：市场条目可锁定到具体 commit；锁定后升级必须显式改动
- **MCPB**：哈希 + 可选签名（与 `.dxt` 等价的封装格式）

> 来源：Claude Code `gitSha` 规则 + `PluginMarketplaceEntrySchema`。

### 12.6.5 MCPB（MCP Bundle）

专门打包 **只做 MCP Server** 的场景，避免每次安装都需要 npm/pip：

- 扩展名：`.mcpb` 或 `.dxt`
- 内容：`manifest.json` + MCP server 二进制 / JS / Python 脚本
- 安装：`octopus plugins install bundle ./my.mcpb`
- 校验：文件哈希 + 可选签名

> 来源：Claude Code `McpbPath` + `mcpbHandler.ts`。

## 12.7 版本与依赖

### 12.7.1 `compat.pluginApi` 协商

每个插件声明 `compat.pluginApi: '^1.0.0'`。加载时：

1. 比较 `SDK_PLUGIN_API_VERSION` 与声明 range
2. **硬不兼容**（major 版本不同）→ 加载失败，UI 显示升级提示
3. **软兼容**（minor/patch）→ 加载成功，允许 deprecated 警告

> 来源：OpenClaw `compat.pluginApi` + `minGatewayVersion`。

### 12.7.2 插件依赖

```jsonc
{
  "name": "my-plugin",
  "dependencies": [
    "core-tools",                 // 同 marketplace
    "openai@official-marketplace" // 跨 marketplace
  ]
}
```

- 启动时拓扑排序（循环依赖 → 拒绝）
- 依赖不可用 → `dependency-unsatisfied` 错误（`not-enabled` / `not-found`）
- 依赖版本：暂不实现（semver range on dependency 留给 v2）

> 来源：Claude Code `DependencyRefSchema`、`dependency-unsatisfied` 错误。

### 12.7.3 Slot（插槽）

某些扩展点**只能有一个活跃实现**（单选），称为 **slot**：

| Slot | 默认 | 说明 |
|---|---|---|
| `contextEngine` | `builtin-default` | 会话上下文引擎（见 §2） |
| `memoryBackend` | `local-sqlite` | 长期记忆后端 |
| `primaryProvider` | 由 model system 决定 | — |

`plugins.slots.<name> = '<plugin-id>'` 显式选定；其他竞争者自动禁用。

> 来源：OpenClaw `plugins.slots.contextEngine`；Hermes 事实上的单 slot 设计（`plugins/context_engine/` + `plugins/memory/<name>/`）。

## 12.8 生命周期（Lifecycle）

```
┌────────┐   ┌──────────┐   ┌──────────┐   ┌────────┐   ┌──────────┐
│discover│ → │enablement│ → │load      │ → │register│ → │expose    │
│        │   │+ validate│   │(in-proc) │   │(api.*) │   │(registry)│
└────────┘   └──────────┘   └──────────┘   └────────┘   └──────────┘
     │           │                                            │
     └──────manifest only──────┘                              │
                                                              ▼
                                                     ┌────────────────┐
                                                     │runtime consume │
                                                     │(core / other)  │
                                                     └────────────────┘
```

### 12.8.1 Discover

候选来源：

1. SDK 内置插件（`bundled/`）
2. Marketplace 缓存（`$OCTOPUS_HOME/plugins/cache/`）
3. Workspace 本地目录（`<workspace>/.octopus/plugins/`）
4. 用户全局扩展目录（`~/.octopus/plugins/`）
5. 显式 `plugins.load.paths[]`

**按优先级合并**；重复 `id` 时靠后者遮蔽（workspace 可临时替换 bundled，用于本地开发热修）。

> 来源：OpenClaw §"Architecture overview" step 1 + "workspace plugin shadows bundled" 规则。

### 12.8.2 Enablement + Validate

```
for each candidate:
  1. parse plugin.json (zod 校验)
  2. compat check (plugin-api range)
  3. security gates:
     - path escape check
     - world-writable reject (非 bundled)
     - ownership/signature check
  4. allowlist / denylist check (plugins.allow / plugins.deny)
  5. slot assignment (若占用 slot)
  6. dependency resolution
  7. produce: enabled | disabled | blocked
```

### 12.8.3 Load

- **Native plugin**：in-process 加载（Node `import` / Python `importlib`）
  - 用 `jiti`（Node）/ 动态 loader（Python）做 TS/ESM 兼容
  - `register(api)`（或 legacy alias `activate(api)`）被调用**一次**
- **MCPB bundle**：解压到 cache，按 manifest 启动 MCP server 子进程

### 12.8.4 Register

在 `register(api)` 里，插件可调用：

- `api.registerTool(...)` / `api.registerSkill(...)` / `api.registerCommand(...)` / ...
- `api.registerProvider({ ...44 optional runtime hooks })`（见 `11-model-system.md`）
- `api.registerChannel(...)` / `api.registerHttpRoute(...)` / ...
- `api.runtime.*`（访问共享能力：`tts` / `mediaUnderstanding` / `webSearch` / `subagent` / `imageGeneration`）

每次注册 registry 做：

- id 唯一性检查（同 category 内）
- schema 校验
- 产生 registration 事件（可观测）

### 12.8.5 Expose

核心（以及其它插件）读 registry：

- 工具：`registry.tools.list()` / `registry.tools.get(name)`
- Provider：`registry.providers.getForCapability('text')` → `[...]`
- Channel：`registry.channels.get(channelId)`

### 12.8.6 热重载（Hot Reload）

- `/reload-plugins` 命令：重新执行 discover → enablement → load → register
- 重新加载前：调用各插件的 `onUnload`（若提供）
- **活跃 session** 不受影响（符合本仓 `AGENTS.md` §Runtime Config 规则：live session 不跟随磁盘变动）
- 新 session 享有新插件

> 来源：Claude Code `performBackgroundPluginInstallations` + `/reload-plugins` UX。

### 12.8.7 禁用与卸载

- `/plugin disable <id>` → 触发 `onDisable` + 从 registry 移除
- 已注册的**服务/HTTP route/channel** 必须可被清理（插件需提供 `stop()`）
- 卸载：删除 cache 目录；清除 `installedPlugins` 记录

## 12.9 Registry 设计

### 12.9.1 数据结构

```ts
type Registry = {
  plugins: Map<PluginId, LoadedPluginRecord>
  tools:    Map<string, ToolRegistration>
  skills:   Map<string, SkillRegistration>
  commands: Map<string, CommandRegistration>
  agents:   Map<string, AgentRegistration>
  hooks:    Map<HookEvent, HookRegistration[]>     // 多个可叠加
  mcpServers: Map<string, McpServerRegistration>
  lspServers: Map<string, LspServerRegistration>
  providers: CapabilityIndex                        // 见 11-model-system.md
  channels:  Map<string, ChannelRegistration>
  contextEngines: Map<string, ContextEngineFactory>
  memoryBackends: Map<string, MemoryBackendFactory>
  outputStyles: Map<string, OutputStyleRegistration>
  httpRoutes: RouteTable
  rpcHandlers: Map<Namespace, Methods>
  services:   Map<string, ServiceRegistration>
}
```

### 12.9.2 所有权与冲突

- 每个 `(category, id)` 唯一拥有
- 重复注册：拒绝 + emit `plugin-conflict` 诊断
- **例外**：`commands` 允许插件级命名空间（`/myplugin:about`）；`mcpServers` 检测到相同 command/URL 会自动去重（`mcp-server-suppressed-duplicate`）
- `plugins.allow` 中的 **workspace 插件**可以有意遮蔽 bundled 同 id（开发/热修）

> 来源：Claude Code `mcp-server-suppressed-duplicate`、OpenClaw §"workspace plugin shadows bundled"。

### 12.9.3 单向流

```
plugin module ──register()──▶ registry ──read()──▶ core
                              ▲
                              │ 只允许 register/unregister，不允许随意 mutate
```

核心**永远不**`import` 插件模块或 reach-into 插件内部。

## 12.10 安全与沙箱

### 12.10.1 执行模型

| 插件类型 | 执行位置 | 信任级别 | 例子 |
|---|---|---|---|
| **Built-in** | harness 进程内 | 核心等价 | 官方 Anthropic/OpenAI provider |
| **Bundled + Marketplace native** | harness 进程内 | 与核心相同 | 多数第三方 |
| **MCP Server (stdio/http)** | 独立子进程 / 远端 | 受 MCP 代理调度 | 大量第三方 MCP server |
| **MCPB Bundle** | 独立子进程 | 由 bundle 声明的运行时 | 纯 MCP 分发 |
| **Sandboxed**（v2 计划）| OS 级隔离 | 受沙箱策略 | 高风险插件 |

**默认 Native 插件 = 核心等价信任**。这是**业界常识**（Claude Code、OpenClaw 都如此），但必须在 UI / 安装流程**明确告知用户**。

> 来源：OpenClaw §"Execution model"："a native plugin bug can crash or destabilize the gateway; a malicious native plugin is equivalent to arbitrary code execution"。

### 12.10.2 安装期安全门

1. **路径逃逸检测**：所有 `extensions[]`、`setupEntry`、`source` 路径经 symlink 解析后必须留在插件根内
2. **世界可写文件拒绝**：非 bundled 插件的文件不能有 world-writable 权限
3. **Ownership check**：`uid` 与期望用户一致
4. **`--omit=dev --ignore-scripts`**：`npm install` / `pip install` 执行时禁用 lifecycle scripts 与 dev 依赖
5. **签名校验**（MCPB / 官方 marketplace entry）

> 来源：OpenClaw §"Load pipeline" 步骤 3；Claude Code `marketplace-blocked-by-policy` 错误 + `npm install --omit=dev --ignore-scripts`。

### 12.10.3 冒名防护

对官方市场保留名做白名单：

```
ALLOWED_OFFICIAL_MARKETPLACES = [
  'octopus-core', 'octopus-plugins-official',
  'octopus-community-featured', ...
]
```

拒绝策略（Claude Code 同款）：

- **非 ASCII 字符**（homograph 攻击，如用 Cyrillic `а` 假冒）→ 直接拒绝
- **命名模式**（`/(official[^a-z]*(octopus|...)|...)/i`）→ 拒绝第三方使用
- **保留名必须来自官方 Git 组织**（`github.com/octopus-io/*`）

> 来源：Claude Code `isBlockedOfficialName` + `BLOCKED_OFFICIAL_NAME_PATTERN` + `validateOfficialNameSource`。

### 12.10.4 企业策略

```yaml
plugins:
  enabled: ['core-tools', 'openai', 'anthropic']
  allow: ['github-copilot', 'gitlab']
  deny: ['risky-plugin']
  strictKnownMarketplaces:
    - { source: github, repo: octopus-io/plugins }
    - { source: hostPattern, hostPattern: '^github\\.corp\\.example\\.com$' }
  blockedMarketplaces:
    - 'community-unverified'
  requireSignature: true     # MCPB 必须有签名
  installScriptsAllowed: false
```

> 来源：Claude Code `marketplace-blocked-by-policy`、`strictKnownMarketplaces`、`blockedMarketplaces`；OpenClaw `plugins.allow` 语义（trusts plugin ids）。

### 12.10.5 凭据隔离

遵循 §6.8 与 §11.8：

- 插件**不**持有长期凭据明文
- 插件声明 `setup.providerAuthEnvVars: ['OPENAI_API_KEY']` → SDK 从 keychain 解析 → 注入给 plugin runtime（短生命周期）
- 敏感凭据**不能**写入 `plugin.settings`（持久化）

### 12.10.6 HTTP Route 的 auth 强制声明

插件暴露 HTTP Route 时必须显式声明 `auth: 'gateway' | 'plugin'`：

- `'gateway'`：走核心 auth（但 shared-secret 模式下 scopes 固定为 `operator.write`；不自动提升）
- `'plugin'`：插件自管 auth（webhook 签名校验等）

核心 admin namespace（`config.*` / `exec.approvals.*` / `wizard.*` / `update.*`）**永远** reserved，不让插件占用。

> 来源：OpenClaw §"Gateway HTTP routes"。

## 12.11 错误分类

以 Zod-style discriminated union 表达所有插件相关错误，支持**精确 UI 展示**与**机读诊断**：

```ts
type PluginError =
  | { type: 'path-not-found',            source, plugin?, path, component }
  | { type: 'git-auth-failed',           source, plugin?, gitUrl, authType }
  | { type: 'git-timeout',               source, plugin?, gitUrl, operation }
  | { type: 'network-error',             source, plugin?, url, details? }
  | { type: 'manifest-parse-error',      source, plugin?, manifestPath, parseError }
  | { type: 'manifest-validation-error', source, plugin?, manifestPath, validationErrors }
  | { type: 'plugin-not-found',          source, pluginId, marketplace }
  | { type: 'marketplace-not-found',     source, marketplace, availableMarketplaces }
  | { type: 'marketplace-load-failed',   source, marketplace, reason }
  | { type: 'marketplace-blocked-by-policy', source, marketplace, blockedByBlocklist, allowedSources }
  | { type: 'mcp-config-invalid',        source, plugin, serverName, validationError }
  | { type: 'mcp-server-suppressed-duplicate', source, plugin, serverName, duplicateOf }
  | { type: 'mcpb-download-failed',      source, plugin, url, reason }
  | { type: 'mcpb-extract-failed',       source, plugin, mcpbPath, reason }
  | { type: 'mcpb-invalid-manifest',     source, plugin, mcpbPath, validationError }
  | { type: 'lsp-config-invalid',        source, plugin, serverName, validationError }
  | { type: 'lsp-server-start-failed',   source, plugin, serverName, reason }
  | { type: 'lsp-server-crashed',        source, plugin, serverName, exitCode, signal? }
  | { type: 'hook-load-failed',          source, plugin, hookPath, reason }
  | { type: 'component-load-failed',     source, plugin, component, path, reason }
  | { type: 'dependency-unsatisfied',    source, plugin, dependency, reason: 'not-enabled'|'not-found' }
  | { type: 'compat-mismatch',           source, plugin, required, actual }
  | { type: 'plugin-cache-miss',         source, plugin, installPath }
  | { type: 'plugin-conflict',           source, plugin, conflictsWith, category, id }
  | { type: 'generic-error',             source, plugin?, error }
```

每类错误都有一个 `getDisplayMessage(err)`（类 Claude Code `getPluginErrorMessage`）与 `suggestFix(err)`（提供可执行的修复提示，对应 C2 ACI 要求）。

> 来源：Claude Code `src/types/plugin.ts` PluginError（~22 种） + `getPluginErrorMessage`。

## 12.12 诊断（Plugin Doctor）

```bash
$ octopus plugins doctor
✓ Marketplace 'octopus-official' up-to-date (commit a1b2c3…)
✓ Plugin 'openai@official' loaded (hybrid-capability: text+speech+image+vision)
⚠ Plugin 'my-tool@local' uses 'before_agent_start' (legacy) — migrate to 'before_model_resolve'
✗ Plugin 'broken@community' compat-mismatch: requires pluginApi ^2 but host is 1.x

$ octopus plugins inspect openai@official
id:           openai@official
shape:        hybrid-capability
capabilities:
  - text-inference   (provider.openai)
  - speech           (speech.openai)
  - realtime-voice   (realtime-voice.openai)
  - media-understanding (openai)
  - image-generation (openai)
hooks:        0
commands:     0
mcpServers:   0
sources:      github:octopus-io/plugin-openai#v1.2.3 (sha: abc…)
```

> 来源：OpenClaw `octopus doctor` + `plugins inspect` 命令；§"Compatibility signals" 的 4 级（valid / advisory / legacy warning / hard error）。

## 12.13 SDK 导出边界

### 12.13.1 公共 SDK 入口

发布 `@octopus/plugin-sdk`（TypeScript）与 `octopus-plugin-sdk`（Python）包。**只**通过子路径导出：

```ts
// 推荐（narrow, 稳定）
import type { OctopusPluginDefinition } from '@octopus/plugin-sdk/plugin-entry'
import { buildMemorySystemPromptAddition } from '@octopus/plugin-sdk/core'
import { createOpenAiCompatibleProvider } from '@octopus/plugin-sdk/provider-shared'
import type { ChannelDirectoryEntry } from '@octopus/plugin-sdk/directory-runtime'

// 禁止（内部实现，不稳定）
import { _internal } from '@octopus/plugin-sdk/internal/*'       // ❌
import { foo } from '@octopus/plugin-sdk'                        // ❌（monolithic barrel）
```

> 来源：OpenClaw §"Plugin SDK import paths"（明确禁止 `openclaw/plugin-sdk` 根 barrel，鼓励 narrow subpath）。

### 12.13.2 插件之间不互相导入

- Plugin A 不能 `import` Plugin B 的 `src/*`
- 如需共享，通过 **Plugin B 的 `api.ts` barrel** 或者上升为 SDK 公共能力

### 12.13.3 SDK 版本化

- `compat.pluginApi` 声明的 range 与 SDK package 的 `package.json:version` 对齐
- Major 突破必须：SDK CHANGELOG + migration guide + deprecation 警告一个 major 版本周期

## 12.14 插件内部文件契约

Octopus 约定插件根内的标准目录：

| 目录 | 内容 | 加载时机 |
|---|---|---|
| `hooks/` | `hooks.json` + 各 hook 脚本 | 启动注入 |
| `commands/` | `*.md`（单 command）或子目录 SKILL 式 | 启动注册 |
| `agents/` | `*.md` Agent Definition（frontmatter + body） | 启动注册 |
| `skills/<name>/` | `SKILL.md` + 资产 | 按需加载 |
| `output-styles/` | `*.md` or `*.json` | 启动注册 |
| `mcp-servers/` | MCP 配置 JSON / `.mcpb` | 启动注入 |
| `i18n/` | `<lang>.json` 文本资源 | 按需加载 |
| `assets/` | 图片、图标 | UI 时懒加载 |

`plugin.json` 里对应字段**优先**于默认目录（见 §12.5.2）。

## 12.15 Octopus 落地约束

### 12.15.1 与本仓治理规则的映射

| 本仓规则 | 插件体系对应约束 |
|---|---|
| `AGENTS.md` §Frontend Governance | 插件 UI 若要出现在桌面端，必须走 `@octopus/ui` 组件；不能在 business 页直接绘制插件自带 UI |
| `AGENTS.md` §Request Contract Governance | 若插件需要通过 `/api/v1/*` 暴露 HTTP，必须在 `contracts/openapi/src/**` 定义 |
| `AGENTS.md` §Persistence Governance | 插件的 metadata/index 可入 `data/main.db`；但**不得**把插件源码、凭据明文、大文件存 DB |
| `AGENTS.md` §Runtime Config | `plugins.enabled` / `plugins.slots` / `plugins.allow` 按 user < workspace < project 合并；live session 不跟随 |
| `AGENTS.md` §Config Snapshot | 每个 session 记录 `pluginsSnapshot: { id → (source, sha, version) }`，保证回放一致 |
| C1 Prompt Cache 稳定性 | 插件注册的工具/Skill/命令**顺序确定**（`id` 升序）；注册时间早晚不影响运行时顺序 |
| C3 凭据零暴露 | 插件配置里的敏感字段必须用 `SecretRef`；不能持久化明文 |

### 12.15.2 文件路径

```
<workspace>/
  .octopus/
    plugins/                   # workspace-local 插件（开发/私有）
      <id>/
config/
  plugins.json                 # workspace 级启用/禁用/slot
  (用户级在 ~/.octopus/config/plugins.json)
data/
  main.db
    tables:
      installed_plugins        # id, source, sha, version, installed_at, enabled
      plugin_marketplaces      # name, source, last_sync_at
      plugin_registrations     # 启动时由 Registry 快照进来；只读
runtime/
  plugin-cache/                # marketplace clone 缓存
  mcpb/                        # MCPB 解压缓存
logs/
  plugin-audit.jsonl           # 安装/启用/禁用/卸载审计
```

### 12.15.3 Host 一致性

Tauri 宿主与 Browser 宿主必须暴露**相同的** `listPlugins` / `installPlugin` / `enablePlugin` / `reloadPlugins` adapter 合约。宿主特定代码（文件系统访问差异等）藏在 adapter 内。

> 来源：本仓 `AGENTS.md` §Host consistency rule。

### 12.15.4 审批流（敏感操作）

- 安装 marketplace 插件 → 显示来源 + 权限摘要 + 签名状态 → 用户审批
- 启用 `hook-only` 插件 → 额外警告（它能拦截所有工具调用）
- 启用 `auth: 'plugin'` HTTP Route → 额外警告（插件自管 auth）

## 12.16 实施优先级（不限本轮）

| 阶段 | 交付 |
|---|---|
| **M0（骨架）** | Manifest schema + Registry + Local bundled 加载 + built-in plugins 注册表 |
| **M1（扩展点）** | tools/skills/commands/agents/hooks/mcpServers/channels 的 register* API |
| **M2（分发）** | Marketplace（github source 先行）+ gitSha 锁版本 + `/reload-plugins` |
| **M3（Provider 迁移）** | 把模型层（第 11 章）所有 provider 拆成 provider plugin；核心不再硬编码 |
| **M4（安全）** | 保留名防护 + homograph + 企业策略（`strictKnownMarketplaces` / `blockedMarketplaces`）|
| **M5（生态）** | MCPB 打包与签名、CLI（`plugins list/install/enable/inspect/doctor`）、UI |

## 12.17 反模式

| 反模式 | 症状 | 纠正 |
|---|---|---|
| **核心 reach-into 插件** | `import('openai-plugin/internal')` 在核心 | 通过 Registry + `api.runtime` |
| **vendor switch in core** | `if provider==='X' else if...` | 让 provider plugin 用 `registerProvider` 的 runtime hooks（见 §11） |
| **无 manifest 即加载** | 启动路径依赖 import-time 副作用 | manifest-first；`register()` 是唯一执行入口 |
| **root barrel import** | `import { X } from 'octopus/plugin-sdk'` | 用 narrow subpath |
| **凭据明文入 settings** | `plugin.settings.apiKey = 'sk-...'` | `SecretRef` + keychain |
| **插件改核心全局** | 插件直接 mutate core 状态 | 只能 `register(...)`；双向改动需 core 明确契约 |
| **无 `compat.pluginApi`** | 发布后核心升级就炸 | 强制声明；缺失则拒绝加载 |
| **跨插件深导入** | `import 'plugin-b/src/internal'` 在 plugin A | 只通过 `plugin-b/api.ts` 或 SDK 上升 |
| **enable-by-import** | 插件放目录就激活 | 必须走 `plugins.enabled` / slot 显式决定 |
| **插件注册顺序影响输出** | 不同启动顺序给 LLM 不同 prompt | 按 `id` 排序稳定化；见 C1 |

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| **Claude Code** `src/types/plugin.ts` | `BuiltinPluginDefinition` / `LoadedPlugin` / `PluginError` 22 型 / `PluginComponent` |
| **Claude Code** `src/plugins/builtinPlugins.ts` + `bundled/index.ts` | built-in 注册表 + `{name}@builtin` 命名 |
| **Claude Code** `src/utils/plugins/schemas.ts` | `PluginManifestSchema` 完整 Zod / `MarketplaceSourceSchema` 9 型 / `PluginSourceSchema` / `gitSha` / `ALLOWED_OFFICIAL_MARKETPLACE_NAMES` / `BLOCKED_OFFICIAL_NAME_PATTERN` / `validateOfficialNameSource` / MCPB path |
| **Claude Code** `src/utils/plugins/*.ts` | marketplace manager / plugin loader / reconciler / refresh / blocklist / policy / versioning / cache / dependency resolver |
| **Claude Code** `src/services/plugins/PluginInstallationManager.ts` | 背景安装、marketplace reconcile、不阻塞启动 |
| **OpenClaw** `docs/plugins/architecture.md` | 四层架构 / 12 类 capability / 4 种 shape / ownership 模型 / manifest-first / load pipeline / registry 单向流 / 44 个 provider runtime hook / HTTP route auth 规则 |
| **OpenClaw** `extensions/CLAUDE.md` | 扩展边界 / 公共契约 / 禁止深导入 / 命名空间规则 |
| **OpenClaw** `packages/plugin-package-contract/src/index.ts` | `ExternalPluginCompatibility` / `openclaw.compat.pluginApi` / `openclaw.build.openclawVersion` / `minGatewayVersion` |
| **OpenClaw** `packages/plugin-sdk/src/*` | plugin-entry / plugin-runtime / provider-entry / channel-contract / provider-shared 子路径划分 |
| **OpenClaw** `extensions/<90+ 插件>` | 实际落地样本：provider(openai/anthropic/google/deepseek/minimax/moonshot/qwen/...)、channel(telegram/discord/slack/matrix/whatsapp/signal/...)、feature(voice-call/image-generation-core/...) |
| **Hermes** `plugins/context_engine/` + `plugins/memory/<name>/` | 轻量 slot 式插件范式（`plugins.slots.contextEngine` / `plugins.slots.memoryBackend` 的灵感来源） |
| **Hermes** `plugins/memory/mem0/__init__.py` | 具体 provider plugin 示例：env + `$HERMES_HOME/<name>.json` 双源配置、Circuit breaker 模式 |
| 本仓 `docs/sdk/03-tool-system.md` / `07-hooks-lifecycle.md` / `11-model-system.md` | 扩展点的能力契约定义（插件注册的接收端） |
| 本仓 `AGENTS.md` | host 一致性 / runtime config / persistence / 合规约束 |
