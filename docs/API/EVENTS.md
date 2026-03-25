# Events API（SSE 实时事件）

> Hub 通过 Server-Sent Events（SSE）向 Client 推送所有实时状态变更。Client 建立一个长连接，Hub 按需推送事件，无需轮询。

---

## 目录

- [连接 SSE 事件流](#1-连接-sse-事件流)
- [SSE 消息格式](#sse-消息格式)
- [Task 相关事件](#task-相关事件)
- [Discussion 相关事件](#discussion-相关事件)
- [Agent 相关事件](#agent-相关事件)
- [MCP 相关事件](#mcp-相关事件)
- [系统事件](#系统事件)
- [事件类型速查表](#事件类型速查表)
- [Client 端处理建议](#client-端处理建议)

---

## 1. 连接 SSE 事件流

```http
GET /api/v1/events
Authorization: Bearer {token}
Accept: text/event-stream
```

**所需角色**：任意已认证用户

**行为**：
- 建立持久 HTTP 连接（`Content-Type: text/event-stream`）
- Hub 推送当前租户范围内用户有权限查看的所有事件
- 连接断开后，Client 应自动重连（建议指数退避，最大间隔 30s）
- 每 30 秒发送一次 `:keep-alive` 心跳注释，防止连接超时

**响应头**：

```
HTTP/1.1 200 OK
Content-Type: text/event-stream
Cache-Control: no-cache
Connection: keep-alive
X-Accel-Buffering: no
```

### 过滤参数

```http
GET /api/v1/events?filter=task.*,discussion.*
```

| 参数 | 类型 | 说明 |
|-----|------|------|
| `filter` | string? | 逗号分隔的事件类型过滤，支持通配符 `*`；默认接收全部事件 |

---

## SSE 消息格式

每个事件为标准 SSE 格式：

```
id: evt_01HX...
event: task.status_changed
data: {"task_id":"tsk_01HX...","status":"running","updated_at":"2026-03-10T08:01:00Z"}

```

（每个事件后跟空行分隔）

| 字段 | 说明 |
|-----|------|
| `id` | 事件唯一 ID，Client 可用于断线重连时的 `Last-Event-ID` 头，避免漏事件 |
| `event` | 事件类型字符串 |
| `data` | JSON 格式的事件数据 |

### 断线重连

Client 重连时携带 `Last-Event-ID` 头，Hub 将推送该 ID 之后的缓存事件（缓存时长 5 分钟）：

```http
GET /api/v1/events
Last-Event-ID: evt_01HX_prev...
```

---

## Task 相关事件

### `task.status_changed`

任务状态变更时推送。

```json
{
  "task_id": "tsk_01HX...",
  "status": "running",
  "previous_status": "planning",
  "updated_at": "2026-03-10T08:01:00Z"
}
```

**触发时机**：任务状态进入任何新状态（`pending → planning → running → waiting_approval → completed / failed / terminated`）

---

### `subtask.progress`

子任务进度更新（状态变更）。

```json
{
  "task_id": "tsk_01HX...",
  "subtask_id": "sub_01HX...",
  "agent_id": "agt_01HX...",
  "status": "running",
  "description": "收集竞品信息与市场数据"
}
```

---

### `agent.token_stream`

Agent 在任务执行中流式输出 token（用于实时渲染 Agent 思考/输出内容）。

```json
{
  "task_id": "tsk_01HX...",
  "subtask_id": "sub_01HX...",
  "agent_id": "agt_01HX...",
  "token": "正在分析"
}
```

> **使用建议**：UI 收到 `agent.token_stream` 后追加渲染，直到收到 `subtask.progress`（status=completed）时替换为最终完整输出。

---

### `decision.pending`

Agent 提交了需要用户审批的决策请求。

```json
{
  "task_id": "tsk_01HX...",
  "decisions": [
    {
      "id": "dec_01HX...",
      "source_agent_id": "agt_01HX...",
      "type": "deliverable_review",
      "content": "市场调研报告初稿已完成，请确认是否继续...",
      "artifact_ref": "https://hub.example.com/artifacts/report.md"
    }
  ]
}
```

> 一次可能推送多个决策（Leader 汇总后统一推送）。

---

### `decision.auto_resolved`

高风险工具触发了审批，但被自动处理（当前不使用，预留字段）。

```json
{
  "task_id": "tsk_01HX...",
  "decision_id": "dec_01HX...",
  "memory_ref": "mem_entry_01HX..."
}
```

---

### `task.completed`

任务完成，附带最终结果摘要。

```json
{
  "task_id": "tsk_01HX...",
  "result": "## 竞品分析报告\n\n经过深度分析，5 家竞品的核心差异如下...",
  "completed_at": "2026-03-10T09:30:00Z"
}
```

---

### `task.failed`

任务执行失败。

```json
{
  "task_id": "tsk_01HX...",
  "error": "Agent 调用 LLM API 超时，已重试 3 次仍失败",
  "failed_subtask_id": "sub_01HX...",
  "failed_at": "2026-03-10T09:30:00Z"
}
```

---

### `task.terminated`

任务被用户手动终止。

```json
{
  "task_id": "tsk_01HX...",
  "terminated_by": "usr_01HX...",
  "terminated_at": "2026-03-10T09:00:00Z"
}
```

---

## Discussion 相关事件

### `discussion.turn_started`

某个 Agent 开始思考 / 准备发言。UI 可展示"正在发言..."状态。

```json
{
  "session_id": "disc_01HX...",
  "agent_id": "agt_01HX_engineer",
  "turn_number": 5
}
```

---

### `discussion.token_stream`

Agent 发言的流式 token（实时渲染用）。

```json
{
  "session_id": "disc_01HX...",
  "agent_id": "agt_01HX_engineer",
  "turn_number": 5,
  "token": "从工程角度"
}
```

---

### `discussion.turn_completed`

单次发言完成，包含完整发言内容。UI 收到后替换流式缓冲区内容。

```json
{
  "session_id": "disc_01HX...",
  "turn": {
    "id": "turn_01HX...",
    "turn_number": 5,
    "speaker_type": "agent",
    "speaker_id": "agt_01HX_engineer",
    "content": "从工程角度看，情绪分析有两条技术路径：一是调用第三方 API（如 AWS Comprehend），二是本地部署轻量级模型...",
    "created_at": "2026-03-10T10:08:00Z"
  }
}
```

---

### `discussion.user_injected`

用户插话已写入，广播给同一会话的其他连接 Client（多端场景）。

```json
{
  "session_id": "disc_01HX...",
  "turn": {
    "id": "turn_01HX_user...",
    "turn_number": 6,
    "speaker_type": "user",
    "speaker_id": "usr_01HX...",
    "content": "重点关注一下成本和上线周期",
    "created_at": "2026-03-10T10:10:00Z"
  }
}
```

---

### `discussion.paused`

讨论被暂停。

```json
{
  "session_id": "disc_01HX...",
  "paused_at": "2026-03-10T10:15:00Z"
}
```

---

### `discussion.resumed`

讨论从暂停恢复。

```json
{
  "session_id": "disc_01HX...",
  "resumed_at": "2026-03-10T10:20:00Z"
}
```

---

### `discussion.concluded`

讨论结束，附带生成的结论全文。

```json
{
  "session_id": "disc_01HX...",
  "conclusion": "## 讨论结论\n\n**各方核心观点**：\n- 产品：分阶段上线，先中英文...\n- 研发：建议优先用第三方 API...\n- 测试：测试周期至少 3 周...\n\n**推荐方案**：...",
  "concluded_at": "2026-03-10T10:30:00Z"
}
```

---

### `discussion.memorized`

讨论结束后各 Agent 的记忆写入完成。UI 可在参与者列表上展示"已学习"标记。

```json
{
  "session_id": "disc_01HX...",
  "agent_summaries": [
    { "agent_id": "agt_01HX_product", "entry_count": 3 },
    { "agent_id": "agt_01HX_engineer", "entry_count": 4 },
    { "agent_id": "agt_01HX_qa", "entry_count": 2 }
  ]
}
```

---

## Agent 相关事件

### `agent.memorized`

Agent 完成任务/讨论后记忆写入完成。UI 可在 Agent 卡片展示"刚刚学到了新经验 ✨"提示。

```json
{
  "agent_id": "agt_01HX...",
  "source_type": "task",
  "source_id": "tsk_01HX...",
  "entry_count": 5
}
```

---

## MCP 相关事件

### `mcp.server_status`

MCP Server 连接状态变更（如刚注册后连接完成，或连接断开）。

```json
{
  "server_id": "mcp_notion",
  "status": "online",
  "discovered_tools_count": 8,
  "error": null,
  "updated_at": "2026-03-10T10:00:00Z"
}
```

---

## 系统事件

### `system.hub_info`

连接建立成功后立即推送一次，通知 Client 当前 Hub 状态。

```json
{
  "hub_version": "0.1.0",
  "tenant_id": "tnt_01HX...",
  "user_id": "usr_01HX...",
  "connected_at": "2026-03-10T08:00:00Z"
}
```

---

## 事件类型速查表

| 事件类型 | 触发场景 | 主要数据字段 |
|---------|---------|-----------|
| `task.status_changed` | 任务状态变更 | `task_id`, `status` |
| `subtask.progress` | 子任务状态变更 | `task_id`, `subtask_id`, `status` |
| `agent.token_stream` | Agent 任务输出 token | `task_id`, `agent_id`, `token` |
| `decision.pending` | 有待处理决策 | `task_id`, `decisions[]` |
| `decision.auto_resolved` | 决策自动处理 | `task_id`, `decision_id` |
| `task.completed` | 任务完成 | `task_id`, `result` |
| `task.failed` | 任务失败 | `task_id`, `error` |
| `task.terminated` | 任务被终止 | `task_id` |
| `discussion.turn_started` | Agent 开始发言 | `session_id`, `agent_id` |
| `discussion.token_stream` | 讨论流式 token | `session_id`, `agent_id`, `token` |
| `discussion.turn_completed` | 发言完成 | `session_id`, `turn{}` |
| `discussion.user_injected` | 用户插话写入 | `session_id`, `turn{}` |
| `discussion.paused` | 讨论暂停 | `session_id` |
| `discussion.resumed` | 讨论恢复 | `session_id` |
| `discussion.concluded` | 讨论结束 | `session_id`, `conclusion` |
| `discussion.memorized` | 讨论记忆写入完成 | `session_id`, `agent_summaries[]` |
| `agent.memorized` | Agent 记忆写入完成 | `agent_id`, `entry_count` |
| `mcp.server_status` | MCP Server 状态变更 | `server_id`, `status` |
| `system.hub_info` | 连接建立时推送 | `hub_version`, `tenant_id` |

---

## Client 端处理建议

### TypeScript 类型定义

```typescript
type HubEvent =
  // ── Task ──────────────────────────────────────────────────────────────
  | { type: 'task.status_changed';     data: { task_id: string; status: TaskStatus; previous_status: TaskStatus; updated_at: string } }
  | { type: 'subtask.progress';        data: { task_id: string; subtask_id: string; agent_id: string; status: SubtaskStatus } }
  | { type: 'agent.token_stream';      data: { task_id: string; subtask_id: string; agent_id: string; token: string } }
  | { type: 'decision.pending';        data: { task_id: string; decisions: Decision[] } }
  | { type: 'task.completed';          data: { task_id: string; result: string; completed_at: string } }
  | { type: 'task.failed';             data: { task_id: string; error: string; failed_at: string } }
  | { type: 'task.terminated';         data: { task_id: string; terminated_by: string; terminated_at: string } }

  // ── Discussion ────────────────────────────────────────────────────────
  | { type: 'discussion.turn_started';    data: { session_id: string; agent_id: string; turn_number: number } }
  | { type: 'discussion.token_stream';    data: { session_id: string; agent_id: string; turn_number: number; token: string } }
  | { type: 'discussion.turn_completed';  data: { session_id: string; turn: DiscussionTurn } }
  | { type: 'discussion.user_injected';   data: { session_id: string; turn: DiscussionTurn } }
  | { type: 'discussion.paused';          data: { session_id: string; paused_at: string } }
  | { type: 'discussion.resumed';         data: { session_id: string; resumed_at: string } }
  | { type: 'discussion.concluded';       data: { session_id: string; conclusion: string; concluded_at: string } }
  | { type: 'discussion.memorized';       data: { session_id: string; agent_summaries: { agent_id: string; entry_count: number }[] } }

  // ── Agent & MCP ───────────────────────────────────────────────────────
  | { type: 'agent.memorized';         data: { agent_id: string; source_type: 'task' | 'discussion'; source_id: string; entry_count: number } }
  | { type: 'mcp.server_status';       data: { server_id: string; status: 'online' | 'offline' | 'error'; discovered_tools_count?: number } }
  | { type: 'system.hub_info';         data: { hub_version: string; tenant_id: string; user_id: string } };
```

### 连接与重连示例（Vue 3 / TypeScript）

```typescript
// transport.ts
import { ref } from 'vue';

export function useHubEvents(baseUrl: string, token: string) {
  const lastEventId = ref<string | null>(null);
  let eventSource: EventSource | null = null;
  let retryTimeout: ReturnType<typeof setTimeout> | null = null;
  let retryDelay = 1000; // 初始重连延迟 1s

  function connect() {
    const url = new URL(`${baseUrl}/api/v1/events`);
    if (lastEventId.value) {
      url.searchParams.set('last_event_id', lastEventId.value);
    }

    eventSource = new EventSource(url.toString(), {
      // 注：标准 EventSource 不支持自定义 Header，生产环境需 token 走 Query Param 或 Cookie
      // 或者使用 fetch + ReadableStream 实现
    });

    eventSource.onopen = () => {
      retryDelay = 1000; // 重置退避延迟
    };

    eventSource.onmessage = (e) => {
      // fallback：处理没有 event 字段的消息
    };

    // 注册各事件类型
    const eventTypes: HubEvent['type'][] = [
      'task.status_changed', 'agent.token_stream', 'decision.pending',
      'discussion.token_stream', 'discussion.concluded', /* ... */
    ];

    for (const eventType of eventTypes) {
      eventSource.addEventListener(eventType, (e) => {
        lastEventId.value = e.lastEventId;
        const data = JSON.parse(e.data) as HubEvent['data'];
        handleEvent(eventType, data);
      });
    }

    eventSource.onerror = () => {
      eventSource?.close();
      // 指数退避重连（最大 30s）
      retryDelay = Math.min(retryDelay * 2, 30000);
      retryTimeout = setTimeout(connect, retryDelay);
    };
  }

  function disconnect() {
    if (retryTimeout) clearTimeout(retryTimeout);
    eventSource?.close();
  }

  return { connect, disconnect };
}
```

> **本地模式**（Tauri）：使用 `listen('hub://task.status_changed', handler)` 替代 SSE，API 完全相同，仅传输层不同。

---

*← 返回 [README.md](./README.md)*