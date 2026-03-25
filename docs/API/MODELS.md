# Models API Draft

> 当前文件仅作为 `M02` 的草案输入，不属于当前正式 required-doc 或 API 导航基线。

> 模型中心是 octopus Hub 管理面的目标态能力，负责统一管理 Provider 连接、模型目录、业务模型档案，以及租户可见性与默认绑定。

---

## 目录

- [对象结构](#对象结构)
- [获取 Provider 列表](#1-获取-provider-列表)
- [创建 Provider](#2-创建-provider)
- [更新 Provider](#3-更新-provider)
- [获取模型目录](#4-获取模型目录)
- [创建模型目录项](#5-创建模型目录项)
- [获取模型档案列表](#6-获取模型档案列表)
- [创建模型档案](#7-创建模型档案)
- [更新模型档案](#8-更新模型档案)
- [获取租户模型策略](#9-获取租户模型策略)
- [更新租户模型策略](#10-更新租户模型策略)

---

## 对象结构

### ModelProvider

```json
{
  "id": "mdlprov_openai_primary",
  "name": "OpenAI 主 Provider",
  "provider_type": "openai",
  "base_url": "https://api.openai.com/v1",
  "auth_mode": "secret_binding",
  "secret_binding_ref": "secret_openai_primary",
  "status": "active",
  "created_at": "2026-03-25T00:00:00Z",
  "updated_at": "2026-03-25T00:00:00Z"
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `provider_type` | enum | `openai` / `anthropic` / `gemini` / `ollama` / `compatible` |
| `base_url` | string? | 自定义端点；官方默认端点可为空 |
| `auth_mode` | enum | 本轮固定为 `secret_binding` |
| `secret_binding_ref` | string? | 指向 Hub 安全存储中的凭据引用 |
| `status` | enum | `active` / `disabled` / `degraded` |

### ModelCatalogItem

```json
{
  "id": "mdlcat_openai_gpt4o",
  "provider_id": "mdlprov_openai_primary",
  "kind": "inference",
  "model": "gpt-4o",
  "display_name": "GPT-4o",
  "capabilities": {
    "streaming": true,
    "tool_calling": true
  },
  "is_enabled": true
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `kind` | enum | `inference` / `embedding` |
| `model` | string | Provider 原始模型 ID |
| `display_name` | string | 面向管理员的可读名称 |
| `capabilities` | object | 能力元数据摘要 |
| `is_enabled` | boolean | 是否可被创建为 Profile |

### ModelProfile

```json
{
  "id": "mdlprof_general_reasoning_v1",
  "name": "通用推理 · 标准档",
  "description": "默认给通用 Agent 使用的推理档案",
  "kind": "inference",
  "provider_id": "mdlprov_openai_primary",
  "catalog_item_id": "mdlcat_openai_gpt4o",
  "model": "gpt-4o",
  "parameter_preset": {
    "temperature": 0.7,
    "max_tokens": 4096,
    "top_p": null
  },
  "status": "active",
  "created_at": "2026-03-25T00:00:00Z",
  "updated_at": "2026-03-25T00:00:00Z"
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `kind` | enum | `inference` / `embedding` |
| `provider_id` | string | 来源 Provider |
| `catalog_item_id` | string | 来源目录项 |
| `model` | string | 冗余展示字段，便于 UI 直接显示 |
| `parameter_preset` | object | 对业务固定的参数预设 |
| `status` | enum | `active` / `disabled` / `deprecated` |

### TenantModelPolicy

```json
{
  "tenant_id": "tnt_01HX...",
  "allowed_model_profile_ids": [
    "mdlprof_general_reasoning_v1",
    "mdlprof_summary_standard_v1",
    "mdlprof_embed_local_v1"
  ],
  "default_agent_profile_id": "mdlprof_general_reasoning_v1",
  "default_summary_profile_id": "mdlprof_summary_standard_v1",
  "default_embedding_profile_id": "mdlprof_embed_local_v1",
  "updated_at": "2026-03-25T00:00:00Z"
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `allowed_model_profile_ids` | string[] | 当前租户可见的模型档案 |
| `default_agent_profile_id` | string | Agent 默认推理档案 |
| `default_summary_profile_id` | string | 讨论结论、记忆提取等系统摘要默认档案 |
| `default_embedding_profile_id` | string | Embedding 默认档案 |

---

## 1. 获取 Provider 列表

```http
GET /api/v1/model-providers
Authorization: Bearer {token}
```

**所需角色**：`hub_admin`

---

## 2. 创建 Provider

```http
POST /api/v1/model-providers
Authorization: Bearer {token}
```

**所需角色**：`hub_admin`

**请求体**：

```json
{
  "name": "OpenAI 主 Provider",
  "provider_type": "openai",
  "base_url": "https://api.openai.com/v1",
  "auth_mode": "secret_binding",
  "secret_binding_ref": "secret_openai_primary"
}
```

---

## 3. 更新 Provider

```http
PUT /api/v1/model-providers/:provider_id
Authorization: Bearer {token}
```

**所需角色**：`hub_admin`

允许更新：

1. `name`
2. `base_url`
3. `secret_binding_ref`
4. `status`

---

## 4. 获取模型目录

```http
GET /api/v1/model-catalog
Authorization: Bearer {token}
```

**所需角色**：`hub_admin`

**查询参数**：

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `provider_id` | string? | 按 Provider 过滤 |
| `kind` | string? | `inference` / `embedding` |
| `enabled` | boolean? | 是否只看启用项 |

---

## 5. 创建模型目录项

```http
POST /api/v1/model-catalog
Authorization: Bearer {token}
```

**所需角色**：`hub_admin`

**请求体**：

```json
{
  "provider_id": "mdlprov_openai_primary",
  "kind": "inference",
  "model": "gpt-4o",
  "display_name": "GPT-4o",
  "capabilities": {
    "streaming": true,
    "tool_calling": true
  },
  "is_enabled": true
}
```

---

## 6. 获取模型档案列表

```http
GET /api/v1/model-profiles
Authorization: Bearer {token}
```

**所需角色**：

1. `hub_admin`：可查看全部档案。
2. `tenant_admin` / `member`：只返回当前租户允许使用的档案。

---

## 7. 创建模型档案

```http
POST /api/v1/model-profiles
Authorization: Bearer {token}
```

**所需角色**：`hub_admin`

**请求体**：

```json
{
  "name": "通用推理 · 标准档",
  "description": "默认给通用 Agent 使用的推理档案",
  "kind": "inference",
  "provider_id": "mdlprov_openai_primary",
  "catalog_item_id": "mdlcat_openai_gpt4o",
  "parameter_preset": {
    "temperature": 0.7,
    "max_tokens": 4096,
    "top_p": null
  }
}
```

---

## 8. 更新模型档案

```http
PUT /api/v1/model-profiles/:profile_id
Authorization: Bearer {token}
```

**所需角色**：`hub_admin`

允许更新：

1. `name`
2. `description`
3. `parameter_preset`
4. `status`

---

## 9. 获取租户模型策略

```http
GET /api/v1/tenant-model-policy
Authorization: Bearer {token}
```

**所需角色**：`tenant_admin` 或 `viewer` 以上

---

## 10. 更新租户模型策略

```http
PUT /api/v1/tenant-model-policy
Authorization: Bearer {token}
```

**所需角色**：`tenant_admin`

**请求体**：

```json
{
  "allowed_model_profile_ids": [
    "mdlprof_general_reasoning_v1",
    "mdlprof_summary_standard_v1",
    "mdlprof_embed_local_v1"
  ],
  "default_agent_profile_id": "mdlprof_general_reasoning_v1",
  "default_summary_profile_id": "mdlprof_summary_standard_v1",
  "default_embedding_profile_id": "mdlprof_embed_local_v1"
}
```

**约束**：

1. 三个默认值都必须包含在 `allowed_model_profile_ids` 中。
2. `default_agent_profile_id` 与 `default_summary_profile_id` 必须引用 `inference` 档案。
3. `default_embedding_profile_id` 必须引用 `embedding` 档案。
