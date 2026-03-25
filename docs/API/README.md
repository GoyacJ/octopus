# Novai Hub API 文档

**版本**: v1 | **对应产品版本**: PRD / ARCH v0.1.0 | **日期**: 2026-03-10

---

## 目录

- [文件索引](#文件索引)
- [API 概览](#api-概览)
- [认证机制](#认证机制)
- [请求 / 响应约定](#请求--响应约定)
- [统一错误格式](#统一错误格式)
- [错误码速查表](#错误码速查表)
- [分页约定](#分页约定)
- [实时事件（SSE）](#实时事件sse)
- [本地 Hub 模式说明](#本地-hub-模式说明)

---

## 文件索引

| 文件 | 覆盖资源 | 关键端点 |
|------|---------|---------|
| [auth.md](./auth.md) | 认证 | 登录、刷新 Token、登出、握手 |
| [agents.md](./agents.md) | Agent + 记忆 | Agent CRUD、记忆查询与管理 |
| [teams.md](./teams.md) | Team + 成员 + 路由规则 | Team CRUD、成员管理、路由规则 |
| [tasks.md](./tasks.md) | Task 生命周期 | 任务下发、审批决策、执行日志 |
| [discussions.md](./discussions.md) | 讨论会话 | 创建/控制讨论、用户插话、结论生成 |
| [skills.md](./skills.md) | Skill（能力模块） | Skill CRUD |
| [tools.md](./tools.md) | 内置工具 | 工具列表、工具搜索 |
| [mcp.md](./mcp.md) | MCP 服务 | MCP Server 注册、测试、管理 |
| [events.md](./events.md) | SSE 实时事件 | 事件流连接、全事件类型目录 |

---

## API 概览

### Base URL

```
https://{hub-host}/api/v1
```

- **远程 Hub（Docker 部署）**：`https://your-hub.example.com/api/v1`
- **本地 Hub（Tauri 内嵌）**：不走 HTTP，通过 Tauri `invoke()` 命令直接调用，无需 Base URL 和认证头。详见[本地 Hub 模式说明](#本地-hub-模式说明)。

### 协议与格式

| 项目 | 约定 |
|-----|------|
| 协议 | HTTPS（TLS 1.2+） |
| 请求 Content-Type | `application/json` |
| 响应 Content-Type | `application/json`（SSE 除外） |
| 字符编码 | UTF-8 |
| 时间戳格式 | ISO 8601 UTC，如 `2026-03-10T08:00:00Z` |
| ID 类型 | UUID v4 字符串，如 `"550e8400-e29b-41d4-a716-446655440000"` |
| HTTP 方法语义 | POST 创建、GET 查询、PUT 全量更新、PATCH 部分更新、DELETE 删除 |

---

## 认证机制

> 仅适用于**远程 Hub** 模式。本地 Hub（桌面端内嵌）不需要 Token。

所有需要认证的端点均通过 **Bearer Token（JWT）** 验证。

### 请求头格式

```http
Authorization: Bearer {access_token}
```

### Token 生命周期

| Token 类型 | 有效期 | 刷新方式 |
|-----------|-------|---------|
| `access_token` | 1 小时 | 使用 `refresh_token` 调用 `/auth/refresh` |
| `refresh_token` | 30 天 | 重新登录 |

### Token 自动刷新

Client 在收到 `401 Unauthorized` 且 `error.code == "token_expired"` 时，应自动尝试刷新 Token，刷新成功后重试原请求。若刷新也失败（`401`），则跳转到登录界面。

### 存储安全

Token 必须存储在 OS 安全存储中：
- macOS：Keychain
- Windows：Credential Store
- Android：Keystore

**禁止**将 Token 存储在普通文件或 localStorage 中。

---

## 请求 / 响应约定

### 成功响应结构

**单体资源**（create / get / update）：

```json
{
  "data": { /* 资源对象 */ }
}
```

**列表资源**（list）：

```json
{
  "data": [ /* 资源数组 */ ],
  "total": 42,
  "page": 1,
  "page_size": 20
}
```

**无内容操作**（delete / terminate 等）：

```
HTTP 204 No Content（无响应体）
```

**流式响应**（SSE）：

```
Content-Type: text/event-stream
```

### HTTP 状态码语义

| 状态码 | 含义 |
|-------|------|
| `200 OK` | 查询 / 更新成功 |
| `201 Created` | 资源创建成功（Location 头包含新资源 URL） |
| `204 No Content` | 删除 / 操作成功，无响应体 |
| `400 Bad Request` | 请求参数校验失败 |
| `401 Unauthorized` | 未携带 Token 或 Token 失效 |
| `403 Forbidden` | Token 有效但权限不足 |
| `404 Not Found` | 资源不存在 |
| `409 Conflict` | 业务冲突（如删除 busy 状态的 Agent） |
| `422 Unprocessable Entity` | 领域不变量违反（如 participant 数量超出 [2,8]） |
| `429 Too Many Requests` | 请求频率超限 |
| `500 Internal Server Error` | Hub 内部错误 |

---

## 统一错误格式

所有错误响应（4xx / 5xx）使用统一结构：

```json
{
  "error": {
    "code": "agent_not_found",
    "message": "Agent with id '550e84...' does not exist in this tenant",
    "details": { }
  }
}
```

| 字段 | 类型 | 说明 |
|-----|------|------|
| `code` | string | 机器可读的错误码（见下表） |
| `message` | string | 人类可读的错误描述 |
| `details` | object? | 可选的额外上下文（如校验失败的字段列表） |

---

## 错误码速查表

### 通用错误码

| 错误码 | HTTP 状态 | 含义 |
|-------|---------|------|
| `unauthorized` | 401 | 未提供 Token |
| `token_expired` | 401 | Access Token 已过期，需刷新 |
| `token_invalid` | 401 | Token 签名无效或格式错误 |
| `forbidden` | 403 | 权限不足 |
| `not_found` | 404 | 资源不存在 |
| `validation_error` | 400 | 请求参数校验失败，`details` 含字段错误列表 |
| `conflict` | 409 | 业务状态冲突 |
| `domain_error` | 422 | 领域不变量违反 |
| `internal_error` | 500 | Hub 内部错误 |
| `rate_limited` | 429 | 请求过频 |

### 业务错误码

| 错误码 | 资源 | 含义 |
|-------|-----|------|
| `agent_not_found` | Agent | Agent 不存在 |
| `agent_busy` | Agent | Agent 正在执行任务，无法执行此操作 |
| `invalid_system_prompt` | Agent | system_prompt 不能为空（I-01） |
| `tool_not_registered` | Agent | tools_whitelist 中的工具未注册 |
| `memory_store_id_immutable` | Agent | 不允许修改 memory_store_id（I-02） |
| `team_not_found` | Team | Team 不存在 |
| `team_missing_leader` | Team | Team 没有 Leader（I-11） |
| `agent_not_member` | Team | 指定 Agent 不是该 Team 的成员 |
| `task_not_found` | Task | Task 不存在 |
| `task_already_terminated` | Task | Task 已终止，无法再操作 |
| `decision_not_pending` | Task | Decision 已处理，无法重复 resolve |
| `dag_cycle_detected` | Task | 任务计划中存在循环依赖（I-05） |
| `discussion_not_found` | Discussion | 讨论会话不存在 |
| `discussion_not_active` | Discussion | 讨论未处于 active 状态，无法操作 |
| `invalid_participant_count` | Discussion | 参与者数量不在 [2, 8] 范围内（I-08） |
| `missing_moderator` | Discussion | Moderated 策略下未指定 moderator（I-10） |
| `debate_missing_position` | Discussion | 辩论模式下部分参与者未设置立场（I-09） |
| `skill_not_found` | Skill | Skill 不存在 |
| `cannot_modify_builtin_skill` | Skill | 内置 Skill 不可修改或删除（I-14） |
| `mcp_server_not_found` | MCP | MCP Server 不存在 |
| `mcp_connection_failed` | MCP | 连接 MCP Server 失败 |

---

## 分页约定

### 列表端点分页参数

所有列表端点支持以下查询参数：

| 参数 | 类型 | 默认值 | 说明 |
|-----|------|-------|------|
| `page` | integer | `1` | 页码，从 1 开始 |
| `page_size` | integer | `20` | 每页条数，最大 `100` |

**示例**：

```http
GET /api/v1/agents?page=2&page_size=10
```

### 游标分页（日志 / 发言记录等高写入追加表）

`task_log_entries` 和 `discussion_turns` 使用游标分页，避免 OFFSET 在大数据量下性能退化：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `after` | string? | 游标：返回此 ID 之后的记录（不含该条） |
| `limit` | integer | 每次拉取条数，默认 `50`，最大 `200` |

**响应**：

```json
{
  "data": [ /* 记录数组，按时间升序 */ ],
  "next_cursor": "entry-id-xxx",
  "has_more": true
}
```

---

## 实时事件（SSE）

Hub 通过 Server-Sent Events（SSE）向 Client 推送实时事件。

**连接端点**：

```http
GET /api/v1/events
Authorization: Bearer {token}
```

连接建立后保持长连接，Hub 按需推送事件。详细事件类型见 [events.md](./events.md)。

---

## 本地 Hub 模式说明

桌面端（Tauri）内嵌本地 Hub 时，所有 API 调用通过 **Tauri `invoke()` 命令**直接执行，绕过 HTTP 层：

```typescript
// Client 侧 transport 抽象（transport.ts）
// 本地模式：直接 invoke
await invoke('create_agent', { payload: agentData });

// 远程模式：HTTP 请求
await fetch(`${hubBaseUrl}/api/v1/agents`, { ... });
```

**差异说明**：

| 项目 | 本地 Hub（Tauri invoke） | 远程 Hub（HTTP） |
|-----|----------------------|---------------|
| 认证 | 无需（同进程） | JWT Bearer Token |
| 实时事件 | Tauri Event System（emit/listen） | SSE |
| 网络开销 | 零（进程内调用） | 正常 HTTP 延迟 |
| 端口 | 无 | 默认 8080 |

> 本文档中的 HTTP API 均指**远程 Hub** 场景。本地 Hub 的 Tauri Command 名称与 HTTP 端点语义一一对应，见 ARCHITECTURE.md § 16.1。

---

*本文档对应 Novai PRD v0.1.0 / ARCHITECTURE v0.1.0。Phase 2 新增端点（多用户协作、TeamGroup、RBAC 管理）将在对应版本更新。*