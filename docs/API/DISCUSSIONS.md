# Discussions API

> Discussion 是多 Agent 圆桌会议 / 头脑风暴 / 辩论会话，Agent 围绕议题迭代发言，用户可插话介入，最终生成结论。

---

## 目录

- [DiscussionSession 对象结构](#discussionsession-对象结构)
- [DiscussionTurn 对象结构](#discussionturn-对象结构)
- [创建讨论会话](#1-创建讨论会话)
- [获取讨论会话列表](#2-获取讨论会话列表)
- [获取单个讨论会话](#3-获取单个讨论会话)
- [用户插话](#4-用户插话)
- [暂停讨论](#5-暂停讨论)
- [恢复讨论](#6-恢复讨论)
- [生成并结束讨论（结论）](#7-生成并结束讨论结论)
- [获取发言记录](#8-获取发言记录)
- [删除讨论会话](#9-删除讨论会话)
- [讨论状态流转](#讨论状态流转)
- [讨论模式与发言策略说明](#讨论模式与发言策略说明)

---

## DiscussionSession 对象结构

```json
{
  "id": "disc_01HX...",
  "tenant_id": "tnt_01HX...",
  "created_by": "usr_01HX...",
  "topic": "新功能「情绪分析」的可行性评审",
  "mode": "roundtable",
  "status": "active",
  "turn_strategy": "sequential",
  "participants": [
    {
      "agent_id": "agt_01HX_product",
      "role_in_discussion": "产品设计师",
      "debate_position": null
    },
    {
      "agent_id": "agt_01HX_engineer",
      "role_in_discussion": "研发工程师",
      "debate_position": null
    },
    {
      "agent_id": "agt_01HX_qa",
      "role_in_discussion": "测试工程师",
      "debate_position": null
    }
  ],
  "moderator_id": null,
  "config": {
    "max_turns_per_agent": 5,
    "context_window_size": 10,
    "tool_augmented": false,
    "debate_positions": null
  },
  "current_round": 2,
  "conclusion": null,
  "created_at": "2026-03-10T10:00:00Z",
  "updated_at": "2026-03-10T10:15:00Z",
  "concluded_at": null
}
```

### 字段说明

| 字段 | 类型 | 说明 |
|-----|------|------|
| `topic` | string | 讨论主题（传递给所有参与 Agent 的上下文） |
| `mode` | enum | 讨论模式，见下方说明 |
| `status` | enum | `active` / `paused` / `concluded` |
| `turn_strategy` | enum | `sequential`（顺序轮流）/ `moderated`（主持人指定） |
| `participants` | object[] | 参与者列表，2–8 个 Agent（I-08） |
| `moderator_id` | string? | 主持人 Agent ID，`turn_strategy = moderated` 时必填（I-10） |
| `config.max_turns_per_agent` | integer | 每个 Agent 最多发言轮数，默认 `5` |
| `config.context_window_size` | integer | 上下文保留近 N 轮完整发言，默认 `10`，更早的历史转为滚动摘要 |
| `config.tool_augmented` | boolean | 是否允许 Agent 在讨论中使用低风险工具（如 `web_search`） |
| `config.debate_positions` | object? | 辩论模式下各 Agent 的预设立场 Map |
| `current_round` | integer | 当前已完成的轮次数 |

**`participants` 元素说明**：

| 字段 | 类型 | 说明 |
|-----|------|------|
| `agent_id` | string | 参与者 Agent ID |
| `role_in_discussion` | string? | 该 Agent 在本次讨论中的角色描述（可覆盖 Agent 的默认角色） |
| `debate_position` | string? | 辩论模式下的预设立场（如 `"支持方"` / `"反对方"`） |

---

## DiscussionTurn 对象结构

```json
{
  "id": "turn_01HX...",
  "session_id": "disc_01HX...",
  "turn_number": 5,
  "speaker_type": "agent",
  "speaker_id": "agt_01HX_engineer",
  "content": "从工程角度看，情绪分析有两条技术路径...",
  "created_at": "2026-03-10T10:08:00Z"
}
```

| 字段 | 说明 |
|-----|------|
| `turn_number` | 全局发言序号，从 1 开始单调递增 |
| `speaker_type` | `agent`（Agent 发言）/ `user`（用户插话）/ `system`（系统消息，如"讨论已暂停"） |
| `speaker_id` | Agent ID 或 User ID；`system` 时为 `null` |
| `content` | 发言全文（不可修改，I-17） |

---

## 1. 创建讨论会话

创建并自动启动讨论。

```http
POST /api/v1/discussions
Authorization: Bearer {token}
```

**所需角色**：`discussion.create` 或 `member` 以上

**请求体**：

```json
{
  "topic": "新功能「情绪分析」的可行性评审",
  "mode": "roundtable",
  "turn_strategy": "sequential",
  "participants": [
    { "agent_id": "agt_01HX_product", "role_in_discussion": "产品设计师" },
    { "agent_id": "agt_01HX_engineer", "role_in_discussion": "研发工程师" },
    { "agent_id": "agt_01HX_qa", "role_in_discussion": "测试工程师" }
  ],
  "moderator_id": null,
  "config": {
    "max_turns_per_agent": 5,
    "context_window_size": 10,
    "tool_augmented": false
  }
}
```

**字段说明**：

| 字段 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `topic` | string | ✅ | 讨论主题，非空 |
| `mode` | enum | ✅ | `roundtable` / `brainstorm` / `debate` |
| `turn_strategy` | enum | ❌ | `sequential`（默认）/ `moderated` |
| `participants` | object[] | ✅ | 2–8 个参与者（I-08） |
| `moderator_id` | string? | 条件必填 | `turn_strategy = moderated` 时必须提供（I-10） |
| `config.max_turns_per_agent` | integer | ❌ | 默认 `5` |
| `config.context_window_size` | integer | ❌ | 默认 `10` |
| `config.tool_augmented` | boolean | ❌ | 默认 `false` |

**辩论模式附加字段**（`mode = debate` 时）：

```json
{
  "mode": "debate",
  "participants": [
    { "agent_id": "agt_01HX_a", "debate_position": "支持方" },
    { "agent_id": "agt_01HX_b", "debate_position": "反对方" }
  ]
}
```

> 辩论模式下若部分参与者未指定立场，系统自动为其分配对立立场（I-09）。

**响应 `201 Created`**：完整 DiscussionSession 对象。

讨论创建后立即启动，第一个 Agent 将在后台开始思考并发言，通过 SSE 推送实时 token。

**错误**：

| 错误码 | 说明 |
|-------|------|
| `invalid_participant_count` | 参与者数量不在 [2, 8]（I-08） |
| `missing_moderator` | Moderated 策略未指定 moderator（I-10） |
| `agent_not_found` | 有参与者 Agent 不存在 |
| `duplicate_participant` | 同一 Agent 重复出现在参与者列表 |

---

## 2. 获取讨论会话列表

```http
GET /api/v1/discussions
Authorization: Bearer {token}
```

**所需角色**：`discussion.view` 或以上

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `page` | integer | 页码，默认 `1` |
| `page_size` | integer | 每页数量，默认 `20` |
| `status` | string? | `active` / `paused` / `concluded`，支持逗号分隔多个 |
| `mode` | string? | `roundtable` / `brainstorm` / `debate` |
| `sort` | string | `created_at`（默认降序） |

**响应 `200 OK`**：

```json
{
  "data": [
    {
      "id": "disc_01HX...",
      "topic": "新功能「情绪分析」的可行性评审",
      "mode": "roundtable",
      "status": "active",
      "participant_count": 3,
      "current_round": 2,
      "created_at": "2026-03-10T10:00:00Z"
    }
  ],
  "total": 8,
  "page": 1,
  "page_size": 20
}
```

> 列表不含 `participants` 详细信息和 `conclusion`，使用单个会话端点获取完整数据。

---

## 3. 获取单个讨论会话

```http
GET /api/v1/discussions/:session_id
Authorization: Bearer {token}
```

**所需角色**：`discussion.view` 或以上

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `include` | string? | 逗号分隔按需包含：`turns`（最近 N 条发言）、`conclusion` |
| `turns_limit` | integer | 配合 `include=turns` 使用，返回最近 N 条，默认 `20` |

**响应 `200 OK`**：完整 DiscussionSession 对象，含请求的 `include` 字段。

---

## 4. 用户插话

用户在进行中的讨论中插入发言，打断或引导讨论方向。

```http
POST /api/v1/discussions/:session_id/inject
Authorization: Bearer {token}
```

**所需角色**：讨论创建者或 `tenant_admin`

**请求体**：

```json
{
  "content": "重点关注一下成本和上线周期",
  "directed_to": null
}
```

| 字段 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `content` | string | ✅ | 用户插话内容，非空 |
| `directed_to` | string? | ❌ | 定向提问：指定某个参与者 Agent ID；`null` 表示广播给所有参与者 |

**行为**：
1. 用户插话写入为一条 `speaker_type = user` 的 DiscussionTurn
2. 若讨论当前有 Agent 正在发言，插话在该轮完成后生效（不会中断流式输出）
3. 下一轮发言时，所有 Agent 的上下文中将包含此插话
4. `directed_to` 非空时，下一个发言的 Agent 优先级提升为指定 Agent

**响应 `200 OK`**：创建的 DiscussionTurn 对象（`speaker_type = user`）。

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `discussion_not_active` | 409 | 讨论未处于 active 状态 |
| `agent_not_participant` | 422 | `directed_to` 指定的 Agent 不是参与者 |

---

## 5. 暂停讨论

暂停正在进行的讨论，Agent 完成当前发言后停止。

```http
POST /api/v1/discussions/:session_id/pause
Authorization: Bearer {token}
```

**所需角色**：讨论创建者或 `tenant_admin`

**响应 `200 OK`**：

```json
{
  "data": {
    "id": "disc_01HX...",
    "status": "paused"
  }
}
```

**错误**：`discussion_not_active`（409）

---

## 6. 恢复讨论

从暂停状态恢复讨论，继续下一轮发言。

```http
POST /api/v1/discussions/:session_id/resume
Authorization: Bearer {token}
```

**所需角色**：讨论创建者或 `tenant_admin`

**请求体**（可选，恢复时附加一条引导语）：

```json
{
  "inject_on_resume": "请继续，重点围绕成本展开"
}
```

**响应 `200 OK`**：

```json
{
  "data": {
    "id": "disc_01HX...",
    "status": "active"
  }
}
```

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `discussion_not_paused` | 409 | 讨论不在暂停状态 |

---

## 7. 生成并结束讨论（结论）

触发结论生成，讨论结束后 Agent 异步写入记忆。

```http
POST /api/v1/discussions/:session_id/conclude
Authorization: Bearer {token}
```

**所需角色**：`discussion.conclude` 或 `tenant_admin`

**请求体**（可选）：

```json
{
  "final_injection": "请各位给出最终建议",
  "write_memories": true
}
```

| 字段 | 类型 | 默认值 | 说明 |
|-----|------|-------|------|
| `final_injection` | string? | — | 结论生成前最后一条用户消息 |
| `write_memories` | boolean | `true` | 讨论结束后是否触发各 Agent 记忆写入 |

**行为**：
1. 将当前 `status → concluded`，`concluded_at` 写入
2. 系统调用 `ConclusionSummarizer` 基于全部发言生成结论（含各方核心观点、风险点、推荐方案、后续行动建议）
3. 结论通过 SSE `discussion.concluded` 事件推送，并存储到 `discussion_sessions.conclusion`
4. 若 `write_memories = true`，异步触发各 Agent 的经验提取与记忆写入（`ExperienceHarvester`），完成后推送 `discussion.memorized` 事件

**响应 `200 OK`**（结论生成完成前可能有几秒延迟，**推荐通过 SSE 监听**，此端点也会等待完成后返回）：

```json
{
  "data": {
    "id": "disc_01HX...",
    "status": "concluded",
    "conclusion": "## 讨论结论\n\n**各方核心观点**：\n...\n\n**主要风险点**：...\n\n**推荐方案**：分阶段上线...\n\n**后续行动建议**：...",
    "concluded_at": "2026-03-10T10:30:00Z"
  }
}
```

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `discussion_not_active` | 409 | 讨论未处于 active 或 paused 状态 |
| `insufficient_turns` | 422 | 发言轮次过少（< 2 轮），无法生成有意义的结论 |

---

## 8. 获取发言记录

```http
GET /api/v1/discussions/:session_id/turns
Authorization: Bearer {token}
```

**所需角色**：`discussion.view` 或以上

**查询参数**：

| 参数 | 类型 | 说明 |
|-----|------|------|
| `after` | string? | 游标：返回此 turn_id 之后的发言记录 |
| `limit` | integer | 每次拉取条数，默认 `50`，最大 `200` |
| `speaker_type` | string? | 按发言者类型过滤：`agent` / `user` / `system` |
| `agent_id` | string? | 只看指定 Agent 的发言 |

**响应 `200 OK`**：

```json
{
  "data": [ /* DiscussionTurn 数组，按 turn_number 升序 */ ],
  "next_cursor": "turn_01HY...",
  "has_more": true
}
```

---

## 9. 删除讨论会话

```http
DELETE /api/v1/discussions/:session_id
Authorization: Bearer {token}
```

**所需角色**：讨论创建者或 `tenant_admin`

**约束**：只允许删除 `status = concluded` 的已结束讨论（进行中的讨论必须先 conclude 或被系统自动关闭）。

**行为**：级联删除所有 `discussion_turns` 发言记录。

**响应 `204 No Content`**

**错误**：

| 错误码 | 状态码 | 说明 |
|-------|-------|------|
| `discussion_not_found` | 404 | 会话不存在 |
| `discussion_still_active` | 409 | 讨论尚未结束 |

---

## 讨论状态流转

```
创建 → active
         │
    ┌────┤ pause
    │    │
  paused │
    │    │ resume
    └────┤
         │
         │ conclude（主动触发）
         │ 或 max_turns_per_agent 耗尽（自动触发）
         │
      concluded（终态）
```

---

## 讨论模式与发言策略说明

### 三种讨论模式

| 模式 | 说明 | 适用场景 |
|-----|------|---------|
| `roundtable` | 圆桌讨论：各方平等发言，从专业视角分析 | 需求评审、方案讨论 |
| `brainstorm` | 头脑风暴：鼓励发散，不批判，积累创意 | 创意生成、产品头脑风暴 |
| `debate` | 辩论：参与者持对立立场互相论辩 | 投资分析、技术选型辩论 |

### 两种发言策略

| 策略 | 说明 |
|-----|------|
| `sequential` | 顺序轮流：Agent 按列表顺序依次发言，简单可预期 |
| `moderated` | 主持人指定：moderator Agent 决定每轮谁发言并引导讨论方向 |

### 工具使用说明

讨论模式下工具使用受到限制（I-07）：

| 场景 | 工具限制 |
|-----|---------|
| `tool_augmented = false`（默认） | Agent 不可使用任何工具，纯语言交流 |
| `tool_augmented = true` | 仅允许 `RiskLevel::Low` 工具（如 `web_search`、`read_file`） |
| 任何情况 | `RiskLevel::High` 工具（如 `run_bash`）**硬编码禁用** |

---

*← 返回 [README.md](./README.md)*
