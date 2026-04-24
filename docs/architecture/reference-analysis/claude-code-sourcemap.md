# Claude Code Sourcemap — Reference Analysis

> Research material only. Source is `docs/references/claude-code-sourcemap-main/restored-src/src`, reconstructed from the `cli.js.map` of npm package `@anthropic-ai/claude-code@2.1.88`, per the upstream `docs/references/claude-code-sourcemap-main/README.md`. Folder structure is **not** authoritative — treat it as evidence of runtime behavior only. This document records observed architectural patterns; it is not a design for Octopus.

## 1. Scope and evidence rules

- Every claim cites a concrete file path under `docs/references/claude-code-sourcemap-main/restored-src/src/...`.
- When a behavior is only circumstantial (e.g. gated behind a feature flag we cannot confirm at runtime), the claim is marked **Unverified**.
- Source code is not copied; only names, shapes, and interaction patterns are summarized.

## 2. Runtime envelope and entrypoints

- A dedicated bootstrap file `src/entrypoints/cli.tsx` handles argv before loading the full CLI. It routes fast paths (`--version`, `--dump-system-prompt`, `--claude-in-chrome-mcp`, `--chrome-native-host`, `--computer-use-mcp`, `--daemon-worker`, `remote-control`/`rc`/`sync`/`bridge`, `daemon`, `ps`/`logs`/`attach`/`kill`/`--bg`, template jobs `new`/`list`/`reply`, `environment-runner`, `self-hosted-runner`, `--worktree --tmux`) with dynamic imports, then delegates to `src/main.tsx` for the REPL/agentic path (`src/entrypoints/cli.tsx:33-302`).
- Two public agent entrypoints coexist:
  - CLI / REPL UI: Ink-based, entered via `src/main.tsx`; tool rendering is explicit in the `Tool` contract (`renderToolUseMessage`, `renderToolResultMessage`, `renderToolUseRejectedMessage`) (`src/Tool.ts:603-653`).
  - SDK / headless: public control and core schemas under `src/entrypoints/sdk/` (`controlSchemas.ts`, `coreSchemas.ts`, `coreTypes.ts`). The agent loop is also exposed via `src/QueryEngine.ts` (1295 lines) for programmatic consumption.
- Per-process bootstrap state (session id, cwd state, allowed setting sources, token budget counters, request metrics) lives in `src/bootstrap/state.ts` and is consumed from tools, query loop, and services.

## 3. Core abstractions

### 3.1 `Tool`

- Defined in `src/Tool.ts`. The interface is deliberately large and covers: input Zod schema, optional output schema, defer/always-load flags for prompt-budgeting tool search, permission preparation, validation, concurrency safety, read-only/destructive/open-world predicates, interrupt behavior, rendering (use/result/queued/rejected/error/grouped) as React nodes, and a reverse-mapping to a tool-result block (`mapToolResultToToolResultBlockParam`) (`src/Tool.ts:362-695`).
- Tools carry a MCP fingerprint (`mcpInfo`) and an explicit `maxResultSizeChars` budget (Infinity only for self-bounding tools like Read) (`src/Tool.ts:436-466`).
- A `buildTool(def)` helper fills fail-closed defaults: `isConcurrencySafe → false`, `isReadOnly → false`, `isDestructive → false`, `checkPermissions → allow`, `toAutoClassifierInput → ''`. Callers never need `?.()` fallbacks (`src/Tool.ts:757-792`).
- `ToolUseContext` is a thick handle passed into every tool call. It carries `abortController`, `readFileState`, callbacks to mutate app state, `handleElicitation` (used for URL elicitations with MCP error code `-32042`), notification/OS-notification sinks, denial tracking, content replacement state, `renderedSystemPrompt` for fork-cache sharing, and more (`src/Tool.ts:158-300`).

### 3.2 Tool registry and assembly

- `src/tools.ts` is the single source of truth for the built-in tool list via `getAllBaseTools()` (`src/tools.ts:193-251`). Inclusion is conditional through a mix of: `feature('FLAG')` (build-time DCE via `bun:bundle`, see top-of-file imports `src/tools.ts:1-157`), `process.env.USER_TYPE === 'ant'`, GrowthBook values, and runtime capability checks (`hasEmbeddedSearchTools()`, `isToolSearchEnabledOptimistic()`, `isWorktreeModeEnabled()`, `isAgentSwarmsEnabled()`, `isPowerShellToolEnabled()`).
- `assembleToolPool(permissionContext, mcpTools)` is the canonical combine step: it filters built-ins, filters MCP tools by deny rules, sorts both partitions by name, and deduplicates with `lodash uniqBy` so built-ins win name conflicts; comment explicitly explains this guards prompt-cache stability (`src/tools.ts:345-367`).
- Tool presets exist but are intentionally minimal: only `'default'` is defined (`src/tools.ts:161-183`).
- `getTools()` has a `CLAUDE_CODE_SIMPLE` "bare" mode that reduces the tool surface to `BashTool`, `FileReadTool`, `FileEditTool`; when REPL tool mode is enabled, primitives are hidden so they are only accessible through the REPL VM context (`src/tools.ts:271-327`).
- Some tools are lazy-required to break circular deps (`TeamCreateTool`, `TeamDeleteTool`, `SendMessageTool`), confirming the module graph is dense and feature-gated (`src/tools.ts:61-72`).

### 3.3 Query loop

- `src/query.ts` implements the agent loop as an async generator (`query → queryLoop`) yielding stream events, request-start events, messages, tombstones, and tool-use summaries (`src/query.ts:219-263`).
- State carried between iterations includes `messages`, `toolUseContext`, `autoCompactTracking`, `maxOutputTokensRecoveryCount`, `hasAttemptedReactiveCompact`, `pendingToolUseSummary`, `stopHookActive`, `turnCount`, and `transition` (`src/query.ts:202-217`).
- Inside each iteration, the observed order of context maintenance is deterministic (`src/query.ts:365-426`, comments lines 397-426):
  1. Tool-result budget enforcement (`applyToolResultBudget`) — persists overflow when `querySource` starts with `agent:` or `repl_main_thread`.
  2. Optional history snip (`HISTORY_SNIP` feature).
  3. Microcompact (possibly cached), possibly deferring a boundary message until after the API response.
  4. Context collapse projection (`CONTEXT_COLLAPSE` feature) before autocompact.
  5. Autocompact decision and, on trigger, `buildPostCompactMessages`.
- Tool orchestration is factored into `src/services/tools/toolOrchestration.ts`: `runTools` partitions the assistant's tool-use blocks into contiguous batches, concurrency-safe tools run in parallel (`runToolsConcurrently`), others run serially (`runToolsSerially`); concurrency-safe is determined by parsing input with the tool's Zod schema and calling `tool.isConcurrencySafe(input)` (`src/services/tools/toolOrchestration.ts:19-116`).
- Max concurrency is overridable via `CLAUDE_CODE_MAX_TOOL_USE_CONCURRENCY` with a default of 10 (`src/services/tools/toolOrchestration.ts:8-12`).
- `contextModifier` returned from non-concurrency-safe tool calls can mutate the next iteration's `ToolUseContext` (`src/Tool.ts:321-336`).

### 3.4 Subagent forking

- A subagent context is created via `src/utils/forkedAgent.ts` (`createSubagentContext`). `ToolUseContext` documents explicitly that:
  - `setAppState` is a no-op for async subagents to avoid polluting the root store (`src/Tool.ts:182-192`).
  - `setAppStateForTasks` exists as an always-shared escape hatch for session-scoped infrastructure (same block).
  - `localDenialTracking` is carried on context when `setAppState` is a no-op, so the denial counter still drives the prompting fallback (`src/Tool.ts:279-283`).
  - `renderedSystemPrompt` is frozen at fork time to share the parent's prompt cache; `forkSubagent` prevents GrowthBook cold→warm diverging the system prompt (`src/Tool.ts:294-299`).
- `src/tools/AgentTool/runAgent.ts` additionally accepts per-agent MCP specs (frontmatter `mcpServers` as either server-name reference or inline `{name: config}`), connects them at agent start, and registers a cleanup that only disconnects inline-created clients (referenced ones are memoized from the parent) (`src/tools/AgentTool/runAgent.ts:95-210`).

## 4. Agent typology

Three sources of agent definitions feed one merged list (`src/tools/AgentTool/loadAgentsDir.ts:186-221`):

1. **Built-in agents**: hard-coded `BuiltInAgentDefinition` objects with dynamic `getSystemPrompt(params)` (`src/tools/AgentTool/loadAgentsDir.ts:135-143`). Observed instances: `GENERAL_PURPOSE_AGENT`, `STATUSLINE_SETUP_AGENT`, `EXPLORE_AGENT`, `PLAN_AGENT`, `CLAUDE_CODE_GUIDE_AGENT`, `VERIFICATION_AGENT`; coordinator-mode sessions replace the list entirely with `getCoordinatorAgents()` (`src/tools/AgentTool/builtInAgents.ts:22-72`).
2. **Custom agents**: markdown/JSON files from `userSettings`, `projectSettings`, `policySettings`, `flagSettings`. Frontmatter schema in `loadAgentsDir.ts:73-99` allows `tools`, `disallowedTools`, `prompt`, `model` (with `inherit` keyword), `effort`, `permissionMode`, `mcpServers`, `hooks`, `maxTurns`, `skills`, `initialPrompt`, `memory` (`user`/`project`/`local`), `background`, `isolation` (`worktree` always; `remote` ant-only).
3. **Plugin agents**: loaded through `src/utils/plugins/loadPluginAgents.ts`, tagged with `source: 'plugin'` and a plugin name (`loadAgentsDir.ts:153-159`). Plugin sources are considered admin-trusted for MCP policy (`runAgent.ts:117-127`).

Other observed agent features:

- `hasRequiredMcpServers(agent, available)` gates agent availability by case-insensitive substring match on configured MCP server names (`loadAgentsDir.ts:229-241`).
- `omitClaudeMd` shortens the subagent context: Explore and Plan skip CLAUDE.md to save tokens across many spawns (`loadAgentsDir.ts:128-132`, `built-in/exploreAgent.ts:79-82`).
- `criticalSystemReminder_EXPERIMENTAL` is a short reminder re-injected on every user turn, wired through `ToolUseContext` (`src/Tool.ts:274-275`, `loadAgentsDir.ts:121`).

## 5. Coordinator / multi-agent mode

- `src/coordinator/coordinatorMode.ts:36-41` gates the mode behind both `feature('COORDINATOR_MODE')` and `CLAUDE_CODE_COORDINATOR_MODE` env var. `matchSessionMode()` flips the env var when resuming a session stored in the opposite mode (lines 49-78).
- A coordinator system prompt is rendered by `getCoordinatorSystemPrompt()` (`src/coordinator/coordinatorMode.ts:111-369`). Its explicit contract:
  - The coordinator's only action verbs are spawn (`AgentTool`), continue (`SendMessageTool`), stop (`TaskStopTool`), and optionally subscribe/unsubscribe to PR activity (ant-only).
  - Worker results arrive as user-role messages containing `<task-notification>` XML with `<task-id>`, `<status>`, `<summary>`, optional `<result>` and `<usage>`; the prompt instructs the model to treat them as system signals, not conversation turns.
  - Worker tools are restricted to `ASYNC_AGENT_ALLOWED_TOOLS` minus a set of "internal worker tools" (`TeamCreateTool`, `TeamDeleteTool`, `SendMessageTool`, `SyntheticOutputTool`) (`coordinatorMode.ts:29-34`).
- When `CLAUDE_CODE_SIMPLE` is also set, the coordinator still injects `AgentTool`, `TaskStopTool`, `SendMessageTool` so the main thread can orchestrate even in bare mode (`src/tools.ts:287-297`).

## 6. Permission model

### 6.1 Modes

- `src/utils/permissions/PermissionMode.ts:42-91` defines six modes: `default`, `plan`, `acceptEdits`, `bypassPermissions`, `dontAsk`, and ant-only `auto`. External users never see `auto`/`bubble` — `isExternalPermissionMode` filters them out (lines 97-105).
- `prePlanMode` is stored on `ToolPermissionContext` so exiting plan mode restores the previous mode (`src/Tool.ts:123-138`).

### 6.2 Rule sources and layering

- Permission rule sources are `SETTING_SOURCES` plus `cliArg`, `command`, `session` (`src/utils/permissions/permissions.ts:109-114`).
- `SETTING_SOURCES` layering is ordered so later sources override earlier: `userSettings < projectSettings < localSettings < flagSettings < policySettings` (`src/utils/settings/constants.ts:7-22`).
- `ToolPermissionContext` carries three rule maps (`alwaysAllowRules`, `alwaysDenyRules`, `alwaysAskRules`), `additionalWorkingDirectories`, `isBypassPermissionsModeAvailable`, `isAutoModeAvailable`, `strippedDangerousRules`, `shouldAvoidPermissionPrompts` (for background agents that can't show UI), and `awaitAutomatedChecksBeforeDialog` (for coordinator workers) (`src/Tool.ts:123-138`).
- `filterToolsByDenyRules` is applied before the model sees tools, stripping MCP server-prefix denies like `mcp__server` and reusing the same matcher as runtime permission checks (`src/tools.ts:262-269`).

### 6.3 Permission handlers

- Separate handler files under `src/hooks/toolPermission/handlers/`: `interactiveHandler.ts` (536 lines), `coordinatorHandler.ts` (65 lines), and `swarmWorkerHandler.ts`.
- The coordinator flow is explicit: try hooks → try BASH classifier (gated by `feature('BASH_CLASSIFIER')`) → otherwise fall through to the interactive dialog (`src/hooks/toolPermission/handlers/coordinatorHandler.ts:26-62`).
- `PermissionContext` exposes `logDecision`, `logCancelled`, `persistPermissions`, `runHooks`, `tryClassifier`, and a resolve-once primitive that atomically claims a resolution to handle concurrent resolution races (`src/hooks/toolPermission/PermissionContext.ts:63-150`).
- Observed classifier plumbing: `classifierDecision.ts`, `autoModeState.ts`, `bashClassifier.ts`, `yoloClassifier.ts`, `classifierShared.ts` under `src/utils/permissions/` (`src/utils/permissions/`). Classifier fail-closed refresh is 30 minutes (`src/utils/permissions/permissions.ts:107`).

## 7. Sandbox (Bash)

- Bash sandbox is adapter-wrapped around `@anthropic-ai/sandbox-runtime` in `src/utils/sandbox/sandbox-adapter.ts:7-46`.
- `shouldUseSandbox` (`src/tools/BashTool/shouldUseSandbox.ts:130-153`) short-circuits when sandboxing is disabled, when the command matches an `excludedCommands` pattern, or when `dangerouslyDisableSandbox` is set and unsandboxed commands are allowed by policy.
- The `excludedCommands` feature is explicitly documented as a "user-facing convenience, not a security boundary" — the sandbox permission system is the control (`src/tools/BashTool/shouldUseSandbox.ts:18-20`).
- Bash-specific validation and permission logic is split across many files under `src/tools/BashTool/`: `bashPermissions.ts`, `bashSecurity.ts`, `commandSemantics.ts`, `destructiveCommandWarning.ts`, `modeValidation.ts`, `pathValidation.ts`, `readOnlyValidation.ts`, `sedEditParser.ts`, `sedValidation.ts`.
- Path pattern resolution encodes three Claude-Code-specific conventions: `//path` → absolute from root; `/path` → relative to settings-file directory; `~/path` and relative paths pass through to `sandbox-runtime` (`src/utils/sandbox/sandbox-adapter.ts:99-119`).

## 8. MCP integration

- MCP transport types observed in schemas: `stdio`, `sse`, `sse-ide`, `http`, `ws`, `ws-ide`, `sdk` (`src/services/mcp/types.ts:23-107`).
- Scopes: `local`, `user`, `project`, `dynamic`, `enterprise`, `claudeai`, `managed` (`src/services/mcp/types.ts:10-21`).
- OAuth support includes per-server XAA (Cross-App Access / SEP-990) flag and shared IdP config in settings (`src/services/mcp/types.ts:37-56`). Files `src/services/mcp/xaa.ts`, `xaaIdpLogin.ts`, `oauthPort.ts`, `officialRegistry.ts` indicate a full MCP OAuth surface.
- In-process MCP transports exist for IDE integration (`InProcessTransport.ts`, `vscodeSdkMcp.ts`, `SdkControlTransport.ts`).
- Agent-scoped MCP servers: each agent can declare `mcpServers` (named or inline); referenced servers reuse the memoized parent connection, inline definitions are cleaned up when the agent terminates (`src/tools/AgentTool/runAgent.ts:95-205`).
- Plugin-only policy (`isRestrictedToPluginOnly('mcp')`) blocks frontmatter MCP servers from user-controlled agents while allowing plugin/built-in/policySettings agents (admin-trusted) (`src/tools/AgentTool/runAgent.ts:112-127`).
- MCP tools carry `mcpInfo: { serverName, toolName }` to preserve unnormalized identity for deny-rule matching and prefix/no-prefix modes (`src/Tool.ts:452-455`).
- MCP elicitation is surfaced as a first-class tool error code (`-32042`) handled by `ToolUseContext.handleElicitation` (`src/Tool.ts:193-202`) and routed differently in print/SDK vs REPL queue mode.

## 9. Hooks

- Hook event names (observed from `src/types/hooks.ts:60-139`): `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `UserPromptSubmit`, `SessionStart`, `Setup`, `SubagentStart`, `PermissionDenied`, `Notification`, `PermissionRequest`, `Elicitation`, plus additional events referenced by `HOOK_EVENTS` exported from `src/entrypoints/agentSdkTypes.js`.
- Sync hook response shape includes `continue`, `suppressOutput`, `stopReason`, `decision` (`approve`/`block`), `reason`, `systemMessage`, and a `hookSpecificOutput` union keyed by `hookEventName` (`src/types/hooks.ts:48-140`).
- `PreToolUse`-specific output can return a `permissionDecision`, `permissionDecisionReason`, `updatedInput`, and `additionalContext` — i.e., hooks can rewrite tool inputs and bypass/block the permission dialog (`src/types/hooks.ts:72-78`).
- Hook execution events (`started`/`progress`/`response`) are broadcast through a generic event bus separate from the main stream (`src/utils/hooks/hookEvents.ts:51-110`). `SessionStart` and `Setup` are always-emitted low-noise events; all others require `setAllHookEventsEnabled(true)` (lines 15-90).
- Hook registrations come from four places: settings (`hooksSettings.ts`), agent/skill frontmatter (`registerFrontmatterHooks.ts`, `registerSkillHooks.ts`), plugin manifests (`utils/plugins/loadPluginHooks.ts`), and session-scoped dynamic registration (`sessionHooks.ts`).
- Transport mechanisms for executing hook bodies: external process (`execPromptHook.ts`), HTTP (`execHttpHook.ts`), agent (`execAgentHook.ts`); there is also an SSRF guard (`ssrfGuard.ts`).

## 10. Skills

- Skills are markdown files with frontmatter, loaded from several source kinds: `commands_DEPRECATED`, `skills`, `plugin`, `managed`, `bundled`, `mcp` (`src/skills/loadSkillsDir.ts:67-73`).
- Bundled skills live in `src/skills/bundled/` and include `batch.ts`, `claudeApi.ts`, `claudeApiContent.ts`, `claudeInChrome.ts`, `debug.ts`, `keybindings.ts`, `loop.ts`, `loremIpsum.ts`, `remember.ts`, `scheduleRemoteAgents.ts`, `simplify.ts`, `skillify.ts`, `stuck.ts`, `updateConfig.ts`, `verify.ts`, `verifyContent.ts` — each is registered through `bundledSkills.ts`.
- `SkillTool` is a first-class built-in tool (`src/tools/SkillTool`, registered in `src/tools.ts:212`), separate from `ToolSearchTool`.
- Skill discovery is prefetched per-iteration inside the query loop to hide the latency under model streaming and tool execution (`src/query.ts:323-335`).

## 11. Plugins and marketplaces

- Official marketplace allowlist includes `claude-code-marketplace`, `claude-code-plugins`, `claude-plugins-official`, `anthropic-marketplace`, `anthropic-plugins`, `agent-skills`, `life-sciences`, `knowledge-work-plugins` (`src/utils/plugins/schemas.ts:19-28`).
- Name-impersonation is blocked by a regex pattern and a non-ASCII homograph pattern (`src/utils/plugins/schemas.ts:71-79`).
- Plugins can contribute agents, commands, hooks, output styles, MCP servers, and LSP integration, each loaded through a dedicated file (`src/utils/plugins/loadPluginAgents.ts`, `loadPluginCommands.ts`, `loadPluginHooks.ts`, `loadPluginOutputStyles.ts`, `lspPluginIntegration.ts`, `mcpPluginIntegration.ts`).
- A `strictPluginOnlyCustomization` policy is consulted when registering MCP from agent frontmatter (`src/tools/AgentTool/runAgent.ts:118-127`); the policy distinguishes admin-trusted sources.
- Installation lifecycle is handled by `src/services/plugins/PluginInstallationManager.ts` with per-marketplace status (`pending`/`installing`/`installed`/`failed`) surfaced to `AppState.plugins.installationStatus` (`src/services/plugins/PluginInstallationManager.ts:30-48`).

## 12. Tasks (backgroundable units of work)

- Task type union (`src/Task.ts:6-13`): `local_bash`, `local_agent`, `remote_agent`, `in_process_teammate`, `local_workflow`, `monitor_mcp`, `dream`.
- Task status union: `pending`, `running`, `completed`, `failed`, `killed` (`src/Task.ts:15-21`).
- Task IDs use a type-prefix then 8 random case-insensitive chars (`b…/a…/r…/t…/w…/m…/d…`), documented as ≈2.8 trillion combinations chosen to resist symlink bruteforcing (`src/Task.ts:78-106`).
- `TaskStateBase` persists `outputFile` and `outputOffset` — task stdout/stderr is written to disk (`getTaskOutputPath(id)`), with offsets tracked for incremental tailing (`src/Task.ts:45-57`, `108-125`).
- Only the `kill` method is dispatched polymorphically across task types; comment notes that `spawn`/`render` were never called polymorphically and `getAppState`/`abortController` were removed as dead weight (`src/Task.ts:69-76`).
- Tasks know whether they are "backgrounded": `isBackgroundTask` excludes non-running tasks and tasks with `isBackgrounded === false` (`src/tasks/types.ts:37-46`).
- Task kinds each live in their own directory with typed state: `LocalShellTask/`, `LocalAgentTask/`, `RemoteAgentTask/`, `InProcessTeammateTask/`, `LocalWorkflowTask/`, `MonitorMcpTask/`, `DreamTask/`.

## 13. Memory (memdir)

- Memory is constrained to four types: `user`, `feedback`, `project`, `reference` (`src/memdir/memoryTypes.ts:14-21`). Each has a declared `<scope>` of `private`, `team`, or a documented selection rule (`memoryTypes.ts:37-60`).
- Memory scope for agent frontmatter is `user | project | local` (`src/tools/AgentTool/loadAgentsDir.ts:92`).
- Dedicated modules: `findRelevantMemories.ts`, `memdir.ts`, `memoryAge.ts`, `memoryScan.ts`, `paths.ts`, `teamMemPaths.ts`, `teamMemPrompts.ts`.

## 14. Context lifecycle

- Layered context management is visible as discrete services under `src/services/compact/`: `apiMicrocompact.ts`, `autoCompact.ts`, `compact.ts`, `compactWarningHook.ts`, `grouping.ts`, `microCompact.ts`, `postCompactCleanup.ts`, `sessionMemoryCompact.ts`, `snipCompact.ts` (referenced from `query.ts` via `HISTORY_SNIP`), `timeBasedMCConfig.ts`.
- The **tool-result budget** is separate: `applyToolResultBudget` persists oversize results to disk with a preview, tracked in `ContentReplacementState` (`src/Tool.ts:58-62`, `286-292`). `maxResultSizeChars = Infinity` opts a tool out (Read, to avoid Read→file→Read loops, per `Tool.ts:461-466`).
- Task-budget accounting (server-side output_config `task_budget`) is carried as `{total, remaining}` with explicit handling of how `remaining` must be resent after compaction because the server only sees the summary (`src/query.ts:181-199`, `282-291`).

## 15. Isolation surfaces

- Worktree mode is a first-class toggle (`isWorktreeModeEnabled()`) with paired tools `EnterWorktreeTool` and `ExitWorktreeTool` (`src/tools.ts:142`, `225`).
- Remote execution is ant-only. Frontmatter `isolation: 'remote'` is only accepted when `USER_TYPE === 'ant'` (`src/tools/AgentTool/loadAgentsDir.ts:94-97`).
- `src/remote/` provides `RemoteSessionManager.ts`, `SessionsWebSocket.ts`, `remotePermissionBridge.ts`, and `sdkMessageAdapter.ts` (`src/remote/`). A `RemotePermissionResponse` simplified shape separates CCR transport from the internal `PermissionResult` (`src/remote/RemoteSessionManager.ts:40-62`).
- CLI has an `environment-runner` (BYOC) and `self-hosted-runner` mode, each with its own main (`src/entrypoints/cli.tsx:225-245`).

## 16. Analytics, feature gating, policy

- Two gating layers: build-time `feature('FLAG')` from `bun:bundle` (dead-code-eliminated for external builds, see `src/tools.ts:104-135`) and runtime `getFeatureValue_CACHED_MAY_BE_STALE` / `checkStatsigFeatureGate_CACHED_MAY_BE_STALE` (GrowthBook-style) in `src/services/analytics/growthbook.ts`.
- `USER_TYPE === 'ant'` is a hard distinction visible throughout: some tools are ant-only (`REPLTool`, `SuggestBackgroundPRTool`, `ConfigTool`, `TungstenTool`) (`src/tools.ts:16-22`, `214-216`), some modes (`auto` permission, `remote` isolation) are ant-only.
- Analytics infrastructure: `src/services/analytics/` includes `datadog.ts`, `firstPartyEventLogger.ts`, `firstPartyEventLoggingExporter.ts`, `growthbook.ts`, `sink.ts`, `sinkKillswitch.ts`.
- Policy limits are fetched before allowing remote control: `waitForPolicyLimitsToLoad` and `isPolicyAllowed('allow_remote_control')` gate the bridge fast path (`src/entrypoints/cli.tsx:152-159`).

## 17. Observations on module boundaries

- **One process, many surfaces**: REPL UI, SDK, daemon worker, bridge/remote-control, template jobs, environment-runner, self-hosted-runner, and Chrome/Computer-Use MCP servers all live in the same binary, dispatched before full module load (`src/entrypoints/cli.tsx`).
- **Narrow contract, wide implementations**: the `Tool` interface is rich but uniform (`src/Tool.ts`); tools bring their own permissioning, rendering, validation, classification inputs, and defer/alwaysLoad metadata. Most module complexity lives inside each tool's folder (e.g., BashTool has ~18 files).
- **Explicit staging of side effects**: the query loop enumerates the order of snip → microcompact → collapse → autocompact → tool execution (`src/query.ts:365-426`); the permission handler enumerates hooks → classifier → dialog (`src/hooks/toolPermission/handlers/coordinatorHandler.ts:26-62`); tool orchestration enumerates read-only-batch → serial-batch (`toolOrchestration.ts:84-116`).
- **Feature-flagged invariants**: many invariants change based on `feature(...)` (e.g., `CONTEXT_COLLAPSE`, `REACTIVE_COMPACT`, `CACHED_MICROCOMPACT`, `TOKEN_BUDGET`, `HISTORY_SNIP`, `COORDINATOR_MODE`, `BG_SESSIONS`, `BRIDGE_MODE`, `DAEMON`). Unverified: whether any of these are enabled in the shipped external build — the sourcemap exposes the branches but not the active set.
- **Policy separation**: settings layering has a fixed precedence (`userSettings < projectSettings < localSettings < flagSettings < policySettings`) independent from the permission-rule sources (`cliArg`, `command`, `session` are added on top) (`src/utils/settings/constants.ts:7-22`, `src/utils/permissions/permissions.ts:109-114`).
- **Cache-conscious assembly**: multiple comments call out that prompt-cache stability drives decisions — built-ins must sort before MCP tools (`src/tools.ts:354-365`), forked agents inherit the parent's frozen system prompt (`Tool.ts:294-299`), and MCP tool inclusion in the base list must track a server-side global-system-caching config (`src/tools.ts:191`).

## 18. Notable tradeoffs observed (not prescriptions)

- **Tool surface is deliberately broad, not presetted**. `TOOL_PRESETS = ['default']` is the only preset; all tailoring happens through rules, deny-rules, and feature flags (`src/tools.ts:161-183`, `262-269`).
- **Coordinator is a prompt-level pattern, not a separate process tree**. Coordinator workers reuse the same `Tool`/`query` machinery; the difference is which tools are visible and which permission handler runs (`src/coordinator/coordinatorMode.ts:36-41`, `src/hooks/toolPermission/handlers/coordinatorHandler.ts`).
- **Persistence is deliberately plural**: settings files by layer, permission rules by source, tool output files by task id, sidechain transcripts, append-only hook events, and separate session/event stores. Each has its own module and path resolver.
- **UI is coupled to the tool type**. The `Tool` interface returns `React.ReactNode` directly (`src/Tool.ts:580-653`); there is no separate "tool descriptor → UI adapter" boundary. Any consumer that is not Ink needs to re-implement rendering.
- **Extensibility has multiple independent channels**: built-in agents, custom agents (markdown frontmatter), plugin agents, plugin commands, plugin hooks, plugin output styles, plugin MCP servers, skills (bundled/user/project/managed/plugin/mcp), MCP servers (scoped), custom hooks (settings/frontmatter/plugin/skill/session). Each channel has independent loaders and trust/admin-trust classification.

## 19. Unverified / partially verified claims

- **Active feature flag set in the shipped external binary** — the sourcemap exposes both the gated code paths and the flag strings (`feature('COORDINATOR_MODE')`, etc.), but does not reveal which flags are enabled at ship time. Unverified.
- **Runtime presence of coordinator mode for external users** — we observed the env-var toggle (`CLAUDE_CODE_COORDINATOR_MODE`) and the prompt, but we have not confirmed that `feature('COORDINATOR_MODE')` is live in external builds (`src/coordinator/coordinatorMode.ts:36-41`). Unverified.
- **Exact `HOOK_EVENTS` membership** — the symbol is imported from two different locations (`src/entrypoints/agentSdkTypes.js` and `src/entrypoints/sdk/coreTypes.js`), but neither source file was inspected directly; the enumeration of event names above is the subset observed through the Zod union in `src/types/hooks.ts:60-139`. Unverified.
- **Sandbox enforcement semantics on each OS** — the adapter delegates to an external runtime (`@anthropic-ai/sandbox-runtime`) not included in the sourcemap. Unverified.
- **Exact interaction between `auto` permission mode and the classifier** — `auto` is ant-only and gated behind `feature('TRANSCRIPT_CLASSIFIER')` (`src/utils/permissions/PermissionMode.ts:80-90`); the full auto-mode state machine lives in `autoModeState.ts` which was not fully read. Unverified.
- **Root directory layout of the original internal repository** — per the upstream `README.md:6-11` the reconstructed directory tree is explicitly not representative. Observations above are therefore scoped to *runtime module names and boundaries*, not to folder organization choices.

## 20. Source-of-evidence index

| Claim cluster | Key files |
|---|---|
| Tool contract | `docs/references/claude-code-sourcemap-main/restored-src/src/Tool.ts` |
| Tool registry / assembly | `docs/references/claude-code-sourcemap-main/restored-src/src/tools.ts`; `src/services/tools/toolOrchestration.ts` |
| Query loop | `docs/references/claude-code-sourcemap-main/restored-src/src/query.ts`; `src/query/config.ts`; `src/query/stopHooks.ts`; `src/query/tokenBudget.ts`; `src/QueryEngine.ts` |
| Coordinator mode | `docs/references/claude-code-sourcemap-main/restored-src/src/coordinator/coordinatorMode.ts`; `src/hooks/toolPermission/handlers/coordinatorHandler.ts` |
| Agents | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/AgentTool/loadAgentsDir.ts`; `src/tools/AgentTool/builtInAgents.ts`; `src/tools/AgentTool/built-in/*.ts`; `src/tools/AgentTool/runAgent.ts` |
| Permission model | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/permissions/permissions.ts`; `src/utils/permissions/PermissionMode.ts`; `src/hooks/toolPermission/PermissionContext.ts`; `src/utils/permissions/*.ts` |
| Sandbox | `docs/references/claude-code-sourcemap-main/restored-src/src/tools/BashTool/shouldUseSandbox.ts`; `src/utils/sandbox/sandbox-adapter.ts`; `src/tools/BashTool/*.ts` |
| MCP | `docs/references/claude-code-sourcemap-main/restored-src/src/services/mcp/types.ts`; `src/services/mcp/config.ts`; `src/services/mcp/MCPConnectionManager.tsx`; `src/tools/AgentTool/runAgent.ts` |
| Hooks | `docs/references/claude-code-sourcemap-main/restored-src/src/types/hooks.ts`; `src/utils/hooks/hookEvents.ts`; `src/utils/hooks/*.ts` |
| Skills | `docs/references/claude-code-sourcemap-main/restored-src/src/skills/loadSkillsDir.ts`; `src/skills/bundled/*.ts`; `src/tools/SkillTool/*` |
| Plugins | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/plugins/schemas.ts`; `src/services/plugins/PluginInstallationManager.ts`; `src/utils/plugins/*.ts` |
| Tasks | `docs/references/claude-code-sourcemap-main/restored-src/src/Task.ts`; `src/tasks/types.ts`; `src/tasks/*/*.tsx` |
| Memory | `docs/references/claude-code-sourcemap-main/restored-src/src/memdir/memoryTypes.ts`; `src/memdir/*.ts` |
| Context lifecycle | `docs/references/claude-code-sourcemap-main/restored-src/src/services/compact/*.ts`; `src/utils/toolResultStorage.ts` |
| Settings layering | `docs/references/claude-code-sourcemap-main/restored-src/src/utils/settings/constants.ts` |
| Isolation / remote | `docs/references/claude-code-sourcemap-main/restored-src/src/remote/*.ts`; `src/entrypoints/cli.tsx` |
| Entrypoints | `docs/references/claude-code-sourcemap-main/restored-src/src/entrypoints/cli.tsx`; `src/entrypoints/sdk/*.ts`; `src/entrypoints/agentSdkTypes.ts`; `src/main.tsx` |
