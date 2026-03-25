# Agents API

> Agent 是 octopus 的核心实体，对现实世界"人"的数字化抽象，拥有 Identity（身份）、Capability（能力）、Memory（记忆）三个维度。

---

## 目录

- [Agent 对象结构](#agent-对象结构)
- [创建 Agent](#1-创建-agent)
- [获取 Agent 列表](#2-获取-agent-列表)
- [获取单个 Agent](#3-获取单个-agent)
- [更新 Agent](#4-更新-agent)
- [删除 Agent](#5-删除-agent)
- [记忆管理](#记忆管理)
    - [查询 Agent 记忆列表](#6-查询-agent-记忆列表)
    - [手动写入记忆](#7-手动写入记忆)
    - [删除记忆条目](#8-删除记忆条目)

---

## Agent 对象结构

```json
{
  "id": "agt_01HX...",
  "tenant_id": "tnt_01HX...",
  "created_by": "usr_01HX...",
  "status": "idle",

  "identity": {
    "name": "Alex · 市场分析师",
    "avatar_url": "https://hub.example.com/avatars/alex.png",
    "role": "负责市场竞品分析，产出结构化洞察报告",
    "persona": ["严谨", "逻辑性强", "简洁"],
    "system_prompt": "你是 Alex，一位经验丰富的市场分析师...",
    "prompt_version": 1
  },

  "capability": {
    "model_config": {
      "provider": "openai",
      "model": "gpt-4o",
      "temperature": 0.7,
      "max_tokens": 4096,
      "top_p": null,
      "api_key_ref": "key_openai_default"
    },
    "tools_whitelist": ["read_file", "web_search", "write_file"],
    "skill_ids": ["skill_research", "skill_report_writing"],
    "mcp_bindings": [
      {
        "server_id": "mcp_notion",
        "enabled_tools": ["notion_search", "notion_create_page"]
      }
    ]
  },

  "memory_store_id": "mem_agt_01HX...",
  "created_at": "2026-03-01T00:00:00Z",
  "updated_at": "2026-03-10T08:00:00Z"
}
```

### 字段说明

**顶层字段**：

| 字段 | 类型 | 说明 |
|-----|------|------|
| `id` | string | Agent 唯一 ID（UUID v4） |
| `tenant_id` | string | 所属租户 ID |
| `created_by` | string | 创建者用户 ID |
| `status` | enum | `idle` / `busy` / `error` |
| `memory_store_id` | string | 向量记忆库 ID，**不可修改**（领域不变量 I-02） |

**`identity` 子对象**：

| 字段 | 类型 | 约束 | 说明 |
|-----|------|-----|------|
| `name` | string | 1–100 字符 | Agent 名称 |
| `avatar_url` | string? | — | 头像 URL |
| `role` | string | 非空 | 角色描述（自然语言） |
| `persona` | string[] | 最多 10 个 | 性格标签 |
| `system_prompt` | string | **非空**（I-01） | 核心 System Prompt，支持 `{{变量}}` 语法 |
| `prompt_version` | integer | 只读，自增 | Prompt 历史版本号 |

**`capability.model_config` 子对象**：

| 字段 | 类型 | 说明 |
|-----|------|------|
| `provider` | enum | `openai` / `anthropic` / `gemini` / `ollama` / `compatible` |
| `model` | string | 具体模型版本，如 `"gpt-4o"` |
| `temperature` | float | 范围 [0.0, 2.0]，默认 `0.7` |
| `max_tokens` | integer | 默认 `4096` |
| `top_p` | float? | 可选 |
| `api_key_ref` | string? | 密钥标识符（非明文，Hub 安全存储中的引用键） |

**`capability.mcp_bindings` 元素**：

| 字段 | 类型 | 说明 |
|-----|------|------|
| `server_id` | string | MCP Server ID（须已注册） |
| `enabled_tools` | string[]? | `null` 表示启用该服务所有工具；否则为允许的工具子集 |

---

## 1. 创建 Agent

```http
POST /api/v1/agents
Authorization: Bearer {token}
```

**所需角色**：`member` 或以上

**请求体**：

```json
{
  "name": "Alex · 市场分析师",
  "avatar_url": null,
  "role": "负责市场竞品分析，产出结构化洞察报告",
  "persona": ["严谨", "逻辑性强", "简洁"],
  "system_prompt": "你是 Alex，一位经验丰富的市场分析师。你擅长...",
  "model_config": {
    "provider": "openai",
    "model": "gpt-4o",
    "temperature": 0.7,
    "max_tokens": 4096,
    "top_p": null,
    "api_key_ref": "key_openai_default"
  },
  "tools_whitelist": ["read_file", "web_search"],
  "skill_ids": ["skill_research"],
  "mcp_bindings": []
}
```

**所有字段**：

| 字段 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `name` | string | ✅ | Agent 名称，1–100 字符 |
| `avatar_url` | string? | ❌ | 头像 URL |
| `role` | string | ✅ | 角色描述 |
| `persona` | string[] | ❌ | 性格标签，默认 `[]` |
| `system_prompt` | string | ✅ | 非空字符串 |
| `model_config` | object | ✅ | LLM 配置，至少需要 `provider` + `model` |
| `tools_whitelist` | string[] | ❌ | 内置工具白名单，默认 `[]` |
| `skill_ids` | string[] | ❌ | 附加 Skill，默认 `[]` |
| `mcp_bindings` | object[] | ❌ | MCP 绑定，默认 `[]` |

**响应 `201 Created`**：

```json
{
  "data": { /* 完整 Agent 对象，含自动生成的 id、memory_store_id 等 */ }
}
```

**错误**：

| 错误码 | 说明 |
|-------|------|
| `invalid_system_prompt` | `system_prompt` 为空 |
| `tool_not_registered` | `tools_whitelist` 中有未注册的工具名 |
| `skill_not_found` | `skill_ids` 中有不存在的 Skill ID |
| `mcp_server_not_found` | `mcp_bindings` 中的 `server_id` 不存在 |
| `validation_error` | 字段格式错误（如 `persona` 超过 10 个） |

---

## 2. 获取 Agent 列表

```http
GET /api/v1/agents
Authorization: Bearer {token}
```

**所需角色**：`viewer` 或以上

**查询参数**：

| 参数 | 类型 | 默认值 | 说明 |
|-----|------|-------|------|
| `page` | integer | `1` | 页码 |
| `page_size` | integer | `20` | 每页数量，最大 `100` |
| `status` | string? | — | 按状态过滤：`idle` / `busy` / `error` |
| `q` | string? | — | 按名称模糊搜索 |

**响应 `200 OK`**：

```json
{
  "data": [ /* Agent 对象数组 */ ],
  "total": 15,
  "page": 1,
  "page_size": 20
}
```

---

## 3. 获取单个 Agent

```http
GET /api/v1/agents/:agent_id
Authorization: Bearer {token}
```

**所需角色**：`viewer` 或以上

**响应 `200 OK`**：完整 Agent 对象。

**错误**：`agent_not_found`（404）

---

## 4. 更新 Agent

全量替换 Agent 配置（Identity + Capability 维度）。

```http
PUT /api/v1/agents/:agent_id
Authorization: Bearer {token}
```

**所需角色**：`member`（只能更新自己创建的 Agent）或 `tenant_admin`

**请求体**：同[创建 Agent](#1-创建-agent) 请求体，所有字段可选（仅传入需要修改的字段）。

> **注意**：`memory_store_id` 不可出现在请求体中，否则返回 `memory_store_id_immutable`（I-02）。
>
> 每次成功修改 `system_prompt` 时，`prompt_version` 自动 +1。

**响应 `200 OK`**：更新后的完整 Agent 对象。

**错误**：

| 错误码 | 说明 |
|-------|------|
| `agent_not_found` | Agent 不存在 |
| `agent_busy` | Agent 正在执行任务，此时不允许修改配置 |
| `memory_store_id_immutable` | 请求体中包含 `memory_store_id` 字段 |

---

## 5. 删除 Agent

```http
DELETE /api/v1/agents/:agent_id
Authorization: Bearer {token}
```

**所需角色**：`member`（只能删除自己创建的 Agent）或 `tenant_admin`

**行为**：
- 级联删除 Agent 的所有记忆（向量库 Collection 同步清理）
- 级联删除 `agent_skills`、`agent_mcp_bindings` 关联记录
- 如果 Agent 是某个 Team 的 Leader，**拒绝删除**（需先更换 Leader 或删除 Team）
- 如果 Agent 当前 `status = busy`，**拒绝删除**

**响应 `204 No Content`**

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `agent_not_found` | 404 | Agent 不存在 |
| `agent_busy` | 409 | Agent 正在执行任务 |
| `agent_is_team_leader` | 409 | Agent 是某 Team 的 Leader，无法删除 |

---

## 记忆管理

Agent 记忆通过任务/讨论结束后由系统自动写入（`ExperienceHarvester`）。以下端点用于**查看和手动管理**记忆条目。

> 设计原则（ADR-11）：记忆 API 挂载在 `/agents/:id/` 下，体现记忆归属于 Agent。系统中没有独立的 `/memories` 顶级路由。

### MemoryEntry 对象结构

```json
{
  "id": "mem_entry_01HX...",
  "agent_id": "agt_01HX...",
  "content": "在 2026年3月 的竞品分析任务中，发现目标市场 A 的主要竞品在定价策略上...",
  "source_type": "task",
  "source_id": "tsk_01HX...",
  "created_at": "2026-03-10T09:00:00Z",
  "metadata": {
    "topic": "竞品分析",
    "task_title": "Q1 市场分析报告"
  }
}
```

| 字段 | 类型 | 说明 |
|-----|------|------|
| `source_type` | enum | `task`（任务完成后自动提取）/ `discussion`（讨论结束后自动提取）/ `manual`（手动写入） |
| `source_id` | string | 来源 Task ID 或 DiscussionSession ID |
| `metadata` | object | 扩展字段，用于 UI 展示和过滤 |

---

### 6. 查询 Agent 记忆列表

```http
GET /api/v1/agents/:agent_id/memories
Authorization: Bearer {token}
```

**所需角色**：`viewer` 或以上（且需有权限访问该 Agent）

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `q` | string? | 语义搜索关键词（通过向量检索相似记忆） |
| `source_type` | string? | 按来源类型过滤：`task` / `discussion` / `manual` |
| `after` | string? | 游标分页：返回此 entry_id 之后的记录 |
| `limit` | integer | 每次拉取条数，默认 `20`，最大 `100` |

**两种查询模式**：

1. **语义搜索**（传入 `q`）：通过 RAG 向量检索，返回语义相关的记忆，按相似度排序。
2. **顺序列表**（不传 `q`）：按时间倒序列出所有记忆，支持游标分页。

**响应 `200 OK`**：

```json
{
  "data": [ /* MemoryEntry 数组 */ ],
  "next_cursor": "mem_entry_01HY...",
  "has_more": true
}
```

> 语义搜索模式下不返回 `next_cursor`（一次性返回 top-k 结果，默认 k=10）。

---

### 7. 手动写入记忆

向 Agent 手动添加一条记忆条目（`source_type = manual`）。

```http
POST /api/v1/agents/:agent_id/memories
Authorization: Bearer {token}
```

**所需角色**：`member` 或以上

**请求体**：

```json
{
  "content": "该 Agent 负责的市场区域已调整为东南亚市场，需重点关注...",
  "metadata": {
    "topic": "市场调整",
    "operator": "alice"
  }
}
```

| 字段 | 类型 | 必填 | 约束 |
|-----|------|-----|------|
| `content` | string | ✅ | 非空（I-04） |
| `metadata` | object | ❌ | 自定义键值对 |

**响应 `201 Created`**：

```json
{
  "data": { /* 完整 MemoryEntry 对象 */ }
}
```

**错误**：

| 错误码 | 说明 |
|-------|------|
| `agent_not_found` | Agent 不存在 |
| `empty_memory_content` | `content` 为空（I-04） |

---

### 8. 删除记忆条目

```http
DELETE /api/v1/agents/:agent_id/memories/:entry_id
Authorization: Bearer {token}
```

**所需角色**：`member`（仅可删除 `source_type = manual` 的手动记忆）或 `tenant_admin`（可删除任意记忆）

**响应 `204 No Content`**

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `agent_not_found` | 404 | Agent 不存在 |
| `memory_entry_not_found` | 404 | 记忆条目不存在 |
| `forbidden` | 403 | 尝试删除非 `manual` 类型记忆但权限不足 |

---

*← 返回 [README.md](./README.md)*
