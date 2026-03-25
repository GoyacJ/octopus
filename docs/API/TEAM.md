# Teams API

> Team 是多个 Agent 组成的协作单元，用于执行复杂任务。每个 Team 有且仅有一个 Leader Agent 负责任务分解和协调。

---

## 目录

- [Team 对象结构](#team-对象结构)
- [创建 Team](#1-创建-team)
- [获取 Team 列表](#2-获取-team-列表)
- [获取单个 Team](#3-获取单个-team)
- [更新 Team](#4-更新-team)
- [删除 Team](#5-删除-team)
- [成员管理](#成员管理)
    - [添加成员](#6-添加成员)
    - [移除成员](#7-移除成员)
- [路由规则管理](#路由规则管理)
    - [获取路由规则列表](#8-获取路由规则列表)
    - [创建路由规则](#9-创建路由规则)
    - [更新路由规则](#10-更新路由规则)
    - [删除路由规则](#11-删除路由规则)

---

## Team 对象结构

```json
{
  "id": "team_01HX...",
  "tenant_id": "tnt_01HX...",
  "name": "内容营销团队",
  "description": "负责 SEO 分析、内容创作、数据追踪的自动化内容团队",
  "leader_id": "agt_01HX_leader",
  "leader_source": "auto_generated",
  "member_ids": ["agt_01HX_seo", "agt_01HX_writer", "agt_01HX_analyst"],
  "routing_rules": [
    {
      "id": "rule_01HX...",
      "priority": 1,
      "condition": { "type": "keyword", "keywords": ["SEO", "关键词"] },
      "action": { "type": "assign_to_agent", "agent_id": "agt_01HX_seo" },
      "description": "SEO 相关任务直接分配给 SEO 分析师"
    }
  ],
  "task_count": 42,
  "created_at": "2026-03-01T00:00:00Z",
  "updated_at": "2026-03-10T08:00:00Z"
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|-----|------|------|
| `leader_id` | string | Leader Agent ID，**必填**，Team 有且仅有一个 Leader（I-11） |
| `leader_source` | enum | `auto_generated`（系统创建 Leader）/ `user_selected`（从已有 Agent 中选择） |
| `member_ids` | string[] | 成员 Agent ID 列表，**不包含** `leader_id`（I-12） |
| `routing_rules` | object[] | 用户自定义路由规则，按 `priority` 升序执行 |
| `task_count` | integer | 该 Team 历史任务总数（只读统计字段） |

### RoutingRule 对象

```json
{
  "id": "rule_01HX...",
  "team_id": "team_01HX...",
  "priority": 1,
  "condition": {
    "type": "keyword",
    "keywords": ["SEO", "关键词排名"]
  },
  "action": {
    "type": "assign_to_agent",
    "agent_id": "agt_01HX_seo"
  },
  "description": "SEO 相关任务直接分配给 SEO 分析师",
  "enabled": true
}
```

**`condition.type` 可选值**：

| 类型 | 额外字段 | 说明 |
|-----|---------|------|
| `keyword` | `keywords: string[]` | 任务描述包含任意关键词时触发 |
| `regex` | `pattern: string` | 正则匹配任务描述 |
| `always` | — | 始终触发（作为 fallback 规则） |

**`action.type` 可选值**：

| 类型 | 额外字段 | 说明 |
|-----|---------|------|
| `assign_to_agent` | `agent_id: string` | 将任务直接分配给指定 Agent |
| `skip_planning` | — | 跳过 Leader 规划，直接执行 |
| `require_approval` | — | 强制插入审批节点 |

---

## 1. 创建 Team

```http
POST /api/v1/teams
Authorization: Bearer {token}
```

**所需角色**：`member` 或以上

**请求体**：

```json
{
  "name": "内容营销团队",
  "description": "负责 SEO 分析、内容创作、数据追踪",
  "leader": {
    "source": "auto_generated",
    "agent_id": null
  },
  "member_ids": ["agt_01HX_seo", "agt_01HX_writer", "agt_01HX_analyst"]
}
```

**`leader` 字段两种模式**：

```json
// 模式 1：系统自动创建 Leader
{ "source": "auto_generated" }

// 模式 2：从已有 Agent 中选择 Leader
{ "source": "user_selected", "agent_id": "agt_01HX_existing_leader" }
```

| 字段 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `name` | string | ✅ | Team 名称，1–100 字符 |
| `description` | string | ❌ | 职能描述 |
| `leader.source` | enum | ✅ | `auto_generated` 或 `user_selected` |
| `leader.agent_id` | string? | 条件必填 | `user_selected` 时必须提供 |
| `member_ids` | string[] | ❌ | 初始成员列表，默认 `[]` |

**行为说明**：
- `auto_generated` 模式：系统自动创建一个 Leader Agent（基于 Team 描述生成 System Prompt）
- `member_ids` 中包含 `leader_id` 时，系统自动剔除（I-12）

**响应 `201 Created`**：完整 Team 对象（含自动生成的 `leader_id`）。

**错误**：

| 错误码 | 说明 |
|-------|------|
| `agent_not_found` | `member_ids` 或 `leader.agent_id` 中有不存在的 Agent |
| `team_missing_leader` | `leader` 字段格式无效（I-11） |

---

## 2. 获取 Team 列表

```http
GET /api/v1/teams
Authorization: Bearer {token}
```

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `page` | integer | 页码，默认 `1` |
| `page_size` | integer | 每页数量，默认 `20` |
| `q` | string? | 按名称模糊搜索 |

**响应 `200 OK`**：

```json
{
  "data": [ /* Team 对象数组，member_ids 和 routing_rules 内联 */ ],
  "total": 5,
  "page": 1,
  "page_size": 20
}
```

---

## 3. 获取单个 Team

```http
GET /api/v1/teams/:team_id
Authorization: Bearer {token}
```

**响应 `200 OK`**：完整 Team 对象，含 `routing_rules` 列表。

---

## 4. 更新 Team

更新 Team 基本信息（名称、描述、更换 Leader）。

```http
PUT /api/v1/teams/:team_id
Authorization: Bearer {token}
```

**所需角色**：`member`（仅创建者）或 `tenant_admin`

**请求体**（字段均可选）：

```json
{
  "name": "内容营销团队 v2",
  "description": "更新后的描述",
  "leader_id": "agt_01HX_new_leader"
}
```

> **更换 Leader**：提供 `leader_id` 时，新 Leader 必须是现有成员（已在 `member_ids` 中）。更换后原 Leader 自动加入 `member_ids`。

**响应 `200 OK`**：更新后的完整 Team 对象。

---

## 5. 删除 Team

```http
DELETE /api/v1/teams/:team_id
Authorization: Bearer {token}
```

**所需角色**：`member`（仅创建者）或 `tenant_admin`

**行为**：
- 级联删除 `team_members`、`routing_rules` 关联记录
- 如果该 Team 有 `status = running` 的任务，**拒绝删除**
- `auto_generated` 类型的 Leader Agent 一并删除；`user_selected` 的 Leader 保留（只是解除关联）

**响应 `204 No Content`**

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `team_not_found` | 404 | Team 不存在 |
| `team_has_running_tasks` | 409 | 该 Team 有进行中的任务 |

---

## 成员管理

### 6. 添加成员

将一个或多个 Agent 添加为 Team 成员。

```http
POST /api/v1/teams/:team_id/members
Authorization: Bearer {token}
```

**请求体**：

```json
{
  "agent_ids": ["agt_01HX_new1", "agt_01HX_new2"]
}
```

**行为**：
- 传入已是成员的 Agent ID 时幂等处理（不报错）
- 传入 `leader_id` 时自动忽略

**响应 `200 OK`**：更新后的 Team 对象（含新成员列表）。

**错误**：

| 错误码 | 说明 |
|-------|------|
| `agent_not_found` | 有 Agent ID 不存在 |
| `agent_belongs_to_different_tenant` | Agent 不属于同一租户 |

---

### 7. 移除成员

```http
DELETE /api/v1/teams/:team_id/members/:agent_id
Authorization: Bearer {token}
```

**行为**：
- 不可移除 Leader（需先更换 Leader 才能移除）
- 如果该 Agent 当前正在执行该 Team 的子任务（`status = busy`），**拒绝移除**

**响应 `204 No Content`**

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `agent_not_member` | 404 | 该 Agent 不是 Team 成员 |
| `cannot_remove_leader` | 409 | 不可直接移除 Leader |
| `agent_busy` | 409 | Agent 正在执行子任务 |

---

## 路由规则管理

路由规则决定任务如何分配给 Agent，优先级高于 Leader 的 LLM 规划决策。

### 8. 获取路由规则列表

```http
GET /api/v1/teams/:team_id/routing-rules
Authorization: Bearer {token}
```

**响应 `200 OK`**：

```json
{
  "data": [ /* RoutingRule 数组，按 priority 升序 */ ]
}
```

---

### 9. 创建路由规则

```http
POST /api/v1/teams/:team_id/routing-rules
Authorization: Bearer {token}
```

**请求体**：

```json
{
  "priority": 1,
  "condition": {
    "type": "keyword",
    "keywords": ["紧急", "P0", "故障"]
  },
  "action": {
    "type": "require_approval"
  },
  "description": "涉及紧急或故障的任务强制审批",
  "enabled": true
}
```

| 字段 | 必填 | 说明 |
|-----|-----|------|
| `priority` | ✅ | 执行优先级，数字越小越先匹配，同优先级按创建时间排序 |
| `condition` | ✅ | 触发条件 |
| `action` | ✅ | 匹配后执行的动作 |
| `description` | ❌ | 规则说明 |
| `enabled` | ❌ | 是否启用，默认 `true` |

**响应 `201 Created`**：完整 RoutingRule 对象。

---

### 10. 更新路由规则

```http
PUT /api/v1/teams/:team_id/routing-rules/:rule_id
Authorization: Bearer {token}
```

**请求体**：同创建，字段均可选。

**响应 `200 OK`**：更新后的 RoutingRule 对象。

---

### 11. 删除路由规则

```http
DELETE /api/v1/teams/:team_id/routing-rules/:rule_id
Authorization: Bearer {token}
```

**响应 `204 No Content`**

---

*← 返回 [README.md](./README.md)*