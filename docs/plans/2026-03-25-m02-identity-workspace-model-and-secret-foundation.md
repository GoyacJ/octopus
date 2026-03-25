# M02 身份、工作区、模型与密钥基础实施计划

- Status: `Not Started`
- Last Updated: `2026-03-25`
- Related Master Plan: `docs/plans/2026-03-25-product-development-master-plan.md`
- Related Change: `docs/changes/<date>-identity-workspace-model-and-secret-foundation.md`
- Objective: `冻结身份、工作区、多 Hub、模型注册与密钥引用的正式合同，避免后续实现阶段反复改写基础对象。`

## Inputs

- `docs/PRD.md`
- `docs/DOMAIN.md`
- `docs/DATA_MODEL.md`
- `docs/API/README.md`
- `docs/API/AUTH.md`

## Contracts To Freeze

- `Tenant`、`User`、RBAC、租户隔离与多 Hub 的对象关系。
- 远程 Hub 握手、登录、刷新、登出和当前用户查询的 API 边界。
- `ModelRegistration / ModelBinding / RoutingPolicy / SecretBinding` 的正式公共合同。
- 本地 Hub 与远程 Hub 在认证、租户视图、模型默认值上的差异。

## Repo Reality

- 当前 `docs/API/MODELS.md` 即使存在，也只能视作草案输入，正式 required-doc 需要在本里程碑完成时再决定。
- `DATA_MODEL.md` 仍保留部分旧的 `model_config` 叙述，执行本计划时必须先统一成最终冻结合同再允许其他里程碑引用。

## Deliverables

- 身份与多 Hub 合同矩阵。
- 模型与密钥对象关系表。
- 正式 API 范围与数据约束清单。
- 默认差异规则：本地单用户 vs 远程多租户。

## Verification

- 对 `tenant_id`、RBAC 角色、Hub 握手、token 生命周期、`SecretBinding` 和模型对象命名做一致性 grep。
- 检查 `DOMAIN / DATA_MODEL / API` 对同一对象没有双重定义。
- 检查后续 `M4 / M5 / M7` 的计划输入只引用冻结后的对象名。

## Docs Sync

- `docs/DOMAIN.md`
- `docs/DATA_MODEL.md`
- `docs/API/README.md`
- `docs/API/AUTH.md`
- `docs/plans/2026-03-25-product-development-master-plan.md`
- `docs/changes/<date>-identity-workspace-model-and-secret-foundation.md`

## Open Risks

- 模型中心与 Agent 绑定语义若未冻结，会继续污染 Agent API、视觉框架和控制台信息架构。

## Out Of Scope

- Agent、Task、Discussion 对模型的具体消费实现。
- SSE、审计和时间线观测实现。
