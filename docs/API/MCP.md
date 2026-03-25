# MCP Servers API

> MCP（Model Context Protocol）是标准化的 AI 工具接入协议。通过 MCP Gateway，Agent 可以连接外部 MCP Server，使用第三方工具（如 Notion、GitHub、数据库等）。

---

## 目录

- [McpServer 对象结构](#mcpserver-对象结构)
- [注册 MCP Server](#1-注册-mcp-server)
- [获取 MCP Server 列表](#2-获取-mcp-server-列表)
- [获取单个 MCP Server](#3-获取单个-mcp-server)
- [更新 MCP Server](#4-更新-mcp-server)
- [测试连接](#5-测试连接)
- [删除 MCP Server](#6-删除-mcp-server)
- [MCP 工具命名规则](#mcp-工具命名规则)

---

## McpServer 对象结构

```json
{
  "id": "mcp_notion",
  "tenant_id": "tnt_01HX...",
  "name": "Notion",
  "description": "Notion 工作区集成，支持页面搜索与创建",
  "transport": "http",
  "endpoint_url": "https://mcp.notion.example.com",
  "auth_config": {
    "type": "bearer",
    "token_ref": "secret_notion_api_key"
  },
  "status": "online",
  "discovered_tools": [
    {
      "name": "mcp__mcp_notion__notion_search",
      "display_name": "搜索 Notion 页面",
      "description": "在 Notion 工作区中搜索页面和内容",
      "parameters": {
        "type": "object",
        "properties": {
          "query": { "type": "string" }
        },
        "required": ["query"]
      }
    },
    {
      "name": "mcp__mcp_notion__notion_create_page",
      "display_name": "创建 Notion 页面",
      "description": "在 Notion 工作区创建新页面",
      "parameters": { /* ... */ }
    }
  ],
  "last_health_check_at": "2026-03-10T08:00:00Z",
  "created_at": "2026-02-01T00:00:00Z",
  "updated_at": "2026-03-10T08:00:00Z"
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|-----|------|------|
| `transport` | enum | `http`（HTTP transport）/ `stdio`（标准 I/O，仅本地 Hub） |
| `endpoint_url` | string | MCP Server 地址（HTTP transport 必填） |
| `auth_config` | object? | 认证配置，见下方 |
| `status` | enum | `online` / `offline` / `error` / `unknown` |
| `discovered_tools` | object[] | 从 MCP Server 动态发现的工具列表（通过 `tools/list` RPC 获取） |

### auth_config 类型

```json
// Bearer Token 认证
{ "type": "bearer", "token_ref": "secret_key_name" }

// API Key 认证（通过 Header）
{ "type": "api_key", "header_name": "X-API-Key", "key_ref": "secret_key_name" }

// 无认证
{ "type": "none" }
```

> `token_ref` / `key_ref` 是密钥标识符，实际密钥值存储在 Hub 安全存储中，不通过 API 明文传输。

---

## 1. 注册 MCP Server

注册并连接一个新的 MCP Server。注册成功后，Hub 会自动发起 `tools/list` 调用发现可用工具。

```http
POST /api/v1/mcp/servers
Authorization: Bearer {token}
```

**所需角色**：`tenant_admin`

**请求体**：

```json
{
  "name": "Notion",
  "description": "Notion 工作区集成",
  "transport": "http",
  "endpoint_url": "https://mcp.notion.example.com",
  "auth_config": {
    "type": "bearer",
    "token": "secret_notion_token_plaintext"
  }
}
```

> **注意**：注册时 `auth_config.token` 传入**明文值**，Hub 接收后加密存储并转换为 `token_ref`。后续查询时不再返回明文，只返回 `token_ref`。

| 字段 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `name` | string | ✅ | Server 显示名称 |
| `description` | string | ❌ | 功能描述 |
| `transport` | enum | ✅ | `http` 或 `stdio` |
| `endpoint_url` | string | 条件必填 | `transport = http` 时必填 |
| `auth_config` | object | ❌ | 认证配置，注册时传明文值 |

**行为**：
1. 保存 MCP Server 配置，密钥加密存储
2. 异步发起 `initialize` + `tools/list` RPC，发现工具列表
3. `status` 初始为 `unknown`，连接成功后变为 `online`
4. 通过 SSE `mcp.server_status` 事件推送连接结果

**响应 `201 Created`**：McpServer 对象（`discovered_tools` 可能为空，待后台异步填充）。

**错误**：

| 错误码 | 说明 |
|-------|------|
| `mcp_connection_failed` | 注册时立即连接失败（endpoint_url 不可达） |
| `validation_error` | 请求字段格式错误 |

---

## 2. 获取 MCP Server 列表

```http
GET /api/v1/mcp/servers
Authorization: Bearer {token}
```

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `status` | string? | `online` / `offline` / `error` / `unknown` |

**响应 `200 OK`**：

```json
{
  "data": [ /* McpServer 数组 */ ],
  "total": 3
}
```

---

## 3. 获取单个 MCP Server

```http
GET /api/v1/mcp/servers/:server_id
Authorization: Bearer {token}
```

**响应 `200 OK`**：完整 McpServer 对象，含 `discovered_tools` 列表。

---

## 4. 更新 MCP Server

更新 MCP Server 的基本信息（不可更改 `transport` 类型）。

```http
PUT /api/v1/mcp/servers/:server_id
Authorization: Bearer {token}
```

**所需角色**：`tenant_admin`

**请求体**（字段均可选）：

```json
{
  "name": "Notion（更新）",
  "description": "Notion 工作区集成 v2",
  "endpoint_url": "https://new-mcp.notion.example.com",
  "auth_config": {
    "type": "bearer",
    "token": "new_token_plaintext"
  }
}
```

> 更新 `endpoint_url` 或 `auth_config` 后，Hub 会自动重新发起连接和工具发现。

**响应 `200 OK`**：更新后的 McpServer 对象。

---

## 5. 测试连接

手动触发对 MCP Server 的连通性测试，并刷新工具发现结果。

```http
POST /api/v1/mcp/servers/:server_id/test
Authorization: Bearer {token}
```

**所需角色**：`tenant_admin`

**响应 `200 OK`**：

```json
{
  "data": {
    "server_id": "mcp_notion",
    "status": "online",
    "latency_ms": 142,
    "discovered_tools_count": 8,
    "error": null,
    "tested_at": "2026-03-10T10:00:00Z"
  }
}
```

| 字段 | 说明 |
|-----|------|
| `status` | 测试后的最新状态 |
| `latency_ms` | 连接延迟（毫秒） |
| `error` | 失败时的错误信息 |

---

## 6. 删除 MCP Server

```http
DELETE /api/v1/mcp/servers/:server_id
Authorization: Bearer {token}
```

**所需角色**：`tenant_admin`

**行为**：
- 从 Hub 注销 MCP Server，断开连接
- 所有 Agent 的 `mcp_bindings` 中引用该 Server 的条目自动移除
- 加密存储的密钥同步清理

**响应 `204 No Content`**

**错误**：`mcp_server_not_found`（404）

---

## MCP 工具命名规则

MCP 工具名称格式（I-13）：

```
mcp__{server_id}__{tool_name}
```

**示例**：

| Server ID | MCP 原始工具名 | octopus 工具名（全局唯一） |
|---------|------------|-------------------|
| `mcp_notion` | `search` | `mcp__mcp_notion__search` |
| `mcp_github` | `create_issue` | `mcp__mcp_github__create_issue` |

**使用方式**：

Agent `mcp_bindings` 中的 `enabled_tools` 使用**原始工具名**（不含前缀），MCP Gateway 负责路由：

```json
{
  "server_id": "mcp_notion",
  "enabled_tools": ["search", "create_page"]
}
```

Agent `tools_whitelist` 也可直接引用完整工具名（`mcp__mcp_notion__search`），两种方式等价。

---

*← 返回 [README.md](./README.md)*
