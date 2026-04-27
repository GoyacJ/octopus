# M4 · L2 Extensions · tool-search / skill / mcp

> 状态：进行中 · 依赖：M3 完成（与 M5 可并行）
> 关键交付：3 个 L2 扩展 crate 完整可用 + Mock + Contract test
> 预计任务卡：19 张 · 累计工时：AI 22 小时（3 路并行约 8 小时墙钟）+ 人类评审 8 小时
> 并行度：**3 路并行**（tool-search / skill / mcp 互相正交）

---

## 0. 里程碑级注意事项

1. **3 路并行**：三 crate 互不依赖（tool-search 仅依赖 tool 公开 trait + model capabilities）
2. **MCP 是最复杂的 crate**：5 transport + Server Adapter + OAuth + Elicitation；任务卡 8 张，比其他多
3. **Skill 第三方分发走 plugin（M5）**：本里程碑只做 workspace / user / mcp source，plugin source 留 M5 做
4. **DeferPolicy 默认在 SessionOptions**：tool-search 完成后必须更新 session crate 的 SessionOptions（小补丁，单卡处理）

---

## 1. 任务卡总览

| Crate | 任务卡 | 内容 |
|---|---|---|
| **tool-search** | M4-T01 ~ T05.5 | DeferPolicy + ToolSearchTool + Anthropic / Inline backend + Scorer + Session 集成补丁 |
| **skill** | M4-T06 ~ T10 | SkillLoader + 3 Source + frontmatter + SkillTool 三件套 + ThreatScanner |
| **mcp** | M4-T11 ~ T18 | Client transport 5 种 + ServerAdapter + OAuth + Elicitation + 治理矩阵 |

---

## 2. 路 L2-TS · `octopus-harness-tool-search`

### M4-T01 · DeferPolicy + ToolSearchMode + 类型骨架

| 字段 | 值 |
|---|---|
| **状态** | 已完成 |

**SPEC 锚点**：
- `harness-tool-search.md` §2
- ADR-009

**预期产物**：
- `src/lib.rs`
- `src/policy.rs`：DeferPolicy / ToolSearchMode（Always / Auto / Disabled）
- `src/scorer.rs`：ToolSearchScorer trait
- `src/backend.rs`：ToolLoadingBackend trait

**预期 diff**：< 300 行

---

### M4-T02 · ToolSearchTool 实现

| 字段 | 值 |
|---|---|
| **状态** | 已完成 |

**SPEC 锚点**：`harness-tool-search.md` §3

**预期产物**：
- `src/search_tool.rs`：实现 `Tool` trait（注入 Anthropic + Inline 双 backend 选择器）
- `tests/search_tool.rs`

**关键不变量**：
- 仅依赖 `harness-tool` 的公开 trait（不依赖 ToolRegistry 私有实现，D2 §10 例外登记）
- 不直接读取 `harness-model` 的 provider 实现，只读 `ModelCapabilities`

**预期 diff**：< 350 行

---

### M4-T03 · AnthropicReferenceBackend

| 字段 | 值 |
|---|---|
| **状态** | 已完成 |

**SPEC 锚点**：`harness-tool-search.md` §4.1

**预期产物**：
- `src/backends/anthropic.rs`：基于 Anthropic 的 tool reference（仅当 `model.supports_tool_reference = true` 时启用）
- `tests/backend.rs`

**Cargo feature**：`backend-anthropic`

**预期 diff**：< 300 行

---

### M4-T04 · InlineReinjectionBackend（含 50ms / max 32 合并窗口）

| 字段 | 值 |
|---|---|
| **状态** | 已完成 |

**SPEC 锚点**：`harness-tool-search.md` §4.2

**预期产物**：
- `src/backends/inline.rs`：50ms 合并窗口 + 最多 32 工具 / 次注入
- `tests/backend.rs` / `tests/coalescer.rs`

**Cargo feature**：`backend-inline`

**预期 diff**：< 350 行

---

### M4-T05 · DefaultScorer + Contract Test

| 字段 | 值 |
|---|---|
| **状态** | 已完成 |

**预期产物**：
- `src/scorer.rs`：DefaultScorer（基于关键词匹配 + tool 元数据）
- `tests/contract.rs`

**Cargo feature**：`scorer-default`

**预期 diff**：< 200 行

> Session 集成（`tool_search: ToolSearchMode` + `discovered_tools: DiscoveredToolProjection`）由独立 chore 卡 **M4-T05.5** 处理（实施前评估 P2-2 修订），避免与 M3 后期 / M5 并行 PR 共改 `harness-session/src/{options,projection}.rs`。

---

### M4-T05.5 · Session 集成补丁（chore，跨 crate 文件锁卡）

| 字段 | 值 |
|---|---|
| **状态** | 已完成 |
| **依赖** | M4-T05 + M5 完成 |
| **预期 diff** | < 100 行 |
| **文件锁声明** | 独占修改 `crates/octopus-harness-session/src/session.rs` + `src/projection.rs`；冲突卡：M4-T08 / M4-T18 / 任何修改 session crate 的 PR |

**预期产物**：
- `octopus-harness-session/src/session.rs`：增加 `pub tool_search: ToolSearchMode` 字段
- `octopus-harness-session/src/projection.rs`：增加 `pub discovered_tools: DiscoveredToolProjection`
- `tests/lifecycle.rs` / `tests/reload.rs`：覆盖创建时 tool_search 配置与 reload 拒绝

**关键不变量**：
- 本卡是 single-writer 卡，必须等 M4-T05 / M5 全部合并后才能派发，避免 session crate 冲突

---

## 3. 路 L2-SK · `octopus-harness-skill`

### M4-T06 · Skill trait + SkillLoader + frontmatter 解析

**SPEC 锚点**：
- `harness-skill.md` §2-§3
- `extensibility.md` §4

**预期产物**：
- `src/lib.rs`
- `src/skill.rs`：Skill struct（id / name / source / agent_allowlist / frontmatter）
- `src/loader.rs`：SkillLoader trait + SkillLoaderBuilder
- `src/frontmatter.rs`：YAML frontmatter 解析 + 字段校验

**预期 diff**：< 350 行

---

### M4-T07 · 3 个 SkillSource（Workspace / User / MCP）

**SPEC 锚点**：`harness-skill.md` §4

**预期产物**：
- `src/sources/workspace.rs`：WorkspaceSource（默认 `data/skills/`）
- `src/sources/user.rs`：UserSource（默认 `~/.octopus/skills/`）
- `src/sources/mcp.rs`：McpSource（通过 MCP server 提供 skill）
- `tests/sources.rs`

**关键不变量**：
- 优先级：Workspace > User > MCP > Plugin（plugin 留 M5）
- per-agent allowlist（subagent 默认禁用 user-source）

**Cargo feature**：`workspace-source / user-source / mcp-source`

**预期 diff**：< 400 行

---

### M4-T08 · SkillTool 三件套（list / view / invoke）

**SPEC 锚点**：`harness-skill.md` §5

**预期产物**：
- `src/skill_tools/list.rs`：SkillsListTool
- `src/skill_tools/view.rs`：SkillsViewTool
- `src/skill_tools/invoke.rs`：SkillsInvokeTool
- `tests/skill_tools.rs`

**关键不变量**：
- SkillsInvokeTool 注入位置必须是 user message（不污染 system prompt）
- Eager / SkillsInvokeTool / Hook 三种路径同终点

**预期 diff**：< 350 行

---

### M4-T09 · ContentThreatScanner（Skill 内容扫描）

**SPEC 锚点**：`harness-skill.md` §6（内容威胁扫描）

**预期产物**：
- `src/scanner.rs`：复用 `harness-memory/src/scanner.rs` 的 ThreatScanner trait + Skill 专属规则集
- `tests/scanner.rs`

**预期 diff**：< 200 行

---

### M4-T10 · Skill Contract Test + Prefetch 策略

**SPEC 锚点**：`harness-skill.md` §7（SkillPrefetchStrategy 四档）

**预期产物**：
- `src/prefetch.rs`：SkillPrefetchStrategy（None / Lazy / Eager / Hybrid）
- `tests/contract.rs`
- `tests/prefetch.rs`

**预期 diff**：< 200 行

---

## 4. 路 L2-MCP · `octopus-harness-mcp`

### M4-T11 · MCP Client 核心抽象

**SPEC 锚点**：
- `harness-mcp.md` §2-§3
- ADR-005

**预期产物**：
- `src/lib.rs`
- `src/client.rs`：McpClient + McpConnection + McpServerSpec（三维：source × trust × scope）
- `src/transport.rs`：McpTransport trait
- `src/types.rs`：McpToolFilter / SamplingPolicy / StdioPolicy / StdioEnv / ReconnectPolicy / TenantIsolationPolicy / McpTimeouts

**关键不变量**：
- 三维正交：source × trust × scope（生命周期）
- canonical 工具命名 `mcp__<server>__<tool>`

**预期 diff**：< 450 行

---

### M4-T12 · stdio + http + websocket transport

**SPEC 锚点**：`harness-mcp.md` §4.1-§4.3

**预期产物**：
- `src/transports/stdio.rs`：StdioTransport（默认屏蔽常见凭证 env）
- `src/transports/http.rs`：HttpTransport
- `src/transports/websocket.rs`：WebsocketTransport
- `tests/stdio.rs / http.rs / websocket.rs`

**Cargo feature**：`stdio / http / websocket`

**预期 diff**：< 500 行

---

### M4-T13 · sse + in-process transport

**SPEC 锚点**：`harness-mcp.md` §4.4-§4.5

**预期产物**：
- `src/transports/sse.rs`
- `src/transports/in_process.rs`

**Cargo feature**：`sse / in-process`

**预期 diff**：< 350 行

---

### M4-T14 · ReconnectPolicy + 重连治理

**SPEC 锚点**：`harness-mcp.md` §5

**预期产物**：
- `src/reconnect.rs`：指数退避 + 自重置 + max_attempts
- `tests/reconnect.rs`

**关键不变量**：
- `max_attempts: 0` = 不限重试（v1.8.1 P3-4 待修订，但 plan 提前显式说明）

**预期 diff**：< 200 行

---

### M4-T15 · OAuth + Elicitation

**SPEC 锚点**：`harness-mcp.md` §6（OAuth）+ §7（Elicitation）

**预期产物**：
- `src/oauth.rs`：OAuth flow（device flow + PKCE）
- `src/elicitation.rs`：ElicitationHandler trait + 3 内置实现（CLI / Stream / Mock）

**Cargo feature**：`oauth`

**预期 diff**：< 400 行

---

### M4-T16 · ServerAdapter（出站）

**SPEC 锚点**：
- `harness-mcp.md` §8（Server Adapter）
- ADR-005（双向 MCP）

**预期产物**：
- `src/server.rs`：ServerAdapter（把 SDK 的 Tool / Resource / Skill 暴露为 MCP server）
- `tests/server.rs`

**Cargo feature**：`server-adapter`

**预期 diff**：< 400 行

---

### M4-T17 · Sampling Policy + 七维 budget + cache 隔离

**SPEC 锚点**：`harness-mcp.md` §9

**预期产物**：
- `src/sampling.rs`：SamplingPolicy（七维 budget + cache 硬隔离 + PermissionMode 联动）

**关键不变量**：
- Sampling cache 与主 model cache 完全隔离
- PermissionMode 提升时 sampling 必须重新审批

**预期 diff**：< 250 行

---

### M4-T18 · MCP Contract Test + 多租户隔离测试 + Tools/list_changed 运行期策略

**SPEC 锚点**：`harness-mcp.md` §10 / §11

**预期产物**：
- `tests/contract.rs`：覆盖 5 transport
- `tests/tenant_isolation.rs`：StrictTenant 隔离
- `tests/list_changed.rs`：动态工具列表变更（与 ADR-003 / DeferPolicy::AutoDefer 联动）

**预期 diff**：< 300 行

---

## 5. M4 Gate 检查

- ✅ 3 crate 各自 `cargo test --all-features` 全绿
- ✅ ToolSearchTool E2E：模拟 100 工具 → 用 ToolSearchTool 选 5 工具 → 注入 → 执行
- ✅ Skill E2E：通过 SkillsListTool 列出 3 skills → SkillsInvokeTool 调用其中一个
- ✅ MCP E2E：连接 stdio MCP server（mock）→ 调用其工具 → 成功响应
- ✅ feature 矩阵 CI（含 stdio+http+ws / oauth / server-adapter）全绿
- ✅ MCP 多租户隔离测试通过

未全绿 → 不得进入 M7（M5 / M6 可并行进行）。

---

## 6. 索引

- **上一里程碑** → [`M3-l2-core.md`](./M3-l2-core.md)
- **下一里程碑（可并行）** → [`M5-l3-engine.md`](./M5-l3-engine.md)
