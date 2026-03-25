# Skills API

> Skill 是可附加到 Agent 的能力模块，包含 Prompt 片段 + 工具授权 + MCP 绑定。多个 Skill 可以叠加组合，运行时合并到 `AgentRuntimeContext.effective_whitelist`。

---

## 目录

- [Skill 对象结构](#skill-对象结构)
- [获取 Skill 列表](#1-获取-skill-列表)
- [获取单个 Skill](#2-获取单个-skill)
- [创建 Skill](#3-创建-skill)
- [更新 Skill](#4-更新-skill)
- [删除 Skill](#5-删除-skill)
- [Skill 组合说明](#skill-组合说明)

---

## Skill 对象结构

```json
{
  "id": "skill_research",
  "tenant_id": null,
  "name": "深度研究",
  "description": "增强 Agent 的信息检索与研究能力，允许使用网络搜索和文件读取工具",
  "is_builtin": true,
  "prompt_fragment": "你具备深度研究能力。在分析问题时，你会主动搜集相关资料，引用可靠来源，对信息进行交叉验证。",
  "tools_grant": ["web_search", "read_file", "grep_files"],
  "mcp_bindings": [],
  "created_at": "2026-01-01T00:00:00Z",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|-----|------|------|
| `tenant_id` | string? | `null` 表示内置 Skill（全租户共享），非 null 表示租户自定义 Skill |
| `is_builtin` | boolean | 内置 Skill 不可修改或删除（I-14） |
| `prompt_fragment` | string | 附加到 Agent System Prompt 末尾的 Prompt 片段 |
| `tools_grant` | string[] | 该 Skill 为 Agent 额外授权的工具列表 |
| `mcp_bindings` | object[] | 该 Skill 附带的 MCP 绑定（结构同 Agent.capability.mcp_bindings） |

### Phase 1 内置 Skill 列表

| Skill ID | 名称 | 工具授权 |
|---------|------|---------|
| `skill_research` | 深度研究 | `web_search`, `read_file`, `grep_files` |
| `skill_code_execution` | 代码执行 | `run_python`, `write_file`, `read_file` |
| `skill_file_management` | 文件管理 | `read_file`, `write_file`, `list_dir`, `delete_file` |
| `skill_http_client` | HTTP 请求 | `http_get`, `http_post` |
| `skill_data_analysis` | 数据分析 | `read_file`, `run_python`, `grep_files` |

---

## 1. 获取 Skill 列表

```http
GET /api/v1/skills
Authorization: Bearer {token}
```

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `page` | integer | 页码，默认 `1` |
| `page_size` | integer | 每页数量，默认 `50` |
| `include_builtin` | boolean | 是否包含内置 Skill，默认 `true` |
| `q` | string? | 按名称搜索 |

**响应 `200 OK`**：

```json
{
  "data": [ /* Skill 数组 */ ],
  "total": 8,
  "page": 1,
  "page_size": 50
}
```

---

## 2. 获取单个 Skill

```http
GET /api/v1/skills/:skill_id
Authorization: Bearer {token}
```

**响应 `200 OK`**：完整 Skill 对象。

---

## 3. 创建 Skill

创建租户自定义 Skill。

```http
POST /api/v1/skills
Authorization: Bearer {token}
```

**所需角色**：`member` 或以上

**请求体**：

```json
{
  "name": "竞品情报分析",
  "description": "专注于竞品数据收集和分析的技能模块",
  "prompt_fragment": "你具备专业的竞品情报分析能力。分析时需关注定价策略、功能特性、市场定位三个维度，并量化比较差异。",
  "tools_grant": ["web_search", "read_file"],
  "mcp_bindings": [
    { "server_id": "mcp_notion", "enabled_tools": ["notion_search"] }
  ]
}
```

| 字段 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `name` | string | ✅ | Skill 名称，1–100 字符 |
| `description` | string | ❌ | 功能描述 |
| `prompt_fragment` | string | ❌ | 附加的 Prompt 片段，默认为空 |
| `tools_grant` | string[] | ❌ | 授权的内置工具列表，默认 `[]` |
| `mcp_bindings` | object[] | ❌ | MCP 绑定，默认 `[]` |

**响应 `201 Created`**：完整 Skill 对象（`is_builtin = false`，`tenant_id` 为当前租户）。

---

## 4. 更新 Skill

```http
PUT /api/v1/skills/:skill_id
Authorization: Bearer {token}
```

**所需角色**：`member`（仅更新自己租户的 Skill）或 `tenant_admin`

**请求体**：同创建，字段均可选。

**错误**：

| 错误码 | 说明 |
|-------|------|
| `cannot_modify_builtin_skill` | 不允许修改内置 Skill（I-14） |
| `skill_not_found` | Skill 不存在 |

**响应 `200 OK`**：更新后的 Skill 对象。

---

## 5. 删除 Skill

```http
DELETE /api/v1/skills/:skill_id
Authorization: Bearer {token}
```

**所需角色**：`tenant_admin`

**行为**：
- 删除 Skill 后，引用该 Skill 的 Agent 的 `skill_ids` 自动移除对应 ID
- 内置 Skill 不可删除（I-14）

**响应 `204 No Content`**

**错误**：`cannot_modify_builtin_skill`（422）

---

## Skill 组合说明

Agent 可附加多个 Skill，运行时合并规则：

```
AgentRuntimeContext.effective_whitelist
  = Agent.tools_whitelist
  ∪ Skill_1.tools_grant
  ∪ Skill_2.tools_grant
  ∪ ...（所有附加 Skill 的授权）
  ∪ MCP 动态工具（通过 mcp_bindings 注册）
```

**示例**：

```
Agent.tools_whitelist = ["read_file"]
+ skill_research.tools_grant = ["web_search", "read_file", "grep_files"]
+ skill_code_execution.tools_grant = ["run_python", "write_file", "read_file"]

→ effective_whitelist = ["read_file", "web_search", "grep_files", "run_python", "write_file"]
（自动去重，无需担心重复）
```

---

---

# Tools API

> 工具是 Agent 执行操作的基本单元。所有工具均注册在 `ToolRegistry` 中，Agent 通过 `tools_whitelist` 显式授权后方可使用。

---

## 目录

- [Tool 对象结构](#tool-对象结构)
- [获取工具列表](#1-获取工具列表)
- [搜索工具](#2-搜索工具)

---

## Tool 对象结构

```json
{
  "name": "web_search",
  "display_name": "网络搜索",
  "description": "通过搜索引擎查找互联网信息",
  "category": "network",
  "risk_level": "medium",
  "parameters": {
    "type": "object",
    "properties": {
      "query": { "type": "string", "description": "搜索关键词" },
      "max_results": { "type": "integer", "description": "返回结果数量，默认 5" }
    },
    "required": ["query"]
  },
  "requires_approval": false,
  "available_in_discussion": true
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|-----|------|------|
| `name` | string | 工具名称（唯一标识，用于 `tools_whitelist`） |
| `category` | string | 工具分类，见下方分类表 |
| `risk_level` | enum | `low` / `medium` / `high` |
| `requires_approval` | boolean | `risk_level = high` 的工具始终为 `true`（I-06） |
| `available_in_discussion` | boolean | 是否可在讨论工具增强模式中使用（high 风险工具始终为 `false`，I-07） |

### 内置工具分类（Phase 1，21 个）

| 分类 | 工具名 | 风险级别 |
|-----|-------|---------|
| **filesystem** | `read_file` | low |
| | `write_file` | medium |
| | `edit_file` | medium |
| | `delete_file` | high |
| | `list_dir` | low |
| | `grep_files` | low |
| **execution** | `run_python` | medium |
| | `run_bash` | high |
| **network** | `web_search` | medium |
| | `http_get` | medium |
| | `http_post` | high |
| **data** | `parse_json` | low |
| | `parse_csv` | low |
| | `render_markdown` | low |
| **system** | `get_system_info` | low |
| | `manage_process` | high |
| **coordination** | `submit_subtask` | low |
| | `wait_for_agent` | low |
| | `report_progress` | low |
| | `raise_decision` | low |
| | `search_tools` | low |

---

## 1. 获取工具列表

```http
GET /api/v1/tools
Authorization: Bearer {token}
```

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `category` | string? | 按分类过滤：`filesystem` / `execution` / `network` / `data` / `system` / `coordination` / `mcp` |
| `risk_level` | string? | `low` / `medium` / `high` |
| `available_in_discussion` | boolean? | 仅返回讨论模式可用工具 |

**响应 `200 OK`**：

```json
{
  "data": [ /* Tool 对象数组 */ ],
  "total": 21
}
```

---

## 2. 搜索工具

供 Agent 自主发现工具使用（`search_tools` 内置工具对应的 API 端点）。

```http
GET /api/v1/tools/search
Authorization: Bearer {token}
```

**查询参数**：

| 参数 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `q` | string | ✅ | 工具功能描述，自然语言，如 `"搜索互联网"` |
| `limit` | integer | ❌ | 返回结果数量，默认 `5`，最大 `20` |
| `context` | enum | ❌ | `task`（默认）/ `discussion`（只搜索讨论模式可用工具） |

**响应 `200 OK`**：

```json
{
  "data": [
    {
      "name": "web_search",
      "display_name": "网络搜索",
      "description": "通过搜索引擎查找互联网信息",
      "risk_level": "medium",
      "relevance_score": 0.95
    }
  ]
}
```

> 此端点使用工具描述的语义相似度搜索，用于 Agent ReAct 循环中的工具自主发现（Tool Search 功能，ADR-09）。

---

*← 返回 [README.md](./README.md)*