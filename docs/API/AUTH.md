# Auth API

> 认证相关端点，仅适用于**远程 Hub** 模式。

---

## 目录

- [Hub 握手](#1-hub-握手)
- [登录](#2-登录)
- [刷新 Token](#3-刷新-token)
- [登出](#4-登出)
- [获取当前用户信息](#5-获取当前用户信息)

---

## 1. Hub 握手

Client 添加新 Hub 时，首先调用此端点探测 Hub 存活状态并获取支持的认证方式。

```http
GET /api/v1/auth/handshake
```

**无需认证**

**响应 `200 OK`**：

```json
{
  "data": {
    "hub_version": "0.1.0",
    "hub_name": "Acme Corp Hub",
    "auth_methods": ["password", "api_key"],
    "tenant_mode": "multi"
  }
}
```

| 字段 | 类型 | 说明 |
|-----|------|------|
| `hub_version` | string | Hub 软件版本 |
| `hub_name` | string | Hub 自定义名称（管理员可配置） |
| `auth_methods` | string[] | 支持的认证方式：`password`、`api_key`、`sso`（Phase 2） |
| `tenant_mode` | string | `single`（单租户）或 `multi`（多租户） |

---

## 2. 登录

使用用户名 + 密码获取 Access Token 和 Refresh Token。

```http
POST /api/v1/auth/login
```

**无需认证**

**请求体**：

```json
{
  "username": "alice",
  "password": "your-password",
  "tenant_id": "optional-tenant-id"
}
```

| 字段 | 类型 | 必填 | 说明 |
|-----|------|-----|------|
| `username` | string | ✅ | 用户名 |
| `password` | string | ✅ | 密码（明文，传输层加密） |
| `tenant_id` | string | ❌ | 多租户模式下指定目标租户；单租户模式忽略 |

**响应 `200 OK`**：

```json
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4...",
    "expires_in": 3600,
    "token_type": "Bearer",
    "user": {
      "id": "usr_01HX...",
      "username": "alice",
      "tenant_id": "tnt_01HX...",
      "roles": ["member"]
    }
  }
}
```

| 字段 | 类型 | 说明 |
|-----|------|------|
| `access_token` | string | JWT Access Token，有效期 1 小时 |
| `refresh_token` | string | Refresh Token，有效期 30 天 |
| `expires_in` | integer | Access Token 剩余秒数（固定 3600） |
| `token_type` | string | 固定为 `"Bearer"` |
| `user.roles` | string[] | 当前用户在本租户的角色列表 |

**错误响应**：

| 错误码 | 状态码 | 触发条件 |
|-------|-------|---------|
| `invalid_credentials` | 401 | 用户名或密码错误 |
| `account_locked` | 403 | 账号因多次失败被临时锁定 |
| `account_suspended` | 403 | 账号被管理员停用 |
| `tenant_not_found` | 404 | 指定的 `tenant_id` 不存在 |

> **登录失败限制**：连续失败 5 次后账号锁定 15 分钟。

---

## 2b. API Key 登录

当 Hub 支持 `api_key` 认证方式时，可用 API Key 直接换取 Token。

```http
POST /api/v1/auth/login
```

**请求体**：

```json
{
  "api_key": "nvai_sk_live_xxxxxxxxxxxxxxxxxxxxxxxx"
}
```

**响应**：同用户名密码登录，但不返回 `refresh_token`（API Key 模式无需刷新）。

---

## 3. 刷新 Token

使用 Refresh Token 换取新的 Access Token。

```http
POST /api/v1/auth/refresh
```

**无需认证**（使用 Refresh Token 而非 Access Token）

**请求体**：

```json
{
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4..."
}
```

**响应 `200 OK`**：

```json
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "expires_in": 3600,
    "token_type": "Bearer"
  }
}
```

> Refresh Token 本身有效期不会延长，只有重新登录才能获得新的 Refresh Token。

**错误响应**：

| 错误码 | 状态码 | 触发条件 |
|-------|-------|---------|
| `token_invalid` | 401 | Refresh Token 格式错误 |
| `token_expired` | 401 | Refresh Token 已过期，需重新登录 |
| `token_revoked` | 401 | Token 已被管理员手动吊销 |

---

## 4. 登出

使当前 Token 失效（服务端加入黑名单）。

```http
POST /api/v1/auth/logout
Authorization: Bearer {access_token}
```

**请求体**（可选，同时吊销 Refresh Token）：

```json
{
  "refresh_token": "dGhpcyBpcyBhIHJlZnJlc2ggdG9rZW4..."
}
```

**响应 `204 No Content`**

> Client 应在登出后立即从 OS Keychain 中删除 Token。

---

## 5. 获取当前用户信息

获取当前 Token 对应的用户信息。

```http
GET /api/v1/auth/me
Authorization: Bearer {access_token}
```

**响应 `200 OK`**：

```json
{
  "data": {
    "id": "usr_01HX...",
    "username": "alice",
    "email": "alice@example.com",
    "tenant_id": "tnt_01HX...",
    "roles": ["member", "discussion.create"],
    "status": "active",
    "last_seen_at": "2026-03-10T08:00:00Z",
    "created_at": "2026-01-01T00:00:00Z"
  }
}
```

| 字段 | 类型 | 说明 |
|-----|------|------|
| `roles` | string[] | 用户在当前租户拥有的所有角色 |
| `status` | string | `active` / `suspended` / `locked` |

---

## 角色与权限速查

| 角色 | 可执行操作 |
|-----|---------|
| `hub_admin` | Hub 所有管理权限，含租户管理 |
| `tenant_admin` | 本租户所有权限 |
| `member` | 创建 Agent、下发任务、查看自己的结果 |
| `viewer` | 只读查询 |
| `discussion.create` | 创建讨论会话 |
| `discussion.view` | 查看讨论记录 |
| `discussion.conclude` | 结束讨论（生成结论） |

---

*← 返回 [README.md](./README.md)*