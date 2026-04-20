# 06 · 权限模型、审批、沙箱与网络隔离

> "Effective sandboxing requires both filesystem and network isolation."
> — [Anthropic · Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)（2025）

本章解决"如何让代理既**自主**又**不炸**"这个核心矛盾。

## 6.1 Why：approval fatigue 与 autonomy dial

### 6.1.1 两种极端

- **全程弹窗**：每个工具调用都问用户 → "approval fatigue"；用户麻木 → 最后一律点 YES → 等于没有审批
- **全部放开**：bypass 模式在不可信任环境等于裸奔

### 6.1.2 "Autonomy Dial"

Anthropic 的解法是一条**可调刻度**：

```
手动审批 ────→ 分类审批 ────→ 安全沙箱+允许自动 ────→ Bypass
                             ↑ 默认推荐
```

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Dial in how autonomous Claude is"。

## 6.2 权限模式（PermissionMode）

Claude Code 的真实枚举定义在 `restored-src/src/types/permissions.ts:16-36`（再由 `Tool.ts` re-export）：

```16:36:docs/references/claude-code-sourcemap-main/restored-src/src/types/permissions.ts
export const EXTERNAL_PERMISSION_MODES = [
  'acceptEdits',
  'bypassPermissions',
  'default',
  'dontAsk',
  'plan',
] as const

export type ExternalPermissionMode = (typeof EXTERNAL_PERMISSION_MODES)[number]

export type InternalPermissionMode = ExternalPermissionMode | 'auto' | 'bubble'
export type PermissionMode = InternalPermissionMode

export const INTERNAL_PERMISSION_MODES = [
  ...EXTERNAL_PERMISSION_MODES,
  ...(feature('TRANSCRIPT_CLASSIFIER') ? (['auto'] as const) : ([] as const)),
] as const satisfies readonly PermissionMode[]
```

把它分成"外部可见"与"内部使用"两栏：

**外部可见**（`ExternalPermissionMode` — 可出现在 `settings.json` / CLI / recovery）：

| Mode | 行为 | 典型场景 |
|---|---|---|
| `default` | 写入/破坏性工具弹窗审批；只读工具放行 | 初次使用 / 高风险任务 |
| `acceptEdits` | 文件写入默认允许；执行类仍审批 | 熟悉的编辑任务 |
| `bypassPermissions` | 全部放行 | 已在外层沙箱、CI/CD 受控环境 |
| `dontAsk` | 非交互场景下"不弹窗"；配 allowlist/denylist 使用 | 后台任务 / headless 运行 |
| `plan` | 所有有副作用工具禁用 | 探索 / 规划阶段 |

**内部使用**（`InternalPermissionMode` 额外取值，不进入 `settings.json`）：

- `auto` — Claude Code 的分类器自动模式，由 feature flag `TRANSCRIPT_CLASSIFIER` 闸控（详见 §6.12）。
- `bubble` — 子代理向父代理"冒泡"请求决策时的中间态。

**Octopus SDK 首版采用子集**：`default` / `acceptEdits` / `bypassPermissions` / `plan`。`dontAsk` 在非交互 session 启用；`auto` / `bubble` 保留为后续增量。

## 6.3 权限规则来源（Rules by Source）

一条规则可来自多个来源，**合并优先级**需确定：

```
user-session > project-settings > workspace-settings > defaults
```

规则集形状与 Claude Code 对齐（`restored-src/src/types/permissions.ts:54-79, 419-421`）：

```54:79:docs/references/claude-code-sourcemap-main/restored-src/src/types/permissions.ts
export type PermissionRuleSource =
  | 'userSettings'
  | 'projectSettings'
  | 'localSettings'
  | 'flagSettings'
  | 'policySettings'
  | 'cliArg'
  | 'command'
  | 'session'

export type PermissionRuleValue = {
  toolName: string
  ruleContent?: string
}

export type PermissionRule = {
  source: PermissionRuleSource
  ruleBehavior: PermissionBehavior    // 'allow' | 'deny' | 'ask'
  ruleValue: PermissionRuleValue
}
```

聚合视图（`ToolPermissionRulesBySource` — 按 `source` 分组的 `ruleContent[]`）：

```ts
type ToolPermissionRulesBySource = {
  [S in PermissionRuleSource]?: string[]
}

// 在 ToolPermissionContext 中按"行为类别"聚合：
//   alwaysAllowRules / alwaysDenyRules / alwaysAskRules / strippedDangerousRules
//   每一项都是 ToolPermissionRulesBySource。
```

> 来源：`restored-src/src/types/permissions.ts` + `restored-src/src/Tool.ts:123-138`。`ToolPermissionContext` 里的 `alwaysAllowRules`、`alwaysDenyRules`、`alwaysAskRules`、`strippedDangerousRules` 都是 `ToolPermissionRulesBySource` 形状。`ruleContent` 是一个与特定 `toolName` 关联的字符串模式（例如 `Bash` 的命令前缀、文件路径 glob）。

## 6.4 关键设计：`canUseTool`

**单一决策入口**：

```ts
type CanUseToolFn = (
  tool: Tool,
  parsedInput: unknown,
  ctx: ToolUseContext
) => Promise<PermissionDecision>

type PermissionDecision =
  | { kind: 'allow' }
  | { kind: 'ask', question: string, options?: string[] }
  | { kind: 'deny', reason: string }
```

决策流程：

```
1. match alwaysDenyRules → deny    (硬红线)
2. match alwaysAllowRules → allow
3. tool.permissionPolicy(input, mode) →
      a. explicit decision
      b. else fall through
4. mode=='bypass' → allow
5. mode=='plan' && tool.writes → deny
6. match alwaysAskRules → ask
7. default by mode:
     default:      ask if tool.writes, else allow
     acceptEdits:  allow if tool.writes or read, ask if shell_exec
     bypass:       allow
```

> 来源：Claude Code `restored-src/src/hooks/toolPermission/useCanUseTool.tsx` + 相关策略。

## 6.5 命令级权限（Shell）

`shell_exec` 特别敏感；SDK 必须提供命令级审批：

| 形态 | 示例 | 默认 |
|---|---|---|
| 命令前缀 | `git status` | 允许 |
| 命令前缀 | `npm install` | ask（默认） |
| Glob 路径 | `rm -rf /*` | deny（硬规则） |
| Regex | `sudo\s.*` | deny |

对 `Bash` 工具，模型提交的命令需：

1. 命令词 allowlist（如 `git`, `ls`, `cat`...）
2. 参数 glob/regex（避免 `rm -rf` 的误用）

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Configure permissions"；Claude Code `restored-src/src/tools/BashTool/*` 的前置检查。

## 6.6 文件级权限（Filesystem）

### 6.6.1 工作目录白名单

`ToolUseContext.additionalWorkingDirectories: Map<path, {writable: bool}>`

- 默认：`projectRoot` 可读写
- 模型主动请求写额外目录：通过 `AskUserQuestion`
- 每个目录条目标明 `writable`

### 6.6.2 硬禁区（永不放行）

- `/etc/**`, `/root/**`, `/boot/**`（除 root 明确允许）
- `~/.ssh/**`, `~/.aws/**`（凭据）
- Sandbox 之外的任何 path（对 sandboxed 模式）

> 来源：[Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing)。

## 6.7 沙箱运行时（Sandbox Runtime）

### 6.7.1 两层目标

Anthropic 明确：

> **"Effective sandboxing requires both filesystem and network isolation."**

只做 FS 不做网络 → 代理可以把代码偷到外网；反之亦然。

### 6.7.2 OS 级原语（平台）

| 平台 | 工具 | 能力 |
|---|---|---|
| Linux | `bubblewrap` + `seccomp` | 命名空间、只读绑定、系统调用过滤 |
| macOS | `sandbox-exec`（seatbelt） | profile 语言，限制 FS + 网络 |
| Windows | AppContainer / Job Object | 受限；需额外工程 |
| 云端 | Firecracker / gVisor | 强隔离、启动快 |

Anthropic 开源 reference：[github.com/anthropic-experimental/sandbox-runtime](https://github.com/anthropic-experimental/sandbox-runtime)

### 6.7.3 Sandboxed Bash Tool（beta）

Claude Code 提供了 `shell_exec` 的**沙箱版本**：

- 跑在隔离环境
- FS 白名单严格
- 网络限制严格

因为是"安全的"，它**默认 auto-approve**（Autonomy Dial 向右）。

> 来源：[Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) §"Sandboxed bash tool"。

### 6.7.4 Cloud Sandbox（Claude Code on the web）

把整个任务跑在云端：

- 每次会话独立容器（cattle）
- 网络出口默认 allowlist（pip/npm 镜像、特定 API）
- Git 操作通过**代理**（见 §6.9）

## 6.8 网络策略（Egress Control）

### 6.8.1 Allowlist 而非 Denylist

- 默认 deny all outbound
- allowlist 常见域名（npmjs.com, pypi.org, github.com, docker.io, ...）
- 用户可追加域

### 6.8.2 网络代理中的凭据注入

**沙箱进程本身无凭据**。代理层：

1. 收到 HTTP/HTTPS 出站请求
2. 按目标域匹配到 vault 中的凭据
3. 注入 `Authorization` header（或 OAuth）
4. 转发到真正后端
5. 回包 sanitize（去掉 Set-Cookie 等服务端内部 header）

### 6.8.3 JWT 模式（Anthropic claude.ai 内部实现）

来自 [Claude Hidden Toolkit](../references/Claude_Hidden_Toolkit.md) 的"Egress Proxy JWT"：

- 沙箱向代理发请求时带短时效 JWT（签署：session_id + 目标域）
- 代理校验 JWT；否则拒绝
- 防止横向越权访问他人凭据

## 6.9 Git Proxy：凭据的零暴露范式

**问题**：模型要 `git clone` / `git push`，但又不能给它令牌。

**解**：

1. 在 clone 阶段写 `.git/config`：

   ```ini
   [http]
     proxy = http://git-proxy.local:8080
   ```

2. 代理层保留真 git 令牌，接收请求时注入。
3. 沙箱内 `git push` 一切如常，**模型永远看不到令牌**。

> 来源：[Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) §"Git integration"；[Managed Agents](https://www.anthropic.com/engineering/managed-agents) §"Credentials never enter the sandbox"。

## 6.10 资源限额（ResourceLimits）

每个 Hands 必须配：

| 维度 | 默认 | 超限行为 |
|---|---|---|
| CPU time | 300s / 工具调用 | SIGTERM → SIGKILL |
| Wall time | 120s / 工具 | 同上 |
| Memory | 1 GB / 沙箱 | OOM-kill |
| 文件描述符 | 4096 | `errno=EMFILE` |
| 进程数 | 32 | `fork` 被拒 |
| 出站请求数 | 100 / 分钟 | 429 |
| Shell 最大输出 | 30_000 字符（`BASH_MAX_OUTPUT_DEFAULT`，硬上限 150_000） | 截断 + 提示 |

> 来源：综合 Claude Code best practices + [Claude Code sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) + 行业常识。

## 6.11 Hooks 与权限系统

权限决定"能不能做"；Hooks 决定"做之前还要干啥"。经典搭配：

- `PreToolUse(shell_exec)` → 把命令写入审计日志
- `PreToolUse(fs_write)` → 跑 linter/formatter on staged content
- `PostToolUse(fs_write)` → 自动 `git add`
- `PreCompact` → 把关键决策冻结到 `NOTES.md`

> 见 [`07-hooks-lifecycle.md`](./07-hooks-lifecycle.md)。

## 6.12 Auto Mode（分类器自动化）

**概念**：用一个小分类器（LLM 或规则）替代每次弹窗：

- 输入：tool_use + 历史上下文
- 输出：`allow | ask | deny + reason`
- 可学习、可审计（所有决策入 event 流）

**前提**：必须**先有好的沙箱**。在无沙箱环境 Auto Mode = 裸奔。

**与 Claude Code 的关系**：Claude Code 把 Auto Mode 实现为**内部** `PermissionMode`（`'auto'`），由 feature flag `TRANSCRIPT_CLASSIFIER` 控制曝光（见 `types/permissions.ts:33-36`）；相关返回形状 `YoloClassifierResult`、`pendingClassifierCheck`、`PermissionDecisionReason.classifier` 也定义在同一文件。Octopus SDK 首版**不启用**该内部模式；当未来需要等效能力时，我们走**外部 hook + `canUseTool`** 方案，把分类器结果通过 `PermissionAskDecision.pendingClassifierCheck` 异步注入，保持与 Claude Code 数据模型的兼容。

> 来源：[Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) §"Auto mode"；`restored-src/src/types/permissions.ts:185-194, 330-397`。

## 6.13 危险操作的 "stripping" 机制

`ToolPermissionContext.strippedDangerousRules` 表示**被剥离的危险权限**：

- 即使用户配置了"总是允许"，某些危险组合仍会被 strip
- 例：`shell_exec + "rm -rf"` 的组合无法通过允许规则放行；只能单次 ask

> 来源：`restored-src/src/Tool.ts:131`（`ToolPermissionContext.strippedDangerousRules`），源类型定义在 `restored-src/src/types/permissions.ts:437`。

## 6.14 Octopus 落地约束

- 所有权限决策必须写 `event.permission_decision` 事件（审计 + UI 回放）
- 权限配置遵循 Octopus 的 runtime config 分层（user < workspace < project；见本仓 `AGENTS.md`）
- 沙箱后端可切换，但契约必须满足 §6.10 的资源限额
- 凭据存入 Octopus 现有的 secrets 存储（见 runtime config rules `Sensitive config values must not be written back...`）
- Git proxy 与 MCP OAuth 代理复用同一"凭据注入层"，不重复造轮子
- Bash 工具默认跑在 Tauri 子进程 + macOS seatbelt；跨平台实现参考 sandbox-runtime

## 6.15 常见反模式

| 反模式 | 症状 | 纠正 |
|---|---|---|
| **只沙箱 FS 不管网络** | 代理 exfiltrate 数据 | 同时隔离两层 |
| **凭据塞进环境变量** | 模型 `env` 一把全见 | 凭据在代理层注入，env 清空 |
| **审批规则全在硬编码** | 无法热调、不可审计 | Rules by Source + session override |
| **用默认模式跑 CI** | 卡在无人看的弹窗 | 非交互模式 + allowlist + 分类器 |
| **Sandbox 作为 Brain 的一部分** | sandbox 崩 → Brain 崩 | sandbox crash = tool error |

---

## 参考来源汇总（本章）

| 来源 | 用途 |
|---|---|
| [Claude Code Sandboxing](https://www.anthropic.com/engineering/claude-code-sandboxing) | FS+网络双隔离、sandbox bash tool、git proxy |
| [Claude Code best practices](https://www.anthropic.com/engineering/claude-code-best-practices) | Autonomy dial、permission modes、auto mode |
| [Managed Agents](https://www.anthropic.com/engineering/managed-agents) | 凭据零暴露 |
| Claude Code restored src `Tool.ts` | PermissionMode、ToolPermissionRulesBySource、strippedDangerousRules |
| Claude Code restored src `hooks/toolPermission/*` | canUseTool 流程 |
| [Claude Hidden Toolkit](../references/Claude_Hidden_Toolkit.md) | Egress proxy JWT 模式 |
| [anthropic-experimental/sandbox-runtime](https://github.com/anthropic-experimental/sandbox-runtime) | OS 级隔离参考实现 |
| 本仓 `AGENTS.md` §Runtime Config | secrets 存储、runtime config 层级 |
